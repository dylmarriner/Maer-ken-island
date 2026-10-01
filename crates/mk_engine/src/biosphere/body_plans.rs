/// Body plans for Phase 3
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BodyPlan {
    pub locomotion_mode: LocomotionMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LocomotionMode {
    Aquatic,
    Terrestrial,
    Amphibious,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentalConstraints {
    pub temperature_range: (f64, f64),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EnvironmentType {
    Marine,
    Terrestrial,
}
