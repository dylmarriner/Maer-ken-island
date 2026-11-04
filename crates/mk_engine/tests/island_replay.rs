//! Phase 4 Task 4: an island's history, run again.
//!
//! Gate 4 asks that a fixed-seed run be reproducible. A run with nothing
//! external in it already is — `island_runtime_acceptance` pins that. What
//! these cover is a run that somebody *interfered with*: creations at
//! particular ticks, control commands in between. The same scenario and
//! the same log must reach the same canonical digest, or the log is not a
//! record of anything.

use std::path::PathBuf;
use std::sync::Arc;

use mk_core::canon::CanonLocked;
use mk_core::human::BiologicalSex;
use mk_engine::regional::commands::{ControlCommand, IslandCommand};
use mk_engine::regional::create_human::{CreateLocation, IslandCreateHuman};
use mk_engine::regional::life::IslandLife;
use mk_engine::regional::replay::{
    replay_island, IslandReplayEntry, IslandReplayError, IslandReplayLog, MAX_REPLAY_LOG_BYTES,
    REPLAY_LOG_VERSION,
};
use mk_island::IslandScenario;

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

fn bootstrap_island() -> IslandLife {
    IslandLife::bootstrap(scenario(), canon()).expect("the island bootstraps")
}

/// Every test here starts from the same island, and bootstrapping spins its
/// climate up for two orbits -- about half a minute in a debug build. So it
/// is bootstrapped once and each test gets its own copy restored from the
/// snapshot, which brings an island back exactly (`island_snapshot.rs`).
fn island() -> IslandLife {
    static BOOTSTRAPPED: std::sync::OnceLock<mk_engine::regional::life::IslandLifeSnapshot> =
        std::sync::OnceLock::new();
    let snapshot = BOOTSTRAPPED
        .get_or_init(|| bootstrap_island().snapshot())
        .clone();
    let canon = std::sync::Arc::new(
        mk_core::canon::CanonLocked::load(&repo("fixtures/island/canon.json")).expect("canon"),
    );
    IslandLife::restore(canon, snapshot).expect("the island restores")
}

fn hex(d: [u8; 32]) -> String {
    d.iter().map(|b| format!("{b:02x}")).collect()
}

fn somebody(name: &str, space: u32) -> IslandCommand {
    IslandCommand::CreateHuman(Box::new(IslandCreateHuman {
        name: name.to_string(),
        biological_sex: BiologicalSex::Female,
        birth_timestamp: "1998-07-06T05:04:03Z".to_string(),
        age_years: 28.0,
        height_cm: 171.0,
        build: "average".to_string(),
        hair_color: "black".to_string(),
        eye_color: "brown".to_string(),
        skin_tone: "olive".to_string(),
        location: CreateLocation::EstateSpace(mk_engine::regional::estate_layout::SpaceId(space)),
        birthplace_here: true,
        birth_latitude: 0.0,
        birth_longitude: 0.0,
    }))
}

/// Commands landing on the same tick keep the order they were applied in.
///
/// It matters: the second creation of a name becomes `name-2`, which only
/// holds if the first is already there. A log that lost the order would
/// come back as two different people.
#[test]
fn a_log_keeps_the_order_commands_arrived_in() {
    let mut life = island();
    life.advance(5 * STEP).unwrap();

    let space = life
        .placed
        .layout
        .spaces_outside_house()
        .next()
        .unwrap()
        .id
        .0;
    life.apply_command(somebody("Rua", space)).unwrap();
    life.apply_command(IslandCommand::Control(ControlCommand::Pause))
        .unwrap();
    life.apply_command(somebody("Rua", space)).unwrap();
    life.advance(STEP).unwrap();
    life.apply_command(somebody("Toru", space)).unwrap();

    let log = life.replay_log();
    assert_eq!(log.entries.len(), 4);
    let order: Vec<(u64, u64)> = log.entries.iter().map(|e| (e.tick, e.sequence)).collect();
    assert_eq!(order, vec![(5, 0), (5, 1), (5, 2), (6, 0)]);
    assert!(!log.scenario_digest.is_empty(), "the log names no scenario");
}

