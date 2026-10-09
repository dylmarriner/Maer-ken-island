//! Phase 4 Task 4: operator interventions against the island.
//!
//! Two things are being held here. The first is that what the island can
//! honour, it honours *through its own machinery* — a spawned person gets
//! the same body in the material ledger that a dashboard creation does, an
//! injection books its flux, and the conservation audit still closes
//! afterwards. The second is that what it cannot honour, it refuses by
//! name: no silent no-ops, and nothing invented to make an upstream action
//! appear to work.

use std::path::PathBuf;
use std::sync::Arc;

use mk_core::canon::CanonLocked;
use mk_core::flux::{FluxKind, Reservoir};
use mk_engine::regional::commands::{Applied, CommandError, IslandCommand};
use mk_engine::regional::interventions::{
    action_name, refusal, IslandApplied, IslandDirective, IslandInterventionError,
};
use mk_engine::regional::life::IslandLife;
use mk_engine::regional::materials::Material;
use mk_engine::regional::replay::{replay_island, IslandReplayLog};
use mk_interventions::{
    BiomassType, ClimateParameter, DisturbanceType, EnergyType, HumanSpawnProfile,
    InterventionAction, Location, Region, ResourceType, ScenarioValue, StructureKind,
};
use mk_island::{DomainLevel, IslandScenario};

const STEP: u64 = 60;

fn repo(path: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path)
}

fn canon() -> Arc<CanonLocked> {
    Arc::new(CanonLocked::load(&repo("fixtures/island/canon.json")).unwrap())
}

fn scenario() -> IslandScenario {
    let mut scenario =
        IslandScenario::load(&repo("fixtures/island/default_scenario.json")).unwrap();
    scenario.estate_patch.tree_cap = 50;
    scenario
}

fn island() -> IslandLife {
    IslandLife::bootstrap(scenario(), canon()).expect("the island bootstraps")
}

fn hex(d: [u8; 32]) -> String {
    d.iter().map(|b| format!("{b:02x}")).collect()
}

/// Where a medium cell is on the planet, in degrees.
///
/// `lat_lon_at_m` answers in radians; an upstream `Location` is degrees.
fn at(life: &IslandLife, row: usize, col: usize) -> Location {
    let (x, y) = life.domain.cell_center_m(DomainLevel::Medium, row, col);
    let (latitude, longitude) = life.domain.lat_lon_at_m(x, y);
    Location::new(latitude.to_degrees() as f32, longitude.to_degrees() as f32)
}

/// A land cell and a sea cell, both well inside the domain so that the
/// round trip through a latitude and longitude cannot push them off its
/// edge.
fn a_land_cell(life: &IslandLife) -> (usize, usize) {
    life.placed.location
}

fn a_sea_cell(life: &IslandLife) -> (usize, usize) {
    let (rows, cols) = (
        life.domain.rows(DomainLevel::Medium),
        life.domain.cols(DomainLevel::Medium),
    );
    (rows / 4..rows * 3 / 4)
        .flat_map(|row| (cols / 4..cols * 3 / 4).map(move |col| (row, col)))
        .find(|(row, col)| !*life.physical.geophysics.land_mask.get(*row, *col))
        .expect("the island has sea inside its middle half")
}

fn a_profile(name: &str) -> HumanSpawnProfile {
    HumanSpawnProfile {
        name: name.to_string(),
        birth_timestamp: "1994-02-03T04:05:06Z".to_string(),
        birth_latitude: -41.0,
        birth_longitude: 174.0,
        age_years: 31.0,
        height_cm: 174.0,
        build: "average".to_string(),
        hair_color: "black".to_string(),
        eye_color: "brown".to_string(),
        skin_tone: "olive".to_string(),
    }
}

