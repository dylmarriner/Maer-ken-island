import { z } from 'zod';

export const ReproductiveSystemsSchema = z.object({
  sexual_system: z.object({
    libido: z.number().min(0).max(1),                    // Sex drive intensity
    attraction: z.record(z.number()),                     // Attraction to specific partners
    bonding: z.record(z.number()),                         // Emotional bonding levels
    arousal: z.number().min(0).max(1),                   // Current physiological arousal
    satisfaction: z.number().min(0).max(1),              // Recent satisfaction level
    frustration: z.number().min(0).max(1),               // Sexual frustration
    attraction_factors: z.object({
      physical: z.number().min(0).max(1),               // Physical appearance preference
      personality: z.number().min(0).max(1),            // Personality compatibility
      status: z.number().min(0).max(1),                  // Social status/power
      proximity: z.number().min(0).max(1),               // Familiarity/proximity effect
      novelty: z.number().min(0).max(1)                  // Novelty/excitement factor
    }),
    hormonal_influence: z.number().min(0).max(1),          // Hormonal modulation of sexuality
    last_activity: z.number()                             // Timestamp of last sexual activity
  }),
  
  reproduction_system: z.object({
    status: z.enum(['dormant', 'fertile_window', 'conception', 'gestation', 'infertile', 'menstrual', 'postpartum']),
    fertility_level: z.number().min(0).max(1),            // Based on actual hormone levels
    conception_probability: z.number().min(0).max(1),      // Calculated from real biology
    gestation_week: z.number().min(0).max(42),            // Pregnancy progression
    pregnancy_complications: z.array(z.string()),
    fertility_cycle: z.object({
      cycle_day: z.number().min(1).max(28),              // Real menstrual cycle
      phase: z.enum(['menstrual', 'follicular', 'ovulation', 'luteal']),
      fertility_peak: z.boolean(),
      hormone_levels: z.object({
        estrogen: z.object({
          current: z.number().min(0).max(1),
          baseline: z.number().min(0).max(1),
          production: z.number().min(0).max(1),
          decay: z.number().min(0).max(1)
        }),
        progesterone: z.object({
          current: z.number().min(0).max(1),
          baseline: z.number().min(0).max(1),
          production: z.number().min(0).max(1),
          decay: z.number().min(0).max(1)
        }),
        lh: z.object({                                   // Luteinizing hormone
          current: z.number().min(0).max(1),
          baseline: z.number().min(0).max(1),
          production: z.number().min(0).max(1),
          decay: z.number().min(0).max(1)
        }),
        fsh: z.object({                                  // Follicle stimulating hormone
          current: z.number().min(0).max(1),
          baseline: z.number().min(0).max(1),
          production: z.number().min(0).max(1),
          decay: z.number().min(0).max(1)
        })
      }),
      cervical_mucus: z.enum(['dry', 'sticky', 'creamy', 'watery', 'egg_white']),
      basal_body_temp: z.number(),                        // °C, rises after ovulation
      ovulation_day: z.number().optional()               // Day 14 typically
    }),
    sperm_analysis: z.object({
      count: z.number(),                                 // million per mL (15-200 normal)
      motility: z.number().min(0).max(1),               // % progressive (40-60 normal)
      morphology: z.number().min(0).max(1),             // % normal forms (4-14 normal)
      volume: z.number(),                                // mL (1.5-5 normal)
      vitality: z.number().min(0).max(1)                // % live sperm (58+ normal)
    }).optional(),
    sexual_activities: z.array(z.object({
      activity_id: z.string(),
      participant1_id: z.string(),
      participant2_id: z.string(),
      location_id: z.string(),
      start_time: z.string().datetime(),
      end_time: z.string().datetime().optional(),
      activity_type: z.enum(['casual', 'intimate', 'reproductive_attempt']),
      mutual_consent: z.boolean(),
      satisfaction: z.array(z.number().min(0).max(1)),   // 0-1 for each participant
      biological_cost: z.array(z.object({
        atp_cost: z.number(),
        stress_impact: z.number()
      })),
      conception_attempted: z.boolean(),
      conception_result: z.enum(['none', 'successful', 'failed'])
    }))
  }),
  
  genetics_system: z.object({
    genotype: z.object({
      id: z.string(),
      paternal_genome: z.any(),                          // Full paternal genome
      maternal_genome: z.any(),                          // Full maternal genome
      creation_timestamp: z.string().datetime(),
      mutations: z.array(z.object({
        type: z.enum(['point', 'insertion', 'deletion', 'recombination']),
        chromosome: z.string(),
        position: z.number(),
        original_value: z.any(),
        mutated_value: z.any(),
        probability: z.number()
      }))
    }),
    gametes: z.array(z.object({
      id: z.string(),
      genome: z.any(),                                   // Haploid genome
      parent_id: z.string(),
      creation_timestamp: z.string().datetime(),
      meiosis_timestamp: z.string().datetime()
    })),
    conception_history: z.array(z.object({
      genotype_id: z.string(),
      father_id: z.string(),
      mother_id: z.string(),
      conception_timestamp: z.string().datetime(),
      mutations: z.array(z.any())
    })),
    birth_records: z.array(z.object({
      birth_id: z.string(),
      genotype_id: z.string(),
      father_id: z.string(),
      mother_id: z.string(),
      birth_timestamp: z.string().datetime(),
      agent_id: z.string(),
      mutations: z.array(z.any())
    })),
    genetic_markers: z.record(z.any()),                   // Genetic traits and markers
    hereditary_conditions: z.array(z.object({
      condition: z.string(),
      inheritance_pattern: z.enum(['dominant', 'recessive', 'x_linked', 'mitochondrial']),
      probability: z.number().min(0).max(1),
      severity: z.number().min(0).max(1)
    }))
  }),
  
  mate_selection: z.object({
    preferences: z.object({
      physical_traits: z.record(z.number().min(0).max(1)),
      personality_traits: z.record(z.number().min(0).max(1)),
      social_status: z.number().min(0).max(1),
      intelligence: z.number().min(0).max(1),
      age_preference: z.object({
        min: z.number(),
        max: z.number(),
        ideal: z.number()
      }),
      genetic_compatibility: z.number().min(0).max(1)
    }),
    courtship_behaviors: z.array(z.object({
      behavior: z.string(),
      effectiveness: z.number().min(0).max(1),
      context: z.enum(['social', 'private', 'public', 'digital']),
      energy_cost: z.number().min(0).max(1)
    })),
    attraction_triggers: z.array(z.object({
      trigger: z.string(),
      intensity: z.number().min(0).max(1),
      duration: z.number(),
      context_modifiers: z.array(z.string())
    })),
    relationship_history: z.array(z.object({
      partner_id: z.string(),
      relationship_type: z.enum(['casual', 'dating', 'committed', 'marriage']),
      duration: z.number(),
      satisfaction: z.number().min(0).max(1),
      termination_reason: z.string().optional()
    }))
  })
});
