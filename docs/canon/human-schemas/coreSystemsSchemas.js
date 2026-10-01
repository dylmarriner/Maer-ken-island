import { z } from 'zod';

export const BioSysConfigSchema = z.object({
  // Metabolic baselines
  metabolic_baselines: z.object({
    atp: z.number().min(0).max(1),
    glucose: z.number().min(0).max(1),
    oxygen: z.number().min(0).max(1)
  }),
  
  // Endocrine baselines (10 hormones)
  endocrine_baselines: z.object({
    testosterone: z.number().min(0).max(1),
    estrogen: z.number().min(0).max(1),
    progesterone: z.number().min(0).max(1),
    oxytocin: z.number().min(0).max(1),
    vasopressin: z.number().min(0).max(1),
    dopamine: z.number().min(0).max(1),
    serotonin: z.number().min(0).max(1),
    cortisol: z.number().min(0).max(1),
    adrenaline: z.number().min(0).max(1),
    melatonin: z.number().min(0).max(1)
  }),
  
  // Drive sensitivities
  drive_sensitivities: z.object({
    hunger: z.number().min(0).max(1),
    thirst: z.number().min(0).max(1),
    fatigue: z.number().min(0).max(1),
    somnolence: z.number().min(0).max(1),
    libido: z.number().min(0).max(1)
  }),
  
  // Metabolic processing rates
  processing_rates: z.object({
    atp_consumption_rate: z.number().min(0),
    glucose_atp_conversion: z.number().min(0),
    oxygen_efficiency: z.number().min(0),
    waste_production_rate: z.number().min(0)
  })
});


export const PsycheSysConfigSchema = z.object({
  // Urge processing weights
  urge_processing: z.object({
    biological_to_psychological_weight: z.number().min(0).max(1),
    emotional_amplification: z.number().min(0).max(2),
    cognitive_filter_strength: z.number().min(0).max(1)
  }),
  
  // Urge type sensitivities
  urge_sensitivities: z.object({
    survival_urgency: z.number().min(0).max(1),
    social_urgency: z.number().min(0).max(1),
    achievement_urgency: z.number().min(0).max(1),
    exploration_urgency: z.number().min(0).max(1),
    reproduction_urgency: z.number().min(0).max(1)
  }),
  
  // Emotional processing
  emotional_processing: z.object({
    affect_intensity: z.number().min(0).max(2),
    emotional_decay_rate: z.number().min(0).max(1),
    mood_stability: z.number().min(0).max(1)
  }),
  
  // Memory integration
  memory_integration: z.object({
    experience_weight: z.number().min(0).max(1),
    trauma_amplification: z.number().min(0).max(2),
    positive_bias: z.number().min(-1).max(1)
  })
});


export const ChaosSysConfigSchema = z.object({
  // Chaos bounds per urge type
  chaos_bounds: z.object({
    survival_chaos: z.object({ min: z.number(), max: z.number() }),
    social_chaos: z.object({ min: z.number(), max: z.number() }),
    achievement_chaos: z.object({ min: z.number(), max: z.number() }),
    exploration_chaos: z.object({ min: z.number(), max: z.number() }),
    reproduction_chaos: z.object({ min: z.number(), max: z.number() })
  }),
  
  // Randomness characteristics
  randomness_profile: z.object({
    entropy_level: z.number().min(0).max(1), // Overall randomness
    predictability: z.number().min(0).max(1), // How predictable the chaos is
    creativity_factor: z.number().min(0).max(2), // Novelty generation
    stability_factor: z.number().min(0).max(1) // Resistance to chaos
  }),
  
  // Chaos response patterns
  chaos_responses: z.object({
    stress_amplification: z.number().min(0).max(2),
    opportunity_seeking: z.number().min(0).max(1),
    risk_tolerance: z.number().min(0).max(1),
    adaptation_rate: z.number().min(0).max(1)
  })
});


export const WillSysConfigSchema = z.object({
  // Willpower characteristics
  willpower_profile: z.object({
    baseline_threshold: z.number().min(0).max(1),
    threshold_range: z.object({ min: z.number().min(0), max: z.number().max(1) }),
    decay_rate: z.number().min(0).max(1),
    recovery_rate: z.number().min(0).max(1),
    fatigue_sensitivity: z.number().min(0).max(1)
  }),
  
  // Interrupt processing
  interrupt_processing: z.object({
    urgency_weight: z.number().min(0).max(1),
    chaos_weight: z.number().min(0).max(1),
    context_modulation: z.number().min(0).max(1),
    interrupt_cooldown: z.number().min(0) // milliseconds
  }),
  
  // Agency patterns
  agency_patterns: z.object({
    autonomy_drive: z.number().min(0).max(1),
    compliance_tendency: z.number().min(0).max(1),
    initiative_probability: z.number().min(0).max(1),
    persistence_factor: z.number().min(0).max(1)
  }),
  
  // Cognitive access
  cognitive_access: z.object({
    attention_threshold: z.number().min(0).max(1),
    working_memory_capacity: z.number().min(1).max(10),
    processing_speed: z.number().min(0).max(1),
    cognitive_flexibility: z.number().min(0).max(1)
  })
});


export const CoreSystemsSchema = z.object({
  biosys: BioSysConfigSchema,
  psychesys: PsycheSysConfigSchema,
  chaossys: ChaosSysConfigSchema,
  willsys: WillSysConfigSchema
});


