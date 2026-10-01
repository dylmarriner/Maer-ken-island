/// Development Schema
///
/// Pure schema definitions for future organism development.
/// No executable logic, no state, no agency.
use serde::{Deserialize, Serialize};

/// Development schema for organism developmental processes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DevelopmentSchema {
    pub developmental_stages: Vec<DevelopmentalStage>,
    pub growth_pattern: GrowthPattern,
    pub maturation_process: MaturationProcess,
    pub plasticity: DevelopmentalPlasticity,
    pub critical_periods: Vec<CriticalPeriod>,
}

/// Developmental stage definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DevelopmentalStage {
    pub stage_name: String,
    pub stage_type: StageType,
    pub duration_proportion: f64, // Proportion of total development
    pub growth_rate: GrowthRate,
    pub differentiation_level: DifferentiationLevel,
    pub vulnerability_factors: Vec<VulnerabilityFactor>,
    pub resource_requirements: DevelopmentalResourceRequirements,
}

/// Types of developmental stages
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StageType {
    /// Embryonic stage
    Embryonic,
    /// Larval stage
    Larval,
    /// Juvenile stage
    Juvenile,
    /// Subadult stage
    Subadult,
    /// Adult stage
    Adult,
    /// Specialized stage
    Specialized(String),
}

/// Growth rate during stage
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum GrowthRate {
    /// No growth
    None,
    /// Very slow growth
    VerySlow,
    /// Slow growth
    Slow,
    /// Moderate growth
    Moderate,
    /// Rapid growth
    Rapid,
    /// Very rapid growth
    VeryRapid,
}

/// Differentiation level
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum DifferentiationLevel {
    /// No differentiation
    None,
    /// Low differentiation
    Low,
    /// Moderate differentiation
    Moderate,
    /// High differentiation
    High,
    /// Complete differentiation
    Complete,
}

/// Vulnerability factors during development
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VulnerabilityFactor {
    pub factor_type: VulnerabilityType,
    pub severity_level: SeverityLevel,
    pub mitigation_strategies: Vec<String>,
}

/// Types of vulnerabilities
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VulnerabilityType {
    /// Environmental stress
    Environmental,
    /// Nutritional deficiency
    Nutritional,
    /// Predation pressure
    Predation,
    /// Disease susceptibility
    Disease,
    /// Genetic abnormalities
    Genetic,
    /// Social stress
    Social,
}

/// Severity levels
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SeverityLevel {
    /// Very low severity
    VeryLow,
    /// Low severity
    Low,
    /// Moderate severity
    Moderate,
    /// High severity
    High,
    /// Very high severity
    VeryHigh,
}

/// Developmental resource requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DevelopmentalResourceRequirements {
    pub energy_requirement: f64,  // Relative to adult BMR
    pub protein_requirement: f64, // Relative to body mass
    pub micronutrient_requirements: Vec<MicronutrientRequirement>,
    pub water_requirement: f64, // Relative to body mass
    pub optimal_conditions: OptimalConditions,
}

/// Micronutrient requirement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MicronutrientRequirement {
    pub nutrient_name: String,
    pub required_amount: f64,
    pub deficiency_symptoms: Vec<String>,
    pub toxicity_threshold: f64,
}

/// Optimal developmental conditions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimalConditions {
    pub temperature_range: (f64, f64), // Celsius
    pub humidity_range: (f64, f64),    // Percentage
    pub photoperiod: Photoperiod,
    pub social_requirements: SocialRequirements,
    pub environmental_stability: EnvironmentalStability,
}

/// Photoperiod requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Photoperiod {
    pub light_hours: f64,
    pub dark_hours: f64,
    pub light_intensity: f64,        // Relative units
    pub light_spectrum: Vec<String>, // Wavelength requirements
}

/// Social requirements during development
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocialRequirements {
    pub social_needed: bool,
    pub group_size_optimal: Option<u16>,
    pub social_learning_required: bool,
    pub parental_contact_needed: bool,
}

/// Environmental stability requirements
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum EnvironmentalStability {
    /// Highly stable environment required
    Stable,
    /// Moderate stability acceptable
    Moderate,
    /// Variable environment tolerated
    Variable,
    /// Highly variable environment required
    HighlyVariable,
}

