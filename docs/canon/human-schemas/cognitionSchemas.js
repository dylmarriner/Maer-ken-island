import { z } from 'zod';

export const LegacyMemorySystemsSchema = z.object({
  // Episodic memory configuration
  episodic_memory: z.object({
    trace_recording: z.boolean(),     // Neural trace recording enabled
    trace_decay_rate: z.number(),     // How quickly traces fade
    consolidation_threshold: z.number(), // When traces become permanent
    retrieval_strength: z.number()     // How easily traces can be accessed
  }),
  
  // Semantic memory configuration
  semantic_memory: z.object({
    concept_formation: z.boolean(),    // Can form abstract concepts
    association_strength: z.number(), // How strongly concepts connect
    knowledge_integration: z.number(), // How well knowledge integrates
    forgetting_curve: z.number()       // How quickly semantic memory fades
  }),
  
  // Working memory configuration
  working_memory: z.object({
    capacity: z.number(),             // Number of items held
    duration: z.number(),              // How long items persist
    rehearsal_required: z.boolean(),   // Needs active maintenance
    interference_susceptibility: z.number() // How easily disrupted
  })
});


export const MemorySystemsSchema = z.object({
  // Episodic memory configuration
  episodic_memory: z.object({
    trace_recording: z.boolean(),     // Neural trace recording enabled
    write_only: z.boolean(),          // Write-only, append-only
    immutable_events: z.boolean(),     // Events are immutable once recorded
    observation_only: z.boolean(),      // MUST NOT influence behavior
    memory_decay_rate: z.number(),     // How memories decay over time
    consolidation_strength: z.number() // Memory consolidation strength
  }),
  
  // Memory types
  memory_types: z.object({
    episodic: z.object({
      capacity: z.number(),           // Number of episodic memories
      detail_level: z.number(),       // Level of detail recorded
      emotional_weight: z.number(),   // How emotions affect memory
      temporal_precision: z.number()  // Time accuracy of memories
    }),
    semantic: z.object({
      concept_network: z.boolean(),    // Concept network structure
      abstraction_levels: z.number(), // Levels of abstraction
      relationship_strength: z.number(), // Relationship between concepts
      learning_rate: z.number()        // How fast semantic knowledge is acquired
    }),
    procedural: z.object({
      skill_acquisition: z.number(),   // Skill learning rate
      automation_level: z.number(),   // How automated procedures become
      error_correction: z.number(),    // Error correction mechanisms
      practice_effect: z.number()      // Practice improves performance
    })
  }),
  
  // Memory processes
  memory_processes: z.object({
    encoding: z.object({
      attention_requirement: z.number(), // Attention needed for encoding
      emotional_amplification: z.number(), // Emotions amplify encoding
      repetition_effect: z.number(),     // Repetition strengthens memory
      context_binding: z.number()        // Context binding strength
    }),
    storage: z.object({
      consolidation_time: z.number(),    // Time to consolidate memories
      interference_susceptibility: z.number(), // How susceptible to interference
      forgetting_curve: z.number(),      // Rate of forgetting
      retrieval_cues: z.number()         // Strength of retrieval cues
    }),
    retrieval: z.object({
      recall_speed: z.number(),          // Speed of memory recall
      recognition_confidence: z.number(), // Confidence in recognition
      reconstruction_accuracy: z.number(), // Accuracy of memory reconstruction
      false_memory_rate: z.number()       // Rate of false memories
    })
  })
});


export const AttentionSystemSchema = z.object({
  // Attention traits
  attention_traits: z.object({
    capacity: z.number(),              // Attention span capacity
    focus: z.number(),                 // Ability to concentrate
    distractibility: z.number(),        // Susceptibility to distractions
    multitasking: z.number(),          // Multitasking ability
    mind_wandering: z.number(),         // Mind wandering tendency
    restoration: z.number()            // Recovery from fatigue
  }),
  
  // Attention state
  attention_state: z.object({
    current_focus: z.string().optional(), // Current object of attention
    focus_level: z.number(),            // Current focus intensity
    cognitive_load: z.number(),          // Mental load
    fatigue: z.number(),                // Attention fatigue
    arousal: z.number(),               // Alertness level
    flow: z.number()                  // Flow state
  }),
  
  // Attention resources
  attention_resources: z.object({
    available: z.number(),             // Available attention units
    allocated: z.array(z.string()),     // Allocated to tasks
    reserved: z.number(),               // Reserved for monitoring
    efficiency: z.number()             // Resource efficiency
  }),
  
  // Attention modes
  attention_modes: z.object({
    focused: z.object({
      width: z.string(),              // Narrow/Wide
      depth: z.string(),              // Deep/Shallow
      duration: z.number(),            // Duration in minutes
      efficiency: z.number()           // Efficiency level
    }),
    diffuse: z.object({
      width: z.string(),
      depth: z.string(),
      duration: z.number(),
      efficiency: z.number()
    }),
    divided: z.object({
      width: z.string(),
      depth: z.string(),
      duration: z.number(),
      efficiency: z.number()
    }),
    monitoring: z.object({
      width: z.string(),
      depth: z.string(),
      duration: z.number(),
      efficiency: z.number()
    })
  }),
  
  // Distraction factors
  distraction_factors: z.object({
    internal: z.object({
      emotions: z.number(),
      thoughts: z.number(),
      needs: z.number(),
      memories: z.number()
    }),
    external: z.object({
      noise: z.number(),
      visual: z.number(),
      social: z.number(),
      notifications: z.number()
    })
  })
});