/// One of every action, so the walk below cannot miss a variant.
fn one_of_each(life: &IslandLife) -> Vec<InterventionAction> {
    let (row, col) = a_land_cell(life);
    let here = at(life, row, col);
    let region = Region::new(here.clone(), 3.0);
    vec![
        InterventionAction::Pause,
        InterventionAction::Resume,
        InterventionAction::Step { ticks: 3 },
        InterventionAction::Scrub { target_tick: 0 },
        InterventionAction::Branch {
            name: "b".to_string(),
            source_snapshot: "s".to_string(),
        },
        InterventionAction::Fork {
            name: "f".to_string(),
            modifications: Vec::new(),
        },
        InterventionAction::InjectResource {
            resource_type: ResourceType::Water,
            amount: 10.0,
            location: here.clone(),
        },
        InterventionAction::InjectResource {
            resource_type: ResourceType::Minerals,
            amount: 10.0,
            location: here.clone(),
        },
        InterventionAction::InjectBiomass {
            biomass_type: BiomassType::Producers,
            amount: 100.0,
            region: region.clone(),
        },
        InterventionAction::InjectBiomass {
            biomass_type: BiomassType::Apex,
            amount: 100.0,
            region: region.clone(),
        },
        InterventionAction::InjectEnergy {
            energy_type: EnergyType::Solar,
            amount: 100.0,
            location: here.clone(),
        },
        InterventionAction::ModifyClimate {
            parameter: ClimateParameter::Temperature,
            value: 288.0,
            region: Some(region.clone()),
        },
        InterventionAction::TriggerDisturbance {
            disturbance_type: DisturbanceType::Fire,
            intensity: 0.5,
            location: here.clone(),
        },
        InterventionAction::SculptTerrain {
            elevation_delta_m: 10.0,
            region: region.clone(),
        },
        InterventionAction::SmoothTerrain {
            strength: 0.5,
            region,
        },
        InterventionAction::ModifyScenario {
            parameter: "climate.co2_concentration_ppm".to_string(),
            value: ScenarioValue::Float(400.0),
        },
        InterventionAction::ConstructStructure {
            structure: StructureKind::WoodenShelter,
            location: here.clone(),
        },
        InterventionAction::SpawnHuman {
            template_id: "female".to_string(),
            location: here,
            profile: Some(a_profile("Awhina")),
        },
        InterventionAction::RemoveHuman {
            human_id: "nobody".to_string(),
        },
    ]
}

/// The refusals this island makes, by action name.
///
/// Written out rather than derived, so that an action quietly changing
/// sides — becoming supported, or stopping being supported — fails here
/// and has to be accounted for in `UPSTREAM.md` at the same time.
const REFUSED: &[&str] = &[
    "Scrub",
    "Branch",
    "Fork",
    "InjectEnergy",
    "TriggerDisturbance",
    "SculptTerrain",
    "SmoothTerrain",
    "ModifyScenario",
];

/// Refused for some arguments and applied for others, because the island
/// shares only part of upstream's vocabulary for them.
const ARGUMENT_DEPENDENT: &[&str] = &["InjectBiomass", "InjectResource"];

#[test]
fn every_action_is_supported_or_refused_by_name() {
    let life = island();
    let (mut refused, mut applied): (Vec<&'static str>, Vec<&'static str>) =
        (Vec::new(), Vec::new());
    for action in one_of_each(&life) {
        let name = action_name(&action);
        let into = match refusal(&action) {
            Some(reason) => {
                assert!(
                    reason.len() > 30,
                    "{name} is refused without saying why: {reason:?}"
                );
                &mut refused
            }
            None => &mut applied,
        };
        if !into.contains(&name) {
            into.push(name);
        }
    }
    // An action in both lists is refused for some of its arguments and
    // applied for others; `the_two_part_refusals_turn_on_their_argument`
    // is where those are pinned.
    let mut always: Vec<&str> = refused
        .iter()
        .copied()
        .filter(|name| !applied.contains(name))
        .collect();
    let mut sometimes: Vec<&str> = refused
        .iter()
        .copied()
        .filter(|name| applied.contains(name))
        .collect();
    always.sort_unstable();
    sometimes.sort_unstable();
    let mut expected = REFUSED.to_vec();
    expected.sort_unstable();
    assert_eq!(always, expected, "the always-refused set has moved");
    assert_eq!(
        sometimes, ARGUMENT_DEPENDENT,
        "the part-refused set has moved"
    );
}