/// Growth pattern definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrowthPattern {
    pub pattern_type: PatternType,
    pub growth_rate_curve: GrowthRateCurve,
    pub size_scaling: SizeScaling,
    pub allometric_relationships: Vec<AllometricRelationship>,
}

/// Types of growth patterns
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PatternType {
    /// Linear growth
    Linear,
    /// Exponential growth
    Exponential,
    /// Sigmoidal growth (S-curve)
    Sigmoidal,
    /// Stepwise growth
    Stepwise,
    /// Indeterminate growth
    Indeterminate,
    /// Specialized growth pattern
    Specialized(String),
}

/// Growth rate curve
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrowthRateCurve {
    pub initial_rate: f64,
    pub peak_rate: f64,
    pub final_rate: f64,
    pub inflection_point: f64, // Relative to total growth period
}

/// Size scaling relationships
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SizeScaling {
    pub scaling_exponent: f64,  // Allometric exponent
    pub scaling_constant: f64,  // Allometric constant
    pub size_range: (f64, f64), // Min and max sizes
}

/// Allometric relationship between traits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllometricRelationship {
    pub trait_1: String,
    pub trait_2: String,
    pub exponent: f64,
    pub correlation_strength: f64, // 0.0 to 1.0
}

/// Maturation process
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaturationProcess {
    pub maturation_type: MaturationType,
    pub sexual_maturation: SexualMaturation,
    pub skeletal_maturation: SkeletalMaturation,
    pub neural_maturation: NeuralMaturation,
    pub behavioral_maturation: BehavioralMaturation,
}

/// Types of maturation processes
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MaturationType {
    /// Synchronous maturation
    Synchronous,
    /// Asynchronous maturation
    Asynchronous,
    /// Gradual maturation
    Gradual,
    /// Saltatory maturation (jumps)
    Saltatory,
}

/// Sexual maturation details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SexualMaturation {
    pub maturation_trigger: MaturationTrigger,
    pub secondary_sexual_characteristics: Vec<String>,
    pub fertility_onset: FertilityOnset,
    pub hormonal_regulation: HormonalRegulation,
}

/// Maturation triggers
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MaturationTrigger {
    /// Age-based maturation
    Age { threshold: f64 },
    /// Size-based maturation
    Size { threshold: f64 },
    /// Environmental trigger
    Environmental { cues: Vec<String> },
    /// Social trigger
    Social { cues: Vec<String> },
    /// Hormonal trigger
    Hormonal { hormones: Vec<String> },
}

/// Fertility onset patterns
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FertilityOnset {
    /// Immediate fertility
    Immediate,
    /// Delayed fertility
    Delayed { delay_period: f64 },
    /// Gradual onset
    Gradual { onset_period: f64 },
    /// Seasonal fertility
    Seasonal { fertile_periods: Vec<String> },
}

/// Hormonal regulation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HormonalRegulation {
    pub key_hormones: Vec<String>,
    pub feedback_loops: Vec<String>,
    pub circadian_rhythm: bool,
    pub seasonal_rhythm: bool,
}

/// Skeletal maturation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkeletalMaturation {
    pub ossification_pattern: OssificationPattern,
    pub growth_plate_closure: GrowthPlateClosure,
    pub bone_density_development: BoneDensityDevelopment,
}

/// Ossification patterns
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OssificationPattern {
    /// Intramembranous ossification
    Intramembranous,
    /// Endochondral ossification
    Endochondral,
    /// Mixed ossification
    Mixed,
    /// Specialized pattern
    Specialized(String),
}

/// Growth plate closure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrowthPlateClosure {
    pub closure_pattern: ClosurePattern,
    pub closure_timing: ClosureTiming,
    pub asynchronous_closure: bool,
}

/// Closure patterns
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClosurePattern {
    /// Simultaneous closure
    Simultaneous,
    /// Proximal to distal closure
    ProximalDistal,
    /// Distal to proximal closure
    DistalProximal,
    /// Irregular closure
    Irregular,
}

/// Closure timing
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClosureTiming {
    /// Early closure
    Early,
    /// Average closure
    Average,
    /// Late closure
    Late,
    /// Very late closure
    VeryLate,
}

