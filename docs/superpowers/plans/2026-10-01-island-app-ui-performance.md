# Island App, UI, Performance & Pruning Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Deliver the regional Maer-Ken Island desktop application with map/free-camera rendering, inspectable humans/property/world state, retained assets, measured performance and removal of unused whole-planet runtime dependencies.

**Architecture:** Reuse the proven Maer-Ken Bevy render/procgen/property/human ideas but project them into flat regional metre coordinates rather than a sphere. The UI reads a presentation projection from `IslandWorldState`; rendering is one-way and never participates in deterministic state hashing. Bevy lives only in a separate `apps/island_ui` crate (binary `island-ui`); the headless `apps/island` binary (`run`, `replay`, `inspect`, `serve`) never links Bevy, winit or audio, so it builds without system graphics/audio packages.

**Tech Stack:** Bevy 0.19 + the bevy_egui release that targets it (0.42 at time of writing; confirm the pairing in Task 0), Rust 1.97.0 (pinned in Phase 0), retained GLB/texture assets, existing deterministic procgen where useful. Upstream `mk_ui`/`mk_studio` target Bevy 0.13, so their render/UI code is a behavioural reference to port, not code to copy unchanged.

**Spec:** `docs/superpowers/specs/2026-10-01-maer-ken-island-regional-world-design.md`

**Depends on:** Phase 4.

## Global Constraints

- Regional terrain/ocean, vegetation, property, humans, vehicles and simulation clock must be visible/inspectable.
- Globe/orbit rendering is excluded.
- Render coordinates derive from `IslandDomain` metres; simulation state never stores Bevy transforms. The island spans ~2,400 km while a room is a few metres, so one display scale cannot serve both: regional views use a coarse scale with documented vertical exaggeration, and estate/interior views use metres around a floating origin at the estate patch.
- All runtime assets live under the repository-root `assets/` tree (spec §11: `assets/humans`, `assets/property`, `assets/vehicles`, `assets/tools`, `assets/computers`, `assets/terrain`, `assets/vegetation`); `island-ui` sets Bevy's `AssetPlugin { file_path }` to that directory.
- Assets copied from Maer-Ken carry their upstream source and licence/provenance notes in `assets/CREDITS.md`, including upstream's own caveats.
- Performance work follows measurement; structural whole-planet allocations are removed before micro-optimization.

## Review Focus

- `apps/island` must build and run on a machine with no display and no graphics/audio dev packages.
- Missing optional render assets must fail visibly/log clearly without changing simulation state.
- Very large terrain meshes must use bounded LOD/chunk generation rather than one allocation proportional to finest-grid cells.
- Property items sharing a space must remain visually distinct without changing their simulated location.
- Pruning must not remove modules transitively required by human/resource/property runtime.

---

### Task 0: Bevy 0.19 port inventory and UI crate

**Files:**
- Create: `apps/island_ui/Cargo.toml` (binary `island-ui`), `apps/island_ui/src/main.rs`
- Modify: `Cargo.toml` (workspace member; `bevy`, `bevy_egui` in `[workspace.dependencies]`)
- Modify: `crates/mk_core/Cargo.toml`, `apps/island_humans/Cargo.toml` (`rust-version` → the minimum Bevy 0.19 requires, ≤ 1.97)
- Modify: `.github/workflows/ci.yml`
- Create: `docs/island/RENDER_STACK.md`