#[test]
fn the_two_part_refusals_turn_on_their_argument() {
    let life = island();
    // Water is a material the island holds; minerals are not. Producer
    // carbon is a field the island carries; apex predators are not. Both
    // actions are refused for one argument and applied for another, which
    // the name-level list above cannot express.
    let here = at(&life, a_land_cell(&life).0, a_land_cell(&life).1);
    assert!(refusal(&InterventionAction::InjectResource {
        resource_type: ResourceType::Water,
        amount: 1.0,
        location: here.clone(),
    })
    .is_none());
    assert!(refusal(&InterventionAction::InjectResource {
        resource_type: ResourceType::Nutrients,
        amount: 1.0,
        location: here.clone(),
    })
    .is_some());
    let region = Region::new(here, 2.0);
    assert!(refusal(&InterventionAction::InjectBiomass {
        biomass_type: BiomassType::Producers,
        amount: 1.0,
        region: region.clone(),
    })
    .is_none());
    for animal in [
        BiomassType::Consumers,
        BiomassType::Apex,
        BiomassType::Decomposers,
    ] {
        assert!(refusal(&InterventionAction::InjectBiomass {
            biomass_type: animal,
            amount: 1.0,
            region: region.clone(),
        })
        .is_some());
    }
}

#[test]
fn a_refused_action_changes_nothing_and_says_what_is_missing() {
    let mut life = island();
    let before = hex(life.state_digest());
    let (row, col) = a_land_cell(&life);
    let action = InterventionAction::SculptTerrain {
        elevation_delta_m: 50.0,
        region: Region::new(at(&life, row, col), 5.0),
    };
    match life.intervene(&action) {
        Err(IslandInterventionError::NotSupportedOnIsland { action, reason }) => {
            assert_eq!(action, "SculptTerrain");
            assert!(
                reason.contains("canon"),
                "the reason should say why: {reason}"
            );
        }
        other => panic!("terrain is canon and should be refused, got {other:?}"),
    }
    assert_eq!(hex(life.state_digest()), before);
}

#[test]
fn pausing_is_a_directive_and_moves_nothing() {
    let mut life = island();
    let before = hex(life.state_digest());
    for (action, expected) in [
        (InterventionAction::Pause, IslandDirective::Pause),
        (InterventionAction::Resume, IslandDirective::Resume),
        (
            InterventionAction::Step { ticks: 7 },
            IslandDirective::Step { ticks: 7 },
        ),
    ] {
        match life.intervene(&action).expect("a control is honoured") {
            IslandApplied::Control { directive } => assert_eq!(directive, expected),
            other => panic!("expected a directive, got {}", other.summary()),
        }
    }
    assert_eq!(hex(life.state_digest()), before);
}

#[test]
fn an_action_aimed_off_the_island_is_refused_rather_than_moved_onto_it() {
    let mut life = island();
    let before = hex(life.state_digest());
    let action = InterventionAction::InjectResource {
        resource_type: ResourceType::Water,
        amount: 5.0,
        // The far side of the planet from an island in the southern ocean.
        location: Location::new(51.5, 0.12),
    };
    match life.intervene(&action) {
        Err(IslandInterventionError::OffIsland { .. }) => {}
        other => panic!("London is not on this island, got {other:?}"),
    }
    assert_eq!(hex(life.state_digest()), before);
}

