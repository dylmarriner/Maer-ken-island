use serde::{Deserialize, Serialize};

use crate::agents::{
    endocrinology::EndocrineSnapshot, physiology::PhysiologySnapshot, AgentWorldObservation,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArousalBand {
    Rest,
    Alert,
    Agitated,
    Collapse,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NervousSystemSnapshot {
    pub arousal_score: f64,
    pub sensory_load: f64,
    pub regulatory_capacity: f64,
    pub band: ArousalBand,
}

impl Default for NervousSystemSnapshot {
    fn default() -> Self {
        Self {
            arousal_score: 0.35,
            sensory_load: 0.25,
            regulatory_capacity: 0.7,
            band: ArousalBand::Rest,
        }
    }
}

impl NervousSystemSnapshot {
    pub fn step(
        &self,
        physiology: &PhysiologySnapshot,
        endocrinology: &EndocrineSnapshot,
        observation: &AgentWorldObservation,
    ) -> Self {
        let sensory_load = (observation.social_density * 0.25
            + observation.hazard_index * 0.65
            + (1.0 - observation.shelter_quality) * 0.2)
            .clamp(0.0, 1.0);
        let regulatory_capacity = (physiology.vitality_score() * 0.7
            + (1.0 - endocrinology.stress_load()) * 0.3)
            .clamp(0.0, 1.0);
        let arousal_score = (sensory_load * 0.6 + endocrinology.cortisol * 0.4
            - endocrinology.melatonin * 0.2)
            .clamp(0.0, 1.0);

        let band = if regulatory_capacity < 0.2 {
            ArousalBand::Collapse
        } else if arousal_score > 0.8 {
            ArousalBand::Agitated
        } else if arousal_score > 0.45 {
            ArousalBand::Alert
        } else {
            ArousalBand::Rest
        };

        Self {
            arousal_score,
            sensory_load,
            regulatory_capacity,
            band,
        }
    }
}
