//! Core Identity Module
//!
//! Defines fundamental identity characteristics of a human

use super::{BiologicalSex, Generation, Locality};
use serde::{Deserialize, Serialize};

/// Geographic coordinates
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Coordinates {
    pub latitude: f64,
    pub longitude: f64,
}

/// Birthplace information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Birthplace {
    pub location: String,
    pub coordinates: Coordinates,
    pub locality: Locality,
}

/// ADHD attention profile (0.0 - 1.0 scale)
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AttentionalProfile {
    pub sustained_attention: Option<f32>,
    pub selective_attention: Option<f32>,
    pub divided_attention: Option<f32>,
    pub alternating_attention: Option<f32>,
}

/// ADHD hyperactivity profile (0.0 - 1.0 scale)
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct HyperactivityProfile {
    pub motor_hyperactivity: Option<f32>,
    pub verbal_hyperactivity: Option<f32>,
    pub mental_hyperactivity: Option<f32>,
    pub impulsivity_level: Option<f32>,
}

/// Executive functioning characteristics (0.0 - 1.0 scale)
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ExecutiveFunctioning {
    pub working_memory: Option<f32>,
    pub planning_organizing: Option<f32>,
    pub time_management: Option<f32>,
    pub emotional_regulation: Option<f32>,
    pub task_initiation: Option<f32>,
    pub task_completion: Option<f32>,
}

/// Circadian rhythm characteristics
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Chronotype {
    Morning,
    Evening,
    Intermediate,
}

/// Sleep and circadian profile (0.0 - 1.0 scale)
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CircadianRhythm {
    pub chronotype: Option<Chronotype>,
    pub sleep_onset_difficulty: Option<f32>,
    pub sleep_maintenance: Option<f32>,
    pub daytime_somnolence: Option<f32>,
}

/// Comorbidity patterns (0.0 - 1.0 scale)
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ComorbidityPatterns {
    pub anxiety_level: Option<f32>,
    pub depression_level: Option<f32>,
    pub emotional_dysregulation: Option<f32>,
    pub rejection_sensitivity: Option<f32>,
}

/// ADHD subtype classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ADHDSubtype {
    InattentivePresentation,
    CombinedPresentation,
    HyperactiveImpulsive,
}

/// ADHD profile (optional neurodivergent trait)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ADHDProfile {
    pub subtype: Option<ADHDSubtype>,
    pub attentional_profile: Option<AttentionalProfile>,
    pub hyperactivity_profile: Option<HyperactivityProfile>,
    pub executive_functioning: Option<ExecutiveFunctioning>,
    pub circadian_rhythm: Option<CircadianRhythm>,
    pub comorbidity_patterns: Option<ComorbidityPatterns>,
}

/// Autism spectrum level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AutismLevel {
    Level1HighFunctioning,
    Level2RequiringSupport,
    Level3RequiringVerySubstantialSupport,
}

/// Sensory hypersensitivity profile (0.0 - 1.0 scale)
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Hypersensitivity {
    pub auditory: Option<f32>,
    pub visual: Option<f32>,
    pub tactile: Option<f32>,
    pub proprioceptive: Option<f32>,
    pub vestibular: Option<f32>,
    pub interoceptive: Option<f32>,
    pub olfactory: Option<f32>,
    pub gustatory: Option<f32>,
}

/// Sensory hyposensitivity profile (0.0 - 1.0 scale)
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Hyposensitivity {
    pub auditory: Option<f32>,
    pub visual: Option<f32>,
    pub tactile: Option<f32>,
    pub proprioceptive: Option<f32>,
    pub vestibular: Option<f32>,
    pub interoceptive: Option<f32>,
    pub olfactory: Option<f32>,
    pub gustatory: Option<f32>,
}

/// Sensory seeking behavior (0.0 - 1.0 scale)
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SeekingProfile {
    pub proprioceptive_seeking: Option<f32>,
    pub vestibular_seeking: Option<f32>,
    pub tactile_seeking: Option<f32>,
    pub oral_seeking: Option<f32>,
}

/// Sensory processing characteristics (0.0 - 1.0 scale)
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SensoryProcessing {
    pub hypersensitivity: Option<Hypersensitivity>,
    pub hyposensitivity: Option<Hyposensitivity>,
    pub sensory_seeking: Option<SeekingProfile>,
}

