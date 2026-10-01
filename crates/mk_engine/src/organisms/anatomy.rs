/// Organism Anatomy Schema
///
/// Pure schema definitions for future organism anatomy.
/// No executable logic, no state, no agency.
use serde::{Deserialize, Serialize};

/// Anatomy schema for organism structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnatomySchema {
    pub body_plan: BodyPlan,
    pub organ_systems: Vec<OrganSystem>,
    pub tissue_types: Vec<TissueType>,
    pub structural_properties: StructuralProperties,
    pub developmental_constraints: DevelopmentalConstraints,
}

/// Body plan type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BodyPlan {
    /// Bilateral symmetry with head-tail axis
    Bilateral,
    /// Radial symmetry with central axis
    Radial,
    /// Asymmetric body plan
    Asymmetric,
    /// Segmented body plan
    Segmented { segment_count: u16 },
    /// Colonial organization
    Colonial { module_type: String },
}

/// Organ system definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganSystem {
    pub system_type: OrganSystemType,
    pub complexity: ComplexityLevel,
    pub integration_level: IntegrationLevel,
    pub energy_requirement: f64, // Relative to body mass
    pub developmental_timing: DevelopmentalTiming,
}

/// Types of organ systems
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrganSystemType {
    /// Circulatory system for transport
    Circulatory,
    /// Respiratory system for gas exchange
    Respiratory,
    /// Digestive system for nutrient processing
    Digestive,
    /// Nervous system for control and coordination
    Nervous,
    /// Muscular system for movement
    Muscular,
    /// Skeletal system for support
    Skeletal,
    /// Excretory system for waste removal
    Excretory,
    /// Endocrine system for chemical regulation
    Endocrine,
    /// Reproductive system for reproduction
    Reproductive,
    /// Integumentary system (skin, covering)
    Integumentary,
}

/// Complexity level of organ system
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ComplexityLevel {
    /// Simple, minimal differentiation
    Simple,
    /// Moderate complexity with some specialization
    Moderate,
    /// High complexity with extensive specialization
    High,
    /// Very high complexity with advanced features
    VeryHigh,
}

/// Integration level between systems
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum IntegrationLevel {
    /// Systems operate independently
    Independent,
    /// Some coordination between systems
    Coordinated,
    /// Highly integrated with feedback loops
    Integrated,
    /// Fully integrated with shared regulation
    FullyIntegrated,
}

/// Developmental timing for organ system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DevelopmentalTiming {
    pub emergence_stage: String,       // When system appears
    pub maturation_stage: String,      // When system matures
    pub critical_periods: Vec<String>, // Critical development windows
}

/// Tissue type definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TissueType {
    pub tissue_name: String,
    pub tissue_class: TissueClass,
    pub cellular_organization: CellularOrganization,
    pub functional_properties: FunctionalProperties,
    pub regenerative_capacity: RegenerativeCapacity,
}

/// Tissue classification
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TissueClass {
    /// Epithelial tissue (lining, covering)
    Epithelial,
    /// Connective tissue (support, transport)
    Connective,
    /// Muscle tissue (contraction)
    Muscle,
    /// Nervous tissue (signaling)
    Nervous,
    /// Vascular tissue (transport in plants)
    Vascular,
    /// Specialized tissue types
    Specialized(String),
}

/// Cellular organization level
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum CellularOrganization {
    /// Single cell layer
    Simple,
    /// Multiple cell layers
    Stratified,
    /// Multiple cell types
    Heterogeneous,
    /// Complex arrangement with ECM
    Complex,
}

/// Functional properties of tissue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionalProperties {
    pub mechanical_strength: f64, // 0.0 to 1.0
    pub elasticity: f64,          // 0.0 to 1.0
    pub conductivity: f64,        // Electrical/chemical
    pub permeability: f64,        // Transport through tissue
    pub metabolic_rate: f64,      // Relative activity
}