/// A run that was interfered with, reproduced from its log.
#[test]
fn replaying_a_log_reaches_the_island_the_run_reached() {
    let mut live = island();
    let space = live
        .placed
        .layout
        .spaces_outside_house()
        .next()
        .unwrap()
        .id
        .0;

    live.advance(3 * STEP).unwrap();
    live.apply_command(somebody("Anahera", space)).unwrap();
    live.advance(4 * STEP).unwrap();
    live.apply_command(IslandCommand::Control(ControlCommand::SetSpeed(
        "60".to_string(),
    )))
    .unwrap();
    live.apply_command(somebody("Anahera", space)).unwrap();
    live.advance(5 * STEP).unwrap();

    let log = live.replay_log().clone();
    let replayed = replay_island(canon(), scenario(), &log, live.tick).expect("the log replays");

    assert_eq!(
        hex(replayed.state_digest()),
        hex(live.state_digest()),
        "the replay reached a different island"
    );
    assert_eq!(replayed.tick, live.tick);
    assert_eq!(
        replayed.humans.registry.iter().count(),
        live.humans.registry.iter().count(),
        "a different number of people came out"
    );
}

/// Control commands are recorded but move nothing, so a log with them and
/// one without reach the same island.
#[test]
fn control_commands_are_recorded_and_change_nothing() {
    let mut with = island();
    let mut without = island();

    for life in [&mut with, &mut without] {
        life.advance(2 * STEP).unwrap();
    }
    with.apply_command(IslandCommand::Control(ControlCommand::Pause))
        .unwrap();
    with.apply_command(IslandCommand::Control(ControlCommand::Step(3)))
        .unwrap();
    with.apply_command(IslandCommand::Control(ControlCommand::Resume))
        .unwrap();
    for life in [&mut with, &mut without] {
        life.advance(2 * STEP).unwrap();
    }

    assert_eq!(
        hex(with.state_digest()),
        hex(without.state_digest()),
        "pausing an island changed it"
    );
    assert_eq!(with.replay_log().entries.len(), 3);
    assert_eq!(without.replay_log().entries.len(), 0);
}

/// Stop halfway, carry on, and the replay still lands where the run did.
#[test]
fn a_replay_that_stops_halfway_and_carries_on_lands_in_the_same_place() {
    let mut live = island();
    let starting_population = live.humans.registry.iter().count();
    let space = live
        .placed
        .layout
        .spaces_outside_house()
        .next()
        .unwrap()
        .id
        .0;
    live.advance(2 * STEP).unwrap();
    live.apply_command(somebody("Moana", space)).unwrap();
    live.advance(6 * STEP).unwrap();
    live.apply_command(somebody("Kahu", space)).unwrap();
    live.advance(2 * STEP).unwrap();
    let log = live.replay_log().clone();

    // Replay to halfway, then replay the whole thing, and compare the
    // second against the run. Stopping a replay early must not change
    // where a full one ends up.
    let halfway = replay_island(canon(), scenario(), &log, 5).expect("a partial replay");
    assert_eq!(halfway.tick, 5);
    assert_eq!(
        halfway.humans.registry.iter().count(),
        starting_population + 1,
        "the creation before tick 5 is missing"
    );

    let whole = replay_island(canon(), scenario(), &log, live.tick).expect("a full replay");
    assert_eq!(hex(whole.state_digest()), hex(live.state_digest()));
}

/// A log written against another scenario is refused, not reproduced
/// wrongly.
#[test]
fn a_log_from_another_island_is_refused() {
    let mut live = island();
    live.advance(STEP).unwrap();
    let mut log = live.replay_log().clone();
    // A foreign scenario must still be rejected when its digest is canonical.
    log.scenario_digest = "0".repeat(64);

    match replay_island(canon(), scenario(), &log, 2) {
        Err(IslandReplayError::WrongScenario { log, .. }) => {
            assert_eq!(log, "0".repeat(64));
        }
        Err(other) => panic!("the wrong refusal for a foreign log: {other}"),
        Ok(_) => panic!("a foreign log was accepted"),
    }
}

#[test]
fn replay_refuses_noncanonical_scenario_digest_before_scenario_comparison() {
    let log = IslandReplayLog::new("not-a-digest".to_string());

    assert!(matches!(
        replay_island(canon(), scenario(), &log, 0),
        Err(IslandReplayError::InvalidScenarioDigest)
    ));
}

#[test]
fn replay_refuses_unsupported_in_memory_log_versions() {
    let mut log = two_command_log();
    let unsupported = REPLAY_LOG_VERSION + 1;
    log.version = unsupported;

    assert!(matches!(
        replay_island(canon(), scenario(), &log, 0),
        Err(IslandReplayError::Version(version)) if version == unsupported
    ));
}

#[test]
fn partial_replay_refuses_invalid_sequences_after_the_requested_tick() {
    let mut log = two_command_log();
    log.entries[1].tick = 1;

    assert!(matches!(
        replay_island(canon(), scenario(), &log, 0),
        Err(IslandReplayError::InvalidSequence {
            tick: 1,
            expected: 0,
            actual: 1
        })
    ));
}

