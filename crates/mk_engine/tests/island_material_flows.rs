//! Phase 3 Task 3: gathered materials move real carbon, oxygen and water.

use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use mk_core::biomes::BiomeType;
use mk_core::canon::CanonLocked;
use mk_core::flux::{FluxKind, Ledger, Reservoir, Reservoir as R};
use mk_engine::conservation::O2_PER_CARBON;
use mk_engine::hydrology::HydrologyState;
use mk_engine::regional::ecology::RegionalEcologyState;
use mk_engine::regional::edible::EdibleFood;
use mk_engine::regional::materials::{
    composition, respired_carbon_kg_per_kcal, AnimalPopulation, Material, MaterialError,
    MaterialLedger, ALL_MATERIALS, QUICKLIME_PER_LIMESTONE,
};
use mk_engine::regional::physical::RegionalPhysicalState;
use mk_engine::regional::timber::{Log, LogPlace, MAX_CROSS_CUT_PIECES};
use mk_island::{DomainLevel, IslandDomain, IslandProfile};

const M: DomainLevel = DomainLevel::Medium;

struct Base {
    domain: IslandDomain,
    ecology: RegionalEcologyState,
    forest_cell: (usize, usize),
}

fn repo(path: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path)
}

fn base() -> &'static Base {
    static BASE: OnceLock<Base> = OnceLock::new();
    BASE.get_or_init(|| {
        let canon = Arc::new(CanonLocked::load(&repo("fixtures/island/canon.json")).unwrap());
        let profile = IslandProfile::load(&repo("fixtures/island/default_profile.json")).unwrap();
        let seed = profile.seed_bytes().unwrap();
        let domain = IslandDomain::from_profile(profile).unwrap();
        let physical = RegionalPhysicalState::bootstrap(&canon, &domain, seed).unwrap();
        let ecology = RegionalEcologyState::bootstrap(&domain, &physical).unwrap();
        let (rows, cols) = (domain.rows(M), domain.cols(M));
        let forest_cell = (0..rows)
            .flat_map(|r| (0..cols).map(move |c| (r, c)))
            .find(|&(r, c)| {
                matches!(
                    ecology.biome_grid.get(r, c),
                    BiomeType::TemperateForest | BiomeType::Woodland | BiomeType::MontaneForest
                ) && *ecology.biomass_kgc_m2.get(r, c) > 1.0
            })
            .expect("a forest cell");
        Base {
            domain,
            ecology,
            forest_cell,
        }
    })
}

/// Net flow of `kind` into `reservoir` over the ledger.
fn net(ledger: &Ledger, reservoir: Reservoir, kind: FluxKind) -> f64 {
    ledger
        .entries()
        .iter()
        .filter(|e| e.kind == kind)
        .map(|e| {
            (if e.sink == reservoir { e.amount } else { 0.0 })
                - (if e.source == reservoir { e.amount } else { 0.0 })
        })
        .sum()
}

/// The stock audit: every reservoir's change equals the ledger's net flow.
struct Audit {
    before: [f64; 3],
}

impl Audit {
    fn start(m: &MaterialLedger) -> Self {
        Self {
            before: [
                m.stock_of(R::MaterialCarbon),
                m.stock_of(R::HumanCarbon),
                m.stock_of(R::MaterialWater),
            ],
        }
    }
    fn close(&self, m: &MaterialLedger, ledger: &Ledger) {
        let after = [
            m.stock_of(R::MaterialCarbon),
            m.stock_of(R::HumanCarbon),
            m.stock_of(R::MaterialWater),
        ];
        for (i, (reservoir, kind)) in [
            (R::MaterialCarbon, FluxKind::Carbon),
            (R::HumanCarbon, FluxKind::Carbon),
            (R::MaterialWater, FluxKind::Water),
        ]
        .into_iter()
        .enumerate()
        {
            let delta = after[i] - self.before[i];
            let flow = net(ledger, reservoir, kind);
            assert!(
                (delta - flow).abs() <= 1e-9 * (1.0 + delta.abs()),
                "{reservoir:?}: stock changed {delta}, ledger says {flow}"
            );
        }
    }
}

#[test]
fn every_material_has_a_row_and_limestone_is_calcium_carbonate() {
    for m in ALL_MATERIALS {
        let c = composition(m);
        assert!((0.0..=1.0).contains(&c.carbon_per_kg) && (0.0..=1.0).contains(&c.water_per_kg));
        assert!(c.carbon_per_kg + c.water_per_kg <= 1.0 + 1e-12, "{m:?}");
        assert!(!c.source.is_empty(), "{m:?} has no cited source");
    }
    assert!((composition(Material::Limestone).carbon_per_kg - 12.011 / 100.087).abs() < 1e-3);
    // Respiration: ~0.35 g CO2 per kcal, so ~0.9 kg CO2 for 2,500 kcal.
    let co2_kg = 2_500.0 * respired_carbon_kg_per_kcal() * 44.009 / 12.011;
    assert!((0.7..1.1).contains(&co2_kg), "{co2_kg} kg CO2/day");
}

#[test]
fn gathering_moves_exactly_the_items_carbon_and_an_empty_cell_yields_nothing_until_npp_regrows_it()
{
    let b = base();
    let area = b.domain.cell_area_m2(M);
    let mut ecology = b.ecology.clone();
    let cell = b.forest_cell;
    let (mut m, mut ledger) = (MaterialLedger::new(), Ledger::new());
    let audit = Audit::start(&m);

    let before = *ecology.biomass_kgc_m2.get(cell.0, cell.1);
    let got = m
        .gather_biotic(Material::Wood, 100.0, &mut ecology, cell, area, &mut ledger)
        .unwrap();
    assert_eq!(got, 100.0);
    let carbon = 100.0 * composition(Material::Wood).carbon_per_kg;
    let after = *ecology.biomass_kgc_m2.get(cell.0, cell.1);
    assert!(
        ((before - after) * area - carbon).abs() < 1e-6,
        "the cell lost {}",
        (before - after) * area
    );
    let entry = ledger
        .entries()
        .iter()
        .find(|e| e.kind == FluxKind::Carbon)
        .unwrap();
    assert_eq!(
        (entry.source, entry.sink),
        (R::BiomassCarbon, R::MaterialCarbon)
    );
    assert!((entry.amount - carbon).abs() < 1e-12);
    audit.close(&m, &ledger);

    // Harvest the cell down to nothing: the request is capped at what is there.
    ecology.biomass_kgc_m2.set(cell.0, cell.1, 1.0e-6);
    let held_kg = 1.0e-6 * area / composition(Material::Wood).carbon_per_kg;
    let got = m
        .gather_biotic(Material::Wood, 1.0e6, &mut ecology, cell, area, &mut ledger)
        .unwrap();
    assert!((got - held_kg).abs() < 1e-9, "took {got} of {held_kg}");
    assert_eq!(*ecology.biomass_kgc_m2.get(cell.0, cell.1), 0.0);
    assert_eq!(
        m.gather_biotic(Material::Wood, 1.0, &mut ecology, cell, area, &mut ledger),
        Err(MaterialError::NothingToHarvest)
    );
    // NPP regrows it, and then it can be gathered again.
    ecology.step(10.0 * 365.25 * 86_400.0, None, None);
    assert!(
        *ecology.biomass_kgc_m2.get(cell.0, cell.1) > 0.0,
        "NPP did not regrow the cell"
    );
    assert!(m
        .gather_biotic(Material::Wood, 10.0, &mut ecology, cell, area, &mut ledger)
        .is_ok());
    audit.close(&m, &ledger);
    // Not every material can be gathered from biomass.
    assert!(matches!(
        m.gather_biotic(Material::Coal, 1.0, &mut ecology, cell, area, &mut ledger),
        Err(MaterialError::WrongMaterial(_))
    ));
}

#[test]
fn burning_coal_releases_its_carbon_and_binds_the_stoichiometric_oxygen() {
    let (mut m, mut ledger) = (MaterialLedger::new(), Ledger::new());
    let audit = Audit::start(&m);
    m.gather_fossil(Material::Coal, 100.0, &mut ledger).unwrap();
    let crust = ledger
        .entries()
        .iter()
        .find(|e| e.kind == FluxKind::Carbon)
        .unwrap();
    assert_eq!(
        (crust.source, crust.sink),
        (R::CrustCarbon, R::MaterialCarbon)
    );
    assert!((crust.amount - 75.0).abs() < 1e-12);
    let kgc = m.burn(Material::Coal, 100.0, &mut ledger).unwrap();
    assert!((kgc - 75.0).abs() < 1e-12);
    assert!(
        (net(&ledger, R::AtmosCO2, FluxKind::Carbon) - 75.0).abs() < 1e-12,
        "CO2 carbon"
    );
    // 2.664 kg of O2 per kg of carbon (C + O2 -> CO2).
    assert!((O2_PER_CARBON - 2.664).abs() < 1e-3);
    assert!(
        (net(&ledger, R::AtmosO2, FluxKind::Oxygen) + 75.0 * O2_PER_CARBON).abs() < 1e-9,
        "O2 consumed"
    );
    assert_eq!(m.stock_kg(Material::Coal), 0.0);
    audit.close(&m, &ledger);
    // Burning what you do not have is refused; a non-fuel cannot burn.
    assert!(matches!(
        m.burn(Material::Coal, 1.0, &mut ledger),
        Err(MaterialError::Insufficient { .. })
    ));
    assert!(matches!(
        m.burn(Material::Water, 1.0, &mut ledger),
        Err(MaterialError::WrongMaterial(_))
    ));
}

#[test]
fn material_withdrawals_reject_negative_and_non_finite_amounts_before_mutation() {
    let (mut materials, mut ledger) = (MaterialLedger::new(), Ledger::new());
    materials
        .bank_mined(Material::Gold, 10.0)
        .expect("gold is a mineral material");
    materials.register_human("Ada", 70.0);

    let gold_before = materials.stock_kg(Material::Gold);
    assert!(matches!(
        materials.bank_mined(Material::Gold, -1.0),
        Err(MaterialError::InvalidAmount {
            material: Material::Gold,
            amount_kg
        }) if amount_kg == -1.0
    ));
    assert!(matches!(
        materials.bank_mined(Material::Gold, f64::INFINITY),
        Err(MaterialError::InvalidAmount {
            material: Material::Gold,
            amount_kg
        }) if amount_kg.is_infinite() && amount_kg.is_sign_positive()
    ));
    let body_carbon_before = materials.stock_of(R::HumanCarbon);
    let body_water_before = materials.stock_of(R::MaterialWater);

    assert!(matches!(
        materials.burn(Material::Coal, -1.0, &mut ledger),
        Err(MaterialError::InvalidAmount {
            material: Material::Coal,
            amount_kg
        }) if amount_kg == -1.0
    ));
    assert!(matches!(
        materials.calcine(f64::NAN, &mut ledger),
        Err(MaterialError::InvalidAmount {
            material: Material::Limestone,
            amount_kg
        }) if amount_kg.is_nan()
    ));
    assert!(matches!(
        materials.make_charcoal(f64::INFINITY, &mut ledger),
        Err(MaterialError::InvalidAmount {
            material: Material::Wood,
            amount_kg
        }) if amount_kg.is_infinite() && amount_kg.is_sign_positive()
    ));
    assert!(matches!(
        materials.eat("Ada", Material::PlantFood, -0.5, &mut ledger),
        Err(MaterialError::InvalidAmount {
            material: Material::PlantFood,
            amount_kg
        }) if amount_kg == -0.5
    ));
    assert!(matches!(
        materials.drink("Ada", f64::NEG_INFINITY),
        Err(MaterialError::InvalidAmount {
            material: Material::Water,
            amount_kg
        }) if amount_kg.is_infinite() && amount_kg.is_sign_negative()
    ));
    assert!(matches!(
        materials.build(Material::Gold, -2.0),
        Err(MaterialError::InvalidAmount {
            material: Material::Gold,
            amount_kg
        }) if amount_kg == -2.0
    ));

    assert_eq!(materials.stock_kg(Material::Gold), gold_before);
    assert_eq!(materials.stock_of(R::HumanCarbon), body_carbon_before);
    assert_eq!(materials.stock_of(R::MaterialWater), body_water_before);
    assert_eq!(materials.structure_count(), 0);
    assert!(ledger.entries().is_empty());
}

