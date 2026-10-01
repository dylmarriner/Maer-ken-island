/// Reproduction Schema
///
/// Pure schema definitions for future organism reproduction.
/// No executable logic, no state, no agency.
use serde::{Deserialize, Serialize};

/// Reproduction schema for organism reproductive strategies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReproductionSchema {
    pub reproductive_strategy: ReproductiveStrategy,
    pub life_cycle: LifeCycle,
    pub mating_system: MatingSystem,
    pub parental_care: ParentalCare,
    pub reproductive_timing: ReproductiveTiming,
}

/// Reproductive strategy type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReproductiveStrategy {
    /// Asexual reproduction
    Asexual { method: AsexualMethod },
    /// Sexual reproduction
    Sexual { gamete_type: GameteType },
    /// Alternation of generations
    Alternating {
        sexual_generation: String,
        asexual_generation: String,
    },
    /// Parthenogenesis (asexual from sexual species)
    Parthenogenesis {
        parthenogenesis_type: ParthenogenesisType,
    },
    /// Hermaphroditic reproduction
    Hermaphroditic { mating_type: HermaphroditicType },
}

/// Asexual reproduction methods
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AsexualMethod {
    /// Binary fission
    BinaryFission,
    /// Budding
    Budding,
    /// Fragmentation
    Fragmentation,
    /// Vegetative reproduction
    Vegetative,
    /// Spore formation
    SporeFormation,
    /// Specialized asexual method
    Specialized(String),
}

/// Gamete types for sexual reproduction
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GameteType {
    /// Isogamy (identical gametes)
    Isogamy,
    /// Anisogamy (different sized gametes)
    Anisogamy,
    /// Oogamy (large egg, small sperm)
    Oogamy,
    /// Specialized gamete type
    Specialized(String),
}

/// Parthenogenesis types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParthenogenesisType {
    /// Obligate parthenogenesis
    Obligate,
    /// Facultative parthenogenesis
    Facultative,
    /// Cyclic parthenogenesis
    Cyclic,
    /// Haploid parthenogenesis
    Haploid,
}

/// Hermaphroditic mating types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HermaphroditicType {
    /// Simultaneous hermaphrodite
    Simultaneous,
    /// Sequential hermaphrodite (protandry)
    Protandric,
    /// Sequential hermaphrodite (protogyny)
    Protogynic,
    /// Pseudogamous hermaphrodite
    Pseudogamous,
}

/// Life cycle definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifeCycle {
    pub cycle_type: CycleType,
    pub generation_time: GenerationTime,
    pub developmental_stages: Vec<DevelopmentalStage>,
    pub lifespan: Lifespan,
}

/// Types of life cycles
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CycleType {
    /// Direct development (no larval stage)
    Direct,
    /// Indirect development (with larval stage)
    Indirect { larval_type: String },
    /// Complex life cycle (multiple stages)
    Complex { stages: Vec<String> },
    /// Metamorphic life cycle
    Metamorphic {
        metamorphosis_type: MetamorphosisType,
    },
}

/// Metamorphosis types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MetamorphosisType {
    /// Complete metamorphosis
    Complete,
    /// Incomplete metamorphosis
    Incomplete,
    /// Partial metamorphosis
    Partial,
    /// Specialized metamorphosis
    Specialized(String),
}

/// Generation time classification
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum GenerationTime {
    /// Very short (hours to days)
    VeryShort,
    /// Short (days to weeks)
    Short,
    /// Medium (weeks to months)
    Medium,
    /// Long (months to years)
    Long,
    /// Very long (years to decades)
    VeryLong,
    /// Extremely long (decades+)
    ExtremelyLong,
}

/// Developmental stage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DevelopmentalStage {
    pub stage_name: String,
    pub duration_relative: f64, // Relative to total development
    pub vulnerability_level: VulnerabilityLevel,
    pub resource_requirements: ResourceRequirements,
    pub growth_rate: GrowthRate,
}

/// Vulnerability level during stage
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum VulnerabilityLevel {
    /// Very low vulnerability
    VeryLow,
    /// Low vulnerability
    Low,
    /// Moderate vulnerability
    Moderate,
    /// High vulnerability
    High,
    /// Very high vulnerability
    VeryHigh,
}

/// Resource requirements for stage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceRequirements {
    pub energy_requirement: f64,     // Relative to adult
    pub water_requirement: f64,      // Relative to body mass
    pub specific_needs: Vec<String>, // Specific nutrients or conditions
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

/// Lifespan classification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lifespan {
    pub lifespan_type: LifespanType,
    pub typical_lifespan: f64, // In appropriate time units
    pub maximum_lifespan: f64, // Maximum observed
    pub senescence_pattern: SenescencePattern,
}

