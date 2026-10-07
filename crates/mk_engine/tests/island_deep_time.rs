//! Phase 4 Task 2b groundwork: what a deep-time spin-up would have to start
//! from. The regional ecology is bootstrapped at its own fixed point
//! (biomass = NPP x residence time), so under steady climate nothing drifts
//! over centuries. A spin-up that changes anything needs models the island
//! does not have yet (soil carbon, erosion, tectonics); see the plan.

use std::path::PathBuf;
use std::sync::Arc;

use mk_core::canon::CanonLocked;
use mk_engine::regional::life::IslandLife;
use mk_island::IslandScenario;

fn repo(p: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(p)
}

const SECONDS_PER_YEAR: f64 = 365.25 * 86_400.0;

#[test]
fn island_biomass_starts_at_its_equilibrium_and_does_not_drift_over_centuries() {
    let scenario = IslandScenario::load(&repo("fixtures/island/default_scenario.json")).unwrap();
    let canon = Arc::new(CanonLocked::load(&repo("fixtures/island/canon.json")).unwrap());
    let life = IslandLife::bootstrap(scenario, canon).unwrap();
    let start = life.ecology.biomass_kgc_m2.data().to_vec();
    let total = |v: &[f64]| v.iter().sum::<f64>();
    assert!(total(&start) > 0.0, "the island has producers");

    // One century in one step, and in a hundred yearly steps: both stay put.
    let mut one = life.ecology.clone();
    one.step(100.0 * SECONDS_PER_YEAR, None, None);
    let mut many = life.ecology.clone();
    for _ in 0..100 {
        many.step(SECONDS_PER_YEAR, None, None);
    }
    for (name, state) in [("one 100-year step", &one), ("100 yearly steps", &many)] {
        let drift = (total(state.biomass_kgc_m2.data()) - total(&start)).abs() / total(&start);
        assert!(drift < 1e-9, "{name}: total biomass drifted by {drift:e}");
        let worst = start
            .iter()
            .zip(state.biomass_kgc_m2.data())
            .filter(|(a, _)| **a > 0.0)
            .map(|(a, b)| ((a - b) / a).abs())
            .fold(0.0, f64::max);
        assert!(worst < 1e-9, "{name}: a cell drifted by {worst:e}");
    }
}
