//! Phase 4 Task 5: the island runs, stops, and carries on as if it never did.
//!
//! Gate 4 asks that two identical fixed-seed runs end in identical canonical
//! hashes, and that save/load round-trips. The round trip is pinned in
//! `island_snapshot`; what is pinned here is the thing a headless runner is
//! actually for — that an interrupted run and an uninterrupted one are the
//! same run.

use std::path::PathBuf;
use std::sync::Arc;

use mk_core::canon::CanonLocked;
use mk_engine::io::{load_island_snapshot, save_island_snapshot};
use mk_engine::regional::life::IslandLife;
use mk_island::IslandScenario;

/// One human step, the cadence the runner counts in.
const DT: u64 = 60;

fn repo(path: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path)
}

fn canon() -> Arc<CanonLocked> {
    Arc::new(CanonLocked::load(&repo("fixtures/island/canon.json")).unwrap())
}

/// The default island with a small tree cap: 200,000 stems make every
/// bootstrap slow and nothing here depends on the patch's size.
fn bootstrap_island() -> IslandLife {
    let mut scenario =
        IslandScenario::load(&repo("fixtures/island/default_scenario.json")).unwrap();
    scenario.estate_patch.tree_cap = 200;
    IslandLife::bootstrap(scenario, canon()).expect("the island bootstraps")
}

/// Every test here starts from the same island, and bootstrapping spins its
/// climate up for two orbits -- about half a minute in a debug build. So it
/// is bootstrapped once and each test gets its own copy restored from the
/// snapshot, which brings an island back exactly (`island_snapshot.rs`).
fn island() -> IslandLife {
    static BOOTSTRAPPED: std::sync::OnceLock<mk_engine::regional::life::IslandLifeSnapshot> =
        std::sync::OnceLock::new();
    let snapshot = BOOTSTRAPPED
        .get_or_init(|| bootstrap_island().snapshot())
        .clone();
    let canon = std::sync::Arc::new(
        mk_core::canon::CanonLocked::load(&repo("fixtures/island/canon.json")).expect("canon"),
    );
    IslandLife::restore(canon, snapshot).expect("the island restores")
}

fn hex(d: [u8; 32]) -> String {
    d.iter().map(|b| format!("{b:02x}")).collect()
}

/// 100 steps, saved, loaded, 100 more — against 200 in one go.
///
/// This is the whole promise of a headless runner with snapshots under it:
/// stopping is not an event in the world's history. If the two hashes ever
/// differ, something in the island depends on having been running rather
/// than on its own state, and no amount of saving would make a run
/// resumable.
#[test]
fn stopping_and_resuming_is_the_same_run_as_never_stopping() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("halfway.mks");

    let mut interrupted = island();
    interrupted.advance(100 * DT).unwrap();
    let halfway = hex(interrupted.state_digest());
    save_island_snapshot(&mut interrupted, &path).unwrap();

    let mut resumed = load_island_snapshot(canon(), &path).unwrap();
    assert_eq!(
        hex(resumed.state_digest()),
        halfway,
        "the island changed on the way through the file"
    );
    resumed.advance(100 * DT).unwrap();

    let mut straight = island();
    straight.advance(200 * DT).unwrap();

    assert_eq!(
        hex(resumed.state_digest()),
        hex(straight.state_digest()),
        "a run that stopped halfway ended somewhere else"
    );
    assert_eq!(resumed.sim_time_s, straight.sim_time_s);
    assert_eq!(resumed.audits_closed, straight.audits_closed);
    assert_eq!(resumed.shortfalls, straight.shortfalls);
}

/// The same scenario and seed, twice, reach the same state — and a run split
/// into different-sized steps reaches it too.
///
/// Gate 4's "identical fixed-seed runs end in identical canonical hashes"
/// has to hold across step sizes as well as across processes, or the
/// runner's `--dt` would silently be part of the world.
#[test]
fn the_same_run_twice_and_at_a_different_step_size_land_together() {
    let (mut a, mut b) = (island(), island());
    a.advance(60 * DT).unwrap();
    b.advance(60 * DT).unwrap();
    assert_eq!(
        hex(a.state_digest()),
        hex(b.state_digest()),
        "two runs differ"
    );

    // The same simulated hour, taken ten steps at a time.
    let mut split = island();
    for _ in 0..6 {
        split.advance(10 * DT).unwrap();
    }
    assert_eq!(
        hex(split.state_digest()),
        hex(a.state_digest()),
        "the size of the steps changed where the island ended up"
    );
}
