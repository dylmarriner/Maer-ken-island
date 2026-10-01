use serde::{Deserialize, Serialize};

use crate::agents::{
    endocrinology::EndocrineSnapshot, nervous_system::NervousSystemSnapshot, AgentWorldObservation,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IntentFocus {
    Recover,
    Feed,
    Hydrate,
    Shelter,
    Socialize,
    Explore,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CognitiveSnapshot {
    pub coherence: f64,
    pub risk_estimate: f64,
    pub intent: IntentFocus,
}

impl Default for CognitiveSnapshot {
    fn default() -> Self {
        Self {
            coherence: 0.75,
            risk_estimate: 0.10,
            intent: IntentFocus::Explore,
        }
    }
}

impl CognitiveSnapshot {
    pub fn step(
        &self,
        nervous_system: &NervousSystemSnapshot,
        endocrinology: &EndocrineSnapshot,
        observation: &AgentWorldObservation,
    ) -> Self {
        let coherence = (nervous_system.regulatory_capacity * 0.8
            + (1.0 - nervous_system.sensory_load) * 0.2)
            .clamp(0.0, 1.0);
        let risk_estimate =
            (observation.hazard_index * 0.75 + endocrinology.stress_load() * 0.25).clamp(0.0, 1.0);

        let intent = if observation.hydration_access < 0.35 {
            IntentFocus::Hydrate
        } else if observation.caloric_access < 0.35 {
            IntentFocus::Feed
        } else if observation.shelter_quality < 0.30 || risk_estimate > 0.7 {
            IntentFocus::Shelter
        } else if observation.social_density > 0.65 && endocrinology.oxytocin > 0.5 {
            IntentFocus::Socialize
        } else if endocrinology.melatonin > 0.7 {
            IntentFocus::Recover
        } else {
            IntentFocus::Explore
        };

        Self {
            coherence,
            risk_estimate,
            intent,
        }
    }
}
