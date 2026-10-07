//! The island never builds the planetary world: run with
//! `cargo test -p mk_engine --features construction-counters --test island_allocation_bounds`.
#![cfg(feature = "construction-counters")]

use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use mk_core::canon::CanonLocked;
use mk_engine::regional::world::IslandWorldState;
use mk_engine::world_integration::{WorldState, PLANETARY_WORLD_CONSTRUCTIONS};
use mk_island::{DomainLevel, IslandScenario};

fn repo(p: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(p)
}

// One test, so nothing else in this process builds a planetary world
// between the two readings.
#[test]
fn the_island_builds_and_steps_without_a_planetary_world_and_the_counter_works() {
    let scenario = IslandScenario::load(&repo("fixtures/island/default_scenario.json")).unwrap();
    let canon = Arc::new(CanonLocked::load(&repo("fixtures/island/canon.json")).unwrap());
    assert_eq!(PLANETARY_WORLD_CONSTRUCTIONS.load(Ordering::SeqCst), 0);

    let mut island = IslandWorldState::new(canon, scenario).unwrap();
    for _ in 0..10 {
        island.step(60).unwrap();
    }
    assert_eq!(
        PLANETARY_WORLD_CONSTRUCTIONS.load(Ordering::SeqCst),
        0,
        "building and stepping the island constructed a planetary world"
    );

    // The grids are the profile's (12 km coarse, 2 km medium), not the
    // planet's 32 x 64.
    let domain = &island.life.domain;
    let dims = |level| (domain.rows(level), domain.cols(level));
    assert_eq!(dims(DomainLevel::Coarse), (160, 200));
    assert_eq!(dims(DomainLevel::Medium), (960, 1200));

    // Positive control: the counter does count a planetary build.
    // (The planetary world needs upstream's own canon; the island's differs.)
    let _planet = WorldState::new(Arc::new(CanonLocked::default()), [7u8; 32]);
    assert_eq!(PLANETARY_WORLD_CONSTRUCTIONS.load(Ordering::SeqCst), 1);
}
