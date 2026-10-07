//! Phase 3 Task 8: the canonical founders on the island.

use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use mk_core::canon::CanonLocked;
use mk_core::rng::RngRegistry;
use mk_engine::humans::GridPosition;
use mk_engine::humans::HumanBeing;
use mk_engine::perception::Occupancy;
use mk_engine::regional::ecology::RegionalEcologyState;
use mk_engine::regional::energy::EstateEnergy;
use mk_engine::regional::estate_layout::Space;
use mk_engine::regional::humans::{
    bootstrap_regional_humans, observe, step_regional_humans, walk_to_space, EstatePosition,
    RegionalHumanContext, RegionalHumanError,
};
use mk_engine::regional::physical::RegionalPhysicalState;
use mk_engine::regional::property::{place_regional_estate, PlacedEstate};
use mk_engine::resource_economy::ResourceEconomyState;
use mk_engine::topology::GridTopology;
use mk_island::{DomainLevel, IslandDomain, IslandScenario};

const M: DomainLevel = DomainLevel::Medium;

struct Base {
    domain: IslandDomain,
    physical: RegionalPhysicalState,
    ecology: RegionalEcologyState,
    placed: PlacedEstate,
    energy: EstateEnergy,
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
        .unwrap();
        let estate = placed
            .property
            .properties
            .iter()
            .find(|p| p.owner_agent_ids.iter().any(|i| i == "Gem-D"))
            .unwrap();
        let energy = EstateEnergy::from_config(&scenario.estate_energy, estate);
        Base {
            domain,
            physical,
            ecology,
            placed,
            energy,
        }
    })
}

fn ctx<'a>(b: &'a Base, energy: &'a EstateEnergy) -> RegionalHumanContext<'a> {
    RegionalHumanContext {
        property: &b.placed.property,
        layout: &b.placed.layout,
        physical: &b.physical,
        ecology: &b.ecology,
        energy,
        domain: &b.domain,
        solar_kw: 0.0,
    }
}

fn space_named(b: &Base, label: &str) -> Space {
    Space::Inside(
        b.placed
            .layout
            .spaces
            .iter()
            .find(|s| s.label == label)
            .unwrap()
            .id,
    )
}

fn occupancy(b: &Base, positions: &[GridPosition]) -> Occupancy {
    Occupancy::new_on(
        &GridTopology::regional(&b.domain, M),
        positions.iter().copied(),
    )
}

#[test]
fn exactly_the_founders_start_in_their_own_bedrooms() {
    let b = base();
    let (humans, positions) =
        bootstrap_regional_humans(&b.placed.property, &b.placed.layout, &b.domain).unwrap();
    let ids: Vec<_> = humans
        .registry
        .iter()
        .map(|h| h.agent_id().to_string())
        .collect();
    assert_eq!(ids, vec!["Gem-D".to_string(), "Gem-K".to_string()]);
    for (agent, canonical) in [
        ("Gem-D", HumanBeing::gem_d_founder()),
        ("Gem-K", HumanBeing::gem_k_founder()),
    ] {
        let h = humans.registry.get_human(agent).unwrap();
        assert_eq!(
            serde_json::to_string(&h.profile).unwrap(),
            serde_json::to_string(&canonical.profile).unwrap(),
            "{agent}'s profile differs from the canonical constructor"
        );
        let entry = positions
            .0
            .get(agent)
            .expect("founder has an estate position");
        assert_eq!(entry.space, space_named(b, &format!("{agent}'s Bedroom")));
        // The runtime cell is the medium cell under the bedroom.
        let size = b.domain.cell_size_m(M);
        assert_eq!(
            h.position,
            GridPosition::new(
                (entry.position_m.1 / size).floor() as i32,
                (entry.position_m.0 / size).floor() as i32
            )
        );
        assert_eq!(b.placed.layout.space_at(entry.position_m), entry.space);
    }
}

#[test]
fn computer_access_needs_an_account_the_computer_room_and_power() {
    let b = base();
    let c = ctx(b, &b.energy);
    let (humans, mut positions) =
        bootstrap_regional_humans(&b.placed.property, &b.placed.layout, &b.domain).unwrap();
    let cell = humans.registry.get_human("Gem-D").unwrap().position;
    let occ = occupancy(b, &[cell]);

    // Gem-D walks to the computer room: through the House hall.
    let route = walk_to_space(
        &mut positions,
        &b.placed.layout,
        "Gem-D",
        space_named(b, "Computer Room"),
    )
    .unwrap();
    assert!(route.contains(&space_named(b, "General")), "{route:?}");
    assert_eq!(route.first(), Some(&space_named(b, "Gem-D's Bedroom")));
    let o = observe(&c, &positions, &occ, 1, &cell, "Gem-D");
    assert_eq!(o.computer_access, 1.0);

    // Gem-K in the Kitchen does not have access.
    walk_to_space(
        &mut positions,
        &b.placed.layout,
        "Gem-K",
        space_named(b, "Kitchen"),
    )
    .unwrap();
    assert_eq!(
        observe(&c, &positions, &occ, 1, &cell, "Gem-K").computer_access,
        0.0
    );
    // ...but in the computer room Gem-K does.
    walk_to_space(
        &mut positions,
        &b.placed.layout,
        "Gem-K",
        space_named(b, "Computer Room"),
    )
    .unwrap();
    assert_eq!(
        observe(&c, &positions, &occ, 1, &cell, "Gem-K").computer_access,
        1.0
    );

    // A human with no account stays denied in the computer room.
    positions.0.insert(
        "Visitor".into(),
        EstatePosition {
            space: space_named(b, "Computer Room"),
            position_m: (0.0, 0.0),
        },
    );
    assert_eq!(
        observe(&c, &positions, &occ, 1, &cell, "Visitor").computer_access,
        0.0
    );

    // No power, no access, even in the computer room.
    let mut dark = b.energy.clone();
    for battery in &mut dark.batteries {
        battery.charge_kwh = 0.0;
    }
    for store in &mut dark.fuel_stores {
        store.litres = 0.0;
    }
    let c_dark = ctx(b, &dark);
    assert_eq!(
        observe(&c_dark, &positions, &occ, 1, &cell, "Gem-D").computer_access,
        0.0
    );

    // Walking to a space that is not in the estate, or from a human who is
    // not in it, is an error.
    assert!(matches!(
        walk_to_space(&mut positions, &b.placed.layout, "Nobody", Space::Outdoors),
        Err(RegionalHumanError::NotInEstate(_))
    ));
}

