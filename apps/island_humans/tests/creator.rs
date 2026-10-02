//! Phase 0b Task 2: a stored population where every human has their own
//! folder, created through upstream's spawn path.

use island_humans::{read_creations, CreateHumanError, CreateHumanRequest, IslandHumanPopulation};
use std::path::Path;

const SEED: [u8; 32] = [7u8; 32];

fn request(name: &str) -> CreateHumanRequest {
    CreateHumanRequest {
        name: name.to_string(),
        biological_sex: "male".to_string(),
        birth_timestamp: "1988-03-02T08:30:00+12:00".to_string(),
        birth_latitude: -41.2,
        birth_longitude: 174.8,
        age_years: 38.0,
        height_cm: 181.0,
        build: "Athletic".to_string(),
        hair_color: "Black".to_string(),
        eye_color: "Hazel".to_string(),
        skin_tone: "Olive".to_string(),
    }
}

const SUBDIRS: [&str; 12] = [
    "profile",
    "traits",
    "cognition",
    "social",
    "development",
    "reproduction",
    "memories/episodic",
    "memories/semantic",
    "memories/procedural",
    "state",
    "relationships",
    "events",
];

fn assert_full_folder(dir: &Path) {
    for sub in SUBDIRS {
        assert!(
            dir.join(sub).is_dir(),
            "missing {}",
            dir.join(sub).display()
        );
    }
}

#[test]
fn first_open_gives_the_founders_folders() {
    let tmp = tempfile::tempdir().unwrap();
    let (population, warnings) = IslandHumanPopulation::open(tmp.path(), SEED).unwrap();
    assert!(warnings.is_empty(), "{warnings:?}");
    assert_eq!(population.len(), 2);
    assert_full_folder(&tmp.path().join("humans/Gem-D"));
    assert_full_folder(&tmp.path().join("humans/Gem-K"));
}

#[test]
fn a_created_human_gets_a_folder_and_a_created_event() {
    let tmp = tempfile::tempdir().unwrap();
    let (mut population, _) = IslandHumanPopulation::open(tmp.path(), SEED).unwrap();
    let created = population
        .create_human(request("Tama Ngata"), "dashboard")
        .unwrap();
    assert_eq!(created.summary.agent_id, "tama-ngata");
    assert_eq!(created.storage_error, None);
    let folder = created.folder.expect("stored population reports a folder");
    assert_eq!(folder, tmp.path().join("humans/tama-ngata"));
    assert_full_folder(&folder);
    let events = std::fs::read_to_string(folder.join("events/event_log.jsonl")).unwrap();
    let event: serde_json::Value = serde_json::from_str(events.lines().last().unwrap()).unwrap();
    assert_eq!(event["kind"], "created");
    assert_eq!(event["by"], "dashboard");
    assert_eq!(event["counter"], 0);
    let human = population.get("tama-ngata").unwrap();
    assert_eq!(human.body.hair_color, "black");
    assert_eq!(human.body.height_cm, 181.0);
}

#[test]
fn reopening_reloads_everyone() {
    let tmp = tempfile::tempdir().unwrap();
    {
        let (mut population, _) = IslandHumanPopulation::open(tmp.path(), SEED).unwrap();
        population.create_human(request("Aroha"), "test").unwrap();
        population.create_human(request("Aroha"), "test").unwrap();
    }
    let (population, _) = IslandHumanPopulation::open(tmp.path(), SEED).unwrap();
    let mut ids: Vec<String> = population
        .summaries()
        .into_iter()
        .map(|s| s.agent_id)
        .collect();
    ids.sort();
    assert_eq!(ids, ["Gem-D", "Gem-K", "aroha", "aroha-2"]);
}

#[test]
fn reopening_with_a_different_seed_is_refused() {
    let tmp = tempfile::tempdir().unwrap();
    IslandHumanPopulation::open(tmp.path(), SEED).unwrap();
    assert!(IslandHumanPopulation::open(tmp.path(), [8u8; 32]).is_err());
}

#[test]
fn replaying_the_creation_log_reproduces_the_same_people() {
    let original = tempfile::tempdir().unwrap();
    {
        let (mut population, _) = IslandHumanPopulation::open(original.path(), SEED).unwrap();
        for name in ["Mere", "Hemi", "Mere"] {
            population.create_human(request(name), "test").unwrap();
        }
    }
    let log = read_creations(original.path()).unwrap();
    assert_eq!(log.len(), 3);
    assert_eq!(log.iter().map(|r| r.counter).collect::<Vec<_>>(), [0, 1, 2]);

    let replay = tempfile::tempdir().unwrap();
    let (mut replayed, _) = IslandHumanPopulation::open(replay.path(), SEED).unwrap();
    for record in &log {
        replayed
            .create_human(record.request.clone(), &record.by)
            .unwrap();
    }
    let (reloaded, _) = IslandHumanPopulation::open(original.path(), SEED).unwrap();
    for record in &log {
        let a = serde_json::to_value(&reloaded.get(&record.agent_id).unwrap().profile).unwrap();
        let b = serde_json::to_value(&replayed.get(&record.agent_id).unwrap().profile).unwrap();
        assert_eq!(a, b, "{}", record.agent_id);
    }
}

#[test]
fn invalid_requests_are_rejected_with_upstream_messages_and_nothing_is_written() {
    let tmp = tempfile::tempdir().unwrap();
    let (mut population, _) = IslandHumanPopulation::open(tmp.path(), SEED).unwrap();
    let mut bad = request("Bad");
    bad.height_cm = 20.0;
    bad.birth_timestamp = "last tuesday".into();
    match population.create_human(bad, "test") {
        Err(CreateHumanError::Invalid(errors)) => {
            assert!(errors.iter().any(|e| e.contains("height")), "{errors:?}");
            assert!(errors.iter().any(|e| e.contains("RFC 3339")), "{errors:?}");
        }
        other => panic!("expected invalid, got {other:?}"),
    }
    assert_eq!(population.len(), 2);
    assert!(!tmp.path().join("humans/bad").exists());
    assert!(read_creations(tmp.path()).unwrap().is_empty());
}

#[test]
fn a_human_is_still_created_when_their_folder_cannot_be_written() {
    let tmp = tempfile::tempdir().unwrap();
    let (mut population, _) = IslandHumanPopulation::open(tmp.path(), SEED).unwrap();
    // A file already sits where Rangi's folder would go, so the folder write
    // fails (this works even when running as root, unlike permissions).
    std::fs::write(tmp.path().join("humans/rangi"), b"in the way").unwrap();
    let created = population.create_human(request("Rangi"), "test").unwrap();
    assert!(created.storage_error.is_some(), "{created:?}");
    assert!(
        population.get("rangi").is_some(),
        "the human exists despite the disk"
    );
    // The creation is still recorded, so it can be replayed.
    assert_eq!(read_creations(tmp.path()).unwrap().len(), 1);
}
