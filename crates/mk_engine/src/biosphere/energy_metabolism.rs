/// Energy metabolism for Phase 3
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnergyMetabolism {
    pub trophic_level: TrophicLevel,
    pub activity_state: ActivityState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TrophicLevel {
    PrimaryConsumer,
    SecondaryConsumer,
    TertiaryConsumer,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActivityState {
    Resting,
    Active,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EnergyStatus {
    Sufficient,
    Deficient,
}
