//! The schema the frontends compile against is the schema the server
//! sends.
//!
//! `mk_island_api` exists so that the web app, the desktop application and
//! this server cannot disagree about what `/api/world` looks like. That
//! guarantee is only worth as much as the thing that checks it, because
//! two of the three ways it can break are silent:
//!
//! - A field added to the server's own `IslandProjection` and not to
//!   `mk_island_api::World` would be serialized and then quietly dropped by
//!   every client, which looks exactly like a server that never sent it.
//! - An intervention constructor in `mk_island_api` that stopped matching
//!   the island's real `InterventionAction` would send a body the island
//!   refuses as malformed, and the refusal would be blamed on the operator.
//!
//! The third way — a field in `World` the server stopped sending — is the
//! one serde already catches, because `World` defaults nothing.
//!
//! So: one test that compares the key sets, and one that puts every
//! constructor through the real `InterventionAction`.

use island::serve::projection::IslandProjection;
use mk_island_api::{InterventionRequest, InterventionTarget, World};

/// A projection with something in every field, so the comparison is not
/// made against a struct of zeroes where a missing key could hide.
fn a_projection() -> IslandProjection {
    let mut projection = IslandProjection {
        running: true,
        records_at_tick: 11,
        economy_at_tick: 22,
        ..Default::default()
    };
    projection.clock.tick = 33;
    projection.clock.requested_speed = "max".to_string();
    projection.clock.achieved_speed = Some(44.0);
    projection.clock.day_length_hours = 36.0;
    projection.estate.cell = (5, 6);
    projection.estate.latitude = -41.0;
    projection.land.rows = 960;
    projection.land.cell_size_m = 2_000.0;
    projection.stocks.audits_closed = 7;
    projection.digest.value = "628d33bc".to_string();
    projection.digest.at_tick = 33;
    projection.digest.current = true;
    projection.people.push(mk_island_api::Person {
        agent_id: "HUM-000001".to_string(),
        alive: true,
        asleep: false,
        age_years: 33.0,
        space: Some("Bedroom".to_string()),
        position_m: Some((1.0, 2.0)),
        cell: Some((5, 6)),
        body_carbon_kg: Some(16.0),
        height_m: 1.74,
    });
    projection
}

fn keys(value: &serde_json::Value) -> Vec<String> {
    let mut keys: Vec<String> = value
        .as_object()
        .expect("an object")
        .keys()
        .cloned()
        .collect();
    keys.sort();
    keys
}

#[test]
fn what_the_server_publishes_is_exactly_what_a_client_reads() {
    let projection = a_projection();
    let sent = serde_json::to_value(&projection).expect("the projection serializes");

    // Direction one: everything `World` needs is there. `World` defaults
    // no field, so a field the server stopped sending fails right here
    // with the name of the missing one.
    let read: World = serde_json::from_value(sent.clone()).unwrap_or_else(|err| {
        panic!(
            "a client cannot read what this server publishes: {err}. The server sends {:?}.",
            keys(&sent)
        )
    });

    // Direction two: and nothing more. This is the silent one — serde
    // ignores an unknown key without complaint, so without this a field
    // added to `IslandProjection` alone would reach no frontend and look
    // like a server that never sent it.
    let round_tripped = serde_json::to_value(&read).expect("World serializes");
    assert_eq!(
        keys(&sent),
        keys(&round_tripped),
        "the server publishes fields the shared schema does not carry, so no frontend will see \
         them: add them to `mk_island_api::World` as well"
    );

    // And the values survive, not just the names.
    assert!(read.running);
    assert_eq!(read.clock.tick, 33);
    assert_eq!(read.clock.requested_speed, "max");
    assert_eq!(read.clock.achieved_speed, Some(44.0));
    assert_eq!(read.clock.day_length_hours, 36.0);
    assert_eq!(read.estate.cell, (5, 6));
    assert_eq!(read.land.cell_size_m, 2_000.0);
    assert_eq!(read.stocks.audits_closed, 7);
    assert_eq!(read.records_at_tick, 11);
    assert_eq!(read.economy_at_tick, 22);
    assert_eq!(read.digest.value, "628d33bc");
    assert!(read.digest.current);
    assert_eq!(read.people.len(), 1);
    assert_eq!(read.people[0].agent_id, "HUM-000001");
    assert_eq!(read.people[0].space.as_deref(), Some("Bedroom"));
}