#[test]
fn warming_a_region_warms_that_region_and_books_the_heat() {
    let mut life = island();
    let (row, col) = a_land_cell(&life);
    let coarse = DomainLevel::Coarse;
    let centre = {
        let (x, y) = life.domain.cell_center_m(DomainLevel::Medium, row, col);
        let (latitude, longitude) = life.domain.lat_lon_at_m(x, y);
        mk_engine::topology::GridTopology::regional(&life.domain, coarse)
            .cell_for_lat_lon(latitude.to_degrees(), longitude.to_degrees())
            .expect("the estate is on the island")
    };
    let far = (
        if centre.0 == 0 {
            life.domain.rows(coarse) - 1
        } else {
            0
        },
        centre.1,
    );
    let before_far = *life.physical.climate.surface_temperature.get(far.0, far.1);

    let action = InterventionAction::ModifyClimate {
        parameter: ClimateParameter::Temperature,
        value: 305.0,
        region: Some(Region::new(at(&life, row, col), 1.0)),
    };
    let outcome = life.intervene(&action).expect("the climate is editable");
    let touched = match outcome {
        IslandApplied::Mutated { touched, .. } => touched,
        other => panic!("expected a mutation, got {}", other.summary()),
    };
    assert!(touched >= 1, "at least the centre cell should be touched");

    assert_eq!(
        *life
            .physical
            .climate
            .surface_temperature
            .get(centre.0, centre.1),
        305.0
    );
    assert_eq!(
        *life.physical.climate.surface_temperature.get(far.0, far.1),
        before_far,
        "a cell outside the region should be untouched"
    );

    // The heat came from outside the island, and says so.
    let booked: f64 = life
        .physical
        .ledger
        .entries()
        .iter()
        .filter(|e| {
            e.kind == FluxKind::Energy
                && (e.source == Reservoir::OperatorIntervention
                    || e.sink == Reservoir::OperatorIntervention)
        })
        .map(|e| e.amount)
        .sum();
    assert!(
        booked > 0.0,
        "an operator's heat should be booked from the boundary"
    );
}

#[test]
fn injecting_water_keeps_the_conservation_audit_closed() {
    let mut life = island();
    let (row, col) = a_land_cell(&life);
    let before_kg = life.materials.stock_kg(Material::Water);
    let before_audits = life.audits_closed;

    let action = InterventionAction::InjectResource {
        resource_type: ResourceType::Water,
        amount: 250.0,
        location: at(&life, row, col),
    };
    life.intervene(&action).expect("water is a material here");

    assert!((life.materials.stock_kg(Material::Water) - before_kg - 250.0).abs() < 1e-9);
    assert_eq!(
        life.audits_closed,
        before_audits + 1,
        "the injection should have closed its own audit"
    );
    // And the island keeps stepping afterwards, which is the part an
    // unbooked injection would break rather than the injection itself.
    life.advance(STEP).expect("the island steps on");
}

#[test]
fn producer_carbon_lands_on_land_and_not_in_the_sea() {
    let mut life = island();
    let (row, col) = a_land_cell(&life);
    let sea = a_sea_cell(&life);
    let before_land = *life.ecology.biomass_kgc_m2.get(row, col);
    let before_sea = *life.ecology.biomass_kgc_m2.get(sea.0, sea.1);

    let action = InterventionAction::InjectBiomass {
        biomass_type: BiomassType::Producers,
        amount: 1_000_000.0,
        region: Region::new(at(&life, row, col), 2.0),
    };
    life.intervene(&action).expect("producers are a field here");

    assert!(
        *life.ecology.biomass_kgc_m2.get(row, col) > before_land,
        "the land cell at the centre should have gained carbon"
    );
    assert_eq!(
        *life.ecology.biomass_kgc_m2.get(sea.0, sea.1),
        before_sea,
        "the sea carries no producer carbon and should not be given any"
    );
}

#[test]
fn a_spawned_person_has_a_body_in_the_ledger_and_a_place_to_stand() {
    let mut life = island();
    let (row, col) = a_land_cell(&life);
    let before = life.humans.registry.iter().count();

    let action = InterventionAction::SpawnHuman {
        template_id: "female".to_string(),
        location: at(&life, row, col),
        profile: Some(a_profile("Awhina")),
    };
    let summary = life
        .intervene(&action)
        .expect("a person can be put on the island")
        .summary();
    assert!(
        summary.to_lowercase().contains("awhina"),
        "the summary should name who was created: {summary}"
    );

    assert_eq!(life.humans.registry.iter().count(), before + 1);
    let human = life
        .humans
        .registry
        .iter()
        .find(|h| h.agent_id().to_lowercase().contains("awhina"))
        .expect("they are in the registry");
    let id = human.agent_id().to_string();
    // The f32 latitude and longitude in an upstream `Location` can round
    // into a neighbouring cell on the way back, so this allows one cell of
    // slack — and still catches the defect it exists for, which is a
    // created person left at the grid's origin with no runtime position at
    // all (and the origin is sea).
    let standing = human.position;
    assert!(
        *life
            .physical
            .geophysics
            .land_mask
            .get(standing.row as usize, standing.col as usize),
        "without a runtime position the next step drops them; they are at {standing:?}"
    );
    assert!(
        (standing.row - row as i32).abs() <= 1 && (standing.col - col as i32).abs() <= 1,
        "they should be at or beside the cell asked for, not {standing:?}"
    );
    assert!(
        life.materials.body_carbon_kg(&id).unwrap_or(0.0) > 0.0,
        "without a body in the ledger they never breathe"
    );
    life.advance(STEP).expect("the island steps on");
}

