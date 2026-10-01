use serde::{Deserialize, Serialize};

use crate::agents::{
    cognition::CognitiveSnapshot, endocrinology::EndocrineSnapshot,
    nervous_system::NervousSystemSnapshot,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EmotionalValence {
    Distressed,
    Guarded,
    Neutral,
    Engaged,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AffectSnapshot {
    pub valence: EmotionalValence,
    pub urgency: f64,
    pub affiliation: f64,
}

impl Default for AffectSnapshot {
    fn default() -> Self {
        Self {
            valence: EmotionalValence::Neutral,
            urgency: 0.2,
            affiliation: 0.4,
        }
    }
}

impl AffectSnapshot {
    pub fn step(
        &self,
        cognition: &CognitiveSnapshot,
        nervous_system: &NervousSystemSnapshot,
        endocrinology: &EndocrineSnapshot,
    ) -> Self {
        let urgency =
            (cognition.risk_estimate * 0.5 + nervous_system.arousal_score * 0.5).clamp(0.0, 1.0);
        let affiliation =
            (endocrinology.oxytocin * 0.7 + cognition.coherence * 0.3).clamp(0.0, 1.0);

        let valence = if urgency > 0.8 {
            EmotionalValence::Distressed
        } else if urgency > 0.55 {
            EmotionalValence::Guarded
        } else if affiliation > 0.65 {
            EmotionalValence::Engaged
        } else {
            EmotionalValence::Neutral
        };

        Self {
            valence,
            urgency,
            affiliation,
        }
    }
}
