import { z } from 'zod';

export const CurrentCognitionSchema = z.object({
  attention_focus: z.record(z.any()),
  active_thoughts: z.array(z.any()),
  goal_stack: z.array(z.any())
});


export const CurrentEmotionSchema = z.object({
  current: z.record(z.any()),
  mood: z.record(z.any()),
  decay_rates: z.record(z.number())
});


export const RuntimeSchema = z.object({
  tick_rate_hz: z.number(),
  last_tick: z.string().datetime()
});
