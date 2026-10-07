# Island Runtime & Persistence Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Compose the retained regional systems into one deterministic `IslandWorldState` with cadence-aware stepping, complete snapshots, per-human folders, external-command replay, and the dashboard (with the Human Creator) running against the live island.

**Architecture:** Introduce a regional world type rather than forcing `world_integration::WorldState` to pretend its planet fields are regional. Keep canonical orbit/clock/hash/ledger concepts, compose Phase 1–3 regional states, and persist every deterministic input/state required for exact continuation. External inputs (operator commands, created humans, control) are the only things replay logs; human and organism decisions are internal and reproduced by deterministic stepping.

**Tech Stack:** Rust 2021, serde_json snapshots with blake3 digest, existing `HashChain`, `RngRegistry`, audit/ledger types; warp 0.4 + tokio (from Phase 0b).

**Spec:** `docs/superpowers/specs/2026-10-01-maer-ken-island-regional-world-design.md`

**Depends on:** Phases 0b and 3.

## Global Constraints

- Snapshot payload includes profile/scenario version, canon digest, seed, sim clock, domain, boundaries and every retained subsystem state.
- Snapshot restore under incompatible profile/canon returns an explicit error.
- Scheduler cadence is deterministic and based only on accumulated simulated time.
- Large time advances execute deterministic accumulated substeps; they may aggregate slow systems but may not silently skip human/interaction transitions.
- **State hash** is blake3 over a canonical serialization: serialize to `serde_json::Value` (sorted maps), then to bytes. Upstream state contains `HashMap`s (e.g. `AdvancedMemorySnapshot`, `advanced_memory.rs:70-85`) whose direct serialization order is not stable across processes. The hash excludes non-deterministic bridges, UI handles and wall-clock timestamps.
- Every human — founders, newborns and created people — has their own folder in the run's human store (upstream `HumanStorage` layout: profile, traits, cognition, social, development, reproduction, episodic/semantic/procedural memories, state, relationships, events; sensitive files encrypted). The folder is created when the human is created.
- The snapshot is the authority for deterministic restore; human folders are the per-person record derived from the same state. Disk success or failure never changes simulation state (Phase 0 Task 4b makes the registry behave this way).

## Review Focus

- Non-divisible `dt_seconds` across subsystem cadences must preserve deterministic remainder accumulators.
- Snapshot tampering/truncation must be rejected before deserialization is trusted.
- Loading then stepping must match uninterrupted stepping exactly.
- Replay command ordering must be stable when multiple commands share a tick.
- Optional live computer bridge must remain excluded from replay/state hash.
- Dashboard handlers never touch world state directly; reads come from a published projection and writes go through the command queue.

---

### Task 1: `IslandWorldState` composition

**Files:**
- Create: `crates/mk_engine/src/regional/world.rs`
- Modify: `crates/mk_engine/src/regional/mod.rs`
- Test: `crates/mk_engine/tests/island_world_bootstrap.rs`

**Interfaces:**
- Produces: `IslandWorldState::new(canon: Arc<CanonLocked>, scenario: IslandScenario) -> Result<Self, IslandWorldError>`.
- State owns `tick`, `sim_time_seconds`, canon/derived, scenario, seed/`RngRegistry`/`HashChain`, `IslandDomain`, `GridTopology` (regional, Medium), `FaultSystem` and recent earthquakes, `RegionalPhysicalState` (including boundaries, zonal background, insolation, synoptic systems), `RegionalEcologyState`, `PropertySystem`, `EstateLayout`, `EstateEnergy`, `LocalVegetationPatch`, `HumanSystem`, `HumanEstatePositions`, active labour tasks, `ResourceEconomyState`, audit/chronicle, scheduler state and the pending command queue.
- Produces: `IslandWorldState::state_hash() -> Result<[u8; 32], IslandWorldError>` (canonical form above).

- [x] **Step 1:** Write a bootstrap test asserting the acceptance criteria for island/domain, physical state, ecology, property, layout, patch vegetation and founders from a single constructor, and that `state_hash` is identical across two separate processes (spawn the test binary twice via `std::process::Command` and compare).
- [x] **Step 2:** Run `cargo test -p mk_engine --test island_world_bootstrap`; expect FAIL.
- [x] **Step 3:** Implement the constructor in the spec bootstrap order.
- [x] **Step 4:** Re-run; expect PASS.
- [x] **Step 5:** Commit `feat(engine): compose regional island world`.

