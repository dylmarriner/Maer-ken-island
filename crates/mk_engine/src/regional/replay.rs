//! Phase 4 Task 4: running an island's history again and arriving where it
//! arrived.
//!
//! An island's state follows from three things and nothing else: the
//! scenario it started from, the number of steps it has taken, and the
//! commands that reached it from outside. Record the third and all three
//! are in hand — so the same scenario, the same log and the same tick must
//! reach the same canonical digest as the original run.
//!
//! That is what makes the digest worth printing. A run that could not be
//! reproduced would be a number about one afternoon on one machine.

use std::path::Path;
use std::sync::Arc;

use mk_core::canon::CanonLocked;
use mk_island::IslandScenario;
use serde::{Deserialize, Serialize};

use super::commands::{Applied, CommandError, ControlCommand, IslandCommand};
use super::life::{IslandLife, IslandLifeError};

/// The log's format. A log written by another version is refused rather
/// than guessed at, for the same reason a snapshot is.
pub const REPLAY_LOG_VERSION: u32 = 1;

/// One command, and exactly when it was applied.
///
/// `sequence` orders commands that landed on the same tick. Without it a
/// log would say two creations happened at tick 40 and not which came
/// first, and they would come back as different people — the agent id of
/// the second depends on the first already being there.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IslandReplayEntry {
    pub tick: u64,
    pub sequence: u64,
    pub command: IslandCommand,
}

/// Everything that reached an island from outside, in order.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IslandReplayLog {
    pub version: u32,
    /// The scenario this log belongs to. Replaying it against a different
    /// island would reproduce nothing, so the log says which one it is.
    pub scenario_digest: String,
    pub entries: Vec<IslandReplayEntry>,
}

impl IslandReplayLog {
    /// An empty log for a scenario.
    pub fn new(scenario_digest: String) -> Self {
        Self {
            version: REPLAY_LOG_VERSION,
            scenario_digest,
            entries: Vec::new(),
        }
    }

    /// Append a command applied at `tick`.
    pub fn record(&mut self, tick: u64, command: IslandCommand) {
        let sequence = self
            .entries
            .iter()
            .rev()
            .take_while(|e| e.tick == tick)
            .count() as u64;
        self.entries.push(IslandReplayEntry {
            tick,
            sequence,
            command,
        });
    }

    /// Read a log from a file.
    pub fn load(path: &Path) -> Result<Self, IslandReplayError> {
        let bytes = std::fs::read(path)
            .map_err(|e| IslandReplayError::Io(path.display().to_string(), e))?;
        let log: Self = serde_json::from_slice(&bytes)
            .map_err(|e| IslandReplayError::NotALog(path.display().to_string(), e.to_string()))?;
        if log.version != REPLAY_LOG_VERSION {
            return Err(IslandReplayError::Version(log.version));
        }
        Ok(log)
    }

    /// Write a log, whole or not at all.
    pub fn save(&self, path: &Path) -> Result<(), IslandReplayError> {
        let text = serde_json::to_vec_pretty(self)
            .map_err(|e| IslandReplayError::NotALog(path.display().to_string(), e.to_string()))?;
        let temp = path.with_extension(format!("tmp-{}", std::process::id()));
        let write = (|| -> std::io::Result<()> {
            std::fs::write(&temp, &text)?;
            std::fs::rename(&temp, path)
        })();
        if let Err(e) = write {
            std::fs::remove_file(&temp).ok();
            return Err(IslandReplayError::Io(path.display().to_string(), e));
        }
        Ok(())
    }
}

/// Why a replay did not happen, or did not finish.
#[derive(Debug)]
pub enum IslandReplayError {
    Io(String, std::io::Error),
    NotALog(String, String),
    /// A log written by a format this build does not know.
    Version(u32),
    /// The log belongs to a different scenario. Replaying it here would
    /// reproduce nothing and look like it had.
    WrongScenario {
        log: String,
        scenario: String,
    },
    /// The island would not start, or would not step.
    Island(IslandLifeError),
    /// A command the original run applied was refused on the replay. The
    /// run cannot be reproduced, and saying so is the only honest answer.
    Refused {
        tick: u64,
        sequence: u64,
        why: String,
    },
}

