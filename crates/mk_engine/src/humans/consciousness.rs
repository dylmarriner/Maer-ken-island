//! Consciousness Snapshot - self-awareness, intrinsic worth, fear, qualia,
//! internal monologue.
//!
//! Surfaces `docs/canon/HumanReplicationSchema.js`'s `ConsciousnessSchema`,
//! ported in `mk_core::human::schema` but previously unread by the engine.
//! This is a pure projection (like genetics/cognition/culture) — the
//! canon fields here are stable trait-like scalars, not something with an
//! obvious real-time physiological driver the way needs/body/immune have.

use mk_core::human::HumanProfile;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsciousnessSnapshot {
    // --- self-awareness ---
    pub meta_cognition: f64,
    pub self_monitoring: f64,
    pub identity_continuity: f64,
    pub agency_recognition: f64,
    pub perspective_taking: f64,

    // --- intrinsic worth ---
    pub self_value: f64,
    pub worth_stability: f64,
    pub external_validation_need: f64,
    pub self_compassion: f64,
    pub growth_mindset: f64,

    // --- fear system ---
    pub fear_of_loss: f64,
    pub uncertainty_tolerance: f64,
    pub threat_detection: f64,
    pub anxiety_baseline: f64,
    pub coping_mechanisms: f64,

    // --- qualia ---
    pub sensory_richness: f64,
    pub emotional_depth: f64,
    pub meaning_making: f64,

    // --- internal monologue ---
    pub verbal_thought: f64,
    pub inner_speech: f64,
    pub narrative_coherence: f64,
}

impl ConsciousnessSnapshot {
    pub fn from_profile(profile: &HumanProfile) -> Self {
        let Some(schema) = profile.canonical_schema() else {
            return Self::defaults();
        };
        let c = &schema.consciousness;

        Self {
            meta_cognition: nonzero_or(c.self_awareness.meta_cognition as f64, 0.5),
            self_monitoring: nonzero_or(c.self_awareness.self_monitoring as f64, 0.5),
            identity_continuity: nonzero_or(c.self_awareness.identity_continuity as f64, 0.7),
            agency_recognition: nonzero_or(c.self_awareness.agency_recognition as f64, 0.7),
            perspective_taking: nonzero_or(c.self_awareness.perspective_taking as f64, 0.5),

            self_value: nonzero_or(c.intrinsic_worth.self_value as f64, 0.6),
            worth_stability: nonzero_or(c.intrinsic_worth.worth_stability as f64, 0.6),
            external_validation_need: c.intrinsic_worth.external_validation_need as f64,
            self_compassion: nonzero_or(c.intrinsic_worth.self_compassion as f64, 0.5),
            growth_mindset: nonzero_or(c.intrinsic_worth.growth_mindset as f64, 0.5),

            fear_of_loss: c.fear_system.fear_of_loss as f64,
            uncertainty_tolerance: nonzero_or(c.fear_system.uncertainty_tolerance as f64, 0.5),
            threat_detection: nonzero_or(c.fear_system.threat_detection as f64, 0.5),
            anxiety_baseline: c.fear_system.anxiety_baseline as f64,
            coping_mechanisms: nonzero_or(c.fear_system.coping_mechanisms as f64, 0.5),

            sensory_richness: nonzero_or(c.qualia_system.sensory_richness as f64, 0.6),
            emotional_depth: nonzero_or(c.qualia_system.emotional_depth as f64, 0.6),
            meaning_making: nonzero_or(c.qualia_system.meaning_making as f64, 0.5),

            verbal_thought: nonzero_or(c.internal_monologue.verbal_thought as f64, 0.6),
            inner_speech: nonzero_or(c.internal_monologue.inner_speech as f64, 0.6),
            narrative_coherence: nonzero_or(c.internal_monologue.narrative_coherence as f64, 0.6),
        }
    }

    fn defaults() -> Self {
        Self {
            meta_cognition: 0.5,
            self_monitoring: 0.5,
            identity_continuity: 0.7,
            agency_recognition: 0.7,
            perspective_taking: 0.5,
            self_value: 0.6,
            worth_stability: 0.6,
            external_validation_need: 0.4,
            self_compassion: 0.5,
            growth_mindset: 0.5,
            fear_of_loss: 0.3,
            uncertainty_tolerance: 0.5,
            threat_detection: 0.5,
            anxiety_baseline: 0.3,
            coping_mechanisms: 0.5,
            sensory_richness: 0.6,
            emotional_depth: 0.6,
            meaning_making: 0.5,
            verbal_thought: 0.6,
            inner_speech: 0.6,
            narrative_coherence: 0.6,
        }
    }
}

fn nonzero_or(value: f64, default: f64) -> f64 {
    if value == 0.0 {
        default
    } else {
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mk_core::human::{HumanId, HumanSchema};

    #[test]
    fn from_profile_uses_sane_defaults() {
        let schema = HumanSchema::canonical_minimal("consciousness_test");
        let profile = HumanProfile::from_canonical_schema(HumanId::new(1), schema);
        let snapshot = ConsciousnessSnapshot::from_profile(&profile);
        assert!(snapshot.identity_continuity > 0.0);
    }
}