Implementation notes: `regional/world.rs` `IslandWorldState` wraps `IslandLife` (which already composes physics, ecology, estate, layout, patch vegetation, humans, energy, materials, economy). **Not yet in the state, added by the tasks that need them:** the fault system and recent earthquakes, the `HashChain`, active labour tasks, the audit/chronicle, scheduler state and the command queue (Tasks 2-4). The cross-process test spawns its own test binary twice and compares hashes with the in-process one.


### Task 2: Cadence-aware deterministic scheduler

**Files:**
- Create: `crates/mk_engine/src/regional/scheduler.rs`
- Modify: `crates/mk_engine/src/regional/world.rs`
- Test: `crates/mk_engine/tests/island_scheduler.rs`

**Interfaces:**
- Consumes: `mk_island::scenario::IslandCadenceProfile` (five cadences, Phase 3 Task 4). Produces: `IslandScheduler`, `SubsystemCadence`, `SchedulerAccumulators`.
- Produces: `IslandWorldState::step(dt_seconds: u64) -> Result<(), IslandWorldError>`.
- Per human substep, in fixed order: apply queued commands for this tick (Task 4) → humans/interactions, active labour tasks and estate energy (`human_seconds`) → weather/ocean/zonal background/insolation (`weather_ocean_seconds`) → hydrology/ecology/resources (`regenerate()` once per firing)/local vegetation (`hydrology_ecology_resource_seconds`) → tectonics/geology and fault stress (`geophysics_seconds`; earthquakes are resolved at the human substep in which they nucleate so shaking reaches humans and structures in the same tick) → human store sync (`human_store_seconds`, Task 3b) → audit/hash commit.

- [ ] **Step 1:** Write tests comparing one 86,400-second step against 1,440 × 60-second calls and requiring identical scheduler counters and final state hash.
- [ ] **Step 2:** Write tests for deterministic remainder handling with irregular `dt_seconds = 3,701` and repeated execution.
- [ ] **Step 3:** Run `cargo test -p mk_engine --test island_scheduler`; expect FAIL.
- [ ] **Step 4:** Implement accumulator-driven substep dispatch in the fixed order above, reusing Phase-2 physical and Phase-3 human/ecology/resource step functions.
- [ ] **Step 5:** Re-run; expect PASS. Slow tier for the 1,440-step comparison if it exceeds ~30 s debug.
- [ ] **Step 6:** Commit `feat(engine): schedule island subsystems`.

### Task 2b: Deep-time spin-up

A realistic island starts with mature soils, rivers and forests, which take centuries to form. Simulating centuries at full detail would take months of computing (~15 s per simulated day ⇒ ~75 days per 1,000 years), so the world is spun up in a fast deep-time mode **before any humans exist**, saved once, and reused. Humans are never fast-forwarded this way: their lives always run at full detail.

**Files:**
- Create: `crates/mk_engine/src/regional/deep_time.rs`
- Modify: `crates/mk_island/src/scenario.rs` (`spin_up_years`, default 1,000), `crates/mk_engine/src/regional/world.rs`
- Test: `crates/mk_engine/tests/island_deep_time.rs`

**Interfaces:**
- `IslandWorldState::spin_up(years: u32) -> Result<SpinUpReport, IslandWorldError>`, allowed only before founders and town are placed. Steps per local year: tectonics, earthquakes, uplift and erosion (annual); soils (annual); hydrology, ecology, biomass, vegetation succession and resources (monthly, 10 steps per local year); climate and weather replaced by a **climatology**, the mean annual cycle (monthly means and variability) measured from one full-detail spin-up year at the start and re-measured every 100 years as terrain and vegetation change. Synoptic weather and the diurnal cycle are represented only by their statistics in this mode.
- `SpinUpReport { years, wall_seconds, equilibrium: EquilibriumMetrics }` with soil carbon, forest biomass and river discharge trends over the last 100 years.
- The spun-up state is snapshotted and cached under `<save_root>/spinup/<digest>.snap`, keyed by the digest of canon + scenario + seed + spin-up settings. Later runs load the cache instead of repeating the spin-up. Bootstrap order becomes: geophysics → physical → ecology → deep-time spin-up → property and estate → founders → town.

