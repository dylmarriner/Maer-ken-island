/// Phase 5 Replay Module
///
/// Provides deterministic replay functionality for the world simulation.
/// Enables loading snapshots and replaying from any tick with hash verification.
use crate::world_integration::{WorldSnapshot, WorldState};
use bincode;
use mk_core::time::Tick;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use tracing::warn;

/// Replay session for deterministic simulation replay
#[derive(Debug, Clone)]
pub struct ReplaySession {
    /// Initial snapshot
    pub initial_snapshot: WorldSnapshot,

    /// Current replay state
    pub current_state: WorldState,

    /// Independent copy of the initial state, stepped in lockstep with
    /// `current_state`. Two independently-stepped copies of the same
    /// starting state diverging is exactly what "non-deterministic replay"
    /// means, so this is what `verify_hashes` actually checks against —
    /// merely observing that a single run's hash changed tick-to-tick
    /// proves nothing about determinism (a previous version of this file
    /// did exactly that).
    shadow_state: WorldState,

    /// Target tick to replay to
    pub target_tick: Tick,

    /// Replay options
    pub options: ReplayOptions,

    /// Replay history for verification
    pub history: Vec<ReplayEntry>,
}

/// Replay options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayOptions {
    /// Verify hash at each step
    pub verify_hashes: bool,

    /// Stop on first error
    pub stop_on_error: bool,

    /// Maximum ticks to replay (0 = unlimited)
    pub max_ticks: u64,

    /// Capture detailed metrics
    pub capture_metrics: bool,
}

impl Default for ReplayOptions {
    fn default() -> Self {
        Self {
            verify_hashes: true,
            stop_on_error: true,
            max_ticks: 0,
            capture_metrics: true,
        }
    }
}

/// Replay entry for tracking replay progress
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayEntry {
    pub tick: Tick,
    pub hash_before: [u8; 32],
    pub hash_after: [u8; 32],
    pub status: ReplayStatus,
    pub error_message: Option<String>,
}

/// Replay status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReplayStatus {
    Success,
    HashMismatch,
    ValidationError(String),
    StepError(String),
}

impl ReplaySession {
    /// Create a new replay session from a snapshot
    pub fn new(snapshot: WorldSnapshot, options: ReplayOptions) -> Result<Self, ReplayError> {
        let current_state = WorldState::from_snapshot(&snapshot)
            .map_err(|e| ReplayError::LoadError(format!("Snapshot error: {e}")))?;
        let shadow_state = current_state.clone();

        Ok(Self {
            initial_snapshot: snapshot.clone(),
            current_state,
            shadow_state,
            target_tick: snapshot.tick,
            options,
            history: Vec::new(),
        })
    }

    /// Create a replay session from an already validated world state.
    ///
    /// Use this after `io::load_snapshot` has already verified and restored
    /// the state, so it is not serialized and parsed a second time before
    /// the replay starts.
    pub fn from_world_state(
        current_state: WorldState,
        options: ReplayOptions,
    ) -> Result<Self, ReplayError> {
        let initial_snapshot = current_state
            .create_snapshot()
            .map_err(|error| ReplayError::LoadError(format!("Snapshot error: {error:?}")))?;
        let target_tick = current_state.tick;
        let shadow_state = current_state.clone();
        Ok(Self {
            initial_snapshot,
            current_state,
            shadow_state,
            target_tick,
            options,
            history: Vec::new(),
        })
    }

    /// Replay to a specific tick
    pub fn replay_to(&mut self, target_tick: Tick) -> Result<(), ReplayError> {
        self.target_tick = target_tick;

        let start_tick = self.current_state.tick;
        let ticks_to_replay = target_tick.saturating_sub(start_tick);

        if self.options.max_ticks > 0 && ticks_to_replay > self.options.max_ticks {
            return Err(ReplayError::MaxTicksExceeded);
        }

        while self.current_state.tick < target_tick {
            if let Err(e) = self.step_replay() {
                if self.options.stop_on_error {
                    return Err(e);
                }
                // Log error and continue if not stopping on error
                warn!(tick = self.current_state.tick, error = %e, "Replay error, continuing");
            }
        }

        Ok(())
    }

