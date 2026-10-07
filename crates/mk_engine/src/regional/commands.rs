//! External commands and the replay log (Phase 4 Task 4).
//!
//! Commands are the **only** external inputs to the simulation. A queued
//! command takes the next human substep's tick plus a sequence number
//! assigned at queue time and applies at the *start* of that substep, in
//! `(tick, sequence)` order, so the same log applied to the same seed gives
//! the same world. Every applied command is appended to the replay log.

use mk_core::human::BiologicalSex;
use mk_interventions::InterventionAction;
use serde::{Deserialize, Serialize};

/// How fast the runner paces simulated time against the wall clock. Only
/// the runner reads it; it never changes the simulation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SimSpeed {
    RealTime,
    /// 10, 60, 360 or 1,440 times real time.
    Times(u32),
    AsFastAsPossible,
}

/// Speeds the interface offers.
pub const OFFERED_SPEEDS: [u32; 4] = [10, 60, 360, 1_440];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ControlCommand {
    Pause,
    Resume,
    /// While paused, run this many human substeps.
    Step(u64),
    /// Ask the runner to write a snapshot.
    Snapshot,
    SetSpeed(SimSpeed),
}

/// Authored identity for an arrival, as the Foundry spawns upstream: the
/// same fields and the same validation (`mk_interventions`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuthoredAppearance {
    pub height_cm: f64,
    pub build: String,
    pub hair_color: String,
    pub eye_color: String,
    pub skin_tone: String,
    /// RFC 3339 birth instant; default: computed from the age.
    #[serde(default)]
    pub birth_timestamp: Option<String>,
    /// Birthplace in degrees; default: where the person arrives
    /// (`birthplace_here`). A person may be born off the island.
    #[serde(default)]
    pub birth_latitude: Option<f64>,
    #[serde(default)]
    pub birth_longitude: Option<f64>,
}

/// Where an arrival appears. Default: the estate yard.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CreateLocation {
    /// A named estate space ("Kitchen"), at its centre.
    EstateSpace { label: String },
    /// A point in the estate patch (metres from the island's SW corner).
    EstatePoint { position_m: (f64, f64) },
    /// The centre of a medium-grid land cell.
    IslandCell { row: usize, col: usize },
}

/// A new human arriving on the island.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IslandCreateHumanRequest {
    pub agent_id: String,
    pub sex: BiologicalSex,
    pub age_years: f64,
    #[serde(default)]
    pub appearance: Option<AuthoredAppearance>,
    #[serde(default)]
    pub location: Option<CreateLocation>,
}

impl IslandCreateHumanRequest {
    /// The upstream spawn profile for this request, with `birth_timestamp`
    /// and the birthplace filled from the given defaults when not authored.
    pub fn spawn_profile(
        &self,
        default_birth: &str,
        default_birthplace: (f64, f64),
    ) -> Option<mk_interventions::HumanSpawnProfile> {
        let a = self.appearance.as_ref()?;
        Some(mk_interventions::HumanSpawnProfile {
            name: self.agent_id.trim().to_string(),
            birth_timestamp: a
                .birth_timestamp
                .clone()
                .unwrap_or_else(|| default_birth.into()),
            birth_latitude: a.birth_latitude.unwrap_or(default_birthplace.0),
            birth_longitude: a.birth_longitude.unwrap_or(default_birthplace.1),
            age_years: self.age_years,
            height_cm: a.height_cm,
            build: a.build.clone(),
            hair_color: a.hair_color.clone(),
            eye_color: a.eye_color.clone(),
            skin_tone: a.skin_tone.clone(),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IslandCommand {
    CreateHuman(IslandCreateHumanRequest),
    Intervention(InterventionAction),
    Control(ControlCommand),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandError {
    /// The action has no meaning on a bounded island (planetary climate
    /// edits, orbit changes, branching of the planet).
    NotSupportedOnIsland(String),
    DuplicateHuman(String),
    InvalidRequest(String),
}

impl std::fmt::Display for CommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for CommandError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueuedCommandId(pub u64);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IslandReplayEntry {
    pub tick: u64,
    pub sequence: u64,
    pub command: IslandCommand,
}

/// Everything needed to reproduce a run from the same scenario and seed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IslandReplayLog {
    /// blake3 of the scenario's canonical JSON: a log replays only on the
    /// scenario it was recorded on.
    pub scenario_digest: [u8; 32],
    pub entries: Vec<IslandReplayEntry>,
}

/// The queue, the applied-command log and the run controls.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandState {
    pub next_sequence: u64,
    pub pending: Vec<IslandReplayEntry>,
    pub log: Vec<IslandReplayEntry>,
    pub paused: bool,
    pub speed: SimSpeed,
    /// Substeps still allowed while paused (`Step`).
    pub step_credit: u64,
    pub snapshots_requested: u64,
}

impl Default for CommandState {
    fn default() -> Self {
        Self {
            next_sequence: 0,
            pending: Vec::new(),
            log: Vec::new(),
            paused: false,
            speed: SimSpeed::RealTime,
            step_credit: 0,
            snapshots_requested: 0,
        }
    }
}

/// Reject commands that cannot apply, before they enter the queue.
pub fn validate(command: &IslandCommand) -> Result<(), CommandError> {
    match command {
        IslandCommand::CreateHuman(r) => {
            if r.agent_id.trim().is_empty() || !(0.0..=120.0).contains(&r.age_years) {
                return Err(CommandError::InvalidRequest(
                    "a name and an age of 0-120 years".into(),
                ));
            }
            if let Some(profile) = r.spawn_profile("2000-01-01T00:00:00Z", (0.0, 0.0)) {
                let checked =
                    mk_interventions::validate_intervention(&InterventionAction::SpawnHuman {
                        template_id: "female".into(),
                        location: mk_interventions::Location::new(0.0, 0.0),
                        profile: Some(profile),
                    });
                if !checked.valid {
                    return Err(CommandError::InvalidRequest(checked.errors.join("; ")));
                }
            }
            Ok(())
        }
        IslandCommand::Control(ControlCommand::SetSpeed(SimSpeed::Times(n))) => {
            if OFFERED_SPEEDS.contains(n) {
                Ok(())
            } else {
                Err(CommandError::InvalidRequest(format!(
                    "speed {n}x is not one of {OFFERED_SPEEDS:?}"
                )))
            }
        }
        IslandCommand::Control(ControlCommand::Step(0)) => {
            Err(CommandError::InvalidRequest("Step(0)".into()))
        }
        IslandCommand::Control(_) => Ok(()),
        IslandCommand::Intervention(action) => match action {
            InterventionAction::Pause
            | InterventionAction::Resume
            | InterventionAction::Step { .. } => Ok(()),
            other => {
                let name = format!("{other:?}");
                Err(CommandError::NotSupportedOnIsland(
                    name.chars().take(60).collect(),
                ))
            }
        },
    }
}

/// An intervention that is a run control, as the control it stands for.
pub fn as_control(action: &InterventionAction) -> Option<ControlCommand> {
    match action {
        InterventionAction::Pause => Some(ControlCommand::Pause),
        InterventionAction::Resume => Some(ControlCommand::Resume),
        InterventionAction::Step { ticks } => Some(ControlCommand::Step(*ticks)),
        _ => None,
    }
}