#[test]
fn every_intervention_constructor_builds_an_intervention_the_island_understands() {
    use mk_interventions::{
        BiomassType, ClimateParameter, InterventionAction, ResourceType, StructureKind,
    };

    let at = InterventionTarget::new(-41.03, 173.56);
    let region = at.region(25.0);

    let cases: Vec<(&str, InterventionRequest)> = vec![
        (
            "modify_climate",
            InterventionRequest::modify_climate("Temperature", 1.5, Some(region.clone())),
        ),
        (
            "modify_climate over the whole island",
            InterventionRequest::modify_climate("Precipitation", -0.5, None),
        ),
        ("inject_water", InterventionRequest::inject_water(100.0, at)),
        (
            "inject_producers",
            InterventionRequest::inject_producers(2.0, region.clone()),
        ),
        (
            "construct",
            InterventionRequest::construct("WoodenShelter", at),
        ),
        (
            "remove_human",
            InterventionRequest::remove_human("HUM-000003"),
        ),
        (
            "sculpt_terrain",
            InterventionRequest::sculpt_terrain(10.0, region.clone()),
        ),
    ];

    for (name, request) in &cases {
        let parsed: InterventionAction =
            serde_json::from_value(request.0.clone()).unwrap_or_else(|err| {
                panic!(
                    "`InterventionRequest::{name}` is not an intervention the island can read: \
                     {err}. It built {}",
                    request.0
                )
            });
        // Parsing is most of it, but a body that parsed into the *wrong*
        // variant would pass that and be wrong in the one way an operator
        // would not forgive, so each is named.
        match (name.split(' ').next().unwrap(), parsed) {
            ("modify_climate", InterventionAction::ModifyClimate { parameter, .. }) => {
                assert!(matches!(
                    parameter,
                    ClimateParameter::Temperature | ClimateParameter::Precipitation
                ));
            }
            (
                "inject_water",
                InterventionAction::InjectResource {
                    resource_type,
                    amount,
                    location,
                },
            ) => {
                assert!(matches!(resource_type, ResourceType::Water));
                assert_eq!(amount, 100.0);
                assert_eq!(location.latitude, -41.03_f32);
            }
            ("inject_producers", InterventionAction::InjectBiomass { biomass_type, .. }) => {
                assert!(matches!(biomass_type, BiomassType::Producers));
            }
            ("construct", InterventionAction::ConstructStructure { structure, .. }) => {
                assert_eq!(structure, StructureKind::WoodenShelter);
            }
            ("remove_human", InterventionAction::RemoveHuman { human_id }) => {
                assert_eq!(human_id, "HUM-000003");
            }
            ("sculpt_terrain", InterventionAction::SculptTerrain { .. }) => {}
            (_, other) => panic!("`InterventionRequest::{name}` built a {other:?}"),
        }
    }
}

#[test]
fn a_control_request_is_the_json_the_dashboard_has_always_posted() {
    use mk_island_api::ControlRequest;

    let cases = [
        (
            ControlRequest::Pause,
            serde_json::json!({"command": "pause"}),
        ),
        (
            ControlRequest::Resume,
            serde_json::json!({"command": "resume"}),
        ),
        (
            ControlRequest::Step { ticks: 60 },
            serde_json::json!({"command": "step", "ticks": 60}),
        ),
        (
            ControlRequest::Snapshot,
            serde_json::json!({"command": "snapshot"}),
        ),
        (
            ControlRequest::SetSpeed {
                speed: "max".to_string(),
            },
            serde_json::json!({"command": "set_speed", "speed": "max"}),
        ),
    ];
    for (request, expected) in cases {
        assert_eq!(
            serde_json::to_value(&request).expect("serializes"),
            expected,
            "a control request's wire shape changed; the web app posts the old one"
        );
    }
}