#[test]
fn craft_aggregates_duplicate_inputs_and_rejects_invalid_outputs_atomically() {
    let (mut materials, mut ledger) = (MaterialLedger::new(), Ledger::new());
    materials
        .bank_mined(Material::Gold, 6.0)
        .expect("gold is a mineral material");

    let insufficient = materials.craft(
        &[(Material::Gold, 5.0), (Material::Gold, 5.0)],
        &[],
        &mut ledger,
    );
    assert!(matches!(
        insufficient,
        Err(MaterialError::Insufficient {
            material: Material::Gold,
            have_kg,
            need_kg
        }) if have_kg == 6.0 && need_kg == 10.0
    ));
    assert_eq!(materials.stock_kg(Material::Gold), 6.0);
    assert!(ledger.entries().is_empty());

    let invalid_output = materials.craft(
        &[(Material::Gold, 1.0)],
        &[(Material::Gold, -1.0)],
        &mut ledger,
    );
    assert!(matches!(
        invalid_output,
        Err(MaterialError::InvalidAmount {
            material: Material::Gold,
            amount_kg
        }) if amount_kg == -1.0
    ));
    assert_eq!(materials.stock_kg(Material::Gold), 6.0);
    assert!(ledger.entries().is_empty());

    // The same aggregate preflight also permits a valid repeated input when
    // the combined requirement is available; the net stock remains 6 kg.
    materials
        .craft(
            &[(Material::Gold, 2.0), (Material::Gold, 3.0)],
            &[(Material::Gold, 5.0)],
            &mut ledger,
        )
        .expect("the combined 5 kg input is available");
    assert_eq!(materials.stock_kg(Material::Gold), 6.0);
    assert!(ledger.entries().is_empty());
}

#[test]
fn material_withdrawals_reject_positive_amounts_rounded_away_before_mutation() {
    let (mut materials, mut ledger) = (MaterialLedger::new(), Ledger::new());
    materials
        .inject(Material::Wood, f64::MAX, &mut ledger)
        .expect("maximum finite wood stock has representable carbon and water");

    let stock_before = materials.stock_kg(Material::Wood);
    let carbon_before = materials.stock_of(R::MaterialCarbon);
    let entries_before = ledger.entries().len();
    let rejected = materials.craft(&[(Material::Wood, 1.0)], &[], &mut ledger);

    assert!(matches!(
        rejected,
        Err(MaterialError::InvalidAmount {
            material: Material::Wood,
            amount_kg
        }) if amount_kg == f64::MAX
    ));
    assert_eq!(materials.stock_kg(Material::Wood), stock_before);
    assert_eq!(materials.stock_of(R::MaterialCarbon), carbon_before);
    assert_eq!(ledger.entries().len(), entries_before);
}

#[test]
fn craft_rejects_positive_withdrawal_when_stock_is_within_tolerance_but_zero() {
    let (mut materials, mut ledger) = (MaterialLedger::new(), Ledger::new());
    let rejected = materials.craft(&[(Material::Wood, 1e-13)], &[], &mut ledger);

    assert!(matches!(
        rejected,
        Err(MaterialError::Insufficient {
            material: Material::Wood,
            have_kg,
            need_kg
        }) if have_kg == 0.0 && need_kg == 1e-13
    ));
    assert_eq!(materials.stock_kg(Material::Wood), 0.0);
    assert_eq!(materials.stock_of(R::MaterialCarbon), 0.0);
    assert!(ledger.entries().is_empty());
}

#[test]
fn direct_withdrawal_rejects_a_positive_shortfall_inside_legacy_tolerance() {
    let (mut materials, mut ledger) = (MaterialLedger::new(), Ledger::new());
    materials.register_human("Ada", 70.0);
    materials
        .inject(Material::Water, 1e-13, &mut ledger)
        .expect("a positive representable water stock can be seeded");

    let stock_before = materials.stock_kg(Material::Water);
    let water_before = materials.stock_of(R::MaterialWater);
    let rejected = materials.drink("Ada", 2e-13);

    assert!(matches!(
        rejected,
        Err(MaterialError::Insufficient {
            material: Material::Water,
            have_kg,
            need_kg
        }) if have_kg == 1e-13 && need_kg == 2e-13
    ));
    assert_eq!(materials.stock_kg(Material::Water), stock_before);
    assert_eq!(materials.stock_of(R::MaterialWater), water_before);
}

#[test]
fn craft_preflight_rejects_tolerance_shortfall_before_any_input_is_debited() {
    let (mut materials, mut ledger) = (MaterialLedger::new(), Ledger::new());
    materials
        .inject(Material::Wood, 10.0, &mut ledger)
        .expect("wood input stock");
    materials
        .inject(Material::Water, 1e-13, &mut ledger)
        .expect("small positive water input stock");

    let wood_before = materials.stock_kg(Material::Wood);
    let water_before = materials.stock_kg(Material::Water);
    let carbon_before = materials.stock_of(R::MaterialCarbon);
    let material_water_before = materials.stock_of(R::MaterialWater);
    let entries_before = ledger.entries().len();

    let rejected = materials.craft(
        &[(Material::Wood, 1.0), (Material::Water, 2e-13)],
        &[],
        &mut ledger,
    );

    assert!(matches!(
        rejected,
        Err(MaterialError::Insufficient {
            material: Material::Water,
            have_kg,
            need_kg
        }) if have_kg == 1e-13 && need_kg == 2e-13
    ));
    assert_eq!(materials.stock_kg(Material::Wood), wood_before);
    assert_eq!(materials.stock_kg(Material::Water), water_before);
    assert_eq!(materials.stock_of(R::MaterialCarbon), carbon_before);
    assert_eq!(materials.stock_of(R::MaterialWater), material_water_before);
    assert_eq!(ledger.entries().len(), entries_before);
}

#[test]
fn bank_mined_rejects_positive_stock_increment_that_rounds_away() {
    let mut materials = MaterialLedger::new();
    materials
        .bank_mined(Material::Gold, f64::MAX)
        .expect("initial finite mineral stock fits");

    let rejected = materials.bank_mined(Material::Gold, 1.0);
    assert!(matches!(
        rejected,
        Err(MaterialError::InvalidAmount {
            material: Material::Gold,
            amount_kg
        }) if amount_kg == f64::MAX
    ));
    assert_eq!(materials.stock_kg(Material::Gold), f64::MAX);
}

#[test]
fn craft_rejects_output_increment_that_rounds_away_before_debiting_inputs() {
    let (mut materials, mut ledger) = (MaterialLedger::new(), Ledger::new());
    materials
        .bank_mined(Material::Gold, f64::MAX)
        .expect("initial finite mineral stock fits");
    materials
        .inject(Material::Wood, 1.0, &mut ledger)
        .expect("wood input fits");

    let wood_before = materials.stock_kg(Material::Wood);
    let gold_before = materials.stock_kg(Material::Gold);
    let entries_before = ledger.entries().len();
    let rejected = materials.craft(
        &[(Material::Wood, 1.0)],
        &[(Material::Gold, 1.0)],
        &mut ledger,
    );

    assert!(matches!(
        rejected,
        Err(MaterialError::InvalidAmount {
            material: Material::Gold,
            amount_kg
        }) if amount_kg == f64::MAX
    ));
    assert_eq!(materials.stock_kg(Material::Wood), wood_before);
    assert_eq!(materials.stock_kg(Material::Gold), gold_before);
    assert_eq!(ledger.entries().len(), entries_before);
}

#[test]
fn craft_rejects_duplicate_input_increment_that_rounds_away_before_mutation() {
    let (mut materials, mut ledger) = (MaterialLedger::new(), Ledger::new());
    let starting_stock = 2_f64.powi(53);
    materials
        .bank_mined(Material::Gold, starting_stock)
        .expect("the starting stock is representable");

    let rejected = materials.craft(
        &[(Material::Gold, starting_stock), (Material::Gold, 1.0)],
        &[],
        &mut ledger,
    );

    assert!(matches!(
        rejected,
        Err(MaterialError::InvalidAmount {
            material: Material::Gold,
            amount_kg
        }) if amount_kg == starting_stock
    ));
    assert_eq!(materials.stock_kg(Material::Gold), starting_stock);
    assert!(ledger.entries().is_empty());
}

#[test]
fn craft_aggregates_duplicate_outputs_before_mutation() {
    let (mut materials, mut ledger) = (MaterialLedger::new(), Ledger::new());
    let starting_stock = 2_f64.powi(53) + 2.0;
    materials
        .bank_mined(Material::Gold, starting_stock)
        .expect("the starting stock is representable");

    materials
        .craft(
            &[(Material::Gold, 2.0)],
            &[(Material::Gold, 1.0), (Material::Gold, 1.0)],
            &mut ledger,
        )
        .expect("the aggregated two-kilogram output restores the stock");

    assert_eq!(materials.stock_kg(Material::Gold), starting_stock);
    assert!(ledger.entries().is_empty());
}

#[test]
fn material_stock_additions_reject_overflow_before_mutation() {
    let (mut materials, mut ledger) = (MaterialLedger::new(), Ledger::new());
    materials
        .bank_mined(Material::Gold, f64::MAX)
        .expect("the first finite gold stock fits");

    let overflow = materials.bank_mined(Material::Gold, f64::MAX);
    assert!(matches!(
        overflow,
        Err(MaterialError::InvalidAmount {
            material: Material::Gold,
            amount_kg
        }) if amount_kg.is_infinite()
    ));
    assert_eq!(materials.stock_kg(Material::Gold), f64::MAX);

    let craft_overflow = materials.craft(&[], &[(Material::Gold, f64::MAX)], &mut ledger);
    assert!(matches!(
        craft_overflow,
        Err(MaterialError::InvalidAmount {
            material: Material::Gold,
            amount_kg
        }) if amount_kg.is_infinite()
    ));
    assert_eq!(materials.stock_kg(Material::Gold), f64::MAX);
    assert!(ledger.entries().is_empty());

    let (mut empty, mut empty_ledger) = (MaterialLedger::new(), Ledger::new());
    let duplicate_output_overflow = empty.craft(
        &[],
        &[(Material::Gold, f64::MAX), (Material::Gold, f64::MAX)],
        &mut empty_ledger,
    );
    assert!(matches!(
        duplicate_output_overflow,
        Err(MaterialError::InvalidAmount {
            material: Material::Gold,
            amount_kg
        }) if amount_kg.is_infinite()
    ));
    assert_eq!(empty.stock_kg(Material::Gold), 0.0);
    assert!(empty_ledger.entries().is_empty());
}

#[test]
fn fossil_gathering_rejects_invalid_and_overflowing_amounts_before_mutation() {
    let (mut materials, mut ledger) = (MaterialLedger::new(), Ledger::new());

    for amount in [-1.0, f64::NAN, f64::INFINITY] {
        let stock_before = materials.stock_kg(Material::Coal);
        let entries_before = ledger.entries().len();
        let result = materials.gather_fossil(Material::Coal, amount, &mut ledger);
        assert!(matches!(
            result,
            Err(MaterialError::InvalidAmount {
                material: Material::Coal,
                amount_kg
            }) if amount_kg.to_bits() == amount.to_bits()
        ));
        assert_eq!(materials.stock_kg(Material::Coal), stock_before);
        assert_eq!(ledger.entries().len(), entries_before);
    }

    materials
        .gather_fossil(Material::Coal, f64::MAX, &mut ledger)
        .expect("the first finite coal deposit fits");
    let entries_before = ledger.entries().len();
    let overflow = materials.gather_fossil(Material::Coal, f64::MAX, &mut ledger);
    assert!(matches!(
        overflow,
        Err(MaterialError::InvalidAmount {
            material: Material::Coal,
            amount_kg
        }) if amount_kg.is_infinite()
    ));
    assert_eq!(materials.stock_kg(Material::Coal), f64::MAX);
    assert_eq!(ledger.entries().len(), entries_before);
}

#[test]
fn water_gathering_rejects_invalid_amounts_and_stock_overflow_before_drain() {
    let b = base();
    let area = b.domain.cell_area_m2(M);
    let cell = b.forest_cell;
    let spec = b.domain.storage_spec(M);
    let mut hydrology = HydrologyState::new(&spec, 0.5);
    hydrology.surface_water.set(cell.0, cell.1, 5.0);
    let (mut materials, mut ledger) = (MaterialLedger::new(), Ledger::new());

    for amount in [-1.0, f64::NAN, f64::INFINITY] {
        let surface_before = *hydrology.surface_water.get(cell.0, cell.1);
        let result = materials.gather_water(amount, &mut hydrology, cell, area, &mut ledger);
        assert!(matches!(
            result,
            Err(MaterialError::InvalidAmount {
                material: Material::Water,
                amount_kg
            }) if amount_kg.to_bits() == amount.to_bits()
        ));
        assert_eq!(*hydrology.surface_water.get(cell.0, cell.1), surface_before);
        assert_eq!(materials.stock_kg(Material::Water), 0.0);
        assert!(ledger.entries().is_empty());
    }

    // Seed a finite maximum stock, then offer enough finite standing water to
    // overflow it. The source cell must not be drained on a rejected transfer.
    materials
        .inject(Material::Water, f64::MAX, &mut ledger)
        .expect("seed the maximum finite water stock");
    hydrology
        .surface_water
        .set(cell.0, cell.1, (0.4 * f64::MAX) / area);
    let surface_before = *hydrology.surface_water.get(cell.0, cell.1);
    let entries_before = ledger.entries().len();
    let overflow = materials.gather_water(f64::MAX, &mut hydrology, cell, area, &mut ledger);
    assert!(matches!(
        overflow,
        Err(MaterialError::InvalidAmount {
            material: Material::Water,
            amount_kg
        }) if amount_kg.is_infinite()
    ));
    assert_eq!(materials.stock_kg(Material::Water), f64::MAX);
    assert_eq!(*hydrology.surface_water.get(cell.0, cell.1), surface_before);
    assert_eq!(ledger.entries().len(), entries_before);
}

