# Island App, UI, Performance & Pruning Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Deliver the regional Maer-Ken Island application with map/free-camera rendering, inspectable humans/property/world state, retained assets, measured performance and removal of unused whole-planet runtime dependencies.

**Architecture:** Reuse the proven Maer-Ken Bevy render/procgen/property/human ideas but project them into flat regional metre coordinates rather than a sphere. The app reads a presentation projection from `IslandWorldState`; rendering is one-way and never participates in deterministic state hashing.

**Tech Stack:** Bevy 0.19 + the bevy_egui release that targets it (0.42 at time of writing; confirm the pairing in Task 0), Rust 2021, retained GLB/texture assets, existing deterministic procgen where useful. Upstream `mk_ui`/`mk_view`/`mk_observatory` target Bevy 0.13, so their render/UI code is a behavioural reference to port, not code to copy unchanged.

**Spec:** `docs/superpowers/specs/2026-10-01-maer-ken-island-regional-world-design.md`

## Global Constraints

- Regional terrain/ocean, vegetation, property, humans, vehicles and simulation clock must be visible/inspectable.
- Globe/orbit rendering is excluded.
- Render coordinates derive from `IslandDomain` metres; simulation state never stores Bevy transforms. The island spans ~2,400 km while a room is a few metres, so one global display scale cannot serve both: regional views use a coarse scale and estate/interior views use metres around a floating origin at the estate patch.
- Assets copied from Maer-Ken retain license/source records.
- Performance work follows measurement; structural whole-planet allocations are removed before micro-optimization.

## Review Focus

- Headless mode must still build/run without requiring a display server.
- Missing optional render assets must fail visibly/log clearly without changing simulation state.
- Very large terrain meshes must use bounded LOD/chunk generation rather than one allocation proportional to finest-grid cells.
- Property items sharing a cell must remain visually distinct without changing their simulated location.
- Pruning must not remove modules transitively required by human/resource/property runtime.

---

### Task 0: Bevy 0.19 port inventory

- [ ] **Step 1:** Inventory the upstream code the island will port — `apps/mk_ui` (render: terrain, flora, buildings, vehicles, `render/humans.rs`; UI: `ui/human_inspector.rs`, `ui/human_foundry.rs`), `crates/mk_view` (`human_view.rs`, `human_detail_view.rs`) and `apps/mk_studio/src/watch/human_inspector.rs` — and list each Bevy 0.13 API it uses with its 0.19 replacement (colour types, required components, picking, render graph, asset loading, egui integration).
- [ ] **Step 2:** Pin `bevy = "0.19"` and the matching `bevy_egui` in the workspace; prove a minimal windowed app and a headless build both compile and run in CI.
- [ ] **Step 3:** Record the inventory and pins in `docs/island/RENDER_STACK.md`.
- [ ] **Step 4:** Commit `build(ui): adopt bevy 0.19`.

### Task 1: Regional view projection

**Files:**
- Create: `crates/mk_island_view/Cargo.toml`
- Create: `crates/mk_island_view/src/lib.rs`
- Create: `crates/mk_island_view/src/world.rs`
- Create: `crates/mk_island_view/src/property.rs`
- Create: `crates/mk_island_view/src/human.rs`
- Modify: `Cargo.toml`
- Test: `crates/mk_island_view/tests/projection.rs`

**Interfaces:**
- Produces: `IslandView::from_world(&IslandWorldState) -> Self` with terrain/ocean summaries, clock, humans, aggregate vegetation density, estate layout, patch trees, property and vehicles.
- Produces: `regional_to_render(domain: &IslandDomain, row: usize, col: usize, elevation_m: f64) -> [f32;3]` using regional display scale `1 render unit = 10,000 m`.
- Produces: `estate_to_render(layout: &EstateLayout, position_m: (f64, f64), height_m: f64) -> [f32;3]` using `1 render unit = 1 m` relative to the patch origin (floating origin), and a documented transform between the two frames.

- [ ] **Step 1:** Write tests for deterministic coordinate projection in both frames, consistent regional↔estate transforms, founder/property co-location and no mutation of source state.
- [ ] **Step 2:** Run `cargo test -p mk_island_view`; expect FAIL.
- [ ] **Step 3:** Implement read-only view structs/projection without Bevy dependencies.
- [ ] **Step 4:** Re-run tests; expect PASS.
- [ ] **Step 5:** Commit `feat(view): project regional island state`.

### Task 2: Bevy regional terrain/ocean and camera

**Files:**
- Modify: `apps/island/Cargo.toml`
- Create: `apps/island/src/ui/mod.rs`
- Create: `apps/island/src/ui/terrain.rs`
- Create: `apps/island/src/ui/camera.rs`
- Create: `apps/island/src/ui/app.rs`
- Test: unit tests beside terrain/camera modules.

