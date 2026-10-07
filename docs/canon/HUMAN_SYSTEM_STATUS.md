# Assess the current human-system status

- **Status:** Active status snapshot
- **Last reviewed:** 2026-09-05
- **Authority:** Descriptive only. Locked roadmap and canon documents override this page.

This page separates human runtime implementation, canonical schema coverage, and client presentation. Use it to identify the right governing track before changing human behavior.

## Separate the governing tracks

Human-related work spans three distinct tracks:

1. **Engine runtime**: `crates/mk_engine/src/humans/`, governed by [`../roadmap/MAERKEN_PHASE_11_HUMANS_MKIII.md`](../roadmap/MAERKEN_PHASE_11_HUMANS_MKIII.md) under the roadmap hierarchy
2. **Canonical schema mapping**: `mk_core::human` and the standalone `crates/mk_human_foundry`, governed by [`HUMAN_SCHEMA_IMPLEMENTATION_ROADMAP.md`](HUMAN_SCHEMA_IMPLEMENTATION_ROADMAP.md) and [`SCHEMA_TO_ENGINE_OWNERSHIP_MAP.md`](SCHEMA_TO_ENGINE_OWNERSHIP_MAP.md)
3. **Observation and clients**: `mk_view`, the daemon API, `mk_desktop`, and legacy Bevy applications, governed by the applicable UI authority and view-state contract

Implementation in one track does not prove completeness in another.

## Current engine runtime

The engine contains live, tick-stepped behavior for:

- body vitals, physical capacity, needs, fatigue, pain, and health
- endocrine, immune, skin, sensory, and proprioceptive summaries
- cognition, attention, learning, memory, predictive processing, and decisions
- granular emotion, temperament, drives, stress, attachment, and social cognition
- reproduction, conception, birth, lineage, development, aging, and death
- relationships, deterministic dialogue, per-human conversation history, and projected thought text
- culture, language, technology, skills, resource actions, and physical carrying constraints

These systems use deterministic state and keyed RNG where randomness is required. Their existence does not imply validated biological or neuroscience fidelity.

## Current schema coverage

[`SCHEMA_TO_ENGINE_OWNERSHIP_MAP.md`](SCHEMA_TO_ENGINE_OWNERSHIP_MAP.md) is the detailed per-family map. Its current classifications are:

| Schema family | Status |
| --- | --- |
| Identity | Implemented |
| Psychology | Implemented |
| Body | Partial |
| Runtime | Partial |
| Core systems | Implemented |
| Cognition | Implemented |
| Emotion | Implemented |
| Biology | Partial |
| Sensory | Partial |
| Reproductive | Implemented |
| Brain | Partial |

`Partial` means the schema and real runtime consumption exist, but the engine does not implement every canonical field or fidelity claim.

## Current observation surfaces

`mk_view::human_view` and `mk_view::human_detail_view` project human state without mutation authority. The detailed projection includes vitals, needs, emotion, cognition, family, thought text, and recent conversations.

The supported daemon-backed clients consume these projections:

- `mk query human ...`
- the browser dashboard served by `mk serve`
- `mk_desktop`

Legacy `mk_observatory`, `mk_ui`, and `mk_studio` surfaces remain in the repository with different compatibility roles. Do not infer feature parity from shared mode or panel names.

## Known gaps

The human track still has explicit limits:

- full kinematic pose and per-joint movement do not have a complete world-coupled driver
- tactile and multimodal perception use summary values rather than full sensor simulation
- brain modules are deterministic engine models, not validated neuron-level neuroscience
- generic runtime schema fields such as `active_thoughts` and `goal_stack` do not have one-to-one engine equivalents
- sibling dialogue has a relationship type but no automatic pairing trigger
- canonical fixtures and valid/invalid contract tests remain incomplete across schema families
- the standalone `mk_human_foundry` crate no longer exists in the repository (previously tracked here as "not a root workspace member"; it has since been removed or was never checked in)
- `formal_predictive_processing::step()` genuinely runs every tick (`lifecycle.rs:479`, real inputs). Its `selected_action` is read only inside its own module, by its TD and habit terms, and by its tests; no other subsystem consumes it, so it costs a full hierarchical-inference and actor-critic update per human per tick for an action nothing executes. `AutonomousMind`, not this subsystem, is what actually drives behaviour. (`thought.rs`'s module comment now says this correctly and can be trusted; the earlier note here that it claimed `step()` had no call site is itself out of date.) Whether to wire the output into decisions or stop running it is the owner's call, and the Phase 5 benchmark plan (Task 6, Step 3b) measures its share of human step time so that call can be made on numbers

Do not fabricate state to close these gaps. Add behavior only when a deterministic source and ownership boundary exist.

## Change human behavior safely

Before editing:

1. identify the governing track
2. check the locked roadmap hierarchy
3. locate the canonical schema owner
4. distinguish durable identity from transient tick state
5. define deterministic inputs and RNG keys
6. update projection and persistence only when the behavior requires them
7. add replay-sensitive tests for serialized changes

A field that defines who a human is or how a durable system is configured belongs in canonical persisted data. A field that describes current focus, mood, or engine bookkeeping belongs in transient runtime state.

## Verify changes

Run the workspace gates from [`../../CONTRIBUTING.md`](../../CONTRIBUTING.md). Also run separate-process replay checks whenever a human change affects IDs, collection ordering, RNG draws, persistence, snapshots, conversations, or lifecycle state.
