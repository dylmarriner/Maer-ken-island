import { z } from 'zod';

export const ImmuneSystemSchema = z.object({
  innate_immunity: z.object({
    macrophages: z.number(),                           // Cell count
    neutrophils: z.number(),                           // Cell count
    nk_cells: z.number(),                               // Natural killer cells
    complement: z.number().min(0).max(1),              // Complement proteins
    inflammation: z.number().min(0).max(1)               // Systemic inflammation
  }),
  adaptive_immunity: z.object({
    t_cells: z.number(),                                // T lymphocyte count
    b_cells: z.number(),                                // B lymphocyte count
    memory_cells: z.number(),                             // Memory cell count
    antibodies: z.record(z.number()),                     // Antigen -> antibody level
    vaccination_history: z.array(z.string()),              // Previous exposures
  }),
  immune_memory: z.record(z.number()),                     // Antigen -> memory strength
  system_stress: z.number().min(0).max(1),                 // Overall immune system load
  autoimmunity_risk: z.number().min(0).max(1),               // Risk of attacking self
  immune_cells: z.array(z.object({
    id: z.string(),
    type: z.enum(['macrophage', 'neutrophil', 'nk_cell', 't_cell', 'b_cell', 'memory_cell']),
    location: z.object({
      x: z.number(),
      y: z.number(),
      z: z.number()
    }),
    activation: z.number().min(0).max(1),               // Current activation level
    specificity: z.array(z.string()),                   // Antigens recognized
    memory: z.number().min(0).max(1),                     // Immune memory strength
    age: z.number(),                                   // Cell age in hours
    effectiveness: z.number().min(0).max(1)               // Cell effectiveness
  })),
  pathogens: z.array(z.object({
    id: z.string(),
    type: z.enum(['virus', 'bacteria', 'fungus', 'parasite', 'toxin']),
    virulence: z.number().min(0).max(1),                   // How harmful it is
    replication_rate: z.number().min(0).max(1),             // How fast it spreads
    immune_evasion: z.number().min(0).max(1),               // Ability to avoid detection
    location: z.object({
      x: z.number(),
      y: z.number(),
      z: z.number()
    }),
    load: z.number(),                                   // Pathogen count in body
    discovered: z.boolean()                              // Whether immune system has detected it
  })),
  immune_responses: z.array(z.object({
    type: z.enum(['inflammation', 'fever', 'antibody', 'cell_mediated', 'complement']),
    intensity: z.number().min(0).max(1),               // Response strength
    location: z.object({
      x: z.number(),
      y: z.number(),
      z: z.number()
    }),
    target: z.string(),                                   // Pathogen ID being targeted
    effectiveness: z.number().min(0).max(1),             // How well it's working
    side_effects: z.array(z.string())                      // Collateral damage symptoms
  }))
});


export const SkinSystemSchema = z.object({
  temperature: z.number(),                               // Celsius, skin surface temp
  cleanliness: z.number().min(0).max(1),                 // Hygiene level
  healing_rate: z.number().min(0).max(1),                 // Regeneration speed
  protection: z.number().min(0).max(1),                  // Pathogen defense
  integrity: z.number().min(0).max(1),                   // Skin integrity
  infection_risk: z.number().min(0).max(1),                // Risk of infection
  healing_events: z.array(z.object({
    type: z.enum(['wound', 'burn', 'infection', 'hygiene_care', 'environmental']),
    severity: z.number().min(0).max(1),                   // Event severity
    location: z.string(),                               // Body part
    description: z.string(),                             // Event description
    timestamp: z.number(),
    healing_progress: z.number().min(0).max(1)            // Healing progress
  }))
});
