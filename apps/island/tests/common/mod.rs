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

/// The island every fixture here runs: small, seeded to pass its own
/// shape rules, and paced rather than flat out.
pub fn a_small_island() -> island::serve::sim::SimHandle {
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
    spawn(life, SimSpeed::Times(600))
}

/// A stored population and a listener on a free loopback port.
async fn a_dashboard_for(
    world: &island::serve::sim::SimHandle,
    seed: u8,
) -> (
    u16,
    tokio::net::TcpListener,
    impl warp::Filter<Extract = (impl warp::Reply,), Error = std::convert::Infallible> + Clone,
) {
    let data_dir = Box::leak(Box::new(tempfile::tempdir().unwrap()));
    let (population, _) =
        island_humans::IslandHumanPopulation::open(data_dir.path(), [seed; 32]).unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let routes = server::routes_with_world(
        Arc::new(Mutex::new(population)),
        ControlAuth::LoopbackOnly,
        Some(world.clone()),
    );
    (port, listener, routes)
}

/// A dashboard with an island of its very own, for a test that changes it.
pub async fn a_running_dashboard() -> (
    island::serve::sim::SimHandle,
    u16,
    tokio::task::JoinHandle<()>,
) {
    let world = a_small_island();
    let (port, listener, routes) = a_dashboard_for(&world, 11).await;
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

/// One island, built once, shared by every test that only reads it.
///
/// `serve.rs` used to bootstrap an island per test -- eighteen of them on
/// a four-core machine, each with its own simulation thread. The dominant
/// cost is not the stepping but the grids: two 1,152,000-cell grids built
/// on every bootstrap and hashed on every `DIGEST_EVERY`, which
/// `tree_cap` does not shrink. A dozen of those starve the tokio runtime
/// answering the HTTP the tests make, and
/// `a_new_person_can_be_read_the_moment_they_exist` missed its deadline
/// between one run in three and every run.
///
/// Two things were done about it. The first, already in place, was the
/// small profile -- 240 x 192 medium cells, 25x fewer. This is the
/// second, and the one `docs/ISLAND_SYSTEM_STATUS.md` named as the real
/// fix: the tests that only *read* an island share one, so the suite
/// builds seven fewer.
///
/// It lives on a thread of its own with its own runtime, and is never
/// stopped: it has to outlive every test that might still be reading it,
/// and the process ending is what stops it. That is also why this hands
/// back no join handle -- there is nothing a caller should abort.
///
/// **Only for tests that do not change it.** A test that creates a
/// person, pauses the clock or intervenes must call
/// [`a_running_dashboard`] and get its own, because the next test to read
/// this one would see what it did.
///
/// It hands back the port and nothing else, deliberately. A `SimHandle`
/// would let a caller pause this island or step it, which is exactly what
/// must not happen to one several tests are reading -- and a test that
/// needs a handle needs its own island, so failing to compile is the
/// right answer rather than a comment asking nicely.
pub fn a_shared_dashboard() -> u16 {
    use std::sync::OnceLock;
    // The handle is kept inside the cell, not handed out: something has
    // to own it or the simulation thread's channel closes and the island
    // stops after one step.
    static SHARED: OnceLock<(island::serve::sim::SimHandle, u16)> = OnceLock::new();
    SHARED
        .get_or_init(|| {
            let (ready, started) = std::sync::mpsc::channel();
            std::thread::Builder::new()
                .name("shared-island".into())
                .spawn(move || {
                    let runtime = tokio::runtime::Builder::new_multi_thread()
                        .worker_threads(2)
                        .enable_all()
                        .build()
                        .expect("a runtime for the shared island");
                    runtime.block_on(async move {
                        let world = a_small_island();
                        let (port, listener, routes) = a_dashboard_for(&world, 13).await;
                        ready.send((world, port)).expect("the test is waiting");
                        warp::serve(routes).incoming(listener).run().await;
                    });
                })
                .expect("a thread for the shared island");
            started.recv().expect("the shared island starts")
        })
        .1
}