export const ArchitecturalLawsSchema = z.object({
  // System authority laws
  authority_laws: z.object({
    biological_authority: z.boolean(), // "THE LAW of biological authority"
    sensory_authority: z.boolean(),   // "THE LAW of sensory authority"
    digital_authority: z.boolean(),    // "THE LAW of digital authority"
    cognitive_authority: z.boolean()   // "THE LAW of cognitive authority"
  }),
  
  // Deterministic principles
  deterministic_principles: z.object({
    state_identicality: z.boolean(),   // Given State T0 and Inputs, State T1 is identical
    fixed_timestep: z.number(),        // 100ms fixed timestep
    no_drift: z.boolean(),             // No wall-clock dependency
    update_order: z.array(z.string())  // CLOCK → PHYSICS → AGENTS → EVENTS
  }),
  
  // Fail-closed principles
  fail_closed_principles: z.object({
    dependency_failure: z.boolean(),  // FAILS CLOSED if dependencies missing
    fail_dead_conditions: z.array(z.string()), // ATP = 0 means total incapacitation
    safe_failure_modes: z.array(z.string()) // Systems fail safely rather than dangerously
  }),
  
  // Information flow laws
  information_flow: z.object({
    unidirectional_chains: z.array(z.string()), // Authority chains
    no_manual_injection: z.boolean(), // Must come from physics/authority
    flow_sequences: z.array(z.string()) // Specific flow sequences
  })
});


export const DeterministicArchitectureSchema = z.object({
  // Tick configuration
  tick_configuration: z.object({
    tick_rate_ms: z.number(),          // 100ms per tick
    monotonic_clock: z.boolean(),       // tickCount and simTime only increase
    state_recalculation: z.boolean(),  // tick() strictly recalculates state
    no_events: z.boolean()             // NO Events - only state recalculation
  }),
  
  // Mathematical constants
  mathematical_constants: z.object({
    // Metabolic constants
    atp_decay_rate: z.number(),        // ATP decay per second
    glucose_decay_rate: z.number(),    // Glucose decay per second
    oxygen_decay_rate: z.number(),     // Oxygen decay per second
    atp_production_rate: z.number(),   // ATP from glucose+oxygen efficiency
    
    // Endocrine constants
    hormone_recovery_rate: z.number(),  // Rate hormones return to baseline
    hormone_sensitivity: z.number(),    // Sensitivity to stimuli
    endocrine_metabolic_cost: z.number() // ATP cost per second for endocrine activity
  }),
  
  // Hard constraints
  hard_constraints: z.object({
    atp_minimum: z.number(),           // Minimum ATP for survival (0.0)
    maximum_values: z.record(z.number()), // Maximum values for each parameter
    minimum_values: z.record(z.number()), // Minimum values for each parameter
    coupling_weights: z.record(z.number()) // Drive coupling weights
  }),
  
  // State machine rules
  state_machine_rules: z.object({
    deterministic_transitions: z.boolean(), // Same inputs produce identical outputs
    state_validation: z.boolean(),     // State must be valid before transition
    rollback_capability: z.boolean(),  // Can rollback to previous state
    state_hashing: z.boolean()         // Generate deterministic hash for validation
  })
});


export const ImmutableStateSchema = z.object({
  // Snapshot principles
  snapshot_principles: z.object({
    single_invocation: z.boolean(),    // SNAPSHOT-ONLY: Single invocation
    read_only: z.boolean(),            // NO SIDE EFFECTS: Read-only data structure
    deep_freeze: z.boolean(),          // Deep freeze to ensure immutability
    immutable_hash: z.boolean()        // Generate deterministic hash
  }),
  
  // Data integrity
  data_integrity: z.object({
    validation_checksums: z.boolean(), // Validate data integrity
    corruption_detection: z.boolean(),  // Detect data corruption
    atomic_operations: z.boolean(),     // Operations are atomic
    transaction_isolation: z.boolean()  // Transactions are isolated
  }),
  
  // Temporal integrity
  temporal_integrity: z.object({
    timestamp_authority: z.boolean(),  // Single source of time truth
    monotonic_timestamps: z.boolean(), // Timestamps only increase
    time_drift_prevention: z.boolean(), // Prevent time drift
    deterministic_timing: z.boolean()   // Same timing across runs
  })
});


export const SystemDependenciesSchema = z.object({
  // Dependency graph
  dependency_graph: z.object({
    biosys_dependencies: z.array(z.string()), // What BioSys depends on
    psychesys_dependencies: z.array(z.string()), // What PsycheSys depends on
    chaossys_dependencies: z.array(z.string()), // What ChaosSys depends on
    willsys_dependencies: z.array(z.string()) // What WillSys depends on
  }),
  
  // Failure propagation
  failure_propagation: z.object({
    cascade_prevention: z.boolean(),   // Prevent cascade failures
    isolation_boundaries: z.array(z.string()), // System isolation boundaries
    graceful_degradation: z.boolean(), // Systems degrade gracefully
    recovery_procedures: z.array(z.string()) // Recovery procedures
  }),
  
  // Resource management
  resource_management: z.object({
    memory_limits: z.record(z.number()), // Memory limits per system
    cpu_allocation: z.record(z.number()), // CPU allocation per system
    resource_sharing: z.record(z.boolean()), // Resource sharing rules
    priority_levels: z.record(z.number()) // System priority levels
  })
});
