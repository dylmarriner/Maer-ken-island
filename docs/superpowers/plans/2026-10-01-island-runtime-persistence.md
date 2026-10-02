# Island Runtime & Persistence Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Compose the retained regional systems into one deterministic `IslandWorldState` with cadence-aware stepping, complete snapshots and replay verification.

**Architecture:** Introduce a regional world type rather than forcing `world_integration::WorldState` to pretend its planet fields are regional. Keep canonical orbit/clock/hash/ledger concepts, compose Phase 1-3 regional states, and persist every deterministic input/state required for exact continuation.

**Tech Stack:** Rust 2021, serde_json snapshots with blake3 digest, existing `HashChain`, `RngRegistry`, audit/ledger types.

**Spec:** `docs/superpowers/specs/2026-10-01-maer-ken-island-regional-world-design.md`

## Global Constraints

- Snapshot payload includes profile/scenario version, canon digest, seed, sim clock, domain, boundaries and every retained subsystem state.
- Snapshot restore under incompatible profile/canon returns an explicit error.
- Scheduler cadence is deterministic and based only on accumulated simulated time.
- Large time advances execute deterministic accumulated substeps; they may aggregate slow systems but may not silently skip human/interaction transitions.
- Final state hash excludes non-deterministic bridges/UI handles/wall-clock timestamps.
- Every human — founders, newborns and foundry-created people — has their own folder in the run's human store (upstream `HumanStorage` layout: profile, traits, cognition, social, development, reproduction, episodic/semantic/procedural memories, state, relationships, events; sensitive files encrypted). The folder is created when the human is created.
- The snapshot is the authority for deterministic restore; human folders are the per-person record and are derived from the same state. Disk success or failure never changes simulation state.

## Review Focus

- Non-divisible `dt_seconds` across subsystem cadences must preserve deterministic remainder accumulators.
- Snapshot tampering/truncation must be rejected before deserialization is trusted.
- Loading then stepping must match uninterrupted stepping exactly.
- Replay action ordering must be stable when multiple actions share a tick.
- Optional live computer bridge must remain excluded from replay/state hash.

---

### Task 1: `IslandWorldState` composition

**Files:**
- Create: `crates/mk_engine/src/regional/world.rs`
- Modify: `crates/mk_engine/src/regional/mod.rs`
- Test: `crates/mk_engine/tests/island_world_bootstrap.rs`

**Interfaces:**
- Produces: `IslandWorldState::new(canon: Arc<CanonLocked>, scenario: IslandScenario) -> Result<Self, IslandWorldError>`.
- State owns `tick`, `sim_time_seconds`, canon/derived, seed/rng/hash chain, `IslandDomain`, `RegionalBoundaryState`, `RegionalPhysicalState` (including `ZonalBackgroundState`), `RegionalEcologyState`, `PropertySystem`, `EstateLayout`, `LocalVegetationPatch`, `HumanSystem`, `ResourceEconomyState`, audit/chronicle and scheduler state.
- Produces: `IslandWorldState::state_hash() -> Result<[u8;32], IslandWorldError>`.

- [ ] **Step 1:** Write a bootstrap test asserting the acceptance criteria for island/domain, physical state, ecology, property and founders from a single constructor.
- [ ] **Step 2:** Run `cargo test -p mk_engine --test island_world_bootstrap`; expect FAIL.
- [ ] **Step 3:** Implement the constructor in the spec bootstrap order and hash deterministic serialized state with transient bridges excluded.
- [ ] **Step 4:** Re-run test; expect PASS.
- [ ] **Step 5:** Commit `feat(engine): compose regional island world`.

### Task 2: Cadence-aware deterministic scheduler

**Files:**
- Create: `crates/mk_engine/src/regional/scheduler.rs`
- Modify: `crates/mk_engine/src/regional/world.rs`
- Test: `crates/mk_engine/tests/island_scheduler.rs`

**Interfaces:**
- Consumes: `mk_island::scenario::IslandCadenceProfile`. Produces: `IslandScheduler`, `SubsystemCadence`, `SchedulerAccumulators`.
- Produces: `IslandWorldState::step(dt_seconds: u64) -> Result<(), IslandWorldError>`.
- The scheduler uses the validated scenario cadence values: humans/interactions `60 s`; weather/ocean (and zonal background) `3,600 s`; hydrology/ecology/resources/local vegetation `21,600 s`; tectonics/geology `86,400 s` by default. `IslandWorldState` persists the resolved `IslandCadenceProfile`.

- [ ] **Step 1:** Write tests comparing one 86,400-second step against 1,440 × 60-second calls and requiring identical scheduler counters/final state hash for systems whose equations are cadence-invariant.
- [ ] **Step 2:** Write tests for deterministic remainder handling with irregular `dt_seconds = 3,701` and repeated execution.
- [ ] **Step 3:** Run `cargo test -p mk_engine --test island_scheduler`; expect FAIL.
- [ ] **Step 4:** Implement accumulator-driven substep dispatch in fixed subsystem order; reuse Phase-2 physical and Phase-3 human/ecology/resource step functions.
- [ ] **Step 5:** Re-run scheduler tests; expect PASS.
- [ ] **Step 6:** Commit `feat(engine): schedule island subsystems`.

### Task 3: Complete island snapshot save/load

**Files:**
- Create: `crates/mk_engine/src/io/island_snapshot.rs`
- Modify: `crates/mk_engine/src/io/mod.rs`
- Test: `crates/mk_engine/tests/island_snapshot.rs`

