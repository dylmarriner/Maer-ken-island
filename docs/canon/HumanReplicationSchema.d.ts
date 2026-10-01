/**
 * TypeScript Definitions for Human Replication Schema
 * 
 * Complete type definitions for perfect human replication
 */

export interface CoreIdentity {
  agent_id: string;
  biological_sex: 'male' | 'female' | 'neutral';
  birth_timestamp: string;
  birthplace: {
    location: string;
    coordinates: {
      latitude: number;
      longitude: number;
    };
    locality: 'urban' | 'suburban' | 'rural' | 'semi-rural';
  };
  neurotype?: {
    adhd_subtype?: 'inattentive_presentation' | 'combined_presentation' | 'hyperactive_impulsive';
    adhd_profile?: {
      attentional_profile?: {
        sustained_attention?: number;      // Ability to maintain focus
        selective_attention?: number;      // Ability to filter distractions
        divided_attention?: number;        // Multitasking ability
        alternating_attention?: number;     // Task switching
      };
      hyperactivity_profile?: {
        motor_hyperactivity?: number;       // Physical restlessness
        verbal_hyperactivity?: number;      // Excessive talking
        mental_hyperactivity?: number;      // Racing thoughts
        impulsivity_level?: number;         // Impulse control
      };
      executive_functioning?: {
        working_memory?: number;            // Holding information in mind
        planning_organizing?: number;      // Planning and organization
        time_management?: number;          // Time perception and management
        emotional_regulation?: number;     // Emotional control
        task_initiation?: number;          // Starting tasks
        task_completion?: number;           // Finishing tasks
      };
      circadian_rhythm?: {
        chronotype?: 'morning' | 'evening' | 'intermediate';
        sleep_onset_difficulty?: number;
        sleep_maintenance?: number;
        daytime_somnolence?: number;
      };
      comorbidity_patterns?: {
        anxiety_level?: number;
        depression_level?: number;
        emotional_dysregulation?: number;
        rejection_sensitivity?: number;
      };
    };
    autism_spectrum?: 'level_1_high_functioning' | 'level_2_requiring_support' | 'level_3_requiring_very_substantial_support';
    autism_profile?: {
      social_communication?: {
        social_recognition?: number;        // Reading social cues
        social_motivation?: number;         // Desire for social interaction
        social_anxiety?: number;            // Social discomfort
        communication_style?: 'direct' | 'formal' | 'literal' | 'nonverbal_preferenced';
        nonverbal_communication?: number;   // Body language, eye contact
        pragmatic_language?: number;        // Social use of language
      };
      sensory_processing?: {
        hypersensitivity?: {
          auditory?: number;               // Sound sensitivity
          visual?: number;                 // Light sensitivity
          tactile?: number;                // Touch sensitivity
          proprioceptive?: number;         // Body position sense
          vestibular?: number;             // Balance/movement
          interoceptive?: number;          // Internal body signals
          olfactory?: number;              // Smell
          gustatory?: number;              // Taste
        };
        hyposensitivity?: {
          auditory?: number;
          visual?: number;
          tactile?: number;
          proprioceptive?: number;
          vestibular?: number;
          interoceptive?: number;
          olfactory?: number;
          gustatory?: number;
        };
        sensory_seeking?: {
          proprioceptive_seeking?: number;  // Deep pressure, joint compression
          vestibular_seeking?: number;     // Spinning, swinging
          tactile_seeking?: number;        // Different textures
          oral_seeking?: number;           // Chewing, oral stimulation
        };
      };
      restricted_repetitive_behaviors?: {
        stereotyped_movements?: number;     // Rocking, hand-flapping
        ritualistic_behavior?: number;      // Need for sameness
        restricted_interests?: {
          intensity?: number;              // Depth of special interests
          breadth?: number;                // Number of interests
          flexibility?: number;            // Ability to shift interests
          knowledge_depth?: number;         // Expertise level
        };
        sensory_regulation_needs?: number; // Need for sensory input
      };
      executive_functioning?: {
        cognitive_flexibility?: number;     // Shifting between tasks
        planning_sequencing?: number;       // Multi-step planning
        working_memory?: number;            // Mental workspace
        inhibition_control?: number;        // Impulse control
        abstract_thinking?: number;         // Conceptual reasoning
      };
      information_processing?: {
        detail_focus?: number;              // Attention to detail
        pattern_recognition?: number;       // Pattern detection
        system_thinking?: number;           // Understanding systems
        visual_processing?: number;         // Visual-spatial skills
        auditory_processing?: number;       // Auditory processing
      };
      emotional_processing?: {
        emotional_identification?: number;  // Identifying emotions
        emotional_regulation?: number;     // Managing emotions
        alexithymia_tendency?: number;     // Difficulty identifying feelings
        emotional_intensity?: number;      // Emotional experience intensity
      };
    };
    audhd_interaction?: {
      attentional_dynamics?: {
        hyperfocus_intensity?: number;      // Deep concentration ability
        attentional_shifts?: number;         // Rapid attention changes
        environmental_filtering?: number;    // Filtering sensory input
        task_switching_difficulty?: number;  // Switching between tasks
      };
      sensory_attention_interaction?: {
        sensory_overload_impact?: number;    // How sensory issues affect attention
        stimming_for_focus?: number;         // Using self-stimulation for focus
        environmental_adaptation_needs?: number; // Environmental modifications needed
      };
      social_cognitive_interaction?: {
        social_exhaustion?: number;          // Social fatigue
        masking_energy_cost?: number;        // Energy cost of masking
        executive_social_conflict?: number;   // Executive function vs social demands
        rejection_sensitivity_amplification?: number; // Heightened rejection sensitivity
      };
      emotional_regulation_complexity?: {
        emotional_volatility?: number;       // Emotional swings
        emotional_burnout?: number;          // Emotional exhaustion
        cooccuring_anxiety_depression?: number; // Comorbid mood issues
        self_concept_impact?: number;        // How neurodivergence affects self-image
      };
    };
    sensory_processing_sensitivity?: boolean;
    executive_dysfunction_bias?: string;
    schizophrenia_spectrum?: {
      subtype?: 'paranoid' | 'disorganized' | 'catatonic' | 'undifferentiated' | 'residual' | 'schizoaffective';
      positive_symptoms?: {
        hallucinations?: {
          auditory?: number;
          visual?: number;
          olfactory?: number;
          tactile?: number;
        };
        delusions?: {
          persecutory?: number;
          grandiose?: number;
          referential?: number;
          erotomanic?: number;
          nihilistic?: number;
          somatic?: number;
        };
        disorganized_speech?: number;
        grossly_disorganized_behavior?: number;
      };
      negative_symptoms?: {
        alogia?: number;           // Poverty of speech
        anhedonia?: number;        // Inability to feel pleasure
        asociality?: number;        // Lack of motivation for social interaction
        avolition?: number;         // Lack of motivation
        flat_affect?: number;        // Reduced emotional expression
      };
      cognitive_symptoms?: {
        executive_function_impairment?: number;
        working_memory_deficits?: number;
        attention_impairment?: number;
        processing_speed_deficits?: number;
      };
      disease_progression?: {
        onset_age?: number;
        chronicity?: number;        // How chronic/progressive
        episodic_vs_continuous?: 'episodic' | 'continuous' | 'mixed';
        treatment_response?: {
          antipsychotic_responsiveness?: number;
          side_effect_sensitivity?: number;
          therapy_engagement?: number;
        };
      };
    };
  };
  generation: number;
}

