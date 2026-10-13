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

use super::deep_time::{self, DeepTimeError, SpinUpReport, SpinUpSettings};
use super::life::{IslandLife, IslandLifeError};

#[derive(Debug)]
pub enum IslandWorldError {
    Bootstrap(IslandLifeError),
    DeepTime(DeepTimeError),
}

impl std::fmt::Display for IslandWorldError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Bootstrap(e) => write!(f, "bootstrap failed: {e}"),
            Self::DeepTime(e) => write!(f, "deep-time spin-up failed: {e}"),
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

    /// Run the living world for `years` more local years of deep time.
    ///
    /// Only an island nobody lives on can be fast-forwarded: humans always
    /// run at full detail, so once the registry holds anyone (alive or
    /// dead) this refuses and changes nothing. A scenario's own spin-up runs
    /// inside [`Self::new`], before the founders arrive.
    pub fn spin_up(&mut self, years: u32) -> Result<SpinUpReport, IslandWorldError> {
        let people = self.life.humans.registry.iter().count();
        if people > 0 {
            return Err(IslandWorldError::DeepTime(DeepTimeError::HumansExist(
                people,
            )));
        }
        let area = self
            .life
            .domain
            .cell_area_m2(mk_island::DomainLevel::Medium);
        deep_time::spin_up(
            &mut self.life.ecology,
            area,
            SpinUpSettings::new(&self.life.canon, years),
        )
        .map_err(IslandWorldError::DeepTime)
    }

    pub fn tick(&self) -> u64 {
        self.life.tick
    }

    pub fn sim_time_seconds(&self) -> f64 {
        self.life.sim_time_s as f64
    }

    /// Advance by `dt_seconds`: whole human substeps run in the scheduler's
    /// fixed order, the remainder is carried to the next call.
    pub fn step(&mut self, dt_seconds: u64) -> Result<(), IslandWorldError> {
        self.life
            .advance(dt_seconds)
            .map_err(IslandWorldError::Bootstrap)
    }

    /// The canonical state hash: JSON `Value` (sorted maps) hashed, so
    /// `HashMap`-backed upstream state hashes the same in every process.
    pub fn state_hash(&self) -> Result<[u8; 32], IslandWorldError> {
        Ok(self.life.state_digest())
    }
}
