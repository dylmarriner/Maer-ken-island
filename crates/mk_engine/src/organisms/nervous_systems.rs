/// Nervous System Schema
///
/// Pure schema definitions for future organism nervous systems.
/// No executable logic, no state, no agency.
use serde::{Deserialize, Serialize};

/// Nervous system schema for organism neural architecture
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NervousSystemSchema {
    pub architecture: NeuralArchitecture,
    pub sensory_modalities: Vec<SensoryModality>,
    pub cognitive_level: CognitiveLevel,
    pub processing_capacity: ProcessingCapacity,
    pub behavioral_complexity: BehavioralComplexity,
}

/// Neural architecture type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NeuralArchitecture {
    /// No nervous system
    None,
    /// Simple nerve net
    NerveNet,
    /// Ganglionic system (segmented ganglia)
    Ganglionic { ganglion_count: u16 },
    /// Centralized brain with spinal cord
    Centralized { brain_regions: Vec<BrainRegion> },
    /// Distributed network with local processing
    Distributed { node_count: u16 },
    /// Hybrid architecture
    Hybrid {
        primary_type: Box<NeuralArchitecture>,
        secondary_features: Vec<NeuralArchitecture>,
    },
}

/// Brain regions for centralized systems
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BrainRegion {
    /// Forebrain (cerebrum, thalamus)
    Forebrain,
    /// Midbrain (tectum, tegmentum)
    Midbrain,
    /// Hindbrain (cerebellum, pons, medulla)
    Hindbrain,
    /// Specialized sensory processing
    Sensory { modality: String },
    /// Motor control centers
    Motor,
    /// Associative areas
    Associative,
    /// Limbic system (emotion, memory)
    Limbic,
}

/// Sensory modality definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensoryModality {
    pub modality_type: SensoryType,
    pub sensitivity_level: SensitivityLevel,
    pub processing_complexity: ProcessingComplexity,
    pub environmental_range: EnvironmentalRange,
    pub adaptation_capability: AdaptationCapability,
}

/// Types of sensory modalities
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SensoryType {
    /// Visual sensing
    Visual,
    /// Auditory sensing
    Auditory,
    /// Chemical sensing (taste/smell)
    Chemical,
    /// Mechanical sensing (touch, pressure)
    Mechanical,
    /// Thermal sensing
    Thermal,
    /// Electromagnetic sensing
    Electromagnetic,
    /// Gravitational sensing
    Gravitational,
    /// Proprioceptive sensing (body position)
    Proprioceptive,
    /// Nociceptive sensing (pain)
    Nociceptive,
    /// Specialized sensing
    Specialized(String),
}

/// Sensitivity level of sensory modality
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

/// Processing complexity for sensory data
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ProcessingComplexity {
    /// Simple reflex processing
    Reflex,
    /// Basic pattern recognition
    Pattern,
    /// Complex processing with integration
    Complex,
    /// Advanced processing with prediction
    Advanced,
}

/// Environmental range for sensory modality
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentalRange {
    pub minimum_detectable: f64,
    pub maximum_detectable: f64,
    pub optimal_range: (f64, f64),
    pub units: String,
}

/// Adaptation capability
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum AdaptationCapability {
    /// No adaptation
    None,
    /// Short-term adaptation (seconds to minutes)
    ShortTerm,
    /// Long-term adaptation (hours to days)
    LongTerm,
    /// Developmental adaptation (permanent)
    Developmental,
    /// Evolutionary adaptation (generational)
    Evolutionary,
}

/// Cognitive level of nervous system
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum CognitiveLevel {
    /// No cognitive processing
    None,
    /// Basic stimulus-response
    Reactive,
    /// Simple learning and memory
    Simple,
    /// Complex problem solving
    Complex,
    /// Abstract reasoning
    Abstract,
    /// Self-awareness
    SelfAware,
    /// Metacognition (thinking about thinking)
    Metacognitive,
}

/// Processing capacity metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingCapacity {
    pub processing_speed: ProcessingSpeed,
    pub memory_capacity: MemoryCapacity,
    pub parallel_processing: bool,
    pub learning_capability: LearningCapability,
}

