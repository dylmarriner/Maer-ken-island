//! The island's water is audited end to end every hour (D29): the change in
//! its soil water and standing water must be what the ledger booked, the
//! hydrology's rain, infiltration, evaporation, drainage and runoff to the
//! sea together with every household flow.

use std::path::PathBuf;
use std::sync::Arc;

use mk_core::canon::CanonLocked;
use mk_engine::hydrology::SoilWater;
use mk_engine::regional::life::{IslandLife, IslandLifeError};
use mk_island::IslandScenario;

fn repo(p: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(p)
}

fn small_island() -> IslandLife {
    let mut scenario =
        IslandScenario::load(&repo("fixtures/island/default_scenario.json")).unwrap();
    scenario.estate_patch.tree_cap = 50;
    scenario.profile = mk_island::IslandProfile::test_small();
    let mut seed = [0u8; 32];
    seed[..4].copy_from_slice(&16u32.to_le_bytes());
    scenario.seed = seed;
    scenario.canon_path = repo("fixtures/island/canon.json");
    scenario.spin_up_years = 0;
    let canon = Arc::new(CanonLocked::load(&repo("fixtures/island/canon.json")).unwrap());
    IslandLife::bootstrap(scenario, canon).unwrap()
}

#[test]
fn every_hour_the_islands_soil_and_standing_water_close_against_the_ledger() {
    let mut life = small_island();
    // The first hour opens the window; every one after it is audited.
    for _ in 0..48 {
        life.advance(3_600).unwrap();
    }
    assert!(
        life.water_audits_closed >= 47,
        "{} hourly water audits closed",
        life.water_audits_closed
    );
}

#[test]
fn water_that_appears_without_a_ledger_entry_fails_the_hour() {
    let mut life = small_island();
    life.advance(3_600).unwrap();
    life.advance(3_600).unwrap();
    // A litre per square metre slipped into one cell's soil, unbooked.
    let (r, c) = life.placed.location;
    let soil = *life.physical.hydrology.soil_water.get(r, c);
    life.physical.hydrology.soil_water.set(
        r,
        c,
        SoilWater {
            storage_mm: soil.storage_mm + 1.0,
            ..soil
        },
    );
    match life.advance(3_600) {
        Err(IslandLifeError::Audit(why)) => assert!(why.contains("SoilWater"), "{why}"),
        other => panic!("the unbooked water passed: {other:?}"),
    }
}
