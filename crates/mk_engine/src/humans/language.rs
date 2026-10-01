//! Language Snapshot - Communication and language capabilities

use mk_core::human::HumanProfile;
use serde::{Deserialize, Serialize};

/// Language processing mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LanguageMode {
    /// Verbal/linguistic processing
    Verbal,
    /// Visual-spatial processing
    Visual,
    /// Kinesthetic/body-based processing
    Kinesthetic,
    /// Mixed modality
    Mixed,
}

/// Runtime language snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageSnapshot {
    /// Current language processing mode
    pub mode: LanguageMode,

    /// Verbal expressivity (0.0 = nonverbal, 1.0 = highly articulate)
    pub expressivity: f64,

    /// Comprehension capacity (0.0 to 1.0)
    pub comprehension: f64,

    /// Pragmatic language skill (social context awareness in speech)
    pub pragmatic_skill: f64,

    /// Vocabulary richness (0.0 to 1.0)
    pub vocabulary_richness: f64,

    /// Narrative ability (storytelling, explanation)
    pub narrative_ability: f64,

    /// Nonverbal communication reading ability
    pub nonverbal_reading: f64,

    /// Second language acquisition ease
    pub acquisition_ease: f64,
}

impl LanguageSnapshot {
    /// Derive language snapshot from a HumanProfile
    pub fn from_profile(profile: &HumanProfile) -> Self {
        let temperament = &profile.temperament_matrix;
        let neuro = &profile.neurocognitive_profile;
        let drives = &profile.drive_weights;

        let extraversion = temperament.introversion_extroversion as f64;
        let openness = temperament.openness_to_experience as f64;
        let empathy = temperament.empathy as f64;

        // Determine language mode from neurotype
        let mode = if profile.core_identity.neurotype.autism.is_some() {
            if neuro
                .sensory_sensitivity
                .as_ref()
                .and_then(|s| s.visual)
                .unwrap_or(0.5)
                > 0.6
            {
                LanguageMode::Visual
            } else {
                LanguageMode::Mixed
            }
        } else {
            LanguageMode::Verbal
        };

        // Expressivity driven by extraversion and empathy
        let expressivity = (extraversion * 0.5 + empathy * 0.3 + openness * 0.2).clamp(0.0, 1.0);

        // Comprehension driven by openness and cognitive capacity
        let comprehension = (openness * 0.4
            + (1.0 - neuro.attention_regulation_variability as f64) * 0.6)
            .clamp(0.0, 1.0);

        Self {
            mode,
            expressivity,
            comprehension,
            pragmatic_skill: empathy * 0.6 + extraversion * 0.4,
            vocabulary_richness: openness * 0.7 + drives.curiosity as f64 * 0.3,
            narrative_ability: openness * 0.4 + extraversion * 0.3 + empathy * 0.3,
            nonverbal_reading: empathy * 0.7
                + (1.0 - neuro.sensory_emotional_permeability as f64) * 0.3,
            acquisition_ease: openness * 0.5 + drives.curiosity as f64 * 0.5,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn language_snapshot_has_expressivity() {
        let schema = mk_core::human::HumanSchema::canonical_minimal("test");
        let profile = mk_core::human::HumanProfile::from_canonical_schema(
            mk_core::human::HumanId::new(1),
            schema,
        );
        let lang = LanguageSnapshot::from_profile(&profile);
        assert!(lang.expressivity >= 0.0 && lang.expressivity <= 1.0);
    }
}