/// Processing speed classification
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ProcessingSpeed {
    /// Very slow processing
    VerySlow,
    /// Slow processing
    Slow,
    /// Moderate processing
    Moderate,
    /// Fast processing
    Fast,
    /// Very fast processing
    VeryFast,
    /// Ultra-fast processing
    UltraFast,
}

/// Memory capacity type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryCapacity {
    pub short_term_capacity: MemorySize,
    pub long_term_capacity: MemorySize,
    pub working_memory: bool,
    pub episodic_memory: bool,
    pub semantic_memory: bool,
}

/// Memory size classification
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum MemorySize {
    /// No memory
    None,
    /// Minimal memory
    Minimal,
    /// Limited memory
    Limited,
    /// Moderate memory
    Moderate,
    /// Large memory
    Large,
    /// Very large memory
    VeryLarge,
}

/// Learning capability
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum LearningCapability {
    /// No learning capability
    None,
    /// Habituation (simple learning)
    Habituation,
    /// Classical conditioning
    Classical,
    /// Operant conditioning
    Operant,
    /// Social learning
    Social,
    /// Insight learning
    Insight,
    /// Abstract learning
    Abstract,
}

/// Behavioral complexity metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehavioralComplexity {
    pub behavior_types: Vec<BehaviorType>,
    pub social_complexity: SocialComplexity,
    pub problem_solving: ProblemSolvingCapability,
    pub communication: CommunicationComplexity,
}

/// Types of behaviors
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BehaviorType {
    /// Instinctive behaviors
    Instinctive,
    /// Learned behaviors
    Learned,
    /// Social behaviors
    Social,
    /// Problem-solving behaviors
    ProblemSolving,
    /// Communicative behaviors
    Communicative,
    /// Play behaviors
    Play,
    /// Exploratory behaviors
    Exploratory,
}

/// Social complexity level
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SocialComplexity {
    /// Solitary lifestyle
    Solitary,
    /// Simple aggregations
    Aggregative,
    /// Hierarchical societies
    Hierarchical,
    /// Cooperative societies
    Cooperative,
    /// Eusocial societies
    Eusocial,
}

/// Problem solving capability
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ProblemSolvingCapability {
    /// No problem solving
    None,
    /// Trial and error
    TrialError,
    /// Insight solving
    Insight,
    /// Tool use
    ToolUse,
    /// Planning
    Planning,
    /// Abstract reasoning
    Abstract,
}

/// Communication complexity
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum CommunicationComplexity {
    /// No communication
    None,
    /// Simple signals
    Simple,
    /// Complex signaling
    Complex,
    /// Symbolic communication
    Symbolic,
    /// Language-like communication
    LanguageLike,
}

impl NeuralArchitecture {
    /// Get complexity level of architecture
    pub fn complexity_level(&self) -> u8 {
        match self {
            NeuralArchitecture::None => 0,
            NeuralArchitecture::NerveNet => 1,
            NeuralArchitecture::Ganglionic { .. } => 2,
            NeuralArchitecture::Centralized { .. } => 4,
            NeuralArchitecture::Distributed { .. } => 3,
            NeuralArchitecture::Hybrid { primary_type, .. } => primary_type.complexity_level() + 1,
        }
    }

    /// Check if architecture requires circulatory support
    pub fn requires_circulatory_support(&self) -> bool {
        matches!(
            self,
            NeuralArchitecture::Centralized { .. } | NeuralArchitecture::Hybrid { .. }
        )
    }

    /// Check if architecture supports learning
    pub fn supports_learning(&self) -> bool {
        self.complexity_level() >= 2
    }

    /// Check if architecture supports memory
    pub fn supports_memory(&self) -> bool {
        self.complexity_level() >= 3
    }
}

impl NervousSystemSchema {
    /// Create a new nervous system schema
    pub fn new(
        architecture: NeuralArchitecture,
        sensory_modalities: Vec<SensoryModality>,
        cognitive_level: CognitiveLevel,
        processing_capacity: ProcessingCapacity,
        behavioral_complexity: BehavioralComplexity,
    ) -> Self {
        Self {
            architecture,
            sensory_modalities,
            cognitive_level,
            processing_capacity,
            behavioral_complexity,
        }
    }

