use serde::{Deserialize, Serialize};

/// Will status — how mentally fatigued the agent is
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WillStatus {
    Fresh,
    Fatigued,
    Exhausted,
}

/// Willpower snapshot for one tick
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WillSnapshot {
    pub reserves: f64, // 0.0-1.0
    pub focus: f64,    // 0.0-1.0, how much attention available
    pub status: WillStatus,
    pub depletion_rate: f64, // rate of will loss per tick
    pub recovery_rate: f64,  // rate of will gain per tick while resting
}

impl Default for WillSnapshot {
    fn default() -> Self {
        Self {
            reserves: 1.0,
            focus: 0.8,
            status: WillStatus::Fresh,
            depletion_rate: 0.02,
            recovery_rate: 0.05,
        }
    }
}

impl WillSnapshot {
    /// Create a new will snapshot
    pub fn new() -> Self {
        Self::default()
    }

    /// Step willpower based on action intensity and stress
    pub fn step(
        &self,
        action_intensity: f64,
        stress_load: f64,
        is_resting: bool,
        coherence: f64,
    ) -> Self {
        let mut reserves = self.reserves;

        if is_resting {
            // Recovery while resting
            reserves = (reserves + self.recovery_rate).min(1.0);
        } else {
            // Depletion based on action and stress
            let depletion =
                self.depletion_rate * (1.0 + action_intensity * 0.5 + stress_load * 0.3);
            reserves = (reserves - depletion).max(0.0);
        }

        // Focus depends on reserves and coherence
        let focus = (reserves * 0.6 + coherence * 0.4).clamp(0.0, 1.0);

        // Status thresholds
        let status = if reserves < 0.2 {
            WillStatus::Exhausted
        } else if reserves < 0.5 {
            WillStatus::Fatigued
        } else {
            WillStatus::Fresh
        };

        Self {
            reserves,
            focus,
            status,
            depletion_rate: self.depletion_rate,
            recovery_rate: self.recovery_rate,
        }
    }

    /// Check if the agent has enough will to act
    pub fn can_act(&self) -> bool {
        matches!(self.status, WillStatus::Fresh | WillStatus::Fatigued)
    }

    /// Get a multiplier for action effectiveness based on will
    pub fn effectiveness_multiplier(&self) -> f64 {
        match self.status {
            WillStatus::Fresh => 1.0,
            WillStatus::Fatigued => 0.6,
            WillStatus::Exhausted => 0.2,
        }
    }
}
