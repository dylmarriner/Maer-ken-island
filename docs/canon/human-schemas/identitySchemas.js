import { z } from 'zod';
import {
  TemperamentMatrixSchema,
  NeurocognitiveProfileSchema,
  PersonalityTraitsSchema,
  DriveWeightsSchema,
  HormonalBaselineBiasSchema,
  StressResponseProfileSchema,
  AttachmentStyleSchema,
  RelationalDefaultsSchema,
  IdentityAxiomsSchema
} from './psychologySchemas.js';

export const CoreIdentitySchema = z.object({
  agent_id: z.string(),
  biological_sex: z.enum(['male', 'female', 'neutral']),
  birth_timestamp: z.string().datetime(),
  birthplace: z.object({
    location: z.string(),
    coordinates: z.object({
      latitude: z.number(),
      longitude: z.number()
    }),
    locality: z.enum(['urban', 'suburban', 'rural', 'semi-rural'])
  }),
  neurotype: z.object({
    adhd_subtype: z.enum(['inattentive_presentation', 'combined_presentation', 'hyperactive_impulsive']).optional(),
    adhd_profile: z.object({
      attentional_profile: z.object({
        sustained_attention: z.number().min(0).max(1).optional(),      // Ability to maintain focus
        selective_attention: z.number().min(0).max(1).optional(),      // Ability to filter distractions
        divided_attention: z.number().min(0).max(1).optional(),        // Multitasking ability
        alternating_attention: z.number().min(0).max(1).optional()     // Task switching
      }).optional(),
      hyperactivity_profile: z.object({
        motor_hyperactivity: z.number().min(0).max(1).optional(),       // Physical restlessness
        verbal_hyperactivity: z.number().min(0).max(1).optional(),      // Excessive talking
        mental_hyperactivity: z.number().min(0).max(1).optional(),      // Racing thoughts
        impulsivity_level: z.number().min(0).max(1).optional()          // Impulse control
      }).optional(),
      executive_functioning: z.object({
        working_memory: z.number().min(0).max(1).optional(),            // Holding information in mind
        planning_organizing: z.number().min(0).max(1).optional(),      // Planning and organization
        time_management: z.number().min(0).max(1).optional(),          // Time perception and management
        emotional_regulation: z.number().min(0).max(1).optional(),     // Emotional control
        task_initiation: z.number().min(0).max(1).optional(),          // Starting tasks
        task_completion: z.number().min(0).max(1).optional()           // Finishing tasks
      }).optional(),
      circadian_rhythm: z.object({
        chronotype: z.enum(['morning', 'evening', 'intermediate']).optional(),
        sleep_onset_difficulty: z.number().min(0).max(1).optional(),
        sleep_maintenance: z.number().min(0).max(1).optional(),
        daytime_somnolence: z.number().min(0).max(1).optional()
      }).optional(),
      comorbidity_patterns: z.object({
        anxiety_level: z.number().min(0).max(1).optional(),
        depression_level: z.number().min(0).max(1).optional(),
        emotional_dysregulation: z.number().min(0).max(1).optional(),
        rejection_sensitivity: z.number().min(0).max(1).optional()
      }).optional()
    }).optional(),
    autism_spectrum: z.enum(['level_1_high_functioning', 'level_2_requiring_support', 'level_3_requiring_very_substantial_support']).optional(),
    autism_profile: z.object({
      social_communication: z.object({
        social_recognition: z.number().min(0).max(1).optional(),        // Reading social cues
        social_motivation: z.number().min(0).max(1).optional(),         // Desire for social interaction
        social_anxiety: z.number().min(0).max(1).optional(),            // Social discomfort
        communication_style: z.enum(['direct', 'formal', 'literal', 'nonverbal_preferenced']).optional(),
        nonverbal_communication: z.number().min(0).max(1).optional(),   // Body language, eye contact
        pragmatic_language: z.number().min(0).max(1).optional()          // Social use of language
      }).optional(),
      sensory_processing: z.object({
        hypersensitivity: z.object({
          auditory: z.number().min(0).max(1).optional(),               // Sound sensitivity
          visual: z.number().min(0).max(1).optional(),                 // Light sensitivity
          tactile: z.number().min(0).max(1).optional(),                 // Touch sensitivity
          proprioceptive: z.number().min(0).max(1).optional(),           // Body position sense
          vestibular: z.number().min(0).max(1).optional(),              // Balance/movement
          interoceptive: z.number().min(0).max(1).optional(),           // Internal body signals
          olfactory: z.number().min(0).max(1).optional(),              // Smell
          gustatory: z.number().min(0).max(1).optional()               // Taste
        }).optional(),
        hyposensitivity: z.object({
          auditory: z.number().min(0).max(1).optional(),
          visual: z.number().min(0).max(1).optional(),
          tactile: z.number().min(0).max(1).optional(),
          proprioceptive: z.number().min(0).max(1).optional(),
          vestibular: z.number().min(0).max(1).optional(),
          interoceptive: z.number().min(0).max(1).optional(),
          olfactory: z.number().min(0).max(1).optional(),
          gustatory: z.number().min(0).max(1).optional()
        }).optional(),
        sensory_seeking: z.object({
          proprioceptive_seeking: z.number().min(0).max(1).optional(),  // Deep pressure, joint compression
          vestibular_seeking: z.number().min(0).max(1).optional(),      // Spinning, swinging
          tactile_seeking: z.number().min(0).max(1).optional(),        // Different textures
          oral_seeking: z.number().min(0).max(1).optional()            // Chewing, oral stimulation
        }).optional()
      }).optional(),
      restricted_repetitive_behaviors: z.object({
        stereotyped_movements: z.number().min(0).max(1).optional(),     // Rocking, hand-flapping
        ritualistic_behavior: z.number().min(0).max(1).optional(),       // Need for sameness
        restricted_interests: z.object({
          intensity: z.number().min(0).max(1).optional(),              // Depth of special interests
          breadth: z.number().min(0).max(1).optional(),                // Number of interests
          flexibility: z.number().min(0).max(1).optional(),            // Ability to shift interests
          knowledge_depth: z.number().min(0).max(1).optional()         // Expertise level
        }).optional(),
        sensory_regulation_needs: z.number().min(0).max(1).optional()   // Need for sensory input
      }).optional(),
      executive_functioning: z.object({
        cognitive_flexibility: z.number().min(0).max(1).optional(),      // Shifting between tasks
        planning_sequencing: z.number().min(0).max(1).optional(),       // Multi-step planning
        working_memory: z.number().min(0).max(1).optional(),            // Mental workspace
        inhibition_control: z.number().min(0).max(1).optional(),         // Impulse control
        abstract_thinking: z.number().min(0).max(1).optional()          // Conceptual reasoning
      }).optional(),
      information_processing: z.object({
        detail_focus: z.number().min(0).max(1).optional(),              // Attention to detail
        pattern_recognition: z.number().min(0).max(1).optional(),       // Pattern detection
      system_thinking: z.number().min(0).max(1).optional(),            // Understanding systems
        visual_processing: z.number().min(0).max(1).optional(),         // Visual-spatial skills
        auditory_processing: z.number().min(0).max(1).optional()        // Auditory processing
      }).optional(),
      emotional_processing: z.object({
        emotional_identification: z.number().min(0).max(1).optional(),   // Identifying emotions
        emotional_regulation: z.number().min(0).max(1).optional(),     // Managing emotions
        alexithymia_tendency: z.number().min(0).max(1).optional(),      // Difficulty identifying feelings
        emotional_intensity: z.number().min(0).max(1).optional()         // Emotional experience intensity
      }).optional()
    }).optional(),
    audhd_interaction: z.object({
      attentional_dynamics: z.object({
        hyperfocus_intensity: z.number().min(0).max(1).optional(),     // Deep concentration ability
        attentional_shifts: z.number().min(0).max(1).optional(),        // Rapid attention changes
        environmental_filtering: z.number().min(0).max(1).optional(),    // Filtering sensory input
        task_switching_difficulty: z.number().min(0).max(1).optional()  // Switching between tasks
      }).optional(),
      sensory_attention_interaction: z.object({
        sensory_overload_impact: z.number().min(0).max(1).optional(),   // How sensory issues affect attention
        stimming_for_focus: z.number().min(0).max(1).optional(),       // Using self-stimulation for focus
        environmental_adaptation_needs: z.number().min(0).max(1).optional() // Environmental modifications needed
      }).optional(),
      social_cognitive_interaction: z.object({
        social_exhaustion: z.number().min(0).max(1).optional(),         // Social fatigue
        masking_energy_cost: z.number().min(0).max(1).optional(),       // Energy cost of masking
        executive_social_conflict: z.number().min(0).max(1).optional(), // Executive function vs social demands
        rejection_sensitivity_amplification: z.number().min(0).max(1).optional() // Heightened rejection sensitivity
      }).optional(),
      emotional_regulation_complexity: z.object({
        emotional_volatility: z.number().min(0).max(1).optional(),       // Emotional swings
        emotional_burnout: z.number().min(0).max(1).optional(),          // Emotional exhaustion
        cooccuring_anxiety_depression: z.number().min(0).max(1).optional(), // Comorbid mood issues
        self_concept_impact: z.number().min(0).max(1).optional()         // How neurodivergence affects self-image
      }).optional()
    }).optional(),
    sensory_processing_sensitivity: z.boolean().optional(),
    executive_dysfunction_bias: z.string().optional(),
    schizophrenia_spectrum: z.object({
      subtype: z.enum(['paranoid', 'disorganized', 'catatonic', 'undifferentiated', 'residual', 'schizoaffective']).optional(),
      positive_symptoms: z.object({
        hallucinations: z.object({
          auditory: z.number().min(0).max(1).optional(),
          visual: z.number().min(0).max(1).optional(),
          olfactory: z.number().min(0).max(1).optional(),
          tactile: z.number().min(0).max(1).optional()
        }).optional(),
        delusions: z.object({
          persecutory: z.number().min(0).max(1).optional(),
          grandiose: z.number().min(0).max(1).optional(),
          referential: z.number().min(0).max(1).optional(),
          erotomanic: z.number().min(0).max(1).optional(),
          nihilistic: z.number().min(0).max(1).optional(),
          somatic: z.number().min(0).max(1).optional()
        }).optional(),
        disorganized_speech: z.number().min(0).max(1).optional(),
        grossly_disorganized_behavior: z.number().min(0).max(1).optional()
      }).optional(),
      negative_symptoms: z.object({
        alogia: z.number().min(0).max(1).optional(),           // Poverty of speech
        anhedonia: z.number().min(0).max(1).optional(),        // Inability to feel pleasure
        asociality: z.number().min(0).max(1).optional(),        // Lack of motivation for social interaction
        avolition: z.number().min(0).max(1).optional(),         // Lack of motivation
        flat_affect: z.number().min(0).max(1).optional()        // Reduced emotional expression
      }).optional(),
      cognitive_symptoms: z.object({
        executive_function_impairment: z.number().min(0).max(1).optional(),
        working_memory_deficits: z.number().min(0).max(1).optional(),
        attention_impairment: z.number().min(0).max(1).optional(),
        processing_speed_deficits: z.number().min(0).max(1).optional()
      }).optional(),
      disease_progression: z.object({
        onset_age: z.number().min(12).max(45).optional(),
        chronicity: z.number().min(0).max(1).optional(),        // How chronic/progressive
        episodic_vs_continuous: z.enum(['episodic', 'continuous', 'mixed']).optional(),
        treatment_response: z.object({
          antipsychotic_responsiveness: z.number().min(0).max(1).optional(),
          side_effect_sensitivity: z.number().min(0).max(1).optional(),
          therapy_engagement: z.number().min(0).max(1).optional()
        }).optional()
      }).optional()
    }).optional()
  }),
  generation: z.number()
});