/// Recording commands for an earlier tick must continue that tick's sequence
/// even if another tick was appended in between. Replay sorts by tick, so
/// duplicate sequence IDs here would make the public recorder emit an invalid log.
#[test]
fn record_keeps_sequences_contiguous_when_ticks_are_interleaved() {
    let mut log = IslandReplayLog::new("scenario".to_string());
    log.record(1, somebody("Rua", 1))
        .expect("replay sequence remains available");
    log.record(2, somebody("Toru", 1))
        .expect("replay sequence remains available");
    log.record(1, somebody("Hana", 1))
        .expect("replay sequence remains available");

    let tick_one_sequences: Vec<u64> = log
        .entries
        .iter()
        .filter(|entry| entry.tick == 1)
        .map(|entry| entry.sequence)
        .collect();

    assert_eq!(tick_one_sequences, vec![0, 1]);
    assert_eq!(log.entries[1].sequence, 0);
}

/// A loaded or transported log may have entries in a different vector order.
/// Continue after the highest sequence for the tick, not the last matching vector element.
#[test]
fn record_uses_max_sequence_when_existing_entries_are_shuffled() {
    let mut log = IslandReplayLog::new("scenario".to_string());
    log.record(1, somebody("Rua", 1))
        .expect("replay sequence remains available");
    log.record(1, somebody("Hana", 1))
        .expect("replay sequence remains available");
    log.entries.reverse();

    log.record(1, somebody("Toru", 1))
        .expect("replay sequence remains available");

    let mut sequences: Vec<u64> = log
        .entries
        .iter()
        .filter(|entry| entry.tick == 1)
        .map(|entry| entry.sequence)
        .collect();
    sequences.sort_unstable();
    assert_eq!(sequences, vec![0, 1, 2]);
}

/// A loaded log with an exhausted same-tick sequence must not wrap or duplicate it.
#[test]
fn record_refuses_sequence_overflow_without_appending_a_duplicate() {
    let mut log = IslandReplayLog::new("scenario".to_string());
    log.entries.push(IslandReplayEntry {
        tick: 1,
        sequence: u64::MAX,
        command: somebody("Rua", 1),
    });

    assert!(matches!(
        log.record(1, somebody("Toru", 1)),
        Err(IslandReplayError::SequenceExhausted { tick: 1 })
    ));
    assert_eq!(log.entries.len(), 1);
    assert_eq!(log.entries[0].sequence, u64::MAX);
}

/// Refuse oversized replay files before allocating or parsing their contents.
/// A sparse file exercises the size guard without consuming equivalent disk space.
#[test]
fn oversized_replay_logs_are_rejected_before_reading() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("oversized.log.json");
    let file = std::fs::File::create(&path).unwrap();
    file.set_len(MAX_REPLAY_LOG_BYTES + 1).unwrap();
    drop(file);

    assert!(matches!(
        IslandReplayLog::load(&path),
        Err(IslandReplayError::TooLarge { bytes, max_bytes, .. })
            if bytes == MAX_REPLAY_LOG_BYTES + 1 && max_bytes == MAX_REPLAY_LOG_BYTES
    ));
}

/// A log survives a trip through a file, order and all.
#[test]
fn a_log_round_trips_through_a_file() {
    let mut live = island();
    let space = live
        .placed
        .layout
        .spaces_outside_house()
        .next()
        .unwrap()
        .id
        .0;
    live.advance(STEP).unwrap();
    live.apply_command(somebody("Pare", space)).unwrap();
    live.apply_command(IslandCommand::Control(ControlCommand::Snapshot))
        .unwrap();

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("run.log.json");
    // A stale temp file from an earlier process must not be truncated or
    // mistaken for this save's own staging file.
    let stale_temp = path.with_extension(format!("tmp-{}", std::process::id()));
    let occupied_candidate = path.with_extension(format!("tmp-{}-0", std::process::id()));
    std::fs::write(&stale_temp, b"owned by another writer").unwrap();
    std::fs::write(&occupied_candidate, b"another save owns this candidate").unwrap();
    live.replay_log().save(&path).unwrap();
    assert_eq!(
        std::fs::read(&stale_temp).unwrap(),
        b"owned by another writer",
        "saving a replay must not reuse the legacy temp path"
    );
    assert_eq!(
        std::fs::read(&occupied_candidate).unwrap(),
        b"another save owns this candidate",
        "exclusive temp creation must skip an occupied candidate"
    );
    let back = IslandReplayLog::load(&path).expect("the log reads back");

    assert_eq!(back.entries.len(), live.replay_log().entries.len());
    assert_eq!(back.scenario_digest, live.replay_log().scenario_digest);
    let replayed = replay_island(canon(), scenario(), &back, live.tick).unwrap();
    assert_eq!(hex(replayed.state_digest()), hex(live.state_digest()));

    // Something that is not a log is refused by name.
    std::fs::write(&path, b"{\"hello\":true}").unwrap();
    assert!(matches!(
        IslandReplayLog::load(&path),
        Err(IslandReplayError::NotALog(..))
    ));
}

