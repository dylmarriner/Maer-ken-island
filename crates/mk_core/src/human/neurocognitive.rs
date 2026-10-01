//! Neurocognitive Profile Module
//!
//! Defines cognitive processing characteristics (0.0 - 1.0 scale)

use serde::{Deserialize, Serialize};

/// Sensory sensitivity profile
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SensorySensitivity {
    pub audio: Option<f32>,
    pub visual: Option<f32>,
    pub tactile: Option<f32>,
}

/// Complete neurocognitive processing profile
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NeurocognitiveProfile {
    /// Variability in ability to regulate attention
    pub attention_regulation_variability: f32,

    /// Probability of entering hyperfocus state
    pub hyperfocus_probability: f32,

    /// Cognitive cost of initiating tasks
    pub task_initiation_cost: f32,

    /// How completion quality decays over time/effort
    pub task_completion_decay: f32,

    /// Cost of switching between tasks
    pub task_switching_cost: Option<f32>,

    /// Tendency toward associative (tangential) thinking
    pub associative_thinking_bias: f32,

    /// How much sensory/emotional input affects cognition
    pub sensory_emotional_permeability: f32,

    /// Latency in detecting social boundary violations
    pub social_boundary_detection_latency: f32,

    /// Rate of executive function fatigue
    pub executive_function_fatigue_rate: f32,

    /// Threshold before emotional overload occurs
    pub emotional_overload_threshold: f32,

    /// Recovery time after conflict or intense interaction
    pub recovery_time_after_fusion_or_conflict: String,

    /// Sensory sensitivity across modalities
    pub sensory_sensitivity: Option<SensorySensitivity>,

    /// Latency in decoding social signals
    pub social_signal_decoding_latency: Option<f32>,
}

impl Default for NeurocognitiveProfile {
    fn default() -> Self {
        Self {
            attention_regulation_variability: 0.5,
            hyperfocus_probability: 0.5,
            task_initiation_cost: 0.5,
            task_completion_decay: 0.5,
            task_switching_cost: None,
            associative_thinking_bias: 0.5,
            sensory_emotional_permeability: 0.5,
            social_boundary_detection_latency: 0.5,
            executive_function_fatigue_rate: 0.5,
            emotional_overload_threshold: 0.5,
            recovery_time_after_fusion_or_conflict: "2-4 hours".to_string(),
            sensory_sensitivity: None,
            social_signal_decoding_latency: None,
        }
    }
}

impl NeurocognitiveProfile {
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if attention is variable (likely ADHD indicator)
    pub fn has_variable_attention(&self) -> bool {
        self.attention_regulation_variability > 0.6
    }

    /// Check if prone to hyperfocus
    pub fn is_hyperfocus_prone(&self) -> bool {
        self.hyperfocus_probability > 0.6
    }

    /// Check if executive function cost is high
    pub fn has_high_executive_cost(&self) -> bool {
        self.task_initiation_cost > 0.6 || self.task_switching_cost.unwrap_or(0.0) > 0.6
    }

    /// Check if emotionally permeable (easily affected by environment)
    pub fn is_emotionally_permeable(&self) -> bool {
        self.sensory_emotional_permeability > 0.6
    }

    /// Check if neurodiverse indicators present
    pub fn shows_neurodiversity_traits(&self) -> bool {
        self.has_variable_attention()
            || self.has_high_executive_cost()
            || self.is_emotionally_permeable()
    }

    /// Estimate if cognitive style is detail-focused
    pub fn is_detail_focused(&self) -> bool {
        self.associative_thinking_bias < 0.4
    }

    /// Estimate if cognitive style is big-picture
    pub fn is_big_picture(&self) -> bool {
        self.associative_thinking_bias > 0.6
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_profile_is_neutral() {
        let profile = NeurocognitiveProfile::default();
        assert_eq!(profile.attention_regulation_variability, 0.5);
        assert_eq!(profile.hyperfocus_probability, 0.5);
    }

    #[test]
    fn attention_variability_detection() {
        let variable = NeurocognitiveProfile {
            attention_regulation_variability: 0.75,
            ..Default::default()
        };
        assert!(variable.has_variable_attention());

        let stable = NeurocognitiveProfile {
            attention_regulation_variability: 0.35,
            ..Default::default()
        };
        assert!(!stable.has_variable_attention());
    }

    #[test]
    fn hyperfocus_detection() {
        let hyperfocus_prone = NeurocognitiveProfile {
            hyperfocus_probability: 0.8,
            ..Default::default()
        };
        assert!(hyperfocus_prone.is_hyperfocus_prone());
    }

    #[test]
    fn neurodiversity_trait_detection() {
        let neurotypical = NeurocognitiveProfile::default();
        assert!(!neurotypical.shows_neurodiversity_traits());

        let neurodivergent = NeurocognitiveProfile {
            attention_regulation_variability: 0.75,
            ..Default::default()
        };
        assert!(neurodivergent.shows_neurodiversity_traits());
    }

    #[test]
    fn cognitive_style_detection() {
        let detail_focused = NeurocognitiveProfile {
            associative_thinking_bias: 0.2,
            ..Default::default()
        };
        assert!(detail_focused.is_detail_focused());

        let big_picture = NeurocognitiveProfile {
            associative_thinking_bias: 0.8,
            ..Default::default()
        };
        assert!(big_picture.is_big_picture());
    }
}
