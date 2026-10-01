use serde::{Deserialize, Serialize};

use crate::agents::{
    endocrinology::EndocrineSnapshot,
    physiology::{PhysiologySnapshot, VitalStatus},
    AgentWorldObservation,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MortalityReason {
    Alive,
    SystemicFailure,
    Exposure,
    Starvation,
    Senescence,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MortalitySnapshot {
    pub alive: bool,
    pub reason: MortalityReason,
}

impl Default for MortalitySnapshot {
    fn default() -> Self {
        Self {
            alive: true,
            reason: MortalityReason::Alive,
        }
    }
}

impl MortalitySnapshot {
    pub fn step(
        &self,
        physiology: &PhysiologySnapshot,
        endocrinology: &EndocrineSnapshot,
        observation: &AgentWorldObservation,
        age_ticks: u64,
    ) -> Self {
        if !self.alive {
            return self.clone();
        }

        let reason = if matches!(physiology.status, VitalStatus::Critical)
            && physiology.tissue_integrity < 0.1
        {
            MortalityReason::SystemicFailure
        } else if physiology.hydration < 0.05 || physiology.energy < 0.05 {
            MortalityReason::Starvation
        } else if observation.hazard_index > 0.95 && endocrinology.stress_load() > 0.9 {
            MortalityReason::Exposure
        } else if age_ticks > 10_000_000 {
            MortalityReason::Senescence
        } else {
            MortalityReason::Alive
        };

        Self {
            alive: matches!(reason, MortalityReason::Alive),
            reason,
        }
    }

    pub fn is_alive(&self) -> bool {
        self.alive
    }
}