/// Bone density development
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoneDensityDevelopment {
    pub peak_density_age: f64,
    pub density_maintenance_period: f64,
    pub age_related_decline: AgeRelatedDecline,
}

/// Age-related decline patterns
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgeRelatedDecline {
    /// No decline
    None,
    /// Gradual decline
    Gradual,
    /// Rapid decline
    Rapid,
    /// Variable decline
    Variable,
}

/// Neural maturation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeuralMaturation {
    pub brain_development: BrainDevelopment,
    pub myelination_pattern: MyelinationPattern,
    pub synaptic_pruning: SynapticPruning,
    pub critical_learning_periods: Vec<CriticalLearningPeriod>,
}

/// Brain development
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrainDevelopment {
    pub neurogenesis_timing: NeurogenesisTiming,
    pub regional_development: RegionalDevelopment,
    pub connectivity_patterns: Vec<ConnectivityPattern>,
}

/// Neurogenesis timing
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NeurogenesisTiming {
    /// Prenatal only
    Prenatal,
    /// Prenatal and early postnatal
    PrenatalEarlyPostnatal,
    /// Lifelong neurogenesis
    Lifelong,
    /// Regional specific
    Regional { regions: Vec<String> },
}

/// Regional brain development
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegionalDevelopment {
    pub development_sequence: Vec<String>, // Order of region development
    pub critical_periods: Vec<(String, f64, f64)>, // (region, start, end)
    pub plasticity_levels: Vec<(String, PlasticityLevel)>,
}

/// Plasticity levels
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum PlasticityLevel {
    /// No plasticity
    None,
    /// Low plasticity
    Low,
    /// Moderate plasticity
    Moderate,
    /// High plasticity
    High,
    /// Very high plasticity
    VeryHigh,
}

/// Connectivity patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectivityPattern {
    pub connection_type: String,
    pub development_timing: f64,
    pub functional_significance: String,
}

/// Myelination patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MyelinationPattern {
    pub myelination_sequence: Vec<String>, // Order of tract myelination
    pub myelination_rate: MyelinationRate,
    pub regional_differences: Vec<(String, f64)>, // (region, relative myelination)
}

/// Myelination rates
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum MyelinationRate {
    /// Very slow myelination
    VerySlow,
    /// Slow myelination
    Slow,
    /// Moderate myelination
    Moderate,
    /// Fast myelination
    Fast,
    /// Very fast myelination
    VeryFast,
}

/// Synaptic pruning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SynapticPruning {
    pub pruning_pattern: PruningPattern,
    pub pruning_timing: PruningTiming,
    pub activity_dependence: ActivityDependence,
}

/// Pruning patterns
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PruningPattern {
    /// Global pruning
    Global,
    /// Regional pruning
    Regional { regions: Vec<String> },
    /// System-specific pruning
    SystemSpecific { systems: Vec<String> },
    /// Experience-dependent pruning
    ExperienceDependent,
}

/// Pruning timing
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PruningTiming {
    /// Early pruning
    Early,
    /// Adolescent pruning
    Adolescent,
    /// Extended pruning
    Extended,
    /// Multiple pruning periods
    Multiple { periods: Vec<(f64, f64)> },
}

/// Activity dependence
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ActivityDependence {
    /// No activity dependence
    None,
    /// Low activity dependence
    Low,
    /// Moderate activity dependence
    Moderate,
    /// High activity dependence
    High,
    /// Very high activity dependence
    VeryHigh,
}

/// Critical learning periods
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CriticalLearningPeriod {
    pub skill_type: String,
    pub period_start: f64,
    pub period_end: f64,
    pub optimal_conditions: Vec<String>,
    pub closure_mechanism: ClosureMechanism,
}

/// Closure mechanisms for critical periods
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClosureMechanism {
    /// Hormonal closure
    Hormonal,
    /// Structural closure
    Structural,
    /// Functional closure
    Functional,
    /// Experience-dependent closure
    ExperienceDependent,
}

/// Behavioral maturation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehavioralMaturation {
    pub motor_skill_development: MotorSkillDevelopment,
    pub social_behavior_development: SocialBehaviorDevelopment,
    pub foraging_skill_development: ForagingSkillDevelopment,
    pub predator_avoidance_development: PredatorAvoidanceDevelopment,
}

