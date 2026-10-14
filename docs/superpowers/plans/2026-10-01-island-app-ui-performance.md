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
- [ ] **Step 2:** Implement chunked terrain heightfield + surrounding ocean surface and a free/orbit-over-map camera, explicitly not a planetary sphere. Lighting follows the island canon: sun position from the 36-hour day, 27° tilt and 288-day year; both moons rendered with their phases; clouds and rain from the synoptic weather.
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
  Status (2026-10-14): partly done. `apps/island_ui/src/stems.rs` draws the patch's individual stems within 250 m of the yard (`/api/trees`, up to 3,000, spread evenly by the server, re-read every 30 s) as trunks at their own height and stem diameter; crowns are not drawn because nothing served gives their size. The wood's floor under the trunks is textured with the island's own picture of the patch's `StandCover` (`/api/patch.png`, one pixel per 5 m cell, re-read every 2 minutes; `stems::stand_uv`), so the stand that is not individual trees shows as what it is rather than one flat green. Not done: `StandCover` as instanced trees (the island gives a stand a stem density but no stem heights or crowns, so an instanced tree would be an invented size), and instanced trees beyond the patch from `biomass_kgc_m2`.
- [ ] **Step 2b:** Buildings and interiors: generate meshes from `EstateLayout` footprints/spaces/doors and place items at their `ItemPlacement` positions; an interior view shows rooms with their furniture, tools, vehicles and computers.
  Status (2026-10-14): partly done. The served layout now carries the island's doors (`mk_island_api::Doorway`: the two spaces by label, `None` outdoors, and a position), and the estate view outlines every `Room` on its floor and marks each door as an upright line to its building's eaves (`estate::room_outlines`, `estate::door_marks`, Bevy gizmos); a picked room lists where its doors lead. Lines, not solid walls, because the island models a door as a point in its graph with no width or height, and gives no wall thickness; cutting door gaps would mean inventing one. Items already stand at their placements. Not done: a separate interior camera.
- [ ] **Step 2d:** Industry and town (Phase 4b): the river and its weirs, penstocks and powerhouses; power lines; oil wells, flowlines, tanks, refinery and gas plant; the town's houses, supermarket and other buildings from their layouts; farm fields and livestock, the fishing boat and wharf; forestry blocks (cut, regrowing, standing), log trucks, the sawmill and houses under construction; vehicles at the depot; live overlays for river flow, power flow and plant status.
- [ ] **Step 2c:** Resources: every `ResourceNodeKind` has a visual (outcrops by lithology, mine workings on deposits, river gravel with placers, ironsand beaches, game, fisheries, bee colonies, salt pans) and every catalogue item has an inventory icon; a test enumerates both lists and fails on any missing entry.
- [ ] **Step 3:** Write tests covering all six property building kinds (the computer room rendered as a room inside the House, not a separate building), every space in the layout, every current named vehicle class, founder model aliases, instanced-tree density following biomass, and deterministic offsets.
  Status (2026-10-14): covered except the trees. Building kinds and item kinds (`property.rs` `every_building_kind_and_item_kind_has_an_answer`), the computer room as a room in the House (`estate.rs`), every space in the served layout as one floor inside a building and every thing on the patch (`tests/reading_an_island.rs`), every named estate vehicle (`property.rs` `every_vehicle_on_the_estate_is_drawn_as_itself_or_as_its_size`), founder model aliases (`a_created_human_is_marked_as_wearing_somebody_elses_face`). Instanced-tree density waits on the rest of Step 2a (only individual stems near the yard are drawn); the renderer applies no visual offsets, so there is nothing to pin.
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