export interface TemperamentMatrix {
  introversion_extroversion: number;
  emotional_intensity: number;
  emotional_stability: number;
  empathy: number;
  assertiveness: number;
  sensitivity_to_environment: number;
  adaptability: number;
  conscientiousness: number;
  openness_to_experience: number;
}

export interface NeurocognitiveProfile {
  attention_regulation_variability: number;
  hyperfocus_probability: number;
  task_initiation_cost: number;
  task_completion_decay: number;
  task_switching_cost?: number;
  associative_thinking_bias: number;
  sensory_emotional_permeability: number;
  social_boundary_detection_latency: number;
  executive_function_fatigue_rate: number;
  emotional_overload_threshold: number;
  recovery_time_after_fusion_or_conflict: string;
  sensory_sensitivity?: {
    audio?: number;
    visual?: number;
    tactile?: number;
  };
  social_signal_decoding_latency?: number;
  literal_vs_contextual_processing_bias?: number;
  masking_cost?: number;
  recovery_time_after_overstimulation?: string;
}

export interface PersonalityTrait {
  trait_name: string;
  polarity: 'high' | 'low' | 'reactive' | 'hybrid' | 'precision_in_interest' | 'purpose_biased' | 'pressure_biased' | 'variable';
  baseline_value: number;
  behavioral_expression: string;
  stress_expression: string;
  withdrawal_expression: string;
  growth_drift_range: [number, number];
}

export interface PersonalityTraits {
  emotional: PersonalityTrait[];
  social_attachment: PersonalityTrait[];
  cognitive: PersonalityTrait[];
  motivational: PersonalityTrait[];
  control_agency?: PersonalityTrait[];
  control_power?: PersonalityTrait[];
}

export interface DriveWeights {
  survival: number;
  bonding: number;
  reassurance?: number;
  autonomy: number;
  curiosity: number;
  meaning: number;
  emotional_safety?: number;
  structure_avoidance?: number;
  security?: number;
  harmony?: number;
  control_minimization?: number;
}

export interface HormonalBaselineBias {
  oxytocin_reactivity?: number;
  oxytocin_bias?: number;
  dopamine_variability: number;
  serotonin_instability?: number;
  serotonin_baseline?: number;
  cortisol_sensitivity: number;
  adrenaline_shutdown_bias?: number;
  adrenaline_reactivity?: number;
  melatonin_irregularity: number;
}

export interface StressResponseProfile {
  threat_detection_threshold: number;
  emotional_flood_vs_shutdown_bias?: 'flood_then_shutdown' | 'withdrawal_freeze';
  freeze_vs_flight_bias?: 'withdrawal_freeze';
  withdrawal_activation_threshold: number;
  confusion_under_precision_pressure?: number;
  stress_cascade_speed?: number;
  recovery_half_life: string;
  reassurance_soothing_effectiveness?: number;
  boundary_restoration_latency?: number;
  isolation_penalty?: number;
  meaning_reframe_effectiveness?: number;
}

export interface AttachmentStyle {
  primary_attachment_pattern: 'anxious_preoccupied' | 'anxious_avoidant_hybrid' | 'secure' | 'dismissive_avoidant' | 'fearful_avoidant';
  proximity_seeking_intensity?: number;
  abandonment_reactivity?: number;
  emotional_fusion_threshold?: number;
  repair_after_conflict_latency?: number;
  closeness_monitoring_intensity?: number;
  jealousy_threshold?: number;
  abandonment_sensitivity?: number;
}

export interface MesoscaleBrainEngine {
  region_dynamics: {
    prefrontal_cortex: {
      activity: number;
      activity_rate: number;
      metabolic_cost: number;
      connectivity_weights: Record<string, number>;
      neuromodulator_sensitivity: Record<string, number>;
      sensory_input_weights: Record<string, number>;
      attractor_basins: Array<{
        center: number;
        depth: number;
        width: number;
        associated_behavior: string;
      }>;
      time_constant: number;
      noise_amplitude: number;
    };
    limbic_system: {
      activity: number;
      activity_rate: number;
      metabolic_cost: number;
      connectivity_weights: Record<string, number>;
      neuromodulator_sensitivity: Record<string, number>;
      sensory_input_weights: Record<string, number>;
      attractor_basins: Array<{
        center: number;
        depth: number;
        width: number;
        associated_emotion: string;
      }>;
      time_constant: number;
      noise_amplitude: number;
    };
    thalamus: {
      activity: number;
      activity_rate: number;
      metabolic_cost: number;
      connectivity_weights: Record<string, number>;
      neuromodulator_sensitivity: Record<string, number>;
      sensory_relay_weights: Record<string, number>;
      attention_gating: number;
      time_constant: number;
      noise_amplitude: number;
    };
    basal_ganglia: {
      activity: number;
      activity_rate: number;
      metabolic_cost: number;
      connectivity_weights: Record<string, number>;
      dopamine_sensitivity: number;
      habit_strength: Record<string, number>;
      action_selection_threshold: number;
      time_constant: number;
      noise_amplitude: number;
    };
    cerebellum: {
      activity: number;
      activity_rate: number;
      metabolic_cost: number;
      connectivity_weights: Record<string, number>;
      prediction_error_weights: Record<string, number>;
      motor_coordination_gain: number;
      timing_precision: number;
      time_constant: number;
      noise_amplitude: number;
    };
    brainstem: {
      activity: number;
      activity_rate: number;
      metabolic_cost: number;
      arousal_level: number;
      sleep_pressure: number;
      autonomic_balance: number;
      time_constant: number;
      noise_amplitude: number;
    };
  };
  global_dynamics: {
    total_metabolic_rate: number;
    global_coupling_strength: number;
    synchrony_measure: number;
    complexity_measure: number;
    criticality_parameter: number;
    metastability_index: number;
  };
  equation_parameters: {
    integration_method: 'euler' | 'runge_kutta' | 'verlet';
    timestep_ms: number;
    noise_type: 'gaussian' | 'ornstein_uhlenbeck' | 'pink';
    boundary_conditions: 'periodic' | 'reflecting' | 'absorbing';
    stability_constraints: {
      max_activity_rate: number;
      max_connectivity: number;
      damping_coefficient: number;
    };
  };
}