#[test]
fn a_spawn_without_an_identity_is_refused_rather_than_defaulted() {
    let mut life = island();
    let (row, col) = a_land_cell(&life);
    let action = InterventionAction::SpawnHuman {
        template_id: "female".to_string(),
        location: at(&life, row, col),
        profile: None,
    };
    match life.intervene(&action) {
        Err(IslandInterventionError::Create(e)) => {
            assert!(e.to_string().contains("name"), "{e}");
        }
        other => panic!("there is no name to default to, got {other:?}"),
    }
}

#[test]
fn removing_somebody_takes_their_body_and_their_accumulators_with_them() {
    let mut life = island();
    let (row, col) = a_land_cell(&life);
    life.intervene(&InterventionAction::SpawnHuman {
        template_id: "male".to_string(),
        location: at(&life, row, col),
        profile: Some(a_profile("Rangi")),
    })
    .expect("a person can be put on the island");
    let id = life
        .humans
        .registry
        .iter()
        .find(|h| h.agent_id().to_lowercase().contains("rangi"))
        .expect("they are there")
        .agent_id()
        .to_string();
    // Let them live a while, so there is something to leave behind.
    life.advance(STEP * 120).expect("the island steps on");
    assert!(life.materials.body_carbon_kg(&id).is_some());

    life.intervene(&InterventionAction::RemoveHuman {
        human_id: id.clone(),
    })
    .expect("they can be taken out again");

    assert!(
        life.humans.registry.get_human(&id).is_none(),
        "they should be out of the registry"
    );
    assert!(
        !life.positions.0.contains_key(&id),
        "they should be out of the estate's position table"
    );
    assert!(
        life.materials.body_carbon_kg(&id).is_none(),
        "their body should be out of the material ledger"
    );
    // The accumulators are not readable from outside, so this is the test
    // that would fail if they were left: they are hashed, and the island
    // has to keep auditing.
    life.advance(STEP * 120)
        .expect("the island steps on without them");
}

#[test]
fn removing_somebody_who_is_not_there_is_an_error_not_a_shrug() {
    let mut life = island();
    match life.intervene(&InterventionAction::RemoveHuman {
        human_id: "nobody-at-all".to_string(),
    }) {
        Err(IslandInterventionError::NotFound { what }) => assert!(what.contains("nobody-at-all")),
        other => panic!("expected a not-found, got {other:?}"),
    }
}

