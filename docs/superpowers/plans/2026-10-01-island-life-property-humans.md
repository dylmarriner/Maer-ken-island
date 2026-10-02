# Island Life, Property & Humans Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Populate the regional island with retained Maer-Ken habitats, vegetation, resources, canonical property/equipment laid out in real metric space, individually simulated trees around the estate, and the full Gem-D/Gem-K human runtime.

**Architecture:** Derive regional biome/ecology/resource state from the Phase-2 physical world, then instantiate property and humans through the already imported canonical engine types. Add only regional adapters for placement, observation and interaction; do not duplicate human internals or property inventories.

Two representation levels exist and must agree:

- **Regional (medium cells, 8 km):** biomass, habitat and resources as aggregates — the authoritative ledger quantities. Upstream `VegetationSystem::seed_from_biomes` materializes only about one plant per 11 cells; that remains a sparse view, not the forest.
- **High-detail estate patch (default 4 km × 4 km, 5 m cells, `LocalPatchSpec` from Phase 1):** buildings, rooms, doors and items with metric positions, and individual trees. Upstream property stores the whole estate as one grid cell with room *labels* only; the layout here adds geometry without copying the inventory. Patch quantities are a disaggregation of the cell aggregates and must sum back to them.

**Tech Stack:** Rust 2021, existing `mk_engine::biosphere`, `organisms`, `resource_economy`, `humans`, `mk_island` scenario/profile types.

**Spec:** `docs/superpowers/specs/2026-10-01-maer-ken-island-regional-world-design.md`

## Global Constraints

- Gem-D/Gem-K constructors and human modules remain sourced from the pinned Maer-Ken runtime.
- The founders' estate must retain house, shed, workshop, armoury, computer room, garage, rooms, named vehicles, attachments, tools, furniture, computers and network accounts already defined upstream.
- Only regional island/ocean cells may seed living populations or resources.
- Computer access remains gated by physical property location + `NetworkAccount`; no global access flag.
- Resource gathering/building uses actual regional `GridPosition` and terrain state.
- Every `PropertyItem` appears exactly once in the estate layout; the layout derives from `PropertySystem` and never defines items of its own.
- Patch tree biomass equals the containing cell's biomass × patch-area share within a documented tolerance; felling a patch tree debits the regional ledger.
- Tree count in the patch is bounded by a configurable cap (default 200,000) so the patch cannot grow memory without limit.

## Review Focus

- All-ocean or no-valid-estate scenarios must fail clearly instead of inventing a property location.
- Founder placement must be on dry, non-dominantly-volcanic, buildable land.
- Aquatic cells must not receive terrestrial resource nodes/vegetation.
- Re-seeding vegetation/ecology must be idempotent for already initialized state.
- Human perception/action must not access property or computers from a different cell/room without the existing gates.
- Building footprints must not overlap each other, the patch edge, water or slopes above the buildable limit; trees must not stand inside footprints or the cleared yard.
- Every room must be reachable from outdoors through the door graph.

---

### Task 1: Regional biome, habitat and vegetation initialization

**Files:**
- Create: `crates/mk_engine/src/regional/ecology.rs`
- Modify: `crates/mk_engine/src/regional/mod.rs`
- Test: `crates/mk_engine/tests/regional_ecology.rs`

**Interfaces:**
- Produces: `RegionalEcologyState { biome_grid, biosphere, organisms, vegetation }`.
- Produces: `bootstrap_regional_ecology(canon: Arc<CanonLocked>, physical: &RegionalPhysicalState, domain: &IslandDomain, seed: [u8;32]) -> Result<RegionalEcologyState, RegionalEcologyError>`.

- [ ] **Step 1:** Write tests asserting ocean/land biome consistency, at least one tree/shrub/grass on eligible land, no terrestrial plant in ocean cells, and deterministic bootstrap.
- [ ] **Step 2:** Run `cargo test -p mk_engine --test regional_ecology`; expect FAIL.
- [ ] **Step 3:** Reuse existing biome classification and biosphere/vegetation systems against regional climatology/hydrology/elevation.
- [ ] **Step 4:** Re-run regional ecology plus focused biosphere/vegetation tests; expect PASS.
- [ ] **Step 5:** Commit `feat(engine): seed island ecology`.

### Task 2: Regional resources and material economy

**Files:**
- Create: `crates/mk_engine/src/regional/resources.rs`
- Test: `crates/mk_engine/tests/regional_resources.rs`

**Interfaces:**
- Produces: `bootstrap_regional_resources(ecology: &RegionalEcologyState, physical: &RegionalPhysicalState) -> ResourceEconomyState`.
- Existing `ResourceEconomyState::{apply_human_action,regenerate}` remains the action API.

- [ ] **Step 1:** Write tests asserting no nodes in aquatic cells, forest tree/food nodes, volcanic ore contribution, finite non-renewable deposits and deterministic IDs/order.
- [ ] **Step 2:** Run `cargo test -p mk_engine --test regional_resources`; expect FAIL.
- [ ] **Step 3:** Call existing `ResourceEconomyState::from_terrain` with the regional biome/volcanic grids and add only regional validation/placement glue.
- [ ] **Step 4:** Run existing resource economy tests plus the regional test; expect PASS.
- [ ] **Step 5:** Commit `feat(engine): bind resources to island terrain`.