/// Regenerative capacity
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RegenerativeCapacity {
    /// No regeneration
    None,
    /// Limited regeneration
    Limited,
    /// Moderate regeneration
    Moderate,
    /// High regeneration capability
    High,
    /// Complete regeneration
    Complete,
}

/// Structural properties of organism
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuralProperties {
    pub body_size_class: BodySizeClass,
    pub support_structure: SupportStructure,
    pub symmetry_type: SymmetryType,
    pub segmentation: Segmentation,
    pub body_cavity: BodyCavity,
}

/// Body size classification
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BodySizeClass {
    /// Microscopic (< 0.1mm)
    Microscopic,
    /// Very small (0.1mm - 1mm)
    VerySmall,
    /// Small (1mm - 10mm)
    Small,
    /// Medium (10mm - 100mm)
    Medium,
    /// Large (100mm - 1m)
    Large,
    /// Very large (> 1m)
    VeryLarge,
}

/// Type of support structure
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SupportStructure {
    /// No rigid support structure
    Hydrostatic,
    /// External skeleton
    Exoskeleton,
    /// Internal skeleton
    Endoskeleton,
    /// Shell-based support
    Shell,
    /// Cartilage-based support
    Cartilaginous,
}

/// Symmetry type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SymmetryType {
    /// No symmetry
    None,
    /// Bilateral symmetry
    Bilateral,
    /// Radial symmetry
    Radial,
    /// Pentaradial symmetry
    Pentaradial,
    /// Biradial symmetry
    Biradial,
}

/// Segmentation information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Segmentation {
    pub is_segmented: bool,
    pub segment_count: Option<u16>,
    pub segment_specialization: Vec<String>, // Types of specialized segments
    pub segmentation_pattern: SegmentationPattern,
}

/// Segmentation pattern
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SegmentationPattern {
    /// No segmentation
    None,
    /// Homonomous segments (similar)
    Homonomous,
    /// Heteronomous segments (different)
    Heteronomous,
    /// Tagmatization (specialized groups)
    Tagmatization,
}

/// Body cavity type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BodyCavity {
    /// No body cavity
    Acoelomate,
    /// Pseudocoelom (partial cavity)
    Pseudocoelomate,
    /// True coelom (complete cavity)
    Coelomate,
}

/// Developmental constraints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DevelopmentalConstraints {
    pub embryonic_pattern: EmbryonicPattern,
    pub growth_type: GrowthType,
    pub molting_required: bool,
    pub metamorphosis_possible: bool,
    pub environmental_constraints: Vec<EnvironmentalConstraint>,
}

/// Embryonic development pattern
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EmbryonicPattern {
    /// No distinct embryonic stage
    None,
    /// Simple embryonic development
    Simple,
    /// Protostome development
    Protostome,
    /// Deuterostome development
    Deuterostome,
}

/// Growth type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GrowthType {
    /// Determinate growth (stops at adult size)
    Determinate,
    /// Indeterminate growth (continues throughout life)
    Indeterminate,
    /// Periodic growth with molts
    Molting,
}

/// Environmental constraint on development
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentalConstraint {
    pub constraint_type: String,
    pub critical_threshold: f64,
    pub tolerance_range: (f64, f64),
    pub developmental_stage: String,
}

impl AnatomySchema {
    /// Create a new anatomy schema
    pub fn new(
        body_plan: BodyPlan,
        organ_systems: Vec<OrganSystem>,
        tissue_types: Vec<TissueType>,
        structural_properties: StructuralProperties,
        developmental_constraints: DevelopmentalConstraints,
    ) -> Self {
        Self {
            body_plan,
            organ_systems,
            tissue_types,
            structural_properties,
            developmental_constraints,
        }
    }

