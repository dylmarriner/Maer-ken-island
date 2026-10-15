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
    let (cell, area) = (
        life.placed.location,
        life.domain.cell_area_m2(mk_island::DomainLevel::Medium),
    );
    let id = life.materials.receive_felled_log(
        log,
        &mut life.physical.hydrology,
        cell,
        area,
        &mut Ledger::new(),
    );
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

/// A stored log dry enough to be the stock's air-dry wood joins the stock:
/// its carbon arrives exactly, the log is gone, and the books close; a
/// log still on the ground, however dry, stays a log.
#[test]
fn a_seasoned_stored_log_joins_the_wood_stock() {
    use mk_engine::regional::materials::{Material, MaterialLedger};
    let mut life = small_island();
    let air_dry = MaterialLedger::air_dry_moisture_content();
    assert!((air_dry - 0.15 / 0.85).abs() < 1e-12);
    let mut ledger = Ledger::new();
    let mut seasoned = felled_log(0, (0.0, 0.0), 0.0, 0.1, 2.0, 20.0, 10.0, air_dry, 0.6);
    seasoned.place = LogPlace::Stored;
    let mut on_ground = seasoned.clone();
    on_ground.place = LogPlace::Ground;
    let (cell, area) = (
        life.placed.location,
        life.domain.cell_area_m2(mk_island::DomainLevel::Medium),
    );
    let stored = life.materials.receive_felled_log(
        seasoned,
        &mut life.physical.hydrology,
        cell,
        area,
        &mut ledger,
    );
    let ground = life.materials.receive_felled_log(
        on_ground,
        &mut life.physical.hydrology,
        cell,
        area,
        &mut ledger,
    );
    let wood_before = life.materials.stock_kg(Material::Wood);
    let audits = life.audits_closed;

    life.advance(3_600).unwrap();

    assert!(life.audits_closed > audits);
    assert!(
        life.materials.log(stored).is_none(),
        "the seasoned log left"
    );
    assert!(
        life.materials.log(ground).is_some(),
        "the one outside stayed"
    );
    let added = life.materials.stock_kg(Material::Wood) - wood_before;
    assert!((added * 0.425 - 10.0).abs() < 1e-9, "{added} kg of wood");
}