- [ ] **Step 1:** Write tests (small test island): spin-up is deterministic and its cache key changes when any input changes; after spin-up, soil carbon and forest biomass trends over the last 100 years are below 1% per century (near equilibrium); spin-up refuses to run once humans exist; one full-detail local year after spin-up has climate statistics matching the climatology used during spin-up within the Phase-2 realism tolerances; loading the cached spin-up then running gives the same hash as spinning up and running.
- [ ] **Step 2:** Run `cargo test -p mk_engine --test island_deep_time`; expect FAIL; implement; re-run; expect PASS.
- [ ] **Step 3:** Measure the full-island 1,000-year spin-up in release (target: a few hours on the reference machine, done once per scenario) and record it in the Phase-5 benchmark.
- [ ] **Step 4:** Add deviation row "deep-time climatology during spin-up" to `docs/island/DEVIATIONS.md` (seeded in Phase 0c as D20). Commit `feat(island): deep-time world spin-up before humans arrive`.

### Task 3: Complete island snapshot save/load

**Files:**
- Create: `crates/mk_engine/src/io/island_snapshot.rs`
- Modify: `crates/mk_engine/src/io/mod.rs`
- Test: `crates/mk_engine/tests/island_snapshot.rs`

**Interfaces:**
- Produces: `save_island_snapshot(state: &IslandWorldState, path: &Path) -> Result<(), IslandSnapshotError>` and `load_island_snapshot(path: &Path) -> Result<IslandWorldState, IslandSnapshotError>`.
- Error variants: digest mismatch, incompatible profile/scenario version, canon mismatch/invalid canon, serialization, filesystem.

- [ ] **Step 1:** Write tests for round-trip equality of `state_hash` (including estate layout, patch trees, estate positions, zonal background and pending commands), tamper rejection, truncation rejection, incompatible fixture profile version and invalid canon.
- [ ] **Step 2:** Run `cargo test -p mk_engine --test island_snapshot`; expect FAIL.
- [ ] **Step 3:** Implement atomic (write temp, fsync, rename) digest-prefixed JSON save/load; re-derive canon-derived values on load; validate scenario/profile compatibility before returning state.
- [ ] **Step 4:** Re-run; expect PASS.
- [ ] **Step 5:** Commit `feat(io): persist complete island world`.

### Task 3b: Per-human folders

**Files:**
- Create: `crates/mk_engine/src/regional/human_store.rs`
- Modify: `crates/mk_engine/src/humans/mod.rs` (`HumanSystem::set_auto_sync`)
- Modify: `crates/mk_engine/src/regional/world.rs`
- Test: `crates/mk_engine/tests/island_human_store.rs`

**Interfaces:**
- Each run owns `<save_root>/<run_id>/humans/`, where `run_id = hex(blake3(scenario digest ‖ seed ‖ run counter))[..12]` and the run counter is persisted in `<save_root>/runs.json`, so a fresh run never reuses an earlier run's folders.
- `IslandWorldState::enable_human_store(save_root: &Path) -> Result<RunId, HumanStoreError>` attaches `HumanStorage` to the registry and creates folders for every current human; every later creation path (founders, `deliver_due_births`, `CreateHuman`) creates the folder at creation time through `HumanRegistry::add_human`.
- Upstream `HumanSystem::step` calls `registry.sync_to_storage()` every call (`humans/mod.rs:880`). Add `HumanSystem::set_auto_sync(bool)` (default `true`, preserving upstream `WorldState` behaviour). The island sets it `false` and syncs on `human_store_seconds`, at every snapshot save, and immediately when a human dies. Event files (`events/`, memories, relationships) are still appended when they happen.
- Storage errors (returned by the Phase-0 registry changes) are counted in the audit trail and shown by the CLI/dashboard, never discarded.

- [ ] **Step 1:** Write tests: founders get folders on enable; a birth during a stepped run creates the child's folder in the same tick with a `born` event and parents' `reproduced` events; folders after a snapshot save match the snapshot's human state; a fresh run with the same seed under the same root uses a new `run_id` directory; with auto-sync off, full-state files are rewritten only on the store cadence; state hash is identical with the store enabled, disabled or failing.
- [ ] **Step 2:** Run `cargo test -p mk_engine --test island_human_store`; expect FAIL.
- [ ] **Step 3:** Implement on top of upstream `HumanStorage` (no second folder format).
- [ ] **Step 4:** Re-run, plus `cargo test -p mk_engine --lib humans::`; expect PASS. Record per-sync cost for the Phase-5 benchmark.
- [ ] **Step 5:** Record `set_auto_sync` in `UPSTREAM.md`. Commit `feat(island): keep a folder for every human`.

### Task 4: External commands and deterministic replay

