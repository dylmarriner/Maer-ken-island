import { z } from 'zod';

export const BodyVitalsSchema = z.object({
  pulse: z.number(),
  bloodPressure: z.object({
    systolic: z.number(),
    diastolic: z.number()
  }),
  spO2: z.number(),
  temperature: z.number(),
  glucose: z.number(),
  energy: z.number(),
  fatigue: z.number(),
  arousal: z.number(),
  tension: z.number()
});


export const BodyPhysiologySchema = z.object({
  hydration: z.number(),
  wastePressure: z.object({
    bladder: z.number(),
    bowel: z.number()
  }),
  hygiene: z.number(),
  hormones: z.object({
    cortisol: z.number(),
    oxytocin: z.number(),
    dopamine: z.number(),
    melatonin: z.number(),
    testosterone: z.number(),
    estrogen: z.number()
  }),
  metabolism: z.object({
    glucose: z.number(),
    atp: z.number(),
    calorieIntake: z.number(),
    calorieBurn: z.number()
  })
});


export const BodyAppearanceSchema = z.object({
  height: z.number(),
  weight: z.number(),
  build: z.string(),
  hairColor: z.string(),
  eyeColor: z.string()
});


export const DnaSchema = z.object({
  helix: z.any().nullable(),
  generation: z.number(),
  traitsEncoded: z.boolean()
});


export const BodySchema = z.object({
  vitals: BodyVitalsSchema,
  physiology: BodyPhysiologySchema,
  appearance: BodyAppearanceSchema,
  dna: DnaSchema
});
