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
    replay_island, IslandReplayEntry, IslandReplayError, IslandReplayLog, ReplaySequenceExhausted,
    MAX_REPLAY_LOG_BYTES, REPLAY_LOG_VERSION,
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
    assert_eq!(log.entries().len(), 4);
    let order: Vec<(u64, u64)> = log.entries().iter().map(|e| (e.tick, e.sequence)).collect();
    assert_eq!(order, vec![(5, 0), (5, 1), (5, 2), (6, 0)]);
    assert!(!log.scenario_digest.is_empty(), "the log names no scenario");
}

/// The live command path keeps same-tick sequencing correct with a long retained log.
#[test]
fn a_long_same_tick_command_run_keeps_sequences_monotonic() {
    let mut life = island();
    for _ in 0..4_096 {
        life.apply_command(IslandCommand::Control(ControlCommand::Pause))
            .expect("control command applies");
    }

    let entries = life.replay_log().entries();
    assert_eq!(entries.len(), 4_096);
    assert_eq!(entries.first().map(|entry| entry.sequence), Some(0));
    assert_eq!(entries.last().map(|entry| entry.sequence), Some(4_095));
    assert!(entries
        .iter()
        .enumerate()
        .all(|(index, entry)| entry.sequence == index as u64));
}

/// A refused command is not part of the log and must not consume its sequence.
#[test]
fn a_refused_command_does_not_consume_the_next_replay_sequence() {
    let mut life = island();
    let space = life
        .placed
        .layout
        .spaces_outside_house()
        .next()
        .unwrap()
        .id
        .0;
    let population_before = life.humans.registry.iter().count();
    let mut invalid = match somebody("placeholder", space) {
        IslandCommand::CreateHuman(request) => *request,
        _ => unreachable!("somebody creates a person"),
    };
    invalid.name = "   ".to_string();

    assert!(
        life.apply_command(IslandCommand::CreateHuman(Box::new(invalid)))
            .is_err(),
        "an unnamed person must be refused"
    );
    assert_eq!(
        life.humans.registry.iter().count(),
        population_before,
        "a refused request must not create a person"
    );

    life.apply_command(IslandCommand::Control(ControlCommand::Pause))
        .expect("the following control command applies");

    let entries = life.replay_log().entries();
    assert_eq!(entries.len(), 1, "the refused request must not be logged");
    assert_eq!(entries[0].tick, life.tick);
    assert_eq!(
        entries[0].sequence, 0,
        "the first accepted command keeps sequence zero"
    );
}

/// Replaying a sparse log preserves its original sequence identifiers.
#[test]
fn replay_preserves_sparse_recorded_sequence_numbers() {
    let log = IslandReplayLog::from_entries(
        REPLAY_LOG_VERSION,
        island().scenario_digest_hex(),
        vec![
            IslandReplayEntry {
                tick: 0,
                sequence: 12,
                command: IslandCommand::Control(ControlCommand::Pause),
            },
            IslandReplayEntry {
                tick: 0,
                sequence: 7,
                command: IslandCommand::Control(ControlCommand::Resume),
            },
        ],
    );

    let replayed = replay_island(canon(), scenario(), &log, 0)
        .expect("sparse, strictly increasing sequence identifiers replay");

    let positions: Vec<(u64, u64)> = replayed
        .replay_log()
        .entries()
        .iter()
        .map(|entry| (entry.tick, entry.sequence))
        .collect();
    assert_eq!(positions, vec![(0, 7), (0, 12)]);
}

/// Appending to a log after an out-of-order tick must not reuse a sequence.
#[test]
fn recording_after_an_out_of_order_tick_uses_the_next_sequence_for_that_tick() {
    let mut log = IslandReplayLog::from_entries(
        REPLAY_LOG_VERSION,
        "scenario".to_string(),
        vec![
            IslandReplayEntry {
                tick: 5,
                sequence: 0,
                command: IslandCommand::Control(ControlCommand::Pause),
            },
            IslandReplayEntry {
                tick: 6,
                sequence: 0,
                command: IslandCommand::Control(ControlCommand::Pause),
            },
            IslandReplayEntry {
                tick: 5,
                sequence: 1,
                command: IslandCommand::Control(ControlCommand::Resume),
            },
        ],
    );
    log.record(5, IslandCommand::Control(ControlCommand::Pause))
        .expect("the imported per-tick maximum is indexed");

    let mut sequences: Vec<u64> = log
        .entries()
        .iter()
        .filter(|entry| entry.tick == 5)
        .map(|entry| entry.sequence)
        .collect();
    sequences.sort_unstable();
    assert_eq!(sequences, vec![0, 1, 2]);
}