### Task 3: Default island scenario and canonical estate

**Files:**
- Create: `crates/mk_island/src/scenario.rs`
- Create: `fixtures/island/default_scenario.json`
- Create: `crates/mk_engine/src/regional/property.rs`
- Test: `crates/mk_engine/tests/island_property_scenario.rs`

**Interfaces:**
- Produces: `IslandCadenceProfile { human_seconds: u64, weather_ocean_seconds: u64, hydrology_ecology_resource_seconds: u64, geophysics_seconds: u64 }` with defaults `60`, `3_600`, `21_600`, `86_400` and `validate() -> Result<(), IslandScenarioError>` rejecting zero values. Produces `IslandScenario { version: u32, seed: [u8; 32], profile: IslandProfile, cadences: IslandCadenceProfile, founders_enabled: bool, estate_enabled: bool }` and `IslandScenario::load(path: &Path) -> Result<Self, IslandScenarioError>`.
- Produces: `choose_regional_estate_location(physical: &RegionalPhysicalState, domain: &IslandDomain) -> Option<(usize, usize)>`, `bootstrap_regional_property(physical: &RegionalPhysicalState, domain: &IslandDomain) -> Result<PropertySystem, RegionalPropertyError>`, and a `LocalPatchSpec` centered on the estate for high-detail property/interior navigation. Existing room names remain the semantic interior authority; the patch supplies local metric coordinates rather than inventing a second room inventory.

- [ ] **Step 1:** Write tests asserting deterministic estate placement and presence of all six building kinds, Gem-D/Gem-K bedrooms, named vehicles, tool categories, computers and two administrator network accounts.
- [ ] **Step 2:** Run `cargo test -p mk_engine --test island_property_scenario`; expect FAIL.
- [ ] **Step 3:** Implement regional estate scoring from real physical/climatology state and instantiate existing `PropertySystem::new(Some(location))` without copying inventory definitions.
- [ ] **Step 4:** Re-run property scenario plus existing `organisms::property` tests; expect PASS.
- [ ] **Step 5:** Commit `feat(island): place canonical founders estate`.

### Task 4: Metric estate layout and navigation

**Files:**
- Create: `crates/mk_engine/src/regional/estate_layout.rs`
- Modify: `crates/mk_engine/src/regional/property.rs`
- Test: `crates/mk_engine/tests/island_estate_layout.rs`

**Interfaces:**
- Produces: `EstateLayout { patch: LocalPatchSpec, terrain_m: Grid2<f64>, buildings: Vec<BuildingFootprint>, rooms: Vec<RoomLayout>, doors: Vec<Door>, items: Vec<ItemPlacement>, yard: Rect }`.
- `BuildingFootprint { building_id, kind, rect_m, rotation_deg }`; `RoomLayout { building_id, label, rect_m }` with labels taken from `PropertyItem` room groupings (`"Kitchen"`, `"<name>'s Bedroom"`, …) and building kinds; `Door { a: Space, b: Space, position_m }` where `Space::{Outdoors, Room(id)}`; `ItemPlacement { item_id, space, position_m }` (vehicles in garage/yard, tools in shed categories, computers in the computer room).
- Produces: `layout_estate(property: &PropertySystem, physical: &RegionalPhysicalState, domain: &IslandDomain, patch: &LocalPatchSpec, seed: [u8; 32]) -> Result<EstateLayout, EstateLayoutError>`, `EstateLayout::space_at(position_m) -> Option<Space>`, `EstateLayout::route(from: Space, to: Space) -> Option<Vec<Space>>`.
- Patch terrain is the medium-cell elevation bilinearly interpolated plus bounded deterministic detail; it never changes the regional elevation.

- [ ] **Step 1:** Write tests: every `PropertyItem` placed exactly once in a space matching its building/room; no footprint overlaps; footprints on dry land under the slope limit; every room routable from `Outdoors`; `space_at` round-trips each placement; identical bytes for identical inputs; a patch with no buildable site returns `EstateLayoutError::NoBuildableSite`.
- [ ] **Step 2:** Run `cargo test -p mk_engine --test island_estate_layout`; expect FAIL.
- [ ] **Step 3:** Implement deterministic site selection inside the patch, rectangle packing per building kind sized from its room/item count, room subdivision and door placement.
- [ ] **Step 4:** Re-run; expect PASS, and existing `organisms::property` tests remain green.
- [ ] **Step 5:** Commit `feat(engine): lay out the founders estate in metres`.

### Task 5: Dense vegetation in the estate patch

**Files:**
- Create: `crates/mk_engine/src/regional/local_vegetation.rs`
- Test: `crates/mk_engine/tests/island_local_vegetation.rs`