#[test]
fn a_run_somebody_intervened_in_replays_to_the_same_island() {
    let mut live = island();
    let (row, col) = a_land_cell(&live);
    let here = at(&live, row, col);

    let script: Vec<(u64, IslandCommand)> = vec![
        (
            2,
            IslandCommand::Intervention(Box::new(InterventionAction::ModifyClimate {
                parameter: ClimateParameter::Temperature,
                value: 301.5,
                region: Some(Region::new(here.clone(), 2.0)),
            })),
        ),
        (
            5,
            IslandCommand::Intervention(Box::new(InterventionAction::InjectResource {
                resource_type: ResourceType::Water,
                amount: 77.0,
                location: here.clone(),
            })),
        ),
        (
            9,
            IslandCommand::Intervention(Box::new(InterventionAction::SpawnHuman {
                template_id: "female".to_string(),
                location: here.clone(),
                profile: Some(a_profile("Mere")),
            })),
        ),
        (
            13,
            IslandCommand::Intervention(Box::new(InterventionAction::InjectBiomass {
                biomass_type: BiomassType::Producers,
                amount: 5_000.0,
                region: Region::new(here, 2.0),
            })),
        ),
    ];

    let until = 20;
    for tick in 0..until {
        for (at_tick, command) in &script {
            if *at_tick == tick {
                live.apply_command(command.clone())
                    .expect("the island honours it");
            }
        }
        live.advance(STEP).expect("the island steps on");
    }
    let live_digest = hex(live.state_digest());
    assert_eq!(live.replay_log().entries.len(), script.len());

    let replayed = replay_island(canon(), scenario(), live.replay_log(), until)
        .expect("the log replays")
        .state_digest();
    assert_eq!(
        hex(replayed),
        live_digest,
        "an intervened-in run must replay to the same island"
    );

    // And through a file, which is how a log actually travels. An
    // intervention that serialized badly would reach exactly this far in
    // memory and no further, so the in-memory replay above is not enough
    // on its own.
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("intervened.json");
    live.replay_log().save(&path).unwrap();
    let from_disk = IslandReplayLog::load(&path).expect("the log reads back");
    assert_eq!(from_disk.entries.len(), script.len());
    let replayed = replay_island(canon(), scenario(), &from_disk, until)
        .expect("the log replays from disk")
        .state_digest();
    assert_eq!(hex(replayed), live_digest);
}

#[test]
fn a_refused_intervention_never_reaches_the_log() {
    let mut life = island();
    let (row, col) = a_land_cell(&life);
    let before = life.replay_log().entries.len();
    let command = IslandCommand::Intervention(Box::new(InterventionAction::TriggerDisturbance {
        disturbance_type: DisturbanceType::Fire,
        intensity: 0.5,
        location: at(&life, row, col),
    }));
    match life.apply_command(command) {
        Err(CommandError::Intervention(IslandInterventionError::NotSupportedOnIsland {
            action,
            ..
        })) => assert_eq!(action, "TriggerDisturbance"),
        other => panic!("the island runs no disturbances, got {other:?}"),
    }
    assert_eq!(
        life.replay_log().entries.len(),
        before,
        "a refused command did not happen, so it is not in the record"
    );
}

#[test]
fn an_applied_intervention_is_in_the_log_with_the_tick_it_landed_on() {
    let mut life = island();
    let (row, col) = a_land_cell(&life);
    life.advance(STEP * 4).expect("the island steps on");
    let tick = life.tick;
    let applied = life
        .apply_command(IslandCommand::Intervention(Box::new(
            InterventionAction::InjectResource {
                resource_type: ResourceType::Water,
                amount: 3.0,
                location: at(&life, row, col),
            },
        )))
        .expect("water is a material here");
    assert!(matches!(applied, Applied::Intervened(_)));
    let entry = life.replay_log().entries.last().expect("it is recorded");
    assert_eq!(entry.tick, tick);
    assert!(matches!(entry.command, IslandCommand::Intervention(_)));
}

#[test]
fn an_empty_log_and_a_log_of_refusals_reach_the_same_island() {
    // Nothing refused is applied, so a session where every intervention
    // bounced must be indistinguishable from one where none was tried.
    let until = 6;
    let mut tried = island();
    let (row, col) = a_land_cell(&tried);
    for _ in 0..until {
        let _ = tried.apply_command(IslandCommand::Intervention(Box::new(
            InterventionAction::SmoothTerrain {
                strength: 0.5,
                region: Region::new(at(&tried, row, col), 4.0),
            },
        )));
        tried.advance(STEP).expect("the island steps on");
    }
    let untouched = replay_island(
        canon(),
        scenario(),
        &IslandReplayLog::new(tried.scenario_digest_hex()),
        until,
    )
    .expect("an empty log replays");
    assert_eq!(hex(tried.state_digest()), hex(untouched.state_digest()));
}