export const ConsciousnessSchema = z.object({
  // Self-awareness
  self_awareness: z.object({
    meta_cognition: z.number(),         // Thinking about thinking
    self_monitoring: z.number(),        // Monitoring own thoughts
    identity_continuity: z.number(),    // Sense of continuous self
    agency_recognition: z.number(),     // Recognition of own agency
    perspective_taking: z.number()     // Ability to take different perspectives
  }),
  
  // Intrinsic worth
  intrinsic_worth: z.object({
    self_value: z.number(),            // Inherent self-value
    worth_stability: z.number(),       // Stability of self-worth
    external_validation_need: z.number(), // Need for external validation
    self_compassion: z.number(),        // Self-compassion level
    growth_mindset: z.number()          // Growth vs fixed mindset
  }),
  
  // Fear and anxiety
  fear_system: z.object({
    fear_of_loss: z.number(),          // Fear of losing what matters
    uncertainty_tolerance: z.number(),  // Tolerance for uncertainty
    threat_detection: z.number(),       // Threat detection sensitivity
    anxiety_baseline: z.number(),      // Baseline anxiety level
    coping_mechanisms: z.number()       // Coping mechanism effectiveness
  }),
  
  // Qualia and subjective experience
  qualia_system: z.object({
    sensory richness: z.number(),       // Richness of sensory experience
    emotional_depth: z.number(),       // Depth of emotional experience
    temporal_flow: z.number(),         // Experience of time flow
    meaning_making: z.number(),        // Meaning-making capacity
    aesthetic_appreciation: z.number()  // Aesthetic appreciation
  }),
  
  // Internal monologue
  internal_monologue: z.object({
    verbal_thought: z.number(),         // Verbal thinking tendency
    visual_imagery: z.number(),         // Visual imagery strength
    inner_speech: z.number(),           // Inner speech activity
    self_talk: z.number(),              // Self-talk patterns
    narrative_coherence: z.number()     // Coherence of internal narrative
  })
});


export const LearningAdaptationSchema = z.object({
  // Plasticity mechanisms
  neural_plasticity: z.object({
    synaptic_plasticity: z.number(),    // Synaptic connection changes
    structural_plasticity: z.number(),  // Brain structure changes
    functional_plasticity: z.number(),  // Functional reorganization
    critical_periods: z.array(z.string()), // Sensitive learning periods
    plasticity_decline: z.number()      // Age-related plasticity decline
  }),
  
  // Learning processes
  learning_processes: z.object({
    acquisition_rate: z.number(),       // Speed of new learning
    retention_rate: z.number(),         // How well learning is retained
    transfer_ability: z.number(),       // Transfer to new situations
    generalization: z.number(),         // Generalization ability
    specialization: z.number()          // Specialization tendency
  }),
  
  // Adaptive traits
  adaptive_traits: z.object({
    logic_weight: z.number(),          // Learned logic emphasis
    efficiency_weight: z.number(),     // Learned efficiency emphasis
    emotion_weight: z.number(),        // Learned emotion emphasis
    creativity_weight: z.number(),     // Learned creativity emphasis
    social_weight: z.number()          // Learned social emphasis
  }),
  
  // Experience integration
  experience_integration: z.object({
    learning_moments: z.number(),       // Recognition of learning moments
    insight_generation: z.number(),     // Insight generation ability
    pattern_recognition: z.number(),     // Pattern recognition skill
    error_correction: z.number(),        // Error correction from experience
    wisdom_accumulation: z.number()     // Wisdom from accumulated experience
  })
});


