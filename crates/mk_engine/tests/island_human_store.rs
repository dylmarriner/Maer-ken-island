//! Phase 4 Task 3b: a folder for every human, under a run of its own.
//!
//! The point of these tests is not that files appear. It is that the island
//! runs the same whether it is keeping records or not — the folders are a
//! record of what happened, never an input to it — and that two runs under
//! one save root cannot be mistaken for each other.

use std::path::PathBuf;
use std::sync::Arc;

use mk_core::canon::CanonLocked;
use mk_engine::io::save_island_snapshot;
use mk_engine::regional::life::IslandLife;
use mk_island::IslandScenario;

const HOUR: u64 = 3_600;

fn repo(path: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path)
}

fn canon() -> Arc<CanonLocked> {
    Arc::new(CanonLocked::load(&repo("fixtures/island/canon.json")).unwrap())
}

/// The default island with a small tree cap: nothing here depends on the
/// patch, and 200,000 stems make every bootstrap slow.
fn island() -> IslandLife {
    let mut scenario =
        IslandScenario::load(&repo("fixtures/island/default_scenario.json")).unwrap();
    scenario.estate_patch.tree_cap = 200;
    IslandLife::bootstrap(scenario, canon()).expect("the island bootstraps")
}

fn hex(d: [u8; 32]) -> String {
    d.iter().map(|b| format!("{b:02x}")).collect()
}

#[test]
fn every_founder_gets_a_folder_when_the_store_is_enabled() {
    let dir = tempfile::tempdir().unwrap();
    let mut life = island();
    let run = life
        .enable_human_store(dir.path())
        .expect("the store opens");

    let humans = dir.path().join(run.as_str()).join("humans");
    assert!(humans.is_dir(), "{} is not a directory", humans.display());
    for id in ["Gem-D", "Gem-K"] {
        assert!(
            humans.join(id).is_dir(),
            "{id} has no folder under {}",
            humans.display()
        );
    }

    // The run is named in the save root's own index, so a person looking at
    // the directory can tell which run a folder belongs to.
    let runs: serde_json::Value =
        serde_json::from_slice(&std::fs::read(dir.path().join("runs.json")).unwrap()).unwrap();
    assert_eq!(runs["next"], 1);
    assert_eq!(runs["ids"][0], run.as_str());
}

#[test]
fn a_second_run_under_the_same_root_gets_its_own_folders() {
    let dir = tempfile::tempdir().unwrap();

    let mut first = island();
    let a = first.enable_human_store(dir.path()).unwrap();
    let mut second = island();
    let b = second.enable_human_store(dir.path()).unwrap();

    assert_ne!(a, b, "two runs of the same scenario reused one id");
    assert!(dir.path().join(a.as_str()).is_dir());
    assert!(dir.path().join(b.as_str()).is_dir());

    let runs: serde_json::Value =
        serde_json::from_slice(&std::fs::read(dir.path().join("runs.json")).unwrap()).unwrap();
    assert_eq!(runs["next"], 2, "the run counter did not advance");
}

#[test]
fn keeping_records_does_not_change_what_happens() {
    let dir = tempfile::tempdir().unwrap();

    let mut kept = island();
    kept.enable_human_store(dir.path()).unwrap();
    let mut plain = island();

    assert_eq!(
        hex(kept.state_digest()),
        hex(plain.state_digest()),
        "attaching a store changed the island before it even ran"
    );

    kept.advance(6 * HOUR).unwrap();
    plain.advance(6 * HOUR).unwrap();

    assert_eq!(
        hex(kept.state_digest()),
        hex(plain.state_digest()),
        "six hours with records diverged from six hours without"
    );
    assert_eq!(kept.audits_closed, plain.audits_closed);
    assert_eq!(kept.shortfalls, plain.shortfalls);
}

#[test]
fn the_island_runs_on_when_every_write_fails() {
    let dir = tempfile::tempdir().unwrap();
    let mut life = island();
    let run = life.enable_human_store(dir.path()).unwrap();

    // Take the humans directory away underneath the store. Every write from
    // here on has nowhere to land.
    let humans = dir.path().join(run.as_str()).join("humans");
    std::fs::remove_dir_all(&humans).unwrap();
    std::fs::write(&humans, b"not a directory any more").unwrap();

    let mut plain = island();
    life.advance(2 * HOUR).unwrap();
    plain.advance(2 * HOUR).unwrap();

    assert_eq!(
        hex(life.state_digest()),
        hex(plain.state_digest()),
        "a failing store changed the island"
    );
    let (failed, last) = life.human_store().unwrap().failures();
    assert!(failed > 0, "failures were not counted");
    assert!(last.is_some(), "no reason was kept for the failures");
}

