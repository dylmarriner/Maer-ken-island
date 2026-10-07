//! The island world as one deterministic state (Phase 4 Task 1).
//!
//! `IslandWorldState` is the single object the runtime steps, saves and
//! replays. It composes every Phase 1-3 system through [`IslandLife`] (the
//! physical world, ecology, estate, layout, trees, humans, energy, materials
//! and economy) from one constructor. The scheduler, snapshot, command queue
//! and chronicle of Phase 4's later tasks are added here.

use std::sync::Arc;

use mk_core::canon::CanonLocked;
use mk_island::IslandScenario;

use super::life::{IslandLife, IslandLifeError};

#[derive(Debug)]
pub enum IslandWorldError {
    Bootstrap(IslandLifeError),
}

impl std::fmt::Display for IslandWorldError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Bootstrap(e) => write!(f, "bootstrap failed: {e}"),
        }
    }
}

impl std::error::Error for IslandWorldError {}

pub struct IslandWorldState {
    pub life: IslandLife,
}

impl IslandWorldState {
    /// The whole island from a scenario: domain, physical state, ecology,
    /// property, layout, patch vegetation and founders, in bootstrap order.
    pub fn new(
        canon: Arc<CanonLocked>,
        scenario: IslandScenario,
    ) -> Result<Self, IslandWorldError> {
        Ok(Self {
            life: IslandLife::bootstrap(scenario, canon).map_err(IslandWorldError::Bootstrap)?,
        })
    }

    pub fn tick(&self) -> u64 {
        self.life.tick
    }

    pub fn sim_time_seconds(&self) -> f64 {
        self.life.sim_time_s as f64
    }

    /// The canonical state hash: JSON `Value` (sorted maps) hashed, so
    /// `HashMap`-backed upstream state hashes the same in every process.
    pub fn state_hash(&self) -> Result<[u8; 32], IslandWorldError> {
        Ok(self.life.state_digest())
    }
}