#[test]
fn an_absurd_radius_covers_the_island_rather_than_hanging_the_thread() {
    // `validate_region` asks only that a radius be finite and positive, so
    // this action is *valid*. Before the reach was clamped to the grid it
    // was also a walk over roughly `(2 x isize::MAX + 1)^2` offsets on the
    // thread that owns the island — one accepted request, and the world
    // stops. A region larger than the island is the island.
    let mut life = island();
    let (row, col) = a_land_cell(&life);
    let coarse_cells =
        life.domain.rows(DomainLevel::Coarse) * life.domain.cols(DomainLevel::Coarse);
    let started = std::time::Instant::now();
    let outcome = life
        .intervene(&InterventionAction::ModifyClimate {
            parameter: ClimateParameter::Temperature,
            value: 290.0,
            region: Some(Region::new(at(&life, row, col), 1.0e30)),
        })
        .expect("an enormous radius is a valid region");
    match outcome {
        IslandApplied::Mutated { touched, .. } => assert_eq!(
            touched, coarse_cells,
            "a region larger than the island should be the island"
        ),
        other => panic!("expected a mutation, got {}", other.summary()),
    }
    assert!(
        started.elapsed() < std::time::Duration::from_secs(20),
        "it took {:?}, which is the hang this test exists for",
        started.elapsed()
    );
}

#[test]
fn a_shelter_goes_up_on_the_ground_people_actually_live_on() {
    // The island's buildability rule exists because upstream's does not
    // survive the change of resolution: an absolute 2 m of relief between
    // neighbouring cells is a gradient of 0.1% on a 2 km grid, and admits
    // none of this island's 66,116 land cells — the estate's own included,
    // at 8.6%, where the founders' house already stands. A gate that turns
    // down the ground people live on is not a gate, it is a bug.
    let mut life = island();
    let (row, col) = a_land_cell(&life);
    let summary = life
        .intervene(&InterventionAction::ConstructStructure {
            structure: StructureKind::WoodenShelter,
            location: at(&life, row, col),
        })
        .expect("the estate's own cell is buildable ground")
        .summary();
    assert!(summary.contains("Wooden Shelter"), "{summary}");
    assert_eq!(life.economy.structures.len(), 1, "{summary}");

    // And it is in the economy's books under the operator, as visible as
    // anything an agent builds, rather than slipped in beside them.
    assert!(
        life.economy.events.iter().any(|e| e.agent_id == "operator"),
        "the construction should be in the event log"
    );
}

#[test]
fn the_sea_and_the_mountainside_are_both_refused() {
    let mut life = island();
    let sea = a_sea_cell(&life);
    match life.intervene(&InterventionAction::ConstructStructure {
        structure: StructureKind::Storage,
        location: at(&life, sea.0, sea.1),
    }) {
        Err(IslandInterventionError::PhysicallyBlocked { reason, .. }) => {
            assert!(reason.contains("sea"), "{reason}")
        }
        other => panic!("you cannot build on the sea, got {other:?}"),
    }

    // The steepest land cell on the island, whatever and wherever it is.
    let size = life.domain.cell_size_m(DomainLevel::Medium);
    let (rows, cols) = (
        life.domain.rows(DomainLevel::Medium),
        life.domain.cols(DomainLevel::Medium),
    );
    let elevation = &life.physical.geophysics.elevation_m;
    let mask = &life.physical.geophysics.land_mask;
    let steepest = (1..rows - 1)
        .flat_map(|r| (1..cols - 1).map(move |c| (r, c)))
        .filter(|(r, c)| *mask.get(*r, *c))
        .map(|(r, c)| {
            let here = *elevation.get(r, c);
            let drop = [(0i64, 1i64), (0, -1), (1, 0), (-1, 0)]
                .iter()
                .filter(|(dr, dc)| *mask.get((r as i64 + dr) as usize, (c as i64 + dc) as usize))
                .map(|(dr, dc)| {
                    (here - *elevation.get((r as i64 + dr) as usize, (c as i64 + dc) as usize))
                        .abs()
                })
                .fold(0.0f64, f64::max);
            ((drop / size * 1e9) as u64, (r, c))
        })
        .max()
        .expect("the island has land");
    let gradient = steepest.0 as f64 / 1e9;
    assert!(
        gradient > mk_engine::regional::geophysics::MAX_BUILD_GRADIENT,
        "this island's steepest land is {:.1}%, under the {:.1}% limit, so there is nothing \
         here this test can refuse — it would pass vacuously",
        gradient * 100.0,
        mk_engine::regional::geophysics::MAX_BUILD_GRADIENT * 100.0
    );
    let (r, c) = steepest.1;
    match life.intervene(&InterventionAction::ConstructStructure {
        structure: StructureKind::StoneHouse,
        location: at(&life, r, c),
    }) {
        Err(IslandInterventionError::PhysicallyBlocked { reason, .. }) => {
            assert!(
                reason.contains('%'),
                "the refusal should give the gradient: {reason}"
            )
        }
        other => panic!("mountainside is not a building plot, got {other:?}"),
    }
}