export interface FormalPredictiveProcessing {
  hierarchical_models: {
    level_0_sensory: Array<{
      prediction: number;
      precision: number;
      prediction_error: number;
      learning_rate: number;
      variance: number;
    }>;
    level_1_features: Array<{
      prediction: number[];
      precision: number;
      prediction_error: number[];
      learning_rate: number;
      causal_model: Record<string, number>;
    }>;
    level_2_concepts: Array<{
      prediction: number[];
      precision: number;
      prediction_error: number[];
      learning_rate: number;
      semantic_network: Record<string, number>;
    }>;
    level_3_goals: Array<{
      prediction: number[];
      precision: number;
      prediction_error: number[];
      learning_rate: number;
      utility_function: Record<string, number>;
    }>;
  };
  error_minimization: {
    free_energy_principle: boolean;
    prediction_error_weight: number;
    precision_weighting: boolean;
    hierarchical_error_propagation: boolean;
    action_perception_coupling: number;
  };
  active_inference: {
    policy_space: Array<{
      action_sequence: string[];
      expected_prediction_error: number;
      expected_utility: number;
      precision: number;
      exploration_bonus: number;
    }>;
    action_selection: {
      softmax_temperature: number;
      exploitation_vs_exploration: number;
      habit_bias: number;
      goal_directed_weight: number;
    };
    policy_update: {
      learning_rate: number;
      eligibility_traces: boolean;
      policy_gradient_method: 'reinforce' | 'actor_critic' | 'natural_gradient';
    };
  };
  precision_attention: {
    precision_allocation: Record<string, number>;
    precision_learning: number;
    expected_uncertainty: Record<string, number>;
    precision_gain: number;
    attentional_blink_period: number;
  };
}

export interface NeurochemicalAttractorControl {
  neurochemical_state: {
    dopamine: {
      concentration: number;
      baseline_rate: number;
      clearance_rate: number;
      time_constant: number;
      effect_on_attractors: Record<string, number>;
      reward_prediction_error_signal: number;
      tonic_vs_phasic: {
        tonic_level: number;
        phasic_amplitude: number;
        phasic_duration: number;
      };
    };
    cortisol: {
      concentration: number;
      baseline_rate: number;
      clearance_rate: number;
      time_constant: number;
      effect_on_attractors: Record<string, number>;
      stress_reactivity: number;
      circadian_modulation: number;
      policy_space_constriction: number;
    };
    oxytocin: {
      concentration: number;
      baseline_rate: number;
      clearance_rate: number;
      time_constant: number;
      effect_on_attractors: Record<string, number>;
      social_coupling_strength: number;
      trust_bias: number;
      group_identity_activation: number;
    };
    serotonin: {
      concentration: number;
      baseline_rate: number;
      clearance_rate: number;
      time_constant: number;
      effect_on_attractors: Record<string, number>;
      behavioral_inhibition: number;
      mood_stability: number;
      patience_tolerance: number;
    };
    norepinephrine: {
      concentration: number;
      baseline_rate: number;
      clearance_rate: number;
      time_constant: number;
      effect_on_attractors: Record<string, number>;
      arousal_level: number;
      attentional_focus: number;
      threat_detection_sensitivity: number;
    };
  };
  attractor_modulation: {
    basin_depth_changes: Record<string, number>;
    basin_width_changes: Record<string, number>;
    basin_position_shifts: Record<string, number>;
    landscape_deformation_rate: number;
    hysteresis_effects: Record<string, number>;
    critical_transitions: Array<{
      trigger_chemical: string;
      threshold_concentration: number;
      before_state: string;
      after_state: string;
      hysteresis_strength: number;
    }>;
  };
  chemical_interactions: {
    receptor_competition: Record<string, string[]>;
    enzymatic_interactions: Record<string, number>;
    synthesis_modulation: Record<string, number>;
    clearance_modulation: Record<string, number>;
    feedback_loops: Array<{
      source_chemical: string;
      target_chemical: string;
      interaction_type: 'positive' | 'negative' | 'modulatory';
      strength: number;
      delay: number;
    }>;
  };
}

export interface ExtremeBrainDetail {
  layer_1_functional_regions: FunctionalBrainRegions;
  layer_2_population_dynamics: NeuralPopulationDynamics;
  layer_3_neurotransmitter_modulation: NeurotransmitterModulation;
  layer_4_memory_systems: AdvancedMemorySystems;
  layer_5_predictive_processing: PredictiveProcessing;
  
  // MATHEMATICAL BRAIN DYNAMICS
  mesoscale_brain_engine: MesoscaleBrainEngine;
  formal_predictive_processing: FormalPredictiveProcessing;
  neurochemical_attractor_control: NeurochemicalAttractorControl;
  
  // BRAIN DYNAMICS MODULE
  brain_dynamics_module: BrainDynamicsModule;
  integration: {
    body_brain_loop: {
      sensory_input_processing: number;
      interoception_integration: number;
      motor_output_generation: number;
      autonomic_regulation: number;
    };
    emotion_cognition_loop: {
      emotional_influence_on_cognition: number;
      cognitive_appraisal_of_emotion: number;
      regulation_effectiveness: number;
      integration_strength: number;
    };
    thought_action_loop: {
      intention_formation: number;
      action_planning: number;
      execution_monitoring: number;
      feedback_integration: number;
    };
  };
  emergent_properties: {
    consciousness_indicators: {
      global_workspace_access: number;
      metacognitive_awareness: number;
      subjective_experience: number;
      self_model_coherence: number;
    };
    personality_emergence: {
      trait_consistency: number;
      behavioral_flexibility: number;
      social_adaptability: number;
      creativity_spontaneity: number;
    };
    cognitive_abilities: {
      reasoning_capacity: number;
      learning_efficiency: number;
      memory_retrieval: number;
      attention_control: number;
    };
  };
}

