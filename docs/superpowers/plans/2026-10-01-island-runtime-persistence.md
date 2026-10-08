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

- [x] **Step 1:** Write tests comparing one 86,400-second step against 1,440 × 60-second calls and requiring identical scheduler counters and final state hash.
- [x] **Step 2:** Write tests for deterministic remainder handling with irregular `dt_seconds = 3,701` and repeated execution.
- [x] **Step 3:** Run `cargo test -p mk_engine --test island_scheduler`; expect FAIL.
- [x] **Step 4:** Implement accumulator-driven substep dispatch in the fixed order above, reusing Phase-2 physical and Phase-3 human/ecology/resource step functions.
- [x] **Step 5:** Re-run; expect PASS. Slow tier for the 1,440-step comparison if it exceeds ~30 s debug.
- [x] **Step 6:** Commit `feat(engine): schedule island subsystems`.

Implementation notes: `regional/scheduler.rs` (`IslandScheduler`, `SchedulerCounters`, `SchedulerAccumulators`, `Due`); `IslandLife::advance` now takes its cadences from the scenario and dispatches by the simulated clock (firings depend only on the clock, so slicing time differently cannot change them). A remainder shorter than a substep is carried (3,701 s = 61 whole minutes + 41 s waiting). Verified: 1 h as one step equals 60 one-minute steps; two 3,701 s steps equal one 7,402 s step; a full day in one step equals 1,440 steps (slow tier). Geophysics and human-store firings are counted but their work is Task 2b/3b; the command queue (Task 4) is not yet applied at the head of a substep. The scheduler state is in the state digest, so the week digest was re-pinned (`bf836cb8…6e3f`).


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
- Produces: `save_island_snapshot(life: &IslandLife, path: &Path) -> Result<(), IslandSnapshotError>` and `load_island_snapshot(canon: Arc<CanonLocked>, path: &Path) -> Result<IslandLife, IslandSnapshotError>`. `IslandLife` is the composition Task 1 delivered under that name; the canon is supplied by the caller rather than read from the file, so a snapshot cannot bring its own physics.
- Error variants: digest mismatch, incompatible profile/scenario version, canon mismatch/invalid canon, serialization, filesystem.

- [x] **Step 1:** Write tests for round-trip equality of `state_hash` (including estate layout, patch trees, estate positions, zonal background and pending commands), tamper rejection, truncation rejection, incompatible fixture profile version and invalid canon.
- [ ] **Step 2:** Run `cargo test -p mk_engine --test island_snapshot`; expect FAIL. Not done as written: the tests and the module were written together, so there was never a run of this file against a missing implementation.
- [x] **Step 3:** Implement atomic (write temp, fsync, rename) digest-prefixed JSON save/load; re-derive canon-derived values on load; validate scenario/profile compatibility before returning state.
- [x] **Step 4:** Re-run; expect PASS.
- [x] **Step 5:** Commit `feat(io): persist complete island world`.

**Built (2026-10-07):** the file is `magic ‖ blake3(rest) ‖ canon digest ‖ deflated JSON`, written
to a temporary file beside the target, fsynced and renamed, with the directory entry fsynced after.
Written plainly the full island came to 338 MB — a dozen grids of 1,152,000 cells, mostly decimal
expansions of numbers close to their neighbours — so the state is deflated, which brings the same
island to 44 MB, and the digest covers the compressed bytes that are actually on the disk. `IslandLife::restore` validates the scenario as
if it had been read from disk, then derives the domain, grid topology and labour table again
instead of trusting the file. The round-trip tests assert the state digest both immediately and
after simulating the same hours on both sides, which is what catches state left out of the
snapshot.

Two things Step 1 names are not covered because they do not exist yet: there are no pending
commands until Task 4, and the zonal background is part of the physical state the round trip
already compares through the digest rather than a separate field to assert on.


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

