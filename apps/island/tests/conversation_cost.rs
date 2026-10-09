//! What it costs to publish the island's conversations on every step.
//!
//! The copy is bounded by `CONVERSATIONS_SHOWN` rather than by the
//! population, so it costs about the same whatever the island holds. That
//! is the useful fact about it, and this measures it rather than asserting
//! it.
//!
//! Read the result carefully, because an earlier version of this change did
//! not. A share of a *full* island's step is not a share of any step: the
//! sim loop runs at `AsFastAsPossible`, where a step on a small island is
//! microseconds and this fixed cost stops being a rounding error. That is
//! what sank publishing conversations on every step — across the serve
//! suite's concurrent sim threads it starved the HTTP runtime until a
//! command missed its sixty-second deadline. They now ride the digest's
//! cadence, with the records of the people having them.
//!
//! So what this number is good for is noticing if the copy ever stops
//! being bounded — if it starts scaling with the population the way the
//! per-human record copy does, this is where that shows up.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use island::serve::projection::IslandProjection;

fn repo(path: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path)
}

/// The real island, at the full stem count: this is measuring a share of a
/// real step, so a step shrunk to fifty trees would flatter the result.
fn island() -> mk_engine::regional::life::IslandLife {
    let scenario =
        mk_island::IslandScenario::load(&repo("fixtures/island/default_scenario.json")).unwrap();
    let canon =
        Arc::new(mk_core::canon::CanonLocked::load(&repo("fixtures/island/canon.json")).unwrap());
    mk_engine::regional::life::IslandLife::bootstrap(scenario, canon)
        .expect("the island bootstraps")
}

#[test]
#[ignore = "slow: bootstraps the full island and steps it"]
fn slow_publishing_conversations_costs_this_share_of_a_step() {
    const STEP_SECONDS: u64 = 60;
    const STEPS: u64 = 120;

    let mut life = island();
    // Warm: the first step touches a quarter of a million stems for the
    // first time, and that allocation is not what is being compared. The
    // human-population benchmark learned this the hard way.
    life.advance(STEP_SECONDS).expect("the island steps on");

    let stepping = Instant::now();
    for _ in 0..STEPS {
        life.advance(STEP_SECONDS).expect("the island steps on");
    }
    let per_step_us = stepping.elapsed().as_secs_f64() * 1e6 / STEPS as f64;

    let publishing = Instant::now();
    for _ in 0..STEPS {
        let _ = IslandProjection::conversations_now(&life);
    }
    let per_publish_us = publishing.elapsed().as_secs_f64() * 1e6 / STEPS as f64;

    let share = 100.0 * per_publish_us / per_step_us;
    println!("\n== Publishing the island's conversations ==\n");
    println!("one step                  : {per_step_us:.0} us");
    println!("one conversation publish  : {per_publish_us:.1} us");
    println!("share of a step           : {share:.2}%");
    println!(
        "conversations held        : {}",
        IslandProjection::conversations_now(&life).len()
    );

    // A loose bar, guarding the property that matters: this copy is sized
    // by the conversations shown, not by the island or its population, so
    // it should stay small against a full island's step however large that
    // island grows. If this ever trips, the copy has started scaling with
    // something it should not.
    assert!(
        share < 5.0,
        "publishing conversations is {share:.2}% of a full island's step ({per_publish_us:.1} us \
         against {per_step_us:.0} us); it is supposed to be bounded by the conversations shown \
         rather than by the world, so something has started scaling with the island"
    );
}