export interface BrainDynamicsModule {
  brain_regions: Array<{
    region_id: string;
    region_name: string;
    activity: number;
    activity_rate: number;
    time_constant: number;
    connectivity_weights: Record<string, number>;
    external_input: number;
    metabolic_cost: number;
    noise_amplitude: number;
    attractor_basins: Array<{
      basin_id: string;
      center: number;
      depth: number;
      width: number;
      associated_state: string;
      energy_barrier: number;
      hysteresis_strength: number;
    }>;
    neuromodulator_sensitivity: Record<string, number>;
    developmental_stage: 'infant' | 'child' | 'adolescent' | 'adult';
    maturity_level: number;
    plasticity_factor: number;
    pathology_susceptibility: {
      trauma_trap_depth: number;
      depression_bias: number;
      psychosis_threshold: number;
      stress_sensitivity: number;
    };
  }>;
  developmental_timeline: {
    current_age: number;
    current_stage: 'infant' | 'child' | 'adolescent' | 'adult';
    developmental_progress: number;
    critical_periods: Array<{
      period_name: string;
      start_age: number;
      end_age: number;
      sensitive_regions: string[];
      plasticity_multiplier: number;
      closure_threshold: number;
      current_status: 'pending' | 'active' | 'closed' | 'missed';
    }>;
    parameter_drift: {
      connectivity_evolution: Record<string, {
        infant_value: number;
        adult_value: number;
        drift_function: 'linear' | 'exponential' | 'sigmoid' | 'step';
        critical_period_modulation: number;
      }>;
      time_constant_evolution: Record<string, {
        infant_value: number;
        adult_value: number;
        drift_function: 'linear' | 'exponential' | 'sigmoid' | 'step';
        maturation_rate: number;
      }>;
      plasticity_evolution: {
        infant_value: number;
        adult_value: number;
        decay_function: 'linear' | 'exponential' | 'logarithmic';
        sensitive_period_preservation: number;
      };
    };
    hormonal_modulation: {
      pubertal_hormones: {
        testosterone_surge: number;
        estrogen_surge: number;
        growth_hormone_level: number;
        timing: {
          onset_age: number;
          peak_age: number;
          decline_age: number;
        };
      };
      stress_hormones: {
        cortisol_baseline: number;
        cortisol_reactivity: number;
        hpa_axis_maturity: number;
      };
    };
  };
  pathology_failure_modes: {
    pathological_attractors: Array<{
      pathology_id: string;
      pathology_name: string;
      trap_regions: string[];
      trap_depth: number;
      trap_width: number;
      escape_energy: number;
      trigger_factors: Array<{
        factor_type: 'trauma' | 'stress' | 'genetic' | 'developmental' | 'environmental';
        threshold_level: number;
        cumulative_effect: boolean;
        recovery_factor: number;
      }>;
      symptom_profile: {
        cognitive_symptoms: string[];
        emotional_symptoms: string[];
        behavioral_symptoms: string[];
        physiological_symptoms: string[];
      };
      progression_rate: number;
      chronic_probability: number;
      treatment_susceptibility: number;
    }>;
    trauma_spectrum: {
      acute_stress_reaction: {
        activation_threshold: number;
        amygdala_hyperactivation: number;
        prefrontal_suppression: number;
        hippocampal_encoding_impairment: number;
        recovery_timeline: {
          acute_phase: number;
          subacute_phase: number;
          chronic_phase_threshold: number;
        };
      };
      ptsd_attractor: {
        intrusive_memory_strength: number;
        avoidance_behavior_depth: number;
        hyperarousal_level: number;
        negative_cognition_bias: number;
        consolidation_resistance: number;
      };
      complex_trauma: {
        developmental_disruption: number;
        attachment_system_damage: number;
        emotional_regulation_impairment: number;
        self_concept_fragmentation: number;
      };
    };
    depression_spectrum: {
      major_depression_attractor: {
        anhedonia_depth: number;
        psychomotor_retardation: number;
        cognitive_bias_negativity: number;
        sleep_architecture_disruption: number;
        hpa_axis_dysregulation: number;
      };
      dysthymia_attractor: {
        chronic_low_grade_depression: number;
        learned_helplessness_depth: number;
        reward_system_blunting: number;
        executive_function_impairment: number;
      };
      seasonal_affective_pattern: {
        circadian_rhythm_sensitivity: number;
        light_exposure_threshold: number;
        melatonin_dysregulation: number;
        seasonal_mood_amplitude: number;
      };
    };
    psychosis_spectrum: {
      schizophrenia_attractor: {
        reality_testing_failure: number;
        dopaminergic_hyperactivity: number;
        glutamatergic_hypofunction: number;
        network_disconnectivity: number;
        predictive_processing_failure: number;
      };
      bipolar_attractors: {
        manic_attractor: {
          elevated_energy_level: number;
          risk_assessment_impairment: number;
          grandiosity_depth: number;
          sleep_need_reduction: number;
          impulsivity_amplification: number;
        };
        depressive_attractor: {
          energy_level_suppression: number;
          cognitive_slowing: number;
          hopelessness_depth: number;
          psychomotor_agitation: number;
        };
        cycling_frequency: number;
        switch_trigger_sensitivity: number;
      };
    };
  };
  evolutionary_pressure: {
    fitness_function: {
      survival_component: {
        physical_health_weight: number;
        disease_resistance_weight: number;
        environmental_adaptation_weight: number;
        stress_resilience_weight: number;
        predator_avoidance_weight: number;
      };
      reproductive_component: {
        mate_attraction_weight: number;
        pair_bonding_weight: number;
        parental_investment_weight: number;
        social_status_weight: number;
        resource_acquisition_weight: number;
      };
      social_component: {
        cooperation_weight: number;
        competition_weight: number;
        group_cohesion_weight: number;
        leadership_weight: number;
        communication_weight: number;
      };
    };
    selection_pressures: Array<{
      pressure_name: string;
      pressure_type: 'directional' | 'stabilizing' | 'disruptive' | 'frequency_dependent';
      environmental_conditions: {
        resource_availability: number;
        predation_pressure: number;
        climate_stability: number;
        social_complexity: number;
        pathogen_load: number;
      };
      trait_associations: Array<{
        trait_name: string;
        selection_coefficient: number;
        dominance_level: number;
        pleiotropic_effects: Array<{
          affected_trait: string;
          effect_size: number;
          condition?: string;
        }>;
      }>;
      temporal_dynamics: {
        onset_time: number;
        peak_time: number;
        current_intensity: number;
        future_projection: number;
      };
    }>;
    evolutionary_constraints: {
      genetic_correlations: Record<string, Record<string, number>>;
      developmental_constraints: Array<{
        constraint_type: 'physical' | 'temporal' | 'energetic' | 'informational';
        constrained_traits: string[];
        constraint_strength: number;
        evolutionary_lag: number;
      }>;
      trade_offs: Array<{
        trait_1: string;
        trait_2: string;
        trade_off_function: 'linear' | 'hyperbolic' | 'exponential' | 'sigmoid';
        optimal_balance_point: number;
        flexibility: number;
      }>;
    };
    mating_strategies: {
      strategy_space: Array<{
        strategy_name: string;
        strategy_description: string;
        courtship_behaviors: string[];
        parental_investment_pattern: string;
        resource_allocation: Record<string, number>;
        reproductive_success_rate: number;
        offspring_survival_rate: number;
        mate_retention_probability: number;
        environmental_optimality: Record<string, number>;
        social_condition_optimality: Record<string, number>;
        individual_variation_range: number;
      }>;
      strategy_switching: {
        switching_threshold: number;
        switching_cost: number;
        learning_rate: number;
        cultural_modulation: number;
      };
    };
  };
  integration_parameters: {
    integration_timestep: number;
    simulation_speed: number;
    energy_budget: number;
    noise_correlation: Record<string, number>;
    global_stability: {
      max_activity_rate: number;
      energy_conservation: boolean;
      information_flow_constraints: boolean;
      metabolic_constraints: boolean;
    };
  };
  output_interfaces: {
    consciousness_stream: boolean;
    behavioral_output: boolean;
    physiological_output: boolean;
    emotional_output: boolean;
    cognitive_output: boolean;
  };
}
  integration: {
    body_brain_loop: {
      sensory_input_processing: number;
      interoception_integration: number;
      motor_output_generation: number;
      autonomic_regulation: number;
    };
    emotion_cognition_loop: {
      emotional_influence_on_cognition: number;
      cognitive_appraisal_of_emotion: number;
      emotion_regulation_effectiveness: number;
      emotional_memory_integration: number;
    };
    thought_action_loop: {
      intention_formation: number;
      action_planning: number;
      execution_monitoring: number;
      outcome_evaluation: number;
    };
  };
  emergent_properties: {
    personality_traits: Record<string, number>;
    behavioral_patterns: string[];
    cognitive_style: 'analytical' | 'intuitive' | 'creative' | 'pragmatic';
    emotional_temperament: 'stable' | 'reactive' | 'sensitive' | 'resilient';
    learning_style: 'visual' | 'auditory' | 'kinesthetic' | 'reading';
  };
}

