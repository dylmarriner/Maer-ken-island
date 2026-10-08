//! Phase 4 Task 4: the only ways in from outside.
//!
//! Everything that reaches a running island from outside it is one of
//! these. That is the point: if the world can only be changed through a
//! named, recordable command, then recording the commands is enough to
//! reproduce the run — which is what [`super::replay`] does.
//!
//! A command is applied by the thread that owns the island, between steps,
//! never by whoever asked for it.

use serde::{Deserialize, Serialize};

use super::create_human::{CreateHumanError, IslandCreateHuman};

/// Something asked of the island from outside.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IslandCommand {
    /// Put a new person into the world.
    CreateHuman(Box<IslandCreateHuman>),
    /// Change how the island is being run, without changing the world.
    Control(ControlCommand),
}

/// Running the island, as against changing it.
///
/// These are recorded because a replay should be able to say what the
/// operator did, but they move nothing: pausing an island and resuming it
/// leaves exactly the state it was paused in, and a speed is a statement
/// about real time, which never enters simulation state. So a replay
/// carries them for the record and applies nothing — and the final digest
/// is the same whether they are in the log or not, which
/// `island_replay` asserts rather than assumes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ControlCommand {
    Pause,
    Resume,
    /// Advance exactly `0` steps and stop again.
    Step(u64),
    /// Write a snapshot. The file it writes is not part of the world.
    Snapshot,
    /// How fast to run, as `island serve --speed` spells it.
    SetSpeed(String),
}

/// Why a command was not applied.
#[derive(Debug)]
pub enum CommandError {
    /// The island refused the creation, with a reason per problem.
    Create(CreateHumanError),
}

impl std::fmt::Display for CommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Create(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for CommandError {}

/// What applying a command did.
#[derive(Debug, Clone)]
pub enum Applied {
    Created(super::create_human::CreatedIslander),
    /// A control command: recorded, and nothing in the world moved.
    Noted,
}
