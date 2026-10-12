//! Phase 3 Task 4: the founders' estate placed on the owner's island.

use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use mk_core::canon::CanonLocked;
use mk_core::grid::Grid2;
use mk_engine::organisms::property::{PropertyBuildingKind, PropertyItemKind};
use mk_engine::regional::ecology::RegionalEcologyState;
use mk_engine::regional::industry::hydro::{ENVIRONMENTAL_FLOW_FRACTION, FIRM_FLOW_EXCEEDANCE};
use mk_engine::regional::industry::hydro_sites::{ESTATE_HYDRO_KW, TOWN_HYDRO_KW};
use mk_engine::regional::physical::RegionalPhysicalState;
use mk_engine::regional::property::{
    bootstrap_regional_property, choose_regional_estate_location, estate_patch_spec,
    place_regional_estate, EstatePatchError, PlacedEstate,
};
use mk_island::{DomainLevel, EstatePatchConfig, IslandDomain, IslandScenario};

const M: DomainLevel = DomainLevel::Medium;

struct Base {
    canon: Arc<CanonLocked>,
    domain: IslandDomain,
    physical: RegionalPhysicalState,
    ecology: RegionalEcologyState,
    config: EstatePatchConfig,
    seed: [u8; 32],
    placed: PlacedEstate,
}

fn repo(path: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path)
}

fn base() -> &'static Base {
    static BASE: OnceLock<Base> = OnceLock::new();
    BASE.get_or_init(|| {
        let scenario =
            IslandScenario::load(&repo("fixtures/island/default_scenario.json")).unwrap();
        let canon = Arc::new(CanonLocked::load(&repo("fixtures/island/canon.json")).unwrap());
        let domain = IslandDomain::from_profile(scenario.profile.clone()).unwrap();
        let physical = RegionalPhysicalState::bootstrap(&canon, &domain, scenario.seed).unwrap();
        let ecology = RegionalEcologyState::bootstrap(&domain, &physical).unwrap();
        let placed = place_regional_estate(
            &canon,
            &physical,
            &ecology,
            &domain,
            &scenario.estate_patch,
            scenario.seed,
        )
        .expect("the island has a valid estate site");
        Base {
            canon,
            domain,
            physical,
            ecology,
            config: scenario.estate_patch,
            seed: scenario.seed,
            placed,
        }
    })
}