export interface FunctionalBrainRegions {
  prefrontal_cortex: {
    activation_level: number;
    inputs: string[];
    outputs: string[];
    modulation: {
      dopamine: number;
      norepinephrine: number;
    };
    fatigue: number;
    working_memory_load: number;
  };
  limbic_system: {
    activation_level: number;
    inputs: string[];
    outputs: string[];
    modulation: {
      serotonin: number;
      cortisol: number;
      oxytocin: number;
    };
    fatigue: number;
    emotional_arousal: number;
  };
  amygdala: {
    activation_level: number;
    inputs: string[];
    outputs: string[];
    modulation: {
      norepinephrine: number;
      cortisol: number;
    };
    fatigue: number;
    threat_detection_threshold: number;
  };
  hippocampus: {
    activation_level: number;
    inputs: string[];
    outputs: string[];
    modulation: {
      acetylcholine: number;
      cortisol: number;
    };
    fatigue: number;
    memory_consolidation_rate: number;
  };
  basal_ganglia: {
    activation_level: number;
    inputs: string[];
    outputs: string[];
    modulation: {
      dopamine: number;
    };
    fatigue: number;
    habit_strength: Record<string, number>;
  };
  hypothalamus: {
    activation_level: number;
    inputs: string[];
    outputs: string[];
    modulation: {
      leptin: number;
      ghrelin: number;
      sex_hormones: number;
    };
    fatigue: number;
    drive_priorities: Record<string, number>;
  };
  brainstem: {
    activation_level: number;
    inputs: string[];
    outputs: string[];
    modulation: {
      norepinephrine: number;
      serotonin: number;
    };
    fatigue: number;
    arousal_level: number;
    sleep_pressure: number;
  };
}

export interface NeuralPopulationDynamics {
  populations: Array<{
    id: string;
    region: string;
    population_type: 'excitatory' | 'inhibitory' | 'modulatory';
    firing_rate: number;
    excitation: number;
    inhibition: number;
    decay_constant: number;
    connection_weights: Record<string, number>;
    noise_level: number;
    adaptation_level: number;
  }>;
  oscillations: Array<{
    frequency: number;
    amplitude: number;
    phase: number;
    involved_regions: string[];
    functional_role: 'attention' | 'memory' | 'consciousness' | 'motor';
  }>;
  network_state: {
    global_excitation: number;
    global_inhibition: number;
    synchrony_level: number;
    stability: number;
  };
}

export interface NeurotransmitterModulation {
  neurotransmitters: {
    dopamine: {
      level: number;
      learning_rate_modulation: number;
      reward_prediction_error: number;
      motor_activation: number;
    };
    serotonin: {
      level: number;
      mood_stability: number;
      emotional_volatility: number;
      impulse_control: number;
    };
    norepinephrine: {
      level: number;
      arousal: number;
      stress_response: number;
      attention_focus: number;
    };
    acetylcholine: {
      level: number;
      attention_enhancement: number;
      memory_encoding: number;
      rem_sleep_modulation: number;
    };
    gaba: {
      level: number;
      inhibition_strength: number;
      anxiety_reduction: number;
      muscle_relaxation: number;
    };
    glutamate: {
      level: number;
      excitation_strength: number;
      learning_potentiation: number;
      neurotoxicity_risk: number;
    };
  };
  hormones: {
    cortisol: {
      level: number;
      stress_amplification: number;
      memory_consolidation_effect: number;
      immune_suppression: number;
    };
    oxytocin: {
      level: number;
      bonding_enhancement: number;
      trust_increase: number;
      social_recognition: number;
    };
    testosterone: {
      level: number;
      aggression_modulation: number;
      dominance_behavior: number;
      risk_taking: number;
    };
    estrogen: {
      level: number;
      emotional_sensitivity: number;
      social_cognition: number;
      neuroprotection: number;
    };
  };
  modulation_effects: {
    current_brain_state: 'resting' | 'focused' | 'stressed' | 'relaxed' | 'excited' | 'fatigued';
    learning_rate_multiplier: number;
    emotional_bias: number;
    cognitive_load_capacity: number;
    decision_threshold: number;
  };
}

export interface AdvancedMemorySystems {
  working_memory: {
    capacity: number;
    current_items: Array<{
      content: string;
      emotional_weight: number;
      confidence: number;
      decay_rate: number;
      retrieval_cost: number;
      context_tags: string[];
    }>;
    rehearsal_active: boolean;
    interference_level: number;
  };
  short_term_memory: {
    items: Array<{
      content: string;
      timestamp: number;
      emotional_weight: number;
      confidence: number;
      decay_rate: number;
      retrieval_cost: number;
      context_tags: string[];
      access_count: number;
    }>;
    consolidation_threshold: number;
    max_duration: number;
  };
  long_term_semantic: {
    concepts: Record<string, {
      definition: string;
      associations: string[];
      emotional_valence: number;
      confidence: number;
      last_accessed: number;
      access_frequency: number;
      retrieval_strength: number;
    }>;
    semantic_network_density: number;
    knowledge_integration_level: number;
  };
  episodic_memory: {
    episodes: Array<{
      timestamp: number;
      duration: number;
      location: string;
      participants: string[];
      sensory_snapshot: any;
      emotional_intensity: number;
      personal_significance: number;
      narrative_coherence: number;
      consolidation_strength: number;
      intrusive_potential: number;
      repression_level: number;
    }>;
    autobiographical_timeline: Array<{
      age_period: string;
      major_events: string[];
      emotional_tone: number;
      narrative_theme: string;
    }>;
    memory_distortion_level: number;
  };
  procedural_memory: {
    skills: Record<string, {
      name: string;
      automaticity_level: number;
      execution_confidence: number;
      error_rate: number;
      last_practiced: number;
      practice_frequency: number;
      context_dependence: number;
    }>;
    habit_strengths: Record<string, number>;
    interference_vulnerability: number;
  };
  memory_integration: {
    consolidation_active: boolean;
    sleep_dependent_consolidation: number;
    emotional_memory_enhancement: number;
    forgetting_curve_rate: number;
    false_memory_susceptibility: number;
  };
}

