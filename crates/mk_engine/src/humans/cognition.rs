//! Cognition Snapshot - Runtime cognitive state derived from profile

use mk_core::human::HumanProfile;
use serde::{Deserialize, Serialize};

/// Cognitive processing mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CognitiveMode {
    /// Analytical, step-by-step processing
    Analytical,
    /// Intuitive, pattern-based processing
    Intuitive,
    /// Creative, divergent thinking
    Creative,
    /// Social, perspective-taking processing
    Social,
    /// Default balanced mode
    Balanced,
}

/// Runtime cognition snapshot derived from HumanProfile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HumanCognitionSnapshot {
    /// Current cognitive processing mode
    pub mode: CognitiveMode,

    /// Attention regulation capacity (0.0 to 1.0)
    pub attention_capacity: f64,

    /// Hyperfocus tendency (0.0 = none, 1.0 = extreme)
    pub hyperfocus_tendency: f64,

    /// Working memory capacity (0.0 to 1.0)
    pub working_memory: f64,

    /// Task initiation ease (0.0 = very difficult, 1.0 = effortless)
    pub task_initiation: f64,

    /// Task completion persistence (0.0 to 1.0)
    pub task_persistence: f64,

    /// Associative thinking strength (0.0 to 1.0)
    pub associative_thinking: f64,

    /// Emotional-cognitive permeability (how much emotions affect thinking)
    pub emotional_permeability: f64,

    /// Executive function fatigue rate (higher = tires faster)
    pub exec_fatigue_rate: f64,

    /// Emotional overload threshold (higher = more resilient)
    pub overload_threshold: f64,

    /// Current cognitive load (0.0 to 1.0, runtime state)
    pub cognitive_load: f64,

    /// Current fatigue level (0.0 to 1.0, runtime state)
    pub fatigue: f64,
}

impl HumanCognitionSnapshot {
    /// Derive cognition snapshot from a HumanProfile
    pub fn from_profile(profile: &HumanProfile) -> Self {
        let neuro = &profile.neurocognitive_profile;
        let temperament = &profile.temperament_matrix;

        // Determine dominant cognitive mode from temperament
        let mode = if temperament.openness_to_experience > 0.7 && temperament.empathy < 0.4 {
            CognitiveMode::Creative
        } else if temperament.empathy > 0.6 {
            CognitiveMode::Social
        } else if neuro.associative_thinking_bias > 0.6 {
            CognitiveMode::Intuitive
        } else if neuro.attention_regulation_variability < 0.3 {
            CognitiveMode::Analytical
        } else {
            CognitiveMode::Balanced
        };

        Self {
            mode,
            attention_capacity: 1.0 - neuro.attention_regulation_variability as f64,
            hyperfocus_tendency: neuro.hyperfocus_probability as f64,
            working_memory: (1.0 - neuro.task_completion_decay) as f64,
            task_initiation: (1.0 - neuro.task_initiation_cost) as f64,
            task_persistence: (1.0 - neuro.task_completion_decay) as f64,
            associative_thinking: neuro.associative_thinking_bias as f64,
            emotional_permeability: neuro.sensory_emotional_permeability as f64,
            exec_fatigue_rate: neuro.executive_function_fatigue_rate as f64,
            overload_threshold: neuro.emotional_overload_threshold as f64,
            cognitive_load: 0.0,
            fatigue: 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cognition_from_profile_has_valid_values() {
        let schema = mk_core::human::HumanSchema::canonical_minimal("test");
        let profile = mk_core::human::HumanProfile::from_canonical_schema(
            mk_core::human::HumanId::new(1),
            schema,
        );
        let cog = HumanCognitionSnapshot::from_profile(&profile);
        assert!(cog.attention_capacity >= 0.0 && cog.attention_capacity <= 1.0);
        assert!(cog.working_memory >= 0.0);
    }
}