#[test]
fn surface_water_gathering_rejects_unrepresentable_source_and_stock_debits_before_mutation() {
    let b = base();
    let area = b.domain.cell_area_m2(M);
    let cell = b.forest_cell;
    let spec = b.domain.storage_spec(M);
    let mut hydrology = HydrologyState::new(&spec, 0.5);
    let (mut materials, mut ledger) = (MaterialLedger::new(), Ledger::new());

    // A 1 kg transfer is too small to change this binary64 source depth.
    // The destination is empty, so the source-debit preflight must reject it.
    let large_source = 2_f64.powi(53);
    hydrology.surface_water.set(cell.0, cell.1, large_source);
    let surface_before = *hydrology.surface_water.get(cell.0, cell.1);
    let runoff_before = *hydrology.runoff.get(cell.0, cell.1);
    let entries_before = ledger.entries().len();
    let source_rounding = materials.gather_water(1.0, &mut hydrology, cell, area, &mut ledger);
    assert!(matches!(
        source_rounding,
        Err(MaterialError::InvalidSourceState {
            material: Material::Water,
            ..
        })
    ));
    assert_eq!(materials.stock_kg(Material::Water), 0.0);
    assert_eq!(*hydrology.surface_water.get(cell.0, cell.1), surface_before);
    assert_eq!(*hydrology.runoff.get(cell.0, cell.1), runoff_before);
    assert_eq!(ledger.entries().len(), entries_before);

    // A normal source can still be drained only if the material inventory
    // actually increases by the same amount.
    hydrology.surface_water.set(cell.0, cell.1, 5.0);
    materials
        .inject(Material::Water, large_source, &mut ledger)
        .expect("2^53 kg is a representable initial stock");
    let stock_before = materials.stock_kg(Material::Water);
    let surface_before = *hydrology.surface_water.get(cell.0, cell.1);
    let runoff_before = *hydrology.runoff.get(cell.0, cell.1);
    let entries_before = ledger.entries().len();
    let stock_rounding = materials.gather_water(1.0, &mut hydrology, cell, area, &mut ledger);
    assert!(matches!(
        stock_rounding,
        Err(MaterialError::InvalidAmount {
            material: Material::Water,
            amount_kg
        }) if amount_kg == large_source
    ));
    assert_eq!(materials.stock_kg(Material::Water), stock_before);
    assert_eq!(*hydrology.surface_water.get(cell.0, cell.1), surface_before);
    assert_eq!(*hydrology.runoff.get(cell.0, cell.1), runoff_before);
    assert_eq!(ledger.entries().len(), entries_before);
}

#[test]
fn operator_injection_rejects_invalid_and_overflowing_amounts_before_mutation() {
    let (mut materials, mut ledger) = (MaterialLedger::new(), Ledger::new());

    for amount in [-1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let stock_before = materials.stock_kg(Material::Water);
        let entries_before = ledger.entries().len();
        let result = materials.inject(Material::Water, amount, &mut ledger);
        assert!(matches!(
            result,
            Err(MaterialError::InvalidAmount {
                material: Material::Water,
                amount_kg
            }) if amount_kg.to_bits() == amount.to_bits()
        ));
        assert_eq!(materials.stock_kg(Material::Water), stock_before);
        assert_eq!(ledger.entries().len(), entries_before);
    }

    materials
        .inject(Material::Water, f64::MAX, &mut ledger)
        .expect("the first finite operator inflow fits");
    let entries_before = ledger.entries().len();
    let overflow = materials.inject(Material::Water, f64::MAX, &mut ledger);
    assert!(matches!(
        overflow,
        Err(MaterialError::InvalidAmount {
            material: Material::Water,
            amount_kg
        }) if amount_kg.is_infinite()
    ));
    assert_eq!(materials.stock_kg(Material::Water), f64::MAX);
    assert_eq!(ledger.entries().len(), entries_before);
}

#[test]
fn operator_injection_rejects_positive_increments_that_round_away() {
    for material in [Material::Wood, Material::Water] {
        let (mut materials, mut ledger) = (MaterialLedger::new(), Ledger::new());
        materials
            .inject(material, f64::MAX, &mut ledger)
            .expect("finite maximum stock is a valid seed");

        let entries_before = ledger.entries().len();
        let rounded_away = materials.inject(material, 1.0, &mut ledger);

        assert!(matches!(
            rounded_away,
            Err(MaterialError::InvalidAmount {
                material: actual,
                amount_kg
            }) if actual == material && amount_kg == f64::MAX
        ));
        assert_eq!(materials.stock_kg(material), f64::MAX);
        assert_eq!(ledger.entries().len(), entries_before);
    }
}

#[test]
fn operator_injection_books_material_carbon_and_water_as_boundary_inflows() {
    let (mut materials, mut ledger) = (MaterialLedger::new(), Ledger::new());
    let audit = Audit::start(&materials);

    materials
        .inject(Material::Wood, 10.0, &mut ledger)
        .expect("valid wood inflow");
    materials
        .inject(Material::Water, 4.0, &mut ledger)
        .expect("valid water inflow");

    let expected_carbon = 10.0 * composition(Material::Wood).carbon_per_kg;
    assert!((net(&ledger, R::MaterialCarbon, FluxKind::Carbon) - expected_carbon).abs() < 1e-12);
    assert!((net(&ledger, R::MaterialWater, FluxKind::Water) - 4.0).abs() < 1e-12);
    audit.close(&materials, &ledger);
}

#[test]
fn calcining_limestone_and_making_charcoal_release_their_carbon_as_co2() {
    let (mut m, mut ledger) = (MaterialLedger::new(), Ledger::new());
    let audit = Audit::start(&m);
    m.gather_fossil(Material::Limestone, 1_000.0, &mut ledger)
        .unwrap();
    m.calcine(1_000.0, &mut ledger).unwrap();
    // CaCO3 -> CaO + CO2: 44.009 kg of CO2 per 100.087 kg of limestone.
    let co2_carbon = net(&ledger, R::AtmosCO2, FluxKind::Carbon);
    assert!(
        (co2_carbon * 44.009 / 12.011 - 1_000.0 * 44.009 / 100.087).abs() < 0.5,
        "CO2 {co2_carbon}"
    );
    assert!((m.stock_kg(Material::Quicklime) - 1_000.0 * QUICKLIME_PER_LIMESTONE).abs() < 1e-9);
    // The lime kept no carbon: the limestone's carbon all left.
    assert!(m.stock_of(R::MaterialCarbon).abs() < 1e-9);
    audit.close(&m, &ledger);

    // Charcoal: wood's carbon is split between the char and the CO2.
    let (mut m, mut ledger) = (MaterialLedger::new(), Ledger::new());
    let audit = Audit::start(&m);
    let b = base();
    let mut ecology = b.ecology.clone();
    m.gather_biotic(
        Material::Wood,
        500.0,
        &mut ecology,
        b.forest_cell,
        b.domain.cell_area_m2(M),
        &mut ledger,
    )
    .unwrap();
    let carbon_before = m.stock_of(R::MaterialCarbon);
    let char_kg = m.make_charcoal(500.0, &mut ledger).unwrap();
    assert!(
        (0.05..0.2).contains(&(char_kg / 500.0)),
        "charcoal yield {}",
        char_kg / 500.0
    );
    let released = carbon_before - m.stock_of(R::MaterialCarbon);
    assert!(
        released > 0.0 && (net(&ledger, R::AtmosCO2, FluxKind::Carbon) - released).abs() < 1e-9
    );
    // Most of the wood's carbon is released; the char keeps ~a quarter.
    assert!((0.15..0.35).contains(&(m.stock_of(R::MaterialCarbon) / carbon_before)));
    audit.close(&m, &ledger);
}

#[test]
fn hunting_reduces_the_population_and_moves_the_carbon_with_an_exact_accumulator() {
    let (mut m, mut ledger) = (MaterialLedger::new(), Ledger::new());
    let audit = Audit::start(&m);
    let mut deer = AnimalPopulation::new(1_000, 50.0);
    let before_c = deer.carbon_kgc();
    // 100 kg of meat is 4 animals of 50 kg at 50% edible.
    m.hunt(&mut deer, 100.0, &mut ledger).unwrap();
    assert_eq!(deer.count, 996);
    let meat_c = 100.0 * composition(Material::Meat).carbon_per_kg;
    let to_material = net(&ledger, R::MaterialCarbon, FluxKind::Carbon);
    assert!((to_material - meat_c).abs() < 1e-9);
    // The animals' whole carbon left biomass: meat to material, carcass to detritus.
    let total_out: f64 = ledger
        .entries()
        .iter()
        .filter(|e| e.source == R::BiomassCarbon && e.kind == FluxKind::Carbon)
        .map(|e| e.amount)
        .sum();
    assert!(
        (total_out - 4.0 * 50.0 * 0.18).abs() < 1e-9,
        "left biomass: {total_out}"
    );
    assert!((before_c - deer.carbon_kgc() - total_out).abs() < 1e-6);
    audit.close(&m, &ledger);

    // Small kills accumulate: ten 5 kg hunts of an animal that yields 25 kg
    // of meat kill two whole animals, never a fraction.
    let mut hares = AnimalPopulation::new(10, 50.0);
    for _ in 0..10 {
        m.hunt(&mut hares, 5.0, &mut ledger).unwrap();
    }
    assert_eq!(hares.count, 8, "ten 5 kg hunts of 25 kg animals kill two");
    // With no animals left there is nothing to hunt.
    let mut none = AnimalPopulation::new(0, 50.0);
    assert_eq!(
        m.hunt(&mut none, 1.0, &mut ledger),
        Err(MaterialError::NoAnimals)
    );
}

#[test]
fn a_human_eating_and_living_a_day_moves_food_carbon_through_the_body_to_the_air() {
    let (mut m, mut ledger) = (MaterialLedger::new(), Ledger::new());
    let b = base();
    let mut ecology = b.ecology.clone();
    let area = b.domain.cell_area_m2(M);
    m.register_human("Gem-D", 80.0);
    let body_c = m.stock_of(R::HumanCarbon);
    assert!((body_c - 80.0 * 0.18).abs() < 1e-12);
    let audit = Audit::start(&m);

    // Gather and eat 2 kg of plant food and 0.5 kg of meat.
    m.gather_biotic(
        Material::PlantFood,
        2.0,
        &mut ecology,
        b.forest_cell,
        area,
        &mut ledger,
    )
    .unwrap();
    let mut game = AnimalPopulation::new(100, 40.0);
    m.hunt(&mut game, 0.5, &mut ledger).unwrap();
    m.eat("Gem-D", Material::PlantFood, 2.0, &mut ledger)
        .unwrap();
    m.eat("Gem-D", Material::Meat, 0.5, &mut ledger).unwrap();
    let eaten_c = 2.0 * 0.1125 + 0.5 * 0.15;
    assert!((m.stock_of(R::HumanCarbon) - body_c - eaten_c).abs() < 1e-12);
    assert!((net(&ledger, R::HumanCarbon, FluxKind::Carbon) - eaten_c).abs() < 1e-12);

    // A day's respiration at 2,500 kcal returns carbon to the air.
    let respired = m.respire("Gem-D", 2_500.0, &mut ledger).unwrap();
    assert!((respired - 2_500.0 * respired_carbon_kg_per_kcal()).abs() < 1e-12);
    assert!((net(&ledger, R::AtmosCO2, FluxKind::Carbon) - respired).abs() < 1e-12);
    assert!((net(&ledger, R::AtmosO2, FluxKind::Oxygen) + respired * O2_PER_CARBON).abs() < 1e-9);
    audit.close(&m, &ledger);

    // Death returns the body's carbon to the detritus.
    let remaining = m.stock_of(R::HumanCarbon);
    // The body's water goes into the soil where it lies, and is booked.
    let mut soil = HydrologyState::new(&b.domain.storage_spec(M), 0.5);
    let cell = b.forest_cell;
    let before = soil.soil_water.get(cell.0, cell.1).storage_mm;
    let body_water = m.stock_of(R::MaterialWater);
    let soil_booked = net(&ledger, R::SoilWater, FluxKind::Water);
    m.on_death("Gem-D", &mut soil, cell, area, &mut ledger)
        .unwrap();
    let gained = (soil.soil_water.get(cell.0, cell.1).storage_mm - before) * area;
    assert!(
        (gained - body_water).abs() < 1e-6,
        "{gained} kg of {body_water}"
    );
    assert!(
        (net(&ledger, R::SoilWater, FluxKind::Water) - soil_booked - gained).abs() < 1e-6,
        "the soil's gain is what the ledger booked"
    );
    assert_eq!(m.stock_of(R::HumanCarbon), 0.0);
    // Detritus holds the hunted carcass (0.025 animals x 40 kg x 18% C less
    // the 0.075 kg C of meat) and the body's remaining carbon.
    let carcass = 0.025 * 40.0 * 0.18 - 0.5 * 0.15;
    assert!(
        (net(&ledger, R::DetritusCarbon, FluxKind::Carbon) - remaining - carcass).abs() < 1e-9,
        "detritus {}",
        net(&ledger, R::DetritusCarbon, FluxKind::Carbon)
    );
    audit.close(&m, &ledger);
    // Only food can be eaten, and only by a registered human.
    assert!(matches!(
        m.eat("Gem-D", Material::PlantFood, 1.0, &mut ledger),
        Err(MaterialError::NoSuchHuman(_))
    ));
}

