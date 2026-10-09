/// Phase 9 Post-MK-I Prep (No Activation)
///
/// Purpose: Prepare future organism detail without contaminating MK-I.
/// Schemas only. No ticking. No memory. No agency.
pub mod anatomy;
pub mod development;
pub mod dimensions;
pub mod metabolism;
pub mod nervous_systems;
pub mod property;
pub mod reproduction;
pub mod runtime;
pub mod settlements;
pub mod vegetation;

// Re-export key schema types
pub use anatomy::{AnatomySchema, BodyPlan, OrganSystem, TissueType};
pub use development::{DevelopmentSchema, DevelopmentalStage, GrowthPattern, MaturationProcess};
pub use metabolism::{EnergyBudget, MetabolicPathway, MetabolismSchema, Thermoregulation};
pub use nervous_systems::{
    CognitiveLevel, NervousSystemSchema, NeuralArchitecture, SensoryModality,
};
pub use property::{
    PropertyBuilding, PropertyBuildingKind, PropertyItem, PropertyItemKind, PropertySystem,
    StarterProperty,
};
pub use reproduction::{LifeCycle, MatingSystem, ReproductionSchema, ReproductiveStrategy};
pub use runtime::{LifecycleState, MovementClass, OrganismBeing, OrganismSystem, OrganismTerrain};
pub use settlements::{SettlementSystem, SiteBeing, SiteKind};
pub use vegetation::{PlantBeing, PlantKind, VegetationSystem};

/// Organism schema collection
///
/// This module contains only schemas and type definitions.
/// No executable logic, no state, no agency.
/// These schemas are for future MK-II implementation only.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OrganismSchema {
    pub anatomy: AnatomySchema,
    pub nervous_system: NervousSystemSchema,
    pub metabolism: MetabolismSchema,
    pub reproduction: ReproductionSchema,
    pub development: DevelopmentSchema,
}

impl OrganismSchema {
    /// Create a new organism schema
    ///
    /// This is a pure data constructor - no behavior, no state changes.
    pub fn new(
        anatomy: AnatomySchema,
        nervous_system: NervousSystemSchema,
        metabolism: MetabolismSchema,
        reproduction: ReproductionSchema,
        development: DevelopmentSchema,
    ) -> Self {
        Self {
            anatomy,
            nervous_system,
            metabolism,
            reproduction,
            development,
        }
    }

    /// Validate schema consistency
    ///
    /// Pure validation function - no side effects, no state mutation.
    pub fn validate(&self) -> Result<(), SchemaValidationError> {
        // Validate anatomy-nervous system compatibility
        self.anatomy
            .validate_nervous_compatibility(&self.nervous_system)?;

        // Validate metabolism-energy budget consistency
        self.metabolism.validate_energy_consistency()?;

        // Validate reproduction-development alignment
        self.reproduction
            .validate_development_alignment(&self.development)?;

        // Validate nervous system cognitive level constraints
        self.nervous_system.validate_cognitive_constraints()?;

        Ok(())
    }
}

/// Schema validation error
#[derive(Debug, Clone)]
pub enum SchemaValidationError {
    AnatomyIncompatible(String),
    MetabolismInconsistent(String),
    ReproductionMisaligned(String),
    NervousSystemViolation(String),
    DevelopmentConstraint(String),
}

impl std::fmt::Display for SchemaValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SchemaValidationError::AnatomyIncompatible(msg) => {
                write!(f, "Anatomy incompatible: {}", msg)
            }
            SchemaValidationError::MetabolismInconsistent(msg) => {
                write!(f, "Metabolism inconsistent: {}", msg)
            }
            SchemaValidationError::ReproductionMisaligned(msg) => {
                write!(f, "Reproduction misaligned: {}", msg)
            }
            SchemaValidationError::NervousSystemViolation(msg) => {
                write!(f, "Nervous system violation: {}", msg)
            }
            SchemaValidationError::DevelopmentConstraint(msg) => {
                write!(f, "Development constraint: {}", msg)
            }
        }
    }
}

impl std::error::Error for SchemaValidationError {}
