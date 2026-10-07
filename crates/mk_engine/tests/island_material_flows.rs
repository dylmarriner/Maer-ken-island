//! Phase 3 Task 3: gathered materials move real carbon, oxygen and water.

use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use mk_core::biomes::BiomeType;
use mk_core::canon::CanonLocked;
use mk_core::flux::{FluxKind, Ledger, Reservoir, Reservoir as R};
use mk_engine::conservation::O2_PER_CARBON;
use mk_engine::hydrology::HydrologyState;
use mk_engine::regional::ecology::RegionalEcologyState;
use mk_engine::regional::materials::{
    composition, respired_carbon_kg_per_kcal, AnimalPopulation, Material, MaterialError,
    MaterialLedger, ALL_MATERIALS, QUICKLIME_PER_LIMESTONE,
};
use mk_engine::regional::physical::RegionalPhysicalState;
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
    m.on_death("Gem-D", &mut ledger).unwrap();
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
    let lost = m.lose_water("Gem-K", 5.0, 0.6, &mut ledger).unwrap();
    assert_eq!(lost, 2.0, "only what the body holds is lost");
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
