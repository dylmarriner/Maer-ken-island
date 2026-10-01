/// Nervous system for Phase 3
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeuralState {
    pub arousal_level: f64,
    pub processing_load: f64,
    pub sensory_buffer: Vec<SensoryInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SensoryInput {
    Visual { intensity: f64 },
    Auditory { intensity: f64 },
    Pressure { intensity: f64 },
    Chemical { intensity: f64 },
    Temperature { intensity: f64 },
    Electromagnetic { intensity: f64 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReflexResponse {
    pub action_type: String,
}