**Interfaces:**
- Produces: `save_island_snapshot(state: &IslandWorldState, path: &Path) -> Result<(), IslandSnapshotError>`.
- Produces: `load_island_snapshot(path: &Path) -> Result<IslandWorldState, IslandSnapshotError>`.
- Error variants include digest mismatch, incompatible profile version, canon mismatch/invalid canon, serialization and filesystem errors.

- [ ] **Step 1:** Write tests for round-trip equality of `state_hash` (including estate layout, patch trees and zonal background), tamper rejection, truncation rejection, incompatible fixture profile version and invalid canon.
- [ ] **Step 2:** Run `cargo test -p mk_engine --test island_snapshot`; expect FAIL.
- [ ] **Step 3:** Implement atomic digest-prefixed JSON save/load, re-derive canon-derived values on load, and validate scenario/profile compatibility before returning state.
- [ ] **Step 4:** Re-run tests; expect PASS.
- [ ] **Step 5:** Commit `feat(io): persist complete island world`.

### Task 3b: Per-human folders

**Files:**
- Create: `crates/mk_engine/src/regional/human_store.rs`
- Modify: `crates/mk_engine/src/regional/world.rs`
- Test: `crates/mk_engine/tests/island_human_store.rs`

**Interfaces:**
- Each run owns a store directory, `<save_root>/<run_id>/humans/`, where `run_id` derives from scenario + seed + a creation counter, so a fresh run never reuses an earlier run's folders.
- `IslandWorldState::enable_human_store(root: &Path) -> Result<(), HumanStoreError>` creates folders for every current human; afterwards every creation path (founders, `deliver_due_births`, the foundry command) creates the new human's folder at creation time.
- Sync cadence: event files (`events/`, memories, relationships) are appended when the event happens; full state files are rewritten on the human-store cadence (default every 3,600 simulated seconds, configurable in `IslandCadenceProfile`), at every snapshot save, and when a human dies — not every 60 s human tick.
- Storage errors are counted and surfaced in the audit trail and CLI output, never discarded.

- [ ] **Step 1:** Write tests: founders get folders on enable; a birth during a stepped run creates the child's folder in the same tick with a `born` event and parents' `reproduced` events; a foundry-created human gets a folder; folders after a snapshot save match the snapshot's human state; a fresh run with the same seed under the same root uses a new `run_id` directory; state hash is identical with the store enabled, disabled or failing.
- [ ] **Step 2:** Run `cargo test -p mk_engine --test island_human_store`; expect FAIL.
- [ ] **Step 3:** Implement on top of upstream `HumanStorage` (no second folder format), adding the run directory, cadence and error reporting.
- [ ] **Step 4:** Re-run; expect PASS. Record per-sync cost in the Phase-5 benchmark.
- [ ] **Step 5:** Commit `feat(island): keep a folder for every human`.

### Task 4: Deterministic action replay

**Files:**
- Create: `crates/mk_engine/src/regional/replay.rs`
- Test: `crates/mk_engine/tests/island_replay.rs`

**Interfaces:**
- Produces: `IslandReplayAction { tick, sequence, actor_id, action }`, `IslandReplayLog`.
- Produces: `replay_island(canon: Arc<CanonLocked>, scenario: IslandScenario, log: &IslandReplayLog) -> Result<IslandWorldState, IslandReplayError>`.

- [ ] **Step 1:** Write a log containing movement/gather/craft/build-style retained actions at repeated ticks and assert stable `(tick,sequence)` order.
- [ ] **Step 2:** Run uninterrupted execution and replay from the same seed; assert identical final `state_hash`.
- [ ] **Step 3:** Save halfway, load, continue replay, and assert the same final hash.
- [ ] **Step 4:** Run `cargo test -p mk_engine --test island_replay`; expect FAIL before implementation, PASS after.
- [ ] **Step 5:** Commit `feat(engine): replay deterministic island actions`.

### Task 5: Phase-4 acceptance and headless runner

**Files:**
- Create: `apps/island/Cargo.toml`
- Create: `apps/island/src/main.rs`
- Modify: `Cargo.toml`
- Test: `crates/mk_engine/tests/island_runtime_acceptance.rs`

**Interfaces:**
- CLI: `island run --scenario <path> --steps <n> --dt <seconds> [--save <path>] [--humans-dir <path>]`, `island replay --scenario <path> --log <path> [--save <path>]`, and `island inspect --snapshot <path>`; no UI dependency yet. `--humans-dir` enables per-human folders (default: `<save dir>/humans` when `--save` is given).

- [ ] **Step 1:** Add acceptance test running a fixed scenario for 100 ticks, snapshotting, loading, continuing 100 ticks and matching an uninterrupted 200-tick hash. Put it in the slow tier if it exceeds ~30 s debug.
- [ ] **Step 2:** Implement the headless CLI around `IslandWorldState`; default with no command prints concise world summary and exits successfully.
- [ ] **Step 3:** Run the fixed-seed CLI twice and compare printed final hash; require exact equality.
- [ ] **Step 4:** Run fmt and all Phase-4 tests; update `UPSTREAM.md`.
- [ ] **Step 5:** Commit `feat(app): add deterministic island runner`.
- [ ] **Step 6:** Add `island_preview world --snapshot <path> --out <dir>` reusing the Phase 1–3 renderers on a loaded `IslandWorldState`, so any saved run can be inspected headlessly.