#[test]
fn observations_are_plausible_and_the_house_shelters() {
    let b = base();
    let c = ctx(b, &b.energy);
    let (humans, mut positions) =
        bootstrap_regional_humans(&b.placed.property, &b.placed.layout, &b.domain).unwrap();
    let cell = humans.registry.get_human("Gem-D").unwrap().position;
    let occ = occupancy(b, &[cell, cell]);
    let inside = observe(&c, &positions, &occ, 5, &cell, "Gem-D");
    walk_to_space(&mut positions, &b.placed.layout, "Gem-D", Space::Outdoors).unwrap();
    let outside = observe(&c, &positions, &occ, 5, &cell, "Gem-D");
    assert!(
        (-25.0..45.0).contains(&inside.ambient_temperature_c),
        "{} C",
        inside.ambient_temperature_c
    );
    for v in [
        inside.hydration_access,
        inside.caloric_access,
        inside.shelter_quality,
        inside.social_density,
        inside.daylight_fraction,
    ] {
        assert!((0.0..=1.0).contains(&v));
    }
    // The House gives more shelter than standing outdoors.
    assert!(
        inside.shelter_quality >= 0.95 || inside.shelter_quality > outside.shelter_quality - 1e-12
    );
    assert!(inside.shelter_quality >= outside.shelter_quality);
    assert_eq!(inside.computer_bridge_available, 0.0);
    // Two humans in one cell are company; one alone is not.
    assert!(inside.social_density > 0.0);
}

#[test]
fn walking_out_of_the_patch_drops_the_estate_entry_and_nothing_wraps() {
    let b = base();
    let c = ctx(b, &b.energy);
    let topology = GridTopology::regional(&b.domain, M);
    let (mut humans, mut positions) =
        bootstrap_regional_humans(&b.placed.property, &b.placed.layout, &b.domain).unwrap();
    let rng = RngRegistry::new([3u8; 32]);
    let mut economy = ResourceEconomyState::new();

    // Gem-D is placed outside the estate block (a few cells away).
    let (row0, col0) = b.placed.location;
    let away = GridPosition::new(row0 as i32 - 3, col0 as i32);
    humans
        .registry
        .get_human_mut("Gem-D")
        .unwrap()
        .set_runtime_position(away);
    step_regional_humans(
        &mut humans,
        &mut positions,
        &mut economy,
        &c,
        &topology,
        1,
        3_600,
        &rng,
    );
    assert!(
        !positions.0.contains_key("Gem-D"),
        "Gem-D left the patch but kept an entry"
    );
    // Gem-K has an estate entry exactly while their cell is in the block
    // (a human may legitimately walk across a cell boundary in an hour).
    let k = humans.registry.get_human("Gem-K").unwrap().position;
    let (r0, c0) = b.placed.location;
    let in_block =
        (r0 as i32..r0 as i32 + 2).contains(&k.row) && (c0 as i32..c0 as i32 + 2).contains(&k.col);
    assert_eq!(positions.0.contains_key("Gem-K"), in_block);

    // On the domain's west edge a human never appears on the east edge.
    let cols = b.domain.cols(M) as i32;
    humans
        .registry
        .get_human_mut("Gem-D")
        .unwrap()
        .set_runtime_position(GridPosition::new(100, 0));
    let mut last = 0;
    for tick in 2..60 {
        step_regional_humans(
            &mut humans,
            &mut positions,
            &mut economy,
            &c,
            &topology,
            tick,
            6 * 3_600,
            &rng,
        );
        let now = humans.registry.get_human("Gem-D").unwrap().position;
        assert!(
            (0..cols).contains(&now.col),
            "col {} left the domain",
            now.col
        );
        assert!(
            (now.col - last).abs() <= 1,
            "jumped from col {last} to {}",
            now.col
        );
        last = now.col;
    }
    // Both are still alive after two weeks of 6-hour steps.
    assert!(humans
        .registry
        .iter()
        .all(|h| matches!(h.profile.status, mk_core::human::HumanStatus::Alive)));
}
