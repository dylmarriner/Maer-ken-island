//! Phase 4 Task 6 Step 1: creating a person on the live island.

use std::path::PathBuf;
use std::sync::Arc;

use mk_core::canon::CanonLocked;
use mk_core::human::BiologicalSex;
use mk_engine::io::island_snapshot::{load_island_snapshot, save_island_snapshot};
use mk_engine::regional::commands::{
    AuthoredAppearance, CommandError, CreateLocation, IslandCommand, IslandCreateHumanRequest,
};
use mk_engine::regional::replay::replay_island;
use mk_engine::regional::world::IslandWorldState;
use mk_island::IslandScenario;

fn repo(p: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(p)
}
fn scenario() -> IslandScenario {
    IslandScenario::load(&repo("fixtures/island/default_scenario.json")).unwrap()
}
fn canon() -> Arc<CanonLocked> {
    Arc::new(CanonLocked::load(&repo("fixtures/island/canon.json")).unwrap())
}
fn world() -> IslandWorldState {
    IslandWorldState::new(canon(), scenario()).unwrap()
}
fn hex(w: &IslandWorldState) -> String {
    w.state_hash()
        .unwrap()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
fn appearance() -> AuthoredAppearance {
    AuthoredAppearance {
        height_cm: 171.0,
        build: "Athletic".into(),
        hair_color: "Black".into(),
        eye_color: "Brown".into(),
        skin_tone: "Olive".into(),
        birth_timestamp: Some("1992-11-03T10:15:00+13:00".into()),
        birth_latitude: Some(-41.3),
        birth_longitude: Some(174.8),
    }
}
fn request(name: &str) -> IslandCreateHumanRequest {
    IslandCreateHumanRequest {
        agent_id: name.into(),
        sex: BiologicalSex::Female,
        age_years: 33.9,
        appearance: Some(appearance()),
        location: None,
    }
}
fn human_json(w: &IslandWorldState, id: &str) -> serde_json::Value {
    serde_json::to_value(w.life.humans.registry.get_human(id).unwrap()).unwrap()
}
fn invalid(w: &mut IslandWorldState, r: IslandCreateHumanRequest, needle: &str) {
    match w.queue(IslandCommand::CreateHuman(r)) {
        Err(CommandError::InvalidRequest(why)) => {
            assert!(
                why.to_lowercase().contains(needle),
                "{why:?} lacks {needle:?}"
            )
        }
        other => panic!("expected a refusal about {needle:?}, got {other:?}"),
    }
}

#[test]
fn identical_requests_at_the_same_tick_make_byte_identical_people() {
    let (mut a, mut b) = (world(), world());
    for w in [&mut a, &mut b] {
        w.step(60).unwrap();
        w.queue(IslandCommand::CreateHuman(request("Hine Moana")))
            .unwrap();
        w.step(60).unwrap();
    }
    assert_eq!(human_json(&a, "Hine Moana"), human_json(&b, "Hine Moana"));
    assert_eq!(hex(&a), hex(&b));
    let h = human_json(&a, "Hine Moana");
    assert!(h.to_string().contains("Olive") || h.to_string().to_lowercase().contains("olive"));
}

#[test]
fn every_validation_rule_refuses_with_a_specific_reason() {
    let mut w = world();
    let mut r = request("X");
    r.appearance.as_mut().unwrap().height_cm = 5.0;
    invalid(&mut w, r, "height");
    let mut r = request("X");
    r.appearance.as_mut().unwrap().build = " ".into();
    invalid(&mut w, r, "build");
    let mut r = request("X");
    r.appearance.as_mut().unwrap().birth_timestamp = Some("yesterday".into());
    invalid(&mut w, r, "rfc 3339");
    let mut r = request("X");
    r.appearance.as_mut().unwrap().birth_latitude = Some(123.0);
    invalid(&mut w, r, "birthplace");
    let mut r = request("X");
    r.age_years = 200.0;
    invalid(&mut w, r, "age");
    let mut r = request("???");
    r.appearance = None;
    r.agent_id = "   ".into();
    invalid(&mut w, r, "name");
    let mut r = request("X");
    r.location = Some(CreateLocation::EstateSpace {
        label: "Moon base".into(),
    });
    invalid(&mut w, r, "no space named");
    let mut r = request("X");
    r.location = Some(CreateLocation::EstatePoint {
        position_m: (1.0, 1.0),
    });
    invalid(&mut w, r, "outside the estate");
    let mut r = request("X");
    r.location = Some(CreateLocation::IslandCell { row: 0, col: 0 });
    invalid(&mut w, r, "not land");
    let mut r = request("X");
    r.location = Some(CreateLocation::IslandCell {
        row: 99_999,
        col: 0,
    });
    invalid(&mut w, r, "off the map");
    assert!(w.life.commands.pending.is_empty(), "refusals queue nothing");
}

#[test]
fn a_person_appears_where_asked_with_a_folder_and_a_created_event() {
    let root = tempfile::tempdir().unwrap();
    let mut w = world();
    w.enable_human_store(root.path()).unwrap();
    let mut r = request("Cook");
    r.location = Some(CreateLocation::EstateSpace {
        label: "Kitchen".into(),
    });
    w.queue(IslandCommand::CreateHuman(r)).unwrap();
    // A land cell on the island well away from the estate.
    let elevation = &w.life.physical.geophysics.elevation_m;
    let (row, col) = (0..960usize)
        .flat_map(|r| (0..1200usize).map(move |c| (r, c)))
        .find(|&(r, c)| r.abs_diff(544) > 40 && c.abs_diff(474) > 40 && *elevation.get(r, c) > 5.0)
        .expect("the island has land away from the estate");
    let mut far = request("Hiker");
    far.appearance = None;
    far.location = Some(CreateLocation::IslandCell { row, col });
    w.queue(IslandCommand::CreateHuman(far)).unwrap();
    w.step(60).unwrap();

    let at = w
        .life
        .positions
        .0
        .get("Cook")
        .expect("Cook is in the estate");
    let kitchen = w
        .life
        .placed
        .layout
        .spaces
        .iter()
        .find(|s| s.label == "Kitchen")
        .unwrap();
    assert_eq!(
        at.space,
        mk_engine::regional::estate_layout::Space::Inside(kitchen.id)
    );
    assert!(kitchen.rect_m.contains(at.position_m.0, at.position_m.1));

    let status = w.human_store_status().unwrap();
    let dir = status.humans_dir.join("Cook");
    assert!(dir.is_dir(), "a folder exists at once");
    let events = std::fs::read_to_string(dir.join("events/event_log.jsonl")).unwrap();
    assert!(events.contains("\"created\""), "{events}");
    assert_eq!(w.human_store_status().unwrap().errors, 0);
    let hiker = w
        .life
        .humans
        .registry
        .get_human("Hiker")
        .expect("the hiker arrived");
    assert_eq!(
        (hiker.position.row, hiker.position.col),
        (row as i32, col as i32)
    );
    assert!(
        !w.life.positions.0.contains_key("Hiker"),
        "off the patch: no metric position"
    );
}

#[test]
fn a_creation_survives_save_load_and_replay() {
    let mut w = world();
    let mut r = request("Visitor");
    r.location = Some(CreateLocation::EstateSpace {
        label: "Kitchen".into(),
    });
    w.queue(IslandCommand::CreateHuman(r)).unwrap();
    for _ in 0..5 {
        w.step(60).unwrap();
    }
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("a.snap");
    save_island_snapshot(&w, &path).unwrap();
    let loaded = load_island_snapshot(&path).unwrap();
    assert_eq!(hex(&loaded), hex(&w));
    assert!(loaded.life.humans.registry.get_human("Visitor").is_some());
    let replayed = replay_island(canon(), scenario(), &w.replay_log(), w.tick()).unwrap();
    assert_eq!(hex(&replayed), hex(&w), "replay reproduces the creation");
}

#[test]
fn a_failing_store_still_creates_the_person_and_reports_the_error() {
    let root = tempfile::tempdir().unwrap();
    let mut w = world();
    w.enable_human_store(root.path()).unwrap();
    let dir = w.human_store_status().unwrap().humans_dir.clone();
    std::fs::remove_dir_all(&dir).unwrap();
    std::fs::write(&dir, b"not a directory").unwrap();
    w.queue(IslandCommand::CreateHuman(request("Visitor")))
        .unwrap();
    w.step(60).unwrap();
    assert!(
        w.life.humans.registry.get_human("Visitor").is_some(),
        "created despite the failure"
    );
    let status = w.human_store_status().unwrap();
    assert!(
        status.errors > 0 && status.last_error.is_some(),
        "the failure is reported"
    );
}