#[test]
fn drinking_needs_water_in_the_cell_and_returns_it_to_the_air_and_soil() {
    let b = base();
    let area = b.domain.cell_area_m2(M);
    let spec = b.domain.storage_spec(M);
    let cell = b.forest_cell;
    let (mut m, mut ledger) = (MaterialLedger::new(), Ledger::new());
    m.register_human("Gem-K", 60.0);
    let audit = Audit::start(&m);

    // A dry cell refuses.
    let mut dry = HydrologyState::new(&spec, 0.5);
    assert_eq!(
        m.gather_water(1.0, &mut dry, cell, area, &mut ledger),
        Err(MaterialError::DryCell)
    );

    // Standing water is drawn down by exactly what is taken.
    let mut wet = HydrologyState::new(&spec, 0.5);
    wet.surface_water.set(cell.0, cell.1, 5.0); // mm
    let got = m
        .gather_water(3.0, &mut wet, cell, area, &mut ledger)
        .unwrap();
    assert_eq!(got, 3.0);
    assert!((wet.surface_water.get(cell.0, cell.1) - (5.0 - 3.0 / area)).abs() < 1e-12);
    // River flow can be drawn when nothing stands (a small share of the flow).
    let mut river = HydrologyState::new(&spec, 0.5);
    river.runoff.set(cell.0, cell.1, 100.0); // mm/day
    assert!(m
        .gather_water(10.0, &mut river, cell, area, &mut ledger)
        .is_ok());
    // You cannot take more than the cell has.
    let mut puddle = HydrologyState::new(&spec, 0.5);
    puddle.surface_water.set(cell.0, cell.1, 1.0e-6);
    let got = m
        .gather_water(1.0e6, &mut puddle, cell, area, &mut ledger)
        .unwrap();
    assert!((got - 1.0e-6 * area).abs() < 1e-9);

    // Drink, then lose it by breath, sweat and excretion: the body's water is bounded.
    m.drink("Gem-K", 2.0).unwrap();
    assert!(m.drink("Gem-K", 1.0e6).is_err());
    let before = puddle.soil_water.get(cell.0, cell.1).storage_mm;
    let lost = m
        .lose_water("Gem-K", 5.0, 0.6, &mut puddle, cell, area, &mut ledger)
        .unwrap();
    assert_eq!(lost, 2.0, "only what the body holds is lost");
    // The excreted share is in that cell's soil, not just in the books.
    let gained = (puddle.soil_water.get(cell.0, cell.1).storage_mm - before) * area;
    assert!((gained - 1.2).abs() < 1e-6, "{gained}");
    assert!((net(&ledger, R::SoilWater, FluxKind::Water) - 1.2).abs() < 1e-12);
    assert!((net(&ledger, R::Atmosphere, FluxKind::Water) - 0.8).abs() < 1e-12);
    audit.close(&m, &ledger);
}

#[test]
fn a_built_shelter_holds_its_carbon_until_it_is_demolished_and_crafting_cannot_create_carbon() {
    let b = base();
    let (mut m, mut ledger) = (MaterialLedger::new(), Ledger::new());
    let mut ecology = b.ecology.clone();
    let area = b.domain.cell_area_m2(M);
    let audit = Audit::start(&m);
    m.gather_biotic(
        Material::Wood,
        800.0,
        &mut ecology,
        b.forest_cell,
        area,
        &mut ledger,
    )
    .unwrap();
    let carbon = m.stock_of(R::MaterialCarbon);
    let id = m.build(Material::Wood, 800.0).unwrap();
    assert_eq!(m.stock_kg(Material::Wood), 0.0);
    assert!(
        (m.stock_of(R::MaterialCarbon) - carbon).abs() < 1e-12,
        "building keeps the carbon"
    );
    audit.close(&m, &ledger);

    // It decays slowly over its documented lifetime: after 60 years it has
    // lost 63% of its mass to detritus; meanwhile the books close.
    m.decay_structures(1.0, &mut ledger);
    assert!(
        m.stock_of(R::MaterialCarbon) < carbon && m.stock_of(R::MaterialCarbon) > 0.95 * carbon
    );
    audit.close(&m, &ledger);
    m.demolish(id, &mut ledger).unwrap();
    assert!(m.stock_of(R::MaterialCarbon).abs() < 1e-12);
    assert!(
        (net(&ledger, R::DetritusCarbon, FluxKind::Carbon) - carbon).abs() < 1e-9,
        "all of it ends in detritus"
    );
    audit.close(&m, &ledger);
    assert_eq!(
        m.demolish(id, &mut ledger),
        Err(MaterialError::NoSuchStructure(id))
    );

    // Crafting: offcuts go to detritus; more carbon out than in is refused.
    m.gather_biotic(
        Material::Wood,
        10.0,
        &mut ecology,
        b.forest_cell,
        area,
        &mut ledger,
    )
    .unwrap();
    let audit = Audit::start(&m);
    let mut l2 = Ledger::new();
    m.craft(
        &[(Material::Wood, 10.0)],
        &[(Material::Fibre, 4.0)],
        &mut l2,
    )
    .unwrap();
    let lost_c = 10.0 * 0.425 - 4.0 * 0.40;
    assert!((net(&l2, R::DetritusCarbon, FluxKind::Carbon) - lost_c).abs() < 1e-9);
    audit.close(&m, &l2);
    assert!(matches!(
        m.craft(&[(Material::Fibre, 1.0)], &[(Material::Wood, 5.0)], &mut l2),
        Err(MaterialError::CreatesCarbon { .. })
    ));
}

#[test]
fn digging_for_water_needs_damp_soil_and_moves_exactly_what_is_taken() {
    use mk_engine::hydrology::SoilWater;
    let b = base();
    let area = b.domain.cell_area_m2(M);
    let spec = b.domain.storage_spec(M);
    let cell = b.forest_cell;
    let (mut m, mut ledger) = (MaterialLedger::new(), Ledger::new());
    let audit = Audit::start(&m);

    // Parched soil gives nothing.
    let mut parched = HydrologyState::new(&spec, 0.05);
    assert_eq!(
        m.gather_soil_water(1.0, &mut parched, cell, area, &mut ledger),
        Err(MaterialError::DryCell)
    );

    // Damp soil gives what is asked and loses exactly that much storage.
    let mut damp = HydrologyState::new(&spec, 0.5);
    let before = damp.soil_water.get(cell.0, cell.1).storage_mm;
    let got = m
        .gather_soil_water(2.0, &mut damp, cell, area, &mut ledger)
        .unwrap();
    assert_eq!(got, 2.0);
    let after: SoilWater = *damp.soil_water.get(cell.0, cell.1);
    assert!((before - after.storage_mm - 2.0 / area).abs() < 1e-12);
    assert!((after.moisture_fraction - after.storage_mm / 300.0).abs() < 1e-12);
    // The books: the soil gave it, the material stock holds it.
    assert!((net(&ledger, R::SoilWater, FluxKind::Water) + 2.0).abs() < 1e-12);
    assert!((m.stock_kg(Material::Water) - 2.0).abs() < 1e-12);

    // It cannot give more than the soil holds.
    let mut thin = HydrologyState::new(&spec, 0.5);
    thin.soil_water.set(
        cell.0,
        cell.1,
        SoilWater {
            moisture_fraction: 0.25,
            storage_mm: 1.0e-6,
        },
    );
    let got = m
        .gather_soil_water(1.0e6, &mut thin, cell, area, &mut ledger)
        .unwrap();
    assert!((got - 1.0e-6 * area).abs() < 1e-9);
    audit.close(&m, &ledger);
}

/// The ledger survives a round trip through JSON unchanged.
///
/// Phase 4 Task 3 needs the whole island to save and load; this is the
/// first state type to carry its own weight. A ledger that came back
/// different would move carbon or water across a save, which is exactly
/// what the flux audit exists to forbid.
#[test]
fn the_material_ledger_round_trips_through_a_snapshot() {
    use mk_engine::regional::materials::MaterialLedger;

    let mut ledger = Ledger::default();
    let mut materials = MaterialLedger::new();
    materials.register_human("gem-d", 72.0);
    materials.register_human("gem-k", 58.0);
    // Move both bodies off their registered state, so a round trip that
    // silently reset them would show.
    materials.respire("gem-d", 500.0, &mut ledger).unwrap();
    let _ = materials.drink("gem-k", 1.5);
    // Pending cross-cut ids must survive a save/load so a planned haul can
    // still materialise its pieces after the island is restored.
    assert_eq!(materials.reserve_log_ids(2), 1);

    let empty_snapshot =
        serde_json::to_string(&MaterialLedger::new()).expect("an empty ledger serializes");
    assert!(
        !empty_snapshot.contains("reserved_log_ids"),
        "empty reservation state must not change legacy snapshot bytes"
    );

    let text = serde_json::to_string(&materials).expect("the ledger serializes");
    let back: MaterialLedger = serde_json::from_str(&text).expect("and comes back");

    assert_eq!(back, materials, "the ledger changed across a round trip");
    for id in ["gem-d", "gem-k"] {
        assert_eq!(
            back.body_carbon_kg(id),
            materials.body_carbon_kg(id),
            "{id}'s body carbon changed across a round trip"
        );
    }
}

#[test]
#[should_panic(expected = "cross-cut piece count exceeds safety bound")]
fn direct_log_cross_cut_rejects_oversized_piece_count_before_allocation() {
    let log = Log {
        id: 1,
        position_m: (0.0, 0.0),
        heading_rad: 0.0,
        length_m: 1.0,
        diameter_m: 0.2,
        dry_kg: 10.0,
        carbon_kgc: 5.0,
        water_kg: 0.0,
        place: LogPlace::Ground,
    };
    let _ = log.cross_cut(MAX_CROSS_CUT_PIECES + 1, 2);
}

#[test]
#[should_panic(expected = "cross-cut source log has invalid physical state")]
fn direct_log_cross_cut_rejects_non_finite_geometry_before_piece_allocation() {
    let mut log = Log {
        id: 1,
        position_m: (0.0, 0.0),
        heading_rad: 0.0,
        length_m: 1.0,
        diameter_m: 0.2,
        dry_kg: 10.0,
        carbon_kgc: 5.0,
        water_kg: 0.0,
        place: LogPlace::Ground,
    };
    log.heading_rad = f64::NAN;
    let _ = log.cross_cut(2, 2);
}

#[test]
fn receiving_invalid_felled_log_fails_before_allocating_or_booking_material() {
    use std::panic::{catch_unwind, AssertUnwindSafe};

    let b = base();
    let cell = b.forest_cell;
    let area = b.domain.cell_area_m2(M);
    let mut hydrology = HydrologyState::new(&b.domain.storage_spec(M), 0.5);
    let mut materials = MaterialLedger::new();
    let mut ledger = Ledger::new();
    let audit = Audit::start(&materials);
    let mut log = Log {
        id: 0,
        position_m: (0.0, 0.0),
        heading_rad: 0.0,
        length_m: 2.0,
        diameter_m: 0.2,
        dry_kg: 10.0,
        carbon_kgc: 5.0,
        water_kg: 8.2,
        place: LogPlace::Ground,
    };
    log.position_m.0 = f64::NAN;
    let next_id_before = materials.next_log_id();

    let result = catch_unwind(AssertUnwindSafe(|| {
        materials.receive_felled_log(log, &mut hydrology, cell, area, &mut ledger)
    }));

    assert!(result.is_err());
    assert_eq!(materials.next_log_id(), next_id_before);
    assert_eq!(materials.logs().count(), 0);
    audit.close(&materials, &ledger);
}

