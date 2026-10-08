//! What it costs to republish the island's individual stems and its two
//! vegetation renders, and what it costs to answer a box of stems.
//!
//! This refresh is nothing like the others the sim loop makes. The whole
//! conversation feed is forty short strings; this is every
//! `TreeInstance` the island holds -- about 200,000 of them at 56 bytes
//! apiece on the default scenario -- plus a 1,152,000-cell image of the
//! island's biomass and a 640,000-cell one of the estate's patch, both
//! PNG-encoded. That is why they have their own cadence, `TREES_EVERY`, a
//! simulated day, rather than riding the views' hourly one, and this
//! measures the cost that justifies the separation instead of asserting
//! it.
//!
//! Read the result the way `conversation_cost.rs` learned to: a share of
//! a *full* island's step is not a share of any step. The sim loop runs
//! at `AsFastAsPossible`, where a step on a small island is microseconds
//! and a fixed cost stops being a rounding error. The number to watch is
//! not "is it small" but "has it changed shape" -- if the per-answer cost
//! starts growing with the island rather than with the stems in the box,
//! that is a scan that has stopped being bounded.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use island::serve::projection::{self, IslandProjection, Terrain};

fn repo(path: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path)
}

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
fn slow_republishing_the_stems_costs_this_share_of_a_step() {
    const STEP_SECONDS: u64 = 60;
    const STEPS: u64 = 60;

    let mut life = island();
    // Warm: the first step touches a quarter of a million stems for the
    // first time, and that allocation is not what is being compared.
    life.advance(STEP_SECONDS).expect("the island steps on");

    let stepping = Instant::now();
    for _ in 0..STEPS {
        life.advance(STEP_SECONDS).expect("the island steps on");
    }
    let per_step_us = stepping.elapsed().as_secs_f64() * 1e6 / STEPS as f64;

    let copying = Instant::now();
    for _ in 0..STEPS {
        let _ = IslandProjection::trees_now(&life);
    }
    let per_copy_us = copying.elapsed().as_secs_f64() * 1e6 / STEPS as f64;

    let stems = IslandProjection::trees_now(&life);
    let terrain = Terrain::of(&life);

    // The two ends of the viewer: the whole domain thinned to the cap,
    // and a forty-metre square where nothing is thinned at all.
    let (cx, cy) = terrain.individual_centre_m;
    let answering = Instant::now();
    for _ in 0..STEPS {
        let _ = projection::trees_in(
            &terrain,
            &stems,
            f64::MIN,
            f64::MIN,
            f64::MAX,
            f64::MAX,
            4_000,
        );
    }
    let per_wide_us = answering.elapsed().as_secs_f64() * 1e6 / STEPS as f64;

    let close = Instant::now();
    for _ in 0..STEPS {
        let _ = projection::trees_in(
            &terrain,
            &stems,
            cx - 20.0,
            cy - 20.0,
            cx + 20.0,
            cy + 20.0,
            4_000,
        );
    }
    let per_close_us = close.elapsed().as_secs_f64() * 1e6 / STEPS as f64;

    // The two vegetation renders ride the same cadence, and they are the
    // other half of what it has to pay for: a 1,152,000-cell image and a
    // 640,000-cell one, each PNG-encoded. A cadence justified only by the
    // stem copy would be justified by half the bill.
    let drawing = Instant::now();
    for _ in 0..STEPS {
        let _ =
            island_preview::biomass_image(&life.domain, &life.ecology, &life.physical.geophysics)
                .png();
    }
    let per_island_png_us = drawing.elapsed().as_secs_f64() * 1e6 / STEPS as f64;

    let drawing = Instant::now();
    for _ in 0..STEPS {
        let _ = island_preview::stand_image(&life.vegetation).png();
    }
    let per_patch_png_us = drawing.elapsed().as_secs_f64() * 1e6 / STEPS as f64;

    let per_refresh_us = per_copy_us + per_island_png_us + per_patch_png_us;
    let share = 100.0 * per_refresh_us / per_step_us;
    // `TREES_EVERY` is a simulated day, and it is private to the sim
    // loop, so the cadence is derived from the day here rather than
    // reached for. At the sixty-second step this suite and the dashboard
    // both use, that is the same 1,440 steps.
    let steps_per_day = 86_400.0 / STEP_SECONDS as f64;
    let amortised = share / steps_per_day;
    println!("\n== Republishing the island's stems ==\n");
    println!("stems held                : {}", stems.len());
    println!("one step                  : {per_step_us:.0} us");
    println!("one stem copy             : {per_copy_us:.0} us");
    println!("island vegetation png     : {per_island_png_us:.0} us");
    println!("patch stand png           : {per_patch_png_us:.0} us");
    println!("the whole refresh         : {per_refresh_us:.0} us");
    println!("share of a step           : {share:.2}%");
    println!("share, over {steps_per_day:.0} steps  : {amortised:.4}%");
    println!("/api/trees, whole island  : {per_wide_us:.0} us");
    println!("/api/trees, 40 m square   : {per_close_us:.0} us");

    // The bar is on the cadence rather than the work. The refresh is
    // allowed to be expensive -- that is the fact this test exists to
    // record -- but amortised over `TREES_EVERY` steps it has to disappear
    // into the noise, or the separate cadence has not bought anything.
    assert!(
        amortised < 1.0,
        "the stems-and-vegetation refresh is {amortised:.4}% of a full island's step once \
         amortised over a simulated day ({per_refresh_us:.0} us against {per_step_us:.0} us, \
         every 1,440 steps); the whole reason it has its own cadence is that this stays \
         negligible"
    );

    // A request must not be able to hold up a step by being slow: the
    // handler runs on the HTTP runtime, but it walks the same stem list
    // the loop does, and an answer costing more than a step means a busy
    // page competing with the island for the same cache.
    assert!(
        per_wide_us < per_step_us,
        "answering /api/trees for the whole island takes {per_wide_us:.0} us against a step's \
         {per_step_us:.0} us; it is a filtered walk of the stems and should be well under one"
    );
}