    /// Validate cognitive constraints
    ///
    /// Ensures cognitive level matches neural architecture
    pub fn validate_cognitive_constraints(
        &self,
    ) -> Result<(), crate::organisms::SchemaValidationError> {
        let max_supported_cognitive = self.architecture.complexity_level();
        let current_cognitive = self.cognitive_level.clone() as u8;

        if current_cognitive > max_supported_cognitive {
            return Err(
                crate::organisms::SchemaValidationError::NervousSystemViolation(format!(
                    "Cognitive level {:?} exceeds neural architecture capacity {}",
                    self.cognitive_level, max_supported_cognitive
                )),
            );
        }

        // Check memory requirements
        if self.processing_capacity.memory_capacity.long_term_capacity != MemorySize::None
            && !self.architecture.supports_memory()
        {
            return Err(
                crate::organisms::SchemaValidationError::NervousSystemViolation(
                    "Memory capacity requires more complex neural architecture".to_string(),
                ),
            );
        }

        // Check learning requirements
        if self.processing_capacity.learning_capability != LearningCapability::None
            && !self.architecture.supports_learning()
        {
            return Err(
                crate::organisms::SchemaValidationError::NervousSystemViolation(
                    "Learning capability requires more complex neural architecture".to_string(),
                ),
            );
        }

        Ok(())
    }

    /// Get sensory modalities by type
    pub fn get_sensory_modalities(&self, modality_type: &SensoryType) -> Vec<&SensoryModality> {
        self.sensory_modalities
            .iter()
            .filter(|sm| sm.modality_type == *modality_type)
            .collect()
    }

    /// Check if system supports specific behavior type
    pub fn supports_behavior(&self, behavior_type: &BehaviorType) -> bool {
        match behavior_type {
            BehaviorType::Instinctive => true,
            BehaviorType::Learned => self.architecture.supports_learning(),
            BehaviorType::Social => {
                self.behavioral_complexity.social_complexity != SocialComplexity::Solitary
            }
            BehaviorType::ProblemSolving => {
                self.behavioral_complexity.problem_solving != ProblemSolvingCapability::None
            }
            BehaviorType::Communicative => {
                self.behavioral_complexity.communication != CommunicationComplexity::None
            }
            BehaviorType::Play => self.cognitive_level >= CognitiveLevel::Simple,
            BehaviorType::Exploratory => self.cognitive_level >= CognitiveLevel::Reactive,
        }
    }

    /// Calculate overall neural complexity score
    pub fn neural_complexity_score(&self) -> f64 {
        let architecture_score = self.architecture.complexity_level() as f64;
        let sensory_score = self.sensory_modalities.len() as f64;
        let cognitive_score = self.cognitive_level.clone() as u8 as f64;
        let processing_score = (self.processing_capacity.processing_speed.clone() as u8 as f64)
            + (self
                .processing_capacity
                .memory_capacity
                .short_term_capacity
                .clone() as u8 as f64);

        (architecture_score + sensory_score + cognitive_score + processing_score) / 4.0
    }
}

impl Default for NervousSystemSchema {
    fn default() -> Self {
        Self {
            architecture: NeuralArchitecture::Ganglionic { ganglion_count: 3 },
            sensory_modalities: vec![SensoryModality {
                modality_type: SensoryType::Mechanical,
                sensitivity_level: SensitivityLevel::Moderate,
                processing_complexity: ProcessingComplexity::Reflex,
                environmental_range: EnvironmentalRange {
                    minimum_detectable: 0.01,
                    maximum_detectable: 100.0,
                    optimal_range: (0.1, 10.0),
                    units: "N".to_string(),
                },
                adaptation_capability: AdaptationCapability::ShortTerm,
            }],
            cognitive_level: CognitiveLevel::Reactive,
            processing_capacity: ProcessingCapacity {
                processing_speed: ProcessingSpeed::Moderate,
                memory_capacity: MemoryCapacity {
                    short_term_capacity: MemorySize::Limited,
                    long_term_capacity: MemorySize::None,
                    working_memory: false,
                    episodic_memory: false,
                    semantic_memory: false,
                },
                parallel_processing: false,
                learning_capability: LearningCapability::Habituation,
            },
            behavioral_complexity: BehavioralComplexity {
                behavior_types: vec![BehaviorType::Instinctive],
                social_complexity: SocialComplexity::Solitary,
                problem_solving: ProblemSolvingCapability::None,
                communication: CommunicationComplexity::None,
            },
        }
    }
}