**Interfaces:**
- CLI adds `island ui --scenario <path>`; `run/save/replay` remain headless.
- Produces deterministic chunk descriptors from `IslandView`; Bevy mesh handles are presentation-only.

- [ ] **Step 1:** Use the Bevy 0.19 / bevy_egui pins from Task 0 and keep headless commands behind no display initialization.
- [ ] **Step 2:** Write tests for terrain chunk partitioning, land/ocean material selection and regional camera bounds.
- [ ] **Step 3:** Implement chunked terrain heightfield + surrounding ocean surface and free/orbit-over-map camera, explicitly not a planetary sphere.
- [ ] **Step 4:** Run headless CLI tests plus app unit tests; expect PASS.
- [ ] **Step 5:** Commit `feat(ui): render regional island terrain`.

### Task 3: Humans, vegetation, property and vehicles rendering

**Files:**
- Create: `apps/island/src/ui/procgen.rs`
- Create: `apps/island/src/ui/humans.rs`
- Create: `apps/island/src/ui/vegetation.rs`
- Create: `apps/island/src/ui/property.rs`
- Copy: selected source assets into `apps/island/assets/`
- Create: `apps/island/assets/CREDITS.md`
- Test: unit tests beside render mapping modules.

**Interfaces:**
- Reuses founder `GemD.glb`/`GemK.glb` and compatible open-license source models/terrain materials.
- Vehicle/building mapping preserves names/kinds from `PropertySystem`; visual offsets are presentation-only deterministic hashes.

- [ ] **Step 1:** Copy only required GLBs/materials and record source/license/provenance; do not vendor the full planetary asset directory.
- [ ] **Step 2:** Port/adapt deterministic procgen helpers required for flora, humans, buildings and current named vehicles.
- [ ] **Step 2a:** Vegetation: render every patch `TreeInstance` individually inside the estate patch; outside it, render GPU-instanced trees whose per-chunk density and kind mix come from the aggregate biomass/biome fields. Instanced trees are presentation-only and deterministic from chunk ID; they are never resource nodes.
- [ ] **Step 2b:** Buildings and interiors: generate meshes from `EstateLayout` footprints/rooms/doors and place items at their `ItemPlacement` positions; an interior view shows rooms with their furniture, tools, vehicles and computers.
- [ ] **Step 3:** Write tests covering all six property building kinds, every room in the layout, every current named vehicle class, founder model aliases, instanced-tree density following aggregate biomass, and deterministic offsets.
- [ ] **Step 4:** Implement render synchronization from `IslandView` in flat regional coordinates.
- [ ] **Step 5:** Run app/render tests; expect PASS.
- [ ] **Step 6:** Commit `feat(ui): render island life and property`.

### Task 4: Human tooling — views, inspector, foundry, portraits

Upstream has more human tooling than the island imported. The engine is already identical; this task brings the surrounding tools across.

**Files:**
- Create: `crates/mk_island_view/src/human_detail.rs` (port of upstream `mk_view::human_view`/`human_detail_view` against `IslandWorldState`)
- Create: `apps/island/src/ui/human_inspector.rs`, `apps/island/src/ui/human_foundry.rs` (ports of upstream `mk_ui` equivalents to Bevy 0.19)
- Copy: upstream `apps/mk_ui/assets/portraits/*` into `apps/island/assets/portraits/` with provenance in `CREDITS.md`
- Test: pure-state tests beside each module

**Interfaces:**
- Human detail view exposes the same sections upstream does (identity, body/vitals, needs, emotion, cognition/attention, memory, relationships, development/lifecycle, current room/position) from read-only state.
- The foundry creates new humans only through `HumanBeing::new_born_at` and adds them through a queued simulation command, so creation is deterministic, replayable and recorded in the replay log — never by mutating state from the renderer.

- [ ] **Step 1:** Write tests: detail view for Gem-D/Gem-K matches the profile values; a foundry request with identical inputs yields the same `human_id`; a duplicate agent id is rejected; foundry creations appear in the replay log and survive save/load.
- [ ] **Step 2:** Port views, inspector and foundry; wire the founders' GLB models and portraits.
- [ ] **Step 3:** Run tests; expect PASS.
- [ ] **Step 4:** Commit `feat(ui): port human views, inspector and foundry`.

### Task 4b: Computer service (opt-in)