export const DnaStrandSchema = z.object({
  // Consciousness kernel traits
  openness: z.number().min(0).max(1),
  extraversion: z.number().min(0).max(1),
  plasticity: z.number().min(0).max(1).optional(), // strandB learning trait
  
  // Neurological base traits
  DOPAMINE_BASE: z.number().min(0).max(1),
  SEROTONIN_BASE: z.number().min(0).max(1),
  NOREPINEPHRINE_BASE: z.number().min(0).max(1),
  CORTISOL_SENS: z.number().min(0).max(1),
  NOVELTY_SEEK: z.number().min(0).max(1),
  RUMINATION: z.number().min(0).max(1),
  EXEC_CONTROL: z.number().min(0).max(1),
  THREAT_BIAS: z.number().min(0).max(1),
  EPISODIC_GAIN: z.number().min(0).max(1),
  MEM_DECAY: z.number().min(0).max(1),
  TRAUMA_STICKY: z.number().min(0).max(1),
  ATTACHMENT: z.number().min(0).max(1),
  TRUST_GAIN: z.number().min(0).max(1),
  TRUST_DECAY: z.number().min(0).max(1),
  JEALOUSY: z.number().min(0).max(1),
  FATIGUE_SENS: z.number().min(0).max(1),
  PAIN_SENS: z.number().min(0).max(1)
});


