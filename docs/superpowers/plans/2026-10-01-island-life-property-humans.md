# Island Life, Property & Humans Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Populate the regional island with retained Maer-Ken habitats, vegetation, resources, canonical property/equipment and the full Gem-D/Gem-K human runtime.

**Architecture:** Derive regional biome/ecology/resource state from the Phase-2 physical world, then instantiate property and humans through the already imported canonical engine types. Add only regional adapters for placement, observation and interaction; do not duplicate human internals or property inventories.

**Tech Stack:** Rust 2021, existing `mk_engine::biosphere`, `organisms`, `resource_economy`, `humans`, `mk_island` scenario/profile types.

**Spec:** `docs/superpowers/specs/2026-10-01-maer-ken-island-regional-world-design.md`

## Global Constraints

- Gem-D/Gem-K constructors and human modules remain sourced from the pinned Maer-Ken runtime.
- The founders' estate must retain house, shed, workshop, armoury, computer room, garage, rooms, named vehicles, attachments, tools, furniture, computers and network accounts already defined upstream.
- Only regional island/ocean cells may seed living populations or resources.
- Computer access remains gated by physical property location + `NetworkAccount`; no global access flag.
- Resource gathering/building uses actual regional `GridPosition` and terrain state.

## Review Focus

- All-ocean or no-valid-estate scenarios must fail clearly instead of inventing a property location.
- Founder placement must be on dry, non-dominantly-volcanic, buildable land.
- Aquatic cells must not receive terrestrial resource nodes/vegetation.
- Re-seeding vegetation/ecology must be idempotent for already initialized state.
- Human perception/action must not access property or computers from a different cell/room without the existing gates.

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

### Task 4: Canonical founders and regional interaction adapter

**Files:**
- Create: `crates/mk_engine/src/regional/humans.rs`
- Test: `crates/mk_engine/tests/island_human_runtime.rs`
- Modify: `apps/island_humans/src/lib.rs` only to delegate to the new shared adapter until the app is removed in Phase 5.

**Interfaces:**
- Produces: `bootstrap_regional_humans(property: &PropertySystem) -> Result<HumanSystem, RegionalHumanError>`.
- Produces: `step_regional_humans(humans: &mut HumanSystem, economy: &mut ResourceEconomyState, property: &PropertySystem, physical: &RegionalPhysicalState, ecology: &RegionalEcologyState, tick: Tick, dt_seconds: u64, rng: &RngRegistry) -> Result<(), RegionalHumanError>`.

- [ ] **Step 1:** Write tests asserting exactly Gem-D/Gem-K default founders, both start at estate location, human IDs/profiles match canonical constructors, and a human away from the estate lacks computer access.
- [ ] **Step 2:** Add a test moving a founder onto the estate and asserting the existing network-account/property gate enables computer access while a non-account human remains denied.
- [ ] **Step 3:** Run `cargo test -p mk_engine --test island_human_runtime`; expect FAIL.
- [ ] **Step 4:** Adapt existing `HumanSystem`/observation/action flow to regional physical/ecology/property fields without changing human cognition/body modules.
- [ ] **Step 5:** Re-run the regional test, `cargo test -p mk_core human -- --test-threads=1`, and `cargo test -p mk_engine humans:: --lib -- --test-threads=1`; expect 0 failures.
- [ ] **Step 6:** Commit `feat(engine): run canonical humans on island`.

### Task 5: Phase-3 integrated life/property acceptance

**Files:**
- Create: `crates/mk_engine/tests/island_life_acceptance.rs`
- Modify: `UPSTREAM.md`

**Interfaces:**
- Consumes all Phase-3 APIs.

- [ ] **Step 1:** Bootstrap fixed-seed physical + ecology + resources + property + humans, advance one simulated week, and assert living vegetation, resources, both founders, intact estate inventory, economy events and deterministic final serialization.
- [ ] **Step 2:** Run the acceptance test twice; expect identical blake3 digest and PASS.
- [ ] **Step 3:** Update provenance for scenario/adapter files; run fmt and Phase-3 focused tests.
- [ ] **Step 4:** Commit `test(island): lock life property human acceptance`.
