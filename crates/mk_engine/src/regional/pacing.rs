//! Pacing the island's simulation clock against the wall clock (Phase 4
//! Task 6). Pure arithmetic over a caller-supplied elapsed time: nothing here
//! reads a clock, and nothing here ever changes a step's size, so the
//! simulation gives the same result at any speed. Wall-clock time never
//! enters simulation state.

use std::time::Duration;

use super::commands::SimSpeed;

/// Human substeps run per call when flat out.
pub const FLAT_OUT_BATCH: u64 = 60;
/// Most steps one call may owe. If the machine falls further behind than
/// this, the debt is dropped (the dashboard shows the achieved speed) rather
/// than growing without bound.
pub const MAX_CATCH_UP_STEPS: u64 = 240;

#[derive(Debug, Clone)]
pub struct Pacer {
    step_seconds: f64,
    /// Simulated seconds owed and not yet run.
    owed_sim_s: f64,
    /// Smoothed simulated seconds per real second actually achieved.
    achieved: f64,
    requested: f64,
}

impl Pacer {
    pub fn new(step_seconds: u64) -> Self {
        Self {
            step_seconds: step_seconds.max(1) as f64,
            owed_sim_s: 0.0,
            achieved: 0.0,
            requested: 1.0,
        }
    }

    /// Simulated seconds per real second the speed asks for; `None` is flat out.
    pub fn factor(speed: SimSpeed) -> Option<f64> {
        match speed {
            SimSpeed::RealTime => Some(1.0),
            SimSpeed::Times(k) => Some(f64::from(k)),
            SimSpeed::AsFastAsPossible => None,
        }
    }

    /// Whole steps to run now, given the real time since the last call.
    pub fn steps_due(&mut self, real_elapsed: Duration, speed: SimSpeed) -> u64 {
        let Some(factor) = Self::factor(speed) else {
            self.requested = f64::INFINITY;
            self.owed_sim_s = 0.0;
            return FLAT_OUT_BATCH;
        };
        self.requested = factor;
        self.owed_sim_s += real_elapsed.as_secs_f64() * factor;
        let whole = (self.owed_sim_s / self.step_seconds).floor() as u64;
        if whole > MAX_CATCH_UP_STEPS {
            self.owed_sim_s = 0.0;
            return MAX_CATCH_UP_STEPS;
        }
        self.owed_sim_s -= whole as f64 * self.step_seconds;
        whole
    }

    /// Record that `steps` steps took `real` of wall-clock time to run.
    pub fn record(&mut self, steps: u64, real: Duration) {
        let real_s = real.as_secs_f64();
        if steps == 0 || real_s <= 0.0 {
            return;
        }
        let rate = steps as f64 * self.step_seconds / real_s;
        self.achieved = if self.achieved == 0.0 {
            rate
        } else {
            0.8 * self.achieved + 0.2 * rate
        };
    }

    /// Simulated seconds per real second achieved so far (0 before any step).
    pub fn achieved_speed(&self) -> f64 {
        self.achieved
    }

    /// Whether the machine cannot keep up with the requested speed.
    pub fn is_behind(&self) -> bool {
        self.requested.is_finite() && self.achieved > 0.0 && self.achieved < self.requested
    }
}