/// Motor skill development
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MotorSkillDevelopment {
    pub skill_sequence: Vec<String>, // Order of skill acquisition
    pub practice_requirements: Vec<(String, f64)>, // (skill, practice_time)
    pub myelination_dependence: bool,
}

/// Social behavior development
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocialBehaviorDevelopment {
    pub social_learning_periods: Vec<SocialLearningPeriod>,
    pub dominance_hierarchy_understanding: DominanceHierarchyUnderstanding,
    pub cooperative_behavior_development: CooperativeBehaviorDevelopment,
}

/// Social learning periods
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocialLearningPeriod {
    pub behavior_type: String,
    pub learning_window: (f64, f64),
    pub required_models: u16, // Number of adult models needed
    pub practice_opportunities: u16,
}

/// Dominance hierarchy understanding
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum DominanceHierarchyUnderstanding {
    /// No understanding
    None,
    /// Basic understanding
    Basic,
    /// Complex understanding
    Complex,
    /// Strategic understanding
    Strategic,
}

/// Cooperative behavior development
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum CooperativeBehaviorDevelopment {
    /// No cooperation
    None,
    /// Basic cooperation
    Basic,
    /// Complex cooperation
    Complex,
    /// Advanced cooperation
    Advanced,
}

/// Foraging skill development
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForagingSkillDevelopment {
    pub skill_progression: Vec<ForagingSkill>,
    pub learning_method: LearningMethod,
    pub cultural_transmission: bool,
}

/// Foraging skills
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ForagingSkill {
    /// Basic food recognition
    FoodRecognition,
    /// Food handling
    FoodHandling,
    /// Food processing
    FoodProcessing,
    /// Food storage
    FoodStorage,
    /// Specialized skill
    Specialized(String),
}

/// Learning methods
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LearningMethod {
    /// Individual learning
    Individual,
    /// Social learning
    Social,
    /// Mixed learning
    Mixed,
}

/// Predator avoidance development
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredatorAvoidanceDevelopment {
    pub innate_responses: Vec<String>,
    pub learned_responses: Vec<LearnedResponse>,
    pub response_timing: ResponseTiming,
}

/// Learned responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearnedResponse {
    pub predator_type: String,
    pub avoidance_behavior: String,
    pub learning_trigger: String,
}

/// Response timing
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ResponseTiming {
    /// Immediate response
    Immediate,
    /// Fast response
    Fast,
    /// Moderate response
    Moderate,
    /// Slow response
    Slow,
}

/// Developmental plasticity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DevelopmentalPlasticity {
    pub plasticity_type: PlasticityType,
    pub environmental_influences: Vec<EnvironmentalInfluence>,
    pub epigenetic_mechanisms: Vec<EpigeneticMechanism>,
    pub adaptive_range: AdaptiveRange,
}

/// Types of developmental plasticity
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlasticityType {
    /// No plasticity
    None,
    /// Limited plasticity
    Limited,
    /// Moderate plasticity
    Moderate,
    /// High plasticity
    High,
    /// Extreme plasticity
    Extreme,
}

/// Environmental influences on development
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentalInfluence {
    pub environmental_factor: String,
    pub influence_direction: InfluenceDirection,
    pub critical_window: (f64, f64),
    pub magnitude: f64, // Effect size
}

/// Influence direction
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum InfluenceDirection {
    /// Positive influence
    Positive,
    /// Negative influence
    Negative,
    /// Biphasic influence (U-shaped)
    Biphasic,
    /// Context-dependent influence
    ContextDependent,
}

/// Epigenetic mechanisms
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EpigeneticMechanism {
    /// DNA methylation
    DNAMethylation,
    /// Histone modification
    HistoneModification,
    /// Non-coding RNA
    NonCodingRNA,
    /// Chromatin remodeling
    ChromatinRemodeling,
}

/// Adaptive range
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdaptiveRange {
    pub phenotypic_range: PhenotypicRange,
    pub fitness_landscape: FitnessLandscape,
    pub trade_offs: Vec<TradeOff>,
}

/// Phenotypic range
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhenotypicRange {
    pub minimum_trait_value: f64,
    pub maximum_trait_value: f64,
    pub optimal_range: (f64, f64),
}

/// Fitness landscape
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FitnessLandscape {
    pub landscape_type: LandscapeType,
    pub peak_positions: Vec<f64>,
    pub valley_positions: Vec<f64>,
}

