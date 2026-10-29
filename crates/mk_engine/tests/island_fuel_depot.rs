//! The island's oil chain keeps the estate's diesel topped up (Phase 4b
//! Tasks 4-5, D16): diesel burned is replaced by the refinery's
//! distillate through the depot, the island-made share of the store is
//! material carbon, and every step's books close.

use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use mk_core::canon::CanonLocked;
use mk_core::flux::Ledger;
use mk_engine::regional::boundary::sample_regional_boundaries;
use mk_engine::regional::energy::FuelKind;
use mk_engine::regional::geophysics::bootstrap_regional_geophysics;
use mk_engine::regional::industry::chain::{ChainModels, IndustryChain};
use mk_engine::regional::life::IslandLife;
use mk_island::{IslandDomain, IslandProfile, IslandScenario};

fn repo(p: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(p)
}

/// The owner's island's oil fields, surveyed once.
fn owners_fields() -> &'static IndustryChain {
    static CHAIN: OnceLock<IndustryChain> = OnceLock::new();
    CHAIN.get_or_init(|| {
        let canon = Arc::new(CanonLocked::load(&repo("fixtures/island/canon.json")).unwrap());
        let profile = IslandProfile::load(&repo("fixtures/island/default_profile.json")).unwrap();
        let seed = profile.seed_bytes().unwrap();
        let domain = IslandDomain::from_profile(profile).unwrap();
        let boundaries = sample_regional_boundaries(seed, &canon, &domain, 0.0);
        let geophysics = bootstrap_regional_geophysics(&canon, &domain, &boundaries, seed).unwrap();
        let chain = IndustryChain::survey(
            &geophysics.deposits,
            &ChainModels::load_default().unwrap(),
            &seed,
        );
        assert!(
            !chain.fields.is_empty(),
            "the owner's island has oil fields"
        );
        chain
    })
}

fn small_island() -> IslandLife {
    let mut scenario =
        IslandScenario::load(&repo("fixtures/island/default_scenario.json")).unwrap();
    scenario.estate_patch.tree_cap = 50;
    scenario.profile = IslandProfile::test_small();
    let mut seed = [0u8; 32];
    seed[..4].copy_from_slice(&16u32.to_le_bytes());
    scenario.seed = seed;
    scenario.canon_path = repo("fixtures/island/canon.json");
    scenario.spin_up_years = 0;
    let canon = Arc::new(CanonLocked::load(&repo("fixtures/island/canon.json")).unwrap());
    IslandLife::bootstrap(scenario, canon).unwrap()
}

/// Run every diesel machine on the estate until `litres` have left the
/// store through their tanks.
fn burn_diesel(life: &mut IslandLife, litres: f64) {
    let machines: Vec<u64> = life
        .energy
        .vehicles
        .iter()
        .filter(|v| v.fuel == FuelKind::Diesel)
        .map(|v| v.item_id)
        .collect();
    let mut left = litres;
    let mut ledger = Ledger::new();
    for id in machines {
        if left <= 0.0 {
            break;
        }
        let moved = life.energy.refuel(id, left);
        left -= moved;
        let per_hour = life.energy.litres_per_hour(id).unwrap();
        life.energy
            .run_engine_hours(id, 1.0e6 / per_hour.max(1e-9), &mut ledger);
    }
    assert!(left < litres, "some diesel left the store");
}

#[test]
fn the_depot_refills_burned_diesel_from_the_islands_refinery_and_the_books_close() {
    let mut life = small_island();
    life.industry = Some(owners_fields().clone());
    // Vehicles start with partly filled tanks drawn from the estate store.
    // The depot replenishes to the scenario's configured store level, not
    // the already-depleted amount left in the store after bootstrap.
    let target = life.scenario.estate_energy.diesel_litres;
    let initial = life.energy.stored_litres(FuelKind::Diesel);
    assert!(initial > 0.0 && initial <= target);
    assert_eq!(life.energy.island_fuel_kgc(), 0.0);

    burn_diesel(&mut life, 0.5 * initial);
    let low = life.energy.stored_litres(FuelKind::Diesel);
    assert!(low < initial);

    let audits = life.audits_closed;
    for _ in 0..48 {
        life.advance(3_600).unwrap();
    }
    assert!(life.audits_closed > audits, "the chain's steps are audited");
    let now = life.energy.stored_litres(FuelKind::Diesel);
    assert!(
        (now - target).abs() < 1e-6 * target,
        "the depot tops the store back up: {low} L then {now} L, {target} L wanted"
    );
    let store = life
        .energy
        .fuel_stores
        .iter()
        .find(|s| s.fuel == FuelKind::Diesel)
        .unwrap();
    let delivered = target - low;
    assert!(
        ((store.island.litres / store.litres) - delivered / target).abs() < 1e-6,
        "the island-made share is what the depot delivered: {}",
        (store.island.litres / store.litres)
    );
    assert!(
        store.island.kgc > 0.0,
        "the refinery supplied real material carbon"
    );
    assert!(
        (life.energy.island_fuel_kgc() - store.island.kgc).abs() < 1e-6 * store.island.kgc,
        "carbon in the estate store must equal the audited island-made fuel"
    );

    // The chain and the fuel survive a save.
    let restored = IslandLife::restore(life.canon.clone(), life.snapshot()).unwrap();
    assert_eq!(restored.state_digest(), life.state_digest());
}

#[test]
fn an_island_without_oil_delivers_nothing() {
    let mut life = small_island();
    life.industry = Some(IndustryChain::default());
    let full = life.energy.stored_litres(FuelKind::Diesel);
    burn_diesel(&mut life, 0.5 * full);
    let low = life.energy.stored_litres(FuelKind::Diesel);
    for _ in 0..24 {
        life.advance(3_600).unwrap();
    }
    assert_eq!(life.energy.stored_litres(FuelKind::Diesel), low);
    assert_eq!(life.energy.island_fuel_kgc(), 0.0);
}