/// A log whose entries have been shuffled still reproduces the run.
///
/// A record that had been reordered in transit should reproduce the island
/// or refuse, never quietly produce a different one.
#[test]
fn a_shuffled_log_still_reproduces_the_run() {
    let mut live = island();
    let space = live
        .placed
        .layout
        .spaces_outside_house()
        .next()
        .unwrap()
        .id
        .0;
    live.advance(2 * STEP).unwrap();
    live.apply_command(somebody("Wiremu", space)).unwrap();
    live.apply_command(somebody("Wiremu", space)).unwrap();
    live.advance(3 * STEP).unwrap();
    live.apply_command(somebody("Hana", space)).unwrap();
    live.advance(STEP).unwrap();

    let mut shuffled = live.replay_log().clone();
    shuffled.entries.reverse();
    let replayed = replay_island(canon(), scenario(), &shuffled, live.tick).unwrap();
    assert_eq!(
        hex(replayed.state_digest()),
        hex(live.state_digest()),
        "a reordered log reproduced a different island"
    );
}

/// A serialized log must not contain two commands claiming the same order
/// within one tick. Guessing would make the replay depend on file ordering.
#[test]
fn replay_refuses_duplicate_command_sequences_at_same_tick() {
    let mut log = two_command_log();
    log.entries[1].sequence = 0;

    match replay_island(canon(), scenario(), &log, 0) {
        Err(IslandReplayError::InvalidSequence {
            tick,
            expected,
            actual,
        }) => assert_eq!((tick, expected, actual), (0, 1, 0)),
        Err(other) => panic!("the wrong refusal for a duplicate sequence: {other}"),
        Ok(_) => panic!("a log with duplicate command sequences was accepted"),
    }
}

/// A sequence gap means at least one command record is missing. The replay
/// must fail closed instead of presenting the remaining commands as a whole log.
#[test]
fn replay_refuses_gaps_in_command_sequences_at_same_tick() {
    let mut log = two_command_log();
    log.entries[1].sequence = 2;

    match replay_island(canon(), scenario(), &log, 0) {
        Err(IslandReplayError::InvalidSequence {
            tick,
            expected,
            actual,
        }) => assert_eq!((tick, expected, actual), (0, 1, 2)),
        Err(other) => panic!("the wrong refusal for a sequence gap: {other}"),
        Ok(_) => panic!("a log with a command sequence gap was accepted"),
    }
}

#[test]
fn load_refuses_duplicate_command_sequences_before_replay() {
    let mut log = two_command_log();
    log.entries[1].sequence = 0;

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("duplicate-sequence.log.json");
    std::fs::write(&path, serde_json::to_vec(&log).unwrap()).unwrap();

    assert!(matches!(
        IslandReplayLog::load(&path),
        Err(IslandReplayError::InvalidSequence {
            tick: 0,
            expected: 1,
            actual: 0
        })
    ));
}

#[test]
fn save_refuses_sequence_gaps_without_creating_a_file() {
    let mut log = two_command_log();
    log.entries[1].sequence = 2;

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("sequence-gap.log.json");
    assert!(matches!(
        log.save(&path),
        Err(IslandReplayError::InvalidSequence {
            tick: 0,
            expected: 1,
            actual: 2
        })
    ));
    assert!(!path.exists(), "invalid replay logs must not be persisted");
}

#[test]
fn save_and_load_accept_shuffled_entries_with_contiguous_sequences() {
    let mut log = two_command_log();
    log.entries.reverse();

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("shuffled-valid.log.json");
    log.save(&path).unwrap();
    let loaded = IslandReplayLog::load(&path).unwrap();

    assert_eq!(
        loaded
            .entries
            .iter()
            .map(|entry| entry.sequence)
            .collect::<Vec<_>>(),
        vec![1, 0],
        "validation must not rewrite the stored entry order"
    );
    let replayed = replay_island(canon(), scenario(), &loaded, 0).unwrap();
    let original = replay_island(canon(), scenario(), &log, 0).unwrap();
    assert_eq!(hex(replayed.state_digest()), hex(original.state_digest()));
}

