//! The oil chain runs on the island (WORK_REMAINING.md §4b-4c, D16, D58):
//! the fields produce, the refinery distils their crude, and the depot
//! tops the estate's diesel up from the refinery's distillate. Every step
//! is audited with the island's materials, and the chain is saved and
//! restored with the island.

use std::path::PathBuf;
use std::sync::Arc;

use mk_core::canon::CanonLocked;
use mk_engine::regional::energy::FuelKind;
use mk_engine::regional::industry::petroleum::{PetroleumField, PetroleumModel};
use mk_engine::regional::life::IslandLife;
use mk_island::IslandScenario;

fn repo(p: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(p)
}

fn canon() -> Arc<CanonLocked> {
    Arc::new(CanonLocked::load(&repo("fixtures/island/canon.json")).unwrap())
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
    IslandLife::bootstrap(scenario, canon()).unwrap()
}

/// The small test island may have no petroleum; give it a field the size
/// of the packs' first New Zealand analogue so the chain has crude to run.
fn with_a_field(life: &mut IslandLife) {
    let chain = life.industry.as_mut().expect("the chain is installed");
    if !chain.fields.is_empty() {
        return;
    }
    let model = PetroleumModel::load_default().unwrap();
    let analogue = &model.analogues[0];
    chain.fields.push(PetroleumField {
        deposit_id: 9_999,
        cell: (0, 0),
        analogue: analogue.name.clone(),
        ultimate_oil_m3: analogue.ultimate_oil_m3,
        ultimate_gas_gj: analogue.ultimate_gas_gj,
        produced_oil_m3: 0.0,
        produced_gas_gj: 0.0,
        tank_oil_m3: 0.0,
        tank_gas_gj: 0.0,
        leaked_kgc: 0.0,
        wells: 0,
        blowout_expectation: 0.0,
        blowouts: 0,
        produced_water_m3: 0.0,
    });
}

#[test]
fn the_chain_refines_and_the_depot_tops_the_estate_diesel_up_with_audited_books() {
    let mut life = small_island();
    with_a_field(&mut life);
    let full = life.scenario.estate_energy.diesel_litres;
    // Most of the estate's diesel has been used.
    for store in &mut life.energy.fuel_stores {
        if store.fuel == FuelKind::Diesel {
            store.litres = 0.25 * full;
        }
    }
    let audits = life.audits_closed;
    for _ in 0..48 {
        life.advance(3_600).unwrap();
    }
    assert!(
        life.audits_closed > audits,
        "the chain's steps were audited"
    );
    let chain = life.industry.as_ref().unwrap();
    assert!(chain.refined_m3 > 0.0, "crude was refined");
    let diesel = life
        .energy
        .fuel_stores
        .iter()
        .find(|s| s.fuel == FuelKind::Diesel)
        .unwrap();
    // The depot fills the store back to the scenario's starting fill; the
    // generator may since have burned a little of it.
    assert!(
        diesel.litres <= full + 1e-6,
        "{} of {full} L",
        diesel.litres
    );
    assert!(diesel.litres > 0.9 * full, "{} of {full} L", diesel.litres);
    assert!(diesel.island.litres > 0.0 && diesel.island.litres <= diesel.litres);
    assert!(diesel.island.kgc > 0.0);
    // Island diesel carries close to diesel's carbon per litre: the
    // refinery's IPCC gas-oil carbon against the estate's C12H23.
    let per_litre = diesel.island.kgc / diesel.island.litres;
    let bought = FuelKind::Diesel.carbon_kg_per_litre();
    assert!(
        (per_litre / bought - 1.0).abs() < 0.1,
        "{per_litre} kg C/L against {bought}"
    );
}

#[test]
fn the_chain_is_saved_and_restored_with_the_island() {
    let mut life = small_island();
    with_a_field(&mut life);
    for _ in 0..6 {
        life.advance(3_600).unwrap();
    }
    let digest = life.state_digest();
    let json = serde_json::to_string(&life.snapshot()).unwrap();
    let restored = IslandLife::restore(canon(), serde_json::from_str(&json).unwrap()).unwrap();
    assert_eq!(restored.industry, life.industry);
    assert_eq!(restored.state_digest(), digest);
}

#[test]
fn an_island_saved_before_the_chain_ran_surveys_it_on_restore() {
    let life = small_island();
    let mut snapshot = life.snapshot();
    snapshot.industry = None;
    let mut json = serde_json::to_value(&snapshot).unwrap();
    json.as_object_mut().unwrap().remove("industry");
    let restored = IslandLife::restore(canon(), serde_json::from_value(json).unwrap()).unwrap();
    assert_eq!(restored.industry, life.industry);
}