#[test]
fn receiving_felled_log_rejects_carbon_mass_above_dry_mass_before_allocating_or_booking_material() {
    use std::panic::{catch_unwind, AssertUnwindSafe};

    let b = base();
    let cell = b.forest_cell;
    let area = b.domain.cell_area_m2(M);
    let mut hydrology = HydrologyState::new(&b.domain.storage_spec(M), 0.5);
    let mut materials = MaterialLedger::new();
    let mut ledger = Ledger::new();
    let audit = Audit::start(&materials);
    let log = Log {
        id: 0,
        position_m: (0.0, 0.0),
        heading_rad: 0.0,
        length_m: 2.0,
        diameter_m: 0.2,
        dry_kg: 10.0,
        carbon_kgc: 10.01,
        water_kg: 8.2,
        place: LogPlace::Ground,
    };
    let next_id_before = materials.next_log_id();

    let result = catch_unwind(AssertUnwindSafe(|| {
        materials.receive_felled_log(log, &mut hydrology, cell, area, &mut ledger)
    }));

    assert!(result.is_err(), "carbon cannot exceed the mass of dry wood");
    assert_eq!(materials.next_log_id(), next_id_before);
    assert_eq!(materials.logs().count(), 0);
    audit.close(&materials, &ledger);
}

#[test]
#[should_panic(expected = "cross-cut reservation must contain at least one id")]
fn material_ledger_rejects_empty_cross_cut_reservations_before_allocation() {
    MaterialLedger::new().reserve_log_ids(0);
}

#[test]
#[should_panic(expected = "cross-cut reservation exceeds safety bound")]
fn material_ledger_rejects_oversized_cross_cut_reservations_before_allocation() {
    let mut materials = MaterialLedger::new();
    materials.reserve_log_ids(u64::from(MAX_CROSS_CUT_PIECES) + 1);
}

#[test]
fn cross_cut_refuses_zero_piece_count_without_consuming_its_reservation() {
    let b = base();
    let cell = b.forest_cell;
    let area = b.domain.cell_area_m2(M);
    let mut hydrology = HydrologyState::new(&b.domain.storage_spec(M), 0.5);
    let mut materials = MaterialLedger::new();
    let mut ledger = Ledger::new();
    let source = Log {
        id: 0,
        position_m: (0.0, 0.0),
        heading_rad: 0.0,
        length_m: 2.0,
        diameter_m: 0.2,
        dry_kg: 10.0,
        carbon_kgc: 5.0,
        water_kg: 8.2,
        place: LogPlace::Ground,
    };
    let source_id = materials.receive_felled_log(source, &mut hydrology, cell, area, &mut ledger);
    let first_piece_id = materials.reserve_log_ids(1);
    let source_before = materials.log(source_id).unwrap().clone();

    assert!(!materials.cross_cut_log(source_id, 0, first_piece_id));
    assert_eq!(materials.log(source_id), Some(&source_before));
    assert_eq!(materials.log(first_piece_id), None);
    assert_eq!(materials.next_log_id(), first_piece_id + 1);

    // The invalid zero-piece request must not consume the pending range.
    assert!(materials.cross_cut_log(source_id, 1, first_piece_id));
    assert_eq!(materials.log(source_id), None);
    assert!(materials.log(first_piece_id).is_some());
}

#[test]
fn material_ledger_rejects_invalid_restored_logs_and_cross_cut_preserves_reservations() {
    let b = base();
    let cell = b.forest_cell;
    let area = b.domain.cell_area_m2(M);
    let mut hydrology = HydrologyState::new(&b.domain.storage_spec(M), 0.5);
    let mut materials = MaterialLedger::new();
    let mut ledger = Ledger::new();
    let source = Log {
        id: 0,
        position_m: (0.0, 0.0),
        heading_rad: 0.0,
        length_m: 2.0,
        diameter_m: 0.2,
        dry_kg: 10.0,
        carbon_kgc: 5.0,
        water_kg: 8.2,
        place: LogPlace::Ground,
    };
    let source_id = materials.receive_felled_log(source, &mut hydrology, cell, area, &mut ledger);
    let first_piece_id = materials.reserve_log_ids(2);
    let carbon_before = materials.stock_of(R::MaterialCarbon);
    let water_before = materials.stock_of(R::MaterialWater);
    let next_id_before = materials.next_log_id();
    let valid_snapshot = serde_json::to_value(&materials).expect("valid ledger serializes");

    // The allocator watermark is exclusive and must remain above every
    // materialised id; otherwise a future allocation could reuse an existing id.
    let mut stale_allocator = valid_snapshot.clone();
    stale_allocator["next_log"] = serde_json::Value::from(source_id);
    let stale_allocator_error = serde_json::from_value::<MaterialLedger>(stale_allocator)
        .expect_err("a watermark at the highest materialised id must be rejected");
    assert!(
        stale_allocator_error
            .to_string()
            .contains("does not exceed highest materialised log id"),
        "unexpected allocator error: {stale_allocator_error}"
    );

    // A pending reservation may neither alias an existing log nor point at
    // the next unallocated id beyond the exclusive watermark.
    let mut colliding_reservation = valid_snapshot.clone();
    colliding_reservation["reserved_log_ids"]
        .as_array_mut()
        .expect("reserved ids serialize as an array")
        .push(serde_json::Value::from(source_id));
    let collision_error = serde_json::from_value::<MaterialLedger>(colliding_reservation)
        .expect_err("a reservation colliding with a materialised log must be rejected");
    assert!(
        collision_error
            .to_string()
            .contains("is already materialised"),
        "unexpected reservation collision error: {collision_error}"
    );

    let mut out_of_range_reservation = valid_snapshot.clone();
    out_of_range_reservation["reserved_log_ids"]
        .as_array_mut()
        .expect("reserved ids serialize as an array")
        .push(serde_json::Value::from(next_id_before));
    let range_error = serde_json::from_value::<MaterialLedger>(out_of_range_reservation)
        .expect_err("a reservation at the exclusive watermark must be rejected");
    assert!(
        range_error
            .to_string()
            .contains("outside allocator watermark"),
        "unexpected reservation range error: {range_error}"
    );

    // Snapshots predating log support omit all three allocator fields. Their
    // documented defaults remain loadable and start allocation at one.
    let mut legacy_snapshot =
        serde_json::to_value(MaterialLedger::new()).expect("empty ledger serializes");
    let legacy_object = legacy_snapshot
        .as_object_mut()
        .expect("ledger snapshot is an object");
    legacy_object.remove("logs");
    legacy_object.remove("next_log");
    legacy_object.remove("reserved_log_ids");
    let legacy: MaterialLedger = serde_json::from_value(legacy_snapshot)
        .expect("pre-log snapshots retain their legacy defaults");
    assert_eq!(legacy.next_log_id(), 1);

    // Persisted logs are validated at the restore boundary, before malformed
    // floating-point state can enter the live single-writer simulation.
    let mut corrupted_snapshot = valid_snapshot.clone();
    let source_key = source_id.to_string();
    let source_value = corrupted_snapshot
        .get_mut("logs")
        .and_then(|logs| logs.get_mut(&source_key))
        .expect("source log is serialized");
    let dry_kg = source_value["dry_kg"]
        .as_f64()
        .expect("dry mass is numeric");
    source_value["carbon_kgc"] = serde_json::Value::from(dry_kg + 1.0);
    let restore_error = serde_json::from_value::<MaterialLedger>(corrupted_snapshot)
        .expect_err("restoring a log whose carbon exceeds dry mass must fail");
    assert!(
        restore_error
            .to_string()
            .contains("material ledger log 1 has invalid physical state"),
        "unexpected restore error: {restore_error}"
    );

    // The failed restore cannot mutate the live ledger or its reservation.
    assert_eq!(materials.stock_of(R::MaterialCarbon), carbon_before);
    assert_eq!(materials.stock_of(R::MaterialWater), water_before);
    assert_eq!(materials.next_log_id(), next_id_before);
    assert_eq!(materials.log(source_id).unwrap().id, source_id);

    // The public ledger operation also refuses invalid in-memory state, even
    // if a caller mutates a log after a valid restore.
    let mut invalid_in_memory = materials.clone();
    invalid_in_memory
        .log_mut(source_id)
        .expect("source log remains present")
        .carbon_kgc = dry_kg + 1.0;
    let invalid_source = invalid_in_memory.log(source_id).unwrap().clone();
    let invalid_carbon_before = invalid_in_memory.stock_of(R::MaterialCarbon);
    let invalid_water_before = invalid_in_memory.stock_of(R::MaterialWater);
    assert!(!invalid_in_memory.cross_cut_log(source_id, 2, first_piece_id));
    assert_eq!(invalid_in_memory.log(source_id), Some(&invalid_source));
    assert_eq!(invalid_in_memory.log(first_piece_id), None);
    assert_eq!(invalid_in_memory.log(first_piece_id + 1), None);
    assert_eq!(
        invalid_in_memory.stock_of(R::MaterialCarbon),
        invalid_carbon_before
    );
    assert_eq!(
        invalid_in_memory.stock_of(R::MaterialWater),
        invalid_water_before
    );
    assert_eq!(invalid_in_memory.next_log_id(), next_id_before);

    // The untouched valid snapshot retains the same reservation and can be
    // restored and consumed exactly once.
    let mut restored: MaterialLedger =
        serde_json::from_value(valid_snapshot).expect("valid reserved ledger restores");
    assert!(restored.cross_cut_log(source_id, 2, first_piece_id));
    assert_eq!(restored.log(source_id), None);
    let first = restored
        .log(first_piece_id)
        .expect("first piece materialised");
    let second = restored
        .log(first_piece_id + 1)
        .expect("second piece materialised");
    assert_eq!((first.dry_kg + second.dry_kg).to_bits(), dry_kg.to_bits());
    assert_eq!(
        (first.carbon_kgc + second.carbon_kgc).to_bits(),
        (dry_kg * 0.5).to_bits()
    );
    assert_eq!(restored.stock_of(R::MaterialCarbon), carbon_before);
    assert_eq!(restored.stock_of(R::MaterialWater), water_before);
    assert_eq!(restored.next_log_id(), next_id_before);
}

#[test]
fn cross_cut_refuses_colliding_overflowing_unreserved_and_consumed_piece_ids_without_mutating_the_ledger(
) {
    let b = base();
    let cell = b.forest_cell;
    let area = b.domain.cell_area_m2(M);
    let mut hydrology = HydrologyState::new(&b.domain.storage_spec(M), 0.5);
    let mut materials = MaterialLedger::new();
    let mut ledger = Ledger::new();

    let make_log = |position_m, dry_kg| Log {
        id: 0,
        position_m,
        heading_rad: 0.0,
        length_m: 2.0,
        diameter_m: 0.2,
        dry_kg,
        carbon_kgc: dry_kg * 0.5,
        water_kg: dry_kg * 0.82,
        place: LogPlace::Ground,
    };

    // A dry stored source lets the test remove its cut pieces through the
    // real seasoning path, leaving the old ids unoccupied but already used.
    let mut first_source = make_log((0.0, 0.0), 10.0);
    first_source.place = LogPlace::Stored;
    first_source.water_kg = 0.0;
    let source_id =
        materials.receive_felled_log(first_source, &mut hydrology, cell, area, &mut ledger);
    let occupied_id = materials.receive_felled_log(
        make_log((10.0, 0.0), 20.0),
        &mut hydrology,
        cell,
        area,
        &mut ledger,
    );
    assert_eq!((source_id, occupied_id), (1, 2));

    let source_before = materials.log(source_id).unwrap().clone();
    let occupied_before = materials.log(occupied_id).unwrap().clone();
    let carbon_before = materials.stock_of(R::MaterialCarbon);
    let water_before = materials.stock_of(R::MaterialWater);

    // The first destination id is already owned by the second log. Refusal
    // must leave both source and destination intact, including audited stocks.
    assert!(!materials.cross_cut_log(source_id, 2, occupied_id));
    assert_eq!(materials.log(source_id), Some(&source_before));
    assert_eq!(materials.log(occupied_id), Some(&occupied_before));
    assert_eq!(materials.stock_of(R::MaterialCarbon), carbon_before);
    assert_eq!(materials.stock_of(R::MaterialWater), water_before);
    assert_eq!(materials.next_log_id(), 3);

    // An overflowing destination range is rejected before any mutation too.
    assert!(!materials.cross_cut_log(source_id, 2, u64::MAX));
    assert_eq!(materials.log(source_id), Some(&source_before));
    assert_eq!(materials.log(occupied_id), Some(&occupied_before));
    assert_eq!(materials.stock_of(R::MaterialCarbon), carbon_before);
    assert_eq!(materials.stock_of(R::MaterialWater), water_before);
    assert_eq!(materials.next_log_id(), 3);

    // An unreserved future range is unoccupied, but it is still unsafe:
    // the allocator would remain at id 3 and eventually reuse id 10.
    assert!(!materials.cross_cut_log(source_id, 2, 10));
    assert_eq!(materials.log(source_id), Some(&source_before));
    assert_eq!(materials.log(10), None);
    assert_eq!(materials.log(11), None);
    assert_eq!(materials.stock_of(R::MaterialCarbon), carbon_before);
    assert_eq!(materials.stock_of(R::MaterialWater), water_before);
    assert_eq!(materials.next_log_id(), 3);

    // The direct ledger API refuses an excessive piece count before it can
    // inspect or materialise a caller-supplied destination range.
    assert!(!materials.cross_cut_log(source_id, MAX_CROSS_CUT_PIECES + 1, 3));
    assert_eq!(materials.log(source_id), Some(&source_before));
    assert_eq!(materials.log(occupied_id), Some(&occupied_before));
    assert_eq!(materials.stock_of(R::MaterialCarbon), carbon_before);
    assert_eq!(materials.stock_of(R::MaterialWater), water_before);
    assert_eq!(materials.next_log_id(), 3);

    // The single writer reserves the range. Snapshot restoration must retain
    // that pending reservation before the planned cut materialises it.
    let first_piece_id = materials.reserve_log_ids(2);
    assert_eq!(first_piece_id, 3);
    let snapshot = serde_json::to_string(&materials).expect("reserved ledger serializes");
    materials = serde_json::from_str(&snapshot).expect("reserved ledger restores");

    assert!(materials.cross_cut_log(source_id, 2, first_piece_id));
    assert_eq!(materials.log(source_id), None);
    assert!(materials.log(first_piece_id).is_some());
    assert!(materials.log(first_piece_id + 1).is_some());
    assert_eq!(materials.next_log_id(), 5);

    // Seasoning removes the two stored pieces while retaining their carbon
    // in the wood stock. The old ids are now empty but must stay consumed.
    assert!(materials.season_stored_logs(&mut ledger) > 0.0);
    assert_eq!(materials.log(first_piece_id), None);
    assert_eq!(materials.log(first_piece_id + 1), None);
    let occupied_after_season = materials.log(occupied_id).unwrap().clone();
    let carbon_after_season = materials.stock_of(R::MaterialCarbon);
    let water_after_season = materials.stock_of(R::MaterialWater);

    // The watermark and occupancy checks alone would accept this old range.
    // The pending-reservation check must reject it without changing anything.
    assert!(!materials.cross_cut_log(occupied_id, 2, first_piece_id));
    assert_eq!(materials.log(occupied_id), Some(&occupied_after_season));
    assert_eq!(materials.log(first_piece_id), None);
    assert_eq!(materials.log(first_piece_id + 1), None);
    assert_eq!(materials.stock_of(R::MaterialCarbon), carbon_after_season);
    assert_eq!(materials.stock_of(R::MaterialWater), water_after_season);
    assert_eq!(materials.next_log_id(), 5);
}