impl std::fmt::Display for IslandReplayError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(path, e) => write!(f, "{path}: {e}"),
            Self::NotALog(path, why) => write!(f, "{path} is not a replay log: {why}"),
            Self::Version(v) => write!(
                f,
                "that log is format {v}; this build reads {REPLAY_LOG_VERSION}"
            ),
            Self::WrongScenario { log, scenario } => write!(
                f,
                "that log was recorded against scenario {log}, and this is {scenario}"
            ),
            Self::Island(e) => write!(f, "{e}"),
            Self::Refused {
                tick,
                sequence,
                why,
            } => write!(
                f,
                "the command at tick {tick} ({sequence}) was accepted on the original run and \
                 refused on this one, so the run cannot be reproduced: {why}"
            ),
        }
    }
}

impl std::error::Error for IslandReplayError {}

/// Run a scenario forward to `until_tick`, applying the log as it goes.
///
/// Commands are applied at the start of the tick they were recorded at, in
/// `sequence` order, which is where the original run applied them. Anything
/// in the log past `until_tick` is left alone, so a replay can stop
/// anywhere.
pub fn replay_island(
    canon: Arc<CanonLocked>,
    scenario: IslandScenario,
    log: &IslandReplayLog,
    until_tick: u64,
) -> Result<IslandLife, IslandReplayError> {
    let mut life = IslandLife::bootstrap(scenario, canon).map_err(IslandReplayError::Island)?;

    let here = life.scenario_digest_hex();
    if here != log.scenario_digest {
        return Err(IslandReplayError::WrongScenario {
            log: log.scenario_digest.clone(),
            scenario: here,
        });
    }

    // In (tick, sequence) order, whatever order the file happens to be in:
    // a log is a record, and a record that had been reordered should still
    // reproduce the run rather than quietly produce a different one.
    let mut entries: Vec<&IslandReplayEntry> = log
        .entries
        .iter()
        .filter(|e| e.tick <= until_tick)
        .collect();
    entries.sort_by_key(|e| (e.tick, e.sequence));

    let step = life.scenario.cadences.human_seconds;
    for entry in entries {
        while life.tick < entry.tick {
            life.advance(step).map_err(IslandReplayError::Island)?;
        }
        life.apply_command(entry.command.clone())
            .map_err(|e| IslandReplayError::Refused {
                tick: entry.tick,
                sequence: entry.sequence,
                why: e.to_string(),
            })?;
    }
    while life.tick < until_tick {
        life.advance(step).map_err(IslandReplayError::Island)?;
    }
    Ok(life)
}

impl IslandLife {
    /// Do what was asked, and remember that it was asked.
    ///
    /// The log is appended before the command's effect is returned, so a
    /// command that applied is always in the record — a run whose log were
    /// missing its last entry would replay to a different island and give
    /// no sign of it.
    pub fn apply_command(&mut self, command: IslandCommand) -> Result<Applied, CommandError> {
        match &command {
            IslandCommand::CreateHuman(request) => {
                let made = self
                    .create_human((**request).clone())
                    .map_err(CommandError::Create)?;
                self.replay_log.record(self.tick, command);
                Ok(Applied::Created(made))
            }
            IslandCommand::Intervention(action) => {
                // Applied before it is recorded, because an intervention
                // the island refuses never happened and must not be in a
                // log that claims to reproduce the run.
                let outcome = self.intervene(action).map_err(CommandError::Intervention)?;
                self.replay_log.record(self.tick, command);
                Ok(Applied::Intervened(outcome))
            }
            IslandCommand::Control(control) => {
                // Nothing in the world moves. It is recorded because a
                // replay should be able to say what the operator did.
                let _: &ControlCommand = control;
                self.replay_log.record(self.tick, command);
                Ok(Applied::Noted)
            }
        }
    }

    /// Everything that has reached this island from outside.
    pub fn replay_log(&self) -> &IslandReplayLog {
        &self.replay_log
    }

    /// The scenario's digest, as the log records it.
    pub fn scenario_digest_hex(&self) -> String {
        self.scenario_digest()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect()
    }
}