- [ ] **Step 1:** Inventory the upstream code to port and list each Bevy 0.13 API it uses with its 0.19 replacement (colour types, required components, picking, render graph, asset loading, egui integration): `apps/mk_ui/src/render/{planet,vegetation,property,procgen,humans,lighting,resources}.rs`, `apps/mk_ui/src/ui/human_inspector.rs`, `apps/mk_studio/src/ui/foundry_panel.rs` (the foundry that actually submits `SpawnHuman`; upstream `mk_ui/ui/human_foundry.rs` only saves templates to `foundry_library.json`), `apps/mk_studio/src/watch/human_inspector.rs`, and `crates/mk_view/src/{human_view,human_detail_view}.rs`.
- [ ] **Step 2:** Pin `bevy = "0.19"` and the matching `bevy_egui`; a minimal `island-ui` window opens locally.
- [ ] **Step 3:** CI: add a job that installs the Linux build dependencies (`libasound2-dev`, `libudev-dev`, `libwayland-dev`, `libxkbcommon-dev`) and runs `cargo build -p island_ui` and its pure-state unit tests. No windowed test runs in CI. Confirm `cargo tree -p island` contains no `bevy`.
- [ ] **Step 4:** Record the inventory, pins and CI packages in `docs/island/RENDER_STACK.md`.
- [ ] **Step 5:** Commit `build(ui): add island-ui crate on bevy 0.19`.

### Task 1: Regional view projection

**Files:**
- Create: `crates/mk_island_view/Cargo.toml` (deps: `mk_engine`, `mk_island`, `serde`; no Bevy)
- Create: `crates/mk_island_view/src/{lib.rs,world.rs,property.rs,human.rs}`
- Modify: `Cargo.toml`
- Modify: `apps/island/src/serve/projection.rs` (Phase 4) to reuse `mk_island_view` types where they overlap
- Test: `crates/mk_island_view/tests/projection.rs`

**Interfaces:**
- Produces: `IslandView::from_world(&IslandWorldState) -> Self` with terrain/ocean summaries, clock, humans, aggregate vegetation density, estate layout, patch trees, property and vehicles.
- Produces: `regional_to_render(domain: &IslandDomain, level: DomainLevel, row: usize, col: usize, elevation_m: f64) -> [f32; 3]` using `1 render unit = 10,000 m` horizontally and a documented vertical exaggeration (default 20×; relief of ~3 km is otherwise ~0.3 units).
- Produces: `estate_to_render(layout: &EstateLayout, position_m: (f64, f64), height_m: f64) -> [f32; 3]` using `1 render unit = 1 m` relative to the patch origin (floating origin), and `estate_origin_in_regional(layout, domain) -> [f32; 3]` for the transform between frames.

- [ ] **Step 1:** Write tests for deterministic projection in both frames, consistent regional↔estate transforms, founder/property co-location and no mutation of source state.
- [ ] **Step 2:** Run `cargo test -p mk_island_view`; expect FAIL.
- [ ] **Step 3:** Implement read-only view structs/projection.
- [ ] **Step 4:** Re-run; expect PASS.
- [ ] **Step 5:** Commit `feat(view): project regional island state`.

### Task 2: Regional terrain/ocean and camera

**Files:**
- Create: `apps/island_ui/src/{app.rs,terrain.rs,camera.rs,sim.rs}`
- Test: unit tests beside terrain/camera modules.

**Interfaces:**
- CLI: `island-ui --scenario <path> | --snapshot <path> [--save-root <dir>]`. `sim.rs` owns `IslandWorldState` on a simulation thread exactly as `island serve` does (Phase 4 Task 6) and publishes `IslandView` after each step; `HttpComputerBridge::from_env()` is attached when `COMPUTER_ACTIONS_ENABLED=1`.
- Produces deterministic chunk descriptors from `IslandView`; Bevy mesh handles are presentation-only.

- [ ] **Step 1:** Write tests for terrain chunk partitioning, land/ocean material selection and regional camera bounds (pure functions, no window).
- [ ] **Step 2:** Implement chunked terrain heightfield + surrounding ocean surface and a free/orbit-over-map camera, explicitly not a planetary sphere. Lighting follows the island canon: sun position from the 36-hour day, 27° tilt and 323-day year; both moons rendered with their phases; clouds and rain from the synoptic weather.
- [ ] **Step 3:** Run `cargo test -p island_ui` and `cargo test -p island`; expect PASS.
- [ ] **Step 4:** Commit `feat(ui): render regional island terrain`.