#[test]
fn empty_cross_cut_reservations_preserve_the_legacy_material_debug_representation() {
    let materials = MaterialLedger::new();
    assert_eq!(
        format!("{materials:?}"),
        "MaterialLedger { stock: {}, structures: {}, bodies: {}, next_structure: 1, logs: {}, next_log: 0 }"
    );
}

#[test]
fn pending_cross_cut_reservations_are_included_in_material_debug_state() {
    let mut materials = MaterialLedger::new();
    assert_eq!(materials.reserve_log_ids(2), 1);
    assert!(
        format!("{materials:?}").ends_with("reserved_log_ids: {1, 2} }"),
        "pending ids affect which future cross-cuts are valid and must affect the digest"
    );
}

#[test]
fn plant_food_is_only_the_forests_fruit_and_seed_and_grassland_feeds_nobody() {
    use mk_engine::regional::edible::ReproductiveShares;
    use mk_engine::validation::{default_reference_dir, ReferenceLibrary};
    let b = base();
    let area = b.domain.cell_area_m2(M);
    let mut ecology = b.ecology.clone();
    let library = ReferenceLibrary::load(&default_reference_dir()).unwrap();
    ecology.install_edible(&ReproductiveShares::from_reference(&library).unwrap());
    let cell = b.forest_cell;
    let i = ecology.biomass_kgc_m2.grid_to_linear_index(cell.0, cell.1);
    let (mut m, mut ledger) = (MaterialLedger::new(), Ledger::new());
    let audit = Audit::start(&m);

    // A year's fruit and seed: the source's temperate share of NPP, far
    // less than the forest standing on the cell.
    let edible = ecology.edible.as_ref().unwrap().at(i);
    let npp = *ecology.npp_kgc_m2_yr.get(cell.0, cell.1);
    assert!((edible - 0.116 * npp).abs() < 1e-12, "{edible} of {npp}");
    let biomass = *ecology.biomass_kgc_m2.get(cell.0, cell.1);
    assert!(
        edible < biomass / 10.0,
        "food {edible} of a forest of {biomass}"
    );

    // Asking for more than the crop takes the crop and no wood: the
    // carbon leaves the edible stock and the biomass together.
    let carbon_per_kg = composition(Material::PlantFood).carbon_per_kg;
    let got = m
        .gather_edible(1.0e12, &mut ecology, cell, area, &mut ledger)
        .unwrap();
    assert!((got * carbon_per_kg - edible * area).abs() < 1e-3 * edible * area);
    assert!(ecology.edible.as_ref().unwrap().at(i) < 1e-12);
    let lost = biomass - *ecology.biomass_kgc_m2.get(cell.0, cell.1);
    assert!(
        (lost - edible).abs() < 1e-9,
        "the forest lost {lost}, not {edible}"
    );
    assert_eq!(
        m.gather_edible(1.0, &mut ecology, cell, area, &mut ledger),
        Err(MaterialError::NothingToHarvest),
        "the wood that is left is not food"
    );
    audit.close(&m, &ledger);

    // Grass, however much of it, is not food for people.
    let (rows, cols) = (b.domain.rows(M), b.domain.cols(M));
    if let Some(grass) = (0..rows)
        .flat_map(|r| (0..cols).map(move |c| (r, c)))
        .find(|&(r, c)| {
            *ecology.biome_grid.get(r, c) == BiomeType::Grassland
                && *ecology.biomass_kgc_m2.get(r, c) > 0.0
        })
    {
        assert_eq!(
            m.gather_edible(1.0, &mut ecology, grass, area, &mut ledger),
            Err(MaterialError::NothingToHarvest)
        );
    }
}

#[test]
fn receiving_zero_dry_mass_log_fails_before_allocating_or_booking_material() {
    use std::panic::{catch_unwind, AssertUnwindSafe};

    let b = base();
    let cell = b.forest_cell;
    let area = b.domain.cell_area_m2(M);
    let mut hydrology = HydrologyState::new(&b.domain.storage_spec(M), 0.5);
    let mut materials = MaterialLedger::new();
    let mut ledger = Ledger::new();
    let audit = Audit::start(&materials);
    let log = Log {
        id: 0,
        position_m: (0.0, 0.0),
        heading_rad: 0.0,
        length_m: 2.0,
        diameter_m: 0.2,
        dry_kg: 0.0,
        carbon_kgc: 0.0,
        water_kg: 0.0,
        place: LogPlace::Ground,
    };
    let next_id_before = materials.next_log_id();

    let result = catch_unwind(AssertUnwindSafe(|| {
        materials.receive_felled_log(log, &mut hydrology, cell, area, &mut ledger)
    }));

    assert!(
        result.is_err(),
        "a positive-volume log cannot have zero dry mass"
    );
    assert_eq!(materials.next_log_id(), next_id_before);
    assert_eq!(materials.logs().count(), 0);
    audit.close(&materials, &ledger);
}

#[test]
fn receiving_zero_carbon_log_fails_before_allocating_or_booking_material() {
    use std::panic::{catch_unwind, AssertUnwindSafe};

    let b = base();
    let cell = b.forest_cell;
    let area = b.domain.cell_area_m2(M);
    let mut hydrology = HydrologyState::new(&b.domain.storage_spec(M), 0.5);
    let mut materials = MaterialLedger::new();
    let mut ledger = Ledger::new();
    let audit = Audit::start(&materials);
    let log = Log {
        id: 0,
        position_m: (0.0, 0.0),
        heading_rad: 0.0,
        length_m: 2.0,
        diameter_m: 0.2,
        dry_kg: 10.0,
        carbon_kgc: 0.0,
        water_kg: 8.2,
        place: LogPlace::Ground,
    };
    let next_id_before = materials.next_log_id();

    let result = catch_unwind(AssertUnwindSafe(|| {
        materials.receive_felled_log(log, &mut hydrology, cell, area, &mut ledger)
    }));

    assert!(result.is_err(), "dry wood must contain carbon");
    assert_eq!(materials.next_log_id(), next_id_before);
    assert_eq!(materials.logs().count(), 0);
    audit.close(&materials, &ledger);
}

#[test]
fn biotic_edible_and_soil_water_gathering_reject_invalid_amounts_without_mutation() {
    use mk_engine::hydrology::SoilWater;

    let b = base();
    let cell = b.forest_cell;
    let area = b.domain.cell_area_m2(M);
    let mut ecology = b.ecology.clone();
    let edible_index = ecology.biomass_kgc_m2.grid_to_linear_index(cell.0, cell.1);
    let cell_count = b.domain.rows(M) * b.domain.cols(M);
    let mut edible = EdibleFood {
        share: vec![0.116; cell_count],
        kgc_m2: vec![0.0; cell_count],
    };
    // This low, finite test stock is below the selected forest cell's biomass.
    edible.kgc_m2[edible_index] = 0.1;
    ecology.edible = Some(edible);
    let mut hydrology = HydrologyState::new(&b.domain.storage_spec(M), 0.5);
    let (mut materials, mut ledger) = (MaterialLedger::new(), Ledger::new());
    let biomass_before = *ecology.biomass_kgc_m2.get(cell.0, cell.1);
    let edible_before = ecology
        .edible
        .as_ref()
        .expect("installed edible layer")
        .at(edible_index);
    let soil_before: SoilWater = *hydrology.soil_water.get(cell.0, cell.1);

    for amount in [-1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(matches!(
            materials.gather_biotic(
                Material::Wood,
                amount,
                &mut ecology,
                cell,
                area,
                &mut ledger
            ),
            Err(MaterialError::InvalidAmount {
                material: Material::Wood,
                amount_kg
            }) if amount_kg.to_bits() == amount.to_bits()
        ));
        assert_eq!(*ecology.biomass_kgc_m2.get(cell.0, cell.1), biomass_before);
        assert_eq!(materials.stock_kg(Material::Wood), 0.0);
        assert!(ledger.entries().is_empty());

        assert!(matches!(
            materials.gather_edible(amount, &mut ecology, cell, area, &mut ledger),
            Err(MaterialError::InvalidAmount {
                material: Material::PlantFood,
                amount_kg
            }) if amount_kg.to_bits() == amount.to_bits()
        ));
        assert_eq!(*ecology.biomass_kgc_m2.get(cell.0, cell.1), biomass_before);
        assert_eq!(
            ecology
                .edible
                .as_ref()
                .expect("edible layer")
                .at(edible_index),
            edible_before
        );
        assert_eq!(materials.stock_kg(Material::PlantFood), 0.0);
        assert!(ledger.entries().is_empty());

        assert!(matches!(
            materials.gather_soil_water(amount, &mut hydrology, cell, area, &mut ledger),
            Err(MaterialError::InvalidAmount {
                material: Material::Water,
                amount_kg
            }) if amount_kg.to_bits() == amount.to_bits()
        ));
        let soil_after: SoilWater = *hydrology.soil_water.get(cell.0, cell.1);
        assert_eq!(soil_after.storage_mm, soil_before.storage_mm);
        assert_eq!(soil_after.moisture_fraction, soil_before.moisture_fraction);
        assert_eq!(materials.stock_kg(Material::Water), 0.0);
        assert!(ledger.entries().is_empty());
    }
}

#[test]
fn edible_gathering_rejects_invalid_source_stock_before_mutation() {
    let b = base();
    let cell = b.forest_cell;
    let area = b.domain.cell_area_m2(M);
    let mut ecology = b.ecology.clone();
    let edible_index = ecology.biomass_kgc_m2.grid_to_linear_index(cell.0, cell.1);
    let cell_count = b.domain.rows(M) * b.domain.cols(M);
    let mut edible = EdibleFood {
        share: vec![0.116; cell_count],
        kgc_m2: vec![0.0; cell_count],
    };
    edible.kgc_m2[edible_index] = f64::NAN;
    ecology.edible = Some(edible);

    let biomass_before = *ecology.biomass_kgc_m2.get(cell.0, cell.1);
    let (mut materials, mut ledger) = (MaterialLedger::new(), Ledger::new());

    assert!(matches!(
        materials.gather_edible(1.0, &mut ecology, cell, area, &mut ledger),
        Err(MaterialError::InvalidSourceState {
            material: Material::PlantFood,
            value
        }) if value.is_nan()
    ));
    assert_eq!(*ecology.biomass_kgc_m2.get(cell.0, cell.1), biomass_before);
    assert!(ecology.edible.as_ref().unwrap().at(edible_index).is_nan());
    assert_eq!(materials.stock_kg(Material::PlantFood), 0.0);
    assert!(ledger.entries().is_empty());
}

