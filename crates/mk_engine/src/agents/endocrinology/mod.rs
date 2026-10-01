use serde::{Deserialize, Serialize};

use crate::agents::{physiology::PhysiologySnapshot, AgentWorldObservation};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HormoneAxis {
    Baseline,
    StressDominant,
    RewardSeeking,
    Recovery,
    FocusedDominant,
    SocialDominant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndocrineSnapshot {
    pub cortisol: f64,
    pub dopamine: f64,
    pub oxytocin: f64,
    pub melatonin: f64,
    pub adrenaline: f64,
    pub serotonin: f64,
    pub norepinephrine: f64,
    pub axis: HormoneAxis,
}

impl Default for EndocrineSnapshot {
    fn default() -> Self {
        Self {
            cortisol: 0.25,
            dopamine: 0.55,
            oxytocin: 0.45,
            melatonin: 0.30,
            adrenaline: 0.2,
            serotonin: 0.5,
            norepinephrine: 0.3,
            axis: HormoneAxis::Baseline,
        }
    }
}

impl EndocrineSnapshot {
    pub fn step(
        &self,
        physiology: &PhysiologySnapshot,
        observation: &AgentWorldObservation,
        action_intensity: f64,
    ) -> Self {
        let cortisol =
            (0.15 + observation.hazard_index * 0.8 + (1.0 - physiology.vitality_score()) * 0.4)
                .clamp(0.0, 1.0);
        let dopamine =
            (0.35 + observation.caloric_access * 0.35 + observation.daylight_fraction * 0.15)
                .clamp(0.0, 1.0);
        let oxytocin =
            (0.15 + observation.social_density * 0.6 + observation.shelter_quality * 0.1)
                .clamp(0.0, 1.0);
        let melatonin = (1.0 - observation.daylight_fraction).clamp(0.0, 1.0);
        let adrenaline = (observation.hazard_index * 0.6
            + (1.0 - physiology.vitality_score()) * 0.3
            + action_intensity * 0.2)
            .clamp(0.0, 1.0);
        let serotonin = (observation.social_density * 0.4
            + observation.daylight_fraction * 0.3
            + (1.0 - self.stress_load()) * 0.3)
            .clamp(0.0, 1.0);
        let norepinephrine = (observation.hazard_index * 0.3
            + action_intensity * 0.3
            + (1.0 - physiology.vitality_score()) * 0.2)
            .clamp(0.0, 1.0);

        let axis = if cortisol > 0.7 {
            HormoneAxis::StressDominant
        } else if norepinephrine > 0.6 && adrenaline > 0.5 {
            HormoneAxis::FocusedDominant
        } else if dopamine > 0.7 {
            HormoneAxis::RewardSeeking
        } else if oxytocin > 0.6 && serotonin > 0.6 {
            HormoneAxis::SocialDominant
        } else if melatonin > 0.7 {
            HormoneAxis::Recovery
        } else {
            HormoneAxis::Baseline
        };

        Self {
            cortisol,
            dopamine,
            oxytocin,
            melatonin,
            adrenaline,
            serotonin,
            norepinephrine,
            axis,
        }
    }

    pub fn stress_load(&self) -> f64 {
        (self.cortisol - self.dopamine * 0.15 - self.oxytocin * 0.1).clamp(0.0, 1.0)
    }

    pub fn reward_response(&self) -> f64 {
        (self.dopamine * 0.4 + self.serotonin * 0.3 + self.oxytocin * 0.2 - self.cortisol * 0.1)
            .clamp(0.0, 1.0)
    }

    pub fn drive_urgency(&self) -> f64 {
        (self.norepinephrine * 0.3 + self.adrenaline * 0.3 + self.cortisol * 0.2
            - self.serotonin * 0.2
            - self.melatonin * 0.2)
            .clamp(0.0, 1.0)
    }
}