#[test]
fn load_refuses_noncanonical_scenario_digest() {
    let mut log = two_command_log();
    log.scenario_digest = "not-a-digest".to_string();

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("invalid-scenario-digest.log.json");
    std::fs::write(&path, serde_json::to_vec(&log).unwrap()).unwrap();

    assert!(matches!(
        IslandReplayLog::load(&path),
        Err(IslandReplayError::InvalidScenarioDigest)
    ));
}

#[test]
fn save_refuses_noncanonical_scenario_digest_without_creating_a_file() {
    let mut log = two_command_log();
    log.scenario_digest = "not-a-digest".to_string();

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("invalid-scenario-digest.log.json");
    assert!(matches!(
        log.save(&path),
        Err(IslandReplayError::InvalidScenarioDigest)
    ));
    assert!(!path.exists(), "invalid replay logs must not be persisted");
}

#[test]
fn load_refuses_unsupported_log_versions() {
    let mut log = two_command_log();
    log.version = REPLAY_LOG_VERSION + 1;

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("unsupported-version.log.json");
    std::fs::write(&path, serde_json::to_vec(&log).unwrap()).unwrap();

    assert!(matches!(
        IslandReplayLog::load(&path),
        Err(IslandReplayError::Version(version)) if version == REPLAY_LOG_VERSION + 1
    ));
}

#[test]
fn save_refuses_unsupported_log_versions_without_creating_a_file() {
    let mut log = two_command_log();
    log.version = REPLAY_LOG_VERSION + 1;

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("unsupported-version.log.json");
    assert!(matches!(
        log.save(&path),
        Err(IslandReplayError::Version(version)) if version == REPLAY_LOG_VERSION + 1
    ));
    assert!(!path.exists(), "unsupported replay logs must not be persisted");
}

fn two_command_log() -> IslandReplayLog {
    let mut life = island();
    let space = life
        .placed
        .layout
        .spaces_outside_house()
        .next()
        .expect("a space outside the founders' house exists")
        .id
        .0;
    life.apply_command(somebody("Rua", space)).unwrap();
    life.apply_command(somebody("Toru", space)).unwrap();
    life.replay_log().clone()
}

/// A writer must not create a replay file that its bounded reader would reject.
#[test]
fn save_refuses_serialized_replay_over_the_size_limit() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("oversized-save.log.json");
    let log = IslandReplayLog::new("x".repeat(MAX_REPLAY_LOG_BYTES as usize + 1));

    assert!(matches!(
        log.save(&path),
        Err(IslandReplayError::TooLarge { max_bytes, .. }) if max_bytes == MAX_REPLAY_LOG_BYTES
    ));
    assert!(
        !path.exists(),
        "an oversized replay must not leave a partial file"
    );
}

/// A replay path must name the private file itself, not a symlink to another file.
#[cfg(unix)]
#[test]
fn load_refuses_symlinked_replay_paths() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("actual-run.log.json");
    let link = dir.path().join("linked-run.log.json");
    let log = IslandReplayLog::new("scenario".to_string());
    std::fs::write(&target, serde_json::to_vec(&log).unwrap()).unwrap();
    std::os::unix::fs::symlink(&target, &link).unwrap();

    assert!(matches!(
        IslandReplayLog::load(&link),
        Err(IslandReplayError::NotALog(_, reason)) if reason.contains("regular file")
    ));
}

/// A FIFO is not a replay file. Loading it must refuse the special file rather than
/// treating a stream with no stable length as a bounded replay document.
#[cfg(unix)]
#[test]
fn load_refuses_fifo_replay_sources() {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;

    let dir = tempfile::tempdir().unwrap();
    let fifo = dir.path().join("replay.fifo");
    let c_path = CString::new(fifo.as_os_str().as_bytes()).unwrap();
    let created = unsafe { libc::mkfifo(c_path.as_ptr(), 0o600) };
    assert_eq!(created, 0, "the FIFO fixture must be created");

    assert!(matches!(
        IslandReplayLog::load(&fifo),
        Err(IslandReplayError::NotALog(_, reason)) if reason.contains("regular file")
    ));
}

/// Replay logs contain personal details and must not be readable by other local users.
#[cfg(unix)]
#[test]
fn saved_replay_files_are_owner_only() {
    use std::os::unix::fs::PermissionsExt;

    let log = IslandReplayLog::new(island().scenario_digest_hex());
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("private-run.log.json");
    log.save(&path).unwrap();

    let mode = std::fs::metadata(&path).unwrap().permissions().mode();
    assert_eq!(
        mode & 0o077,
        0,
        "replay files must not grant group/world access"
    );
    assert_ne!(mode & 0o400, 0, "the owner must be able to read the replay");
}
