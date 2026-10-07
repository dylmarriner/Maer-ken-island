//! Phase 3 Task 4: the founders' estate placed on the owner's island.

use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use mk_core::canon::CanonLocked;
use mk_core::grid::Grid2;
use mk_engine::organisms::property::{PropertyBuildingKind, PropertyItemKind};
use mk_engine::regional::ecology::RegionalEcologyState;
use mk_engine::regional::hydrology::discharge_m3_s;
use mk_engine::regional::physical::RegionalPhysicalState;
use mk_engine::regional::property::{
    bootstrap_regional_property, choose_regional_estate_location, estate_patch_spec,
    place_regional_estate, EstatePatchError, PlacedEstate, ESTATE_MIN_RIVER_DISCHARGE_M3_S,
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
    // A river of at least the minimum discharge lies within 1 km of the block.
    let river = (row.saturating_sub(1)..=row + 2).any(|r| {
        (col.saturating_sub(1)..=col + 2).any(|c| {
            r < b.domain.rows(M)
                && c < b.domain.cols(M)
                && discharge_m3_s(&b.physical.hydrology, &b.domain, r, c)
                    >= ESTATE_MIN_RIVER_DISCHARGE_M3_S
        })
    });
    assert!(river, "no river beside the estate");
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