    /// Validate nervous system compatibility
    ///
    /// Pure validation function - no side effects
    pub fn validate_nervous_compatibility(
        &self,
        nervous_schema: &crate::organisms::nervous_systems::NervousSystemSchema,
    ) -> Result<(), crate::organisms::SchemaValidationError> {
        // Check if nervous system complexity matches anatomical complexity
        let nervous_complexity = nervous_schema.architecture.complexity_level();
        let anatomical_complexity = self.calculate_anatomical_complexity();

        if nervous_complexity > anatomical_complexity + 1 {
            return Err(
                crate::organisms::SchemaValidationError::AnatomyIncompatible(
                    "Nervous system complexity exceeds anatomical support".to_string(),
                ),
            );
        }

        // Check for required organ systems
        if nervous_schema.architecture.requires_circulatory_support()
            && !self.has_organ_system(&crate::organisms::anatomy::OrganSystemType::Circulatory)
        {
            return Err(
                crate::organisms::SchemaValidationError::AnatomyIncompatible(
                    "Nervous system requires circulatory support but none present".to_string(),
                ),
            );
        }

        Ok(())
    }

    /// Calculate overall anatomical complexity
    pub fn calculate_anatomical_complexity(&self) -> u8 {
        let organ_complexity: u8 = self
            .organ_systems
            .iter()
            .map(|os| os.complexity.clone() as u8)
            .sum();

        let tissue_complexity: u8 = self
            .tissue_types
            .iter()
            .map(|t| t.cellular_organization.clone() as u8)
            .sum();

        (organ_complexity + tissue_complexity) / self.organ_systems.len().max(1) as u8
    }

    /// Check if organism has specific organ system
    pub fn has_organ_system(&self, system_type: &OrganSystemType) -> bool {
        self.organ_systems
            .iter()
            .any(|os| os.system_type == *system_type)
    }

    /// Get organ systems by complexity level
    pub fn get_systems_by_complexity(&self, complexity: ComplexityLevel) -> Vec<&OrganSystem> {
        self.organ_systems
            .iter()
            .filter(|os| os.complexity == complexity)
            .collect()
    }

    /// Get tissue types by class
    pub fn get_tissues_by_class(&self, tissue_class: &TissueClass) -> Vec<&TissueType> {
        self.tissue_types
            .iter()
            .filter(|t| t.tissue_class == *tissue_class)
            .collect()
    }
}

impl Default for AnatomySchema {
    fn default() -> Self {
        Self {
            body_plan: BodyPlan::Bilateral,
            organ_systems: vec![OrganSystem {
                system_type: OrganSystemType::Nervous,
                complexity: ComplexityLevel::Moderate,
                integration_level: IntegrationLevel::Integrated,
                energy_requirement: 0.2,
                developmental_timing: DevelopmentalTiming {
                    emergence_stage: "early".to_string(),
                    maturation_stage: "late".to_string(),
                    critical_periods: vec!["neural_tube".to_string()],
                },
            }],
            tissue_types: vec![TissueType {
                tissue_name: "epithelial".to_string(),
                tissue_class: TissueClass::Epithelial,
                cellular_organization: CellularOrganization::Simple,
                functional_properties: FunctionalProperties {
                    mechanical_strength: 0.3,
                    elasticity: 0.5,
                    conductivity: 0.1,
                    permeability: 0.7,
                    metabolic_rate: 0.4,
                },
                regenerative_capacity: RegenerativeCapacity::Moderate,
            }],
            structural_properties: StructuralProperties {
                body_size_class: BodySizeClass::Medium,
                support_structure: SupportStructure::Endoskeleton,
                symmetry_type: SymmetryType::Bilateral,
                segmentation: Segmentation {
                    is_segmented: false,
                    segment_count: None,
                    segment_specialization: vec![],
                    segmentation_pattern: SegmentationPattern::None,
                },
                body_cavity: BodyCavity::Coelomate,
            },
            developmental_constraints: DevelopmentalConstraints {
                embryonic_pattern: EmbryonicPattern::Deuterostome,
                growth_type: GrowthType::Determinate,
                molting_required: false,
                metamorphosis_possible: false,
                environmental_constraints: vec![],
            },
        }
    }
}