export const SocialCognitionSchema = z.object({
  // Theory of mind
  theory_of_mind: z.object({
    mental_state_inference: z.number(), // Ability to infer others' mental states
    intention_recognition: z.number(),   // Recognition of others' intentions
    belief_desire_reasoning: z.number(), // Understanding beliefs/desires/reasoning
    perspective_taking: z.number(),     // Taking others' perspectives
    false_belief_understanding: z.number() // Understanding false beliefs
  }),
  
  // Social perception
  social_perception: z.object({
    emotion_recognition: z.number(),    // Recognizing others' emotions
    social_cue_interpretation: z.number(), // Interpreting social cues
    trust_assessment: z.number(),       // Assessing trustworthiness
    social_hierarchy: z.number(),       // Understanding social hierarchies
    group_dynamics: z.number()         // Understanding group dynamics
  }),
  
  // Social interaction
  social_interaction: z.object({
    communication_style: z.string(),    // Communication approach
    conflict_resolution: z.number(),     // Conflict resolution ability
    cooperation_tendency: z.number(),   // Cooperation vs competition
    empathy_level: z.number(),         // Empathy toward others
    social_anxiety: z.number()          // Social anxiety level
  }),
  
  // Relationship patterns
  relationship_patterns: z.object({
    attachment_style: z.string(),        // Attachment pattern
    intimacy_needs: z.number(),         // Need for intimacy
    autonomy_balance: z.number(),        // Balance of autonomy/connection
    jealousy_tendency: z.number(),      // Jealousy proneness
    commitment_style: z.string()        // Commitment approach
  })
});


export const CreativeSystemsSchema = z.object({
  // Creative thinking
  creative_thinking: z.object({
    divergent_thinking: z.number(),      // Generating multiple ideas
    convergent_thinking: z.number(),      // Combining ideas effectively
    originality: z.number(),             // Originality of ideas
    flexibility: z.number(),             // Cognitive flexibility
    elaboration: z.number()             // Idea elaboration skill
  }),
  
  // Problem solving
  problem_solving: z.object({
    analytical_solving: z.number(),      // Analytical problem solving
    intuitive_solving: z.number(),      // Intuitive problem solving
    creative_solving: z.number(),       // Creative problem solving
    systematic_approach: z.number(),     // Systematic approach
    insight_generation: z.number()       // Insight generation ability
  }),
  
  // Innovation
  innovation: z.object({
    novelty_seeking: z.number(),         // Seeking novelty
    risk_tolerance: z.number(),          // Tolerance for risk
    experimentation: z.number(),         // Experimentation tendency
    paradigm_shift: z.number(),         // Paradigm shifting ability
    implementation_skill: z.number()     // Implementation skill
  }),
  
  // Aesthetic creativity
  aesthetic_creativity: z.object({
    artistic_expression: z.number(),     // Artistic expression ability
    aesthetic_sensitivity: z.number(),   // Sensitivity to aesthetics
    pattern_beauty: z.number(),         // Recognizing pattern beauty
    symbolic_thinking: z.number(),       // Symbolic thinking ability
    narrative_creativity: z.number()    // Narrative creativity
  })
});


export const DecisionMakingSchema = z.object({
  // Decision weights
  decision_weights: z.object({
    logic_weight: z.number(),           // Logic emphasis in decisions
    efficiency_weight: z.number(),     // Efficiency emphasis
    emotion_weight: z.number(),        // Emotion emphasis
    creativity_weight: z.number(),     // Creativity emphasis
    social_weight: z.number()          // Social considerations emphasis
  }),
  
  // Decision processes
  decision_processes: z.object({
    rational_analysis: z.number(),      // Rational analysis capability
    intuitive_judgment: z.number(),      // Intuitive judgment ability
    emotional_guidance: z.number(),     // Emotional guidance in decisions
    social_consideration: z.number(),   // Social consideration in decisions
    ethical_reasoning: z.number()       // Ethical reasoning capability
  }),
  
  // Choice architecture
  choice_architecture: z.object({
    option_generation: z.number(),      // Generating options
    consequence_analysis: z.number(),   // Analyzing consequences
    probability_assessment: z.number(), // Assessing probabilities
    value_calculation: z.number(),      // Calculating values
    commitment_level: z.number()        // Commitment to decisions
  }),
  
  // Decision context
  decision_context: z.object({
    time_pressure: z.number(),          // Effect of time pressure
    cognitive_load: z.number(),         // Effect of cognitive load
    emotional_state: z.number(),        // Effect of emotional state
    social_context: z.number(),         // Effect of social context
    risk_environment: z.number()        // Effect of risk environment
  })
});