/// A deserialised append-only log rebuilds its runtime index without changing its JSON shape.
#[test]
fn deserialised_log_rebuilds_the_per_tick_sequence_index_before_recording() {
    let source = IslandReplayLog::from_entries(
        REPLAY_LOG_VERSION,
        "scenario".to_string(),
        vec![
            IslandReplayEntry {
                tick: 5,
                sequence: 0,
                command: IslandCommand::Control(ControlCommand::Pause),
            },
            IslandReplayEntry {
                tick: 6,
                sequence: 0,
                command: IslandCommand::Control(ControlCommand::Pause),
            },
            IslandReplayEntry {
                tick: 5,
                sequence: 4,
                command: IslandCommand::Control(ControlCommand::Resume),
            },
        ],
    );
    let json = serde_json::to_string(&source).expect("the replay log serialises");
    let mut loaded: IslandReplayLog =
        serde_json::from_str(&json).expect("the replay log deserialises");

    loaded
        .record(5, IslandCommand::Control(ControlCommand::Pause))
        .expect("deserialisation rebuilds the skipped index on first append");

    let sequences: Vec<u64> = loaded
        .entries()
        .iter()
        .filter(|entry| entry.tick == 5)
        .map(|entry| entry.sequence)
        .collect();
    assert_eq!(sequences, vec![0, 4, 5]);
    assert!(
        !json.contains("sequence_max_by_tick"),
        "the runtime index must not change the persisted replay format"
    );
}

/// A log at the maximum sequence must fail rather than append an ambiguous duplicate.
#[test]
fn recording_after_sequence_exhaustion_fails_without_appending() {
    let mut log = IslandReplayLog::from_entries(
        REPLAY_LOG_VERSION,
        "scenario".to_string(),
        vec![IslandReplayEntry {
            tick: 5,
            sequence: u64::MAX,
            command: IslandCommand::Control(ControlCommand::Pause),
        }],
    );

    assert_eq!(
        log.record(5, IslandCommand::Control(ControlCommand::Resume)),
        Err(ReplaySequenceExhausted { tick: 5 })
    );
    assert_eq!(log.entries().len(), 1);
    assert_eq!(log.entries()[0].sequence, u64::MAX);
}

/// The final representable same-tick sequence is valid exactly once; the next append is refused.
#[test]
fn recording_the_last_available_sequence_then_exhaustion_is_fail_closed() {
    let mut log = IslandReplayLog::from_entries(
        REPLAY_LOG_VERSION,
        "scenario".to_string(),
        vec![IslandReplayEntry {
            tick: 5,
            sequence: u64::MAX - 1,
            command: IslandCommand::Control(ControlCommand::Pause),
        }],
    );

    log.record(5, IslandCommand::Control(ControlCommand::Resume))
        .expect("u64::MAX is the final available sequence");
    assert_eq!(log.entries().len(), 2);
    assert_eq!(log.entries()[1].sequence, u64::MAX);

    assert_eq!(
        log.record(5, IslandCommand::Control(ControlCommand::Pause)),
        Err(ReplaySequenceExhausted { tick: 5 })
    );
    assert_eq!(log.entries().len(), 2, "exhaustion must not append a duplicate");
}

/// Duplicate ordering positions make same-tick replay ambiguous and must fail closed.
#[test]
fn replay_refuses_duplicate_tick_sequence_positions() {
    let log = IslandReplayLog::from_entries(
        REPLAY_LOG_VERSION,
        "not-this-scenario".to_string(),
        vec![
            IslandReplayEntry {
                tick: 5,
                sequence: 0,
                command: IslandCommand::Control(ControlCommand::Pause),
            },
            IslandReplayEntry {
                tick: 5,
                sequence: 0,
                command: IslandCommand::Control(ControlCommand::Resume),
            },
        ],
    );

    assert!(matches!(
        replay_island(canon(), scenario(), &log, 5),
        Err(IslandReplayError::DuplicateSequence {
            tick: 5,
            sequence: 0
        })
    ));
}