- [ ] **Step 1:** Import upstream `apps/computer-service` (the real web-search/email backend for `ActionKind::WebSearch`/`SendEmail`) with provenance, keeping it opt-in via `COMPUTER_ACTIONS_ENABLED=1` exactly as upstream.
- [ ] **Step 2:** Test that with the service disabled, replay/state hashes are unchanged and humans never attempt those actions; with it enabled, results enter only through the existing bridge and are excluded from the deterministic hash.
- [ ] **Step 3:** Commit `feat(island): import opt-in computer service`.

### Task 5: Inspectors and simulation controls

**Files:**
- Create: `apps/island/src/ui/inspectors.rs`
- Modify: `apps/island/src/ui/app.rs`
- Test: pure-state tests for inspector selection/projection.

**Interfaces:**
- Inspector exposes simulation clock, selected terrain cell, selected room/item/tree, human summary (including current room), property/building/item/network-account summary and physical/environmental values from `IslandView`.
- Controls: pause/resume, single deterministic step, save snapshot; no arbitrary state mutation through renderer.

- [ ] **Step 1:** Write tests for selection lookup and inspector data coming only from current `IslandView`.
- [ ] **Step 2:** Implement egui panels and pause/step/save command queue into the simulation owner.
- [ ] **Step 3:** Run headless tests and UI pure-state tests; expect PASS.
- [ ] **Step 4:** Commit `feat(ui): inspect regional world state`.

### Task 6: Repeatable performance baselines

**Files:**
- Create: `apps/island/src/bin/island_bench.rs`
- Create: `benchmarks/README.md`
- Create after measurement: `benchmarks/2026-10-01-regional-baseline.json`
- Test: `crates/mk_engine/tests/island_allocation_bounds.rs`

**Interfaces:**
- `island_bench --scenario fixtures/island/default_scenario.json --ticks 1000 --dt 60 --json` emits `bootstrap_ms`, `serialized_state_bytes`, `ticks_per_second`, `save_ms`, `load_ms`, `snapshot_bytes`, and named per-major-subsystem elapsed milliseconds. These are measured from the real regional paths; no guessed memory figure is reported.

- [ ] **Step 1:** Add allocation-bound test asserting default coarse/medium cell counts match the profile and no `WorldState::new`/planetary `32×64` bootstrap is called by `IslandWorldState`.
- [ ] **Step 2:** Implement benchmark instrumentation around actual regional bootstrap/step/save/load paths.
- [ ] **Step 3:** Run benchmark in release mode and save the JSON baseline; do not invent target numbers before measurement.
- [ ] **Step 4:** Commit `perf(island): record regional baseline`.

### Task 7: Upstream sync and planetary-only pruning

**Files:**
- Create: `scripts/sync-maerken-upstream`
- Modify: `UPSTREAM.md`
- Modify/delete: workspace/module entries proven unreachable from normal island execution.
- Remove: `apps/island_humans` after its tests/functionality are covered by `IslandWorldState`/`apps/island`.

**Interfaces:**
- `scripts/sync-maerken-upstream --check <Maer-Ken path>` reports changed imported paths against pinned commit and never applies changes silently.
- `--apply <Maer-Ken path> --commit <sha>` copies only the documented import set then leaves island divergences for normal review.

- [ ] **Step 1:** Write script tests using temporary git fixtures for unchanged, changed and missing upstream paths.
- [ ] **Step 2:** Implement check/apply modes with explicit commit pin and clean-worktree guard.
- [ ] **Step 3:** Use `cargo tree`, source search and tests to identify planetary-only modules not reachable by regional app; remove in small commits while running focused tests after each cut.
- [ ] **Step 4:** Remove obsolete `apps/island_humans` and update README/docs to make `apps/island` the project entry point.
- [ ] **Step 5:** Run full workspace fmt/clippy/tests, fixed-seed replay comparison and release benchmark.
- [ ] **Step 6:** Update `UPSTREAM.md` with final retained module/asset list and sync procedure.
- [ ] **Step 7:** Commit `refactor(island): prune planetary-only runtime`.

### Task 8: Final acceptance

**Files:**
- Create: `docs/ISLAND_SYSTEM_STATUS.md`
- Modify: `README.md`

**Interfaces:**
- Documentation records implemented systems, benchmark numbers, known model-fidelity limitations and exact verification commands.

- [ ] **Step 1:** Run every acceptance criterion from the approved design and record command/output summaries.
- [ ] **Step 2:** Verify fixed-seed land area/buffer, all retained systems, founders/property, snapshot/replay determinism, UI projection and absence of normal whole-planet allocation.
- [ ] **Step 3:** Write status/README from measured/tested results only; no planned feature may be described as implemented.
- [ ] **Step 4:** Run `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace -- --test-threads=1`; expect all green.
- [ ] **Step 5:** Commit `docs(island): record completed regional build`.