### Task 3: Humans, vegetation, property and vehicles rendering

**Files:**
- Create: `apps/island_ui/src/{procgen.rs,humans.rs,vegetation.rs,property.rs}`
- Copy: selected upstream assets into `assets/` subdirectories; generator scripts they come from (e.g. upstream `tools/assets/generate_gem_property_assets.py`, `generate_maerken_terrain_materials.py`) into `tools/assets/`
- Modify: `assets/CREDITS.md`
- Test: unit tests beside render mapping modules.

**Interfaces:**
- Reuses founder `assets/humans/GemD.glb`/`GemK.glb` and compatible upstream property/vehicle/terrain models and materials.
- Vehicle/building mapping preserves names/kinds from `PropertySystem`; visual offsets are presentation-only deterministic hashes.

- [ ] **Step 1:** Copy only required GLBs/materials and their generator scripts; record source and upstream licence/provenance notes; do not vendor the full planetary asset directory.
- [ ] **Step 2:** Port/adapt deterministic procgen helpers required for flora, humans, buildings and current named vehicles.
- [ ] **Step 2a:** Vegetation: render every patch `TreeInstance` individually inside the estate patch, and patch `StandCover` as instanced trees at its stem density; outside the patch, render GPU-instanced trees whose per-chunk density and kind mix come from `biomass_kgc_m2`/biome. Instanced trees are presentation-only and deterministic from chunk ID; they are never resource nodes.
- [ ] **Step 2b:** Buildings and interiors: generate meshes from `EstateLayout` footprints/spaces/doors and place items at their `ItemPlacement` positions; an interior view shows rooms with their furniture, tools, vehicles and computers.
- [ ] **Step 2d:** Industry and town (Phase 4b): the river and its weirs, penstocks and powerhouses; power lines; oil wells, flowlines, tanks, refinery and gas plant; the town's houses, supermarket and other buildings from their layouts; farm fields and livestock, the fishing boat and wharf; forestry blocks (cut, regrowing, standing), log trucks, the sawmill and houses under construction; vehicles at the depot; live overlays for river flow, power flow and plant status.
- [ ] **Step 2c:** Resources: every `ResourceNodeKind` has a visual (outcrops by lithology, mine workings on deposits, river gravel with placers, ironsand beaches, game, fisheries, bee colonies, salt pans) and every catalogue item has an inventory icon; a test enumerates both lists and fails on any missing entry.
- [ ] **Step 3:** Write tests covering all six property building kinds (the computer room rendered as a room inside the House, not a separate building), every space in the layout, every current named vehicle class, founder model aliases, instanced-tree density following biomass, and deterministic offsets.
- [ ] **Step 4:** Implement render synchronization from `IslandView`.
- [ ] **Step 5:** Run `cargo test -p island_ui`; expect PASS.
- [ ] **Step 6:** Commit `feat(ui): render island life and property`.

### Task 4: Human tooling — views, inspector, foundry, portraits

The engine is already identical to upstream; this task brings the surrounding desktop tools across.

**Files:**
- Create: `crates/mk_island_view/src/human_detail.rs` (port of upstream `mk_view::human_view`/`human_detail_view` sections against `IslandWorldState`)
- Create: `apps/island_ui/src/human_inspector.rs` (port of `mk_ui/ui/human_inspector.rs` and `mk_studio/watch/human_inspector.rs`)
- Create: `apps/island_ui/src/foundry.rs` (port of `mk_studio/src/ui/foundry_panel.rs` layout)
- Copy: the Gem-D and Gem-K portraits the owner chooses from upstream `apps/mk_ui/assets/portraits/` (55 MB including duplicates and third-party brand logos) into `assets/humans/portraits/`, downscaled to ≤1024 px
- Test: pure-state tests beside each module

