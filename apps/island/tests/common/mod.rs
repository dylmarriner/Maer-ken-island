//! The fixture both integration suites run against: a real island on a
//! real sim thread, behind a real listener on a real port.
//!
//! One definition because there were two, and two fixtures for one thing
//! drift -- the comments below are the record of what this island had to
//! be tuned to, and that record is worth exactly one copy.

#![allow(dead_code)]

use island::serve::auth::ControlAuth;
use island::serve::server;
use std::sync::{Arc, Mutex};

/// A dashboard with a small island running on it, and its port.
pub async fn a_running_dashboard() -> (
    island::serve::sim::SimHandle,
    u16,
    tokio::task::JoinHandle<()>,
) {
    use island::serve::sim::{spawn, SimSpeed};
    use std::path::PathBuf;

    let repo = |path: &str| {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(path)
    };
    let mut scenario =
        mk_island::IslandScenario::load(&repo("fixtures/island/default_scenario.json")).unwrap();
    scenario.estate_patch.tree_cap = 50;
    // A small island: 240 x 192 medium cells against the default's
    // 1,200 x 960, 25x fewer. The grids are what load this suite, not the
    // trees -- `tree_cap` shrinks the vegetation and leaves two
    // 1,152,000-cell grids to build on every bootstrap and hash on every
    // digest, a dozen times over, on four cores.
    //
    // The seed changes with the profile because it has to. `IslandLife`
    // validates the coastline it generates against the profile's shape
    // rules, and the default scenario's seed makes a small island with
    // "0 major headlands, need 3". Seed 16 is simply the first that does
    // not: found by trying 0, 1, 2 ... through
    // `RegionalPhysicalState::bootstrap`, which runs that check before the
    // expensive spin-up, so bad seeds cost nothing. It took 17 tries and
    // 1.3 seconds.
    scenario.profile = mk_island::IslandProfile::test_small();
    let mut small_seed = [0u8; 32];
    small_seed[..4].copy_from_slice(&16u32.to_le_bytes());
    scenario.seed = small_seed;
    let canon =
        Arc::new(mk_core::canon::CanonLocked::load(&repo("fixtures/island/canon.json")).unwrap());
    let life = mk_engine::regional::life::IslandLife::bootstrap(scenario, canon).unwrap();
    // Paced, not flat out. This container has four cores and this file
    // holds eighteen islands, each on its own sim thread: at
    // `AsFastAsPossible` every one of them pegs a core and the tokio
    // runtime serving the HTTP requests is starved, so
    // `a_new_person_can_be_read_the_moment_they_exist` misses its
    // sixty-second deadline. Measured, that was about a coin flip --
    // 19 passed twice, then failed twice, on the same commit.
    //
    // The dominant cost is not the stepping, it is `DIGEST_EVERY`: every
    // 60 ticks the loop hashes the whole state, and that walks two
    // 1,152,000-cell grids whatever `tree_cap` is, so a test island pays
    // nearly what the real one does. Slowing the ticks is what thins the
    // digests out. `Times(600)` runs a 60-second step every 100 ms, so a
    // digest lands every six seconds rather than every half one, and a
    // test needing sixty ticks still gets them well inside its deadline.
    let world = spawn(life, SimSpeed::Times(600));

    let data_dir = Box::leak(Box::new(tempfile::tempdir().unwrap()));
    let (population, _) =
        island_humans::IslandHumanPopulation::open(data_dir.path(), [11u8; 32]).unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let routes = server::routes_with_world(
        Arc::new(Mutex::new(population)),
        ControlAuth::LoopbackOnly,
        Some(world.clone()),
    );
    let server = tokio::spawn(async move { warp::serve(routes).incoming(listener).run().await });
    (world, port, server)
}

/// As [`a_running_dashboard`], with the backend exposed the way an
/// operator would expose it to another machine.
pub async fn a_remote_backend(
    config: server::ServeConfig,
) -> (
    island::serve::sim::SimHandle,
    u16,
    tokio::task::JoinHandle<()>,
) {
    let (world, _, old) = a_running_dashboard().await;
    old.abort();
    // The same island, served again under the given configuration. The
    // listener has to be new because the first one went with the task.
    let data_dir = Box::leak(Box::new(tempfile::tempdir().unwrap()));
    let (population, _) =
        island_humans::IslandHumanPopulation::open(data_dir.path(), [12u8; 32]).unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let routes = server::routes_with_config(
        Arc::new(Mutex::new(population)),
        Some(world.clone()),
        config,
    );
    let server = tokio::spawn(async move { warp::serve(routes).incoming(listener).run().await });
    (world, port, server)
}
