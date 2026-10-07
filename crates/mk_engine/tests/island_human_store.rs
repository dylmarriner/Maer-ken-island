//! Phase 4 Task 3b: a folder for every human, written through upstream
//! `HumanStorage`, never changing the simulation.

use std::path::PathBuf;
use std::sync::Arc;

use mk_core::canon::CanonLocked;
use mk_core::human::BiologicalSex;
use mk_engine::io::human_storage::HumanStorage;
use mk_engine::regional::commands::{IslandCommand, IslandCreateHumanRequest};
use mk_engine::regional::world::IslandWorldState;
use mk_island::IslandScenario;

fn repo(p: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(p)
}

fn world() -> IslandWorldState {
    let scenario = IslandScenario::load(&repo("fixtures/island/default_scenario.json")).unwrap();
    let canon = Arc::new(CanonLocked::load(&repo("fixtures/island/canon.json")).unwrap());
    IslandWorldState::new(canon, scenario).unwrap()
}

fn hex(w: &IslandWorldState) -> String {
    w.state_hash()
        .unwrap()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn folders(w: &IslandWorldState) -> Vec<String> {
    let dir = &w.human_store_status().unwrap().humans_dir;
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

fn stored_json(w: &IslandWorldState, id: &str) -> serde_json::Value {
    let dir = &w.human_store_status().unwrap().humans_dir;
    let storage = HumanStorage::try_new(dir).unwrap();
    serde_json::to_value(storage.load_human(id).unwrap().expect("stored human")).unwrap()
}

fn live_json(w: &IslandWorldState, id: &str) -> serde_json::Value {
    serde_json::to_value(w.life.humans.registry.get_human(id).unwrap()).unwrap()
}

fn ids(w: &IslandWorldState) -> Vec<String> {
    let mut v: Vec<String> = w
        .life
        .humans
        .registry
        .iter()
        .map(|h| h.agent_id().to_string())
        .collect();
    v.sort();
    v
}

#[test]
fn founders_and_arrivals_get_folders_and_a_fresh_run_gets_a_new_run_id() {
    let root = tempfile::tempdir().unwrap();
    let mut w = world();
    let run = w.enable_human_store(root.path()).unwrap();
    assert_eq!(run.len(), 12);
    assert_eq!(folders(&w), ids(&w), "every founder has a folder on enable");

    w.queue(IslandCommand::CreateHuman(IslandCreateHumanRequest {
        agent_id: "Visitor-1".into(),
        sex: BiologicalSex::Female,
        age_years: 30.0,
        appearance: None,
        location: None,
    }))
    .unwrap();
    w.step(60).unwrap();
    assert!(
        folders(&w).contains(&"Visitor-1".to_string()),
        "the arrival has a folder at once"
    );
    assert_eq!(w.human_store_status().unwrap().errors, 0);

    let mut again = world();
    let second = again.enable_human_store(root.path()).unwrap();
    assert_ne!(run, second, "a fresh run reuses no earlier run's folders");
    assert!(root.path().join(&run).join("humans").is_dir());
    assert!(root.path().join(&second).join("humans").is_dir());
}

#[test]
fn full_state_is_written_on_the_store_cadence_and_before_a_snapshot() {
    let root = tempfile::tempdir().unwrap();
    let mut w = world();
    w.enable_human_store(root.path()).unwrap();
    let id = ids(&w)[0].clone();

    for _ in 0..5 {
        w.step(60).unwrap();
    }
    assert_ne!(
        stored_json(&w, &id),
        live_json(&w, &id),
        "auto-sync is off: the folder lags"
    );
    assert_eq!(w.human_store_status().unwrap().syncs, 0);

    for _ in 0..55 {
        w.step(60).unwrap(); // reaches the first hour
    }
    assert_eq!(
        w.human_store_status().unwrap().syncs,
        1,
        "one cadence sync at one hour"
    );
    assert_eq!(stored_json(&w, &id), live_json(&w, &id));

    w.step(60).unwrap();
    assert_ne!(stored_json(&w, &id), live_json(&w, &id));
    w.sync_human_store(); // what the runner does before saving a snapshot
    assert_eq!(stored_json(&w, &id), live_json(&w, &id));
}

#[test]
fn the_state_hash_is_the_same_with_the_store_off_on_or_failing() {
    let steps = 65; // crosses one store cadence
    let mut off = world();
    for _ in 0..steps {
        off.step(60).unwrap();
    }

    let root = tempfile::tempdir().unwrap();
    let mut on = world();
    on.enable_human_store(root.path()).unwrap();
    for _ in 0..steps {
        on.step(60).unwrap();
    }
    assert_eq!(on.human_store_status().unwrap().errors, 0);

    let root2 = tempfile::tempdir().unwrap();
    let mut failing = world();
    failing.enable_human_store(root2.path()).unwrap();
    let dir = failing.human_store_status().unwrap().humans_dir.clone();
    std::fs::remove_dir_all(&dir).unwrap();
    std::fs::write(&dir, b"not a directory").unwrap();
    for _ in 0..steps {
        failing.step(60).unwrap();
    }
    let status = failing.human_store_status().unwrap();
    assert!(
        status.errors > 0 && status.last_error.is_some(),
        "failures are counted, not discarded"
    );

    assert_eq!(hex(&on), hex(&off));
    assert_eq!(hex(&failing), hex(&off));
}

#[test]
fn a_large_population_is_written_in_rotating_shards_and_a_snapshot_writes_everyone() {
    let root = tempfile::tempdir().unwrap();
    let mut w = world();
    w.enable_human_store(root.path()).unwrap();
    w.set_store_max_per_sync(1); // two founders -> two shards
    let ids = ids(&w);
    for _ in 0..60 {
        w.step(60).unwrap(); // first hour: shard 0 only
    }
    let status = w.human_store_status().unwrap();
    assert_eq!(status.syncs, 1);
    let fresh = |w: &IslandWorldState, id: &str| stored_json(w, id) == live_json(w, id);
    let first_hour: Vec<bool> = ids.iter().map(|i| fresh(&w, i)).collect();
    assert_eq!(
        first_hour.iter().filter(|b| **b).count(),
        1,
        "one shard written: {first_hour:?}"
    );
    for _ in 0..60 {
        w.step(60).unwrap(); // second hour: shard 1
    }
    let second_hour: Vec<bool> = ids.iter().map(|i| fresh(&w, i)).collect();
    assert_eq!(second_hour.iter().filter(|b| **b).count(), 1);
    assert_ne!(first_hour, second_hour, "the shards rotate");
    w.sync_human_store();
    assert!(
        ids.iter().all(|i| fresh(&w, i)),
        "a snapshot save writes everyone"
    );
}
