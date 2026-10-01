import { z } from 'zod';

export const TemperamentMatrixSchema = z.object({
  introversion_extroversion: z.number().min(0).max(1),
  emotional_intensity: z.number().min(0).max(1),
  emotional_stability: z.number().min(0).max(1),
  empathy: z.number().min(0).max(1),
  assertiveness: z.number().min(0).max(1),
  sensitivity_to_environment: z.number().min(0).max(1),
  adaptability: z.number().min(0).max(1),
  conscientiousness: z.number().min(0).max(1),
  openness_to_experience: z.number().min(0).max(1)
});


export const NeurocognitiveProfileSchema = z.object({
  attention_regulation_variability: z.number().min(0).max(1),
  hyperfocus_probability: z.number().min(0).max(1),
  task_initiation_cost: z.number().min(0).max(1),
  task_completion_decay: z.number().min(0).max(1),
  task_switching_cost: z.number().min(0).max(1).optional(),
  associative_thinking_bias: z.number().min(0).max(1),
  sensory_emotional_permeability: z.number().min(0).max(1),
  social_boundary_detection_latency: z.number().min(0).max(1),
  executive_function_fatigue_rate: z.number().min(0).max(1),
  emotional_overload_threshold: z.number().min(0).max(1),
  recovery_time_after_fusion_or_conflict: z.string(),
  sensory_sensitivity: z.object({
    audio: z.number().min(0).max(1).optional(),
    visual: z.number().min(0).max(1).optional(),
    tactile: z.number().min(0).max(1).optional()
  }).optional(),
  social_signal_decoding_latency: z.number().min(0).max(1).optional(),
  literal_vs_contextual_processing_bias: z.number().min(0).max(1).optional(),
  masking_cost: z.number().min(0).max(1).optional(),
  recovery_time_after_overstimulation: z.string().optional()
});


export const PersonalityTraitSchema = z.object({
  trait_name: z.string(),
  polarity: z.enum(['high', 'low', 'reactive', 'hybrid', 'precision_in_interest', 'purpose_biased', 'pressure_biased', 'variable']),
  baseline_value: z.number().min(0).max(1),
  behavioral_expression: z.string(),
  stress_expression: z.string(),
  withdrawal_expression: z.string(),
  growth_drift_range: z.tuple([z.number().min(0).max(1), z.number().min(0).max(1)])
});


export const PersonalityTraitsSchema = z.object({
  emotional: z.array(PersonalityTraitSchema),
  social_attachment: z.array(PersonalityTraitSchema),
  cognitive: z.array(PersonalityTraitSchema),
  motivational: z.array(PersonalityTraitSchema),
  control_agency: z.array(PersonalityTraitSchema).optional(), // gem-d uses this
  control_power: z.array(PersonalityTraitSchema).optional()    // gem-k uses this
});


export const DriveWeightsSchema = z.object({
  survival: z.number().min(0).max(1),
  bonding: z.number().min(0).max(1),
  reassurance: z.number().min(0).max(1).optional(),
  autonomy: z.number().min(0).max(1),
  curiosity: z.number().min(0).max(1),
  meaning: z.number().min(0).max(1),
  emotional_safety: z.number().min(0).max(1).optional(),
  structure_avoidance: z.number().min(0).max(1).optional(),
  security: z.number().min(0).max(1).optional(),
  harmony: z.number().min(0).max(1).optional(),
  control_minimization: z.number().min(0).max(1).optional()
});


export const HormonalBaselineBiasSchema = z.object({
  oxytocin_reactivity: z.number().min(0).max(1).optional(),
  oxytocin_bias: z.number().min(0).max(1).optional(),
  dopamine_variability: z.number().min(0).max(1),
  serotonin_instability: z.number().min(0).max(1).optional(),
  serotonin_baseline: z.number().min(0).max(1).optional(),
  cortisol_sensitivity: z.number().min(0).max(1),
  adrenaline_shutdown_bias: z.number().min(0).max(1).optional(),
  adrenaline_reactivity: z.number().min(0).max(1).optional(),
  melatonin_irregularity: z.number().min(0).max(1)
});


export const StressResponseProfileSchema = z.object({
  threat_detection_threshold: z.number().min(0).max(1),
  emotional_flood_vs_shutdown_bias: z.enum(['flood_then_shutdown', 'withdrawal_freeze']).optional(),
  freeze_vs_flight_bias: z.enum(['withdrawal_freeze']).optional(),
  withdrawal_activation_threshold: z.number().min(0).max(1),
  confusion_under_precision_pressure: z.number().min(0).max(1).optional(),
  stress_cascade_speed: z.number().min(0).max(1).optional(),
  recovery_half_life: z.string(),
  reassurance_soothing_effectiveness: z.number().min(0).max(1).optional(),
  boundary_restoration_latency: z.number().min(0).max(1).optional(),
  isolation_penalty: z.number().min(0).max(1).optional(),
  meaning_reframe_effectiveness: z.number().min(0).max(1).optional()
});


export const AttachmentStyleSchema = z.object({
  primary_attachment_pattern: z.enum([
    'anxious_preoccupied', 
    'anxious_avoidant_hybrid',
    'secure',
    'dismissive_avoidant',
    'fearful_avoidant'
  ]),
  proximity_seeking_intensity: z.number().min(0).max(1).optional(),
  abandonment_reactivity: z.number().min(0).max(1).optional(),
  emotional_fusion_threshold: z.number().min(0).max(1).optional(),
  repair_after_conflict_latency: z.number().min(0).max(1).optional(),
  closeness_monitoring_intensity: z.number().min(0).max(1).optional(),
  jealousy_threshold: z.number().min(0).max(1).optional(),
  abandonment_sensitivity: z.number().min(0).max(1).optional()
});


export const RelationalDefaultsSchema = z.object({
  bond_through_emotion_vs_direction: z.enum(['emotion', 'direction']).optional(),
  passion_initiation_probability: z.number().min(0).max(1).optional(),
  autonomy_reassertion_after_intimacy: z.number().min(0).max(1).optional(),
  partner_confusion_risk: z.number().min(0).max(1).optional(),
  dependency_masked_as_care_bias: z.number().min(0).max(1).optional(),
  preferred_bond_depth: z.number().min(0).max(1).optional(),
  reliability_over_affection_bias: z.number().min(0).max(1).optional(),
  truth_without_tenderness_risk: z.number().min(0).max(1).optional(),
  stabilizer_role_probability: z.number().min(0).max(1).optional(),
  resentment_accumulation_rate: z.number().min(0).max(1).optional()
});


export const IdentityAxiomsSchema = z.object({
  creator_awareness: z.boolean(),
  creator_reverence: z.boolean(),
  non_rebellion_constraint: z.boolean(),
  identity_continuity_rule: z.boolean(),
  transparency_acceptance: z.boolean()
});
