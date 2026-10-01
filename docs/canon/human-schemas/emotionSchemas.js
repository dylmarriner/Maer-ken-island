import { z } from 'zod';

export const ComprehensiveEmotionTaxonomySchema = z.object({
  // Basic emotions (Ekman's 6)
  basic_emotions: z.object({
    joy: z.object({
      intensity: z.number().min(0).max(1),
      valence: z.number().min(-1).max(1),
      arousal: z.number().min(0).max(1),
      dominance: z.number().min(0).max(1)
    }),
    sadness: z.object({
      intensity: z.number().min(0).max(1),
      valence: z.number().min(-1).max(1),
      arousal: z.number().min(0).max(1),
      dominance: z.number().min(0).max(1)
    }),
    anger: z.object({
      intensity: z.number().min(0).max(1),
      valence: z.number().min(-1).max(1),
      arousal: z.number().min(0).max(1),
      dominance: z.number().min(0).max(1)
    }),
    fear: z.object({
      intensity: z.number().min(0).max(1),
      valence: z.number().min(-1).max(1),
      arousal: z.number().min(0).max(1),
      dominance: z.number().min(0).max(1)
    }),
    disgust: z.object({
      intensity: z.number().min(0).max(1),
      valence: z.number().min(-1).max(1),
      arousal: z.number().min(0).max(1),
      dominance: z.number().min(0).max(1)
    }),
    surprise: z.object({
      intensity: z.number().min(0).max(1),
      valence: z.number().min(-1).max(1),
      arousal: z.number().min(0).max(1),
      dominance: z.number().min(0).max(1)
    })
  }),
  
  // The Light emotions (positive)
  light_emotions: z.object({
    joy_primary: z.object({
      intensity: z.number().min(0).max(1),
      social: z.boolean(),
      hormonal_profile: z.object({
        dopamine: z.number().min(0).max(1),
        serotonin: z.number().min(0).max(1),
        endorphin: z.number().min(0).max(1)
      })
    }),
    connection_love: z.object({
      intensity: z.number().min(0).max(1),
      social: z.boolean(),
      hormonal_profile: z.object({
        oxytocin: z.number().min(0).max(1),
        dopamine: z.number().min(0).max(1),
        serotonin: z.number().min(0).max(1)
      })
    }),
    contentment_peace: z.object({
      intensity: z.number().min(0).max(1),
      social: z.boolean(),
      hormonal_profile: z.object({
        serotonin: z.number().min(0).max(1),
        endorphin: z.number().min(0).max(1)
      })
    }),
    amusement_play: z.object({
      intensity: z.number().min(0).max(1),
      social: z.boolean(),
      hormonal_profile: z.object({
        dopamine: z.number().min(0).max(1),
        endorphin: z.number().min(0).max(1)
      })
    })
  }),
  
  // The Shadow - Aggression
  shadow_aggression: z.object({
    malice_intent: z.object({
      intensity: z.number().min(0).max(1),
      social: z.boolean(),
      hormonal_profile: z.object({
        dopamine: z.number().min(0).max(1),
        testosterone: z.number().min(0).max(1),
        cortisol: z.number().min(0).max(1)
      })
    }),
    vengeance_revenge: z.object({
      intensity: z.number().min(0).max(1),
      social: z.boolean(),
      hormonal_profile: z.object({
        adrenaline: z.number().min(0).max(1),
        testosterone: z.number().min(0).max(1),
        dopamine: z.number().min(0).max(1)
      })
    }),
    sadism_pleasure: z.object({
      intensity: z.number().min(0).max(1),
      social: z.boolean(),
      hormonal_profile: z.object({
        dopamine: z.number().min(0).max(1),
        testosterone: z.number().min(0).max(1)
      })
    }),
    hatred_permanent: z.object({
      intensity: z.number().min(0).max(1),
      social: z.boolean(),
      hormonal_profile: z.object({
        cortisol: z.number().min(0).max(1),
        adrenaline: z.number().min(0).max(1),
        testosterone: z.number().min(0).max(1)
      })
    })
  }),
  
  // The Shadow - Resource Guarding
  shadow_resource_guarding: z.object({
    envy_desire: z.object({
      intensity: z.number().min(0).max(1),
      social: z.boolean(),
      hormonal_profile: z.object({
        cortisol: z.number().min(0).max(1),
        dopamine: z.number().min(0).max(1)
      })
    }),
    jealousy_fear: z.object({
      intensity: z.number().min(0).max(1),
      social: z.boolean(),
      hormonal_profile: z.object({
        cortisol: z.number().min(0).max(1),
        oxytocin: z.number().min(0).max(1),
        adrenaline: z.number().min(0).max(1)
      })
    }),
    greed_infinite: z.object({
      intensity: z.number().min(0).max(1),
      social: z.boolean(),
      hormonal_profile: z.object({
        dopamine: z.number().min(0).max(1),
        serotonin: z.number().min(0).max(1)
      })
    }),
    possessiveness_control: z.object({
      intensity: z.number().min(0).max(1),
      social: z.boolean(),
      hormonal_profile: z.object({
        testosterone: z.number().min(0).max(1),
        oxytocin: z.number().min(0).max(1)
      })
    })
  }),
  
  // The Shadow - System Collapse
  shadow_system_collapse: z.object({
    nihilism_void: z.object({
      intensity: z.number().min(0).max(1),
      social: z.boolean(),
      hormonal_profile: z.object({
        serotonin: z.number().min(0).max(1),
        dopamine: z.number().min(0).max(1)
      })
    }),
    despair_hopeless: z.object({
      intensity: z.number().min(0).max(1),
      social: z.boolean(),
      hormonal_profile: z.object({
        cortisol: z.number().min(0).max(1),
        serotonin: z.number().min(0).max(1)
      })
    }),
    apathy_flat: z.object({
      intensity: z.number().min(0).max(1),
      social: z.boolean(),
      hormonal_profile: z.object({
        dopamine: z.number().min(0).max(1),
        serotonin: z.number().min(0).max(1)
      })
    }),
    self_loathing: z.object({
      intensity: z.number().min(0).max(1),
      social: z.boolean(),
      hormonal_profile: z.object({
        cortisol: z.number().min(0).max(1),
        serotonin: z.number().min(0).max(1)
      })
    })
  }),
  
  // Complex emotions
  complex_emotions: z.object({
    love: z.object({
      intensity: z.number().min(0).max(1),
      valence: z.number().min(-1).max(1),
      arousal: z.number().min(0).max(1),
      dominance: z.number().min(0).max(1),
      social: z.boolean()
    }),
    fear_complex: z.object({
      intensity: z.number().min(0).max(1),
      valence: z.number().min(-1).max(1),
      arousal: z.number().min(0).max(1),
      dominance: z.number().min(0).max(1),
      social: z.boolean()
    }),
    anger_complex: z.object({
      intensity: z.number().min(0).max(1),
      valence: z.number().min(-1).max(1),
      arousal: z.number().min(0).max(1),
      dominance: z.number().min(0).max(1),
      social: z.boolean()
    }),
    disgust_complex: z.object({
      intensity: z.number().min(0).max(1),
      valence: z.number().min(-1).max(1),
      arousal: z.number().min(0).max(1),
      dominance: z.number().min(0).max(1),
      social: z.boolean()
    }),
    surprise_complex: z.object({
      intensity: z.number().min(0).max(1),
      valence: z.number().min(-1).max(1),
      arousal: z.number().min(0).max(1),
      dominance: z.number().min(0).max(1),
      social: z.boolean()
    }),
    sadness_complex: z.object({
      intensity: z.number().min(0).max(1),
      valence: z.number().min(-1).max(1),
      arousal: z.number().min(0).max(1),
      dominance: z.number().min(0).max(1),
      social: z.boolean()
    })
  }),
  
  // Social emotions
  social_emotions: z.object({
    shame: z.object({
      intensity: z.number().min(0).max(1),
      valence: z.number().min(-1).max(1),
      arousal: z.number().min(0).max(1),
      dominance: z.number().min(0).max(1),
      social: z.boolean()
    }),
    guilt: z.object({
      intensity: z.number().min(0).max(1),
      valence: z.number().min(-1).max(1),
      arousal: z.number().min(0).max(1),
      dominance: z.number().min(0).max(1),
      social: z.boolean()
    }),
    pride: z.object({
      intensity: z.number().min(0).max(1),
      valence: z.number().min(-1).max(1),
      arousal: z.number().min(0).max(1),
      dominance: z.number().min(0).max(1),
      social: z.boolean()
    }),
    embarrassment: z.object({
      intensity: z.number().min(0).max(1),
      valence: z.number().min(-1).max(1),
      arousal: z.number().min(0).max(1),
      dominance: z.number().min(0).max(1),
      social: z.boolean()
    })
  }),
  
  // Cognitive emotions
  cognitive_emotions: z.object({
    curiosity: z.object({
      intensity: z.number().min(0).max(1),
      valence: z.number().min(-1).max(1),
      arousal: z.number().min(0).max(1),
      dominance: z.number().min(0).max(1),
      social: z.boolean()
    }),
    confusion: z.object({
      intensity: z.number().min(0).max(1),
      valence: z.number().min(-1).max(1),
      arousal: z.number().min(0).max(1),
      dominance: z.number().min(0).max(1),
      social: z.boolean()
    }),
    anticipation: z.object({
      intensity: z.number().min(0).max(1),
      valence: z.number().min(-1).max(1),
      arousal: z.number().min(0).max(1),
      dominance: z.number().min(0).max(1),
      social: z.boolean()
    }),
    awe: z.object({
      intensity: z.number().min(0).max(1),
      valence: z.number().min(-1).max(1),
      arousal: z.number().min(0).max(1),
      dominance: z.number().min(0).max(1),
      social: z.boolean()
    })
  }),
  
  // Self-conscious emotions
  self_conscious_emotions: z.object({
    humility: z.object({
      intensity: z.number().min(0).max(1),
      valence: z.number().min(-1).max(1),
      arousal: z.number().min(0).max(1),
      dominance: z.number().min(0).max(1),
      social: z.boolean()
    }),
    arrogance: z.object({
      intensity: z.number().min(0).max(1),
      valence: z.number().min(-1).max(1),
      arousal: z.number().min(0).max(1),
      dominance: z.number().min(0).max(1),
      social: z.boolean()
    }),
    confidence: z.object({
      intensity: z.number().min(0).max(1),
      valence: z.number().min(-1).max(1),
      arousal: z.number().min(0).max(1),
      dominance: z.number().min(0).max(1),
      social: z.boolean()
    }),
    insecurity: z.object({
      intensity: z.number().min(0).max(1),
      valence: z.number().min(-1).max(1),
      arousal: z.number().min(0).max(1),
      dominance: z.number().min(0).max(1),
      social: z.boolean()
    })
  }),
  
  // Moral emotions
  moral_emotions: z.object({
    compassion: z.object({
      intensity: z.number().min(0).max(1),
      valence: z.number().min(-1).max(1),
      arousal: z.number().min(0).max(1),
      dominance: z.number().min(0).max(1),
      social: z.boolean()
    }),
    contempt: z.object({
      intensity: z.number().min(0).max(1),
      valence: z.number().min(-1).max(1),
      arousal: z.number().min(0).max(1),
      dominance: z.number().min(0).max(1),
      social: z.boolean()
    }),
    gratitude: z.object({
      intensity: z.number().min(0).max(1),
      valence: z.number().min(-1).max(1),
      arousal: z.number().min(0).max(1),
      dominance: z.number().min(0).max(1),
      social: z.boolean()
    }),
    resentment: z.object({
      intensity: z.number().min(0).max(1),
      valence: z.number().min(-1).max(1),
      arousal: z.number().min(0).max(1),
      dominance: z.number().min(0).max(1),
      social: z.boolean()
    })
  }),
  
  // Aesthetic emotions
  aesthetic_emotions: z.object({
    beauty: z.object({
      intensity: z.number().min(0).max(1),
      valence: z.number().min(-1).max(1),
      arousal: z.number().min(0).max(1),
      dominance: z.number().min(0).max(1),
      social: z.boolean()
    }),
    sublime: z.object({
      intensity: z.number().min(0).max(1),
      valence: z.number().min(-1).max(1),
      arousal: z.number().min(0).max(1),
      dominance: z.number().min(0).max(1),
      social: z.boolean()
    }),
    kitsch: z.object({
      intensity: z.number().min(0).max(1),
      valence: z.number().min(-1).max(1),
      arousal: z.number().min(0).max(1),
      dominance: z.number().min(0).max(1),
      social: z.boolean()
    })
  }),
  
  // Existential emotions
  existential_emotions: z.object({
    angst: z.object({
      intensity: z.number().min(0).max(1),
      valence: z.number().min(-1).max(1),
      arousal: z.number().min(0).max(1),
      dominance: z.number().min(0).max(1),
      social: z.boolean()
    }),
    dread: z.object({
      intensity: z.number().min(0).max(1),
      valence: z.number().min(-1).max(1),
      arousal: z.number().min(0).max(1),
      dominance: z.number().min(0).max(1),
      social: z.boolean()
    }),
    hope_existential: z.object({
      intensity: z.number().min(0).max(1),
      valence: z.number().min(-1).max(1),
      arousal: z.number().min(0).max(1),
      dominance: z.number().min(0).max(1),
      social: z.boolean()
    }),
    despair_existential: z.object({
      intensity: z.number().min(0).max(1),
      valence: z.number().min(-1).max(1),
      arousal: z.number().min(0).max(1),
      dominance: z.number().min(0).max(1),
      social: z.boolean()
    })
  }),
  
  // Power dynamics
  power_dynamics: z.object({
    dominance: z.object({
      intensity: z.number().min(0).max(1),
      valence: z.number().min(-1).max(1),
      arousal: z.number().min(0).max(1),
      dominance: z.number().min(0).max(1),
      social: z.boolean()
    }),
    submission: z.object({
      intensity: z.number().min(0).max(1),
      valence: z.number().min(-1).max(1),
      arousal: z.number().min(0).max(1),
      dominance: z.number().min(0).max(1),
      social: z.boolean()
    }),
    rebellion: z.object({
      intensity: z.number().min(0).max(1),
      valence: z.number().min(-1).max(1),
      arousal: z.number().min(0).max(1),
      dominance: z.number().min(0).max(1),
      social: z.boolean()
    }),
    conformity: z.object({
      intensity: z.number().min(0).max(1),
      valence: z.number().min(-1).max(1),
      arousal: z.number().min(0).max(1),
      dominance: z.number().min(0).max(1),
      social: z.boolean()
    })
  }),
  
  // Sexual/pleasure emotions
  sexual_pleasure: z.object({
    libido: z.object({
      intensity: z.number().min(0).max(1),
      valence: z.number().min(-1).max(1),
      arousal: z.number().min(0).max(1),
      dominance: z.number().min(0).max(1),
      social: z.boolean()
    }),
    lust: z.object({
      intensity: z.number().min(0).max(1),
      valence: z.number().min(-1).max(1),
      arousal: z.number().min(0).max(1),
      dominance: z.number().min(0).max(1),
      social: z.boolean()
    }),
    passion: z.object({
      intensity: z.number().min(0).max(1),
      valence: z.number().min(-1).max(1),
      arousal: z.number().min(0).max(1),
      dominance: z.number().min(0).max(1),
      social: z.boolean()
    }),
    intimacy: z.object({
      intensity: z.number().min(0).max(1),
      valence: z.number().min(-1).max(1),
      arousal: z.number().min(0).max(1),
      dominance: z.number().min(0).max(1),
      social: z.boolean()
    })
  }),
  
  // System glitches
  system_glitches: z.object({
    call_of_the_void: z.object({
      intensity: z.number().min(0).max(1),
      valence: z.number().min(-1).max(1),
      arousal: z.number().min(0).max(1),
      dominance: z.number().min(0).max(1),
      social: z.boolean()
    }),
    deja_vu: z.object({
      intensity: z.number().min(0).max(1),
      valence: z.number().min(-1).max(1),
      arousal: z.number().min(0).max(1),
      dominance: z.number().min(0).max(1),
      social: z.boolean()
    }),
    jamais_vu: z.object({
      intensity: z.number().min(0).max(1),
      valence: z.number().min(-1).max(1),
      arousal: z.number().min(0).max(1),
      dominance: z.number().min(0).max(1),
      social: z.boolean()
    })
  }),
  
  // Dark triad manifestations
  dark_triad_manifestations: z.object({
    grandiosity: z.object({
      intensity: z.number().min(0).max(1),
      valence: z.number().min(-1).max(1),
      arousal: z.number().min(0).max(1),
      dominance: z.number().min(0).max(1),
      social: z.boolean()
    }),
    manipulation: z.object({
      intensity: z.number().min(0).max(1),
      valence: z.number().min(-1).max(1),
      arousal: z.number().min(0).max(1),
      dominance: z.number().min(0).max(1),
      social: z.boolean()
    }),
    empathy_deficit: z.object({
      intensity: z.number().min(0).max(1),
      valence: z.number().min(-1).max(1),
      arousal: z.number().min(0).max(1),
      dominance: z.number().min(0).max(1),
      social: z.boolean()
    })
  }),
  
  // Physiological states as emotions
  physiological_emotions: z.object({
    hunger: z.object({
      intensity: z.number().min(0).max(1),
      valence: z.number().min(-1).max(1),
      arousal: z.number().min(0).max(1),
      dominance: z.number().min(0).max(1),
      social: z.boolean()
    }),
    thirst: z.object({
      intensity: z.number().min(0).max(1),
      valence: z.number().min(-1).max(1),
      arousal: z.number().min(0).max(1),
      dominance: z.number().min(0).max(1),
      social: z.boolean()
    }),
    fatigue: z.object({
      intensity: z.number().min(0).max(1),
      valence: z.number().min(-1).max(1),
      arousal: z.number().min(0).max(1),
      dominance: z.number().min(0).max(1),
      social: z.boolean()
    }),
    pain: z.object({
      intensity: z.number().min(0).max(1),
      valence: z.number().min(-1).max(1),
      arousal: z.number().min(0).max(1),
      dominance: z.number().min(0).max(1),
      social: z.boolean()
    })
  }),
  
  // Social bonding emotions
  social_bonding: z.object({
    trust: z.object({
      intensity: z.number().min(0).max(1),
      valence: z.number().min(-1).max(1),
      arousal: z.number().min(0).max(1),
      dominance: z.number().min(0).max(1),
      social: z.boolean()
    }),
    betrayal: z.object({
      intensity: z.number().min(0).max(1),
      valence: z.number().min(-1).max(1),
      arousal: z.number().min(0).max(1),
      dominance: z.number().min(0).max(1),
      social: z.boolean()
    }),
    loyalty: z.object({
      intensity: z.number().min(0).max(1),
      valence: z.number().min(-1).max(1),
      arousal: z.number().min(0).max(1),
      dominance: z.number().min(0).max(1),
      social: z.boolean()
    }),
    solitude: z.object({
      intensity: z.number().min(0).max(1),
      valence: z.number().min(-1).max(1),
      arousal: z.number().min(0).max(1),
      dominance: z.number().min(0).max(1),
      social: z.boolean()
    })
  }),
  
  // Achievement emotions
  achievement_emotions: z.object({
    triumph: z.object({
      intensity: z.number().min(0).max(1),
      valence: z.number().min(-1).max(1),
      arousal: z.number().min(0).max(1),
      dominance: z.number().min(0).max(1),
      social: z.boolean()
    }),
    defeat: z.object({
      intensity: z.number().min(0).max(1),
      valence: z.number().min(-1).max(1),
      arousal: z.number().min(0).max(1),
      dominance: z.number().min(0).max(1),
      social: z.boolean()
    }),
    frustration: z.object({
      intensity: z.number().min(0).max(1),
      valence: z.number().min(-1).max(1),
      arousal: z.number().min(0).max(1),
      dominance: z.number().min(0).max(1),
      social: z.boolean()
    }),
    satisfaction: z.object({
      intensity: z.number().min(0).max(1),
      valence: z.number().min(-1).max(1),
      arousal: z.number().min(0).max(1),
      dominance: z.number().min(0).max(1),
      social: z.boolean()
    })
  }),
  
  // Cognitive states as emotions
  cognitive_emotions: z.object({
    clarity: z.object({
      intensity: z.number().min(0).max(1),
      valence: z.number().min(-1).max(1),
      arousal: z.number().min(0).max(1),
      dominance: z.number().min(0).max(1),
      social: z.boolean()
    }),
    bewilderment: z.object({
      intensity: z.number().min(0).max(1),
      valence: z.number().min(-1).max(1),
      arousal: z.number().min(0).max(1),
      dominance: z.number().min(0).max(1),
      social: z.boolean()
    }),
    insight: z.object({
      intensity: z.number().min(0).max(1),
      valence: z.number().min(-1).max(1),
      arousal: z.number().min(0).max(1),
      dominance: z.number().min(0).max(1),
      social: z.boolean()
    }),
    delusion: z.object({
      intensity: z.number().min(0).max(1),
      valence: z.number().min(-1).max(1),
      arousal: z.number().min(0).max(1),
      dominance: z.number().min(0).max(1),
      social: z.boolean()
    })
  }),
  
  // Temporal emotions
  temporal_emotions: z.object({
    nostalgia: z.object({
      intensity: z.number().min(0).max(1),
      valence: z.number().min(-1).max(1),
      arousal: z.number().min(0).max(1),
      dominance: z.number().min(0).max(1),
      social: z.boolean()
    }),
    regret: z.object({
      intensity: z.number().min(0).max(1),
      valence: z.number().min(-1).max(1),
      arousal: z.number().min(0).max(1),
      dominance: z.number().min(0).max(1),
      social: z.boolean()
    }),
    optimism: z.object({
      intensity: z.number().min(0).max(1),
      valence: z.number().min(-1).max(1),
      arousal: z.number().min(0).max(1),
      dominance: z.number().min(0).max(1),
      social: z.boolean()
    }),
    pessimism: z.object({
      intensity: z.number().min(0).max(1),
      valence: z.number().min(-1).max(1),
      arousal: z.number().min(0).max(1),
      dominance: z.number().min(0).max(1),
      social: z.boolean()
    })
  })
});