/// Landscape types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LandscapeType {
    /// Single peak
    SinglePeak,
    /// Multiple peaks
    MultiPeak,
    /// Flat landscape
    Flat,
    /// Rugged landscape
    Rugged,
}

/// Trade-offs in development
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeOff {
    pub trait_1: String,
    pub trait_2: String,
    pub trade_off_strength: f64, // 0.0 to 1.0
    pub environmental_modulation: bool,
}

/// Critical period definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CriticalPeriod {
    pub period_name: String,
    pub developmental_process: String,
    pub timing: (f64, f64), // Start and end relative to conception
    pub sensitivity_level: SensitivityLevel,
    pub irrevocability: Irrevocability,
    pub environmental_modifiers: Vec<EnvironmentalModifier>,
}

/// Sensitivity levels for critical periods
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SensitivityLevel {
    /// Very low sensitivity
    VeryLow,
    /// Low sensitivity
    Low,
    /// Moderate sensitivity
    Moderate,
    /// High sensitivity
    High,
    /// Very high sensitivity
    VeryHigh,
    /// Ultra-high sensitivity
    UltraHigh,
}

/// Irrevocability of critical period effects
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Irrevocability {
    /// Fully reversible
    FullyReversible,
    /// Partially reversible
    PartiallyReversible,
    /// Mostly irreversible
    MostlyIrreversible,
    /// Completely irreversible
    CompletelyIrreversible,
}

/// Environmental modifiers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentalModifier {
    pub modifier_type: String,
    pub effect_direction: EffectDirection,
    pub effect_magnitude: f64,
    pub timing_specificity: TimingSpecificity,
}

/// Effect direction
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EffectDirection {
    /// Enhances development
    Enhancement,
    /// Inhibits development
    Inhibition,
    /// Alters developmental trajectory
    Alteration,
    /// Delays development
    Delay,
}

/// Timing specificity
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TimingSpecificity {
    /// No timing specificity
    None,
    /// Early period specific
    Early,
    /// Late period specific
    Late,
    /// Window-specific
    WindowSpecific { window: (f64, f64) },
}

impl DevelopmentSchema {
    /// Create a new development schema
    pub fn new(
        developmental_stages: Vec<DevelopmentalStage>,
        growth_pattern: GrowthPattern,
        maturation_process: MaturationProcess,
        plasticity: DevelopmentalPlasticity,
        critical_periods: Vec<CriticalPeriod>,
    ) -> Self {
        Self {
            developmental_stages,
            growth_pattern,
            maturation_process,
            plasticity,
            critical_periods,
        }
    }

    /// Check when development is complete
    pub fn development_complete_at(&self) -> f64 {
        self.developmental_stages
            .iter()
            .map(|stage| stage.duration_proportion)
            .sum()
    }

    /// Check if parental care is required
    pub fn requires_parental_care(&self) -> bool {
        self.developmental_stages.iter().any(|stage| {
            stage.vulnerability_factors.iter().any(|vf| {
                matches!(vf.factor_type, VulnerabilityType::Nutritional)
                    && vf.severity_level >= SeverityLevel::High
            })
        }) || self
            .critical_periods
            .iter()
            .any(|cp| cp.sensitivity_level >= SensitivityLevel::High)
    }

    /// Get critical periods for specific process
    pub fn get_critical_periods_for_process(&self, process: &str) -> Vec<&CriticalPeriod> {
        self.critical_periods
            .iter()
            .filter(|cp| cp.developmental_process.contains(process))
            .collect()
    }

    /// Calculate overall developmental plasticity
    pub fn overall_plasticity_score(&self) -> f64 {
        let stage_plasticity: f64 = self
            .developmental_stages
            .iter()
            .map(|stage| match stage.differentiation_level {
                DifferentiationLevel::None => 1.0,
                DifferentiationLevel::Low => 0.8,
                DifferentiationLevel::Moderate => 0.6,
                DifferentiationLevel::High => 0.4,
                DifferentiationLevel::Complete => 0.2,
            })
            .sum::<f64>()
            / self.developmental_stages.len().max(1) as f64;

        let plasticity_bonus = match self.plasticity.plasticity_type {
            PlasticityType::None => 0.0,
            PlasticityType::Limited => 0.2,
            PlasticityType::Moderate => 0.4,
            PlasticityType::High => 0.6,
            PlasticityType::Extreme => 0.8,
        };

        (stage_plasticity + plasticity_bonus) / 2.0
    }