- [x] **Step 1:** Write tests: founders get folders on enable; a birth during a stepped run creates the child's folder in the same tick with a `born` event and parents' `reproduced` events (covered as `somebody_added_after_the_store_is_open_gets_a_folder_at_once`, which pins `HumanRegistry::add_human`'s storage branch — the call every birth path reaches — rather than waiting out a pregnancy; the `born`/`reproduced` event files are *not* pinned, and nothing guards them yet); folders after a snapshot save match the snapshot's human state; a fresh run with the same seed under the same root uses a new `run_id` directory; with auto-sync off, full-state files are rewritten only on the store cadence; state hash is identical with the store enabled, disabled or failing.
- [ ] **Step 2:** Run `cargo test -p mk_engine --test island_human_store`; expect FAIL. Not done as written, as in Task 3: the tests and the module were written together.
- [x] **Step 3:** Implement on top of upstream `HumanStorage` (no second folder format).
- [x] **Step 4:** Re-run, plus `cargo test -p mk_engine --lib humans::`; expect PASS. Record per-sync cost for the Phase-5 benchmark.
- [x] **Step 5:** Record `set_auto_sync` in `UPSTREAM.md`. Commit `feat(island): keep a folder for every human`.

**Built (2026-10-07):** `open_run` claims the next run under the save root — the counter in
`runs.json` is written back *before* any human is, so a crash mid-run burns an id rather than
reusing one — and the id is `blake3(scenario digest ‖ seed ‖ counter)[..12]`, stable for a given
scenario, seed and run number rather than a timestamp. The store opens through
`HumanStorage::try_new`, which refuses a missing storage key instead of writing people's records
in plaintext.

`IslandLife::enable_human_store` sets `auto_sync` false and syncs on `human_store_seconds` (the
cadence and its counter already existed in the scheduler and were computed but never acted on),
at every `save_island_snapshot` — which now takes `&mut` for that reason, so a checkpoint
checkpoints everything — and immediately when somebody dies, which is the one moment a stale file
would be a lie rather than a lag. Failed writes are counted on the store with their most recent
reason and surfaced by the CLI, never discarded: a human whose folder will not write still exists.

What the tests are actually for: `keeping_records_does_not_change_what_happens` and
`the_island_runs_on_when_every_write_fails` both assert an identical state digest against an
island with no store at all. A world that ran differently when asked to keep records would not be
worth the records.

`crates/mk_engine/src/regional/world.rs` is not modified: this repository's composition is
`regional/life.rs` (`IslandLife`), which is where the store lives.


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

- [x] **Step 1:** Write a log with interventions and controls at repeated ticks and assert stable `(tick, sequence)` order.
- [x] **Step 2:** Run uninterrupted execution and replay from the same seed and log; assert identical final `state_hash`.
- [x] **Step 3:** Save halfway, load, continue replay; assert the same final hash.
- [x] **Step 4:** Run `cargo test -p mk_engine --test island_replay`; PASS. The FAIL half was not done as written, as in Tasks 3 and 3b: the tests and the module were written together.
- [x] **Step 5:** Commit `feat(engine): replay deterministic island commands`.

**Built (2026-10-08).** `IslandCommand::{CreateHuman, Control}` in `regional/commands.rs` is the
only way in from outside, and `IslandLife::apply_command` records every applied command in an
`IslandReplayLog` with the `(tick, sequence)` it applied at. `replay_island` runs a scenario
forward, applying the log as it goes, and reaches the same canonical digest.

Measured end to end rather than only in tests: a dashboard was run at 2,000x, three people were
created through `POST /api/world/humans` from the command line, and the session's log was
replayed on its own:

    live dashboard, tick 720   -> 628d33bc0bd85c59...
    island replay --until 720  -> 628d33bc0bd85c59...

The log recorded the three creations at ticks 480, 544 and 609 — where they actually landed —
and the replay rebuilt all five islanders.

`sequence` is why commands landing on one tick come back in order: the second creation of a name
becomes `name-2`, which only holds if the first is already there. `replay_island` sorts by
`(tick, sequence)` rather than trusting the file's order, and
`a_shuffled_log_still_reproduces_the_run` reverses a log and requires the same digest.

Control commands are recorded and apply nothing — pausing an island leaves the state it was
paused in, and a speed is a statement about real time. `control_commands_are_recorded_and_change_
nothing` asserts that rather than assuming it.

`island serve --log <path>` records a dashboard session, written as each command lands rather
than at the end, because a log that only existed in memory would be lost by the thing a log is
for. `island run --log` does the same, and `island replay --scenario --log [--until]` runs one
again.

**Not built:** `IslandCommand::Intervention`. Upstream's `InterventionAction` executor adapted to
the island is its own piece of work, and the `NotSupportedOnIsland` list it needs belongs with
it. Nothing here pretends otherwise: the variant does not exist rather than existing and
refusing.


### Task 5: Phase-4 acceptance and headless runner

**Files:**
- Modify: `apps/island/Cargo.toml`, `apps/island/src/main.rs` (created in Phase 0b)
- Test: `crates/mk_engine/tests/island_runtime_acceptance.rs`

**Interfaces:**
- CLI: `island run --scenario <path> --steps <n> --dt <seconds> [--save <path>] [--save-root <dir>]`, `island replay --scenario <path> --log <path> [--save <path>]`, `island inspect --snapshot <path>`; no UI dependency. `--save-root` enables per-human folders under `<save-root>/<run_id>/humans/` (default `./island-data`); the chosen `run_id` is printed.

- [x] **Step 1:** Add an acceptance test running the default scenario for 100 ticks at `--dt 60`, snapshotting, loading, continuing 100 ticks and matching an uninterrupted 200-tick hash. Slow tier if it exceeds ~30 s debug.
- [x] **Step 2:** Implement the headless CLI around `IslandWorldState`; default with no command prints a concise world summary and exits successfully.
- [x] **Step 3:** Run the fixed-seed CLI twice and compare printed final hash; require exact equality.
- [x] **Step 4:** Run fmt, clippy `-D warnings` and all Phase-4 tests; update `UPSTREAM.md`.
- [x] **Step 5:** Commit `feat(app): add deterministic island runner`.
- [ ] **Step 6:** Add `island_preview world --snapshot <path> --out <dir>` reusing the Phase 1–3 renderers on a loaded `IslandWorldState`, so any saved run can be inspected headlessly.

**Built (2026-10-07):** `island run` and `island inspect`, in `apps/island/src/run.rs`. Measured on
the full island from the command line, not only in tests:

    island run --scenario fixtures/island/default_scenario.json --steps 60   -> 4f6502fd…
    island run --scenario fixtures/island/default_scenario.json --steps 60   -> 4f6502fd…
    island run --snapshot a.mks --steps 60                                   -> d645a097…
    island run --scenario fixtures/island/default_scenario.json --steps 120  -> d645a097…

Two runs of one scenario and seed agree, and a run that stopped at 60 steps and resumed from the
file reaches the same digest as one that never stopped — at the island's real size, 199,997 stems
and a 43 MB snapshot. `island_runtime_acceptance.rs` pins both, plus the independence of `--dt`:
the same simulated hour in six steps of ten lands where one step of sixty does.

`--save-root` enables Task 3b's per-human folders and prints the run id. `island inspect` reads a
snapshot and describes it without running it.

**Not built:** `island replay --log`, which needs Task 4's command log, and Step 6's
`island_preview world --snapshot`. The pacing and speed control in Task 6's interfaces section
belong with the sim thread; `island run` is deliberately unpaced — wall-clock time never enters
simulation state, so a headless run goes as fast as the machine allows.


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

- [x] **Step 1:** Write engine tests: identical requests at the same tick produce byte-identical humans; each validation rule rejects with a specific error; a created human appears at the requested space/cell with a folder and `created` event; the creation survives save/load and is reproduced by replay; storage failure still creates the human and reports the error.
- [ ] **Step 1b:** Write pacing tests: the same scenario run for 1,000 steps at `RealTime` (with a fake clock), `Times(1440)` and `AsFastAsPossible` reaches the identical state hash; at `RealTime` with a fake clock, simulated time advances one second per fake second; the achieved-speed readout drops below the requested speed when steps take longer than the pacing budget.
- [x] **Step 2:** Write API tests with `ISLAND_CONTROL_TOKEN` configured: a `POST /api/humans` without the bearer token returns 401; a valid request returns 202 with the command id and the human is listed by `GET /api/humans` after one step; an invalid request returns 422 with the field errors; `--import-0b` of a Phase-0b data directory yields identical human profiles. Without a token on a loopback bind, the same valid request succeeds.
- [ ] **Step 3:** Run both suites; expect FAIL.
- [x] **Step 4:** Implement the sim thread, projection, command path and location picker; switch `serve` from the human-only population to `IslandWorldState`.
- [x] **Step 5:** Re-run; expect PASS. Manually create a human in the Kitchen from the browser and confirm their folder, detail view and estate position.
- [ ] **Step 6:** Commit `feat(island): dashboard and human creator on the live island`.

**Built (2026-10-07), except where noted at the end.** The island now runs behind the dashboard:
`island serve --scenario <path> | --snapshot <path> [--speed real|max|<n>]` starts
`IslandLife` on its own thread (`serve/sim.rs`), which publishes an `IslandProjection`
(`serve/projection.rs`) behind an `RwLock` after every step. `GET /api/world` reads only that,
so a request can never hold up a step and no page can catch the island half-stepped. The
overview page shows the island's clock on its own 36-hour day, the founders with where they are
and their body carbon, the estate's power, the patch, and whether the books closed; it refreshes
every two seconds.

Pacing works as the interfaces above require, and `how_fast_it_runs_does_not_change_what_happens`
pins it: at any speed the thread's island is the island a plain `advance` reaches. Wall-clock time
never enters simulation state.

Two things running it taught, neither of which a test of mine would have caught:

- The achieved-speed readout claimed **20,000x while the island was advancing at 2,160x**. It
  timed `advance` only, and averaged over a count of steps — but one step in sixty also hashes
  the island and costs hundreds of times more, so a step-count window usually fell between two of
  them. It now times the whole loop over a five-second wall-clock window.
- A step is **1.4 ms** and a digest is **932 ms** (`slow_what_a_step_and_a_digest_cost`), so
  publishing the digest every step made the island hundreds of times slower than it needed to be.
  The projection now carries a digest with the tick it was taken at, refreshed hourly, and the
  page says "as of tick N" rather than implying it is live.

The page's own copy was wrong once the clock started — it said "The island, before the clock
starts" and "Time is not running" — so the overview now swaps both when a world is running and
keeps the old wording when there is none. Checked in Chromium at 390 px against both: no console
errors, no failed requests, no sideways scroll.

**The write half (2026-10-07).** `IslandCommand` is queued to the sim thread and applied between
steps, so a request never reaches into a running world. `POST /api/world/humans` answers **202**
with a command id and a poll URL; `GET /api/world/commands/{id}` says what became of it. The
creator page offers the estate's own 14 spaces by the layout's ids — so it cannot ask for a room
the island will refuse — and follows the command to a created person or a refusal with reasons.
Write auth is the same rule as storing a person: a world write is no less a write for happening a
step later.

`IslandLife::create_human` validates against the live world on its own thread, which is the only
place those answers are true: a room that exists, a cell that is land and not sea, an age inside
the packs' verified maximum of 122.45 years. Every problem is collected and returned together, so
a form learns all of them at once.

Two things the tests caught that the code did not do:

- A created person had an estate position but no **runtime grid position**, so the next step saw
  them outside the estate block and dropped them. The founders are placed by both; so are they
  now (`somebody_created_in_a_room_is_in_that_room_and_in_the_world`).
- A created person was never registered in the **material ledger**, so they carried no body
  carbon, respired nothing and ate nothing — visibly on the page, and not actually alive. The
  same test now requires their body carbon to fall over an hour, as a founder's does.

Verified in Chromium end to end: filled the creator form, picked a room, and watched the person
appear in the running island with body carbon beside the founders — "hine awake · Gem-D's
Bedroom · 30 years old · 19.2 kg of body carbon". No console errors, no failed requests.

**Control and the world's parts (2026-10-08).** `POST /api/control` takes `pause`, `resume` and
`set_speed`, and the overview carries buttons for them. Pacing is read by the loop each
iteration rather than captured at startup, so it changes without restarting the island — and
changes nothing about the island: a pause leaves exactly the state it was paused in, and a speed
decides only how often a fixed step happens. Measured in Chromium: pause held the clock dead
still at tick 1080 across a second of real time, resume moved it again, and `60x` took effect.
Control commands are applied at once rather than queued, since there is no world state for them
to race with, and are then recorded in the replay log for the account of what the operator did.

`GET /api/world/{estate|vegetation|materials|clock}` serves each part on its own path, from the
same projection, and an unknown part is refused by name.

**Still not built:** `--import-0b`, and an island-map picker for `IslandCell` — the API takes row
and col and the engine validates them against the land mask, but the creator page offers only
estate spaces. `/api/properties`, `/api/economy` and `/api/timeline` are not served: the
projection does not carry a property list, an economy summary or a timeline, so serving them
would mean inventing the data rather than re-slicing it. The roster is the world's. `GET /api/world/humans` and
`/api/world/humans/{id}` serve the island's own people in the shape the People page already
draws — the same ten per-person sections — plus where they are and whether they are asleep,
which the stored roster has no way to know. With a world running the page uses them, and says
what it is showing: "2 people living on the island. Their full records were read 59 steps ago;
where they are and whether they are asleep is current." Its standing copy claimed "nothing has
stepped them yet", so that swaps too.

Full records are published on the digest's hourly cadence rather than every step, and
additionally the moment a creation lands — `a_new_person_can_be_read_the_moment_they_exist` pins
that. The cadence is a choice about scale rather than a present necessity: measured, copying
every record costs 336 µs for the two founders against a 1.2 ms step, which is cheap, but at
Phase 4b's town it would be tens of milliseconds per step for records nobody is reading.

Correcting the measurements while here: a digest is **800 ms**, not the 932 ms recorded earlier,
and a projection without one is 3.6 ms. The earlier figure came from a timing block where an
inserted line had left "one projection" printing the wrong timer — clippy caught the shadowed
variable, and the numbers above are from the fixed block.

`/api/humans` still serves the stored population, which is right: a dashboard started without
`--scenario` has no world, and that endpoint is what it has.