**Interfaces:**
- Produces: `LocalVegetationPatch { trees: Vec<TreeInstance>, ground_cover: Grid2<GroundCover>, cap: usize }`, `TreeInstance { id, kind, position_m, height_m, stem_diameter_m, biomass_kgc, alive }`.
- Produces: `seed_local_vegetation(ecology: &RegionalEcologyState, layout: &EstateLayout, domain: &IslandDomain, seed: [u8; 32], cap: usize) -> Result<LocalVegetationPatch, LocalVegetationError>` and `step_local_vegetation(&mut self, ecology: &RegionalEcologyState, dt_seconds: u64)` (growth/mortality scaled to the cell's aggregate NPP).
- Produces: `fell_tree(&mut self, tree_id, economy: &mut ResourceEconomyState, ledger: &mut Ledger) -> Result<WoodYield, LocalVegetationError>` — wood enters the economy and biomass carbon is debited from the regional pool.

- [ ] **Step 1:** Write tests: tree density follows biome/biomass (forest ≫ grassland, none in water); patch biomass sums to the cell share within ±2% (initial tolerance; revise with evidence); no tree in footprints or yard; count never exceeds `cap`; felling conserves carbon through the ledger; deterministic bytes.
- [ ] **Step 2:** Run `cargo test -p mk_engine --test island_local_vegetation`; expect FAIL.
- [ ] **Step 3:** Implement deterministic Poisson-disc placement with spacing from biome/kind, size distribution from biomass, and growth/mortality tied to aggregate NPP.
- [ ] **Step 4:** Re-run plus focused biosphere/vegetation tests; expect PASS.
- [ ] **Step 5:** Commit `feat(engine): simulate individual trees around the estate`.

### Task 6: Canonical founders and regional interaction adapter

**Files:**
- Create: `crates/mk_engine/src/regional/humans.rs`
- Test: `crates/mk_engine/tests/island_human_runtime.rs`
- Modify: `apps/island_humans/src/lib.rs` only to delegate to the new shared adapter until the app is removed in Phase 5.

**Interfaces:**
- Produces: `bootstrap_regional_humans(property: &PropertySystem, layout: &EstateLayout) -> Result<HumanSystem, RegionalHumanError>`; each founder's estate position is an `EstateLayout` `Space` plus metres.
- Produces: `step_regional_humans(humans: &mut HumanSystem, economy: &mut ResourceEconomyState, property: &PropertySystem, layout: &EstateLayout, vegetation: &mut LocalVegetationPatch, physical: &RegionalPhysicalState, ecology: &RegionalEcologyState, tick: Tick, dt_seconds: u64, rng: &RngRegistry) -> Result<(), RegionalHumanError>`. Movement inside the estate follows `EstateLayout::route`.

- [ ] **Step 1:** Write tests asserting exactly Gem-D/Gem-K default founders, each starts in their own bedroom per the layout, human IDs/profiles match canonical constructors, and a human away from the estate lacks computer access.
- [ ] **Step 2:** Add tests: a founder routed to the computer room gains access through the network-account/property gate, a founder in the kitchen does not, and a non-account human in the computer room remains denied; a founder felling a patch tree adds wood to the economy. Upstream `AgentWorldObservation::computer_access` is a per-cell building check (anyone on the estate cell qualifies); deriving it from the human's current `Space` is a deliberate island divergence — record it in `UPSTREAM.md` and leave the network-account gate itself unchanged.
- [ ] **Step 3:** Run `cargo test -p mk_engine --test island_human_runtime`; expect FAIL.
- [ ] **Step 4:** Adapt existing `HumanSystem`/observation/action flow to regional physical/ecology/property fields without changing human cognition/body modules.
- [ ] **Step 5:** Re-run the regional test, `cargo test -p mk_core human -- --test-threads=1`, and `cargo test -p mk_engine humans:: --lib -- --test-threads=1`; expect 0 failures.
- [ ] **Step 6:** Commit `feat(engine): run canonical humans on island`.

### Task 7: Phase-3 integrated life/property acceptance

**Files:**
- Create: `crates/mk_engine/tests/island_life_acceptance.rs`
- Modify: `UPSTREAM.md`

**Interfaces:**
- Consumes all Phase-3 APIs.

- [ ] **Step 1:** Bootstrap fixed-seed physical + ecology + resources + property + layout + local vegetation + humans, advance one simulated week, and assert living vegetation (regional and patch), resources, both founders inside the layout, intact estate inventory, economy events and deterministic final serialization.
- [ ] **Step 2:** Run the acceptance test twice; expect identical blake3 digest and PASS.
- [ ] **Step 3:** Update provenance for scenario/adapter files; run fmt and Phase-3 focused tests.
- [ ] **Step 4:** Commit `test(island): lock life property human acceptance`.
- [ ] **Step 5:** Add `island_preview life` writing `biomes.png`, `resources.png`, `estate_plan.png` (footprints, rooms, doors, item markers, founder positions) and `estate_trees.png`, with the Phase-1 determinism test pattern; commit the default preview under `docs/previews/phase3/`.
- [ ] **Step 6:** Gate 3 review: the owner checks the estate plan and tree map before Phase 4.
