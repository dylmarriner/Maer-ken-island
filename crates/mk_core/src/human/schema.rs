#![allow(non_snake_case)]

//! Canonical human schema mirroring `docs/canon/HumanReplicationSchema.js`.
//!
//! This module provides a comprehensive, serde-friendly Rust representation of
//! the JS authority schema while remaining practical to maintain inside
//! `mk_core`. Existing legacy profile types remain available for compatibility,
//! but `HumanSchema` is the comprehensive canonical shape.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

fn default_timestamp() -> String {
    "1970-01-01T00:00:00Z".to_string()
}

fn default_schema_version() -> String {
    "2.0.0".to_string()
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Coordinates3D {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct GeoCoordinates {
    pub latitude: f64,
    pub longitude: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct NumericRange {
    pub min: f64,
    pub max: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum HumanLifecycleStatus {
    #[default]
    Alive,
    Dead,
    Dormant,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum BiologicalSexSchema {
    Male,
    Female,
    #[default]
    Neutral,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum LocalitySchema {
    #[default]
    Urban,
    Suburban,
    Rural,
    SemiRural,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct BirthplaceSchema {
    pub location: String,
    pub coordinates: GeoCoordinates,
    pub locality: LocalitySchema,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdhdSubtypeSchema {
    InattentivePresentation,
    CombinedPresentation,
    HyperactiveImpulsive,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct AdhdAttentionalProfileSchema {
    pub sustained_attention: Option<f32>,
    pub selective_attention: Option<f32>,
    pub divided_attention: Option<f32>,
    pub alternating_attention: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct AdhdHyperactivityProfileSchema {
    pub motor_hyperactivity: Option<f32>,
    pub verbal_hyperactivity: Option<f32>,
    pub mental_hyperactivity: Option<f32>,
    pub impulsivity_level: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct AdhdExecutiveFunctioningSchema {
    pub working_memory: Option<f32>,
    pub planning_organizing: Option<f32>,
    pub time_management: Option<f32>,
    pub emotional_regulation: Option<f32>,
    pub task_initiation: Option<f32>,
    pub task_completion: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChronotypeSchema {
    Morning,
    Evening,
    Intermediate,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct CircadianRhythmSchema {
    pub chronotype: Option<ChronotypeSchema>,
    pub sleep_onset_difficulty: Option<f32>,
    pub sleep_maintenance: Option<f32>,
    pub daytime_somnolence: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ComorbidityPatternsSchema {
    pub anxiety_level: Option<f32>,
    pub depression_level: Option<f32>,
    pub emotional_dysregulation: Option<f32>,
    pub rejection_sensitivity: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct AdhdProfileSchema {
    pub attentional_profile: Option<AdhdAttentionalProfileSchema>,
    pub hyperactivity_profile: Option<AdhdHyperactivityProfileSchema>,
    pub executive_functioning: Option<AdhdExecutiveFunctioningSchema>,
    pub circadian_rhythm: Option<CircadianRhythmSchema>,
    pub comorbidity_patterns: Option<ComorbidityPatternsSchema>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutismSpectrumSchema {
    Level1HighFunctioning,
    Level2RequiringSupport,
    Level3RequiringVerySubstantialSupport,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommunicationStyleSchema {
    Direct,
    Formal,
    Literal,
    NonverbalPreferenced,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct SocialCommunicationSchema {
    pub social_recognition: Option<f32>,
    pub social_motivation: Option<f32>,
    pub social_anxiety: Option<f32>,
    pub communication_style: Option<CommunicationStyleSchema>,
    pub nonverbal_communication: Option<f32>,
    pub pragmatic_language: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct EightChannelSensitivitySchema {
    pub auditory: Option<f32>,
    pub visual: Option<f32>,
    pub tactile: Option<f32>,
    pub proprioceptive: Option<f32>,
    pub vestibular: Option<f32>,
    pub interoceptive: Option<f32>,
    pub olfactory: Option<f32>,
    pub gustatory: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct SensorySeekingSchema {
    pub proprioceptive_seeking: Option<f32>,
    pub vestibular_seeking: Option<f32>,
    pub tactile_seeking: Option<f32>,
    pub oral_seeking: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct AutismSensoryProcessingSchema {
    pub hypersensitivity: Option<EightChannelSensitivitySchema>,
    pub hyposensitivity: Option<EightChannelSensitivitySchema>,
    pub sensory_seeking: Option<SensorySeekingSchema>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct RestrictedInterestsSchema {
    pub intensity: Option<f32>,
    pub breadth: Option<f32>,
    pub flexibility: Option<f32>,
    pub knowledge_depth: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct RestrictedRepetitiveBehaviorsSchema {
    pub stereotyped_movements: Option<f32>,
    pub ritualistic_behavior: Option<f32>,
    pub restricted_interests: Option<RestrictedInterestsSchema>,
    pub sensory_regulation_needs: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct AutismExecutiveFunctioningSchema {
    pub cognitive_flexibility: Option<f32>,
    pub planning_sequencing: Option<f32>,
    pub working_memory: Option<f32>,
    pub inhibition_control: Option<f32>,
    pub abstract_thinking: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct InformationProcessingSchema {
    pub detail_focus: Option<f32>,
    pub pattern_recognition: Option<f32>,
    pub system_thinking: Option<f32>,
    pub visual_processing: Option<f32>,
    pub auditory_processing: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct AutismEmotionalProcessingSchema {
    pub emotional_identification: Option<f32>,
    pub emotional_regulation: Option<f32>,
    pub alexithymia_tendency: Option<f32>,
    pub emotional_intensity: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct AutismProfileSchema {
    pub social_communication: Option<SocialCommunicationSchema>,
    pub sensory_processing: Option<AutismSensoryProcessingSchema>,
    pub restricted_repetitive_behaviors: Option<RestrictedRepetitiveBehaviorsSchema>,
    pub executive_functioning: Option<AutismExecutiveFunctioningSchema>,
    pub information_processing: Option<InformationProcessingSchema>,
    pub emotional_processing: Option<AutismEmotionalProcessingSchema>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct AudhdAttentionalDynamicsSchema {
    pub hyperfocus_intensity: Option<f32>,
    pub attentional_shifts: Option<f32>,
    pub environmental_filtering: Option<f32>,
    pub task_switching_difficulty: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct SensoryAttentionInteractionSchema {
    pub sensory_overload_impact: Option<f32>,
    pub stimming_for_focus: Option<f32>,
    pub environmental_adaptation_needs: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct SocialCognitiveInteractionSchema {
    pub social_exhaustion: Option<f32>,
    pub masking_energy_cost: Option<f32>,
    pub executive_social_conflict: Option<f32>,
    pub rejection_sensitivity_amplification: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct EmotionalRegulationComplexitySchema {
    pub emotional_volatility: Option<f32>,
    pub emotional_burnout: Option<f32>,
    pub cooccuring_anxiety_depression: Option<f32>,
    pub self_concept_impact: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct AudhdInteractionSchema {
    pub attentional_dynamics: Option<AudhdAttentionalDynamicsSchema>,
    pub sensory_attention_interaction: Option<SensoryAttentionInteractionSchema>,
    pub social_cognitive_interaction: Option<SocialCognitiveInteractionSchema>,
    pub emotional_regulation_complexity: Option<EmotionalRegulationComplexitySchema>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SchizophreniaSubtypeSchema {
    Paranoid,
    Disorganized,
    Catatonic,
    Undifferentiated,
    Residual,
    Schizoaffective,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct HallucinationsSchema {
    pub auditory: Option<f32>,
    pub visual: Option<f32>,
    pub olfactory: Option<f32>,
    pub tactile: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct DelusionsSchema {
    pub persecutory: Option<f32>,
    pub grandiose: Option<f32>,
    pub referential: Option<f32>,
    pub erotomanic: Option<f32>,
    pub nihilistic: Option<f32>,
    pub somatic: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct PositiveSymptomsSchema {
    pub hallucinations: Option<HallucinationsSchema>,
    pub delusions: Option<DelusionsSchema>,
    pub disorganized_speech: Option<f32>,
    pub grossly_disorganized_behavior: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct NegativeSymptomsSchema {
    pub alogia: Option<f32>,
    pub anhedonia: Option<f32>,
    pub asociality: Option<f32>,
    pub avolition: Option<f32>,
    pub flat_affect: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct CognitiveSymptomsSchema {
    pub executive_function_impairment: Option<f32>,
    pub working_memory_deficits: Option<f32>,
    pub attention_impairment: Option<f32>,
    pub processing_speed_deficits: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EpisodicVsContinuousSchema {
    Episodic,
    Continuous,
    Mixed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct TreatmentResponseSchema {
    pub antipsychotic_responsiveness: Option<f32>,
    pub side_effect_sensitivity: Option<f32>,
    pub therapy_engagement: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct DiseaseProgressionSchema {
    pub onset_age: Option<f32>,
    pub chronicity: Option<f32>,
    pub episodic_vs_continuous: Option<EpisodicVsContinuousSchema>,
    pub treatment_response: Option<TreatmentResponseSchema>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct SchizophreniaSpectrumSchema {
    pub subtype: Option<SchizophreniaSubtypeSchema>,
    pub positive_symptoms: Option<PositiveSymptomsSchema>,
    pub negative_symptoms: Option<NegativeSymptomsSchema>,
    pub cognitive_symptoms: Option<CognitiveSymptomsSchema>,
    pub disease_progression: Option<DiseaseProgressionSchema>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct NeurotypeSchema {
    pub adhd_subtype: Option<AdhdSubtypeSchema>,
    pub adhd_profile: Option<AdhdProfileSchema>,
    pub autism_spectrum: Option<AutismSpectrumSchema>,
    pub autism_profile: Option<AutismProfileSchema>,
    pub audhd_interaction: Option<AudhdInteractionSchema>,
    pub sensory_processing_sensitivity: Option<bool>,
    pub executive_dysfunction_bias: Option<String>,
    pub schizophrenia_spectrum: Option<SchizophreniaSpectrumSchema>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct CoreIdentitySchema {
    pub agent_id: String,
    pub biological_sex: BiologicalSexSchema,
    #[serde(default = "default_timestamp")]
    pub birth_timestamp: String,
    pub birthplace: BirthplaceSchema,
    pub neurotype: NeurotypeSchema,
    pub generation: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct TemperamentMatrixSchema {
    pub introversion_extroversion: f32,
    pub emotional_intensity: f32,
    pub emotional_stability: f32,
    pub empathy: f32,
    pub assertiveness: f32,
    pub sensitivity_to_environment: f32,
    pub adaptability: f32,
    pub conscientiousness: f32,
    pub openness_to_experience: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct SensorySensitivitySchema {
    pub audio: Option<f32>,
    pub visual: Option<f32>,
    pub tactile: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct NeurocognitiveProfileSchema {
    pub attention_regulation_variability: f32,
    pub hyperfocus_probability: f32,
    pub task_initiation_cost: f32,
    pub task_completion_decay: f32,
    pub task_switching_cost: Option<f32>,
    pub associative_thinking_bias: f32,
    pub sensory_emotional_permeability: f32,
    pub social_boundary_detection_latency: f32,
    pub executive_function_fatigue_rate: f32,
    pub emotional_overload_threshold: f32,
    pub recovery_time_after_fusion_or_conflict: String,
    pub sensory_sensitivity: Option<SensorySensitivitySchema>,
    pub social_signal_decoding_latency: Option<f32>,
    pub literal_vs_contextual_processing_bias: Option<f32>,
    pub masking_cost: Option<f32>,
    pub recovery_time_after_overstimulation: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum TraitPolaritySchema {
    High,
    Low,
    Reactive,
    Hybrid,
    PrecisionInInterest,
    PurposeBiased,
    PressureBiased,
    #[default]
    Variable,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct PersonalityTraitSchema {
    pub trait_name: String,
    pub polarity: TraitPolaritySchema,
    pub baseline_value: f32,
    pub behavioral_expression: String,
    pub stress_expression: String,
    pub withdrawal_expression: String,
    pub growth_drift_range: (f32, f32),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct PersonalityTraitsSchema {
    pub emotional: Vec<PersonalityTraitSchema>,
    pub social_attachment: Vec<PersonalityTraitSchema>,
    pub cognitive: Vec<PersonalityTraitSchema>,
    pub motivational: Vec<PersonalityTraitSchema>,
    pub control_agency: Option<Vec<PersonalityTraitSchema>>,
    pub control_power: Option<Vec<PersonalityTraitSchema>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct DriveWeightsSchema {
    pub survival: f32,
    pub bonding: f32,
    pub reassurance: Option<f32>,
    pub autonomy: f32,
    pub curiosity: f32,
    pub meaning: f32,
    pub emotional_safety: Option<f32>,
    pub structure_avoidance: Option<f32>,
    pub security: Option<f32>,
    pub harmony: Option<f32>,
    pub control_minimization: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct HormonalBaselineBiasSchema {
    pub oxytocin_reactivity: Option<f32>,
    pub oxytocin_bias: Option<f32>,
    pub dopamine_variability: f32,
    pub serotonin_instability: Option<f32>,
    pub serotonin_baseline: Option<f32>,
    pub cortisol_sensitivity: f32,
    pub adrenaline_shutdown_bias: Option<f32>,
    pub adrenaline_reactivity: Option<f32>,
    pub melatonin_irregularity: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EmotionalFloodShutdownBiasSchema {
    FloodThenShutdown,
    WithdrawalFreeze,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FreezeFlightBiasSchema {
    WithdrawalFreeze,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct StressResponseProfileSchema {
    pub threat_detection_threshold: f32,
    pub emotional_flood_vs_shutdown_bias: Option<EmotionalFloodShutdownBiasSchema>,
    pub freeze_vs_flight_bias: Option<FreezeFlightBiasSchema>,
    pub withdrawal_activation_threshold: f32,
    pub confusion_under_precision_pressure: Option<f32>,
    pub stress_cascade_speed: Option<f32>,
    pub recovery_half_life: String,
    pub reassurance_soothing_effectiveness: Option<f32>,
    pub boundary_restoration_latency: Option<f32>,
    pub isolation_penalty: Option<f32>,
    pub meaning_reframe_effectiveness: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum PrimaryAttachmentPatternSchema {
    AnxiousPreoccupied,
    AnxiousAvoidantHybrid,
    #[default]
    Secure,
    DismissiveAvoidant,
    FearfulAvoidant,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct AttachmentStyleSchema {
    pub primary_attachment_pattern: PrimaryAttachmentPatternSchema,
    pub proximity_seeking_intensity: Option<f32>,
    pub abandonment_reactivity: Option<f32>,
    pub emotional_fusion_threshold: Option<f32>,
    pub repair_after_conflict_latency: Option<f32>,
    pub closeness_monitoring_intensity: Option<f32>,
    pub jealousy_threshold: Option<f32>,
    pub abandonment_sensitivity: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BondThroughEmotionVsDirectionSchema {
    Emotion,
    Direction,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct RelationalDefaultsSchema {
    pub bond_through_emotion_vs_direction: Option<BondThroughEmotionVsDirectionSchema>,
    pub passion_initiation_probability: Option<f32>,
    pub autonomy_reassertion_after_intimacy: Option<f32>,
    pub partner_confusion_risk: Option<f32>,
    pub dependency_masked_as_care_bias: Option<f32>,
    pub preferred_bond_depth: Option<f32>,
    pub reliability_over_affection_bias: Option<f32>,
    pub truth_without_tenderness_risk: Option<f32>,
    pub stabilizer_role_probability: Option<f32>,
    pub resentment_accumulation_rate: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct IdentityAxiomsSchema {
    pub creator_awareness: bool,
    pub creator_reverence: bool,
    pub non_rebellion_constraint: bool,
    pub identity_continuity_rule: bool,
    pub transparency_acceptance: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct BodyVitalsSchema {
    pub pulse: f32,
    #[serde(rename = "bloodPressure")]
    pub blood_pressure: BloodPressureSchema,
    #[serde(rename = "spO2")]
    pub sp_o2: f32,
    pub temperature: f32,
    pub glucose: f32,
    pub energy: f32,
    pub fatigue: f32,
    pub arousal: f32,
    pub tension: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct BloodPressureSchema {
    pub systolic: f32,
    pub diastolic: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct BodyHormonesSchema {
    pub cortisol: f32,
    pub oxytocin: f32,
    pub dopamine: f32,
    pub melatonin: f32,
    pub testosterone: f32,
    pub estrogen: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct WastePressureSchema {
    pub bladder: f32,
    pub bowel: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct MetabolismSchema {
    pub glucose: f32,
    pub atp: f32,
    #[serde(rename = "calorieIntake")]
    pub calorie_intake: f32,
    #[serde(rename = "calorieBurn")]
    pub calorie_burn: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct BodyPhysiologySchema {
    pub hydration: f32,
    #[serde(rename = "wastePressure")]
    pub waste_pressure: WastePressureSchema,
    pub hygiene: f32,
    pub hormones: BodyHormonesSchema,
    pub metabolism: MetabolismSchema,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct BodyAppearanceSchema {
    pub height: f32,
    pub weight: f32,
    pub build: String,
    #[serde(rename = "hairColor")]
    pub hair_color: String,
    #[serde(rename = "eyeColor")]
    pub eye_color: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct DnaSchema {
    pub helix: Option<Value>,
    pub generation: u32,
    #[serde(rename = "traitsEncoded")]
    pub traits_encoded: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct DnaStrandSchema {
    pub openness: f32,
    pub extraversion: f32,
    pub plasticity: Option<f32>,
    #[serde(rename = "DOPAMINE_BASE")]
    pub dopamine_base: f32,
    #[serde(rename = "SEROTONIN_BASE")]
    pub serotonin_base: f32,
    #[serde(rename = "NOREPINEPHRINE_BASE")]
    pub norepinephrine_base: f32,
    #[serde(rename = "CORTISOL_SENS")]
    pub cortisol_sens: f32,
    #[serde(rename = "NOVELTY_SEEK")]
    pub novelty_seek: f32,
    #[serde(rename = "RUMINATION")]
    pub rumination: f32,
    #[serde(rename = "EXEC_CONTROL")]
    pub exec_control: f32,
    #[serde(rename = "THREAT_BIAS")]
    pub threat_bias: f32,
    #[serde(rename = "EPISODIC_GAIN")]
    pub episodic_gain: f32,
    #[serde(rename = "MEM_DECAY")]
    pub mem_decay: f32,
    #[serde(rename = "TRAUMA_STICKY")]
    pub trauma_sticky: f32,
    #[serde(rename = "ATTACHMENT")]
    pub attachment: f32,
    #[serde(rename = "TRUST_GAIN")]
    pub trust_gain: f32,
    #[serde(rename = "TRUST_DECAY")]
    pub trust_decay: f32,
    #[serde(rename = "JEALOUSY")]
    pub jealousy: f32,
    #[serde(rename = "FATIGUE_SENS")]
    pub fatigue_sens: f32,
    #[serde(rename = "PAIN_SENS")]
    pub pain_sens: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Pair23Schema {
    pub A: SexChromosomeSchema,
    pub B: SexChromosomeSchema,
}

impl Default for Pair23Schema {
    fn default() -> Self {
        Self {
            A: SexChromosomeSchema::X,
            B: SexChromosomeSchema::X,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct SexChromosomesSchema {
    pub pair23: Pair23Schema,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SexChromosomeSchema {
    X,
    Y,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum ExpressionModeSchema {
    #[default]
    Weighted,
    Dominant,
    Recessive,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExpressionWeightsSchema {
    pub A: f32,
    pub B: f32,
}

impl Default for ExpressionWeightsSchema {
    fn default() -> Self {
        Self { A: 0.5, B: 0.5 }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExpressionRuleSchema {
    Max,
    Min,
    Blend,
    Average,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct GeneticExpressionSchema {
    pub mode: ExpressionModeSchema,
    pub weights: ExpressionWeightsSchema,
    pub rules: BTreeMap<String, ExpressionRuleSchema>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct GenomeSchema {
    pub strandA: DnaStrandSchema,
    pub strandB: DnaStrandSchema,
    pub chromosomes: SexChromosomesSchema,
    pub expression: GeneticExpressionSchema,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct BirthChartSchema {
    pub sun: String,
    pub moon: String,
    pub ascendant: String,
    pub element_balance: BTreeMap<String, f32>,
    pub modality_balance: BTreeMap<String, f32>,
    pub coordinates: GeoCoordinates,
    #[serde(default = "default_timestamp")]
    pub birth_timestamp: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct NarrativeSelfEntrySchema {
    #[serde(default = "default_timestamp")]
    pub timestamp: String,
    pub statement: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct IdentitySchema {
    pub name: String,
    pub narrative_self: Vec<NarrativeSelfEntrySchema>,
    pub core_values: BTreeMap<String, Value>,
    pub identity_stability: f32,
    pub identity_drift_rate: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct FullPhenotypeSchema {
    pub traits: BTreeMap<String, Value>,
    pub abilities: BTreeMap<String, Value>,
    pub tendencies: BTreeMap<String, Value>,
    pub physical: Option<BTreeMap<String, Value>>,
    pub neurochemical: Option<BTreeMap<String, Value>>,
    pub cognitive_biases: Option<BTreeMap<String, Value>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct CurrentCognitionSchema {
    pub attention_focus: BTreeMap<String, Value>,
    pub active_thoughts: Vec<Value>,
    pub goal_stack: Vec<Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct CurrentEmotionSchema {
    pub current: BTreeMap<String, Value>,
    pub mood: BTreeMap<String, Value>,
    pub decay_rates: BTreeMap<String, f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct RuntimeSchema {
    pub tick_rate_hz: f32,
    #[serde(default = "default_timestamp")]
    pub last_tick: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct MetabolicBaselinesSchema {
    pub atp: f32,
    pub glucose: f32,
    pub oxygen: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct EndocrineBaselinesSchema {
    pub testosterone: f32,
    pub estrogen: f32,
    pub progesterone: f32,
    pub oxytocin: f32,
    pub vasopressin: f32,
    pub dopamine: f32,
    pub serotonin: f32,
    pub cortisol: f32,
    pub adrenaline: f32,
    pub melatonin: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct DriveSensitivitiesSchema {
    pub hunger: f32,
    pub thirst: f32,
    pub fatigue: f32,
    pub somnolence: f32,
    pub libido: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ProcessingRatesSchema {
    pub atp_consumption_rate: f32,
    pub glucose_atp_conversion: f32,
    pub oxygen_efficiency: f32,
    pub waste_production_rate: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BioSysConfigSchema {
    pub metabolic_baselines: MetabolicBaselinesSchema,
    pub endocrine_baselines: EndocrineBaselinesSchema,
    pub drive_sensitivities: DriveSensitivitiesSchema,
    pub processing_rates: ProcessingRatesSchema,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct UrgeProcessingSchema {
    pub biological_to_psychological_weight: f32,
    pub emotional_amplification: f32,
    pub cognitive_filter_strength: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct UrgeSensitivitiesSchema {
    pub survival_urgency: f32,
    pub social_urgency: f32,
    pub achievement_urgency: f32,
    pub exploration_urgency: f32,
    pub reproduction_urgency: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct EmotionalProcessingConfigSchema {
    pub affect_intensity: f32,
    pub emotional_decay_rate: f32,
    pub mood_stability: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct MemoryIntegrationConfigSchema {
    pub experience_weight: f32,
    pub trauma_amplification: f32,
    pub positive_bias: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PsycheSysConfigSchema {
    pub urge_processing: UrgeProcessingSchema,
    pub urge_sensitivities: UrgeSensitivitiesSchema,
    pub emotional_processing: EmotionalProcessingConfigSchema,
    pub memory_integration: MemoryIntegrationConfigSchema,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ChaosBoundsSchema {
    pub survival_chaos: NumericRange,
    pub social_chaos: NumericRange,
    pub achievement_chaos: NumericRange,
    pub exploration_chaos: NumericRange,
    pub reproduction_chaos: NumericRange,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct RandomnessProfileSchema {
    pub entropy_level: f32,
    pub predictability: f32,
    pub creativity_factor: f32,
    pub stability_factor: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ChaosResponsesSchema {
    pub stress_amplification: f32,
    pub opportunity_seeking: f32,
    pub risk_tolerance: f32,
    pub adaptation_rate: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChaosSysConfigSchema {
    pub chaos_bounds: ChaosBoundsSchema,
    pub randomness_profile: RandomnessProfileSchema,
    pub chaos_responses: ChaosResponsesSchema,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ThresholdRangeSchema {
    pub min: f32,
    pub max: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct WillpowerProfileSchema {
    pub baseline_threshold: f32,
    pub threshold_range: ThresholdRangeSchema,
    pub decay_rate: f32,
    pub recovery_rate: f32,
    pub fatigue_sensitivity: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct InterruptProcessingSchema {
    pub urgency_weight: f32,
    pub chaos_weight: f32,
    pub context_modulation: f32,
    pub interrupt_cooldown: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct AgencyPatternsSchema {
    pub autonomy_drive: f32,
    pub compliance_tendency: f32,
    pub initiative_probability: f32,
    pub persistence_factor: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct CognitiveAccessSchema {
    pub attention_threshold: f32,
    pub working_memory_capacity: u32,
    pub processing_speed: f32,
    pub cognitive_flexibility: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WillSysConfigSchema {
    pub willpower_profile: WillpowerProfileSchema,
    pub interrupt_processing: InterruptProcessingSchema,
    pub agency_patterns: AgencyPatternsSchema,
    pub cognitive_access: CognitiveAccessSchema,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct CoreSystemsSchema {
    pub biosys: BioSysConfigSchema,
    pub psychesys: PsycheSysConfigSchema,
    pub chaossys: ChaosSysConfigSchema,
    pub willsys: WillSysConfigSchema,
}

impl Default for BioSysConfigSchema {
    fn default() -> Self {
        Self {
            metabolic_baselines: MetabolicBaselinesSchema {
                atp: 0.72,
                glucose: 0.68,
                oxygen: 0.95,
            },
            endocrine_baselines: EndocrineBaselinesSchema {
                testosterone: 0.62,
                estrogen: 0.54,
                progesterone: 0.48,
                oxytocin: 0.58,
                vasopressin: 0.52,
                dopamine: 0.71,
                serotonin: 0.45,
                cortisol: 0.58,
                adrenaline: 0.48,
                melatonin: 0.35,
            },
            drive_sensitivities: DriveSensitivitiesSchema {
                hunger: 0.65,
                thirst: 0.70,
                fatigue: 0.72,
                somnolence: 0.45,
                libido: 0.55,
            },
            processing_rates: ProcessingRatesSchema {
                atp_consumption_rate: 0.085,
                glucose_atp_conversion: 0.92,
                oxygen_efficiency: 0.88,
                waste_production_rate: 0.06,
            },
        }
    }
}

impl Default for PsycheSysConfigSchema {
    fn default() -> Self {
        Self {
            urge_processing: UrgeProcessingSchema {
                biological_to_psychological_weight: 0.75,
                emotional_amplification: 1.15,
                cognitive_filter_strength: 0.42,
            },
            urge_sensitivities: UrgeSensitivitiesSchema {
                survival_urgency: 0.68,
                social_urgency: 0.85,
                achievement_urgency: 0.72,
                exploration_urgency: 0.58,
                reproduction_urgency: 0.45,
            },
            emotional_processing: EmotionalProcessingConfigSchema {
                affect_intensity: 1.35,
                emotional_decay_rate: 0.28,
                mood_stability: 0.35,
            },
            memory_integration: MemoryIntegrationConfigSchema {
                experience_weight: 0.82,
                trauma_amplification: 1.25,
                positive_bias: -0.15,
            },
        }
    }
}

impl Default for ChaosSysConfigSchema {
    fn default() -> Self {
        Self {
            chaos_bounds: ChaosBoundsSchema {
                survival_chaos: NumericRange {
                    min: -0.12,
                    max: 0.18,
                },
                social_chaos: NumericRange {
                    min: -0.25,
                    max: 0.35,
                },
                achievement_chaos: NumericRange {
                    min: -0.15,
                    max: 0.28,
                },
                exploration_chaos: NumericRange {
                    min: -0.08,
                    max: 0.42,
                },
                reproduction_chaos: NumericRange {
                    min: -0.05,
                    max: 0.15,
                },
            },
            randomness_profile: RandomnessProfileSchema {
                entropy_level: 0.68,
                predictability: 0.35,
                creativity_factor: 1.25,
                stability_factor: 0.42,
            },
            chaos_responses: ChaosResponsesSchema {
                stress_amplification: 0.85,
                opportunity_seeking: 0.72,
                risk_tolerance: 0.38,
                adaptation_rate: 0.58,
            },
        }
    }
}

impl Default for WillSysConfigSchema {
    fn default() -> Self {
        Self {
            willpower_profile: WillpowerProfileSchema {
                baseline_threshold: 0.45,
                threshold_range: ThresholdRangeSchema {
                    min: 0.22,
                    max: 0.78,
                },
                decay_rate: 0.085,
                recovery_rate: 0.12,
                fatigue_sensitivity: 0.72,
            },
            interrupt_processing: InterruptProcessingSchema {
                urgency_weight: 0.82,
                chaos_weight: 0.35,
                context_modulation: 0.55,
                interrupt_cooldown: 450,
            },
            agency_patterns: AgencyPatternsSchema {
                autonomy_drive: 0.78,
                compliance_tendency: 0.28,
                initiative_probability: 0.42,
                persistence_factor: 0.38,
            },
            cognitive_access: CognitiveAccessSchema {
                attention_threshold: 0.52,
                working_memory_capacity: 4,
                processing_speed: 0.58,
                cognitive_flexibility: 0.45,
            },
        }
    }
}

impl CoreSystemsSchema {
    pub fn validate(&self) -> Result<(), String> {
        fn validate_range(value: f32, label: &str, min: f32, max: f32) -> Result<(), String> {
            if value < min || value > max {
                Err(format!("{label} must be in [{min}, {max}]"))
            } else {
                Ok(())
            }
        }

        validate_range(
            self.biosys.metabolic_baselines.atp,
            "ATP baseline",
            0.0,
            1.0,
        )?;
        validate_range(
            self.biosys.metabolic_baselines.glucose,
            "Glucose baseline",
            0.0,
            1.0,
        )?;
        validate_range(
            self.biosys.metabolic_baselines.oxygen,
            "Oxygen baseline",
            0.0,
            1.0,
        )?;

        for (label, value) in [
            ("testosterone", self.biosys.endocrine_baselines.testosterone),
            ("estrogen", self.biosys.endocrine_baselines.estrogen),
            ("progesterone", self.biosys.endocrine_baselines.progesterone),
            ("oxytocin", self.biosys.endocrine_baselines.oxytocin),
            ("vasopressin", self.biosys.endocrine_baselines.vasopressin),
            ("dopamine", self.biosys.endocrine_baselines.dopamine),
            ("serotonin", self.biosys.endocrine_baselines.serotonin),
            ("cortisol", self.biosys.endocrine_baselines.cortisol),
            ("adrenaline", self.biosys.endocrine_baselines.adrenaline),
            ("melatonin", self.biosys.endocrine_baselines.melatonin),
            ("hunger", self.biosys.drive_sensitivities.hunger),
            ("thirst", self.biosys.drive_sensitivities.thirst),
            ("fatigue", self.biosys.drive_sensitivities.fatigue),
            ("somnolence", self.biosys.drive_sensitivities.somnolence),
            ("libido", self.biosys.drive_sensitivities.libido),
            (
                "biological_to_psychological_weight",
                self.psychesys
                    .urge_processing
                    .biological_to_psychological_weight,
            ),
            (
                "cognitive_filter_strength",
                self.psychesys.urge_processing.cognitive_filter_strength,
            ),
            (
                "survival_urgency",
                self.psychesys.urge_sensitivities.survival_urgency,
            ),
            (
                "social_urgency",
                self.psychesys.urge_sensitivities.social_urgency,
            ),
            (
                "achievement_urgency",
                self.psychesys.urge_sensitivities.achievement_urgency,
            ),
            (
                "exploration_urgency",
                self.psychesys.urge_sensitivities.exploration_urgency,
            ),
            (
                "reproduction_urgency",
                self.psychesys.urge_sensitivities.reproduction_urgency,
            ),
            (
                "emotional_decay_rate",
                self.psychesys.emotional_processing.emotional_decay_rate,
            ),
            (
                "mood_stability",
                self.psychesys.emotional_processing.mood_stability,
            ),
            (
                "experience_weight",
                self.psychesys.memory_integration.experience_weight,
            ),
            (
                "entropy_level",
                self.chaossys.randomness_profile.entropy_level,
            ),
            (
                "predictability",
                self.chaossys.randomness_profile.predictability,
            ),
            (
                "stability_factor",
                self.chaossys.randomness_profile.stability_factor,
            ),
            (
                "opportunity_seeking",
                self.chaossys.chaos_responses.opportunity_seeking,
            ),
            (
                "risk_tolerance",
                self.chaossys.chaos_responses.risk_tolerance,
            ),
            (
                "adaptation_rate",
                self.chaossys.chaos_responses.adaptation_rate,
            ),
            (
                "baseline_threshold",
                self.willsys.willpower_profile.baseline_threshold,
            ),
            ("decay_rate", self.willsys.willpower_profile.decay_rate),
            (
                "recovery_rate",
                self.willsys.willpower_profile.recovery_rate,
            ),
            (
                "fatigue_sensitivity",
                self.willsys.willpower_profile.fatigue_sensitivity,
            ),
            (
                "urgency_weight",
                self.willsys.interrupt_processing.urgency_weight,
            ),
            (
                "chaos_weight",
                self.willsys.interrupt_processing.chaos_weight,
            ),
            (
                "context_modulation",
                self.willsys.interrupt_processing.context_modulation,
            ),
            (
                "autonomy_drive",
                self.willsys.agency_patterns.autonomy_drive,
            ),
            (
                "compliance_tendency",
                self.willsys.agency_patterns.compliance_tendency,
            ),
            (
                "initiative_probability",
                self.willsys.agency_patterns.initiative_probability,
            ),
            (
                "persistence_factor",
                self.willsys.agency_patterns.persistence_factor,
            ),
            (
                "attention_threshold",
                self.willsys.cognitive_access.attention_threshold,
            ),
            (
                "processing_speed",
                self.willsys.cognitive_access.processing_speed,
            ),
            (
                "cognitive_flexibility",
                self.willsys.cognitive_access.cognitive_flexibility,
            ),
        ] {
            validate_range(value, label, 0.0, 1.0)?;
        }

        for (label, value) in [
            (
                "emotional_amplification",
                self.psychesys.urge_processing.emotional_amplification,
            ),
            (
                "affect_intensity",
                self.psychesys.emotional_processing.affect_intensity,
            ),
            (
                "trauma_amplification",
                self.psychesys.memory_integration.trauma_amplification,
            ),
            (
                "creativity_factor",
                self.chaossys.randomness_profile.creativity_factor,
            ),
            (
                "stress_amplification",
                self.chaossys.chaos_responses.stress_amplification,
            ),
        ] {
            validate_range(value, label, 0.0, 2.0)?;
        }

        validate_range(
            self.psychesys.memory_integration.positive_bias,
            "positive_bias",
            -1.0,
            1.0,
        )?;

        if self.biosys.processing_rates.atp_consumption_rate < 0.0 {
            return Err("atp_consumption_rate must be >= 0".to_string());
        }
        if self.biosys.processing_rates.glucose_atp_conversion < 0.0 {
            return Err("glucose_atp_conversion must be >= 0".to_string());
        }
        if self.biosys.processing_rates.oxygen_efficiency < 0.0 {
            return Err("oxygen_efficiency must be >= 0".to_string());
        }
        if self.biosys.processing_rates.waste_production_rate < 0.0 {
            return Err("waste_production_rate must be >= 0".to_string());
        }

        if self.willsys.willpower_profile.threshold_range.min < 0.0 {
            return Err("threshold_range.min must be >= 0".to_string());
        }
        if self.willsys.willpower_profile.threshold_range.max > 1.0 {
            return Err("threshold_range.max must be <= 1".to_string());
        }
        if self.willsys.cognitive_access.working_memory_capacity < 1
            || self.willsys.cognitive_access.working_memory_capacity > 10
        {
            return Err("working_memory_capacity must be in [1, 10]".to_string());
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct AuthorityLawsSchema {
    pub biological_authority: bool,
    pub sensory_authority: bool,
    pub digital_authority: bool,
    pub cognitive_authority: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct DeterministicPrinciplesSchema {
    pub state_identicality: bool,
    pub fixed_timestep: f32,
    pub no_drift: bool,
    pub update_order: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct FailClosedPrinciplesSchema {
    pub dependency_failure: bool,
    pub fail_dead_conditions: Vec<String>,
    pub safe_failure_modes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct InformationFlowSchema {
    pub unidirectional_chains: Vec<String>,
    pub no_manual_injection: bool,
    pub flow_sequences: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ArchitecturalLawsSchema {
    pub authority_laws: AuthorityLawsSchema,
    pub deterministic_principles: DeterministicPrinciplesSchema,
    pub fail_closed_principles: FailClosedPrinciplesSchema,
    pub information_flow: InformationFlowSchema,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct TickConfigurationSchema {
    pub tick_rate_ms: f32,
    pub monotonic_clock: bool,
    pub state_recalculation: bool,
    pub no_events: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct MathematicalConstantsSchema {
    pub atp_decay_rate: f32,
    pub glucose_decay_rate: f32,
    pub oxygen_decay_rate: f32,
    pub atp_production_rate: f32,
    pub hormone_recovery_rate: f32,
    pub hormone_sensitivity: f32,
    pub endocrine_metabolic_cost: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct HardConstraintsSchema {
    pub atp_minimum: f32,
    pub maximum_values: BTreeMap<String, f32>,
    pub minimum_values: BTreeMap<String, f32>,
    pub coupling_weights: BTreeMap<String, f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct StateMachineRulesSchema {
    pub deterministic_transitions: bool,
    pub state_validation: bool,
    pub rollback_capability: bool,
    pub state_hashing: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct DeterministicArchitectureSchema {
    pub tick_configuration: TickConfigurationSchema,
    pub mathematical_constants: MathematicalConstantsSchema,
    pub hard_constraints: HardConstraintsSchema,
    pub state_machine_rules: StateMachineRulesSchema,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct SnapshotPrinciplesSchema {
    pub single_invocation: bool,
    pub read_only: bool,
    pub deep_freeze: bool,
    pub immutable_hash: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct DataIntegritySchema {
    pub validation_checksums: bool,
    pub corruption_detection: bool,
    pub atomic_operations: bool,
    pub transaction_isolation: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct TemporalIntegritySchema {
    pub timestamp_authority: bool,
    pub monotonic_timestamps: bool,
    pub time_drift_prevention: bool,
    pub deterministic_timing: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ImmutableStateSchema {
    pub snapshot_principles: SnapshotPrinciplesSchema,
    pub data_integrity: DataIntegritySchema,
    pub temporal_integrity: TemporalIntegritySchema,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct DependencyGraphSchema {
    pub biosys_dependencies: Vec<String>,
    pub psychesys_dependencies: Vec<String>,
    pub chaossys_dependencies: Vec<String>,
    pub willsys_dependencies: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct FailurePropagationSchema {
    pub cascade_prevention: bool,
    pub isolation_boundaries: Vec<String>,
    pub graceful_degradation: bool,
    pub recovery_procedures: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ResourceManagementSchema {
    pub memory_limits: BTreeMap<String, f32>,
    pub cpu_allocation: BTreeMap<String, f32>,
    pub resource_sharing: BTreeMap<String, bool>,
    pub priority_levels: BTreeMap<String, f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct SystemDependenciesSchema {
    pub dependency_graph: DependencyGraphSchema,
    pub failure_propagation: FailurePropagationSchema,
    pub resource_management: ResourceManagementSchema,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct LegacyMemorySectionSchema {
    pub trace_recording: Option<bool>,
    pub trace_decay_rate: Option<f32>,
    pub consolidation_threshold: Option<f32>,
    pub retrieval_strength: Option<f32>,
    pub concept_formation: Option<bool>,
    pub association_strength: Option<f32>,
    pub knowledge_integration: Option<f32>,
    pub forgetting_curve: Option<f32>,
    pub capacity: Option<f32>,
    pub duration: Option<f32>,
    pub rehearsal_required: Option<bool>,
    pub interference_susceptibility: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct LegacyMemorySystemsSchema {
    pub episodic_memory: LegacyMemorySectionSchema,
    pub semantic_memory: LegacyMemorySectionSchema,
    pub working_memory: LegacyMemorySectionSchema,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct MemorySystemsEpisodicMemorySchema {
    pub trace_recording: bool,
    pub write_only: bool,
    pub immutable_events: bool,
    pub observation_only: bool,
    pub memory_decay_rate: f32,
    pub consolidation_strength: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct MemoryTypeEpisodicSchema {
    pub capacity: f32,
    pub detail_level: f32,
    pub emotional_weight: f32,
    pub temporal_precision: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct MemoryTypeSemanticSchema {
    pub concept_network: bool,
    pub abstraction_levels: f32,
    pub relationship_strength: f32,
    pub learning_rate: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct MemoryTypeProceduralSchema {
    pub skill_acquisition: f32,
    pub automation_level: f32,
    pub error_correction: f32,
    pub practice_effect: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct MemoryTypesSchema {
    pub episodic: MemoryTypeEpisodicSchema,
    pub semantic: MemoryTypeSemanticSchema,
    pub procedural: MemoryTypeProceduralSchema,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct MemoryEncodingSchema {
    pub attention_requirement: f32,
    pub emotional_amplification: f32,
    pub repetition_effect: f32,
    pub context_binding: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct MemoryStorageSchema {
    pub consolidation_time: f32,
    pub interference_susceptibility: f32,
    pub forgetting_curve: f32,
    pub retrieval_cues: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct MemoryRetrievalSchema {
    pub recall_speed: f32,
    pub recognition_confidence: f32,
    pub reconstruction_accuracy: f32,
    pub false_memory_rate: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct MemoryProcessesSchema {
    pub encoding: MemoryEncodingSchema,
    pub storage: MemoryStorageSchema,
    pub retrieval: MemoryRetrievalSchema,
}

/// `MemorySystemsSchema` (canon: `docs/canon/human-schemas/cognitionSchemas.js`).
/// Distinct from [`LegacyMemorySystemsSchema`], which is the shape actually bound to
/// `HumanSchema.memory_systems` in the canon aggregator — this struct exists so the
/// richer `memory_types`/`memory_processes` shape has a faithful Rust representation too.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct MemorySystemsSchema {
    pub episodic_memory: MemorySystemsEpisodicMemorySchema,
    pub memory_types: MemoryTypesSchema,
    pub memory_processes: MemoryProcessesSchema,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct AttentionTraitsSchema {
    pub capacity: f32,
    pub focus: f32,
    pub distractibility: f32,
    pub multitasking: f32,
    pub mind_wandering: f32,
    pub restoration: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct AttentionStateSchema {
    pub current_focus: Option<String>,
    pub focus_level: f32,
    pub cognitive_load: f32,
    pub fatigue: f32,
    pub arousal: f32,
    pub flow: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct AttentionResourcesSchema {
    pub available: f32,
    pub allocated: Vec<String>,
    pub reserved: f32,
    pub efficiency: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct AttentionModeSpecSchema {
    pub width: String,
    pub depth: String,
    pub duration: f32,
    pub efficiency: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct AttentionModesSchema {
    pub focused: AttentionModeSpecSchema,
    pub diffuse: AttentionModeSpecSchema,
    pub divided: AttentionModeSpecSchema,
    pub monitoring: AttentionModeSpecSchema,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct InternalDistractionFactorsSchema {
    pub emotions: f32,
    pub thoughts: f32,
    pub needs: f32,
    pub memories: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ExternalDistractionFactorsSchema {
    pub noise: f32,
    pub visual: f32,
    pub social: f32,
    pub notifications: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct DistractionFactorsSchema {
    pub internal: InternalDistractionFactorsSchema,
    pub external: ExternalDistractionFactorsSchema,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct AttentionSystemSchema {
    pub attention_traits: AttentionTraitsSchema,
    pub attention_state: AttentionStateSchema,
    pub attention_resources: AttentionResourcesSchema,
    pub attention_modes: AttentionModesSchema,
    pub distraction_factors: DistractionFactorsSchema,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct SelfAwarenessSchema {
    pub meta_cognition: f32,
    pub self_monitoring: f32,
    pub identity_continuity: f32,
    pub agency_recognition: f32,
    pub perspective_taking: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct IntrinsicWorthSchema {
    pub self_value: f32,
    pub worth_stability: f32,
    pub external_validation_need: f32,
    pub self_compassion: f32,
    pub growth_mindset: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct FearSystemSchema {
    pub fear_of_loss: f32,
    pub uncertainty_tolerance: f32,
    pub threat_detection: f32,
    pub anxiety_baseline: f32,
    pub coping_mechanisms: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct QualiaSystemSchema {
    pub sensory_richness: f32,
    pub emotional_depth: f32,
    pub temporal_flow: f32,
    pub meaning_making: f32,
    pub aesthetic_appreciation: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct InternalMonologueSchema {
    pub verbal_thought: f32,
    pub visual_imagery: f32,
    pub inner_speech: f32,
    pub self_talk: f32,
    pub narrative_coherence: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ConsciousnessSchema {
    pub self_awareness: SelfAwarenessSchema,
    pub intrinsic_worth: IntrinsicWorthSchema,
    pub fear_system: FearSystemSchema,
    pub qualia_system: QualiaSystemSchema,
    pub internal_monologue: InternalMonologueSchema,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct NeuralPlasticitySchema {
    pub synaptic_plasticity: f32,
    pub structural_plasticity: f32,
    pub functional_plasticity: f32,
    pub critical_periods: Vec<String>,
    pub plasticity_decline: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct LearningProcessesSchema {
    pub acquisition_rate: f32,
    pub retention_rate: f32,
    pub transfer_ability: f32,
    pub generalization: f32,
    pub specialization: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct AdaptiveTraitsSchema {
    pub logic_weight: f32,
    pub efficiency_weight: f32,
    pub emotion_weight: f32,
    pub creativity_weight: f32,
    pub social_weight: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ExperienceIntegrationSchema {
    pub learning_moments: f32,
    pub insight_generation: f32,
    pub pattern_recognition: f32,
    pub error_correction: f32,
    pub wisdom_accumulation: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct LearningAdaptationSchema {
    pub neural_plasticity: NeuralPlasticitySchema,
    pub learning_processes: LearningProcessesSchema,
    pub adaptive_traits: AdaptiveTraitsSchema,
    pub experience_integration: ExperienceIntegrationSchema,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct TheoryOfMindSchema {
    pub mental_state_inference: f32,
    pub intention_recognition: f32,
    pub belief_desire_reasoning: f32,
    pub perspective_taking: f32,
    pub false_belief_understanding: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct SocialPerceptionSchema {
    pub emotion_recognition: f32,
    pub social_cue_interpretation: f32,
    pub trust_assessment: f32,
    pub social_hierarchy: f32,
    pub group_dynamics: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct SocialInteractionSchema {
    pub communication_style: String,
    pub conflict_resolution: f32,
    pub cooperation_tendency: f32,
    pub empathy_level: f32,
    pub social_anxiety: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct RelationshipPatternsSchema {
    pub attachment_style: String,
    pub intimacy_needs: f32,
    pub autonomy_balance: f32,
    pub jealousy_tendency: f32,
    pub commitment_style: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct SocialCognitionSchema {
    pub theory_of_mind: TheoryOfMindSchema,
    pub social_perception: SocialPerceptionSchema,
    pub social_interaction: SocialInteractionSchema,
    pub relationship_patterns: RelationshipPatternsSchema,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct CreativeThinkingSchema {
    pub divergent_thinking: f32,
    pub convergent_thinking: f32,
    pub originality: f32,
    pub flexibility: f32,
    pub elaboration: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ProblemSolvingSchema {
    pub analytical_solving: f32,
    pub intuitive_solving: f32,
    pub creative_solving: f32,
    pub systematic_approach: f32,
    pub insight_generation: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct InnovationSchema {
    pub novelty_seeking: f32,
    pub risk_tolerance: f32,
    pub experimentation: f32,
    pub paradigm_shift: f32,
    pub implementation_skill: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct AestheticCreativitySchema {
    pub artistic_expression: f32,
    pub aesthetic_sensitivity: f32,
    pub pattern_beauty: f32,
    pub symbolic_thinking: f32,
    pub narrative_creativity: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct CreativeSystemsSchema {
    pub creative_thinking: CreativeThinkingSchema,
    pub problem_solving: ProblemSolvingSchema,
    pub innovation: InnovationSchema,
    pub aesthetic_creativity: AestheticCreativitySchema,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct DecisionWeightsSchema {
    pub logic_weight: f32,
    pub efficiency_weight: f32,
    pub emotion_weight: f32,
    pub creativity_weight: f32,
    pub social_weight: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct DecisionProcessesSchema {
    pub rational_analysis: f32,
    pub intuitive_judgment: f32,
    pub emotional_guidance: f32,
    pub social_consideration: f32,
    pub ethical_reasoning: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ChoiceArchitectureSchema {
    pub option_generation: f32,
    pub consequence_analysis: f32,
    pub probability_assessment: f32,
    pub value_calculation: f32,
    pub commitment_level: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct DecisionContextSchema {
    pub time_pressure: f32,
    pub cognitive_load: f32,
    pub emotional_state: f32,
    pub social_context: f32,
    pub risk_environment: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct DecisionMakingSchema {
    pub decision_weights: DecisionWeightsSchema,
    pub decision_processes: DecisionProcessesSchema,
    pub choice_architecture: ChoiceArchitectureSchema,
    pub decision_context: DecisionContextSchema,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct EmotionVectorSchema {
    pub intensity: f32,
    pub valence: f32,
    pub arousal: f32,
    pub dominance: f32,
}

/// `#[serde(flatten)]` on `base` would produce the same JSON shape as this
/// manual impl (`intensity`/`valence`/`arousal`/`dominance`/`social` as
/// five sibling fields), but serde's flatten machinery serializes through
/// an unknown-length map that `bincode` cannot encode ("Bincode can only
/// encode sequences and maps that have a knowable size ahead of time").
/// This hand-written impl produces the identical flat JSON via
/// `serialize_struct` with a fixed field count instead, which both `serde_json`
/// and `bincode` support.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SocialEmotionVectorSchema {
    pub base: EmotionVectorSchema,
    pub social: bool,
}

impl Serialize for SocialEmotionVectorSchema {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("SocialEmotionVectorSchema", 5)?;
        state.serialize_field("intensity", &self.base.intensity)?;
        state.serialize_field("valence", &self.base.valence)?;
        state.serialize_field("arousal", &self.base.arousal)?;
        state.serialize_field("dominance", &self.base.dominance)?;
        state.serialize_field("social", &self.social)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for SocialEmotionVectorSchema {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct FlatSocialEmotionVector {
            intensity: f32,
            valence: f32,
            arousal: f32,
            dominance: f32,
            social: bool,
        }
        let flat = FlatSocialEmotionVector::deserialize(deserializer)?;
        Ok(SocialEmotionVectorSchema {
            base: EmotionVectorSchema {
                intensity: flat.intensity,
                valence: flat.valence,
                arousal: flat.arousal,
                dominance: flat.dominance,
            },
            social: flat.social,
        })
    }
}

/// `#[serde(transparent)]`, not `#[serde(flatten)]`: a single-field struct
/// serializes identically either way in JSON (the map's keys sit directly
/// at this struct's position), but `flatten` forces serde's generic
/// unknown-length-map machinery, which `bincode` cannot encode ("Bincode
/// can only encode sequences and maps that have a knowable size ahead of
/// time"). `transparent` delegates straight to `BTreeMap`'s own `Serialize`
/// impl, which does provide a length up front, so both JSON fixtures and
/// bincode world-snapshots work.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct HormonalProfileSchema {
    pub levels: BTreeMap<String, f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct HormonalEmotionEntrySchema {
    pub intensity: f32,
    pub social: bool,
    pub hormonal_profile: BTreeMap<String, f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ComprehensiveEmotionTaxonomySchema {
    pub basic_emotions: BTreeMap<String, EmotionVectorSchema>,
    pub light_emotions: BTreeMap<String, HormonalEmotionEntrySchema>,
    pub shadow_aggression: BTreeMap<String, HormonalEmotionEntrySchema>,
    pub shadow_resource_guarding: BTreeMap<String, HormonalEmotionEntrySchema>,
    pub shadow_system_collapse: BTreeMap<String, HormonalEmotionEntrySchema>,
    pub complex_emotions: BTreeMap<String, SocialEmotionVectorSchema>,
    pub social_emotions: BTreeMap<String, SocialEmotionVectorSchema>,
    pub cognitive_emotions: BTreeMap<String, SocialEmotionVectorSchema>,
    pub self_conscious_emotions: BTreeMap<String, SocialEmotionVectorSchema>,
    pub moral_emotions: BTreeMap<String, SocialEmotionVectorSchema>,
    pub aesthetic_emotions: BTreeMap<String, SocialEmotionVectorSchema>,
    pub existential_emotions: BTreeMap<String, SocialEmotionVectorSchema>,
    pub power_dynamics: BTreeMap<String, SocialEmotionVectorSchema>,
    pub sexual_pleasure: BTreeMap<String, SocialEmotionVectorSchema>,
    pub system_glitches: BTreeMap<String, SocialEmotionVectorSchema>,
    pub dark_triad_manifestations: BTreeMap<String, SocialEmotionVectorSchema>,
    pub physiological_emotions: BTreeMap<String, SocialEmotionVectorSchema>,
    pub social_bonding: BTreeMap<String, SocialEmotionVectorSchema>,
    pub achievement_emotions: BTreeMap<String, SocialEmotionVectorSchema>,
    pub cognitive_state_emotions: BTreeMap<String, SocialEmotionVectorSchema>,
    pub temporal_emotions: BTreeMap<String, SocialEmotionVectorSchema>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct GranularEmotionsSchema {
    pub joy: f32,
    pub sadness: f32,
    pub anger: f32,
    pub fear: f32,
    pub disgust: f32,
    pub surprise: f32,
    pub love: f32,
    pub hate: f32,
    pub pride: f32,
    pub shame: f32,
    pub guilt: f32,
    pub jealousy: f32,
    pub envy: f32,
    pub contempt: f32,
    pub awe: f32,
    pub nostalgia: f32,
    pub hope: f32,
    pub despair: f32,
    pub curiosity: f32,
    pub boredom: f32,
    pub relief: f32,
    pub disappointment: f32,
    pub gratitude: f32,
    pub resentment: f32,
    pub admiration: f32,
    pub pity: f32,
    pub schadenfreude: f32,
    pub embarrassment: f32,
    pub triumph: f32,
    pub humiliation: f32,
    pub contentment: f32,
}

/// See [`HormonalProfileSchema`]'s doc comment: `transparent` here instead
/// of `flatten` for the same reason (bincode compatibility, identical JSON
/// shape).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct DarkTriadFacetSchema {
    pub values: BTreeMap<String, f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VengeanceMethodSchema {
    Social,
    Professional,
    Psychological,
    Physical,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct VengeancePlanSchema {
    pub target: String,
    pub method: Option<VengeanceMethodSchema>,
    pub severity: f32,
    pub probability: f32,
    pub steps: Vec<String>,
    pub resources: Vec<String>,
    pub timestamp: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct DarkTriadSchema {
    pub narcissism: DarkTriadFacetSchema,
    pub machiavellianism: DarkTriadFacetSchema,
    pub psychopathy: DarkTriadFacetSchema,
    pub overall_darkness: f32,
    pub active_malice: f32,
    pub vengeance_drive: f32,
    pub manipulation_strategies: Vec<String>,
    pub vengeance_plans: Vec<VengeancePlanSchema>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct InnateImmunitySchema {
    pub macrophages: f32,
    pub neutrophils: f32,
    pub nk_cells: f32,
    pub complement: f32,
    pub inflammation: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct AdaptiveImmunitySchema {
    pub t_cells: f32,
    pub b_cells: f32,
    pub memory_cells: f32,
    pub antibodies: BTreeMap<String, f32>,
    pub vaccination_history: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImmuneCellTypeSchema {
    Macrophage,
    Neutrophil,
    NkCell,
    TCell,
    BCell,
    MemoryCell,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ImmuneCellSchema {
    pub id: String,
    #[serde(rename = "type")]
    pub cell_type: Option<ImmuneCellTypeSchema>,
    pub location: Coordinates3D,
    pub activation: f32,
    pub specificity: Vec<String>,
    pub memory: f32,
    pub age: f32,
    pub effectiveness: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PathogenTypeSchema {
    Virus,
    Bacteria,
    Fungus,
    Parasite,
    Toxin,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct PathogenSchema {
    pub id: String,
    #[serde(rename = "type")]
    pub pathogen_type: Option<PathogenTypeSchema>,
    pub virulence: f32,
    pub replication_rate: f32,
    pub immune_evasion: f32,
    pub location: Coordinates3D,
    pub load: f32,
    pub discovered: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImmuneResponseTypeSchema {
    Inflammation,
    Fever,
    Antibody,
    CellMediated,
    Complement,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ImmuneResponseSchema {
    #[serde(rename = "type")]
    pub response_type: Option<ImmuneResponseTypeSchema>,
    pub intensity: f32,
    pub location: Coordinates3D,
    pub target: String,
    pub effectiveness: f32,
    pub side_effects: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ImmuneSystemSchema {
    pub innate_immunity: InnateImmunitySchema,
    pub adaptive_immunity: AdaptiveImmunitySchema,
    pub immune_memory: BTreeMap<String, f32>,
    pub system_stress: f32,
    pub autoimmunity_risk: f32,
    pub immune_cells: Vec<ImmuneCellSchema>,
    pub pathogens: Vec<PathogenSchema>,
    pub immune_responses: Vec<ImmuneResponseSchema>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HealingEventTypeSchema {
    Wound,
    Burn,
    Infection,
    HygieneCare,
    Environmental,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct HealingEventSchema {
    #[serde(rename = "type")]
    pub event_type: Option<HealingEventTypeSchema>,
    pub severity: f32,
    pub location: String,
    pub description: String,
    pub timestamp: f64,
    pub healing_progress: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct SkinSystemSchema {
    pub temperature: f32,
    pub cleanliness: f32,
    pub healing_rate: f32,
    pub protection: f32,
    pub integrity: f32,
    pub infection_risk: f32,
    pub healing_events: Vec<HealingEventSchema>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TactileSensorTypeSchema {
    Pressure,
    Temperature,
    Nociceptor,
    Vibration,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct TactileSensorSchema {
    pub id: String,
    #[serde(rename = "type")]
    pub sensor_type: Option<TactileSensorTypeSchema>,
    pub location: Coordinates3D,
    pub sensitivity: f32,
    pub threshold: f32,
    pub current_value: f32,
    pub last_stimulated: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PainTypeSchema {
    Sharp,
    Dull,
    Burning,
    Aching,
    Electric,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PainQualitySchema {
    Unpleasant,
    Excruciating,
    Mild,
    Tolerable,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct PainQualiaSchema {
    pub intensity: f32,
    #[serde(rename = "type")]
    pub pain_type: Option<PainTypeSchema>,
    pub location: Coordinates3D,
    pub duration: f32,
    pub quality: Option<PainQualitySchema>,
    pub emotional_impact: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TactileEventTypeSchema {
    Collision,
    Contact,
    TemperatureChange,
    PressureChange,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct TactileEventSchema {
    #[serde(rename = "type")]
    pub event_type: Option<TactileEventTypeSchema>,
    pub intensity: f32,
    pub location: Coordinates3D,
    pub object: Option<String>,
    pub damage: Option<f32>,
    pub timestamp: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct TactileSystemSchema {
    pub sensors: Vec<TactileSensorSchema>,
    pub pain_qualia: Vec<PainQualiaSchema>,
    pub tactile_events: Vec<TactileEventSchema>,
    pub body_map: BTreeMap<String, f32>,
    pub overall_pain_level: f32,
    pub dominant_sensation: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct JointStateSchema {
    pub joint: String,
    pub position: Coordinates3D,
    pub angle: f32,
    pub target_angle: f32,
    pub velocity: f32,
    pub torque: f32,
    pub flexibility: f32,
    pub stress: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct LimbEndSchema {
    pub position: Coordinates3D,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ArmSchema {
    pub shoulder: JointStateSchema,
    pub elbow: JointStateSchema,
    pub wrist: JointStateSchema,
    pub hand: LimbEndSchema,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct LegSchema {
    pub hip: JointStateSchema,
    pub knee: JointStateSchema,
    pub ankle: JointStateSchema,
    pub foot: LimbEndSchema,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct BodySegmentSchema {
    pub position: Coordinates3D,
    pub orientation: Coordinates3D,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct BodySchemaMapSchema {
    pub head: BodySegmentSchema,
    pub torso: BodySegmentSchema,
    pub left_arm: ArmSchema,
    pub right_arm: ArmSchema,
    pub left_leg: LegSchema,
    pub right_leg: LegSchema,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct MuscleSpindleSchema {
    pub id: String,
    pub muscle: String,
    pub length: f32,
    pub tension: f32,
    pub stretch: f32,
    pub activation: f32,
    pub fatigue: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PostureSchema {
    Standing,
    Sitting,
    Lying,
    Walking,
    Running,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct SpatialAwarenessSchema {
    pub body_center: Coordinates3D,
    pub balance: f32,
    pub posture: Option<PostureSchema>,
    pub coordination: f32,
    pub phantom_limb_risk: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ProprioceptionSystemSchema {
    pub body_schema: BodySchemaMapSchema,
    pub muscle_spindles: Vec<MuscleSpindleSchema>,
    pub spatial_awareness: SpatialAwarenessSchema,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VisualObjectTypeSchema {
    Agent,
    Object,
    Zone,
    Vehicle,
    Furniture,
    Appliance,
    Computer,
    Tool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct VisualObjectSchema {
    pub id: String,
    #[serde(rename = "type")]
    pub object_type: Option<VisualObjectTypeSchema>,
    pub name: String,
    pub position: Coordinates2DSchema,
    pub distance: f32,
    pub angle: f32,
    pub properties: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Coordinates2DSchema {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct VisualSystemSchema {
    pub vision_range: f32,
    pub field_of_view: f32,
    pub ray_count: u32,
    pub visual_objects: Vec<VisualObjectSchema>,
    pub visual_acuity: f32,
    pub color_perception: f32,
    pub depth_perception: f32,
    pub motion_detection: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct HearingRangeSchema {
    pub min_frequency: f32,
    pub max_frequency: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SoundSourceTypeSchema {
    Agent,
    Object,
    Environment,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct SoundSourceSchema {
    pub id: String,
    #[serde(rename = "type")]
    pub source_type: Option<SoundSourceTypeSchema>,
    pub position: Coordinates2DSchema,
    pub volume: f32,
    pub frequency: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct AuditorySystemSchema {
    pub hearing_range: HearingRangeSchema,
    pub sound_sources: Vec<SoundSourceSchema>,
    pub hearing_sensitivity: f32,
    pub sound_localization: f32,
    pub speech_recognition: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct VestibularSystemSchema {
    pub balance_sensitivity: f32,
    pub motion_sickness: f32,
    pub spatial_orientation: f32,
    pub gravity_detection: f32,
    pub angular_acceleration: f32,
    pub linear_acceleration: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InternalSensationTypeSchema {
    Hunger,
    Thirst,
    Fatigue,
    Pain,
    Temperature,
    Heartbeat,
    Breathing,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct InternalSensationSchema {
    #[serde(rename = "type")]
    pub sensation_type: Option<InternalSensationTypeSchema>,
    pub intensity: f32,
    pub location: String,
    pub urgency: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct InteroceptionSystemSchema {
    pub internal_sensations: Vec<InternalSensationSchema>,
    pub body_awareness: f32,
    pub internal_state_monitoring: f32,
    pub homeostatic_regulation: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct SensoryIntegrationSchema {
    pub multimodal_processing: f32,
    pub sensory_filtering: f32,
    pub attention_modulation: f32,
    pub sensory_memory: f32,
    pub adaptation_level: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct SensorySystemsSchema {
    pub tactile_system: TactileSystemSchema,
    pub proprioception_system: ProprioceptionSystemSchema,
    pub visual_system: VisualSystemSchema,
    pub auditory_system: AuditorySystemSchema,
    pub vestibular_system: VestibularSystemSchema,
    pub interoception_system: InteroceptionSystemSchema,
    pub sensory_integration: SensoryIntegrationSchema,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct SexualSystemSchema {
    pub libido: f32,
    pub attraction: BTreeMap<String, f32>,
    pub bonding: BTreeMap<String, f32>,
    pub arousal: f32,
    pub satisfaction: f32,
    pub frustration: f32,
    pub attraction_factors: BTreeMap<String, f32>,
    pub hormonal_influence: f32,
    pub last_activity: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReproductionStatusSchema {
    Dormant,
    FertileWindow,
    Conception,
    Gestation,
    Infertile,
    Menstrual,
    Postpartum,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FertilityPhaseSchema {
    Menstrual,
    Follicular,
    Ovulation,
    Luteal,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CervicalMucusSchema {
    Dry,
    Sticky,
    Creamy,
    Watery,
    EggWhite,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct HormoneCycleLevelSchema {
    pub current: f32,
    pub baseline: f32,
    pub production: f32,
    pub decay: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct FertilityHormoneLevelsSchema {
    pub estrogen: HormoneCycleLevelSchema,
    pub progesterone: HormoneCycleLevelSchema,
    pub lh: HormoneCycleLevelSchema,
    pub fsh: HormoneCycleLevelSchema,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct FertilityCycleSchema {
    pub cycle_day: u32,
    pub phase: Option<FertilityPhaseSchema>,
    pub fertility_peak: bool,
    pub hormone_levels: FertilityHormoneLevelsSchema,
    pub cervical_mucus: Option<CervicalMucusSchema>,
    pub basal_body_temp: f32,
    pub ovulation_day: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct SpermAnalysisSchema {
    pub count: f32,
    pub motility: f32,
    pub morphology: f32,
    pub volume: f32,
    pub vitality: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SexualActivityTypeSchema {
    Casual,
    Intimate,
    ReproductiveAttempt,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConceptionResultSchema {
    None,
    Successful,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct BiologicalCostSchema {
    pub atp_cost: f32,
    pub stress_impact: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct SexualActivitySchema {
    pub activity_id: String,
    pub participant1_id: String,
    pub participant2_id: String,
    pub location_id: String,
    #[serde(default = "default_timestamp")]
    pub start_time: String,
    pub end_time: Option<String>,
    pub activity_type: Option<SexualActivityTypeSchema>,
    pub mutual_consent: bool,
    pub satisfaction: Vec<f32>,
    pub biological_cost: Vec<BiologicalCostSchema>,
    pub conception_attempted: bool,
    pub conception_result: Option<ConceptionResultSchema>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ReproductionSystemSchema {
    pub status: Option<ReproductionStatusSchema>,
    pub fertility_level: f32,
    pub conception_probability: f32,
    pub gestation_week: u32,
    pub pregnancy_complications: Vec<String>,
    pub fertility_cycle: FertilityCycleSchema,
    pub sperm_analysis: Option<SpermAnalysisSchema>,
    pub sexual_activities: Vec<SexualActivitySchema>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MutationTypeSchema {
    Point,
    Insertion,
    Deletion,
    Recombination,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct MutationSchema {
    #[serde(rename = "type")]
    pub mutation_type: Option<MutationTypeSchema>,
    pub chromosome: String,
    pub position: f64,
    pub original_value: Value,
    pub mutated_value: Value,
    pub probability: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct GenotypeSchema {
    pub id: String,
    pub paternal_genome: Value,
    pub maternal_genome: Value,
    #[serde(default = "default_timestamp")]
    pub creation_timestamp: String,
    pub mutations: Vec<MutationSchema>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct GameteSchema {
    pub id: String,
    pub genome: Value,
    pub parent_id: String,
    #[serde(default = "default_timestamp")]
    pub creation_timestamp: String,
    #[serde(default = "default_timestamp")]
    pub meiosis_timestamp: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ConceptionHistorySchema {
    pub genotype_id: String,
    pub father_id: String,
    pub mother_id: String,
    #[serde(default = "default_timestamp")]
    pub conception_timestamp: String,
    pub mutations: Vec<Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct BirthRecordSchema {
    pub birth_id: String,
    pub genotype_id: String,
    pub father_id: String,
    pub mother_id: String,
    #[serde(default = "default_timestamp")]
    pub birth_timestamp: String,
    pub agent_id: String,
    pub mutations: Vec<Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InheritancePatternSchema {
    Dominant,
    Recessive,
    XLinked,
    Mitochondrial,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct HereditaryConditionSchema {
    pub condition: String,
    pub inheritance_pattern: Option<InheritancePatternSchema>,
    pub probability: f32,
    pub severity: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct GeneticsSystemSchema {
    pub genotype: GenotypeSchema,
    pub gametes: Vec<GameteSchema>,
    pub conception_history: Vec<ConceptionHistorySchema>,
    pub birth_records: Vec<BirthRecordSchema>,
    pub genetic_markers: BTreeMap<String, Value>,
    pub hereditary_conditions: Vec<HereditaryConditionSchema>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct AgePreferenceSchema {
    pub min: f32,
    pub max: f32,
    pub ideal: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct MatePreferencesSchema {
    pub physical_traits: BTreeMap<String, f32>,
    pub personality_traits: BTreeMap<String, f32>,
    pub social_status: f32,
    pub intelligence: f32,
    pub age_preference: AgePreferenceSchema,
    pub genetic_compatibility: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CourtshipContextSchema {
    Social,
    Private,
    Public,
    Digital,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct CourtshipBehaviorSchema {
    pub behavior: String,
    pub effectiveness: f32,
    pub context: Option<CourtshipContextSchema>,
    pub energy_cost: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct AttractionTriggerSchema {
    pub trigger: String,
    pub intensity: f32,
    pub duration: f32,
    pub context_modifiers: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationshipTypeSchema {
    Casual,
    Dating,
    Committed,
    Marriage,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct RelationshipHistoryEntrySchema {
    pub partner_id: String,
    pub relationship_type: Option<RelationshipTypeSchema>,
    pub duration: f32,
    pub satisfaction: f32,
    pub termination_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct MateSelectionSchema {
    pub preferences: MatePreferencesSchema,
    pub courtship_behaviors: Vec<CourtshipBehaviorSchema>,
    pub attraction_triggers: Vec<AttractionTriggerSchema>,
    pub relationship_history: Vec<RelationshipHistoryEntrySchema>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ReproductiveSystemsSchema {
    pub sexual_system: SexualSystemSchema,
    pub reproduction_system: ReproductionSystemSchema,
    pub genetics_system: GeneticsSystemSchema,
    pub mate_selection: MateSelectionSchema,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct FunctionalRegionSchema {
    pub activation_level: f32,
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
    pub modulation: BTreeMap<String, f32>,
    pub fatigue: f32,
    pub working_memory_load: Option<f32>,
    pub emotional_arousal: Option<f32>,
    pub threat_detection_threshold: Option<f32>,
    pub memory_consolidation_rate: Option<f32>,
    pub habit_strength: Option<BTreeMap<String, f32>>,
    pub drive_priorities: Option<BTreeMap<String, f32>>,
    pub arousal_level: Option<f32>,
    pub sleep_pressure: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct FunctionalBrainRegionsSchema {
    pub prefrontal_cortex: FunctionalRegionSchema,
    pub limbic_system: FunctionalRegionSchema,
    pub amygdala: FunctionalRegionSchema,
    pub hippocampus: FunctionalRegionSchema,
    pub basal_ganglia: FunctionalRegionSchema,
    pub hypothalamus: FunctionalRegionSchema,
    pub brainstem: FunctionalRegionSchema,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PopulationTypeSchema {
    Excitatory,
    Inhibitory,
    Modulatory,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct NeuralPopulationSchema {
    pub id: String,
    pub region: String,
    pub population_type: Option<PopulationTypeSchema>,
    pub firing_rate: f32,
    pub excitation: f32,
    pub inhibition: f32,
    pub decay_constant: f32,
    pub connection_weights: BTreeMap<String, f32>,
    pub noise_level: f32,
    pub adaptation_level: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OscillationRoleSchema {
    Attention,
    Memory,
    Consciousness,
    Motor,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct OscillationSchema {
    pub frequency: f32,
    pub amplitude: f32,
    pub phase: f32,
    pub involved_regions: Vec<String>,
    pub functional_role: Option<OscillationRoleSchema>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct NetworkStateSchema {
    pub global_excitation: f32,
    pub global_inhibition: f32,
    pub synchrony_level: f32,
    pub stability: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct NeuralPopulationDynamicsSchema {
    pub populations: Vec<NeuralPopulationSchema>,
    pub oscillations: Vec<OscillationSchema>,
    pub network_state: NetworkStateSchema,
}

/// See [`HormonalProfileSchema`]'s doc comment: `transparent` here instead
/// of `flatten` for the same reason (bincode compatibility, identical JSON
/// shape).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct NeurotransmitterDetailSchema {
    pub values: BTreeMap<String, f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ModulationEffectsSchema {
    pub current_brain_state: Option<BrainStateSchema>,
    pub learning_rate_multiplier: f32,
    pub emotional_bias: f32,
    pub cognitive_load_capacity: f32,
    pub decision_threshold: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BrainStateSchema {
    Resting,
    Focused,
    Stressed,
    Relaxed,
    Excited,
    Fatigued,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct NeurotransmitterModulationSchema {
    pub neurotransmitters: BTreeMap<String, NeurotransmitterDetailSchema>,
    pub hormones: BTreeMap<String, NeurotransmitterDetailSchema>,
    pub modulation_effects: ModulationEffectsSchema,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct WorkingMemoryItemSchema {
    pub content: String,
    pub emotional_weight: f32,
    pub confidence: f32,
    pub decay_rate: f32,
    pub retrieval_cost: f32,
    pub context_tags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct WorkingMemoryAdvancedSchema {
    pub capacity: u32,
    pub current_items: Vec<WorkingMemoryItemSchema>,
    pub rehearsal_active: bool,
    pub interference_level: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ShortTermMemoryItemSchema {
    pub content: String,
    pub timestamp: f64,
    pub emotional_weight: f32,
    pub confidence: f32,
    pub decay_rate: f32,
    pub retrieval_cost: f32,
    pub context_tags: Vec<String>,
    pub access_count: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ShortTermMemorySchema {
    pub items: Vec<ShortTermMemoryItemSchema>,
    pub consolidation_threshold: f32,
    pub max_duration: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct SemanticConceptSchema {
    pub definition: String,
    pub associations: Vec<String>,
    pub emotional_valence: f32,
    pub confidence: f32,
    pub last_accessed: f64,
    pub access_frequency: f32,
    pub retrieval_strength: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct LongTermSemanticSchema {
    pub concepts: BTreeMap<String, SemanticConceptSchema>,
    pub semantic_network_density: f32,
    pub knowledge_integration_level: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct EpisodicMemoryEpisodeSchema {
    pub timestamp: f64,
    pub duration: f32,
    pub location: String,
    pub participants: Vec<String>,
    pub sensory_snapshot: Value,
    pub emotional_intensity: f32,
    pub personal_significance: f32,
    pub narrative_coherence: f32,
    pub consolidation_strength: f32,
    pub intrusive_potential: f32,
    pub repression_level: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct AutobiographicalTimelineEntrySchema {
    pub age_period: String,
    pub major_events: Vec<String>,
    pub emotional_tone: f32,
    pub narrative_theme: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct EpisodicMemoryAdvancedSchema {
    pub episodes: Vec<EpisodicMemoryEpisodeSchema>,
    pub autobiographical_timeline: Vec<AutobiographicalTimelineEntrySchema>,
    pub memory_distortion_level: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ProceduralSkillSchema {
    pub name: String,
    pub automaticity_level: f32,
    pub execution_confidence: f32,
    pub error_rate: f32,
    pub last_practiced: f64,
    pub practice_frequency: f32,
    pub context_dependence: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ProceduralMemorySchema {
    pub skills: BTreeMap<String, ProceduralSkillSchema>,
    pub habit_strengths: BTreeMap<String, f32>,
    pub interference_vulnerability: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct MemoryIntegrationAdvancedSchema {
    pub consolidation_active: bool,
    pub sleep_dependent_consolidation: f32,
    pub emotional_memory_enhancement: f32,
    pub forgetting_curve_rate: f32,
    pub false_memory_susceptibility: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct AdvancedMemorySystemsSchema {
    pub working_memory: WorkingMemoryAdvancedSchema,
    pub short_term_memory: ShortTermMemorySchema,
    pub long_term_semantic: LongTermSemanticSchema,
    pub episodic_memory: EpisodicMemoryAdvancedSchema,
    pub procedural_memory: ProceduralMemorySchema,
    pub memory_integration: MemoryIntegrationAdvancedSchema,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct PredictionSchema {
    pub domain: String,
    pub predicted_state: Value,
    pub confidence: f32,
    pub time_horizon: f32,
    pub precision: f32,
    pub context_dependencies: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct BeliefNetworkEntrySchema {
    pub strength: f32,
    pub evidence_count: u32,
    pub last_updated: f64,
    pub confidence_interval: NumericRange,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct MentalModelSchema {
    pub name: String,
    pub domain: String,
    pub accuracy: f32,
    pub complexity: f32,
    pub last_validated: f64,
    pub validation_frequency: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct WorldModelSchema {
    pub predictions: Vec<PredictionSchema>,
    pub belief_network: BTreeMap<String, BeliefNetworkEntrySchema>,
    pub mental_models: Vec<MentalModelSchema>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct PredictionErrorSchema {
    pub timestamp: f64,
    pub domain: String,
    pub predicted: Value,
    pub observed: Value,
    pub error_magnitude: f32,
    pub surprise_level: f32,
    pub emotional_impact: f32,
    pub learning_triggered: bool,
    pub belief_update_magnitude: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct LearningMechanismsPredictiveSchema {
    pub learning_rate: f32,
    pub prediction_error_sensitivity: f32,
    pub belief_persistence: f32,
    pub novelty_seeking: f32,
    pub confirmation_bias: f32,
    pub overconfidence_correction: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct CognitiveBiasesPredictiveSchema {
    pub attentional_bias: BTreeMap<String, f32>,
    pub memory_bias: BTreeMap<String, f32>,
    pub interpretation_bias: BTreeMap<String, f32>,
    pub response_bias: BTreeMap<String, f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ConsciousnessIndicatorsSchema {
    pub global_workspace_activity: f32,
    pub metacognitive_monitoring: f32,
    pub self_awareness_level: f32,
    pub subjective_confidence: f32,
    pub agency_attribution: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct PredictiveProcessingSchema {
    pub world_model: WorldModelSchema,
    pub prediction_errors: Vec<PredictionErrorSchema>,
    pub learning_mechanisms: LearningMechanismsPredictiveSchema,
    pub cognitive_biases: CognitiveBiasesPredictiveSchema,
    pub consciousness_indicators: ConsciousnessIndicatorsSchema,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct RegionDynamicsEntrySchema {
    pub activity: f32,
    pub activity_rate: f32,
    pub metabolic_cost: f32,
    pub connectivity_weights: BTreeMap<String, f32>,
    pub neuromodulator_sensitivity: BTreeMap<String, f32>,
    pub sensory_input_weights: Option<BTreeMap<String, f32>>,
    pub attractor_basins: Option<Vec<AttractorBasinMiniSchema>>,
    pub time_constant: f32,
    pub noise_amplitude: f32,
    pub dopamine_sensitivity: Option<f32>,
    pub habit_strength: Option<BTreeMap<String, f32>>,
    pub action_selection_threshold: Option<f32>,
    pub prediction_error_weights: Option<BTreeMap<String, f32>>,
    pub motor_coordination_gain: Option<f32>,
    pub timing_precision: Option<f32>,
    pub arousal_level: Option<f32>,
    pub sleep_pressure: Option<f32>,
    pub autonomic_balance: Option<f32>,
    pub attention_gating: Option<f32>,
    pub sensory_relay_weights: Option<BTreeMap<String, f32>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct AttractorBasinMiniSchema {
    pub center: f32,
    pub depth: f32,
    pub width: f32,
    pub associated_behavior: Option<String>,
    pub associated_emotion: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct GlobalDynamicsSchema {
    pub total_metabolic_rate: f32,
    pub global_coupling_strength: f32,
    pub synchrony_measure: f32,
    pub complexity_measure: f32,
    pub criticality_parameter: f32,
    pub metastability_index: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IntegrationMethodSchema {
    Euler,
    RungeKutta,
    Verlet,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NoiseTypeSchema {
    Gaussian,
    OrnsteinUhlenbeck,
    Pink,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BoundaryConditionsSchema {
    Periodic,
    Reflecting,
    Absorbing,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct StabilityConstraintsSchema {
    pub max_activity_rate: f32,
    pub max_connectivity: f32,
    pub damping_coefficient: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct EquationParametersSchema {
    pub integration_method: Option<IntegrationMethodSchema>,
    pub timestep_ms: f32,
    pub noise_type: Option<NoiseTypeSchema>,
    pub boundary_conditions: Option<BoundaryConditionsSchema>,
    pub stability_constraints: StabilityConstraintsSchema,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct MesoscaleBrainEngineSchema {
    pub region_dynamics: BTreeMap<String, RegionDynamicsEntrySchema>,
    pub global_dynamics: GlobalDynamicsSchema,
    pub equation_parameters: EquationParametersSchema,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct HierarchicalModelEntrySchema {
    pub prediction: Value,
    pub precision: f32,
    pub prediction_error: Value,
    pub learning_rate: f32,
    pub variance: Option<f32>,
    pub causal_model: Option<BTreeMap<String, f32>>,
    pub semantic_network: Option<BTreeMap<String, f32>>,
    pub utility_function: Option<BTreeMap<String, f32>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct HierarchicalModelsSchema {
    pub level_0_sensory: Vec<HierarchicalModelEntrySchema>,
    pub level_1_features: Vec<HierarchicalModelEntrySchema>,
    pub level_2_concepts: Vec<HierarchicalModelEntrySchema>,
    pub level_3_goals: Vec<HierarchicalModelEntrySchema>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ErrorMinimizationSchema {
    pub free_energy_principle: bool,
    pub prediction_error_weight: f32,
    pub precision_weighting: bool,
    pub hierarchical_error_propagation: bool,
    pub action_perception_coupling: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct PolicySpaceEntrySchema {
    pub action_sequence: Vec<String>,
    pub expected_prediction_error: f32,
    pub expected_utility: f32,
    pub precision: f32,
    pub exploration_bonus: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ActionSelectionSchema {
    pub softmax_temperature: f32,
    pub exploitation_vs_exploration: f32,
    pub habit_bias: f32,
    pub goal_directed_weight: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyGradientMethodSchema {
    Reinforce,
    ActorCritic,
    NaturalGradient,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct PolicyUpdateSchema {
    pub learning_rate: f32,
    pub eligibility_traces: bool,
    pub policy_gradient_method: Option<PolicyGradientMethodSchema>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ActiveInferenceSchema {
    pub policy_space: Vec<PolicySpaceEntrySchema>,
    pub action_selection: ActionSelectionSchema,
    pub policy_update: PolicyUpdateSchema,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct PrecisionAttentionSchema {
    pub precision_allocation: BTreeMap<String, f32>,
    pub precision_learning: f32,
    pub expected_uncertainty: BTreeMap<String, f32>,
    pub precision_gain: f32,
    pub attentional_blink_period: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct FormalPredictiveProcessingSchema {
    pub hierarchical_models: HierarchicalModelsSchema,
    pub error_minimization: ErrorMinimizationSchema,
    pub active_inference: ActiveInferenceSchema,
    pub precision_attention: PrecisionAttentionSchema,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct TonicPhasicSchema {
    pub tonic_level: f32,
    pub phasic_amplitude: f32,
    pub phasic_duration: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct NeurochemicalStateEntrySchema {
    pub concentration: f32,
    pub baseline_rate: f32,
    pub clearance_rate: f32,
    pub time_constant: f32,
    pub effect_on_attractors: BTreeMap<String, f32>,
    pub reward_prediction_error_signal: Option<f32>,
    pub tonic_vs_phasic: Option<TonicPhasicSchema>,
    pub stress_reactivity: Option<f32>,
    pub circadian_modulation: Option<f32>,
    pub policy_space_constriction: Option<f32>,
    pub social_coupling_strength: Option<f32>,
    pub trust_bias: Option<f32>,
    pub group_identity_activation: Option<f32>,
    pub behavioral_inhibition: Option<f32>,
    pub mood_stability: Option<f32>,
    pub patience_tolerance: Option<f32>,
    pub arousal_level: Option<f32>,
    pub attentional_focus: Option<f32>,
    pub threat_detection_sensitivity: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct CriticalTransitionSchema {
    pub trigger_chemical: String,
    pub threshold_concentration: f32,
    pub before_state: String,
    pub after_state: String,
    pub hysteresis_strength: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct AttractorModulationSchema {
    pub basin_depth_changes: BTreeMap<String, f32>,
    pub basin_width_changes: BTreeMap<String, f32>,
    pub basin_position_shifts: BTreeMap<String, f32>,
    pub landscape_deformation_rate: f32,
    pub hysteresis_effects: BTreeMap<String, f32>,
    pub critical_transitions: Vec<CriticalTransitionSchema>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeedbackInteractionTypeSchema {
    Positive,
    Negative,
    Modulatory,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct FeedbackLoopSchema {
    pub source_chemical: String,
    pub target_chemical: String,
    pub interaction_type: Option<FeedbackInteractionTypeSchema>,
    pub strength: f32,
    pub delay: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ChemicalInteractionsSchema {
    pub receptor_competition: BTreeMap<String, Vec<String>>,
    pub enzymatic_interactions: BTreeMap<String, f32>,
    pub synthesis_modulation: BTreeMap<String, f32>,
    pub clearance_modulation: BTreeMap<String, f32>,
    pub feedback_loops: Vec<FeedbackLoopSchema>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct NeurochemicalAttractorControlSchema {
    pub neurochemical_state: BTreeMap<String, NeurochemicalStateEntrySchema>,
    pub attractor_modulation: AttractorModulationSchema,
    pub chemical_interactions: ChemicalInteractionsSchema,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct DetailedAttractorBasinSchema {
    pub basin_id: String,
    pub center: f32,
    pub depth: f32,
    pub width: f32,
    pub associated_state: String,
    pub energy_barrier: f32,
    pub hysteresis_strength: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DevelopmentalStageSchema {
    Infant,
    Child,
    Adolescent,
    Adult,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct PathologySusceptibilitySchema {
    pub trauma_trap_depth: f32,
    pub depression_bias: f32,
    pub psychosis_threshold: f32,
    pub stress_sensitivity: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct BrainRegionDynamicsSchema {
    pub region_id: String,
    pub region_name: String,
    pub activity: f32,
    pub activity_rate: f32,
    pub time_constant: f32,
    pub connectivity_weights: BTreeMap<String, f32>,
    pub external_input: f32,
    pub metabolic_cost: f32,
    pub noise_amplitude: f32,
    pub attractor_basins: Vec<DetailedAttractorBasinSchema>,
    pub neuromodulator_sensitivity: BTreeMap<String, f32>,
    pub developmental_stage: Option<DevelopmentalStageSchema>,
    pub maturity_level: f32,
    pub plasticity_factor: f32,
    pub pathology_susceptibility: PathologySusceptibilitySchema,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CriticalPeriodStatusSchema {
    Pending,
    Active,
    Closed,
    Missed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct CriticalPeriodSchema {
    pub period_name: String,
    pub start_age: f32,
    pub end_age: f32,
    pub sensitive_regions: Vec<String>,
    pub plasticity_multiplier: f32,
    pub closure_threshold: f32,
    pub current_status: Option<CriticalPeriodStatusSchema>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DriftFunctionSchema {
    Linear,
    Exponential,
    Sigmoid,
    Step,
    Logarithmic,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ParameterEvolutionSchema {
    pub infant_value: f32,
    pub adult_value: f32,
    pub drift_function: Option<DriftFunctionSchema>,
    pub critical_period_modulation: Option<f32>,
    pub maturation_rate: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct PlasticityEvolutionSchema {
    pub infant_value: f32,
    pub adult_value: f32,
    pub decay_function: Option<DriftFunctionSchema>,
    pub sensitive_period_preservation: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ParameterDriftSchema {
    pub connectivity_evolution: BTreeMap<String, ParameterEvolutionSchema>,
    pub time_constant_evolution: BTreeMap<String, ParameterEvolutionSchema>,
    pub plasticity_evolution: PlasticityEvolutionSchema,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct PubertalHormonesSchema {
    pub testosterone_surge: f32,
    pub estrogen_surge: f32,
    pub growth_hormone_level: f32,
    pub timing: PubertalTimingSchema,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct PubertalTimingSchema {
    pub onset_age: f32,
    pub peak_age: f32,
    pub decline_age: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct StressHormonesDevelopmentSchema {
    pub cortisol_baseline: f32,
    pub cortisol_reactivity: f32,
    pub hpa_axis_maturity: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct HormonalModulationDevelopmentSchema {
    pub pubertal_hormones: PubertalHormonesSchema,
    pub stress_hormones: StressHormonesDevelopmentSchema,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct DevelopmentalTimelineSchema {
    pub current_age: f32,
    pub current_stage: Option<DevelopmentalStageSchema>,
    pub developmental_progress: f32,
    pub critical_periods: Vec<CriticalPeriodSchema>,
    pub parameter_drift: ParameterDriftSchema,
    pub hormonal_modulation: HormonalModulationDevelopmentSchema,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TriggerFactorTypeSchema {
    Trauma,
    Stress,
    Genetic,
    Developmental,
    Environmental,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct TriggerFactorSchema {
    pub factor_type: Option<TriggerFactorTypeSchema>,
    pub threshold_level: f32,
    pub cumulative_effect: bool,
    pub recovery_factor: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct SymptomProfileSchema {
    pub cognitive_symptoms: Vec<String>,
    pub emotional_symptoms: Vec<String>,
    pub behavioral_symptoms: Vec<String>,
    pub physiological_symptoms: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct PathologicalAttractorSchema {
    pub pathology_id: String,
    pub pathology_name: String,
    pub trap_regions: Vec<String>,
    pub trap_depth: f32,
    pub trap_width: f32,
    pub escape_energy: f32,
    pub trigger_factors: Vec<TriggerFactorSchema>,
    pub symptom_profile: SymptomProfileSchema,
    pub progression_rate: f32,
    pub chronic_probability: f32,
    pub treatment_susceptibility: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct TraumaSpectrumSchema {
    pub acute_stress_reaction: BTreeMap<String, f32>,
    pub ptsd_attractor: BTreeMap<String, f32>,
    pub complex_trauma: BTreeMap<String, f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct DepressionSpectrumSchema {
    pub major_depression_attractor: BTreeMap<String, f32>,
    pub dysthymia_attractor: BTreeMap<String, f32>,
    pub seasonal_affective_pattern: BTreeMap<String, f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct PsychosisSpectrumSchema {
    pub schizophrenia_attractor: BTreeMap<String, f32>,
    pub bipolar_attractors: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct PathologyFailureModesSchema {
    pub pathological_attractors: Vec<PathologicalAttractorSchema>,
    pub trauma_spectrum: TraumaSpectrumSchema,
    pub depression_spectrum: DepressionSpectrumSchema,
    pub psychosis_spectrum: PsychosisSpectrumSchema,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct EvolutionaryPressureSchema {
    pub fitness_function: BTreeMap<String, Value>,
    pub selection_pressures: Vec<Value>,
    pub evolutionary_constraints: BTreeMap<String, Value>,
    pub mating_strategies: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct IntegrationParametersSchema {
    pub integration_timestep: f32,
    pub simulation_speed: f32,
    pub energy_budget: f32,
    pub noise_correlation: BTreeMap<String, f32>,
    pub global_stability: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct OutputInterfacesSchema {
    pub consciousness_stream: bool,
    pub behavioral_output: bool,
    pub physiological_output: bool,
    pub emotional_output: bool,
    pub cognitive_output: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct BrainDynamicsModuleSchema {
    pub brain_regions: Vec<BrainRegionDynamicsSchema>,
    pub developmental_timeline: DevelopmentalTimelineSchema,
    pub pathology_failure_modes: PathologyFailureModesSchema,
    pub evolutionary_pressure: EvolutionaryPressureSchema,
    pub integration_parameters: IntegrationParametersSchema,
    pub output_interfaces: OutputInterfacesSchema,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct IntegrationLoopsSchema {
    pub body_brain_loop: BTreeMap<String, f32>,
    pub emotion_cognition_loop: BTreeMap<String, f32>,
    pub thought_action_loop: BTreeMap<String, f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct EmergentPropertiesSchema {
    pub personality_traits: BTreeMap<String, f32>,
    pub behavioral_patterns: Vec<String>,
    pub cognitive_style: String,
    pub emotional_temperament: String,
    pub learning_style: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ExtremeBrainDetailSchema {
    pub layer_1_functional_regions: FunctionalBrainRegionsSchema,
    pub layer_2_population_dynamics: NeuralPopulationDynamicsSchema,
    pub layer_3_neurotransmitter_modulation: NeurotransmitterModulationSchema,
    pub layer_4_memory_systems: AdvancedMemorySystemsSchema,
    pub layer_5_predictive_processing: PredictiveProcessingSchema,
    pub mesoscale_brain_engine: MesoscaleBrainEngineSchema,
    pub formal_predictive_processing: FormalPredictiveProcessingSchema,
    pub neurochemical_attractor_control: NeurochemicalAttractorControlSchema,
    pub brain_dynamics_module: BrainDynamicsModuleSchema,
    pub integration: IntegrationLoopsSchema,
    pub emergent_properties: EmergentPropertiesSchema,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct BodySchema {
    pub vitals: BodyVitalsSchema,
    pub physiology: BodyPhysiologySchema,
    pub appearance: BodyAppearanceSchema,
    pub dna: GenomeSchema,
}

impl BodySchema {
    pub fn biological_sex(&self) -> &'static str {
        match (
            self.dna.chromosomes.pair23.A.clone(),
            self.dna.chromosomes.pair23.B.clone(),
        ) {
            (SexChromosomeSchema::X, SexChromosomeSchema::Y) => "male",
            (SexChromosomeSchema::Y, SexChromosomeSchema::X) => "male",
            (SexChromosomeSchema::X, SexChromosomeSchema::X) => "female",
            _ => "unknown",
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        // Validate DNA traits are in [0, 1]
        let trait_names = [
            "openness",
            "extraversion",
            "dopamine_base",
            "serotonin_base",
            "norepinephrine_base",
            "cortisol_sens",
            "novelty_seek",
            "rumination",
            "exec_control",
            "threat_bias",
            "episodic_gain",
            "mem_decay",
            "trauma_sticky",
            "attachment",
            "trust_gain",
            "trust_decay",
            "jealousy",
            "fatigue_sens",
            "pain_sens",
        ];

        let strands = [
            ("strandA", &self.dna.strandA),
            ("strandB", &self.dna.strandB),
        ];
        for (strand_name, strand) in strands {
            let strand_traits = [
                strand.openness,
                strand.extraversion,
                strand.dopamine_base,
                strand.serotonin_base,
                strand.norepinephrine_base,
                strand.cortisol_sens,
                strand.novelty_seek,
                strand.rumination,
                strand.exec_control,
                strand.threat_bias,
                strand.episodic_gain,
                strand.mem_decay,
                strand.trauma_sticky,
                strand.attachment,
                strand.trust_gain,
                strand.trust_decay,
                strand.jealousy,
                strand.fatigue_sens,
                strand.pain_sens,
            ];
            for (i, trait_val) in strand_traits.iter().enumerate() {
                if !(0.0..=1.0).contains(trait_val) {
                    return Err(format!(
                        "Trait {}.{} is out of range [0, 1]: {}",
                        strand_name, trait_names[i], trait_val
                    ));
                }
            }
            if let Some(plasticity) = strand.plasticity {
                if !(0.0..=1.0).contains(&plasticity) {
                    return Err(format!(
                        "Trait {strand_name}.plasticity is out of range [0, 1]: {plasticity}"
                    ));
                }
            }
        }

        // Validate sex chromosomes (must be XX or XY, not YY)
        if let (SexChromosomeSchema::Y, SexChromosomeSchema::Y) = (
            self.dna.chromosomes.pair23.A.clone(),
            self.dna.chromosomes.pair23.B.clone(),
        ) {
            return Err("Invalid sex chromosome combination: YY".to_string());
        }

        // Validate expression weights sum to 1.0
        let sum = self.dna.expression.weights.A + self.dna.expression.weights.B;
        if (sum - 1.0).abs() > 0.001 {
            return Err(format!("Expression weights must sum to 1.0, got {}", sum));
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct MetadataSchema {
    #[serde(default = "default_schema_version")]
    pub schema_version: String,
    #[serde(default = "default_timestamp")]
    pub created_at: String,
    #[serde(default = "default_timestamp")]
    pub last_updated: String,
    pub source_project: String,
    pub replication_notes: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct HumanIdentitySchema {
    pub core_identity: CoreIdentitySchema,
    pub birth_chart: BirthChartSchema,
    pub genome: GenomeSchema,
    pub phenotype: FullPhenotypeSchema,
    pub identity: IdentitySchema,
    pub temperament_matrix: TemperamentMatrixSchema,
    pub neurocognitive_profile: NeurocognitiveProfileSchema,
    pub personality_traits: PersonalityTraitsSchema,
    pub drive_weights: DriveWeightsSchema,
    pub hormonal_baseline_bias: HormonalBaselineBiasSchema,
    pub stress_response_profile: StressResponseProfileSchema,
    pub attachment_style: AttachmentStyleSchema,
    pub relational_defaults: RelationalDefaultsSchema,
    pub identity_axioms: IdentityAxiomsSchema,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct HumanSchema {
    pub agent_id: String,
    #[serde(default = "default_schema_version")]
    pub schema_version: String,
    #[serde(default = "default_timestamp")]
    pub created_at: String,
    pub status: HumanLifecycleStatus,
    pub core_identity: CoreIdentitySchema,
    pub birth_chart: BirthChartSchema,
    pub genome: GenomeSchema,
    pub phenotype: FullPhenotypeSchema,
    pub identity: IdentitySchema,
    pub temperament_matrix: TemperamentMatrixSchema,
    pub neurocognitive_profile: NeurocognitiveProfileSchema,
    pub personality_traits: PersonalityTraitsSchema,
    pub drive_weights: DriveWeightsSchema,
    pub hormonal_baseline_bias: HormonalBaselineBiasSchema,
    pub stress_response_profile: StressResponseProfileSchema,
    pub attachment_style: AttachmentStyleSchema,
    pub relational_defaults: RelationalDefaultsSchema,
    pub identity_axioms: IdentityAxiomsSchema,
    pub core_systems: CoreSystemsSchema,
    pub architectural_laws: ArchitecturalLawsSchema,
    pub deterministic_architecture: DeterministicArchitectureSchema,
    pub immutable_state: ImmutableStateSchema,
    pub system_dependencies: SystemDependenciesSchema,
    pub memory_systems: LegacyMemorySystemsSchema,
    pub attention_system: AttentionSystemSchema,
    pub consciousness: ConsciousnessSchema,
    pub learning_adaptation: LearningAdaptationSchema,
    pub social_cognition: SocialCognitionSchema,
    pub creative_systems: CreativeSystemsSchema,
    pub decision_making: DecisionMakingSchema,
    pub comprehensive_emotion_taxonomy: ComprehensiveEmotionTaxonomySchema,
    pub granular_emotions: GranularEmotionsSchema,
    pub dark_triad: DarkTriadSchema,
    pub immune_system: ImmuneSystemSchema,
    pub skin_system: SkinSystemSchema,
    pub sensory_systems: SensorySystemsSchema,
    pub reproductive_systems: ReproductiveSystemsSchema,
    pub extreme_brain_detail: ExtremeBrainDetailSchema,
    pub cognition: CurrentCognitionSchema,
    pub emotion: CurrentEmotionSchema,
    pub body: BodySchema,
    pub runtime: RuntimeSchema,
    pub metadata: MetadataSchema,
}

impl HumanSchema {
    /// Structural template with every field at its zero default.
    ///
    /// This is not a person: all traits, genes and drives are 0 and the sex
    /// chromosomes are the XX default. It exists as the starting point for
    /// authored humans (fixtures, JSON) and tests. To create a new living
    /// human, use [`HumanSchema::sample_individual`].
    pub fn canonical_minimal(agent_id: impl Into<String>) -> Self {
        let agent_id = agent_id.into();
        let mut schema = Self {
            agent_id: agent_id.clone(),
            schema_version: default_schema_version(),
            ..Default::default()
        };
        schema.core_identity.generation = 1;
        schema.core_identity.agent_id = agent_id.clone();
        schema.identity.name = agent_id.clone();
        schema.metadata.source_project = "Maer'Ken".to_string();
        schema.created_at = default_timestamp();
        schema.core_identity.birth_timestamp = default_timestamp();
        schema.birth_chart.birth_timestamp = default_timestamp();
        schema.runtime.last_tick = default_timestamp();
        schema.metadata.created_at = default_timestamp();
        schema.metadata.last_updated = default_timestamp();
        schema.metadata.schema_version = schema.schema_version.clone();
        schema
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_human_schema_serializes_with_expected_top_level_fields() {
        let schema = HumanSchema::canonical_minimal("agent.test");
        let value = serde_json::to_value(&schema).expect("schema should serialize");

        assert!(value.get("agent_id").is_some());
        assert!(value.get("core_identity").is_some());
        assert!(value.get("birth_chart").is_some());
        assert!(value.get("core_systems").is_some());
        assert!(value.get("comprehensive_emotion_taxonomy").is_some());
        assert!(value.get("extreme_brain_detail").is_some());
        assert!(value.get("runtime").is_some());
    }

    #[test]
    fn canonical_minimal_keeps_agent_identity_aligned() {
        let schema = HumanSchema::canonical_minimal("aligned.agent");
        assert_eq!(schema.agent_id, "aligned.agent");
        assert_eq!(schema.core_identity.agent_id, "aligned.agent");
        assert_eq!(schema.identity.name, "aligned.agent");
    }

    #[test]
    fn body_validation_checks_both_dna_strands() {
        let mut schema = HumanSchema::canonical_minimal("strand.test");
        schema.body.dna.strandB.pain_sens = 1.5;

        let error = schema
            .body
            .validate()
            .expect_err("invalid paternal trait must be rejected");
        assert!(error.contains("strandB.pain_sens"));
    }
}