/// Types of lifespan patterns
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LifespanType {
    /// Annual (lives one year)
    Annual,
    /// Perennial (lives multiple years)
    Perennial,
    /// Biennial (lives two years)
    Biennial,
    /// Ephemeral (very short lifespan)
    Ephemeral,
    /// Indeterminate (no fixed lifespan)
    Indeterminate,
}

/// Senescence (aging) patterns
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SenescencePattern {
    /// No senescence (negligible senescence)
    Negligible,
    /// Rapid senescence
    Rapid,
    /// Gradual senescence
    Gradual,
    /// Programmed senescence
    Programmed,
    /// Environmental senescence
    Environmental,
}

/// Mating system definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatingSystem {
    pub system_type: MatingSystemType,
    pub mate_selection: MateSelection,
    pub competition_level: CompetitionLevel,
    pub pair_bond: PairBond,
}

/// Types of mating systems
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MatingSystemType {
    /// Monogamy (one partner)
    Monogamy { monogamy_type: MonogamyType },
    /// Polygamy (multiple partners)
    Polygamy { polygamy_type: PolygamyType },
    /// Promiscuity (no pair bonds)
    Promiscuity,
    /// No mating (asexual)
    None,
    /// Specialized mating system
    Specialized(String),
}

/// Monogamy types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MonogamyType {
    /// Serial monogamy
    Serial,
    /// Lifetime monogamy
    Lifetime,
    /// Social monogamy (sexual cheating)
    Social,
}

/// Polygamy types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PolygamyType {
    /// Polygyny (one male, multiple females)
    Polygyny,
    /// Polyandry (one female, multiple males)
    Polyandry,
    /// Polygynandry (multiple males and females)
    Polygynandry,
    /// Resource defense polygyny
    ResourceDefense,
    /// Lek polygyny
    Lek,
}

/// Mate selection criteria
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MateSelection {
    pub selection_criteria: Vec<SelectionCriterion>,
    pub selection_method: SelectionMethod,
    pub choosiness_level: ChoosinessLevel,
}

/// Selection criteria for mates
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SelectionCriterion {
    /// Physical traits
    Physical { traits: Vec<String> },
    /// Behavioral traits
    Behavioral { behaviors: Vec<String> },
    /// Resource holding
    ResourceHolding { resources: Vec<String> },
    /// Genetic compatibility
    Genetic { markers: Vec<String> },
    /// Environmental adaptation
    Environmental { adaptations: Vec<String> },
}

/// Selection methods
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SelectionMethod {
    /// Female choice
    FemaleChoice,
    /// Male-male competition
    MaleCompetition,
    /// Mutual choice
    MutualChoice,
    /// No choice (random mating)
    Random,
    /// Environmental filtering
    Environmental,
}

/// Choosiness level in mate selection
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ChoosinessLevel {
    /// Not choosy
    None,
    /// Slightly choosy
    Low,
    /// Moderately choosy
    Moderate,
    /// Very choosy
    High,
    /// Extremely choosy
    VeryHigh,
}

/// Competition level for mates
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum CompetitionLevel {
    /// No competition
    None,
    /// Low competition
    Low,
    /// Moderate competition
    Moderate,
    /// High competition
    High,
    /// Very high competition
    VeryHigh,
}

/// Pair bond characteristics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairBond {
    pub bond_strength: BondStrength,
    pub bond_duration: BondDuration,
    pub extra_pair_copulation: bool,
    pub divorce_rate: f64, // 0.0 to 1.0
}

/// Bond strength
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum BondStrength {
    /// No bond
    None,
    /// Weak bond
    Weak,
    /// Moderate bond
    Moderate,
    /// Strong bond
    Strong,
    /// Very strong bond
    VeryStrong,
}

/// Bond duration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BondDuration {
    /// No duration
    None,
    /// Very short (minutes to hours)
    VeryShort,
    /// Short (hours to days)
    Short,
    /// Medium (days to weeks)
    Medium,
    /// Long (weeks to months)
    Long,
    /// Lifetime
    Lifetime,
}

/// Parental care system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParentalCare {
    pub care_type: CareType,
    pub care_duration: CareDuration,
    pub care_intensity: CareIntensity,
    pub care_division: CareDivision,
}

/// Types of parental care
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CareType {
    /// No parental care
    None,
    /// Egg guarding
    EggGuarding,
    /// Live birth with care
    LiveBirth,
    /// Feeding offspring
    Feeding,
    /// Teaching offspring
    Teaching,
    /// Complex care (multiple types)
    Complex { care_methods: Vec<String> },
}

/// Duration of parental care
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CareDuration {
    /// No care
    None,
    /// Very brief (minutes to hours)
    VeryBrief,
    /// Brief (hours to days)
    Brief,
    /// Extended (days to weeks)
    Extended,
    /// Prolonged (weeks to months)
    Prolonged,
    /// Very prolonged (months to years)
    VeryProlonged,
}

