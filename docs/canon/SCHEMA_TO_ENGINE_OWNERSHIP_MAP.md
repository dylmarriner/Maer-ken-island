# Human Schema to Engine Ownership Map

This document maps the active human replication schema surface to the current consuming Rust modules and runtime responsibilities in this repository.

It is intended to satisfy the engine-mapping requirement from `docs/canon/HUMAN_SCHEMA_IMPLEMENTATION_ROADMAP.md` and to make it clear which schema families are:

- implemented as canonical data shapes,
- mapped into Rust runtime/profile structures,
- partially consumed,
- or still awaiting fuller runtime integration.

---

## Status Key

- **Implemented** — canonical schema exists and has an identified Rust owner/consumer
- **Partial** — canonical schema exists and some mapping/consumption exists, but not all runtime behavior is implemented
- **Deferred** — canonical schema exists but runtime behavior is intentionally deferred or not yet wired

---

## Ownership Table

| Schema family | Canon source | Primary Rust owner | Current consumer(s) | Status | Notes |
|---|---|---|---|---|---|
| identity | `docs/canon/human-schemas/identitySchemas.js` | `crates/mk_core/src/human/schema.rs` | `crates/mk_core/src/human/profile.rs`, `crates/mk_engine/src/humans/mod.rs` | Implemented | Core identity, genome, birth chart, phenotype, and identity payloads map into `HumanProfile` and `HumanBeing`. |
| psychology | `docs/canon/human-schemas/psychologySchemas.js` | `crates/mk_core/src/human/schema.rs` | `crates/mk_core/src/human/profile.rs`, `crates/mk_engine/src/humans/mod.rs` | Implemented | Temperament, drives, attachment, relational defaults, and neurocognitive profile map into runtime profile fields. |
| body | `docs/canon/human-schemas/bodySchemas.js` | `crates/mk_core/src/human/schema.rs` | `crates/mk_engine/src/humans/body.rs` (`BodySnapshot`), `physical_capacity.rs`, `proprioception.rs` | Partial | Vitals (pulse/BP/SpO2/temperature/hormones/ATP), physiology, and static appearance are real and stepped each tick from schema data. Kinematic pose (position/angle/velocity/torque per joint) is deliberately not modeled — no movement/pose system exists yet to drive it honestly (see `proprioception.rs` module docs). |
| runtime | `docs/canon/human-schemas/runtimeSchemas.js` | `crates/mk_core/src/human/schema.rs` | `HumanBeing`'s per-tick `current` fields across every snapshot type (e.g. `emotion.current`, `cognition.mode`) | Partial | The schema's generic `current` cognition/emotion snapshot shape and tick-rate fields have no literal 1:1 Rust struct, but the underlying *concept* — live per-tick state distinct from dispositional baseline — is real and pervasive (every `*Snapshot` type steps a `current`-style value each tick). The schema's specific `active_thoughts`/`goal_stack` array fields have no engine equivalent. |
| core systems | `docs/canon/human-schemas/coreSystemsSchemas.js` | `crates/mk_core/src/human/schema.rs` | `crates/mk_engine/src/humans/core_systems.rs` (`CoreSystemsSnapshot`), `needs.rs`, `neurochemistry.rs` | Implemented | The four foundational config layers (biology -> psychology -> chaos -> will -> cognition gateway) are real; `needs.rs`/`neurochemistry.rs` read `schema.core_systems.biosys`'s metabolic/endocrine baselines directly, and `chaossys` draws genuine seeded RNG (`mk_core::rng::RngRegistry`), deterministic per (human id, tick). |
| cognition | `docs/canon/human-schemas/cognitionSchemas.js` | `crates/mk_core/src/human/schema.rs` | `crates/mk_engine/src/humans/{cognition,brain_regions,mesoscale_brain,formal_predictive_processing,predictive_processing,decision,attention,memory,advanced_memory,learning,social_cognition}.rs` | Implemented | Substantially more built out than the schema alone suggests: cognitive mode, attention/working-memory traits, hierarchical predictive coding with precision weighting, deterministic expected-utility action selection, and decision weights/processes all step from real state each tick. Not a validated neuroscience model — several of these modules explicitly document themselves as simplified engine approximations. |
| emotion | `docs/canon/human-schemas/emotionSchemas.js` | `crates/mk_core/src/human/schema.rs` | `crates/mk_engine/src/humans/{emotion,comprehensive_emotion,dark_triad}.rs` | Implemented | The canon's full 30-entry granular vector (`EmotionLevels`) and ~130-entry `ComprehensiveEmotionTaxonomySchema` are both real, stepped from needs/social/creativity/attention/sensory(pain)/immune(sickness) state, not placeholder taxonomy. |
| biology | `docs/canon/human-schemas/biologySchemas.js` | `crates/mk_core/src/human/schema.rs` | `crates/mk_engine/src/humans/{immune,skin}.rs` | Partial | Innate/adaptive immunity, pathogen load, system stress, temperature, cleanliness, healing, and infection risk are real and stepped from live state (not merely represented canonically, as this row previously claimed) — but a full per-condition causal/pathology model is not yet built; see `pathology.rs`/`attractor_control.rs` for the (also simplified) psychiatric-risk side of biology. |
| sensory | `docs/canon/human-schemas/sensorySchemas.js` | `crates/mk_core/src/human/schema.rs` | `crates/mk_engine/src/humans/{sensory,proprioception}.rs` | Partial | Tactile/visual/auditory/vestibular/interoception/proprioception headline scalars are real, stepped from real fatigue/pain/joint state — but this is summary-scalar perception, not detailed multi-modal simulation. |
| reproductive | `docs/canon/human-schemas/reproductiveSchemas.js` | `crates/mk_core/src/human/schema.rs` | `crates/mk_engine/src/humans/{reproduction,lifecycle}.rs` | Implemented | Sexual system, mate selection, fertility cycle, conception, birth, and lineage-history recording (father/mother agent-id records, inherited traits/birth charts) are real end-to-end — `lifecycle::step_reproduction` actually grows the population deterministically. |
| brain | `docs/canon/human-schemas/brainSchemas.js` | `crates/mk_core/src/human/schema.rs` | `crates/mk_engine/src/humans/{brain_regions,population_dynamics,neurochemistry,predictive_processing,attractor_control,pathology,mesoscale_brain,formal_predictive_processing}.rs` | Partial | No longer "primarily a data contract" — this is now one of the most built-out families in the engine (7 functional brain regions, leaky-integrator neural population dynamics, neurotransmitter/hormone levels, two-basin attractor dynamics, per-named psychiatric-risk tracking). Still Partial rather than Implemented because every one of these modules explicitly self-documents as a simplified engine model, not a validated neuroscience claim — the canon's full `ExtremeBrainDetailSchema` fidelity is not fully realized. |