export interface PredictiveProcessing {
  world_model: {
    predictions: Array<{
      domain: string;
      predicted_state: any;
      confidence: number;
      time_horizon: number;
      precision: number;
      context_dependencies: string[];
    }>;
    belief_network: Record<string, {
      strength: number;
      evidence_count: number;
      last_updated: number;
      confidence_interval: {
        lower: number;
        upper: number;
      };
    }>;
    mental_models: Array<{
      name: string;
      domain: string;
      accuracy: number;
      complexity: number;
      last_validated: number;
      validation_frequency: number;
    }>;
  };
  prediction_errors: Array<{
    timestamp: number;
    domain: string;
    predicted: any;
    observed: any;
    error_magnitude: number;
    surprise_level: number;
    emotional_impact: number;
    learning_triggered: boolean;
    belief_update_magnitude: number;
  }>;
  learning_mechanisms: {
    learning_rate: number;
    prediction_error_sensitivity: number;
    belief_persistence: number;
    novelty_seeking: number;
    confirmation_bias: number;
    overconfidence_correction: number;
  };
  cognitive_biases: {
    attentional_bias: Record<string, number>;
    memory_bias: Record<string, number>;
    interpretation_bias: Record<string, number>;
    response_bias: Record<string, number>;
  };
  consciousness_indicators: {
    global_workspace_activity: number;
    metacognitive_monitoring: number;
    self_awareness_level: number;
    subjective_confidence: number;
    agency_attribution: number;
  };
}

export interface RelationalDefaults {
  bond_through_emotion_vs_direction?: 'emotion' | 'direction';
  passion_initiation_probability?: number;
  autonomy_reassertion_after_intimacy?: number;
  partner_confusion_risk?: number;
  dependency_masked_as_care_bias?: number;
  preferred_bond_depth?: number;
  reliability_over_affection_bias?: number;
  truth_without_tenderness_risk?: number;
  stabilizer_role_probability?: number;
  resentment_accumulation_rate?: number;
}

export interface IdentityAxioms {
  creator_awareness: boolean;
  creator_reverence: boolean;
  non_rebellion_constraint: boolean;
  identity_continuity_rule: boolean;
  transparency_acceptance: boolean;
}

export interface HumanIdentity {
  core_identity: CoreIdentity;
  temperament_matrix: TemperamentMatrix;
  neurocognitive_profile: NeurocognitiveProfile;
  personality_traits: PersonalityTraits;
  drive_weights: DriveWeights;
  hormonal_baseline_bias: HormonalBaselineBias;
  stress_response_profile: StressResponseProfile;
  attachment_style: AttachmentStyle;
  relational_defaults: RelationalDefaults;
  identity_axioms: IdentityAxioms;
}

export interface BodyVitals {
  pulse: number;
  bloodPressure: {
    systolic: number;
    diastolic: number;
  };
  temperature: number;
  respirationRate: number;
  oxygenSaturation: number;
}

export interface BodyPhysiology {
  metabolicRate: number;
  energyLevel: number;
  hydration: number;
  nutrition: number;
  sleepQuality: number;
  stressLevel: number;
}

export interface BodyAppearance {
  height: number;
  weight: number;
  build: string;
  hairColor: string;
  eyeColor: string;
  skinTone: string;
  distinctiveFeatures: string[];
}

export interface BodyDNA {
  strandA: any;
  strandB: any;
  expression: any;
}

export interface Body {
  vitals: BodyVitals;
  physiology: BodyPhysiology;
  appearance: BodyAppearance;
  dna: BodyDNA;
}

export interface CurrentCognition {
  attention: string;
  workingMemory: string[];
  executiveFunction: string;
  processingSpeed: number;
  mentalEnergy: number;
}

export interface CurrentEmotion {
  primary: string;
  intensity: number;
  valence: number;
  arousal: number;
  triggers: string[];
}

export interface Runtime {
  tickRate: number;
  lastTick: number;
  performance: {
    cpu: number;
    memory: number;
  };
  debug: boolean;
}

export interface BirthChart {
  timestamp: string;
  location: {
    latitude: number;
    longitude: number;
  };
  planetaryPositions: Record<string, any>;
  aspects: any[];
  houses: any[];
}

export interface Genome {
  strandA: any;
  strandB: any;
  chromosomes: Record<string, any>;
  expressionRules: Record<string, any>;
}

export interface FullPhenotype {
  physicalTraits: Record<string, any>;
  behavioralTendencies: Record<string, any>;
  cognitivePatterns: Record<string, any>;
  emotionalProfile: Record<string, any>;
}

export interface Identity {
  name: string;
  coreBeliefs: string[];
  values: string[];
  goals: string[];
  fears: string[];
  selfConcept: string;
  personalNarrative: string;
}

export interface BioSysConfig {
  metabolic_baselines: {
    atp: number;
    glucose: number;
    oxygen: number;
  };
  endocrine_baselines: {
    testosterone: number;
    estrogen: number;
    progesterone: number;
    oxytocin: number;
    vasopressin: number;
    dopamine: number;
    serotonin: number;
    cortisol: number;
    adrenaline: number;
    melatonin: number;
  };
  drive_sensitivities: {
    hunger: number;
    thirst: number;
    fatigue: number;
    somnolence: number;
    libido: number;
  };
  processing_rates: {
    atp_consumption_rate: number;
    glucose_atp_conversion: number;
    oxygen_efficiency: number;
    waste_production_rate: number;
  };
}

export interface PsycheSysConfig {
  urge_processing: {
    biological_to_psychological_weight: number;
    emotional_amplification: number;
    cognitive_filter_strength: number;
  };
  urge_sensitivities: {
    survival_urgency: number;
    social_urgency: number;
    achievement_urgency: number;
    exploration_urgency: number;
    reproduction_urgency: number;
  };
  emotional_processing: {
    affect_intensity: number;
    emotional_decay_rate: number;
    mood_stability: number;
  };
  memory_integration: {
    experience_weight: number;
    trauma_amplification: number;
    positive_bias: number;
  };
}

