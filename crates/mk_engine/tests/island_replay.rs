//! Phase 4 Task 4: external commands and deterministic replay.

use std::path::PathBuf;
use std::sync::Arc;

use mk_core::canon::CanonLocked;
use mk_core::human::BiologicalSex;
use mk_engine::io::island_snapshot::{load_island_snapshot, save_island_snapshot};
use mk_engine::regional::commands::{
    CommandError, ControlCommand, IslandCommand, IslandCreateHumanRequest, SimSpeed,
};
use mk_engine::regional::replay::{replay_island, IslandReplayError};
use mk_engine::regional::world::IslandWorldState;
use mk_interventions::InterventionAction;
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

fn newcomer(name: &str) -> IslandCommand {
    IslandCommand::CreateHuman(IslandCreateHumanRequest {
        agent_id: name.into(),
        sex: BiologicalSex::Female,
        age_years: 28.0,
        appearance: None,
        location: None,
    })
}

fn part1(w: &mut IslandWorldState) {
    w.queue(newcomer("Visitor-1")).unwrap();
    for _ in 0..4 {
        w.step(60).unwrap();
    }
}

fn part2(w: &mut IslandWorldState) {
    w.queue(IslandCommand::Control(ControlCommand::Pause))
        .unwrap();
    w.step(60).unwrap(); // paused before its substep: no time passes
    w.queue(IslandCommand::Intervention(InterventionAction::Step {
        ticks: 2,
    }))
    .unwrap();
    w.step(60).unwrap(); // two stepped substeps
    w.queue(IslandCommand::Control(ControlCommand::Resume))
        .unwrap();
    w.step(60).unwrap(); // resumes; no time yet
    w.step(120).unwrap();
}

#[test]
fn commands_apply_in_tick_then_sequence_order_and_bad_ones_are_refused() {
    let mut w = world();
    w.queue(IslandCommand::Control(ControlCommand::SetSpeed(
        SimSpeed::Times(60),
    )))
    .unwrap();
    w.queue(newcomer("Visitor-1")).unwrap();
    w.queue(IslandCommand::Control(ControlCommand::Snapshot))
        .unwrap();
    w.step(120).unwrap();
    w.queue(newcomer("Visitor-2")).unwrap();
    w.queue(IslandCommand::Control(ControlCommand::SetSpeed(
        SimSpeed::AsFastAsPossible,
    )))
    .unwrap();
    w.step(60).unwrap();

    let log = w.replay_log();
    let order: Vec<(u64, u64)> = log.entries.iter().map(|e| (e.tick, e.sequence)).collect();
    assert_eq!(order, vec![(1, 0), (1, 1), (1, 2), (3, 3), (3, 4)]);
    let mut sorted = order.clone();
    sorted.sort();
    assert_eq!(order, sorted, "applied out of (tick, sequence) order");
    assert_eq!(
        w.life.humans.registry.iter().count(),
        4,
        "two founders and two visitors"
    );
    assert!(w.life.positions.0.contains_key("Visitor-2"));
    assert_eq!(w.life.commands.snapshots_requested, 1);
    assert_eq!(w.life.commands.speed, SimSpeed::AsFastAsPossible);

    // Refusals happen at queue time and change nothing.
    let pending = w.life.commands.pending.len();
    assert!(matches!(
        w.queue(newcomer("Visitor-1")),
        Err(CommandError::DuplicateHuman(_))
    ));
    assert!(matches!(
        w.queue(IslandCommand::Intervention(InterventionAction::Scrub {
            target_tick: 5
        })),
        Err(CommandError::NotSupportedOnIsland(_))
    ));
    assert!(matches!(
        w.queue(IslandCommand::Control(ControlCommand::SetSpeed(
            SimSpeed::Times(7)
        ))),
        Err(CommandError::InvalidRequest(_))
    ));
    assert!(matches!(
        w.queue(IslandCommand::Control(ControlCommand::Step(0))),
        Err(CommandError::InvalidRequest(_))
    ));
    assert_eq!(w.life.commands.pending.len(), pending);
}

#[test]
fn replaying_the_log_from_the_same_seed_gives_the_same_world() {
    let mut original = world();
    part1(&mut original);
    part2(&mut original);
    assert_eq!(
        original.tick(),
        8,
        "pause and resume cost no time; the step cost two substeps"
    );
    let log = original.replay_log();
    assert_eq!(log.entries.len(), 4);

    let replayed = replay_island(canon(), scenario(), &log, original.tick()).unwrap();
    assert_eq!(replayed.tick(), original.tick());
    assert_eq!(hex(&replayed), hex(&original));

    // A log from another scenario is refused.
    let mut other = scenario();
    other.cadences.human_seconds = 120;
    other.cadences.weather_ocean_seconds = 3_600;
    assert!(matches!(
        replay_island(canon(), other, &log, 1),
        Err(IslandReplayError::ScenarioMismatch)
    ));
}

#[test]
fn saving_halfway_loading_and_continuing_gives_the_same_final_world() {
    let dir = tempfile::tempdir().unwrap();
    let mut full = world();
    part1(&mut full);
    part2(&mut full);

    let mut half = world();
    part1(&mut half);
    let path = dir.path().join("half.snap");
    save_island_snapshot(&half, &path).unwrap();
    let mut resumed = load_island_snapshot(&path).unwrap();
    assert_eq!(hex(&resumed), hex(&half));
    part2(&mut resumed);
    assert_eq!(hex(&resumed), hex(&full), "the resumed run diverged");

    // Its log is the same log, so replaying it gives the same world too.
    let replayed =
        replay_island(canon(), scenario(), &resumed.replay_log(), resumed.tick()).unwrap();
    assert_eq!(hex(&replayed), hex(&full));
}

#[test]
fn a_control_applies_without_waiting_for_a_substep_and_replay_still_matches() {
    let mut w = world();
    w.step(60).unwrap();
    w.queue(IslandCommand::Control(ControlCommand::Pause))
        .unwrap();
    w.step(0).unwrap(); // no time to run, yet the pause lands at once
    assert!(
        w.life.commands.paused,
        "a pause must not wait for the next substep"
    );
    assert_eq!(w.tick(), 1);
    w.queue(IslandCommand::Control(ControlCommand::Resume))
        .unwrap();
    w.step(0).unwrap();
    assert!(!w.life.commands.paused);
    w.step(60).unwrap();
    assert_eq!(w.tick(), 2);
    let log = w.replay_log();
    let replayed = replay_island(canon(), scenario(), &log, w.tick()).unwrap();
    assert_eq!(
        hex(&replayed),
        hex(&w),
        "applying early is the same as at the next substep"
    );
}