/// Intensity of parental care
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum CareIntensity {
    /// No care
    None,
    /// Minimal care
    Minimal,
    /// Moderate care
    Moderate,
    /// Intensive care
    Intensive,
    /// Very intensive care
    VeryIntensive,
}

/// Division of parental care
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CareDivision {
    /// No care
    None,
    /// Female only
    FemaleOnly,
    /// Male only
    MaleOnly,
    /// Biparental (both parents)
    Biparental,
    /// Communal (group care)
    Communal,
    /// Specialized division
    Specialized { roles: Vec<(String, String)> },
}

/// Reproductive timing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReproductiveTiming {
    pub reproductive_frequency: ReproductiveFrequency,
    pub seasonal_breeding: SeasonalBreeding,
    pub age_at_maturity: AgeAtMaturity,
    pub reproductive_lifespan: ReproductiveLifespan,
}

/// Reproductive frequency
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReproductiveFrequency {
    /// Reproduce once (semelparous)
    Once,
    /// Reproduce multiple times (iteroparous)
    Multiple { frequency: Frequency },
    /// Continuous reproduction
    Continuous,
    /// Seasonal reproduction
    Seasonal,
}

/// Frequency for multiple reproduction
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Frequency {
    /// Multiple times per year
    MultipleYearly { times_per_year: u8 },
    /// Once per year
    Yearly,
    /// Every few years
    MultiYear { years_between: u8 },
}

/// Seasonal breeding patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeasonalBreeding {
    pub is_seasonal: bool,
    pub breeding_season: String,             // Season name
    pub environmental_triggers: Vec<String>, // What triggers breeding
    pub migration_required: bool,
}

/// Age at maturity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgeAtMaturity {
    pub maturity_type: MaturityType,
    pub age_relative: f64,                  // Relative to total lifespan
    pub size_at_maturity: f64,              // Relative to adult size
    pub environmental_factors: Vec<String>, // Factors affecting maturity
}

/// Types of maturity
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MaturityType {
    /// Fixed age at maturity
    Fixed,
    /// Size-dependent maturity
    SizeDependent,
    /// Environment-dependent maturity
    Environmental,
    /// Socially-mediated maturity
    Social,
}

/// Reproductive lifespan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReproductiveLifespan {
    pub start_age: f64,                    // Age when reproduction begins
    pub end_age: Option<f64>,              // Age when reproduction ends (None = lifelong)
    pub peak_fertility_period: (f64, f64), // Peak fertility window
    pub fertility_decline: FertilityDecline,
}

/// Fertility decline patterns
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FertilityDecline {
    /// No decline
    None,
    /// Gradual decline
    Gradual,
    /// Rapid decline
    Rapid,
    /// Sudden decline
    Sudden,
    /// Seasonal fluctuation
    Seasonal,
}

impl ReproductionSchema {
    /// Create a new reproduction schema
    pub fn new(
        reproductive_strategy: ReproductiveStrategy,
        life_cycle: LifeCycle,
        mating_system: MatingSystem,
        parental_care: ParentalCare,
        reproductive_timing: ReproductiveTiming,
    ) -> Self {
        Self {
            reproductive_strategy,
            life_cycle,
            mating_system,
            parental_care,
            reproductive_timing,
        }
    }

    /// Validate development alignment
    ///
    /// Ensures reproductive strategy aligns with developmental patterns
    pub fn validate_development_alignment(
        &self,
        development_schema: &crate::organisms::development::DevelopmentSchema,
    ) -> Result<(), crate::organisms::SchemaValidationError> {
        // Check if reproductive timing matches developmental timing
        let maturity_age = self.reproductive_timing.age_at_maturity.age_relative;
        let development_complete = development_schema.development_complete_at();

        if maturity_age < development_complete {
            return Err(
                crate::organisms::SchemaValidationError::ReproductionMisaligned(
                    "Sexual maturity occurs before development is complete".to_string(),
                ),
            );
        }

        // Check if parental care matches developmental needs
        if self.parental_care.care_type == CareType::None
            && development_schema.requires_parental_care()
        {
            return Err(
                crate::organisms::SchemaValidationError::ReproductionMisaligned(
                    "No parental care but development requires parental support".to_string(),
                ),
            );
        }

        // Check if life cycle complexity matches reproductive strategy
        match (&self.life_cycle.cycle_type, &self.reproductive_strategy) {
            (CycleType::Direct, ReproductiveStrategy::Sexual { .. }) => {
                // This is acceptable
            }
            (CycleType::Indirect { .. }, ReproductiveStrategy::Asexual { .. }) => {
                return Err(
                    crate::organisms::SchemaValidationError::ReproductionMisaligned(
                        "Indirect development with asexual reproduction is unusual".to_string(),
                    ),
                );
            }
            _ => {
                // Other combinations are generally acceptable
            }
        }

        Ok(())
    }