Task 4 status (2026-10-14): the person inspector is done without the portraits. Choosing somebody in the desktop application reads `/api/world/humans/{id}` off the frame thread, every 10 s while they stay chosen, and shows the backend's own sections as collapsible groups with meters (`apps/island_ui/src/human_inspector.rs`, pure and tested; the web dashboard renders the same sections): identity, body, vital signs, needs, temperament, feeling, attention and thinking, **what they are doing** (new: this hour's thought from `humans::thought::generate_thought`, the chosen action, whether the last choice came off, what they have learned it is worth, reward so far, choices remembered; upstream's autonomy telemetry), other people, growing up, their record and **family** (new: parents and children by the population's birth records). The record's age is said when it lags the island. Rather than porting upstream's `HumanDetailView` against the engine types, the inspector reads the wire sections, so the desktop and the web cannot disagree about a person. Not done: the foundry panel, and the portraits (owner's pick).

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
- Controls send `ControlCommand::{Pause, Resume, Step(n), Snapshot, SetSpeed}` (Phase 4 Task 4): a speed selector (real time, 10×, 60×, 360×, 1,440×, max) showing both requested and achieved speed, plus the local date and time (36-hour day, 288-day year) and the equivalent elapsed Earth time; no arbitrary state mutation through the renderer.

- [x] **Step 1:** Write tests for selection lookup and inspector data coming only from current `IslandView`.
- [x] **Step 2:** Implement egui panels and the control command path.
- [x] **Step 3:** Run `cargo test -p island_ui`; expect PASS.
- [ ] **Step 4:** Commit `feat(ui): inspect regional world state`.

Task 5 status (2026-10-14): `apps/island_ui/src/inspectors.rs` (pure, tested) and the left panel. The clock reads as the local calendar (year, day of the 288-day year, time on the 36-hour day; the backend's clock now carries `year_length_days`) and as Earth time elapsed; the speed selector offers real, 10x, 60x, 360x, 1,440x and max with the achieved speed beside the requested one; Pause, Resume, Step and Save send `ControlRequest`s and nothing else changes state from the renderer. The terrain inspector is the cell under the middle of the view (the estate's own cell in the estate frame), fetched off the frame thread from `/api/cell`, which now also gives the cell's biome, NPP and climatology mean temperature and rain. Picking (2026-10-14): in the estate view a click (a press and release without a drag, not on a panel) casts a ray from the camera and picks the smallest thing under it -- an item before a room, a room before a building, because the buildings are drawn translucent -- and the right panel describes it from the served layout: kind, room, position, real size and the size's or height's source (`estate.rs` `ray_hits`, `pick`, `describe`, tested against turned boxes and the layout). Not done: picking a tree, which waits on the stems being drawn (Task 3 Step 2a).

### Task 6: Repeatable performance baselines

**Files:**
- Create: `apps/island/src/bin/island_bench.rs`
- Create: `benchmarks/README.md`
- Create after measurement: `benchmarks/<date>-regional-baseline.json`
- Modify: `crates/mk_engine/src/world_integration.rs` (feature-gated construction counter)
- Modify: `crates/mk_engine/Cargo.toml` (feature `construction-counters`, off by default)
- Test: `crates/mk_engine/tests/island_allocation_bounds.rs`

**Interfaces:**
- `island_bench --scenario fixtures/island/default_scenario.json --ticks 1000 --dt 60 --json` emits `bootstrap_ms`, `serialized_state_bytes`, `ticks_per_second`, `save_ms`, `load_ms`, `snapshot_bytes`, `peak_rss_bytes` (Linux `VmHWM` from `/proc/self/status`; `null` elsewhere), `human_store_sync_ms`, `threads` and the speed-up from 1 thread to all cores, `spin_up_1000y_seconds`, and named per-major-subsystem elapsed milliseconds. All measured from the real regional paths; nothing estimated.

- [ ] **Step 1:** Add an allocation-bound test, run with `--features construction-counters`: `WorldState::new` (`world_integration.rs:462`, the planetary `GridSpec::new(32, 64)` bootstrap) increments a static counter; building and stepping `IslandWorldState` for 10 ticks leaves it at zero; default coarse/medium cell counts match the profile.
- [ ] **Step 2:** Implement benchmark instrumentation around the actual regional bootstrap/step/save/load/store-sync paths.
- [ ] **Step 3:** Run the benchmark with `--release` and save the JSON baseline.
- [ ] **Step 3b (dead compute):** measure `formal_predictive_processing::step()`'s share of human step time; upstream documents that it runs every tick and its `selected_action` is read nowhere but its own tests (`docs/canon/HUMAN_SYSTEM_STATUS.md`, Known gaps). Owner decides: wire its output into decisions upstream, or stop running it. Also prune any planetary-only module still stepped by the island (Task 7).
- [ ] **Step 3a (resolution gate):** Default targets (owner to confirm): headless, at least one simulated day per real minute; with the UI open, at least real time (one simulated second per real second). If the 2 km / 12 km default misses them after profiling and fixing measured hotspots, switch the default profile to the 4 km / 24 km fallback (Phase 1), re-run every acceptance test, and record the measured trade-off in `docs/island/DEVIATIONS.md`.
Task 6 status (partly done): `apps/island/src/bin/island_bench.rs` (JSON: bootstrap, ticks per second, simulated days per real minute, per-subsystem ms from runtime-only `SubsystemTimings` in `IslandLife`, snapshot size, save, load and hash round trip, human-store sync, peak RSS, threads and the one-thread against all-cores speedup) and `benchmarks/README.md` and `benchmarks/2026-10-08-regional-baseline.json` are in (Steps 2-3 and 3a: the default 2 km / 12 km profile makes 12.9 simulated days per real minute headless, well over the target of one, so no fallback profile is needed; with the UI open the live server ran flat out at about 30,000x with three people). Step 1 is done: `mk_engine` feature `construction-counters` (off by default) counts planetary `WorldState` builds; `tests/island_allocation_bounds.rs` (run with `--features construction-counters`) shows building and stepping the island 10 ticks leaves the counter at 0, the grids are 160 x 200 coarse and 960 x 1200 medium, and a planetary build does count (positive control; it needs upstream's own canon, the island's is rejected by `WorldState::new`). **Not done:** Step 3b is measured, no decision needed: `formal_predictive_processing::step()` is **0.9% of the human step** (402 people, 120 steps; `tests/island_dead_compute.rs`, `--features construction-counters`, timing counter `FORMAL_PREDICTIVE_NANOS`), so leaving it running costs nothing worth saving. `spin_up_1000y_seconds` is now measured too (2026-10-14: 10.8 s on 8 threads, Phase 4 Task 2b).
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

- [x] **Step 1:** Write script tests using temporary git repositories for unchanged, changed and missing upstream paths.
- [x] **Step 2:** Implement check/apply modes with explicit commit pin and clean-worktree guard.
- [x] **Step 3:** Move the Phase 0b creator/population code out of `apps/island_humans` with its tests; point `apps/island` at the new location; then remove `apps/island_humans`.
Task 7 status (2026-10-14): Steps 1-2 are done: `scripts/sync_maerken_upstream.py` with the file-by-file import manifest `scripts/upstream_imports.json` (built by matching every local file to the upstream trees) and eight tests on throwaway repositories, run in CI's `repository scripts` job; `UPSTREAM.md` has the procedure and the drift against upstream master. Step 4's inventory is done (`docs/island/PRUNING_INVENTORY.md`): 9 of 35 modules (8.5% of the lines) are unreachable from the apps, all tooling, with no runtime or memory benefit; **the recommendation is not to prune** (it only makes the upstream sync harder). Pruning is therefore an owner decision, not outstanding work.
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