#[test]
fn placement_is_deterministic_on_dry_buildable_land_beside_a_river() {
    let b = base();
    let again = place_regional_estate(
        &b.canon,
        &b.physical,
        &b.ecology,
        &b.domain,
        &b.config,
        b.seed,
    )
    .unwrap();
    assert_eq!(again.location, b.placed.location);
    assert_eq!(
        serde_json::to_string(&again.layout).unwrap(),
        serde_json::to_string(&b.placed.layout).unwrap()
    );
    // The best-scoring site is the first the layout tried; the placed one
    // is among the first few ranked.
    let top =
        choose_regional_estate_location(&b.canon, &b.physical, &b.ecology, &b.domain).unwrap();
    println!("best-scoring site {top:?}, placed {:?}", b.placed.location);

    let (row, col) = b.placed.location;
    let elev = &b.physical.geophysics.elevation_m;
    let net = b.physical.flow_network();
    for (r, c) in [
        (row, col),
        (row, col + 1),
        (row + 1, col),
        (row + 1, col + 1),
    ] {
        assert!(*elev.get(r, c) > 0.0, "block cell ({r},{c}) is not dry");
        assert_eq!(net.lake_depth_m(r, c), 0.0);
        assert!(!matches!(
            b.ecology.biome_grid.get(r, c),
            mk_core::biomes::BiomeType::Volcanic | mk_core::biomes::BiomeType::River
        ));
    }
    // A perennial river beside the block runs the estate's micro-hydro:
    // the powerhouse is within 1 km of the block, the intake upstream on
    // the same river, and at the firm flow the plant takes its full design
    // flow with the environmental flow left in the river.
    let hydro = b
        .placed
        .hydro
        .as_ref()
        .expect("the estate has a hydro site");
    let site = &hydro.estate;
    let (pr, pc) = site.powerhouse_cell;
    assert!(
        (row.saturating_sub(1)..=row + 2).contains(&pr)
            && (col.saturating_sub(1)..=col + 2).contains(&pc),
        "powerhouse {:?} is not beside the block",
        site.powerhouse_cell
    );
    let cols = b.domain.cols(M);
    let intake = b
        .physical
        .river_flows
        .get(site.intake_cell.0 * cols + site.intake_cell.1)
        .expect("the intake is on a perennial river");
    assert!((site.firm_flow_m3_s - intake.flow_at(FIRM_FLOW_EXCEEDANCE)).abs() < 1e-9);
    assert!(
        (site.environmental_flow_m3_s - ENVIRONMENTAL_FLOW_FRACTION * f64::from(intake.mean_m3_s))
            .abs()
            < 1e-9
    );
    assert!(site.firm_flow_m3_s - site.environmental_flow_m3_s >= site.design_flow_m3_s);
    assert!(site.head_m > 0.0 && site.penstock_length_m > 0.0);
    assert!(
        (site.design.rated_kw - ESTATE_HYDRO_KW).abs() < 0.1,
        "rated {} kW",
        site.design.rated_kw
    );
    // The water drains from the intake down to the powerhouse.
    let mut cell = site.intake_cell;
    let mut steps = 0;
    while cell != site.powerhouse_cell {
        match net.receiver_of(cell.0, cell.1) {
            mk_engine::regional::hydrology::Receiver::Cell(i) => {
                cell = (i as usize / cols, i as usize % cols);
            }
            other => panic!("the intake's river leaves at {other:?} before the powerhouse"),
        }
        steps += 1;
        assert!(steps < 10_000);
    }
    println!(
        "estate hydro: {:?} at {:.0} m head, {:.0} m penstock, Qd {:.3} m3/s, firm {:.3}, mean {:.3}; town site {}",
        site.design.turbine.kind,
        site.head_m,
        site.penstock_length_m,
        site.design_flow_m3_s,
        site.firm_flow_m3_s,
        site.mean_flow_m3_s,
        match &hydro.town {
            Some(t) => format!(
                "{:?} {:.0} m head, Qd {:.2} m3/s ({} kW)",
                t.design.turbine.kind, t.head_m, t.design_flow_m3_s, TOWN_HYDRO_KW
            ),
            None => "none".into(),
        }
    );
    // The layout stands on the patch, which is the block.
    assert_eq!(b.placed.layout.patch, b.placed.patch);
}

#[test]
fn the_patch_is_exactly_the_two_by_two_estate_block() {
    let b = base();
    let size = b.domain.cell_size_m(M);
    let (row, col) = b.placed.location;
    let p = &b.placed.patch;
    assert_eq!(p.origin_x_m, col as f64 * size);
    assert_eq!(p.origin_y_m, row as f64 * size);
    assert_eq!((p.width_m, p.height_m), (2.0 * size, 2.0 * size));
    assert_eq!((p.rows, p.cols), (800, 800));
    assert_eq!(p.cell_size_m, 5.0);
    // A mismatched extent or a block off the domain is refused.
    let wrong = EstatePatchConfig {
        extent_m: 3_000.0,
        ..b.config
    };
    assert!(matches!(
        estate_patch_spec(&b.domain, (row, col), &wrong),
        Err(EstatePatchError::ExtentIsNotTwoMediumCells { .. })
    ));
    assert!(matches!(
        estate_patch_spec(&b.domain, (b.domain.rows(M) - 1, 0), &b.config),
        Err(EstatePatchError::BlockOutsideDomain)
    ));
}