    /// Get developmental vulnerabilities
    pub fn get_developmental_vulnerabilities(&self) -> Vec<&VulnerabilityFactor> {
        self.developmental_stages
            .iter()
            .flat_map(|stage| stage.vulnerability_factors.iter())
            .filter(|vf| vf.severity_level >= SeverityLevel::Moderate)
            .collect()
    }
}

impl Default for DevelopmentSchema {
    fn default() -> Self {
        Self {
            developmental_stages: vec![
                DevelopmentalStage {
                    stage_name: "embryo".to_string(),
                    stage_type: StageType::Embryonic,
                    duration_proportion: 0.3,
                    growth_rate: GrowthRate::Moderate,
                    differentiation_level: DifferentiationLevel::High,
                    vulnerability_factors: vec![VulnerabilityFactor {
                        factor_type: VulnerabilityType::Environmental,
                        severity_level: SeverityLevel::High,
                        mitigation_strategies: vec!["parental_protection".to_string()],
                    }],
                    resource_requirements: DevelopmentalResourceRequirements {
                        energy_requirement: 0.5,
                        protein_requirement: 0.8,
                        micronutrient_requirements: vec![],
                        water_requirement: 0.9,
                        optimal_conditions: OptimalConditions {
                            temperature_range: (20.0, 30.0),
                            humidity_range: (40.0, 60.0),
                            photoperiod: Photoperiod {
                                light_hours: 12.0,
                                dark_hours: 12.0,
                                light_intensity: 0.5,
                                light_spectrum: vec!["visible".to_string()],
                            },
                            social_requirements: SocialRequirements {
                                social_needed: false,
                                group_size_optimal: None,
                                social_learning_required: false,
                                parental_contact_needed: true,
                            },
                            environmental_stability: EnvironmentalStability::Stable,
                        },
                    },
                },
                DevelopmentalStage {
                    stage_name: "juvenile".to_string(),
                    stage_type: StageType::Juvenile,
                    duration_proportion: 0.5,
                    growth_rate: GrowthRate::Rapid,
                    differentiation_level: DifferentiationLevel::Moderate,
                    vulnerability_factors: vec![VulnerabilityFactor {
                        factor_type: VulnerabilityType::Predation,
                        severity_level: SeverityLevel::Moderate,
                        mitigation_strategies: vec![
                            "group_living".to_string(),
                            "hiding".to_string(),
                        ],
                    }],
                    resource_requirements: DevelopmentalResourceRequirements {
                        energy_requirement: 0.8,
                        protein_requirement: 0.6,
                        micronutrient_requirements: vec![],
                        water_requirement: 0.7,
                        optimal_conditions: OptimalConditions {
                            temperature_range: (15.0, 35.0),
                            humidity_range: (30.0, 70.0),
                            photoperiod: Photoperiod {
                                light_hours: 14.0,
                                dark_hours: 10.0,
                                light_intensity: 0.7,
                                light_spectrum: vec!["visible".to_string(), "UV".to_string()],
                            },
                            social_requirements: SocialRequirements {
                                social_needed: true,
                                group_size_optimal: Some(5),
                                social_learning_required: true,
                                parental_contact_needed: false,
                            },
                            environmental_stability: EnvironmentalStability::Moderate,
                        },
                    },
                },
            ],
            growth_pattern: GrowthPattern {
                pattern_type: PatternType::Sigmoidal,
                growth_rate_curve: GrowthRateCurve {
                    initial_rate: 0.2,
                    peak_rate: 0.8,
                    final_rate: 0.1,
                    inflection_point: 0.5,
                },
                size_scaling: SizeScaling {
                    scaling_exponent: 0.75,
                    scaling_constant: 1.0,
                    size_range: (0.1, 10.0),
                },
                allometric_relationships: vec![],
            },
            maturation_process: MaturationProcess {
                maturation_type: MaturationType::Asynchronous,
                sexual_maturation: SexualMaturation {
                    maturation_trigger: MaturationTrigger::Age { threshold: 0.8 },
                    secondary_sexual_characteristics: vec![],
                    fertility_onset: FertilityOnset::Delayed { delay_period: 0.1 },
                    hormonal_regulation: HormonalRegulation {
                        key_hormones: vec!["testosterone".to_string(), "estrogen".to_string()],
                        feedback_loops: vec!["negative_feedback".to_string()],
                        circadian_rhythm: true,
                        seasonal_rhythm: false,
                    },
                },
                skeletal_maturation: SkeletalMaturation {
                    ossification_pattern: OssificationPattern::Endochondral,
                    growth_plate_closure: GrowthPlateClosure {
                        closure_pattern: ClosurePattern::ProximalDistal,
                        closure_timing: ClosureTiming::Average,
                        asynchronous_closure: true,
                    },
                    bone_density_development: BoneDensityDevelopment {
                        peak_density_age: 0.9,
                        density_maintenance_period: 0.1,
                        age_related_decline: AgeRelatedDecline::Gradual,
                    },
                },
                neural_maturation: NeuralMaturation {
                    brain_development: BrainDevelopment {
                        neurogenesis_timing: NeurogenesisTiming::PrenatalEarlyPostnatal,
                        regional_development: RegionalDevelopment {
                            development_sequence: vec![
                                "brainstem".to_string(),
                                "cerebellum".to_string(),
                                "cortex".to_string(),
                            ],
                            critical_periods: vec![],
                            plasticity_levels: vec![],
                        },
                        connectivity_patterns: vec![],
                    },
                    myelination_pattern: MyelinationPattern {
                        myelination_sequence: vec![
                            "brainstem".to_string(),
                            "cerebellum".to_string(),
                            "cortex".to_string(),
                        ],
                        myelination_rate: MyelinationRate::Moderate,
                        regional_differences: vec![],
                    },
                    synaptic_pruning: SynapticPruning {
                        pruning_pattern: PruningPattern::Global,
                        pruning_timing: PruningTiming::Adolescent,
                        activity_dependence: ActivityDependence::High,
                    },
                    critical_learning_periods: vec![],
                },
                behavioral_maturation: BehavioralMaturation {
                    motor_skill_development: MotorSkillDevelopment {
                        skill_sequence: vec![
                            "basic_locomotion".to_string(),
                            "fine_motor".to_string(),
                        ],
                        practice_requirements: vec![],
                        myelination_dependence: true,
                    },
                    social_behavior_development: SocialBehaviorDevelopment {
                        social_learning_periods: vec![],
                        dominance_hierarchy_understanding: DominanceHierarchyUnderstanding::Basic,
                        cooperative_behavior_development: CooperativeBehaviorDevelopment::Basic,
                    },
                    foraging_skill_development: ForagingSkillDevelopment {
                        skill_progression: vec![ForagingSkill::FoodRecognition],
                        learning_method: LearningMethod::Mixed,
                        cultural_transmission: false,
                    },
                    predator_avoidance_development: PredatorAvoidanceDevelopment {
                        innate_responses: vec!["freeze".to_string()],
                        learned_responses: vec![],
                        response_timing: ResponseTiming::Fast,
                    },
                },
            },
            plasticity: DevelopmentalPlasticity {
                plasticity_type: PlasticityType::Moderate,
                environmental_influences: vec![],
                epigenetic_mechanisms: vec![EpigeneticMechanism::DNAMethylation],
                adaptive_range: AdaptiveRange {
                    phenotypic_range: PhenotypicRange {
                        minimum_trait_value: 0.8,
                        maximum_trait_value: 1.2,
                        optimal_range: (0.9, 1.1),
                    },
                    fitness_landscape: FitnessLandscape {
                        landscape_type: LandscapeType::SinglePeak,
                        peak_positions: vec![1.0],
                        valley_positions: vec![],
                    },
                    trade_offs: vec![],
                },
            },
            critical_periods: vec![CriticalPeriod {
                period_name: "language_acquisition".to_string(),
                developmental_process: "neural_development".to_string(),
                timing: (0.2, 0.8),
                sensitivity_level: SensitivityLevel::High,
                irrevocability: Irrevocability::MostlyIrreversible,
                environmental_modifiers: vec![],
            }],
        }
    }
}
