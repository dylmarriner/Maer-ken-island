//! Deterministic replay of an island run from its command log (Phase 4
//! Task 4). The same scenario, seed and log give the same world.

use std::sync::Arc;

use mk_core::canon::CanonLocked;
use mk_island::IslandScenario;

use super::commands::{
    CommandError, IslandCommand, IslandReplayEntry, IslandReplayLog, QueuedCommandId,
};
use super::world::{IslandWorldError, IslandWorldState};

#[derive(Debug)]
pub enum IslandReplayError {
    /// The log was recorded on a different scenario.
    ScenarioMismatch,
    World(IslandWorldError),
}

impl std::fmt::Display for IslandReplayError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for IslandReplayError {}

pub fn scenario_digest(scenario: &IslandScenario) -> [u8; 32] {
    *blake3::hash(&serde_json::to_vec(scenario).expect("scenario serialises")).as_bytes()
}

impl IslandWorldState {
    /// Queue an external command for the next human substep.
    pub fn queue(&mut self, command: IslandCommand) -> Result<QueuedCommandId, CommandError> {
        self.life.queue_command(command)
    }

    /// The log of every command applied so far.
    pub fn replay_log(&self) -> IslandReplayLog {
        IslandReplayLog {
            scenario_digest: scenario_digest(&self.life.scenario),
            entries: self.life.commands.log.clone(),
        }
    }
}

/// Rebuild the world from the scenario and run it to `until_tick`, applying
/// the log's commands at their recorded ticks and sequences. A run whose
/// log pauses it for good stops where it paused.
pub fn replay_island(
    canon: Arc<CanonLocked>,
    scenario: IslandScenario,
    log: &IslandReplayLog,
    until_tick: u64,
) -> Result<IslandWorldState, IslandReplayError> {
    if log.scenario_digest != scenario_digest(&scenario) {
        return Err(IslandReplayError::ScenarioMismatch);
    }
    let human_s = scenario.cadences.human_seconds;
    let mut world = IslandWorldState::new(canon, scenario).map_err(IslandReplayError::World)?;
    let mut entries: Vec<IslandReplayEntry> = log.entries.clone();
    entries.sort_by_key(|e| (e.tick, e.sequence));
    let mut next = 0;
    while world.tick() < until_tick {
        let upcoming = world.tick() + 1;
        while next < entries.len() && entries[next].tick <= upcoming {
            world.life.queue_recorded(entries[next].clone());
            next += 1;
        }
        let before = world.tick();
        world.step(human_s).map_err(IslandReplayError::World)?;
        if world.tick() == before && world.life.commands.paused && next >= entries.len() {
            break; // paused for good
        }
    }
    Ok(world)
}