---

## Aggregator and Type Surface

### Canonical JS aggregator
- `docs/canon/HumanReplicationSchema.js`

### Canonical JS split modules
- `docs/canon/human-schemas/index.js`
- `docs/canon/human-schemas/*.js`

### Rust canonical representation
- `crates/mk_core/src/human/schema.rs`

### Rust mapping layer
- `crates/mk_core/src/human/profile.rs`

### Engine-side human runtime entry point
- `crates/mk_engine/src/humans/mod.rs`

---

## Current gaps

The following work remains before every schema family can claim complete runtime coverage:

1. add authoritative fixtures for each schema family
2. add contract tests that load valid and invalid fixtures
3. complete partial JavaScript template examples
4. implement currently unmodeled body, runtime, biology, sensory, and brain fields only when real deterministic drivers exist
5. document the boundary between canonical persisted data and transient runtime state at each persistence entry point

Core systems, cognition, emotion, and reproductive behavior have real runtime owners. Do not list those families as generally unwired; assess any missing field against the ownership table above.

---

## Canonical Persistence Boundary

The immediate boundary for schema/test work is:

- **Persisted canonical data** — configuration and durable profile state that should round-trip through the canonical schema and fixture set. This includes identity, psychology, core systems configuration, cognition architecture, emotion taxonomies, biology payloads, sensory payloads, reproductive payloads, brain detail payloads, and the stable embodiment snapshot under `body`.
- **Transient runtime state** — live tick-specific execution values that can be regenerated or replaced by engine execution without changing who the human is. This includes `runtime`, the current `cognition` snapshot, the current `emotion` snapshot, and any future engine-only working state that should not be treated as authoring-time canon.

### Practical rule for engine extraction

- If a field defines who the human is, what systems exist, or how they are configured, it belongs in canonical persisted data and should be represented in fixtures.
- If a field exists only to describe the current tick, current focus/mood, or engine bookkeeping, it belongs in transient runtime state and should be documented as runtime-owned even when a canonical snapshot shape exists for interchange.
- `body` is mixed but still fixture-worthy: durable embodiment data (appearance, DNA, baseline physiology) belongs to canonical persisted data, while its values may be mirrored into engine-owned live state during execution.