#[test]
fn biotic_and_soil_water_gathering_reject_stock_overflow_before_source_mutation() {
    use mk_engine::hydrology::SoilWater;

    let b = base();
    let cell = b.forest_cell;
    let area = b.domain.cell_area_m2(M);
    let mut ecology = b.ecology.clone();
    let mut hydrology = HydrologyState::new(&b.domain.storage_spec(M), 0.5);
    let (mut materials, mut ledger) = (MaterialLedger::new(), Ledger::new());

    assert_eq!(
        materials
            .inject(Material::Wood, f64::MAX, &mut ledger)
            .expect("finite maximum wood seed is valid"),
        f64::MAX,
    );
    let biomass_before = (0.4 * f64::MAX) / area;
    ecology.biomass_kgc_m2.set(cell.0, cell.1, biomass_before);
    let ledger_entries_before = ledger.entries().len();
    assert!(matches!(
        materials.gather_biotic(
            Material::Wood,
            f64::MAX,
            &mut ecology,
            cell,
            area,
            &mut ledger
        ),
        Err(MaterialError::InvalidAmount {
            material: Material::Wood,
            amount_kg
        }) if amount_kg.is_infinite()
    ));
    assert_eq!(*ecology.biomass_kgc_m2.get(cell.0, cell.1), biomass_before);
    assert_eq!(materials.stock_kg(Material::Wood), f64::MAX);
    assert_eq!(ledger.entries().len(), ledger_entries_before);

    assert_eq!(
        materials
            .inject(Material::Water, f64::MAX, &mut ledger)
            .expect("finite maximum water seed is valid"),
        f64::MAX,
    );
    let soil_before = (f64::MAX / 2.0) / area;
    hydrology.soil_water.set(
        cell.0,
        cell.1,
        SoilWater {
            moisture_fraction: 0.5,
            storage_mm: soil_before,
        },
    );
    let ledger_entries_before = ledger.entries().len();
    assert!(matches!(
        materials.gather_soil_water(
            f64::MAX / 2.0,
            &mut hydrology,
            cell,
            area,
            &mut ledger
        ),
        Err(MaterialError::InvalidAmount {
            material: Material::Water,
            amount_kg
        }) if amount_kg.is_infinite()
    ));
    let soil_after: SoilWater = *hydrology.soil_water.get(cell.0, cell.1);
    assert_eq!(soil_after.storage_mm, soil_before);
    assert_eq!(materials.stock_kg(Material::Water), f64::MAX);
    assert_eq!(ledger.entries().len(), ledger_entries_before);
}

#[test]
fn biotic_and_edible_harvests_reject_invalid_source_reservoirs_without_mutation() {
    use mk_engine::regional::edible::ReproductiveShares;
    use mk_engine::validation::{default_reference_dir, ReferenceLibrary};

    let b = base();
    let cell = b.forest_cell;
    let area = b.domain.cell_area_m2(M);
    let invalid_values = [-1.0, f64::NAN, f64::INFINITY];

    for invalid in invalid_values {
        let mut ecology = b.ecology.clone();
        ecology.biomass_kgc_m2.set(cell.0, cell.1, invalid);
        let (mut materials, mut ledger) = (MaterialLedger::new(), Ledger::new());

        assert!(matches!(
            materials.gather_biotic(Material::Wood, 1.0, &mut ecology, cell, area, &mut ledger),
            Err(MaterialError::InvalidSourceState { material: Material::Wood, value })
                if value.to_bits() == invalid.to_bits()
        ));
        assert_eq!(materials.stock_kg(Material::Wood), 0.0);
        assert!(ledger.entries().is_empty());
        assert_eq!(
            ecology.biomass_kgc_m2.get(cell.0, cell.1).to_bits(),
            invalid.to_bits()
        );
    }

    let library = ReferenceLibrary::load(&default_reference_dir()).unwrap();
    for invalid in invalid_values {
        let mut ecology = b.ecology.clone();
        ecology.install_edible(&ReproductiveShares::from_reference(&library).unwrap());
        let index = ecology.biomass_kgc_m2.grid_to_linear_index(cell.0, cell.1);
        let biomass_before = *ecology.biomass_kgc_m2.get(cell.0, cell.1);
        ecology.edible.as_mut().unwrap().kgc_m2[index] = invalid;
        let (mut materials, mut ledger) = (MaterialLedger::new(), Ledger::new());

        assert!(matches!(
            materials.gather_edible(1.0, &mut ecology, cell, area, &mut ledger),
            Err(MaterialError::InvalidSourceState { material: Material::PlantFood, value })
                if value.to_bits() == invalid.to_bits()
        ));
        assert_eq!(*ecology.biomass_kgc_m2.get(cell.0, cell.1), biomass_before);
        assert_eq!(
            ecology.edible.as_ref().unwrap().kgc_m2[index].to_bits(),
            invalid.to_bits()
        );
        assert_eq!(materials.stock_kg(Material::PlantFood), 0.0);
        assert!(ledger.entries().is_empty());
    }
}

#[test]
fn soil_and_surface_water_harvests_reject_invalid_source_reservoirs_without_mutation() {
    use mk_engine::hydrology::SoilWater;

    let b = base();
    let cell = b.forest_cell;
    let area = b.domain.cell_area_m2(M);
    let spec = b.domain.storage_spec(M);
    let invalid_values = [-1.0, f64::NAN, f64::INFINITY];

    for invalid in invalid_values {
        let mut hydrology = HydrologyState::new(&spec, 0.5);
        hydrology.soil_water.set(
            cell.0,
            cell.1,
            SoilWater {
                moisture_fraction: 0.5,
                storage_mm: invalid,
            },
        );
        let (mut materials, mut ledger) = (MaterialLedger::new(), Ledger::new());

        assert!(matches!(
            materials.gather_soil_water(1.0, &mut hydrology, cell, area, &mut ledger),
            Err(MaterialError::InvalidSourceState { material: Material::Water, value })
                if value.to_bits() == invalid.to_bits()
        ));
        assert_eq!(
            hydrology
                .soil_water
                .get(cell.0, cell.1)
                .storage_mm
                .to_bits(),
            invalid.to_bits()
        );
        assert_eq!(materials.stock_kg(Material::Water), 0.0);
        assert!(ledger.entries().is_empty());
    }

    for invalid in invalid_values {
        let mut hydrology = HydrologyState::new(&spec, 0.5);
        hydrology.surface_water.set(cell.0, cell.1, invalid);
        let runoff_before = *hydrology.runoff.get(cell.0, cell.1);
        let (mut materials, mut ledger) = (MaterialLedger::new(), Ledger::new());

        assert!(matches!(
            materials.gather_water(1.0, &mut hydrology, cell, area, &mut ledger),
            Err(MaterialError::InvalidSourceState { material: Material::Water, value })
                if value.to_bits() == invalid.to_bits()
        ));
        assert_eq!(
            hydrology.surface_water.get(cell.0, cell.1).to_bits(),
            invalid.to_bits()
        );
        assert_eq!(*hydrology.runoff.get(cell.0, cell.1), runoff_before);
        assert_eq!(materials.stock_kg(Material::Water), 0.0);
        assert!(ledger.entries().is_empty());
    }

    let mut hydrology = HydrologyState::new(&spec, 0.5);
    hydrology.runoff.set(cell.0, cell.1, f64::INFINITY);
    let surface_before = *hydrology.surface_water.get(cell.0, cell.1);
    let (mut materials, mut ledger) = (MaterialLedger::new(), Ledger::new());
    assert!(matches!(
        materials.gather_water(1.0, &mut hydrology, cell, area, &mut ledger),
        Err(MaterialError::InvalidSourceState { material: Material::Water, value })
            if value.is_infinite()
    ));
    assert_eq!(*hydrology.surface_water.get(cell.0, cell.1), surface_before);
    assert!(ledger.entries().is_empty());
}

#[test]
fn cell_harvests_reject_nonpositive_or_nonfinite_area_before_mutation() {
    use mk_engine::regional::edible::ReproductiveShares;
    use mk_engine::validation::{default_reference_dir, ReferenceLibrary};

    let b = base();
    let cell = b.forest_cell;
    let spec = b.domain.storage_spec(M);
    let library = ReferenceLibrary::load(&default_reference_dir()).unwrap();

    for invalid_area in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        let mut ecology = b.ecology.clone();
        let (mut materials, mut ledger) = (MaterialLedger::new(), Ledger::new());
        assert!(matches!(
            materials.gather_biotic(Material::Wood, 1.0, &mut ecology, cell, invalid_area, &mut ledger),
            Err(MaterialError::InvalidCellArea { area_m2 })
                if area_m2.to_bits() == invalid_area.to_bits()
        ));
        assert_eq!(materials.stock_kg(Material::Wood), 0.0);
        assert!(ledger.entries().is_empty());

        ecology.install_edible(&ReproductiveShares::from_reference(&library).unwrap());
        assert!(matches!(
            materials.gather_edible(1.0, &mut ecology, cell, invalid_area, &mut ledger),
            Err(MaterialError::InvalidCellArea { area_m2 })
                if area_m2.to_bits() == invalid_area.to_bits()
        ));
        assert_eq!(materials.stock_kg(Material::PlantFood), 0.0);
        assert!(ledger.entries().is_empty());

        let mut soil = HydrologyState::new(&spec, 0.5);
        assert!(matches!(
            materials.gather_soil_water(1.0, &mut soil, cell, invalid_area, &mut ledger),
            Err(MaterialError::InvalidCellArea { area_m2 })
                if area_m2.to_bits() == invalid_area.to_bits()
        ));
        assert_eq!(materials.stock_kg(Material::Water), 0.0);
        assert!(ledger.entries().is_empty());

        let mut surface = HydrologyState::new(&spec, 0.5);
        assert!(matches!(
            materials.gather_water(1.0, &mut surface, cell, invalid_area, &mut ledger),
            Err(MaterialError::InvalidCellArea { area_m2 })
                if area_m2.to_bits() == invalid_area.to_bits()
        ));
        assert_eq!(materials.stock_kg(Material::Water), 0.0);
        assert!(ledger.entries().is_empty());
    }
}

#[test]
fn receiving_felled_wood_rejects_rounded_away_stock_before_booking_carbon() {
    use std::panic::{catch_unwind, AssertUnwindSafe};

    let mut materials = MaterialLedger::new();
    let mut ledger = Ledger::new();
    materials
        .inject(Material::Wood, f64::MAX, &mut ledger)
        .expect("the initial Wood stock is finite");
    let stock_before = materials.stock_kg(Material::Wood);
    let carbon_before = materials.stock_of(R::MaterialCarbon);
    let entries_before = ledger.entries().len();

    let result = catch_unwind(AssertUnwindSafe(|| {
        materials.receive_felled_wood(0.425, &mut ledger)
    }));

    assert!(result.is_err(), "the one-kilogram inflow rounds away");
    assert_eq!(materials.stock_kg(Material::Wood), stock_before);
    assert_eq!(materials.stock_of(R::MaterialCarbon), carbon_before);
    assert_eq!(ledger.entries().len(), entries_before);
}

#[test]
fn roof_water_capture_rejects_rounded_away_stock_before_debiting_soil() {
    use std::panic::{catch_unwind, AssertUnwindSafe};

    let b = base();
    let cell = b.forest_cell;
    let area = b.domain.cell_area_m2(M);
    let mut hydrology = HydrologyState::new(&b.domain.storage_spec(M), 0.5);
    hydrology.soil_water.get_mut(cell.0, cell.1).storage_mm = 100.0;
    let mut materials = MaterialLedger::new();
    let mut ledger = Ledger::new();
    materials
        .inject(Material::Water, f64::MAX, &mut ledger)
        .expect("the initial water stock is finite");
    let stock_before = materials.stock_kg(Material::Water);
    let water_before = materials.stock_of(R::MaterialWater);
    let soil_before = hydrology.soil_water.get(cell.0, cell.1).storage_mm;
    let entries_before = ledger.entries().len();

    let result = catch_unwind(AssertUnwindSafe(|| {
        materials.capture_roof_water(1.0, &mut hydrology, cell, area, &mut ledger)
    }));

    assert!(result.is_err(), "the one-kilogram inflow rounds away");
    assert_eq!(materials.stock_kg(Material::Water), stock_before);
    assert_eq!(materials.stock_of(R::MaterialWater), water_before);
    assert_eq!(
        hydrology.soil_water.get(cell.0, cell.1).storage_mm,
        soil_before
    );
    assert_eq!(ledger.entries().len(), entries_before);
}

