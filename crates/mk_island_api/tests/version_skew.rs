//! What happens when a frontend and a backend are not the same version.
//!
//! This is not a hypothetical. The whole point of `mk_island_api` is that
//! the island runs on one machine and the dashboards run on others, and
//! machines are not upgraded at the same moment. A frontend on a laptop
//! will read a backend that was restarted last week, and a backend that
//! was restarted this morning will be read by a dashboard nobody has
//! reloaded. Both directions have to work, and "serde does the right
//! thing" is a belief until something checks it.
//!
//! Two directions, two different mechanisms:
//!
//! * **An old backend read by a new frontend.** The JSON simply lacks the
//!   fields that were added later. Every one of them carries
//!   `#[serde(default)]`, so it must parse, and the missing value must be
//!   the one the frontends are written to treat as "not said" -- zero for
//!   a size or a height, empty for a provenance string. `estate.rs` and
//!   `people.js` both branch on exactly that.
//! * **A new backend read by an old frontend.** The JSON carries fields
//!   the old type has never heard of. Serde ignores unknown fields unless
//!   a type opts into `deny_unknown_fields`, and nothing here does -- but
//!   that is a property of the code as written, so it is asserted against
//!   a payload full of fields from a future nobody has written yet.
//!
//! The payloads below are written out by hand rather than produced by
//! serialising the current types. A round trip through today's code would
//! pass whatever today's code happens to emit and would notice nothing
//! when a field stopped being optional.

use mk_island_api::wire::{BuildingFootprint, Conversation, Person, PlacedItem};

#[test]
fn a_conversation_from_a_backend_that_predates_spanning_ticks_still_reads() {
    // What `/api/conversations` served before an exchange could run
    // across ticks: one tick, no `last_tick`, no `lines_said`.
    let old = r#"{
        "tick": 1799,
        "relationship": "founders",
        "lines": [
            {"speaker_id": "1", "speaker_name": "Gem-D", "text": "There you are."}
        ]
    }"#;

    let conversation: Conversation = serde_json::from_str(old).expect("an old conversation reads");
    assert_eq!(conversation.tick, 1799);
    assert_eq!(conversation.lines.len(), 1);
    // Zero rather than a guess. A frontend reads this as "no span given"
    // and shows a single tick, which is what the old backend meant.
    assert_eq!(conversation.last_tick, 0);
    // Zero means "no count given", not "nothing was said" -- the lines are
    // right there. Both frontends only claim to be showing a tail when
    // this is larger than the number of lines they were handed.
    assert_eq!(conversation.lines_said, 0);
    assert!(
        conversation.lines_said <= conversation.lines.len(),
        "an absent count must never make a frontend say it is showing a tail of something longer"
    );
}

#[test]
fn an_item_from_a_backend_that_had_no_real_sizes_still_reads() {
    let old = r#"{
        "kind": "Vehicle",
        "name": "Ford Ranger Raptor",
        "position_m": [12.0, 4.0],
        "rotation_deg": 90.0
    }"#;

    let item: PlacedItem = serde_json::from_str(old).expect("an old item reads");
    assert_eq!(item.name, "Ford Ranger Raptor");
    // The frontends treat a zero size as "the island did not say" and fall
    // back to their own drawing convention, which is exactly what this
    // backend meant by not sending one.
    assert_eq!(item.size_m, (0.0, 0.0, 0.0));
    assert!(item.size_source.is_empty());
}

#[test]
fn a_building_from_a_backend_that_had_no_real_heights_still_reads() {
    let old = r#"{
        "name": "machinery shed",
        "kind": "Shed",
        "rect_m": {"x0": 0.0, "y0": 0.0, "x1": 12.0, "y1": 8.0},
        "rotation_deg": 0.0
    }"#;

    let building: BuildingFootprint = serde_json::from_str(old).expect("an old building reads");
    assert_eq!(building.name, "machinery shed");
    // `estate.rs` falls back to `WALL_HEIGHT_M` on exactly this zero,
    // which is how a new desktop application draws an old island's shed
    // without pretending to know how tall it is.
    assert_eq!(building.height_m, 0.0);
    assert!(building.height_source.is_empty());
}

#[test]
fn a_person_from_a_backend_that_sent_no_height_still_reads() {
    let old = r#"{
        "agent_id": "Gem-D",
        "alive": true,
        "asleep": false,
        "age_years": 25.0,
        "space": "Gem-D's Bedroom",
        "position_m": [113967.0, 124003.0],
        "cell": [62, 56],
        "body_carbon_kg": 13.0
    }"#;

    let person: Person = serde_json::from_str(old).expect("an old person reads");
    assert_eq!(person.agent_id, "Gem-D");
    // Zero is "not said". The desktop application stands such a person at
    // the mean of the population the island draws heights from, rather
    // than at nothing or at a figure it chose for everybody.
    assert_eq!(person.height_m, 0.0);
}

#[test]
fn a_frontend_reading_a_backend_newer_than_itself_ignores_what_it_does_not_know() {
    // Fields from a version that has not been written. If any of these
    // types ever takes `deny_unknown_fields`, every deployed frontend
    // starts failing the moment the island is upgraded, and it fails as a
    // parse error rather than as a missing value -- the whole page, not
    // one field. So this is asserted rather than assumed.
    let newer = r#"{
        "tick": 10,
        "last_tick": 420,
        "relationship": "founders",
        "lines": [
            {
                "speaker_id": "1",
                "speaker_name": "Gem-D",
                "text": "There you are.",
                "tone": "warm",
                "language": "island-common"
            }
        ],
        "lines_said": 1021,
        "setting": "the kitchen",
        "overheard_by": ["Gem-K"]
    }"#;

    let conversation: Conversation =
        serde_json::from_str(newer).expect("a newer backend's conversation still reads");
    assert_eq!(conversation.tick, 10);
    assert_eq!(conversation.last_tick, 420);
    assert_eq!(conversation.lines_said, 1021);
    assert_eq!(conversation.lines.len(), 1);
    assert_eq!(conversation.lines[0].speaker_name, "Gem-D");
}

#[test]
fn todays_payload_round_trips_so_these_old_shapes_are_the_only_difference() {
    // A guard on the tests above rather than on the types: if the current
    // shape ever stopped round-tripping, the old-shape tests would still
    // pass and would be proving nothing about the real wire.
    let now = Conversation {
        tick: 10,
        last_tick: 420,
        relationship: "founders".to_string(),
        lines: Vec::new(),
        lines_said: 1021,
    };
    let json = serde_json::to_string(&now).expect("it serialises");
    let back: Conversation = serde_json::from_str(&json).expect("it comes back");
    assert_eq!(back, now);
}
