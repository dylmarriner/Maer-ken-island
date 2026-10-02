# Island Life, Property & Humans Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Populate the regional island with retained Maer-Ken habitats, vegetation, resources, canonical property/equipment laid out in real metric space, individually simulated trees around the estate, and the full Gem-D/Gem-K human runtime moving on the island.

**Architecture:** Derive regional biome/ecology/resource state from the Phase-2 physical world, then instantiate property and humans through the already imported canonical engine types. Add regional adapters for placement, observation and interaction. Human cognition, biology and behaviour code is not forked; the one engine change is geometry — the human and organism runtimes currently assume the planetary grid (spherical cell sizes, east/west wrap, lat/lon from grid rows), so Task 5 threads a topology abstraction through them that reproduces upstream exactly on the planet and uses flat, non-wrapping geometry on the island.

Two representation levels exist:

- **Regional (medium cells, 8 km):** habitat, NPP, a derived standing-biomass density and resources as per-cell fields; species populations as upstream. Upstream has no per-cell biomass stock — living biomass carbon is `Σ species population × body mass` (`conservation.rs:255-285`) — so the per-cell biomass field here is a *derived diagnostic* used to seed and scale the patch, not a ledger stock. Upstream `VegetationSystem::seed_from_biomes` materializes about one plant per 11 cells; that remains a sparse view, not the forest.
- **High-detail estate patch:** 4 km × 4 km at 5 m cells (800 × 800), centred on the estate's medium cell centre, so it covers 25% of exactly one 8 km cell. Buildings, rooms, doors and items have metric positions; trees within 1 km of the estate centre are individual stems, the rest of the patch is stand-level cover. Upstream property stores the whole estate as one grid cell with room *labels* only; the layout adds geometry without copying the inventory.

**Tech Stack:** Rust 2021, existing `mk_engine::biosphere`, `organisms`, `resource_economy`, `humans`, `perception`, `mk_island` scenario/profile types.

**Spec:** `docs/superpowers/specs/2026-10-01-maer-ken-island-regional-world-design.md`

**Depends on:** Phase 2 (`RegionalPhysicalState` including `insolation`).

## Global Constraints

- Gem-D/Gem-K constructors and human cognition/biology modules remain sourced from the pinned Maer-Ken runtime; geometry changes (Task 5) must leave every existing upstream test result unchanged.
- The founders' estate retains every building upstream defines — House, Shed, Workshop, Armoury, ComputerRoom, Garage (`PropertyBuildingKind`) — plus rooms, named vehicles, attachments, tools, furniture, computers and network accounts.
- Only the founders' estate (the `StarterProperty` whose `owner_agent_ids` contains `"Gem-D"`) is laid out and placed. The unowned homestead template that `PropertySystem::new` always includes stays unplaced. Placements are keyed by `(property_id, item_id)` because item ids restart at 1 per property.
- Only regional island/ocean cells may seed living populations or resources.
- Computer access requires the upstream gate (a ComputerRoom building and a matching `NetworkAccount`) *and*, on the island, that the human's current estate space is inside the ComputerRoom building (Task 7).
- Every founders'-estate `PropertyItem` appears exactly once in the layout; the layout derives from `PropertySystem` and never defines items of its own.
- At seed time, patch tree + stand biomass equals the estate cell's derived biomass × 0.25 within ±2%, with the tree cap active.
- Wood gathering stays an economy action as upstream; it does not debit species biomass carbon (upstream does not couple them either). This limitation is recorded in `UPSTREAM.md`, not silently "fixed".

## Review Focus

- All-ocean or no-valid-estate scenarios must fail clearly instead of inventing a property location.
- Founder placement must be on dry, non-dominantly-volcanic, buildable land.
- Aquatic cells must not receive terrestrial resource nodes/vegetation.
- Re-seeding vegetation/ecology must be idempotent for already initialized state.
- Building footprints must not overlap each other, the patch edge, water or slopes above the buildable limit; trees must not stand inside footprints or the cleared yard.
- Every room/zone must be reachable from outdoors through the door graph.
- Planet-topology results must be bit-identical before and after Task 5.

---

### Task 1: Regional biome, habitat, NPP and vegetation initialization

