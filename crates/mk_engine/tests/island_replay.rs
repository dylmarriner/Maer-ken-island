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
use mk_engine::regional::replay::{replay_island, IslandReplayError, IslandReplayLog};
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

    let space = life.placed.layout.spaces[0].id.0;
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
    let space = live.placed.layout.spaces[0].id.0;

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
    let space = live.placed.layout.spaces[0].id.0;
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
        3,
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
    log.scenario_digest = "not this island".to_string();

    match replay_island(canon(), scenario(), &log, 2) {
        Err(IslandReplayError::WrongScenario { log, .. }) => {
            assert_eq!(log, "not this island");
        }
        Err(other) => panic!("the wrong refusal for a foreign log: {other}"),
        Ok(_) => panic!("a foreign log was accepted"),
    }
}

/// A log survives a trip through a file, order and all.
#[test]
fn a_log_round_trips_through_a_file() {
    let mut live = island();
    let space = live.placed.layout.spaces[0].id.0;
    live.advance(STEP).unwrap();
    live.apply_command(somebody("Pare", space)).unwrap();
    live.apply_command(IslandCommand::Control(ControlCommand::Snapshot))
        .unwrap();

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("run.log.json");
    live.replay_log().save(&path).unwrap();
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
    let space = live.placed.layout.spaces[0].id.0;
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
