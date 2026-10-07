//! Phase 4 acceptance: 100 ticks, snapshot, load, 100 more ticks equals an
//! uninterrupted 200-tick run.

use std::path::PathBuf;
use std::sync::Arc;

use mk_core::canon::CanonLocked;
use mk_engine::io::island_snapshot::{load_island_snapshot, save_island_snapshot};
use mk_engine::regional::world::IslandWorldState;
use mk_island::IslandScenario;

fn repo(p: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(p)
}

fn world() -> IslandWorldState {
    let scenario = IslandScenario::load(&repo("fixtures/island/default_scenario.json")).unwrap();
    let canon = Arc::new(CanonLocked::load(&repo("fixtures/island/canon.json")).unwrap());
    IslandWorldState::new(canon, scenario).unwrap()
}

#[test]
fn a_saved_and_loaded_run_matches_an_uninterrupted_one() {
    let mut straight = world();
    for _ in 0..200 {
        straight.step(60).unwrap();
    }

    let mut first = world();
    for _ in 0..100 {
        first.step(60).unwrap();
    }
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("run.snap");
    save_island_snapshot(&first, &path).unwrap();
    drop(first);
    let mut second = load_island_snapshot(&path).unwrap();
    assert_eq!(second.tick(), 100);
    for _ in 0..100 {
        second.step(60).unwrap();
    }
    assert_eq!(second.tick(), 200);
    assert_eq!(
        second.state_hash().unwrap(),
        straight.state_hash().unwrap(),
        "the resumed run diverged from the uninterrupted one"
    );
}
