//! How much of the human step is `formal_predictive_processing::step()`, whose
//! sampled action upstream documents as read nowhere but its own tests.
//! Run: `cargo test --release -p mk_engine --features construction-counters
//! --test island_dead_compute -- --ignored --nocapture`.
#![cfg(feature = "construction-counters")]

use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Instant;

use mk_core::canon::CanonLocked;
use mk_core::human::BiologicalSex;
use mk_engine::humans::lifecycle::FORMAL_PREDICTIVE_NANOS;
use mk_engine::regional::commands::{IslandCommand, IslandCreateHumanRequest};
use mk_engine::regional::world::IslandWorldState;
use mk_island::IslandScenario;

fn repo(p: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(p)
}

#[test]
#[ignore = "profiling: prints the share of the human step"]
fn the_share_of_the_human_step_spent_in_formal_predictive_processing() {
    let scenario = IslandScenario::load(&repo("fixtures/island/default_scenario.json")).unwrap();
    let canon = Arc::new(CanonLocked::load(&repo("fixtures/island/canon.json")).unwrap());
    let mut w = IslandWorldState::new(canon, scenario).unwrap();
    for i in 0..400u32 {
        w.queue(IslandCommand::CreateHuman(IslandCreateHumanRequest {
            agent_id: format!("Settler-{i:04}"),
            sex: if i % 2 == 0 {
                BiologicalSex::Female
            } else {
                BiologicalSex::Male
            },
            age_years: 20.0 + f64::from(i % 30),
            appearance: None,
            location: None,
        }))
        .unwrap();
    }
    w.step(60).unwrap(); // the arrivals
    FORMAL_PREDICTIVE_NANOS.store(0, Ordering::Relaxed);
    let before = w.life.timings;
    let started = Instant::now();
    for _ in 0..120 {
        w.step(60).unwrap();
    }
    let wall = started.elapsed();
    let humans = w.life.timings.humans - before.humans;
    let fpp = std::time::Duration::from_nanos(FORMAL_PREDICTIVE_NANOS.load(Ordering::Relaxed));
    println!(
        "DEADCOMPUTE people={} steps=120 wall={:.2}s humans_step={:.2}s fpp={:.3}s ({:.1}% of the human step, {:.1}% of wall)",
        w.life.humans.registry.iter().count(),
        wall.as_secs_f64(),
        humans.as_secs_f64(),
        fpp.as_secs_f64(),
        100.0 * fpp.as_secs_f64() / humans.as_secs_f64(),
        100.0 * fpp.as_secs_f64() / wall.as_secs_f64(),
    );
}
