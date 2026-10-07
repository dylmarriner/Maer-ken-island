//! Phase 4 Task 6 Step 1b: pacing changes how often steps run, never what
//! they do.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use mk_core::canon::CanonLocked;
use mk_engine::regional::commands::SimSpeed;
use mk_engine::regional::pacing::{Pacer, FLAT_OUT_BATCH, MAX_CATCH_UP_STEPS};
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
fn real_time_runs_one_simulated_second_per_real_second_and_carries_remainders() {
    let mut p = Pacer::new(60);
    let mut steps = 0;
    for _ in 0..600 {
        steps += p.steps_due(Duration::from_secs(1), SimSpeed::RealTime);
    }
    assert_eq!(steps, 10, "600 real seconds = 10 sixty-second steps");
    let mut p = Pacer::new(60);
    let mut steps = 0;
    for _ in 0..1_200 {
        steps += p.steps_due(Duration::from_millis(500), SimSpeed::RealTime);
    }
    assert_eq!(steps, 10, "fractions of a second are carried, not lost");
}

#[test]
fn offered_speeds_scale_and_flat_out_runs_a_batch() {
    for k in [10u32, 60, 360, 1_440] {
        let mut p = Pacer::new(60);
        let steps: u64 = (0..60)
            .map(|_| p.steps_due(Duration::from_secs(1), SimSpeed::Times(k)))
            .sum();
        assert_eq!(
            steps,
            u64::from(k),
            "{k}x for a real minute = {k} minutes simulated"
        );
    }
    let mut p = Pacer::new(60);
    assert_eq!(
        p.steps_due(Duration::ZERO, SimSpeed::AsFastAsPossible),
        FLAT_OUT_BATCH
    );
    assert!(!p.is_behind());
}

#[test]
fn a_long_stall_drops_its_debt_and_the_readout_shows_the_machine_is_behind() {
    let mut p = Pacer::new(60);
    let due = p.steps_due(Duration::from_secs(3_600), SimSpeed::Times(1_440));
    assert_eq!(due, MAX_CATCH_UP_STEPS, "bounded, not 86,400 steps");
    assert_eq!(
        p.steps_due(Duration::ZERO, SimSpeed::Times(1_440)),
        0,
        "debt dropped, no spiral"
    );
    // 240 steps (4 simulated hours) took 60 real seconds: 240 x real time.
    p.record(due, Duration::from_secs(60));
    assert!((p.achieved_speed() - 240.0).abs() < 1e-9);
    assert!(p.is_behind(), "asked for 1,440x, achieved 240x");
    let mut fast = Pacer::new(60);
    fast.steps_due(Duration::from_secs(1), SimSpeed::Times(10));
    fast.record(1, Duration::from_millis(10)); // 6,000x
    assert!(!fast.is_behind());
}

#[test]
fn the_same_1000_steps_hash_identically_at_every_speed() {
    let run = |speed: SimSpeed, tick_real: Duration| {
        let mut w = world();
        let mut p = Pacer::new(60);
        while w.tick() < 1_000 {
            let due = p.steps_due(tick_real, speed).min(1_000 - w.tick());
            for _ in 0..due {
                w.step(60).unwrap();
            }
        }
        w.state_hash().unwrap()
    };
    let real = run(SimSpeed::RealTime, Duration::from_secs(60)); // a fake clock
    let fast = run(SimSpeed::Times(1_440), Duration::from_millis(41));
    let flat = run(SimSpeed::AsFastAsPossible, Duration::ZERO);
    assert_eq!(real, fast);
    assert_eq!(real, flat);
}