#[test]
fn roof_water_capture_rejects_source_debit_that_rounds_away_before_stock_mutation() {
    use std::panic::{catch_unwind, AssertUnwindSafe};

    let b = base();
    let cell = b.forest_cell;
    let mut hydrology = HydrologyState::new(&b.domain.storage_spec(M), 0.5);
    let mut soil = *hydrology.soil_water.get(cell.0, cell.1);
    soil.storage_mm = 100.0;
    soil.moisture_fraction = soil.storage_mm / 300.0;
    hydrology.soil_water.set(cell.0, cell.1, soil);
    let mut materials = MaterialLedger::new();
    let mut ledger = Ledger::new();
    let soil_before = *hydrology.soil_water.get(cell.0, cell.1);

    let result = catch_unwind(AssertUnwindSafe(|| {
        materials.capture_roof_water(1.0, &mut hydrology, cell, 1.0e18, &mut ledger)
    }));

    assert!(
        result.is_err(),
        "one kg over a 1e18 m2 cell cannot change the source soil depth"
    );
    assert_eq!(materials.stock_kg(Material::Water), 0.0);
    assert_eq!(materials.stock_of(R::MaterialWater), 0.0);
    assert_eq!(*hydrology.soil_water.get(cell.0, cell.1), soil_before);
    assert!(ledger.entries().is_empty());
}

#[test]
fn receiving_felled_log_rejects_unrepresentable_carbon_before_allocating_or_debiting_soil() {
    use std::panic::{catch_unwind, AssertUnwindSafe};

    let b = base();
    let cell = b.forest_cell;
    let area = b.domain.cell_area_m2(M);
    let mut hydrology = HydrologyState::new(&b.domain.storage_spec(M), 0.5);
    let mut materials = MaterialLedger::new();
    let mut ledger = Ledger::new();
    materials
        .inject(Material::Coal, f64::MAX, &mut ledger)
        .expect("the initial Coal stock is finite");
    let carbon_before = materials.stock_of(R::MaterialCarbon);
    let soil_before = hydrology.soil_water.get(cell.0, cell.1).storage_mm;
    let next_id_before = materials.next_log_id();
    let entries_before = ledger.entries().len();
    let log = Log {
        id: 0,
        position_m: (0.0, 0.0),
        heading_rad: 0.0,
        length_m: 2.0,
        diameter_m: 0.2,
        dry_kg: 10.0,
        carbon_kgc: 5.0,
        water_kg: 8.2,
        place: LogPlace::Ground,
    };

    let result = catch_unwind(AssertUnwindSafe(|| {
        materials.receive_felled_log(log, &mut hydrology, cell, area, &mut ledger)
    }));

    assert!(
        result.is_err(),
        "the log carbon disappears in aggregate stock"
    );
    assert_eq!(materials.next_log_id(), next_id_before);
    assert_eq!(materials.logs().count(), 0);
    assert_eq!(materials.stock_of(R::MaterialCarbon), carbon_before);
    assert_eq!(
        hydrology.soil_water.get(cell.0, cell.1).storage_mm,
        soil_before
    );
    assert_eq!(ledger.entries().len(), entries_before);
}

#[test]
fn receiving_felled_log_rejects_unrepresentable_water_before_allocating_or_debiting_soil() {
    use std::panic::{catch_unwind, AssertUnwindSafe};

    let b = base();
    let cell = b.forest_cell;
    let area = b.domain.cell_area_m2(M);
    let mut hydrology = HydrologyState::new(&b.domain.storage_spec(M), 0.5);
    let mut materials = MaterialLedger::new();
    let mut ledger = Ledger::new();
    materials
        .inject(Material::Water, f64::MAX, &mut ledger)
        .expect("the initial water stock is finite");
    let water_before = materials.stock_of(R::MaterialWater);
    let soil_before = hydrology.soil_water.get(cell.0, cell.1).storage_mm;
    let next_id_before = materials.next_log_id();
    let entries_before = ledger.entries().len();
    let log = Log {
        id: 0,
        position_m: (0.0, 0.0),
        heading_rad: 0.0,
        length_m: 2.0,
        diameter_m: 0.2,
        dry_kg: 10.0,
        carbon_kgc: 5.0,
        water_kg: 8.2,
        place: LogPlace::Ground,
    };

    let result = catch_unwind(AssertUnwindSafe(|| {
        materials.receive_felled_log(log, &mut hydrology, cell, area, &mut ledger)
    }));

    assert!(
        result.is_err(),
        "the log water disappears in aggregate stock"
    );
    assert_eq!(materials.next_log_id(), next_id_before);
    assert_eq!(materials.logs().count(), 0);
    assert_eq!(materials.stock_of(R::MaterialWater), water_before);
    assert_eq!(
        hydrology.soil_water.get(cell.0, cell.1).storage_mm,
        soil_before
    );
    assert_eq!(ledger.entries().len(), entries_before);
}

#[test]
fn seasoning_preflights_the_complete_wood_batch_before_removing_any_log() {
    use std::panic::{catch_unwind, AssertUnwindSafe};

    let b = base();
    let cell = b.forest_cell;
    let area = b.domain.cell_area_m2(M);
    let mut hydrology = HydrologyState::new(&b.domain.storage_spec(M), 0.5);
    let mut materials = MaterialLedger::new();
    let mut ledger = Ledger::new();
    let log = Log {
        id: 0,
        position_m: (0.0, 0.0),
        heading_rad: 0.0,
        length_m: 2.0,
        diameter_m: 0.2,
        dry_kg: 1.0,
        carbon_kgc: 0.425,
        water_kg: 0.0,
        place: LogPlace::Stored,
    };
    let id = materials.receive_felled_log(log, &mut hydrology, cell, area, &mut ledger);
    materials
        .inject(Material::Wood, f64::MAX, &mut ledger)
        .expect("the initial Wood stock is finite");
    let stored_before = materials.log(id).cloned();
    let wood_before = materials.stock_kg(Material::Wood);
    let entries_before = ledger.entries().len();

    let result = catch_unwind(AssertUnwindSafe(|| {
        materials.season_stored_logs(&mut ledger)
    }));

    assert!(
        result.is_err(),
        "the seasoned log cannot increase Wood stock"
    );
    assert_eq!(materials.log(id), stored_before.as_ref());
    assert_eq!(materials.stock_kg(Material::Wood), wood_before);
    assert_eq!(ledger.entries().len(), entries_before);
}

#[test]
fn seasoning_aggregates_small_logs_before_preflighting_large_wood_stock() {
    let b = base();
    let cell = b.forest_cell;
    let area = b.domain.cell_area_m2(M);
    let mut hydrology = HydrologyState::new(&b.domain.storage_spec(M), 0.5);
    let mut materials = MaterialLedger::new();
    let mut ledger = Ledger::new();
    for offset in [0.0, 10.0] {
        let log = Log {
            id: 0,
            position_m: (offset, 0.0),
            heading_rad: 0.0,
            length_m: 2.0,
            diameter_m: 0.2,
            dry_kg: 1.0,
            carbon_kgc: 0.425,
            water_kg: 0.0,
            place: LogPlace::Stored,
        };
        materials.receive_felled_log(log, &mut hydrology, cell, area, &mut ledger);
    }
    let first_id = 1;
    let second_id = 2;
    materials
        .inject(Material::Wood, 2_f64.powi(53), &mut ledger)
        .expect("the initial Wood stock is finite");

    // The audit baseline starts after setup. Only seasoning's new ledger entries
    // belong in the delta comparison; felled-log and seed-stock flows are history.
    let mut seasoning_ledger = Ledger::new();
    let audit = Audit::start(&materials);

    let added = materials.season_stored_logs(&mut seasoning_ledger);

    assert_eq!(added, 2.0);
    assert_eq!(materials.stock_kg(Material::Wood), 2_f64.powi(53) + 2.0);
    assert!(materials.log(first_id).is_none());
    assert!(materials.log(second_id).is_none());
    audit.close(&materials, &seasoning_ledger);
}

#[test]
fn roof_water_capture_moves_audited_soil_water_into_the_material_stock() {
    let b = base();
    let cell = b.forest_cell;
    let area = b.domain.cell_area_m2(M);
    let mut hydrology = HydrologyState::new(&b.domain.storage_spec(M), 0.5);
    hydrology.soil_water.get_mut(cell.0, cell.1).storage_mm = 100.0;
    let soil_before = hydrology.soil_water.get(cell.0, cell.1).storage_mm;
    let mut materials = MaterialLedger::new();
    let mut ledger = Ledger::new();
    let audit = Audit::start(&materials);

    let caught = materials.capture_roof_water(1.0, &mut hydrology, cell, area, &mut ledger);

    assert_eq!(caught, 1.0);
    assert_eq!(materials.stock_kg(Material::Water), 1.0);
    assert!(
        (soil_before - hydrology.soil_water.get(cell.0, cell.1).storage_mm - 1.0 / area).abs()
            < 1e-12
    );
    audit.close(&materials, &ledger);
}

#[test]
fn seasoning_rejects_wood_conversion_that_would_change_material_carbon_audit() {
    use std::panic::{catch_unwind, AssertUnwindSafe};

    let b = base();
    let cell = b.forest_cell;
    let area = b.domain.cell_area_m2(M);
    let mut hydrology = HydrologyState::new(&b.domain.storage_spec(M), 0.5);
    let mut materials = MaterialLedger::new();
    let mut ledger = Ledger::new();
    let log = Log {
        id: 0,
        position_m: (0.0, 0.0),
        heading_rad: 0.0,
        length_m: 2.0,
        diameter_m: 0.2,
        dry_kg: 10.0,
        carbon_kgc: 4.25,
        water_kg: 0.0,
        place: LogPlace::Stored,
    };
    let id = materials.receive_felled_log(log, &mut hydrology, cell, area, &mut ledger);
    materials
        .inject(Material::Wood, 9_007_199_254_740_992.0, &mut ledger)
        .expect("the initial Wood stock is finite");

    let wood_before = materials.stock_kg(Material::Wood);
    let carbon_before = materials.stock_of(R::MaterialCarbon);
    let stored_before = materials.log(id).cloned();
    let entries_before = ledger.entries().len();

    let result = catch_unwind(AssertUnwindSafe(|| {
        materials.season_stored_logs(&mut ledger)
    }));

    assert!(
        result.is_err(),
        "the 10 kg Wood increment changes the rounded carbon aggregate by 0.5 kgC"
    );
    assert_eq!(materials.stock_kg(Material::Wood), wood_before);
    assert_eq!(materials.stock_of(R::MaterialCarbon), carbon_before);
    assert_eq!(materials.log(id), stored_before.as_ref());
    assert_eq!(ledger.entries().len(), entries_before);
}

#[test]
fn seasoning_rejects_water_debit_that_would_not_match_aggregate_stock_delta() {
    use std::panic::{catch_unwind, AssertUnwindSafe};

    let b = base();
    let cell = b.forest_cell;
    let area = b.domain.cell_area_m2(M);
    let mut hydrology = HydrologyState::new(&b.domain.storage_spec(M), 0.5);
    let mut materials = MaterialLedger::new();
    let mut ledger = Ledger::new();
    let mut ids = Vec::new();

    for offset in [0.0, 10.0, 20.0] {
        let log = Log {
            id: 0,
            position_m: (offset, 0.0),
            heading_rad: 0.0,
            length_m: 2.0,
            diameter_m: 0.2,
            dry_kg: 10.0,
            carbon_kgc: 4.25,
            water_kg: 0.5,
            place: LogPlace::Stored,
        };
        ids.push(materials.receive_felled_log(log, &mut hydrology, cell, area, &mut ledger));
    }
    materials
        .inject(Material::Water, 9_007_199_254_740_992.0, &mut ledger)
        .expect("the initial Water stock is finite");

    let wood_before = materials.stock_kg(Material::Wood);
    let water_stock_before = materials.stock_kg(Material::Water);
    let water_before = materials.stock_of(R::MaterialWater);
    let logs_before: Vec<_> = materials.logs().cloned().collect();
    let entries_before = ledger.entries().len();

    let result = catch_unwind(AssertUnwindSafe(|| {
        materials.season_stored_logs(&mut ledger)
    }));

    assert!(
        result.is_err(),
        "the rounded aggregate falls by 2 kg while the three log fluxes book 1.5 kg"
    );
    assert_eq!(materials.stock_kg(Material::Wood), wood_before);
    assert_eq!(materials.stock_kg(Material::Water), water_stock_before);
    assert_eq!(materials.stock_of(R::MaterialWater), water_before);
    assert_eq!(materials.logs().map(|log| log.id).collect::<Vec<_>>(), ids);
    assert_eq!(materials.logs().cloned().collect::<Vec<_>>(), logs_before);
    assert_eq!(ledger.entries().len(), entries_before);
}
