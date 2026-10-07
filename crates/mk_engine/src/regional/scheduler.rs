//! The island's deterministic scheduler (Phase 4 Task 2).
//!
//! Time advances in **human substeps** (`human_seconds`, default 60 s). Slower
//! systems fire when the simulated clock reaches a multiple of their
//! cadence, so the sequence of firings depends only on the clock, never on
//! how the caller sliced time: one 86,400 s step and 1,440 steps of 60 s
//! fire the same systems at the same instants. A step that is not a whole
//! number of substeps (`dt = 3,701`) runs the whole substeps and carries the
//! remainder (41 s) in the accumulator into the next call.
//!
//! Order within a substep: queued commands, then humans and estate energy,
//! then weather/ocean/zonal background/insolation, then hydrology,
//! ecology, resources and local vegetation, then geophysics, then the human
//! store sync, then the audit and hash commit.

use mk_island::IslandCadenceProfile;
use serde::{Deserialize, Serialize};

/// How many times each subsystem has fired.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchedulerCounters {
    pub human: u64,
    pub weather_ocean: u64,
    pub hydrology_ecology_resource: u64,
    pub geophysics: u64,
    pub human_store: u64,
}

/// What fires at a given instant (besides humans, which fire every substep).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Due {
    pub weather_ocean: bool,
    pub hydrology_ecology_resource: bool,
    pub geophysics: bool,
    pub human_store: bool,
}

/// Seconds of a step not yet run as a whole substep.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchedulerAccumulators {
    pub remainder_s: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IslandScheduler {
    pub cadences: IslandCadenceProfile,
    pub accumulators: SchedulerAccumulators,
    pub counters: SchedulerCounters,
}

impl IslandScheduler {
    pub fn new(cadences: IslandCadenceProfile) -> Self {
        Self {
            cadences,
            accumulators: SchedulerAccumulators::default(),
            counters: SchedulerCounters::default(),
        }
    }

    /// Whole substeps in `dt_seconds` plus the carried remainder; the new
    /// remainder is kept for the next call.
    pub fn take_substeps(&mut self, dt_seconds: u64) -> u64 {
        let total = self.accumulators.remainder_s + dt_seconds;
        self.accumulators.remainder_s = total % self.cadences.human_seconds;
        total / self.cadences.human_seconds
    }

    /// What is due at simulated time `sim_time_s` (just after a substep).
    pub fn due(&self, sim_time_s: u64) -> Due {
        let c = &self.cadences;
        Due {
            weather_ocean: sim_time_s.is_multiple_of(c.weather_ocean_seconds),
            hydrology_ecology_resource: sim_time_s
                .is_multiple_of(c.hydrology_ecology_resource_seconds),
            geophysics: sim_time_s.is_multiple_of(c.geophysics_seconds),
            human_store: sim_time_s.is_multiple_of(c.human_store_seconds),
        }
    }

    /// Record a substep and what fired in it.
    pub fn record(&mut self, due: Due) {
        self.counters.human += 1;
        self.counters.weather_ocean += u64::from(due.weather_ocean);
        self.counters.hydrology_ecology_resource += u64::from(due.hydrology_ecology_resource);
        self.counters.geophysics += u64::from(due.geophysics);
        self.counters.human_store += u64::from(due.human_store);
    }
}