**Files:**
- Create: `crates/mk_engine/src/regional/ecology.rs`
- Modify: `crates/mk_engine/src/biosphere/mod.rs`, `crates/mk_engine/src/biosphere/habitat.rs` (extract geometry inputs; see Step 3)
- Modify: `crates/mk_engine/src/regional/mod.rs`
- Test: `crates/mk_engine/tests/regional_ecology.rs`

**Interfaces:**
- Produces: `RegionalEcologyState { biome_grid: Grid2<BiomeType>, biosphere: BiosphereState, organisms, vegetation: VegetationSystem, biomass_kgc_m2: Grid2<f64> }` on Medium.
- Produces: `bootstrap_regional_ecology(canon: &Arc<CanonLocked>, physical: &RegionalPhysicalState, domain: &IslandDomain, seed: [u8; 32]) -> Result<RegionalEcologyState, RegionalEcologyError>` and `step_regional_ecology(&mut self, …, dt_seconds)`.
- `biomass_kgc_m2 = npp_kgc_m2_yr × residence_years(biome)`, with a per-biome residence-time table in code citing its source for each value.

- [ ] **Step 1:** Write tests asserting ocean/land biome consistency, at least one tree/shrub/grass on eligible land, no terrestrial plant in ocean cells, NPP higher in warm wet lowland than cold alpine cells, `biomass_kgc_m2` zero over ocean, and deterministic bootstrap.
- [ ] **Step 2:** Run `cargo test -p mk_engine --test regional_ecology`; expect FAIL.
- [ ] **Step 3:** Upstream `update_primary_production` derives latitude as `(row+0.5)/nlat·π−π/2` and uses planet radius and `dlon` (`biosphere/mod.rs:628-660`); `habitat_cells` takes `&WorldState` (`habitat.rs:55`). Extract the latitude/area inputs as parameters (global path filled exactly as today; regional path from `domain.latitude_rad_for_row` and flat area) and add a habitat function over explicit grids. Reuse biome classification and biosphere/vegetation systems against regional climatology/hydrology/elevation resampled to Medium.
- [ ] **Step 4:** Run `cargo test -p mk_engine --test regional_ecology` and `cargo test -p mk_engine --lib biosphere:: organisms::vegetation`; expect PASS.
- [ ] **Step 5:** Commit `feat(engine): seed island ecology`.

### Task 2: Regional resources and material economy

**Files:**
- Create: `crates/mk_engine/src/regional/resources.rs`
- Test: `crates/mk_engine/tests/regional_resources.rs`

**Interfaces:**
- Produces: `bootstrap_regional_resources(ecology: &RegionalEcologyState, physical: &RegionalPhysicalState, domain: &IslandDomain) -> ResourceEconomyState` calling upstream `ResourceEconomyState::from_terrain(&biome_grid, &volcanic_area_fraction_medium)`.
- Existing `ResourceEconomyState::{apply_human_action, regenerate}` remains the action API. `regenerate()` takes no `dt` and applies one regeneration per call (`resource_economy.rs:238`), so regrowth rate is tied to the scheduler cadence (21,600 s by default, Phase 4). Record that coupling; changing the cadence changes regrowth.
- Buildability: upstream `is_buildable` allows at most `MAX_CLIMB_HEIGHT_M = 2.0` between neighbouring cells (`physics.rs:34, 76`), which almost no 8 km cell passes. Inside the estate patch, buildability uses patch terrain at 5 m; outside it, `regional_is_buildable` applies the same 2 m rise over the 5 m patch spacing as a slope limit (0.4) to medium-cell slope. Record as a divergence.

- [ ] **Step 1:** Write tests asserting no nodes in aquatic cells, forest tree/food nodes, volcanic ore contribution, finite non-renewable deposits, deterministic IDs/order, and that a gentle lowland medium cell is buildable while a steep alpine one is not.
- [ ] **Step 2:** Run `cargo test -p mk_engine --test regional_resources`; expect FAIL.
- [ ] **Step 3:** Implement with only regional validation/placement glue around `from_terrain`, plus `regional_is_buildable`.
- [ ] **Step 4:** Run `cargo test -p mk_engine --test regional_resources` and `cargo test -p mk_engine --lib resource_economy::`; expect PASS.
- [ ] **Step 5:** Commit `feat(engine): bind resources to island terrain`.