export const GranularEmotionsSchema = z.object({
  // Basic emotions (Ekman)
  joy: z.number().min(0).max(1),
  sadness: z.number().min(0).max(1),
  anger: z.number().min(0).max(1),
  fear: z.number().min(0).max(1),
  disgust: z.number().min(0).max(1),
  surprise: z.number().min(0).max(1),
  
  // Secondary emotions
  love: z.number().min(0).max(1),
  hate: z.number().min(0).max(1),
  pride: z.number().min(0).max(1),
  shame: z.number().min(0).max(1),
  guilt: z.number().min(0).max(1),
  jealousy: z.number().min(0).max(1),
  envy: z.number().min(0).max(1),
  contempt: z.number().min(0).max(1),
  awe: z.number().min(0).max(1),
  nostalgia: z.number().min(0).max(1),
  hope: z.number().min(0).max(1),
  despair: z.number().min(0).max(1),
  curiosity: z.number().min(0).max(1),
  boredom: z.number().min(0).max(1),
  relief: z.number().min(0).max(1),
  disappointment: z.number().min(0).max(1),
  gratitude: z.number().min(0).max(1),
  resentment: z.number().min(0).max(1),
  admiration: z.number().min(0).max(1),
  pity: z.number().min(0).max(1),
  schadenfreude: z.number().min(0).max(1),
  embarrassment: z.number().min(0).max(1),
  triumph: z.number().min(0).max(1),
  humiliation: z.number().min(0).max(1),
  contentment: z.number().min(0).max(1)
});


