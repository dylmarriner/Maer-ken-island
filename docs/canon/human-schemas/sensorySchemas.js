import { z } from 'zod';

export const SensorySystemsSchema = z.object({
  tactile_system: z.object({
    sensors: z.array(z.object({
      id: z.string(),
      type: z.enum(['pressure', 'temperature', 'nociceptor', 'vibration']),
      location: z.object({
        x: z.number(),
        y: z.number(),
        z: z.number()
      }),
      sensitivity: z.number().min(0).max(1),
      threshold: z.number(),
      current_value: z.number(),
      last_stimulated: z.number()
    })),
    pain_qualia: z.array(z.object({
      intensity: z.number().min(0).max(1),
      type: z.enum(['sharp', 'dull', 'burning', 'aching', 'electric']),
      location: z.object({
        x: z.number(),
        y: z.number(),
        z: z.number()
      }),
      duration: z.number(),
      quality: z.enum(['unpleasant', 'excruciating', 'mild', 'tolerable']),
      emotional_impact: z.number().min(0).max(1)
    })),
    tactile_events: z.array(z.object({
      type: z.enum(['collision', 'contact', 'temperature_change', 'pressure_change']),
      intensity: z.number().min(0).max(1),
      location: z.object({
        x: z.number(),
        y: z.number(),
        z: z.number()
      }),
      object: z.string().optional(),
      damage: z.number().min(0).max(1).optional(),
      timestamp: z.number()
    })),
    body_map: z.record(z.number()), // Body part -> sensitivity mapping
    overall_pain_level: z.number().min(0).max(1),
    dominant_sensation: z.string()
  }),
  
  proprioception_system: z.object({
    body_schema: z.object({
      head: z.object({
        position: z.object({ x: z.number(), y: z.number(), z: z.number() }),
        orientation: z.object({ x: z.number(), y: z.number(), z: z.number() })
      }),
      torso: z.object({
        position: z.object({ x: z.number(), y: z.number(), z: z.number() }),
        orientation: z.object({ x: z.number(), y: z.number(), z: z.number() })
      }),
      left_arm: z.object({
        shoulder: z.object({
          joint: z.string(),
          position: z.object({ x: z.number(), y: z.number(), z: z.number() }),
          angle: z.number(),
          target_angle: z.number(),
          velocity: z.number(),
          torque: z.number(),
          flexibility: z.number().min(0).max(1),
          stress: z.number().min(0).max(1)
        }),
        elbow: z.object({
          joint: z.string(),
          position: z.object({ x: z.number(), y: z.number(), z: z.number() }),
          angle: z.number(),
          target_angle: z.number(),
          velocity: z.number(),
          torque: z.number(),
          flexibility: z.number().min(0).max(1),
          stress: z.number().min(0).max(1)
        }),
        wrist: z.object({
          joint: z.string(),
          position: z.object({ x: z.number(), y: z.number(), z: z.number() }),
          angle: z.number(),
          target_angle: z.number(),
          velocity: z.number(),
          torque: z.number(),
          flexibility: z.number().min(0).max(1),
          stress: z.number().min(0).max(1)
        }),
        hand: z.object({
          position: z.object({ x: z.number(), y: z.number(), z: z.number() })
        })
      }),
      right_arm: z.object({
        shoulder: z.object({
          joint: z.string(),
          position: z.object({ x: z.number(), y: z.number(), z: z.number() }),
          angle: z.number(),
          target_angle: z.number(),
          velocity: z.number(),
          torque: z.number(),
          flexibility: z.number().min(0).max(1),
          stress: z.number().min(0).max(1)
        }),
        elbow: z.object({
          joint: z.string(),
          position: z.object({ x: z.number(), y: z.number(), z: z.number() }),
          angle: z.number(),
          target_angle: z.number(),
          velocity: z.number(),
          torque: z.number(),
          flexibility: z.number().min(0).max(1),
          stress: z.number().min(0).max(1)
        }),
        wrist: z.object({
          joint: z.string(),
          position: z.object({ x: z.number(), y: z.number(), z: z.number() }),
          angle: z.number(),
          target_angle: z.number(),
          velocity: z.number(),
          torque: z.number(),
          flexibility: z.number().min(0).max(1),
          stress: z.number().min(0).max(1)
        }),
        hand: z.object({
          position: z.object({ x: z.number(), y: z.number(), z: z.number() })
        })
      }),
      left_leg: z.object({
        hip: z.object({
          joint: z.string(),
          position: z.object({ x: z.number(), y: z.number(), z: z.number() }),
          angle: z.number(),
          target_angle: z.number(),
          velocity: z.number(),
          torque: z.number(),
          flexibility: z.number().min(0).max(1),
          stress: z.number().min(0).max(1)
        }),
        knee: z.object({
          joint: z.string(),
          position: z.object({ x: z.number(), y: z.number(), z: z.number() }),
          angle: z.number(),
          target_angle: z.number(),
          velocity: z.number(),
          torque: z.number(),
          flexibility: z.number().min(0).max(1),
          stress: z.number().min(0).max(1)
        }),
        ankle: z.object({
          joint: z.string(),
          position: z.object({ x: z.number(), y: z.number(), z: z.number() }),
          angle: z.number(),
          target_angle: z.number(),
          velocity: z.number(),
          torque: z.number(),
          flexibility: z.number().min(0).max(1),
          stress: z.number().min(0).max(1)
        }),
        foot: z.object({
          position: z.object({ x: z.number(), y: z.number(), z: z.number() })
        })
      }),
      right_leg: z.object({
        hip: z.object({
          joint: z.string(),
          position: z.object({ x: z.number(), y: z.number(), z: z.number() }),
          angle: z.number(),
          target_angle: z.number(),
          velocity: z.number(),
          torque: z.number(),
          flexibility: z.number().min(0).max(1),
          stress: z.number().min(0).max(1)
        }),
        knee: z.object({
          joint: z.string(),
          position: z.object({ x: z.number(), y: z.number(), z: z.number() }),
          angle: z.number(),
          target_angle: z.number(),
          velocity: z.number(),
          torque: z.number(),
          flexibility: z.number().min(0).max(1),
          stress: z.number().min(0).max(1)
        }),
        ankle: z.object({
          joint: z.string(),
          position: z.object({ x: z.number(), y: z.number(), z: z.number() }),
          angle: z.number(),
          target_angle: z.number(),
          velocity: z.number(),
          torque: z.number(),
          flexibility: z.number().min(0).max(1),
          stress: z.number().min(0).max(1)
        }),
        foot: z.object({
          position: z.object({ x: z.number(), y: z.number(), z: z.number() })
        })
      })
    }),
    muscle_spindles: z.array(z.object({
      id: z.string(),
      muscle: z.string(),
      length: z.number().min(0).max(1),
      tension: z.number().min(0).max(1),
      stretch: z.number().min(0).max(1),
      activation: z.number().min(0).max(1),
      fatigue: z.number().min(0).max(1)
    })),
    spatial_awareness: z.object({
      body_center: z.object({ x: z.number(), y: z.number(), z: z.number() }),
      balance: z.number().min(0).max(1),
      posture: z.enum(['standing', 'sitting', 'lying', 'walking', 'running']),
      coordination: z.number().min(0).max(1),
      phantom_limb_risk: z.number().min(0).max(1)
    })
  }),
  
  visual_system: z.object({
    vision_range: z.number(),
    field_of_view: z.number(),
    ray_count: z.number(),
    visual_objects: z.array(z.object({
      id: z.string(),
      type: z.enum(['agent', 'object', 'zone', 'vehicle', 'furniture', 'appliance', 'computer', 'tool']),
      name: z.string(),
      position: z.object({ x: z.number(), y: z.number() }),
      distance: z.number(),
      angle: z.number(),
      properties: z.any()
    })),
    visual_acuity: z.number().min(0).max(1),
    color_perception: z.number().min(0).max(1),
    depth_perception: z.number().min(0).max(1),
    motion_detection: z.number().min(0).max(1)
  }),
  
  auditory_system: z.object({
    hearing_range: z.object({
      min_frequency: z.number(),
      max_frequency: z.number()
    }),
    sound_sources: z.array(z.object({
      id: z.string(),
      type: z.enum(['agent', 'object', 'environment']),
      position: z.object({ x: z.number(), y: z.number() }),
      volume: z.number().min(0).max(1),
      frequency: z.number()
    })),
    hearing_sensitivity: z.number().min(0).max(1),
    sound_localization: z.number().min(0).max(1),
    speech_recognition: z.number().min(0).max(1)
  }),
  
  vestibular_system: z.object({
    balance_sensitivity: z.number().min(0).max(1),
    motion_sickness: z.number().min(0).max(1),
    spatial_orientation: z.number().min(0).max(1),
    gravity_detection: z.number().min(0).max(1),
    angular_acceleration: z.number(),
    linear_acceleration: z.number()
  }),
  
  interoception_system: z.object({
    internal_sensations: z.array(z.object({
      type: z.enum(['hunger', 'thirst', 'fatigue', 'pain', 'temperature', 'heartbeat', 'breathing']),
      intensity: z.number().min(0).max(1),
      location: z.string(),
      urgency: z.number().min(0).max(1)
    })),
    body_awareness: z.number().min(0).max(1),
    internal_state_monitoring: z.number().min(0).max(1),
    homeostatic_regulation: z.number().min(0).max(1)
  }),
  
  sensory_integration: z.object({
    multimodal_processing: z.number().min(0).max(1),
    sensory_filtering: z.number().min(0).max(1),
    attention_modulation: z.number().min(0).max(1),
    sensory_memory: z.number().min(0).max(1),
    adaptation_level: z.number().min(0).max(1)
  })
});