/// A partial replay validates only commands within its requested horizon.
#[test]
fn partial_replay_ignores_duplicate_sequences_after_its_horizon() {
    let scenario_digest = island().scenario_digest_hex();
    let log = IslandReplayLog::from_entries(
        REPLAY_LOG_VERSION,
        scenario_digest,
        vec![
            IslandReplayEntry {
                tick: 6,
                sequence: 0,
                command: IslandCommand::Control(ControlCommand::Pause),
            },
            IslandReplayEntry {
                tick: 6,
                sequence: 0,
                command: IslandCommand::Control(ControlCommand::Resume),
            },
        ],
    );

    let replayed = replay_island(canon(), scenario(), &log, 5)
        .expect("duplicates beyond the requested replay horizon are left alone");
    assert_eq!(replayed.tick, 5);
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
    assert_eq!(with.replay_log().entries().len(), 3);
    assert_eq!(without.replay_log().entries().len(), 0);
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

/// In-memory logs bypass the file loader, so replay must enforce the same format gate.
#[test]
fn an_in_memory_log_with_an_unsupported_version_is_refused_before_replay() {
    let mut log = IslandReplayLog::new("not-a-current-scenario".to_string());
    log.version = REPLAY_LOG_VERSION + 1;

    let result = replay_island(canon(), scenario(), &log, 0);
    assert!(matches!(
        result,
        Err(IslandReplayError::Version(version)) if version == REPLAY_LOG_VERSION + 1
    ));
}

/// A log written against another scenario is refused, not reproduced
/// wrongly.
#[test]
fn a_log_from_another_island_is_refused() {
    let mut live = island();
    live.advance(STEP).unwrap();
    let mut log = live.replay_log().clone();
    log.scenario_digest = "not this island".to_string();

    match replay_island(canon(), scenario(), &log, 2) {
        Err(IslandReplayError::WrongScenario { log, .. }) => {
            assert_eq!(log, "not this island");
        }
        Err(other) => panic!("the wrong refusal for a foreign log: {other}"),
        Ok(_) => panic!("a foreign log was accepted"),
    }
}

/// An oversized replay file is refused from its metadata before its contents are read.
#[test]
fn a_replay_file_over_the_offline_size_limit_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("oversized-replay.json");
    let file = std::fs::File::create(&path).unwrap();
    file.set_len(MAX_REPLAY_LOG_BYTES + 1).unwrap();
    drop(file);

    assert!(matches!(
        IslandReplayLog::load(&path),
        Err(IslandReplayError::TooLarge { limit_bytes, .. })
            if limit_bytes == MAX_REPLAY_LOG_BYTES
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
    live.replay_log().save(&path).unwrap();
    let back = IslandReplayLog::load(&path).expect("the log reads back");

    assert_eq!(back.entries().len(), live.replay_log().entries().len());
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

/// Saving never overwrites a predictable temporary path reserved by another writer.
#[test]
fn saving_uses_a_fresh_temporary_path_without_clobbering_existing_files() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("run.log.json");
    let reserved_temp = path.with_extension(format!("tmp-{}-0", std::process::id()));
    std::fs::write(&reserved_temp, b"reserved by another writer").unwrap();

    let mut log = IslandReplayLog::new(island().scenario_digest_hex());
    log.record(0, IslandCommand::Control(ControlCommand::Pause))
        .expect("sequence available");

    log.save(&path)
        .expect("save should choose the next unused temporary path");

    assert_eq!(
        std::fs::read(&reserved_temp).unwrap(),
        b"reserved by another writer",
        "a pre-existing temporary path must never be truncated or followed"
    );
    let loaded = IslandReplayLog::load(&path).expect("the saved log reads back");
    assert_eq!(loaded.entries().len(), 1);
    assert_eq!(loaded.entries()[0].sequence, 0);
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

    let mut shuffled_entries = live.replay_log().entries().to_vec();
    shuffled_entries.reverse();
    let shuffled = IslandReplayLog::from_entries(
        live.replay_log().version,
        live.replay_log().scenario_digest.clone(),
        shuffled_entries,
    );
    let replayed = replay_island(canon(), scenario(), &shuffled, live.tick).unwrap();
    assert_eq!(
        hex(replayed.state_digest()),
        hex(live.state_digest()),
        "a reordered log reproduced a different island"
    );
}