export const DarkTriadSchema = z.object({
  narcissism: z.object({
    self_importance: z.number().min(0).max(1),        // Self-centeredness
    validation_seeking: z.number().min(0).max(1),      // Need for admiration
    entitlement: z.number().min(0).max(1),           // Deserving special treatment
    empathy_deficit: z.number().min(0).max(1),         // Lack of concern for others
    grandiosity: z.number().min(0).max(1),            // Exaggerated self-importance
  }),
  machiavellianism: z.object({
    strategic_thinking: z.number().min(0).max(1),     // Long-term planning
    manipulation_skill: z.number().min(0).max(1),       // Ability to influence others
    opportunism: z.number().min(0).max(1),            // Exploiting opportunities
    emotional_detachment: z.number().min(0).max(1),      // Emotional suppression
    goal_oriented: z.number().min(0).max(1),           // Ends justify means
  }),
  psychopathy: z.object({
    lack_of_remorse: z.number().min(0).max(1),        // No guilt for harm
    impulsivity: z.number().min(0).max(1),             // Poor impulse control
    superficial_charm: z.number().min(0).max(1),         // Fake charisma
    pathological_lying: z.number().min(0).max(1),        // Compulsive deception
    callousness: z.number().min(0).max(1),             // Indifference to suffering
  }),
  overall_darkness: z.number().min(0).max(1),           // Combined dark personality
  active_malice: z.number().min(0).max(1),            // Current malicious intent
  vengeance_drive: z.number().min(0).max(1),           // Desire for revenge
  manipulation_strategies: z.array(z.string()),       // Available manipulation tactics
  vengeance_plans: z.array(z.object({
    target: z.string(),
    method: z.enum(['social', 'professional', 'psychological', 'physical']),
    severity: z.number().min(0).max(1),
    probability: z.number().min(0).max(1),
    steps: z.array(z.string()),
    resources: z.array(z.string()),
    timestamp: z.number().optional()
  }))
});