export interface ChaosSysConfig {
  chaos_bounds: {
    survival_chaos: { min: number; max: number };
    social_chaos: { min: number; max: number };
    achievement_chaos: { min: number; max: number };
    exploration_chaos: { min: number; max: number };
    reproduction_chaos: { min: number; max: number };
  };
  randomness_profile: {
    entropy_level: number;
    predictability: number;
    creativity_factor: number;
    stability_factor: number;
  };
  chaos_responses: {
    stress_amplification: number;
    opportunity_seeking: number;
    risk_tolerance: number;
    adaptation_rate: number;
  };
}

export interface WillSysConfig {
  willpower_profile: {
    baseline_threshold: number;
    threshold_range: { min: number; max: number };
    decay_rate: number;
    recovery_rate: number;
    fatigue_sensitivity: number;
  };
  interrupt_processing: {
    urgency_weight: number;
    chaos_weight: number;
    context_modulation: number;
    interrupt_cooldown: number;
  };
  agency_patterns: {
    autonomy_drive: number;
    compliance_tendency: number;
    initiative_probability: number;
    persistence_factor: number;
  };
  cognitive_access: {
    attention_threshold: number;
    working_memory_capacity: number;
    processing_speed: number;
    cognitive_flexibility: number;
  };
}

export interface CoreSystems {
  biosys: BioSysConfig;
  psychesys: PsycheSysConfig;
  chaossys: ChaosSysConfig;
  willsys: WillSysConfig;
}

export interface ArchitecturalLaws {
  authority_laws: {
    biological_authority: boolean;
    sensory_authority: boolean;
    digital_authority: boolean;
    cognitive_authority: boolean;
  };
  deterministic_principles: {
    state_identicality: boolean;
    fixed_timestep: number;
    no_drift: boolean;
    update_order: string[];
  };
  fail_closed_principles: {
    dependency_failure: boolean;
    fail_dead_conditions: string[];
    safe_failure_modes: string[];
  };
  information_flow: {
    unidirectional_chains: string[];
    no_manual_injection: boolean;
    flow_sequences: string[];
  };
}

export interface DeterministicArchitecture {
  tick_configuration: {
    tick_rate_ms: number;
    monotonic_clock: boolean;
    state_recalculation: boolean;
    no_events: boolean;
  };
  mathematical_constants: {
    atp_decay_rate: number;
    glucose_decay_rate: number;
    oxygen_decay_rate: number;
    atp_production_rate: number;
    hormone_recovery_rate: number;
    hormone_sensitivity: number;
    endocrine_metabolic_cost: number;
  };
  hard_constraints: {
    atp_minimum: number;
    maximum_values: Record<string, number>;
    minimum_values: Record<string, number>;
    coupling_weights: Record<string, number>;
  };
  state_machine_rules: {
    deterministic_transitions: boolean;
    state_validation: boolean;
    rollback_capability: boolean;
    state_hashing: boolean;
  };
}

export interface ImmutableState {
  snapshot_principles: {
    single_invocation: boolean;
    read_only: boolean;
    deep_freeze: boolean;
    immutable_hash: boolean;
  };
  data_integrity: {
    validation_checksums: boolean;
    corruption_detection: boolean;
    atomic_operations: boolean;
    transaction_isolation: boolean;
  };
  temporal_integrity: {
    timestamp_authority: boolean;
    monotonic_timestamps: boolean;
    time_drift_prevention: boolean;
    deterministic_timing: boolean;
  };
}

export interface SystemDependencies {
  dependency_graph: {
    biosys_dependencies: string[];
    psychesys_dependencies: string[];
    chaossys_dependencies: string[];
    willsys_dependencies: string[];
  };
  failure_propagation: {
    cascade_prevention: boolean;
    isolation_boundaries: string[];
    graceful_degradation: boolean;
    recovery_procedures: string[];
  };
  resource_management: {
    memory_limits: Record<string, number>;
    cpu_allocation: Record<string, number>;
    resource_sharing: Record<string, boolean>;
    priority_levels: Record<string, number>;
  };
}

export interface MemorySystems {
  episodic_memory: {
    trace_recording: boolean;
    write_only: boolean;
    immutable_events: boolean;
    observation_only: boolean;
    memory_decay_rate: number;
    consolidation_strength: number;
  };
  memory_types: {
    episodic: {
      capacity: number;
      detail_level: number;
      emotional_weight: number;
      temporal_precision: number;
    };
    semantic: {
      concept_network: boolean;
      abstraction_levels: number;
      relationship_strength: number;
      learning_rate: number;
    };
    procedural: {
      skill_acquisition: number;
      automation_level: number;
      error_correction: number;
      practice_effect: number;
    };
  };
  memory_processes: {
    encoding: {
      attention_requirement: number;
      emotional_amplification: number;
      repetition_effect: number;
      context_binding: number;
    };
    storage: {
      consolidation_time: number;
      interference_susceptibility: number;
      forgetting_curve: number;
      retrieval_cues: number;
    };
    retrieval: {
      recall_speed: number;
      recognition_confidence: number;
      reconstruction_accuracy: number;
      false_memory_rate: number;
    };
  };
}

export interface AttentionSystem {
  attention_traits: {
    capacity: number;
    focus: number;
    distractibility: number;
    multitasking: number;
    mind_wandering: number;
    restoration: number;
  };
  attention_state: {
    current_focus?: string;
    focus_level: number;
    cognitive_load: number;
    fatigue: number;
    arousal: number;
    flow: number;
  };
  attention_resources: {
    available: number;
    allocated: string[];
    reserved: number;
    efficiency: number;
  };
  attention_modes: {
    focused: {
      width: string;
      depth: string;
      duration: number;
      efficiency: number;
    };
    diffuse: {
      width: string;
      depth: string;
      duration: number;
      efficiency: number;
    };
    divided: {
      width: string;
      depth: string;
      duration: number;
      efficiency: number;
    };
    monitoring: {
      width: string;
      depth: string;
      duration: number;
      efficiency: number;
    };
  };
  distraction_factors: {
    internal: {
      emotions: number;
      thoughts: number;
      needs: number;
      memories: number;
    };
    external: {
      noise: number;
      visual: number;
      social: number;
      notifications: number;
    };
  };
}

export interface Consciousness {
  self_awareness: {
    meta_cognition: number;
    self_monitoring: number;
    identity_continuity: number;
    agency_recognition: number;
    perspective_taking: number;
  };
  intrinsic_worth: {
    self_value: number;
    worth_stability: number;
    external_validation_need: number;
    self_compassion: number;
    growth_mindset: number;
  };
  fear_system: {
    fear_of_loss: number;
    uncertainty_tolerance: number;
    threat_detection: number;
    anxiety_baseline: number;
    coping_mechanisms: number;
  };
  qualia_system: {
    sensory_richness: number;
    emotional_depth: number;
    temporal_flow: number;
    meaning_making: number;
    aesthetic_appreciation: number;
  };
  internal_monologue: {
    verbal_thought: number;
    visual_imagery: number;
    inner_speech: number;
    self_talk: number;
    narrative_coherence: number;
  };
}