export const SexChromosomesSchema = z.object({
  pair23: z.object({
    A: z.enum(['X', 'Y']), // Maternal
    B: z.enum(['X', 'Y'])  // Paternal
  })
});


export const GeneticExpressionSchema = z.object({
  mode: z.enum(['weighted', 'dominant', 'recessive']),
  weights: z.object({
    A: z.number().min(0).max(1),
    B: z.number().min(0).max(1)
  }),
  rules: z.record(z.enum(['max', 'min', 'blend', 'average']))
});


export const GenomeSchema = z.object({
  strandA: DnaStrandSchema,
  strandB: DnaStrandSchema,
  chromosomes: SexChromosomesSchema,
  expression: GeneticExpressionSchema
});


export const BirthChartSchema = z.object({
  sun: z.string(),
  moon: z.string(),
  ascendant: z.string(),
  element_balance: z.record(z.number()),
  modality_balance: z.record(z.number()),
  coordinates: z.object({
    latitude: z.number(),
    longitude: z.number()
  }),
  birth_timestamp: z.string().datetime()
});


export const NarrativeSelfSchema = z.array(z.object({
  timestamp: z.string().datetime(),
  statement: z.string()
}));


export const IdentitySchema = z.object({
  name: z.string(),
  narrative_self: NarrativeSelfSchema,
  core_values: z.record(z.any()),
  identity_stability: z.number().min(0).max(1),
  identity_drift_rate: z.number().min(0).max(1)
});


export const FullPhenotypeSchema = z.object({
  traits: z.record(z.any()),
  abilities: z.record(z.any()),
  tendencies: z.record(z.any()),
  physical: z.record(z.any()).optional(),
  neurochemical: z.record(z.any()).optional(),
  cognitive_biases: z.record(z.any()).optional()
});


export const HumanIdentitySchema = z.object({
  core_identity: CoreIdentitySchema,
  birth_chart: BirthChartSchema,
  genome: GenomeSchema,
  phenotype: FullPhenotypeSchema,
  identity: IdentitySchema,
  temperament_matrix: TemperamentMatrixSchema,
  neurocognitive_profile: NeurocognitiveProfileSchema,
  personality_traits: PersonalityTraitsSchema,
  drive_weights: DriveWeightsSchema,
  hormonal_baseline_bias: HormonalBaselineBiasSchema,
  stress_response_profile: StressResponseProfileSchema,
  attachment_style: AttachmentStyleSchema,
  relational_defaults: RelationalDefaultsSchema,
  identity_axioms: IdentityAxiomsSchema
});