#[test]
fn the_founders_estate_has_every_building_bedroom_vehicle_tool_computer_and_account() {
    let b = base();
    let sys = &b.placed.property;
    assert_eq!(sys.founders_estate_location(), Some(b.placed.location));
    let estate = sys
        .properties
        .iter()
        .find(|p| p.owner_agent_ids.iter().any(|id| id == "Gem-D"))
        .unwrap();
    assert_eq!(
        estate.owner_agent_ids,
        vec!["Gem-D".to_string(), "Gem-K".to_string()]
    );
    for kind in [
        PropertyBuildingKind::House,
        PropertyBuildingKind::Shed,
        PropertyBuildingKind::Workshop,
        PropertyBuildingKind::Armoury,
        PropertyBuildingKind::ComputerRoom,
        PropertyBuildingKind::Garage,
    ] {
        assert!(estate.buildings.iter().any(|x| x.kind == kind), "{kind:?}");
    }
    let rooms: std::collections::HashSet<_> = estate
        .items
        .iter()
        .filter_map(|i| i.room.as_deref())
        .collect();
    for r in [
        "Gem-D's Bedroom",
        "Gem-K's Bedroom",
        "Kitchen",
        "Lounge",
        "Bathroom",
    ] {
        assert!(rooms.contains(r), "room {r} missing from {rooms:?}");
    }
    for category in [
        "Construction Tools",
        "Digging Tools",
        "Repair Tools",
        "Exploration Tools",
        "Forestry Tools",
        "Smithing Tools",
    ] {
        assert!(rooms.contains(category), "shed category {category}");
    }
    let names: Vec<_> = estate.items.iter().map(|i| i.name.as_str()).collect();
    for vehicle in [
        "Polaris RZR 1000 Turbo",
        "Ford Raptor 4x4 Ute",
        "Fendt 900 Vario",
    ] {
        assert!(names.contains(&vehicle), "{vehicle}");
    }
    assert!(
        estate
            .items
            .iter()
            .filter(|i| i.kind == PropertyItemKind::Computer)
            .count()
            >= 2
    );
    assert_eq!(estate.network_accounts.len(), 2);
    assert!(estate
        .network_accounts
        .iter()
        .all(|a| a.access_level == "Administrator"));
    // The unowned homestead template is present but is not what was laid out.
    assert!(sys.properties.iter().any(|p| p.owner_agent_ids.is_empty()));
    assert_eq!(b.placed.layout.property_id, estate.id);
    // It is exactly upstream's instantiation, not a copy.
    assert_eq!(
        bootstrap_regional_property(b.placed.location)
            .properties
            .len(),
        sys.properties.len()
    );
}

#[test]
fn an_all_ocean_island_has_no_estate_site() {
    let b = base();
    let mut sea = b.physical.clone();
    let spec = b.domain.storage_spec(M);
    sea.geophysics.elevation_m = Grid2::new(&spec, -100.0);
    assert_eq!(
        choose_regional_estate_location(&b.canon, &sea, &b.ecology, &b.domain),
        None
    );
    assert!(
        place_regional_estate(&b.canon, &sea, &b.ecology, &b.domain, &b.config, b.seed).is_err()
    );
}

#[test]
fn an_island_whose_rivers_cannot_run_the_plant_has_no_estate() {
    // The same island with every river a trickle: perennial, so blocks
    // beside them still qualify, but none can run the estate's plant.
    let b = base();
    let mut dry = b.physical.clone();
    for fd in dry.river_flows.cells.values_mut() {
        for q in fd.flows_m3_s.iter_mut() {
            *q = 1e-4;
        }
        fd.mean_m3_s = 1e-4;
    }
    assert_eq!(
        choose_regional_estate_location(&b.canon, &dry, &b.ecology, &b.domain),
        None
    );
    assert!(matches!(
        place_regional_estate(&b.canon, &dry, &b.ecology, &b.domain, &b.config, b.seed),
        Err(mk_engine::regional::property::PlaceEstateError::NoHydroSite)
    ));
}
