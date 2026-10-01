# Human scope

The imported runtime includes deterministic models for identity, genome/phenotype, body state, needs, endocrine and immune summaries, cognition, attention, learning, memory, emotion, social cognition, reproduction, development, aging, death, lineage, dialogue, culture, language, technology and several brain-state abstractions.

The runtime is a simulation model, not validated biological or neuroscience ground truth. The upstream status document records known limitations including incomplete full-body kinematics, summary-level tactile/multimodal perception, non-neuron-level brain models and some schema/runtime fields without one-to-one engine equivalents.

For island work, preserve these boundaries:

- durable identity/configuration belongs in canonical human profile/schema data;
- current mood, focus, vitals and other tick state belong in runtime state;
- deterministic randomness must remain keyed/replayable;
- island geography and resources should feed humans through world observations rather than being hardcoded into identity;
- do not silently fabricate missing human state to make UI or gameplay look complete.