#[test]
fn the_full_state_is_written_on_the_cadence_and_not_every_step() {
    let dir = tempfile::tempdir().unwrap();
    let mut life = island();
    let run = life.enable_human_store(dir.path()).unwrap();
    let cadence = life.scenario.cadences.human_store_seconds;
    let step = life.scenario.cadences.human_seconds;
    assert!(
        cadence > step,
        "this test is meaningless unless the store cadence is slower than the human step"
    );

    let state = dir
        .path()
        .join(run.as_str())
        .join("humans")
        .join("Gem-D")
        .join("profile")
        // `.enc.json`, not `.json`: the island opens its store through
        // `HumanStorage::try_new`, which refuses rather than writing a
        // person's record in plaintext, so the encrypted name is the only
        // one that ever exists here.
        .join("identity.enc.json");
    assert!(state.is_file(), "no profile at {}", state.display());

    // One human step short of the cadence: the folder exists from the
    // enable, but nothing has rewritten it.
    let at_enable = std::fs::metadata(&state).unwrap().modified().unwrap();
    life.advance(cadence - step).unwrap();
    assert_eq!(
        std::fs::metadata(&state).unwrap().modified().unwrap(),
        at_enable,
        "the full state was rewritten before the cadence came round"
    );
    assert!(
        !life.humans.auto_sync(),
        "the island left upstream's every-step sync on"
    );

    // And the step that reaches the cadence writes it. The file itself is
    // encrypted, so what is checked is that it was rewritten, not what it
    // says.
    life.advance(step).unwrap();
    assert!(
        std::fs::metadata(&state).unwrap().modified().unwrap() > at_enable,
        "the cadence came round and the full state was not rewritten"
    );
}

#[test]
fn a_snapshot_brings_the_folders_up_to_date_with_it() {
    let dir = tempfile::tempdir().unwrap();
    let saves = dir.path().join("saves");
    let mut life = island();
    let run = life.enable_human_store(&saves).unwrap();

    // Stop one human step short of the store's cadence, so the folders are
    // deliberately behind when the snapshot is taken.
    let cadence = life.scenario.cadences.human_store_seconds;
    let step = life.scenario.cadences.human_seconds;
    life.advance(cadence - step).unwrap();

    let path = dir.path().join("island.mks");
    save_island_snapshot(&mut life, &path).expect("the island saves");

    let state = saves
        .join(run.as_str())
        .join("humans")
        .join("Gem-D")
        .join("profile")
        // `.enc.json`, not `.json`: the island opens its store through
        // `HumanStorage::try_new`, which refuses rather than writing a
        // person's record in plaintext, so the encrypted name is the only
        // one that ever exists here.
        .join("identity.enc.json");
    let written = std::fs::metadata(&state).unwrap().modified().unwrap();
    let snapshot = std::fs::metadata(&path).unwrap().modified().unwrap();
    assert!(
        written <= snapshot,
        "the folders were not brought up to date before the snapshot was written"
    );
    let (failed, last) = life.human_store().unwrap().failures();
    assert_eq!(failed, 0, "the save failed to write a folder: {last:?}");
}

#[test]
fn a_death_is_written_at_once_rather_than_at_the_next_cadence() {
    let dir = tempfile::tempdir().unwrap();
    let mut life = island();
    let run = life.enable_human_store(dir.path()).unwrap();
    let step = life.scenario.cadences.human_seconds;
    let cadence = life.scenario.cadences.human_store_seconds;

    let state = dir
        .path()
        .join(run.as_str())
        .join("humans")
        .join("Gem-K")
        .join("profile")
        .join("identity.enc.json");
    let at_enable = std::fs::metadata(&state).unwrap().modified().unwrap();

    // Nobody dies of anything in a few minutes on this island, so the death
    // is staged directly. What is under test is the store's response to one,
    // not the biology that would eventually produce it.
    life.humans
        .registry
        .get_human_mut("Gem-K")
        .unwrap()
        .profile
        .status = mk_core::human::HumanStatus::Dead;

    life.advance(step).unwrap();
    assert!(
        step < cadence,
        "this test is meaningless unless one step is short of the store cadence"
    );
    assert!(
        std::fs::metadata(&state).unwrap().modified().unwrap() > at_enable,
        "a death waited for the cadence instead of being written at once"
    );

    // And it is written once, not once per step from here on.
    let at_death = std::fs::metadata(&state).unwrap().modified().unwrap();
    life.advance(step).unwrap();
    assert_eq!(
        std::fs::metadata(&state).unwrap().modified().unwrap(),
        at_death,
        "the dead are being rewritten every step"
    );
}