**Interfaces:**
- Human detail view exposes the same sections upstream does (identity, body/vitals, needs, emotion, cognition/attention, memory, relationships, development/lifecycle, current room/position) from read-only state.
- The foundry panel's input fields are exactly `IslandCreateHumanRequest` (Phase 4 Task 6). Upstream panel fields with no request equivalent (e.g. neurotype, zodiac) are shown read-only as derived output from the created human, not as inputs. Submission sends `IslandCommand::CreateHuman` through the same command path as the dashboard.
- Portrait provenance: upstream's `CREDITS.md` says origin/licence was not independently re-verified. Carry that caveat into `assets/CREDITS.md`; the owner confirms provenance before any distribution outside this private repository.

- [ ] **Step 1:** Write tests: detail view for Gem-D/Gem-K matches profile values; a foundry-panel request produces exactly the same human as the identical dashboard request; derived fields are not editable.
- [ ] **Step 2:** Owner picks the two portraits; copy and downscale only those.
- [ ] **Step 3:** Port views, inspector and foundry; wire GLB models and portraits.
- [ ] **Step 4:** Run `cargo test -p mk_island_view` and `cargo test -p island_ui`; expect PASS.
- [ ] **Step 5:** Commit `feat(ui): port human views, inspector and foundry`.

### Task 4b: Computer service (opt-in)

**Files:**
- Copy: upstream `apps/computer-service` (Node.js) to `services/computer-service/`
- Modify: `.github/workflows/ci.yml` (Node job running `npm ci && npm test` in that directory)
- Modify: `assets/CREDITS.md`/`UPSTREAM.md` (provenance; upstream licence is "All rights reserved", same as this repository)

- [ ] **Step 1:** Import the service unchanged, keeping it opt-in via `COMPUTER_ACTIONS_ENABLED=1` exactly as upstream; `island serve` and `island-ui` attach `HttpComputerBridge::from_env()`.
- [ ] **Step 2:** Test that with the service disabled, replay/state hashes are unchanged and humans never attempt those actions; with it enabled, results enter only through the existing bridge and are excluded from the deterministic hash.
- [ ] **Step 3:** Commit `feat(island): import opt-in computer service`.

### Task 5: Inspectors and simulation controls

**Files:**
- Create: `apps/island_ui/src/inspectors.rs`
- Modify: `apps/island_ui/src/app.rs`
- Test: pure-state tests for inspector selection/projection.

**Interfaces:**
- Inspector exposes simulation clock, selected terrain cell, selected space/item/tree, human summary (including current space), property/building/item/network-account summary and physical/environmental values from `IslandView`.
- Controls send `ControlCommand::{Pause, Resume, Step(n), Snapshot}` (Phase 4 Task 4); no arbitrary state mutation through the renderer.

- [ ] **Step 1:** Write tests for selection lookup and inspector data coming only from current `IslandView`.
- [ ] **Step 2:** Implement egui panels and the control command path.
- [ ] **Step 3:** Run `cargo test -p island_ui`; expect PASS.
- [ ] **Step 4:** Commit `feat(ui): inspect regional world state`.

### Task 6: Repeatable performance baselines

**Files:**
- Create: `apps/island/src/bin/island_bench.rs`
- Create: `benchmarks/README.md`
- Create after measurement: `benchmarks/<date>-regional-baseline.json`
- Modify: `crates/mk_engine/src/world_integration.rs` (feature-gated construction counter)
- Modify: `crates/mk_engine/Cargo.toml` (feature `construction-counters`, off by default)
- Test: `crates/mk_engine/tests/island_allocation_bounds.rs`

**Interfaces:**
- `island_bench --scenario fixtures/island/default_scenario.json --ticks 1000 --dt 60 --json` emits `bootstrap_ms`, `serialized_state_bytes`, `ticks_per_second`, `save_ms`, `load_ms`, `snapshot_bytes`, `peak_rss_bytes` (Linux `VmHWM` from `/proc/self/status`; `null` elsewhere), `human_store_sync_ms`, and named per-major-subsystem elapsed milliseconds. All measured from the real regional paths; nothing estimated.

