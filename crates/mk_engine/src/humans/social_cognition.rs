//! Social Cognition Snapshot - theory of mind, social perception, and
//! interaction style.
//!
//! Surfaces `docs/canon/HumanReplicationSchema.js`'s `SocialCognitionSchema`,
//! ported in `mk_core::human::schema` but previously unread by the engine.
//! Distinct from [`super::social_systems::SocialSystemsSnapshot`], which
//! projects `RelationalDefaultsSchema`/`AttachmentStyleSchema` (attachment
//! security, boundary detection, jealousy) — this module covers theory of
//! mind and moment-to-moment social perception. Coupled to `attention`:
//! mental-state inference and social-cue reading both draw on attentional
//! resources and degrade under fatigue, mirroring real cognitive-empathy
//! effects.

use mk_core::human::HumanProfile;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocialCognitionSnapshot {
    // --- theory of mind (traits) ---
    pub mental_state_inference: f64,
    pub intention_recognition: f64,
    pub perspective_taking: f64,
    pub false_belief_understanding: f64,

    // --- social perception (traits) ---
    pub emotion_recognition: f64,
    pub social_cue_interpretation: f64,
    pub trust_assessment: f64,

    // --- social interaction (traits) ---
    pub conflict_resolution: f64,
    pub cooperation_tendency: f64,
    pub empathy_level: f64,
    pub social_anxiety: f64,

    // --- state (stepped) ---
    /// Effective mental-state inference after attentional-resource
    /// availability (0-1).
    pub effective_mental_state_inference: f64,
    /// Effective social-cue reading, degraded by fatigue/attention load
    /// (0-1).
    pub effective_cue_reading: f64,
}

impl SocialCognitionSnapshot {
    pub fn from_profile(profile: &HumanProfile) -> Self {
        let Some(schema) = profile.canonical_schema() else {
            return Self::defaults();
        };
        let s = &schema.social_cognition;

        Self {
            mental_state_inference: nonzero_or(s.theory_of_mind.mental_state_inference, 0.5),
            intention_recognition: nonzero_or(s.theory_of_mind.intention_recognition, 0.5),
            perspective_taking: nonzero_or(s.theory_of_mind.perspective_taking, 0.5),
            false_belief_understanding: nonzero_or(
                s.theory_of_mind.false_belief_understanding,
                0.5,
            ),

            emotion_recognition: nonzero_or(s.social_perception.emotion_recognition, 0.5),
            social_cue_interpretation: nonzero_or(
                s.social_perception.social_cue_interpretation,
                0.5,
            ),
            trust_assessment: nonzero_or(s.social_perception.trust_assessment, 0.5),

            conflict_resolution: nonzero_or(s.social_interaction.conflict_resolution, 0.5),
            cooperation_tendency: nonzero_or(s.social_interaction.cooperation_tendency, 0.5),
            empathy_level: nonzero_or(s.social_interaction.empathy_level, 0.5),
            social_anxiety: s.social_interaction.social_anxiety as f64,

            effective_mental_state_inference: nonzero_or(
                s.theory_of_mind.mental_state_inference,
                0.5,
            ),
            effective_cue_reading: nonzero_or(s.social_perception.social_cue_interpretation, 0.5),
        }
    }

    /// Couple theory-of-mind/social-perception to real attentional
    /// resources instead of a static schema readout.
    pub fn step(&self, attention: &super::attention::AttentionSnapshot) -> Self {
        let fatigue_penalty = attention.attention_fatigue.clamp(0.0, 1.0);

        let effective_mental_state_inference =
            (self.mental_state_inference * attention.available.max(0.1)).clamp(0.0, 1.0);
        let effective_cue_reading =
            (self.social_cue_interpretation * (1.0 - fatigue_penalty * 0.5)).clamp(0.0, 1.0);

        Self {
            effective_mental_state_inference,
            effective_cue_reading,
            ..self.clone()
        }
    }

    fn defaults() -> Self {
        Self {
            mental_state_inference: 0.5,
            intention_recognition: 0.5,
            perspective_taking: 0.5,
            false_belief_understanding: 0.5,
            emotion_recognition: 0.5,
            social_cue_interpretation: 0.5,
            trust_assessment: 0.5,
            conflict_resolution: 0.5,
            cooperation_tendency: 0.5,
            empathy_level: 0.5,
            social_anxiety: 0.3,
            effective_mental_state_inference: 0.5,
            effective_cue_reading: 0.5,
        }
    }
}

fn nonzero_or(value: f32, default: f64) -> f64 {
    if value == 0.0 {
        default
    } else {
        value as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::humans::attention::AttentionSnapshot;
    use mk_core::human::{HumanId, HumanSchema};

    fn profile() -> HumanProfile {
        let schema = HumanSchema::canonical_minimal("social_cognition_test");
        HumanProfile::from_canonical_schema(HumanId::new(1), schema)
    }

    #[test]
    fn from_profile_uses_sane_defaults() {
        let snapshot = SocialCognitionSnapshot::from_profile(&profile());
        assert!(snapshot.mental_state_inference > 0.0);
        assert!(snapshot.empathy_level > 0.0);
    }

    #[test]
    fn fatigue_degrades_social_perception() {
        let snapshot = SocialCognitionSnapshot::from_profile(&profile());
        let mut fatigued = AttentionSnapshot::from_profile(&profile());
        fatigued.attention_fatigue = 1.0;
        fatigued.available = 0.1;

        let rested = AttentionSnapshot::from_profile(&profile());

        let stepped_fatigued = snapshot.step(&fatigued);
        let stepped_rested = snapshot.step(&rested);

        assert!(
            stepped_fatigued.effective_mental_state_inference
                <= stepped_rested.effective_mental_state_inference
        );
        assert!(stepped_fatigued.effective_cue_reading <= stepped_rested.effective_cue_reading);
    }
}