#[test]
fn the_buildability_rule_survives_a_change_of_resolution() {
    use mk_core::grid::{Grid2, GridSpec};
    use mk_engine::regional::geophysics::{is_buildable_cell, MAX_BUILD_GRADIENT};

    // The same hillside, described by two grids. A rule written as a height
    // difference calls these two different places; a gradient does not, and
    // that is the whole reason the island does not use upstream's.
    let spec = GridSpec::new(3, 3);
    let mut coarse = Grid2::new(&spec, 0.0);
    let mut fine = Grid2::new(&spec, 0.0);
    let land = Grid2::new(&spec, true);
    // A 10% slope: 200 m over 2 km, and 2 m over 20 m.
    coarse.set(1, 2, 200.0);
    fine.set(1, 2, 2.0);
    assert!(is_buildable_cell(&coarse, &land, 2000.0, 1, 1));
    assert!(is_buildable_cell(&fine, &land, 20.0, 1, 1));

    // And a 50% slope is refused at either resolution.
    let mut coarse = Grid2::new(&spec, 0.0);
    let mut fine = Grid2::new(&spec, 0.0);
    coarse.set(1, 2, 1000.0);
    fine.set(1, 2, 10.0);
    // The 50% slope below only tests anything while it is over the limit.
    const { assert!(0.5 > MAX_BUILD_GRADIENT) };
    assert!(!is_buildable_cell(&coarse, &land, 2000.0, 1, 1));
    assert!(!is_buildable_cell(&fine, &land, 20.0, 1, 1));

    // The coast is buildable: a sea neighbour is not a drop to the sea
    // floor, and treating it as one would make every shoreline refuse.
    let mut shore = Grid2::new(&spec, 5.0);
    shore.set(1, 2, -800.0);
    let mut mask = Grid2::new(&spec, true);
    mask.set(1, 2, false);
    assert!(is_buildable_cell(&shore, &mask, 2000.0, 1, 1));
}

#[test]
fn a_summary_a_person_reads_counts_one_cell_as_one_cell() {
    // These strings go straight onto the dashboard after an intervention.
    // A region small enough to touch exactly one cell used to report "over
    // 1 coarse cells", which is the sort of thing that makes software look
    // like it is talking to itself.
    let mut life = island();
    let (row, col) = a_land_cell(&life);
    let one = life
        .intervene(&InterventionAction::ModifyClimate {
            parameter: ClimateParameter::Temperature,
            value: 291.0,
            // Smaller than a coarse cell, so only the centre is touched.
            region: Some(Region::new(at(&life, row, col), 0.2)),
        })
        .expect("the climate is editable")
        .summary();
    assert!(one.contains("over 1 coarse cell,"), "{one}");
    assert!(!one.contains("1 coarse cells"), "{one}");

    let many = life
        .intervene(&InterventionAction::ModifyClimate {
            parameter: ClimateParameter::Temperature,
            value: 292.0,
            region: Some(Region::new(at(&life, row, col), 400.0)),
        })
        .expect("the climate is editable")
        .summary();
    assert!(many.contains("coarse cells"), "{many}");
}
