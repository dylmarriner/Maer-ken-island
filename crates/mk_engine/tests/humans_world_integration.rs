use mk_core::canon::CanonLocked;
use mk_core::human::{HumanId, HumanSchema};
use mk_engine::runtime::human::{BiologicalSex, HumanBeing};
use mk_engine::runtime::GridPosition;
use mk_engine::world_integration::WorldState;
use std::sync::Arc;

/// Proves the human population is actually ticked by `step_world`, not just
/// exercised via direct unit calls to `HumanSystem::step`.
#[test]
fn step_world_ages_the_human_population() {
    let mut world = WorldState::new(Arc::new(CanonLocked::default()), [11u8; 32]);

    let human = HumanBeing::new("resident".to_string(), BiologicalSex::Female);
    let starting_age = human.development.age_years;
    world.humans_state.registry.add_human_no_storage(human);

    // A handful of yearly ticks so aging is observable within the test.
    let one_year_seconds = (365.25 * 24.0 * 3600.0) as u64;
    for _ in 0..5 {
        world.step_world(one_year_seconds).unwrap();
    }

    let resident = world
        .humans_state
        .registry
        .get_human("resident")
        .expect("resident must still be tracked");

    assert!(
        resident.development.age_years > starting_age,
        "expected age to advance via step_world, got {} (started at {})",
        resident.development.age_years,
        starting_age
    );
}

/// Proves reproduction actually runs inside the world tick loop and can grow
/// the population: a consensual `Intimacy` act between two co-located
/// fertile adults may conceive, and the child is born only after a full
/// gestation — never within a single tick.
#[test]
fn step_world_can_grow_population_via_reproduction() {
    let mut world = WorldState::new(Arc::new(CanonLocked::default()), [22u8; 32]);

    // One step fills the live productivity field. The couple then lives in
    // the land cell with the best food and water a mild climate offers, so
    // the test exercises reproduction rather than the habitability of
    // wherever the default birthplace happens to fall.
    world.step_world(1).unwrap();
    let nlon = world.grid_spec.nlon;
    let home = (0..world.grid_spec.nlat)
        .flat_map(|row| (0..nlon).map(move |col| (row, col)))
        .filter(|&(row, col)| {
            let annual_k = world.climatology.temperature_k[row * nlon + col];
            *world.elevation_grid.get(row, col) >= 0.0 && (283.0..303.0).contains(&annual_k)
        })
        .map(|(row, col)| {
            let food =
                mk_engine::perception::caloric_access(&world.biosphere_state, nlon, row, col);
            let water = mk_engine::perception::hydration_access(
                &world.hydrology_state,
                &world.weather_state.precipitation,
                world.biome_grid.get_safe(row, col).copied(),
                row,
                col,
            );
            (row, col, food.min(water))
        })
        .max_by(|a, b| a.2.total_cmp(&b.2))
        .map(|(row, col, _)| GridPosition::new(row as i32, col as i32))
        .expect("the world has mild land");
    for (name, sex) in [
        ("mother", BiologicalSex::Female),
        ("father", BiologicalSex::Male),
    ] {
        let mut human = HumanBeing::new(name.to_string(), sex);
        human.set_runtime_position(home);
        world.humans_state.registry.add_human_no_storage(human);
    }

    let before = world.humans_state.registry.population_count();
    world.step_world(1).unwrap();
    assert_eq!(
        world.humans_state.registry.population_count(),
        before,
        "a child must never be born within one tick of the parents meeting"
    );

    let one_week_seconds = 7 * 24 * 3600;
    let mut conceived_at_week = None;
    let mut born_at_week = None;
    for week in 1..=(6 * 52) {
        world.step_world(one_week_seconds).unwrap();
        let mother = world.humans_state.registry.get_human("mother").unwrap();
        if conceived_at_week.is_none() && mother.reproduction.pregnancy.is_some() {
            conceived_at_week = Some(week);
        }
        if world.humans_state.registry.population_count() > before {
            born_at_week = Some(week);
            break;
        }
    }

    let conceived = conceived_at_week.expect("the couple should conceive within six years");
    let born = born_at_week.expect("a conceived child should be born");
    assert!(
        born - conceived >= 38,
        "birth must follow a full gestation (conceived week {conceived}, born week {born})"
    );
}

#[test]
fn step_world_seeds_runtime_position_from_birthplace_once_human_enters_world() {
    let mut world = WorldState::new(Arc::new(CanonLocked::default()), [33u8; 32]);

    let mut schema = HumanSchema::canonical_minimal("traveler");
    schema.core_identity.birthplace.coordinates.latitude = -45.0;
    schema.core_identity.birthplace.coordinates.longitude = 120.0;

    world
        .humans_state
        .registry
        .add_human_no_storage(HumanBeing::from_schema(HumanId::new(3), schema));

    world.step_world(1).unwrap();

    let traveler = world
        .humans_state
        .registry
        .get_human("traveler")
        .expect("traveler must still be tracked");

    // Seeded at the birthplace cell (24, 53); the human may already have
    // taken one real movement step during this first tick.
    assert!(
        (traveler.position.row - 24).abs() <= 1 && (traveler.position.col - 53).abs() <= 1,
        "expected to start at the birthplace cell (24, 53), found {:?}",
        traveler.position
    );
    assert!(world.settlements_state.sites.is_empty());
}