**Files:**
- Create: `crates/mk_engine/src/regional/commands.rs`
- Create: `crates/mk_engine/src/regional/replay.rs`
- Test: `crates/mk_engine/tests/island_replay.rs`

**Interfaces:**
- Produces: `IslandCommand::{CreateHuman(IslandCreateHumanRequest), Intervention(mk_interventions::InterventionAction), Control(ControlCommand)}` — the only external inputs to the simulation. `ControlCommand::{Pause, Resume, Step(n), Snapshot, SetSpeed(SimSpeed)}` follows upstream `mk serve` naming plus speed. `SimSpeed::{RealTime, Times(u32), AsFastAsPossible}` (`Times` offered as 10, 60, 360, 1,440).
- Produces: `IslandWorldState::queue(command) -> Result<QueuedCommandId, CommandError>`; commands apply at the start of the next human substep in `(tick, sequence)` order, where `sequence` is assigned at queue time.
- Produces: `IslandReplayLog { scenario_digest, entries: Vec<IslandReplayEntry { tick, sequence, command }> }`, appended for every applied command, and `replay_island(canon: Arc<CanonLocked>, scenario: IslandScenario, log: &IslandReplayLog, until_tick: Tick) -> Result<IslandWorldState, IslandReplayError>`.
- Interventions are applied through the upstream executor's logic adapted to `IslandWorldState`; actions with no island meaning (planetary climate edits, orbit changes) return `CommandError::NotSupportedOnIsland` and are listed in `UPSTREAM.md`.

- [ ] **Step 1:** Write a log with interventions and controls at repeated ticks and assert stable `(tick, sequence)` order.
- [ ] **Step 2:** Run uninterrupted execution and replay from the same seed and log; assert identical final `state_hash`.
- [ ] **Step 3:** Save halfway, load, continue replay; assert the same final hash.
- [ ] **Step 4:** Run `cargo test -p mk_engine --test island_replay`; expect FAIL before implementation, PASS after.
- [ ] **Step 5:** Commit `feat(engine): replay deterministic island commands`.

### Task 5: Phase-4 acceptance and headless runner

**Files:**
- Modify: `apps/island/Cargo.toml`, `apps/island/src/main.rs` (created in Phase 0b)
- Test: `crates/mk_engine/tests/island_runtime_acceptance.rs`

**Interfaces:**
- CLI: `island run --scenario <path> --steps <n> --dt <seconds> [--save <path>] [--save-root <dir>]`, `island replay --scenario <path> --log <path> [--save <path>]`, `island inspect --snapshot <path>`; no UI dependency. `--save-root` enables per-human folders under `<save-root>/<run_id>/humans/` (default `./island-data`); the chosen `run_id` is printed.

- [ ] **Step 1:** Add an acceptance test running the default scenario for 100 ticks at `--dt 60`, snapshotting, loading, continuing 100 ticks and matching an uninterrupted 200-tick hash. Slow tier if it exceeds ~30 s debug.
- [ ] **Step 2:** Implement the headless CLI around `IslandWorldState`; default with no command prints a concise world summary and exits successfully.
- [ ] **Step 3:** Run the fixed-seed CLI twice and compare printed final hash; require exact equality.
- [ ] **Step 4:** Run fmt, clippy `-D warnings` and all Phase-4 tests; update `UPSTREAM.md`.
- [ ] **Step 5:** Commit `feat(app): add deterministic island runner`.
- [ ] **Step 6:** Add `island_preview world --snapshot <path> --out <dir>` reusing the Phase 1–3 renderers on a loaded `IslandWorldState`, so any saved run can be inspected headlessly.

### Task 6: Dashboard and Human Creator on the live island

Phase 0b built the dashboard, the Creator page, `CreateHumanRequest`, `create_human`, the extracted upstream spawn core (`build_authored_human`, `agent_id_for_name`) and `ControlAuth`, against a human-only population. This task moves them onto `IslandWorldState`: humans live in the stepped world, gain an island/estate location, and creation becomes an `IslandCommand::CreateHuman`.

**Files:**
- Create: `crates/mk_engine/src/regional/create_human.rs`
- Modify: `apps/island/src/serve/*`, `apps/island/static/*` (from Phase 0b)
- Create: `apps/island/src/serve/projection.rs`
- Test: `crates/mk_engine/tests/island_create_human.rs`, `apps/island/src/serve/server.rs` (`#[tokio::test]`; dev-deps `warp = { workspace = true, features = ["test"] }`, `tokio`)

