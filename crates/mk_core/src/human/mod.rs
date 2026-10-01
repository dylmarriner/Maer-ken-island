//! Human Profile Definition Module
//!
//! Defines the complete structure for human being profiles in Maer'Ken
//! Based on HumanReplicationSchema specifications
//!
//! Humans are data-driven entities - all personality, neurology, and behavioral
//! characteristics are stored as configuration data, completely separate from
//! any execution engine.

use serde::{Deserialize, Serialize};

pub mod astrology;
pub mod attachment;
pub mod drives;
pub mod emotional;
pub mod genetics;
pub mod hormonal;
pub mod identity;
pub mod identity_axioms;
pub mod individual;
pub mod motivations;
pub mod neurocognitive;
pub mod personality_traits;
pub mod profile;
pub mod relational;
pub mod schema;
pub mod skills;
pub mod stress_response;
pub mod temperament;
pub mod trait_evolution;

pub use attachment::AttachmentStyle;
pub use drives::DriveWeights;
pub use emotional::EmotionalBaseline;
pub use genetics::{
    conceive, create_gamete, AstrologicalProfile, Chromosome, DnaStrand, ExpressionMode,
    ExpressionWeights, Gamete, Genome, SexChromosomePair, Zodiac,
};
pub use hormonal::HormonalBaselineBias;
pub use identity::{
    ADHDProfile, ADHDSubtype, AutismLevel, AutismProfile, Birthplace, Chronotype,
    CommunicationStyle, Coordinates, CoreIdentity, Neurotype,
};
pub use identity_axioms::IdentityAxioms;
pub use motivations::{BehavioralTendencies, GoalsAndMotivations};
pub use neurocognitive::{NeurocognitiveProfile, SensorySensitivity};
pub use personality_traits::{PersonalityTrait, PersonalityTraits};
pub use profile::HumanProfile;
pub use relational::RelationalDefaults;
pub use schema::{HumanIdentitySchema, HumanSchema};
pub use skills::SkillMatrix;
pub use stress_response::StressResponseProfile;
pub use temperament::TemperamentMatrix;
pub use trait_evolution::{
    GrowthProfile, ModificationEventType, TraitEvolution, TraitModification, TraumaProfile,
};

/// Top-level lifecycle state for a human profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HumanStatus {
    Alive,
    Dead,
    Dormant,
}

/// Unique identifier for a human
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct HumanId(pub u64);

impl HumanId {
    pub fn new(id: u64) -> Self {
        HumanId(id)
    }
}

impl std::fmt::Display for HumanId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "HUM-{:06}", self.0)
    }
}

/// Generation of human (used for lineage tracking)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Generation {
    First,
    Second,
    Third,
    Later(u8),
}

impl From<u8> for Generation {
    fn from(n: u8) -> Self {
        match n {
            1 => Generation::First,
            2 => Generation::Second,
            3 => Generation::Third,
            n => Generation::Later(n),
        }
    }
}

/// Biological sex designation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BiologicalSex {
    Male,
    Female,
    Neutral,
}

/// Locality type (urban density)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Locality {
    Urban,
    Suburban,
    Rural,
    SemiRural,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn human_id_displays_correctly() {
        let id = HumanId(1);
        assert_eq!(id.to_string(), "HUM-000001");
    }

    #[test]
    fn generation_from_u8() {
        assert_eq!(Generation::from(1), Generation::First);
        assert_eq!(Generation::from(2), Generation::Second);
        assert_eq!(Generation::from(3), Generation::Third);
        assert_eq!(Generation::from(5), Generation::Later(5));
    }
}