#[test]
fn step_world_materializes_inspectable_fauna_from_live_species() {
    let canon = Arc::new(CanonLocked::default());
    let mut world = WorldState::new(canon, [3u8; 32]);

    assert!(world.organisms_state.beings.is_empty());
    world.step_world(1).unwrap();

    assert!(!world.biosphere_state.species.is_empty());
    assert!(!world.organisms_state.beings.is_empty());
    let being = &world.organisms_state.beings[0];
    assert!(being.species_id > 0);
    assert_eq!(being.lifecycle, mk_engine::organisms::LifecycleState::Alive);
    assert!(!world.vegetation_state.plants.is_empty());
    assert!(world.vegetation_state.plants[0].alive);
}

#[test]
fn sites_require_explicit_construction() {
    let mut world = WorldState::new(Arc::new(CanonLocked::default()), [5u8; 32]);
    assert!(world.settlements_state.sites.is_empty());

    let site_id = world.settlements_state.build_site(
        mk_engine::organisms::SiteKind::Shelter,
        mk_engine::runtime::GridPosition::new(4, 8),
        17,
    );

    assert_eq!(site_id, 1);
    assert_eq!(world.settlements_state.sites[0].occupants, vec![17]);
}

#[test]
fn constructed_structures_become_shelters_and_settlements() {
    use mk_engine::organisms::SiteKind;
    use mk_engine::resource_economy::ConstructedStructure;
    let mut world = WorldState::new(Arc::new(CanonLocked::default()), [5u8; 32]);
    let cell = GridPosition::new(3, 3);
    let structure = |id: u64| ConstructedStructure {
        id,
        position: cell,
        recipe: mk_engine::resource_economy::RecipeId::WoodenShelter,
        material_cost: 1,
    };
    let first = structure(1);
    world.resource_economy_state.structures.push(first);

    let mut resident = HumanBeing::new("resident".to_string(), BiologicalSex::Female);
    resident.set_runtime_position(cell);
    let mut ghost = HumanBeing::new("ghost".to_string(), BiologicalSex::Male);
    ghost.set_runtime_position(cell);
    ghost.profile.status = mk_engine::runtime::human::HumanStatus::Dead;
    world.humans_state.registry.add_human_no_storage(resident);
    world.humans_state.registry.add_human_no_storage(ghost);

    world.settlements_state.sync_humans(
        &world.humans_state,
        &world.resource_economy_state.structures,
    );
    let site = &world.settlements_state.sites[0];
    assert_eq!(site.kind, SiteKind::Shelter);
    assert_eq!(site.occupants.len(), 1, "the dead are not occupants");
    assert!(site.active);

    for id in 2..=3 {
        let more = structure(id);
        world.resource_economy_state.structures.push(more);
    }
    world.settlements_state.sync_humans(
        &world.humans_state,
        &world.resource_economy_state.structures,
    );
    assert_eq!(world.settlements_state.sites[0].kind, SiteKind::Settlement);
}

/// Humans walk: however long they explore or forage, someone who starts on
/// land never ends a tick standing on open ocean.
#[test]
fn humans_on_land_never_walk_onto_open_ocean() {
    let mut world = WorldState::new(Arc::new(CanonLocked::default()), [44u8; 32]);
    let spec = world.grid_spec.clone();
    // A land cell with ocean next to it, so the walk is actually tested.
    let coast = (0..spec.nlat)
        .flat_map(|row| (0..spec.nlon).map(move |col| (row, col)))
        .find(|&(row, col)| {
            *world.elevation_grid.get(row, col) >= 0.0
                && (0..spec.nlon).any(|c| {
                    let dc = (c as i32 - col as i32).rem_euclid(spec.nlon as i32);
                    (dc == 1 || dc == spec.nlon as i32 - 1)
                        && *world.elevation_grid.get(row, c) < 0.0
                })
        })
        .expect("the planet has a coastline");
    for name in ["walker_a", "walker_b", "walker_c"] {
        let mut human = HumanBeing::new(name.to_string(), BiologicalSex::Female);
        human.set_runtime_position(GridPosition::new(coast.0 as i32, coast.1 as i32));
        world.humans_state.registry.add_human_no_storage(human);
    }
    for _ in 0..60 {
        world.step_world(7 * 24 * 3600).unwrap();
        for human in world.humans_state.registry.get_all_humans() {
            let (row, col) = (human.position.row as usize, human.position.col as usize);
            assert!(
                *world.elevation_grid.get(row, col) >= 0.0,
                "{} walked onto ocean at ({row}, {col})",
                human.agent_id()
            );
        }
    }
}
