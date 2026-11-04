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

use std::fs::{self, OpenOptions};
use std::io::{self, Read, Write};
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

/// Maximum serialized replay-log size accepted from disk (128 MiB). This
/// bounds memory use when loading corrupt or untrusted replay paths.
pub const MAX_REPLAY_LOG_BYTES: u64 = 128 * 1024 * 1024;

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
        // Entries may be appended out of tick order before replay sorts them.
        // Continue the last sequence for this tick, not merely the contiguous
        // suffix, or an interleaved tick would create duplicate sequence IDs.
        let sequence = self
            .entries
            .iter()
            .filter(|entry| entry.tick == tick)
            .map(|entry| entry.sequence)
            .max()
            .map_or(0, |sequence| sequence.saturating_add(1));
        self.entries.push(IslandReplayEntry {
            tick,
            sequence,
            command,
        });
    }

    /// Read a log from a file.
    pub fn load(path: &Path) -> Result<Self, IslandReplayError> {
        let file = fs::File::open(path)
            .map_err(|e| IslandReplayError::Io(path.display().to_string(), e))?;
        let size = file
            .metadata()
            .map_err(|e| IslandReplayError::Io(path.display().to_string(), e))?
            .len();
        if size > MAX_REPLAY_LOG_BYTES {
            return Err(IslandReplayError::TooLarge {
                path: path.display().to_string(),
                bytes: size,
                max_bytes: MAX_REPLAY_LOG_BYTES,
            });
        }

        // The metadata check gives a fast refusal for known-large files. The
        // bounded read also protects against a file growing after that check.
        let mut bytes = Vec::with_capacity(size as usize);
        let mut bounded = file.take(MAX_REPLAY_LOG_BYTES + 1);
        let read = bounded
            .read_to_end(&mut bytes)
            .map_err(|e| IslandReplayError::Io(path.display().to_string(), e))?;
        if read as u64 > MAX_REPLAY_LOG_BYTES {
            return Err(IslandReplayError::TooLarge {
                path: path.display().to_string(),
                bytes: read as u64,
                max_bytes: MAX_REPLAY_LOG_BYTES,
            });
        }

        let log: Self = serde_json::from_slice(&bytes)
            .map_err(|e| IslandReplayError::NotALog(path.display().to_string(), e.to_string()))?;
        if log.version != REPLAY_LOG_VERSION {
            return Err(IslandReplayError::Version(log.version));
        }
        Ok(log)
    }

    /// Write a log atomically, using a unique temporary file for each writer.
    ///
    /// Exclusive file creation matters even with a single simulation writer: a
    /// stale temp file or concurrent save must never be truncated before rename.
    pub fn save(&self, path: &Path) -> Result<(), IslandReplayError> {
        let text = serde_json::to_vec_pretty(self)
            .map_err(|e| IslandReplayError::NotALog(path.display().to_string(), e.to_string()))?;

        let mut candidate = None;
        for sequence in 0..128 {
            let temp = path.with_extension(format!("tmp-{}-{sequence}", std::process::id()));
            let mut options = OpenOptions::new();
            options.write(true).create_new(true);
            // Replay logs contain names, birth timestamps and physical details.
            // Do not expose those records to other local users by default.
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            match options.open(&temp) {
                Ok(file) => {
                    candidate = Some((temp, file));
                    break;
                }
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(IslandReplayError::Io(path.display().to_string(), error)),
            }
        }

        let Some((temp, mut file)) = candidate else {
            return Err(IslandReplayError::Io(
                path.display().to_string(),
                io::Error::new(
                    io::ErrorKind::AlreadyExists,
                    "could not allocate a unique temporary replay file",
                ),
            ));
        };

        if let Err(error) = file.write_all(&text).and_then(|()| file.sync_all()) {
            drop(file);
            let _ = fs::remove_file(&temp);
            return Err(IslandReplayError::Io(path.display().to_string(), error));
        }
        drop(file);

        if let Err(error) = fs::rename(&temp, path) {
            let _ = fs::remove_file(&temp);
            return Err(IslandReplayError::Io(path.display().to_string(), error));
        }

        // The file's contents are synced before rename. On Unix, sync the
        // containing directory as well so the new name survives a crash or
        // power loss; atomic rename alone guarantees visibility, not durability.
        if let Err(error) = sync_parent_directory(path) {
            return Err(IslandReplayError::Io(path.display().to_string(), error));
        }
        Ok(())
    }
}

#[cfg(unix)]
fn sync_parent_directory(path: &Path) -> io::Result<()> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::File::open(parent)?.sync_all()
}

#[cfg(not(unix))]
fn sync_parent_directory(_path: &Path) -> io::Result<()> {
    // Portable directory handles are not exposed by std on every platform.
    // The staged file itself is still synced before the atomic rename.
    Ok(())
}

/// Why a replay did not happen, or did not finish.
#[derive(Debug)]
pub enum IslandReplayError {
    Io(String, std::io::Error),
    NotALog(String, String),
    /// A replay file exceeded the bounded input size accepted by this build.
    TooLarge {
        path: String,
        bytes: u64,
        max_bytes: u64,
    },
    /// A log written by a format this build does not know.
    Version(u32),
    /// A tick's command sequence is duplicated or skips an expected entry.
    InvalidSequence {
        tick: u64,
        expected: u64,
        actual: u64,
    },
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
            Self::TooLarge {
                path,
                bytes,
                max_bytes,
            } => write!(
                f,
                "{path} is {bytes} bytes; replay logs are limited to {max_bytes} bytes"
            ),
            Self::Version(v) => write!(
                f,
                "that log is format {v}; this build reads {REPLAY_LOG_VERSION}"
            ),
            Self::InvalidSequence {
                tick,
                expected,
                actual,
            } => write!(
                f,
                "the command at tick {tick} has sequence {actual}; expected {expected}"
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

    // `record` assigns a zero-based, contiguous sequence to commands at a
    // tick. A duplicate or missing sequence makes same-tick ordering
    // ambiguous after a log is serialized and reordered, so refuse the log
    // before applying any of its commands rather than guessing.
    let mut current_tick = None;
    let mut expected_sequence = 0_u64;
    for entry in &entries {
        if current_tick != Some(entry.tick) {
            current_tick = Some(entry.tick);
            expected_sequence = 0;
        }
        if entry.sequence != expected_sequence {
            return Err(IslandReplayError::InvalidSequence {
                tick: entry.tick,
                expected: expected_sequence,
                actual: entry.sequence,
            });
        }
        expected_sequence += 1;
    }

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