/// Restricted and repetitive behaviors (0.0 - 1.0 scale)
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RestrictedInterests {
    pub intensity: Option<f32>,
    pub breadth: Option<f32>,
    pub flexibility: Option<f32>,
    pub knowledge_depth: Option<f32>,
}

/// Restricted repetitive behavior profile
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RestrictedRepetitiveBehaviors {
    pub stereotyped_movements: Option<f32>,
    pub ritualistic_behavior: Option<f32>,
    pub restricted_interests: Option<RestrictedInterests>,
    pub sensory_regulation_needs: Option<f32>,
}

/// Autism executive functioning (0.0 - 1.0 scale)
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AutismExecutiveFunctioning {
    pub cognitive_flexibility: Option<f32>,
    pub planning_sequencing: Option<f32>,
    pub working_memory: Option<f32>,
    pub inhibition_control: Option<f32>,
    pub abstract_thinking: Option<f32>,
}

/// Autism information processing (0.0 - 1.0 scale)
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct InformationProcessing {
    pub detail_focus: Option<f32>,
    pub pattern_recognition: Option<f32>,
    pub system_thinking: Option<f32>,
    pub visual_processing: Option<f32>,
    pub auditory_processing: Option<f32>,
}

/// Autism emotional processing (0.0 - 1.0 scale)
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AutismEmotionalProcessing {
    pub emotional_identification: Option<f32>,
    pub emotional_regulation: Option<f32>,
    pub alexithymia_tendency: Option<f32>,
    pub emotional_intensity: Option<f32>,
}

/// Social communication characteristics (0.0 - 1.0 scale)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CommunicationStyle {
    Direct,
    Formal,
    Literal,
    NonverbalPreferenced,
}

/// Autism social communication profile (0.0 - 1.0 scale)
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SocialCommunication {
    pub social_recognition: Option<f32>,
    pub social_motivation: Option<f32>,
    pub social_anxiety: Option<f32>,
    pub communication_style: Option<CommunicationStyle>,
    pub nonverbal_communication: Option<f32>,
    pub pragmatic_language: Option<f32>,
}

/// Autism profile (optional neurodivergent trait)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AutismProfile {
    pub level: Option<AutismLevel>,
    pub social_communication: Option<SocialCommunication>,
    pub sensory_processing: Option<SensoryProcessing>,
    pub restricted_repetitive_behaviors: Option<RestrictedRepetitiveBehaviors>,
    pub executive_functioning: Option<AutismExecutiveFunctioning>,
    pub information_processing: Option<InformationProcessing>,
    pub emotional_processing: Option<AutismEmotionalProcessing>,
}

/// Complete neurotype definition
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Neurotype {
    pub adhd: Option<ADHDProfile>,
    pub autism: Option<AutismProfile>,
    pub sensory_processing_sensitivity: Option<bool>,
    pub executive_dysfunction_bias: Option<String>,
}

/// Core identity of a human
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CoreIdentity {
    pub agent_id: String,
    pub biological_sex: BiologicalSex,
    pub birth_timestamp: String,
    pub birthplace: Birthplace,
    pub neurotype: Neurotype,
    pub generation: Generation,
}

impl CoreIdentity {
    pub fn new(
        agent_id: String,
        biological_sex: BiologicalSex,
        birth_timestamp: String,
        birthplace: Birthplace,
        generation: Generation,
    ) -> Self {
        Self {
            agent_id,
            biological_sex,
            birth_timestamp,
            birthplace,
            neurotype: Neurotype {
                adhd: None,
                autism: None,
                sensory_processing_sensitivity: None,
                executive_dysfunction_bias: None,
            },
            generation,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_basic_identity() {
        let birthplace = Birthplace {
            location: "Test City".to_string(),
            coordinates: Coordinates {
                latitude: 45.0,
                longitude: -120.0,
            },
            locality: Locality::Urban,
        };

        let identity = CoreIdentity::new(
            "HUM-001".to_string(),
            BiologicalSex::Male,
            "2090-01-01T00:00:00Z".to_string(),
            birthplace,
            Generation::First,
        );

        assert_eq!(identity.agent_id, "HUM-001");
        assert_eq!(identity.biological_sex, BiologicalSex::Male);
        assert_eq!(identity.generation, Generation::First);
    }
}