    /// Check if organism is semelparous (reproduces once)
    pub fn is_semelparous(&self) -> bool {
        matches!(
            self.reproductive_timing.reproductive_frequency,
            ReproductiveFrequency::Once
        )
    }

    /// Check if organism is iteroparous (reproduces multiple times)
    pub fn is_iteroparous(&self) -> bool {
        !self.is_semelparous()
    }

    /// Check if organism has sexual reproduction
    pub fn has_sexual_reproduction(&self) -> bool {
        matches!(
            self.reproductive_strategy,
            ReproductiveStrategy::Sexual { .. }
                | ReproductiveStrategy::Alternating { .. }
                | ReproductiveStrategy::Hermaphroditic { .. }
        )
    }

    /// Calculate reproductive investment
    pub fn reproductive_investment(&self) -> f64 {
        let parental_care_investment = match self.parental_care.care_intensity {
            CareIntensity::None => 0.0,
            CareIntensity::Minimal => 0.1,
            CareIntensity::Moderate => 0.3,
            CareIntensity::Intensive => 0.6,
            CareIntensity::VeryIntensive => 0.9,
        };

        let gamete_investment = match &self.reproductive_strategy {
            ReproductiveStrategy::Sexual { gamete_type } => {
                match gamete_type {
                    GameteType::Oogamy => 0.8, // High investment in eggs
                    GameteType::Anisogamy => 0.5,
                    GameteType::Isogamy => 0.3,
                    GameteType::Specialized(_) => 0.5,
                }
            }
            ReproductiveStrategy::Asexual { .. } => 0.2,
            _ => 0.4,
        };

        (parental_care_investment + gamete_investment) / 2.0
    }
}

impl Default for ReproductionSchema {
    fn default() -> Self {
        Self {
            reproductive_strategy: ReproductiveStrategy::Sexual {
                gamete_type: GameteType::Oogamy,
            },
            life_cycle: LifeCycle {
                cycle_type: CycleType::Direct,
                generation_time: GenerationTime::Medium,
                developmental_stages: vec![DevelopmentalStage {
                    stage_name: "embryo".to_string(),
                    duration_relative: 0.2,
                    vulnerability_level: VulnerabilityLevel::High,
                    resource_requirements: ResourceRequirements {
                        energy_requirement: 0.5,
                        water_requirement: 0.8,
                        specific_needs: vec!["nutrients".to_string()],
                    },
                    growth_rate: GrowthRate::Moderate,
                }],
                lifespan: Lifespan {
                    lifespan_type: LifespanType::Perennial,
                    typical_lifespan: 5.0,
                    maximum_lifespan: 10.0,
                    senescence_pattern: SenescencePattern::Gradual,
                },
            },
            mating_system: MatingSystem {
                system_type: MatingSystemType::Polygamy {
                    polygamy_type: PolygamyType::Polygyny,
                },
                mate_selection: MateSelection {
                    selection_criteria: vec![SelectionCriterion::Physical {
                        traits: vec!["size".to_string(), "coloration".to_string()],
                    }],
                    selection_method: SelectionMethod::FemaleChoice,
                    choosiness_level: ChoosinessLevel::Moderate,
                },
                competition_level: CompetitionLevel::Moderate,
                pair_bond: PairBond {
                    bond_strength: BondStrength::Weak,
                    bond_duration: BondDuration::Short,
                    extra_pair_copulation: true,
                    divorce_rate: 0.3,
                },
            },
            parental_care: ParentalCare {
                care_type: CareType::EggGuarding,
                care_duration: CareDuration::Brief,
                care_intensity: CareIntensity::Minimal,
                care_division: CareDivision::FemaleOnly,
            },
            reproductive_timing: ReproductiveTiming {
                reproductive_frequency: ReproductiveFrequency::Multiple {
                    frequency: Frequency::Yearly,
                },
                seasonal_breeding: SeasonalBreeding {
                    is_seasonal: true,
                    breeding_season: "spring".to_string(),
                    environmental_triggers: vec![
                        "temperature".to_string(),
                        "photoperiod".to_string(),
                    ],
                    migration_required: false,
                },
                age_at_maturity: AgeAtMaturity {
                    maturity_type: MaturityType::Fixed,
                    age_relative: 0.8,
                    size_at_maturity: 0.9,
                    environmental_factors: vec!["nutrition".to_string()],
                },
                reproductive_lifespan: ReproductiveLifespan {
                    start_age: 0.8,
                    end_age: Some(0.95),
                    peak_fertility_period: (0.85, 0.9),
                    fertility_decline: FertilityDecline::Gradual,
                },
            },
        }
    }
}