### Task 3: Island scenario and canonical estate placement

**Files:**
- Create: `crates/mk_island/src/scenario.rs`
- Modify: `crates/mk_island/src/lib.rs` (export `scenario`)
- Create: `fixtures/island/default_scenario.json`
- Create: `crates/mk_engine/src/regional/property.rs`
- Test: `crates/mk_island/tests/scenario.rs`, `crates/mk_engine/tests/island_property_scenario.rs`

**Interfaces:**
- Produces: `IslandCadenceProfile { human_seconds: u64, weather_ocean_seconds: u64, hydrology_ecology_resource_seconds: u64, geophysics_seconds: u64, human_store_seconds: u64 }` with defaults `60`, `3_600`, `21_600`, `86_400`, `3_600`; `validate()` rejects zero values and any cadence that is not a multiple of `human_seconds`.
- Produces: `EstatePatchConfig { extent_m: f64, cell_size_m: f64, individual_tree_radius_m: f64, tree_cap: usize }` with defaults `4_000.0`, `5.0`, `1_000.0`, `200_000`.
- Produces: `IslandScenario { version: u32, seed: [u8; 32], profile: IslandProfile, cadences: IslandCadenceProfile, estate_patch: EstatePatchConfig, founders_enabled: bool, estate_enabled: bool }` (serde) and `IslandScenario::load(path: &Path) -> Result<Self, IslandScenarioError>`; the default fixture embeds `fixtures/island/default_profile.json`'s values and seed.
- Produces: `choose_regional_estate_location(canon: &CanonLocked, physical: &RegionalPhysicalState, ecology: &RegionalEcologyState, domain: &IslandDomain) -> Option<(usize, usize)>` (Medium cell; scoring follows upstream's year-round-climate placement, which uses `canon.obliquity_deg`), `estate_patch_spec(domain, cell, &EstatePatchConfig) -> Result<LocalPatchSpec, …>` (centred on `cell_center_m(Medium, cell)`, must fit inside that cell), and `bootstrap_regional_property(location) -> PropertySystem` = upstream `PropertySystem::new(Some(location))`.

- [ ] **Step 1:** Write scenario tests: defaults, JSON round-trip, unknown version rejected, invalid cadences rejected. Write property tests: deterministic estate placement on dry buildable land; patch inside one medium cell; founders' estate has all six `PropertyBuildingKind`s, Gem-D's and Gem-K's bedrooms, named vehicles, all shed tool categories, computers and two administrator network accounts; an all-ocean physical state returns `None`.
- [ ] **Step 2:** Run `cargo test -p mk_island --test scenario` and `cargo test -p mk_engine --test island_property_scenario`; expect FAIL.
- [ ] **Step 3:** Implement scoring from real physical/climatology state; instantiate upstream `PropertySystem::new(Some(location))` without copying inventory definitions.
- [ ] **Step 4:** Run both tests and `cargo test -p mk_engine --lib organisms::property`; expect PASS.
- [ ] **Step 5:** Commit `feat(island): place canonical founders estate`.

### Task 4: Metric estate layout and navigation

**Files:**
- Create: `crates/mk_engine/src/regional/estate_layout.rs`
- Modify: `crates/mk_engine/src/regional/property.rs`
- Test: `crates/mk_engine/tests/island_estate_layout.rs`

**Interfaces:**
- Produces: `EstateLayout { property_id, patch: LocalPatchSpec, terrain_m: Grid2<f64>, buildings: Vec<BuildingFootprint>, spaces: Vec<SpaceLayout>, doors: Vec<Door>, items: Vec<ItemPlacement>, yard: Rect }`.
- `BuildingFootprint { building_id, kind: PropertyBuildingKind, rect_m, rotation_deg }`.
- `SpaceLayout { id: SpaceId, building_id, label, kind: SpaceKind, rect_m }` where `SpaceKind::{Room, Zone, WholeBuilding}`. Mapping from upstream labels: House items with `room` = `"Kitchen"`, `"Lounge"`, `"Bathroom"`, `"<name>'s Bedroom"` → `Room`; House `"General"` → `Zone` (hall); Shed items' tool categories (`"Construction Tools"`, `"Digging Tools"`, `"Repair Tools"`, `"Exploration Tools"`, `"Smithing Tools"`) → `Zone`s inside the Shed; Workshop, Armoury, ComputerRoom and Garage items (`room: None`) → one `WholeBuilding` space each.
- `Door { a: Space, b: Space, position_m }` where `Space::{Outdoors, Inside(SpaceId)}`; `ItemPlacement { property_id, item_id, space, position_m }` (vehicles in the Garage or yard; computers in the ComputerRoom).
- Produces: `layout_estate(property: &StarterProperty, physical: &RegionalPhysicalState, domain: &IslandDomain, patch: &LocalPatchSpec, seed: [u8; 32]) -> Result<EstateLayout, EstateLayoutError>`, `EstateLayout::space_at(position_m) -> Space`, `EstateLayout::route(from: Space, to: Space) -> Option<Vec<Space>>`.
- Patch terrain is the medium-cell elevation bilinearly interpolated plus bounded deterministic detail; it never changes the regional elevation.

- [ ] **Step 1:** Write tests: every founders'-estate item placed exactly once in the space its label/building maps to; homestead items not placed; no footprint overlaps; footprints on dry land under the slope limit; every space routable from `Outdoors`; `space_at` round-trips each placement; identical bytes for identical inputs; a patch with no buildable site returns `EstateLayoutError::NoBuildableSite`.
- [ ] **Step 2:** Run `cargo test -p mk_engine --test island_estate_layout`; expect FAIL.
- [ ] **Step 3:** Implement deterministic site selection inside the patch, rectangle packing per building kind sized from its space/item count, space subdivision and door placement.
- [ ] **Step 4:** Run the test and `cargo test -p mk_engine --lib organisms::property`; expect PASS.
- [ ] **Step 5:** Commit `feat(engine): lay out the founders estate in metres`.

### Task 5: Grid topology for the human and organism runtimes

**Files:**
- Create: `crates/mk_engine/src/topology.rs`
- Modify: `crates/mk_engine/src/humans/mod.rs` (`HumanSystem::step`, `seed_runtime_position_from_birthplace`, `cell_crossing_probability`, `step_toward`, `step_toward_nearest`, `someone_within_reach`, `step_by`, `grid_position_to_birthplace`, `birthplace_to_grid_position`)
- Modify: `crates/mk_engine/src/humans/lifecycle.rs` (`deliver_due_births`, `deliver_birth`)
- Modify: `crates/mk_engine/src/perception.rs` (`Occupancy::social_density`)
- Modify: `crates/mk_engine/src/organisms/runtime.rs` (wrapping neighbour steps)
- Modify: `crates/mk_engine/src/world_integration.rs` (pass `GridTopology::planetary`)
- Modify: `crates/mk_engine/src/regional/mod.rs`
- Test: unit tests in `topology.rs`; existing human/organism tests

**Interfaces:**
- Produces: `GridTopology { rows, cols, wraps_east_west: bool, … }` with `cell_area_m2(row)`, `cell_width_m(row)`, `cell_height_m()`, `neighbour(row, col, drow, dcol) -> Option<(usize, usize)>`, `col_delta(from, to) -> i32` (shortest, wrap-aware only when wrapping), `lat_lon(row, col) -> (f64, f64)`, `cell_for_lat_lon(lat, lon) -> Option<(usize, usize)>`.
- `GridTopology::planetary(grid_spec: &GridSpec, planet_radius_m: f64)` reproduces today's behaviour exactly (spherical area via `cell_area_at_row_m2`, `rem_euclid` wrap, lat/lon from `GridSpec`).
- `GridTopology::regional(domain: &IslandDomain, level: DomainLevel)` uses flat cells, no wrap, `domain.latitude_rad_for_row`/`longitude_rad_for_col`; `cell_for_lat_lon` returns `None` outside the domain.
- Every listed function takes `&GridTopology` where it took `&GridSpec`. A child born on the island gets a birthplace from `topology.lat_lon` of the mother's cell.

- [ ] **Step 1:** Run `cargo test -p mk_engine --lib humans:: perception:: organisms::` and `cargo test -p mk_engine --test humans_world_integration`; record results and the `state_hash` of a 10-step default `WorldState`.
- [ ] **Step 2:** Write topology tests: planetary area/neighbours/lat-lon equal the current functions for every row of a 32 × 64 grid; regional west-edge cells have no east-edge neighbours; regional `lat_lon` ↔ `cell_for_lat_lon` round-trips.
- [ ] **Step 3:** Introduce `GridTopology` and thread it through the listed functions; `WorldState` passes `GridTopology::planetary`.
- [ ] **Step 4:** Re-run Step 1; expect identical results and identical 10-step hash.
- [ ] **Step 5:** Record in `UPSTREAM.md` as a geometry-only divergence suitable for upstreaming. Commit `refactor(engine): grid topology for human and organism movement`.

### Task 6: Dense vegetation in the estate patch

**Files:**
- Create: `crates/mk_engine/src/regional/local_vegetation.rs`
- Test: `crates/mk_engine/tests/island_local_vegetation.rs`

**Interfaces:**
- Produces: `LocalVegetationPatch { trees: Vec<TreeInstance>, stands: Grid2<StandCover>, individual_radius_m: f64, cap: usize }`, `TreeInstance { id, kind: PlantKind, position_m, height_m, stem_diameter_m, biomass_kgc, alive }`, `StandCover { biome, biomass_kgc, stem_density_per_ha }` per 5 m cell outside the individual radius (and under footprints/yard: zero).
- Individual stems are trees with stem diameter ≥ 0.10 m within `individual_radius_m` of the estate centre. If seeding would exceed `cap`, the radius shrinks deterministically (halving the excess area per iteration) until it fits; the displaced biomass stays in `stands`, so totals are unchanged.
- Produces: `seed_local_vegetation(ecology: &RegionalEcologyState, layout: &EstateLayout, domain: &IslandDomain, config: &EstatePatchConfig, seed: [u8; 32]) -> Result<LocalVegetationPatch, LocalVegetationError>` and `step_local_vegetation(&mut self, ecology: &RegionalEcologyState, dt_seconds: u64)` (growth/mortality scaled to the estate cell's NPP).
- Produces: `fell_tree(&mut self, tree_id, economy: &mut ResourceEconomyState) -> Result<WoodYield, LocalVegetationError>` — the stem is removed, wood enters the economy through the same item kinds a Tree node yields. Species biomass is not debited (Global Constraints).

- [ ] **Step 1:** Write tests: stem density follows biome/biomass (forest ≫ grassland, none in water); trees + stands sum to the cell's derived biomass × 0.25 within ±2% with the cap both inactive and forced active (cap = 1,000); no tree in footprints or yard; count never exceeds `cap`; felling removes the stem and adds wood; deterministic bytes.
- [ ] **Step 2:** Run `cargo test -p mk_engine --test island_local_vegetation`; expect FAIL.
- [ ] **Step 3:** Implement deterministic Poisson-disc placement with spacing from biome/kind, size distribution from biomass, stands for the remainder, and growth/mortality tied to NPP.
- [ ] **Step 4:** Run the test and `cargo test -p mk_engine --lib organisms::vegetation`; expect PASS.
- [ ] **Step 5:** Commit `feat(engine): simulate individual trees around the estate`.

### Task 7: Founders on the island, observation and interaction

**Files:**
- Create: `crates/mk_engine/src/humans/observation.rs` (extracted)
- Modify: `crates/mk_engine/src/world_integration.rs` (`step_humans_period` uses the extracted builder)
- Create: `crates/mk_engine/src/regional/humans.rs`
- Modify: `apps/island_humans/src/lib.rs` (delegate to the adapter where it overlaps; the app is retired in Phase 5)
- Test: `crates/mk_engine/tests/island_human_runtime.rs`

**Interfaces:**
- Produces: `build_observation(inputs: &ObservationInputs<'_>, position: &GridPosition, agent_id: &str) -> AgentWorldObservation`, extracted unchanged from the closure in private `WorldState::step_humans_period` (`world_integration.rs:2066-2150`). `ObservationInputs` borrows the climate, weather, insolation (`daylight_fraction`), hydrology, biome, property, economy and occupancy data the closure reads today.
- Produces: `HumanEstatePositions(BTreeMap<String, EstatePosition { space: Space, position_m: (f64, f64) }>)` — serialized side-table for humans inside the patch; humans outside the patch have no entry.
- Produces: `bootstrap_regional_humans(property: &PropertySystem, layout: &EstateLayout, topology: &GridTopology) -> Result<(HumanSystem, HumanEstatePositions), RegionalHumanError>`; founders start in their own bedrooms.
- Produces: `step_regional_humans(humans: &mut HumanSystem, positions: &mut HumanEstatePositions, economy: &mut ResourceEconomyState, property: &PropertySystem, layout: &EstateLayout, vegetation: &mut LocalVegetationPatch, physical: &RegionalPhysicalState, ecology: &RegionalEcologyState, topology: &GridTopology, tick: Tick, dt_seconds: u64, rng: &RngRegistry) -> Result<(), RegionalHumanError>` — calls upstream `HumanSystem::step` with the regional topology and `build_observation`; movement inside the patch follows `EstateLayout::route`.
- Computer access (island divergence): `computer_access` is the upstream gate (ComputerRoom on the cell and a matching `NetworkAccount`, `world_integration.rs:2130-2146`) AND `positions[agent].space` is inside the ComputerRoom building. The network-account gate itself is unchanged.

- [ ] **Step 1:** Run `cargo test -p mk_engine --test humans_world_integration` and record results; extract `build_observation`; re-run; expect identical results.
- [ ] **Step 2:** Write tests: exactly Gem-D/Gem-K by default, each starting in their own bedroom; IDs/profiles match canonical constructors; a founder routed to the ComputerRoom gains access, a founder in the Kitchen does not, and a non-account human in the ComputerRoom remains denied; a founder felling a patch tree adds wood; a founder walking out of the patch loses their `HumanEstatePositions` entry and moves on medium cells without wrapping.
- [ ] **Step 3:** Run `cargo test -p mk_engine --test island_human_runtime`; expect FAIL.
- [ ] **Step 4:** Implement the adapter without changing human cognition/body modules.
- [ ] **Step 5:** Run the test, `cargo test -p mk_core human`, and `cargo test -p mk_engine --lib humans::`; expect 0 failures.
- [ ] **Step 6:** Record the observation extraction and room-level computer access in `UPSTREAM.md`. Commit `feat(engine): run canonical humans on island`.

### Task 8: Phase-3 integrated life/property acceptance

**Files:**
- Create: `crates/mk_engine/tests/island_life_acceptance.rs`
- Modify: `UPSTREAM.md`

**Interfaces:**
- Consumes all Phase-3 APIs.

- [ ] **Step 1:** Bootstrap fixed-seed physical + ecology + resources + property + layout + local vegetation + humans, advance one simulated week at 60 s human steps (hourly physical, 6-hourly ecology — call the Phase-2/3 step functions directly; the scheduler arrives in Phase 4), and assert living vegetation (regional and patch), resources, both founders inside the layout, intact estate inventory, economy events and deterministic final serialization. Hash a canonical form (serialize to `serde_json::Value`, whose maps are sorted, then to bytes): upstream state such as `AdvancedMemorySnapshot` holds `HashMap`s (`advanced_memory.rs:70-85`) whose iteration order is not stable across processes. Slow tier.
- [ ] **Step 2:** Run the acceptance test twice in separate processes; expect identical blake3 digests and PASS.
- [ ] **Step 3:** Update provenance for scenario/adapter files; run fmt, clippy `-D warnings` and Phase-3 focused tests.
- [ ] **Step 4:** Commit `test(island): lock life property human acceptance`.
- [ ] **Step 5:** Add `island_preview life` writing `biomes.png`, `resources.png`, `estate_plan.png` (footprints, spaces, doors, item markers, founder positions) and `estate_trees.png`, with the Phase-1 determinism test pattern; commit the default preview under `docs/previews/phase3/`.
- [ ] **Step 6 (Gate 3 review):** The owner checks the estate plan and tree map before Phase 4.
