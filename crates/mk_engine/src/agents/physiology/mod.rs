use serde::{Deserialize, Serialize};

use crate::agents::AgentWorldObservation;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VitalStatus {
    Stable,
    Strained,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HomeostasisSystem {
    pub thirst_rate: f64,
    pub hunger_rate: f64,
    pub recovery_rate: f64,
    pub thermoregulation_efficiency: f64,
    pub metabolite_buffer: f64,
}

impl Default for HomeostasisSystem {
    fn default() -> Self {
        Self {
            thirst_rate: 0.03,
            hunger_rate: 0.02,
            recovery_rate: 0.05,
            thermoregulation_efficiency: 0.7,
            metabolite_buffer: 1.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhysiologySnapshot {
    pub hydration: f64,
    pub energy: f64,
    pub body_temperature_c: f64,
    pub tissue_integrity: f64,
    pub status: VitalStatus,
    pub homeostasis: HomeostasisSystem,
}

impl Default for PhysiologySnapshot {
    fn default() -> Self {
        Self {
            hydration: 0.8,
            energy: 0.8,
            body_temperature_c: 37.0,
            tissue_integrity: 1.0,
            status: VitalStatus::Stable,
            homeostasis: HomeostasisSystem::default(),
        }
    }
}

impl HomeostasisSystem {
    pub fn step(&self, action_intensity: f64) -> Self {
        let metabolite_consumption = action_intensity * 0.1;
        let metabolite_buffer = (self.metabolite_buffer - metabolite_consumption).clamp(0.0, 2.0);
        let thirst_rate = (0.03 + action_intensity * 0.02).clamp(0.01, 0.15);
        let hunger_rate = (0.02 + action_intensity * 0.015).clamp(0.01, 0.12);
        let recovery_rate = if metabolite_buffer < 0.3 { 0.01 } else { 0.05 };
        let thermoregulation_efficiency = self.thermoregulation_efficiency;

        Self {
            thirst_rate,
            hunger_rate,
            recovery_rate,
            thermoregulation_efficiency,
            metabolite_buffer,
        }
    }

    pub fn compute_needs(&self, physiology: &PhysiologySnapshot) -> (f64, f64, f64) {
        let need_water = (1.0 - physiology.hydration) + self.thirst_rate;
        let need_food = (1.0 - physiology.energy) + self.hunger_rate;
        let need_rest = if physiology.tissue_integrity < 0.5 {
            (1.0 - physiology.tissue_integrity) * 2.0
        } else if physiology.status == VitalStatus::Strained {
            0.5
        } else {
            0.0
        };
        (need_food, need_water, need_rest)
    }
}

impl PhysiologySnapshot {
    pub fn step(&self, observation: &AgentWorldObservation, action_intensity: f64) -> Self {
        let homeostasis = self.homeostasis.step(action_intensity);

        let hydration = (self.hydration + (observation.hydration_access - 0.5) * 0.08
            - homeostasis.thirst_rate * 0.05)
            .clamp(0.0, 1.0);
        let energy = (self.energy + (observation.caloric_access - 0.5) * 0.07
            - homeostasis.hunger_rate * 0.05)
            .clamp(0.0, 1.0);
        let thermal_delta = (observation.ambient_temperature_c - 18.0)
            * 0.05
            * (1.0 + (1.0 - homeostasis.thermoregulation_efficiency) * 2.0);
        let body_temperature_c = (36.8 + thermal_delta).clamp(30.0, 42.0);
        let wear = (observation.hazard_index * 0.02)
            + ((1.0 - hydration) * 0.01)
            + ((1.0 - energy) * 0.01)
            + action_intensity * 0.01;
        let tissue_integrity =
            (self.tissue_integrity - wear + homeostasis.recovery_rate * 0.02).clamp(0.0, 1.0);

        let status = if tissue_integrity < 0.2 || hydration < 0.15 || energy < 0.15 {
            VitalStatus::Critical
        } else if tissue_integrity < 0.5 || hydration < 0.4 || energy < 0.4 {
            VitalStatus::Strained
        } else {
            VitalStatus::Stable
        };

        Self {
            hydration,
            energy,
            body_temperature_c,
            tissue_integrity,
            status,
            homeostasis,
        }
    }

    pub fn vitality_score(&self) -> f64 {
        ((self.hydration
            + self.energy
            + self.tissue_integrity
            + self.homeostasis.metabolite_buffer.min(1.0))
            / 4.0)
            .clamp(0.0, 1.0)
    }
}