**Interfaces:**
- Pacing: the simulation clock is independent of the real clock. At `SimSpeed::RealTime` the sim thread advances one simulated second per real second (one 60 s human step per real minute; a 36-hour local day takes 36 real hours); `Times(k)` runs k× faster; `AsFastAsPossible` runs flat out (the default for `island run`/`replay`; `island serve` and `island-ui` default to `RealTime`). If the machine cannot keep up with the requested speed, it runs as fast as it can and the dashboard shows the achieved speed. Speed only changes how often fixed steps run, never their size, so results are identical at any speed; wall-clock time never enters simulation state. Speed changes are replay-log entries for the record but do not affect the state hash.
- Threading: the simulation runs on its own thread and owns `IslandWorldState`. After each step it publishes an immutable `IslandProjection` (roster, human details, properties/estate layout, vegetation summary, economy, timeline, clock) behind an `Arc<RwLock<…>>`; GET handlers read only the projection. Writes send an `IslandCommand` plus a reply channel to the sim thread, which validates against live world state and replies before the handler responds.
- CLI: `island serve --scenario <path> | --snapshot <path> [--bind 127.0.0.1:8080] [--save-root <dir>] [--import-0b <data-dir>]`. Write auth stays Phase 0b's port of upstream `ControlAuth` (`ISLAND_CONTROL_TOKEN` token if set; otherwise loopback-only; non-loopback without a token refuses writes). `--import-0b` replays a Phase-0b `creations.jsonl` as `CreateHuman` commands at tick 0, once.
- Read API: `GET /api/status`, `/api/humans` (roster; `?id=`/`?name=` for full detail), `/api/properties`, `/api/estate`, `/api/vegetation`, `/api/economy`, `/api/timeline`, all from `IslandProjection`. Upstream `mk_view` is not ported: it projects from planetary `WorldState`.
- Write API: `POST /api/humans` accepts `IslandCreateHumanRequest` = Phase 0b's `CreateHumanRequest` + `birthplace_here: bool` + `location: CreateLocation::{EstateSpace { space, position_m }, IslandCell { row, col }}` and now returns `202` (queued for the next tick) instead of 0b's immediate `201`; the dashboard JS polls the command id. `POST /api/control` accepts `ControlCommand`.
- Validation: upstream `validate_intervention` rules (as in 0b) plus location on land or inside an estate space, run on the sim thread. Names follow `agent_id_for_name` (duplicates become `-2`, `-3`, …). Appearance fields are free text as upstream; the page offers upstream's option lists.
- `birthplace_here` maps the chosen location to latitude/longitude with `IslandDomain::lat_lon_at_m`; otherwise explicit coordinates are used (a human may be born off-island).
- On apply: `build_authored_human` with the world `RngRegistry` and current tick; position set from the location (and `HumanEstatePositions` if inside the patch); added through the registry (folder created); `created` event written; command appended to the replay log. A folder-write failure is reported but does not undo the creation.
- Creator page: Phase 0b form plus a location picker (estate plan from `/api/estate`, or island map from the Phase-4 preview data); removes the "time not running" banner.

- [ ] **Step 1:** Write engine tests: identical requests at the same tick produce byte-identical humans; each validation rule rejects with a specific error; a created human appears at the requested space/cell with a folder and `created` event; the creation survives save/load and is reproduced by replay; storage failure still creates the human and reports the error.
- [ ] **Step 1b:** Write pacing tests: the same scenario run for 1,000 steps at `RealTime` (with a fake clock), `Times(1440)` and `AsFastAsPossible` reaches the identical state hash; at `RealTime` with a fake clock, simulated time advances one second per fake second; the achieved-speed readout drops below the requested speed when steps take longer than the pacing budget.
- [ ] **Step 2:** Write API tests with `ISLAND_CONTROL_TOKEN` configured: a `POST /api/humans` without the bearer token returns 401; a valid request returns 202 with the command id and the human is listed by `GET /api/humans` after one step; an invalid request returns 422 with the field errors; `--import-0b` of a Phase-0b data directory yields identical human profiles. Without a token on a loopback bind, the same valid request succeeds.
- [ ] **Step 3:** Run both suites; expect FAIL.
- [ ] **Step 4:** Implement the sim thread, projection, command path and location picker; switch `serve` from the human-only population to `IslandWorldState`.
- [ ] **Step 5:** Re-run; expect PASS. Manually create a human in the Kitchen from the browser and confirm their folder, detail view and estate position.
- [ ] **Step 6:** Commit `feat(island): dashboard and human creator on the live island`.