    /// Execute one replay step
    fn step_replay(&mut self) -> Result<(), ReplayError> {
        let tick_before = self.current_state.tick;
        let hash_before = self.current_state.hash_chain.current;

        // Execute world step
        self.current_state
            .step_world(1)
            .map_err(|e| ReplayError::StepError(format!("{:?}", e)))?;

        let hash_after = self.current_state.hash_chain.current;

        // Verify determinism if enabled: step an independent copy of the
        // same starting state through the same tick and confirm it lands
        // on the exact same hash chain. Two runs that start identical and
        // diverge is the actual definition of non-deterministic replay.
        if self.options.verify_hashes {
            self.shadow_state
                .step_world(1)
                .map_err(|e| ReplayError::StepError(format!("shadow run: {:?}", e)))?;

            if self.shadow_state.hash_chain.current != hash_after {
                return Err(ReplayError::HashMismatch);
            }

            // Hash equality alone can't catch a divergence in fields the
            // hash chain doesn't cover; a full byte-level comparison is the
            // strongest available proof of determinism at each step.
            let primary_bytes = bincode::serialize(&self.current_state)
                .map_err(|e| ReplayError::LoadError(format!("Serialization error: {}", e)))?;
            let shadow_bytes = bincode::serialize(&self.shadow_state)
                .map_err(|e| ReplayError::LoadError(format!("Serialization error: {}", e)))?;
            if primary_bytes != shadow_bytes {
                return Err(ReplayError::IntegrityError(format!(
                    "state diverged at tick {} despite matching hash chain",
                    tick_before
                )));
            }
        }

        // Record replay entry
        let entry = ReplayEntry {
            tick: tick_before,
            hash_before,
            hash_after,
            status: ReplayStatus::Success,
            error_message: None,
        };

        self.history.push(entry);

        Ok(())
    }

    /// Verify replay integrity
    pub fn verify(&self) -> Result<(), ReplayError> {
        // Verify final state
        self.current_state
            .validate()
            .map_err(|e| ReplayError::ValidationError(format!("{:?}", e)))?;

        // Verify all replay entries
        for entry in &self.history {
            match &entry.status {
                ReplayStatus::Success => continue,
                ReplayStatus::HashMismatch => {
                    return Err(ReplayError::IntegrityError(format!(
                        "Hash mismatch at tick {}",
                        entry.tick
                    )));
                }
                ReplayStatus::ValidationError(msg) => {
                    return Err(ReplayError::IntegrityError(format!(
                        "Validation error at tick {}: {}",
                        entry.tick, msg
                    )));
                }
                ReplayStatus::StepError(msg) => {
                    return Err(ReplayError::IntegrityError(format!(
                        "Step error at tick {}: {}",
                        entry.tick, msg
                    )));
                }
            }
        }

        Ok(())
    }

    /// Get replay statistics
    pub fn get_statistics(&self) -> ReplayStatistics {
        let successful_steps = self
            .history
            .iter()
            .filter(|e| matches!(e.status, ReplayStatus::Success))
            .count();

        let failed_steps = self.history.len() - successful_steps;

        ReplayStatistics {
            total_steps: self.history.len(),
            successful_steps,
            failed_steps,
            start_tick: self.initial_snapshot.tick,
            end_tick: self.current_state.tick,
            final_hash: self.current_state.hash_chain.current,
        }
    }
}

/// Replay statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayStatistics {
    pub total_steps: usize,
    pub successful_steps: usize,
    pub failed_steps: usize,
    pub start_tick: Tick,
    pub end_tick: Tick,
    pub final_hash: [u8; 32],
}

/// Replay errors
#[derive(Debug, Clone)]
pub enum ReplayError {
    LoadError(String),
    StepError(String),
    HashMismatch,
    ValidationError(String),
    IntegrityError(String),
    MaxTicksExceeded,
}

impl fmt::Display for ReplayError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReplayError::LoadError(msg) => write!(f, "Load error: {}", msg),
            ReplayError::StepError(msg) => write!(f, "Step error: {}", msg),
            ReplayError::HashMismatch => write!(f, "Hash mismatch"),
            ReplayError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
            ReplayError::IntegrityError(msg) => write!(f, "Integrity error: {}", msg),
            ReplayError::MaxTicksExceeded => write!(f, "Max ticks exceeded"),
        }
    }
}

/// Replay manager for managing multiple replay sessions
#[derive(Debug, Clone)]
pub struct ReplayManager {
    sessions: HashMap<String, ReplaySession>,
}

impl ReplayManager {
    /// Create new replay manager
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
        }
    }

    /// Add a replay session
    pub fn add_session(&mut self, name: String, session: ReplaySession) {
        self.sessions.insert(name, session);
    }

    /// Get a replay session
    pub fn get_session(&self, name: &str) -> Option<&ReplaySession> {
        self.sessions.get(name)
    }

    /// Get mutable replay session
    pub fn get_session_mut(&mut self, name: &str) -> Option<&mut ReplaySession> {
        self.sessions.get_mut(name)
    }

    /// Remove a replay session
    pub fn remove_session(&mut self, name: &str) -> Option<ReplaySession> {
        self.sessions.remove(name)
    }

    /// List all session names
    pub fn list_sessions(&self) -> Vec<String> {
        self.sessions.keys().cloned().collect()
    }
}

impl Default for ReplayManager {
    fn default() -> Self {
        Self::new()
    }
}
