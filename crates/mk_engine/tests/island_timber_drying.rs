//! Felled timber dries in the estate's air (WORK_REMAINING.md §1a, D40):
//! the log's water leaves for the atmosphere through the audited books,
//! toward the wood's equilibrium for that air and no further.

use std::path::PathBuf;
use std::sync::Arc;

use mk_core::canon::CanonLocked;
use mk_core::flux::Ledger;
use mk_engine::regional::drying::WoodDrying;
use mk_engine::regional::life::IslandLife;
use mk_engine::regional::timber::{felled_log, LogPlace};
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
fn a_stored_green_log_dries_toward_the_air_and_the_books_close() {
    let mut life = small_island();
    // A 30 cm, 4 m hardwood log, green at the packs' 0.82.
    let mut log = felled_log(0, (0.0, 0.0), 0.0, 0.3, 4.0, 160.0, 80.0, 0.82, 0.6);
    log.place = LogPlace::Stored;
    let id = life.materials.receive_felled_log(log, &mut Ledger::new());
    let start = life.materials.log(id).unwrap().clone();
    let audits = life.audits_closed;

    // Ten local days of the island's weather, every hour audited.
    for _ in 0..(10 * 36) {
        life.advance(3_600).unwrap();
    }
    let now = life.materials.log(id).unwrap();
    assert!(life.audits_closed > audits + 300, "hourly drying audited");
    assert!(
        now.water_kg < start.water_kg,
        "{} kg of water then, {} now",
        start.water_kg,
        now.water_kg
    );
    // Wood and carbon stay; only water leaves.
    assert_eq!(now.dry_kg, start.dry_kg);
    assert_eq!(now.carbon_kgc, start.carbon_kgc);
    // A thick round log dries slowly: ten days take a few percent of its
    // water, nowhere near air-dry.
    let drying = WoodDrying::from_reference(
        &mk_engine::validation::ReferenceLibrary::load(
            &mk_engine::validation::default_reference_dir(),
        )
        .unwrap(),
    )
    .unwrap();
    assert!(now.moisture_content() > drying.equilibrium_moisture(20.0, 0.6) + 0.3);
}