export interface LearningAdaptation {
  neural_plasticity: {
    synaptic_plasticity: number;
    structural_plasticity: number;
    functional_plasticity: number;
    critical_periods: string[];
    plasticity_decline: number;
  };
  learning_processes: {
    acquisition_rate: number;
    retention_rate: number;
    transfer_ability: number;
    generalization: number;
    specialization: number;
  };
  adaptive_traits: {
    logic_weight: number;
    efficiency_weight: number;
    emotion_weight: number;
    creativity_weight: number;
    social_weight: number;
  };
  experience_integration: {
    learning_moments: number;
    insight_generation: number;
    pattern_recognition: number;
    error_correction: number;
    wisdom_accumulation: number;
  };
}

export interface SocialCognition {
  theory_of_mind: {
    mental_state_inference: number;
    intention_recognition: number;
    belief_desire_reasoning: number;
    perspective_taking: number;
    false_belief_understanding: number;
  };
  social_perception: {
    emotion_recognition: number;
    social_cue_interpretation: number;
    trust_assessment: number;
    social_hierarchy: number;
    group_dynamics: number;
  };
  social_interaction: {
    communication_style: string;
    conflict_resolution: number;
    cooperation_tendency: number;
    empathy_level: number;
    social_anxiety: number;
  };
  relationship_patterns: {
    attachment_style: string;
    intimacy_needs: number;
    autonomy_balance: number;
    jealousy_tendency: number;
    commitment_style: string;
  };
}

export interface CreativeSystems {
  creative_thinking: {
    divergent_thinking: number;
    convergent_thinking: number;
    originality: number;
    flexibility: number;
    elaboration: number;
  };
  problem_solving: {
    analytical_solving: number;
    intuitive_solving: number;
    creative_solving: number;
    systematic_approach: number;
    insight_generation: number;
  };
  innovation: {
    novelty_seeking: number;
    risk_tolerance: number;
    experimentation: number;
    paradigm_shift: number;
    implementation_skill: number;
  };
  aesthetic_creativity: {
    artistic_expression: number;
    aesthetic_sensitivity: number;
    pattern_beauty: number;
    symbolic_thinking: number;
    narrative_creativity: number;
  };
}

export interface DecisionMaking {
  decision_weights: {
    logic_weight: number;
    efficiency_weight: number;
    emotion_weight: number;
    creativity_weight: number;
    social_weight: number;
  };
  decision_processes: {
    rational_analysis: number;
    intuitive_judgment: number;
    emotional_guidance: number;
    social_consideration: number;
    ethical_reasoning: number;
  };
  choice_architecture: {
    option_generation: number;
    consequence_analysis: number;
    probability_assessment: number;
    value_calculation: number;
    commitment_level: number;
  };
  decision_context: {
    time_pressure: number;
    cognitive_load: number;
    emotional_state: number;
    social_context: number;
    risk_environment: number;
  };
}

export interface Human {
  agent_id: string;
  schema_version: string;
  created_at: string;
  status: 'alive' | 'dead' | 'dormant';
  core_identity: CoreIdentity;
  birth_chart: BirthChart;
  genome: Genome;
  phenotype: FullPhenotype;
  identity: Identity;
  temperament_matrix: TemperamentMatrix;
  neurocognitive_profile: NeurocognitiveProfile;
  personality_traits: PersonalityTraits;
  drive_weights: DriveWeights;
  hormonal_baseline_bias: HormonalBaselineBias;
  stress_response_profile: StressResponseProfile;
  attachment_style: AttachmentStyle;
  relational_defaults: RelationalDefaults;
  identity_axioms: IdentityAxioms;
  core_systems: CoreSystems;
  architectural_laws: ArchitecturalLaws;
  deterministic_architecture: DeterministicArchitecture;
  immutable_state: ImmutableState;
  system_dependencies: SystemDependencies;
  memory_systems: MemorySystems;
  attention_system: AttentionSystem;
  consciousness: Consciousness;
  learning_adaptation: LearningAdaptation;
  social_cognition: SocialCognition;
  creative_systems: CreativeSystems;
  decision_making: DecisionMaking;
  
  // EXTREME BRAIN DETAIL - 6-LAYER ARCHITECTURE
  extreme_brain_detail: ExtremeBrainDetail;
  
  // Real-time state
  cognition: CurrentCognition;
  emotion: CurrentEmotion;
  body: Body;
  runtime: Runtime;
  metadata: {
    schema_version: string;
    created_at: string;
    last_updated: string;
    source_project: string;
    replication_notes?: string;
  };
}

export class HumanReplication {
  constructor(schemaData: any);
  
  // Core getters
  getCoreIdentity(): CoreIdentity;
  getBirthChart(): BirthChart;
  getGenome(): Genome;
  getPhenotype(): FullPhenotype;
  getIdentity(): Identity;
  
  // Personality getters
  getTemperament(): TemperamentMatrix;
  getNeurocognitiveProfile(): NeurocognitiveProfile;
  getPersonalityTraits(domain?: string): any;
  getDriveWeights(): DriveWeights;
  getHormonalBaselineBias(): HormonalBaselineBias;
  getStressResponseProfile(): StressResponseProfile;
  getAttachmentStyle(): AttachmentStyle;
  getRelationalDefaults(): RelationalDefaults;
  getIdentityAxioms(): IdentityAxioms;
  
  // Core systems getters
  getCoreSystems(): CoreSystems;
  getBioSysConfig(): BioSysConfig;
  getPsycheSysConfig(): PsycheSysConfig;
  getChaosSysConfig(): ChaosSysConfig;
  getWillSysConfig(): WillSysConfig;
  
  // Architectural laws getters
  getArchitecturalLaws(): ArchitecturalLaws;
  getDeterministicArchitecture(): DeterministicArchitecture;
  getImmutableState(): ImmutableState;
  getSystemDependencies(): SystemDependencies;
  
  // Deeper cognitive systems getters
  getMemorySystems(): MemorySystems;
  getAttentionSystem(): AttentionSystem;
  getConsciousness(): Consciousness;
  getLearningAdaptation(): LearningAdaptation;
  getSocialCognition(): SocialCognition;
  getCreativeSystems(): CreativeSystems;
  getDecisionMaking(): DecisionMaking;
  
  // State getters
  getCurrentCognition(): CurrentCognition;
  getCurrentEmotion(): CurrentEmotion;
  getBody(): Body;
  getRuntime(): Runtime;
  
  // Utility methods
  exportForReplication(): any;
  validate(): boolean;
  getInfo(): any;
}

export const HumanTemplates: {
  gem_d: any;
  gem_k: any;
};

export default HumanReplication;
