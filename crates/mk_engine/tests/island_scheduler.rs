//! Phase 4 Task 2: the cadence-aware deterministic scheduler.

use std::path::PathBuf;
use std::sync::Arc;

use mk_core::canon::CanonLocked;
use mk_engine::regional::scheduler::SchedulerCounters;
use mk_engine::regional::world::IslandWorldState;
use mk_island::IslandScenario;

fn repo(p: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(p)
}

fn bootstrap_world() -> IslandWorldState {
    let scenario = IslandScenario::load(&repo("fixtures/island/default_scenario.json")).unwrap();
    let canon = Arc::new(CanonLocked::load(&repo("fixtures/island/canon.json")).unwrap());
    IslandWorldState::new(canon, scenario).unwrap()
}

/// Every test here starts from the same island, and bootstrapping spins its
/// climate up for two orbits -- about half a minute in a debug build. So it
/// is bootstrapped once and each test gets its own copy restored from the
/// snapshot, which brings an island back exactly (`island_snapshot.rs`).
fn world() -> IslandWorldState {
    static BOOTSTRAPPED: std::sync::OnceLock<mk_engine::regional::life::IslandLifeSnapshot> =
        std::sync::OnceLock::new();
    let snapshot = BOOTSTRAPPED
        .get_or_init(|| bootstrap_world().life.snapshot())
        .clone();
    let canon = Arc::new(CanonLocked::load(&repo("fixtures/island/canon.json")).unwrap());
    IslandWorldState {
        life: mk_engine::regional::life::IslandLife::restore(canon, snapshot)
            .expect("the island restores"),
    }
}

fn hash(w: &IslandWorldState) -> String {
    w.state_hash()
        .unwrap()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn counters(w: &IslandWorldState) -> SchedulerCounters {
    w.life.scheduler.counters
}

#[test]
fn one_hour_in_one_step_equals_sixty_one_minute_steps() {
    let (mut a, mut b) = (world(), world());
    a.step(3_600).unwrap();
    for _ in 0..60 {
        b.step(60).unwrap();
    }
    assert_eq!(counters(&a), counters(&b));
    assert_eq!(
        counters(&a),
        SchedulerCounters {
            human: 60,
            weather_ocean: 1,
            hydrology_ecology_resource: 0,
            geophysics: 0,
            human_store: 1,
        }
    );
    assert_eq!(hash(&a), hash(&b));
    assert_eq!((a.tick(), a.sim_time_seconds()), (60, 3_600.0));
}

#[test]
fn an_irregular_step_carries_its_remainder_deterministically() {
    let mut a = world();
    a.step(3_701).unwrap();
    // 61 whole minutes ran; 41 s wait for the next call.
    assert_eq!((a.tick(), a.sim_time_seconds()), (61, 3_660.0));
    assert_eq!(a.life.scheduler.accumulators.remainder_s, 41);
    assert_eq!(counters(&a).human, 61);

    // Two 3,701 s steps are exactly one 7,402 s step (123 minutes, 22 s left).
    a.step(3_701).unwrap();
    let mut b = world();
    b.step(7_402).unwrap();
    assert_eq!(a.life.scheduler.accumulators.remainder_s, 22);
    assert_eq!(b.life.scheduler.accumulators.remainder_s, 22);
    assert_eq!(counters(&a), counters(&b));
    assert_eq!(hash(&a), hash(&b));

    // Repeated execution gives the same state.
    let mut c = world();
    c.step(3_701).unwrap();
    c.step(3_701).unwrap();
    assert_eq!(hash(&c), hash(&a));
    // A zero step changes nothing.
    let before = hash(&a);
    a.step(0).unwrap();
    assert_eq!(hash(&a), before);
}

/// One 86,400 s step against 1,440 steps of 60 s. Slow tier.
#[test]
#[ignore = "slow: two simulated days"]
fn slow_a_day_in_one_step_equals_1440_minute_steps() {
    let (mut a, mut b) = (world(), world());
    a.step(86_400).unwrap();
    for _ in 0..1_440 {
        b.step(60).unwrap();
    }
    assert_eq!(counters(&a), counters(&b));
    assert_eq!(
        counters(&a),
        SchedulerCounters {
            human: 1_440,
            weather_ocean: 24,
            hydrology_ecology_resource: 4,
            geophysics: 1,
            human_store: 24,
        }
    );
    assert_eq!(hash(&a), hash(&b));
}