- [ ] **Step 1:** Add an allocation-bound test, run with `--features construction-counters`: `WorldState::new` (`world_integration.rs:462`, the planetary `GridSpec::new(32, 64)` bootstrap) increments a static counter; building and stepping `IslandWorldState` for 10 ticks leaves it at zero; default coarse/medium cell counts match the profile.
- [ ] **Step 2:** Implement benchmark instrumentation around the actual regional bootstrap/step/save/load/store-sync paths.
- [ ] **Step 3:** Run the benchmark with `--release` and save the JSON baseline.
- [ ] **Step 3a (resolution gate):** Default targets (owner to confirm): headless, at least one simulated day per real minute; with the UI open, at least real time (one simulated second per real second). If the 2 km / 12 km default misses them after profiling and fixing measured hotspots, switch the default profile to the 4 km / 24 km fallback (Phase 1), re-run every acceptance test, and record the measured trade-off in `docs/island/DEVIATIONS.md`.
- [ ] **Step 4:** Commit `perf(island): record regional baseline`.

### Task 7: Upstream sync, retiring `island_humans`, and planetary-only pruning

**Files:**
- Create: `scripts/sync_maerken_upstream.py` (Python 3, standard library only)
- Create: `scripts/tests/test_sync_maerken_upstream.py` (`python3 -m unittest`)
- Modify: `.github/workflows/ci.yml` (run the script tests)
- Modify: `UPSTREAM.md`
- Move: `CreateHumanRequest`, `create_human` and population code from `apps/island_humans` into `crates/mk_engine/src/regional/create_human.rs` (or `apps/island`), keeping its tests
- Remove: `apps/island_humans`
- Modify/delete: workspace/module entries proven unreachable from normal island execution

**Interfaces:**
- `python3 scripts/sync_maerken_upstream.py --check <Maer-Ken path>` reports changed imported paths against the pinned commit and never applies changes.
- `--apply <Maer-Ken path> --commit <sha>` copies only the documented import set, refuses on a dirty worktree, and leaves island divergences for normal review.

- [ ] **Step 1:** Write script tests using temporary git repositories for unchanged, changed and missing upstream paths.
- [ ] **Step 2:** Implement check/apply modes with explicit commit pin and clean-worktree guard.
- [ ] **Step 3:** Move the Phase 0b creator/population code out of `apps/island_humans` with its tests; point `apps/island` at the new location; then remove `apps/island_humans`.
- [ ] **Step 4:** Use `cargo tree`, source search and tests to identify planetary-only modules not reachable from `apps/island`/`apps/island_ui`; remove in small commits, running the fast tier after each cut.
- [ ] **Step 5:** Update README/docs to make `apps/island` and `apps/island_ui` the entry points.
- [ ] **Step 6:** Run fmt, clippy `-D warnings`, fast and slow tiers, the fixed-seed replay comparison and the release benchmark.
- [ ] **Step 7:** Update `UPSTREAM.md` with the final retained module/asset list and sync procedure.
- [ ] **Step 8:** Commit `refactor(island): prune planetary-only runtime`.

### Task 8: Final acceptance

**Files:**
- Create: `docs/ISLAND_SYSTEM_STATUS.md`
- Modify: `README.md`

**Interfaces:**
- Documentation records implemented systems, benchmark numbers, known model-fidelity limitations and exact verification commands.

- [ ] **Step 1:** Run every acceptance criterion from the approved design and record command/output summaries.
- [ ] **Step 2:** Verify fixed-seed land area/buffer/shape, all retained systems, founders/property, material budgets, snapshot/replay determinism, UI projection and absence of normal whole-planet allocation.
- [ ] **Step 3:** Write status/README from measured/tested results only; no planned feature may be described as implemented.
- [ ] **Step 4:** Run `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` and `cargo test --workspace --release -- --ignored slow_`; expect all green.
- [ ] **Step 5:** Commit `docs(island): record completed regional build`.
