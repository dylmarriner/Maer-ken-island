# Island Water & Atmosphere Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Run Maer-Ken climate, weather, ocean, tides and hydrology on the bounded island domain with explicit deterministic edge exchange.

**Architecture:** Keep existing state types and global functions as upstream baselines, but add regional stepping functions that use `IslandDomain` for cell area/latitude and `RegionalBoundaryState` for external ocean/atmosphere forcing. Regional routing never wraps across domain edges; flows reaching ocean/edges are explicit budget terms.

**Tech Stack:** Rust 2021, `mk_core`, `mk_island`, `mk_engine` climate/weather/ocean/hydrology/tides modules.

**Spec:** `docs/superpowers/specs/2026-10-01-maer-ken-island-regional-world-design.md`

## Global Constraints

- Detailed atmosphere/ocean state exists only inside the regional domain.
- Canonical orbit/rotation/day/season values remain analytic inputs.
- Edge forcing is deterministic and serializable.
- Regional cell area is flat `cell_width_m × cell_height_m`, never spherical row area.
- Hydrology reaching ocean or a domain edge is conserved and ledger/audit visible.

## Review Focus

- West/east hydrology and current calculations must not wrap.
- Extreme but finite edge forcing must stay numerically finite and clamped to physical model ranges.
- Zero-length steps must not mutate prognostic state.
- Rain/runoff entering ocean must be reflected in water budgets rather than disappearing.
- Regional latitude mapping must be consistent across climate, weather and Coriolis calculations.

---

### Task 1: Regional climate and atmospheric edge forcing

**Files:**
- Create: `crates/mk_engine/src/regional/climate.rs`
- Create: `crates/mk_engine/src/regional/weather.rs`
- Modify: `crates/mk_engine/src/regional/mod.rs`
- Test: `crates/mk_engine/tests/regional_atmosphere.rs`

**Interfaces:**
- Produces: `step_regional_climate(previous: &ClimateState, forcing: &ClimateForcing<'_>, domain: &IslandDomain, atmosphere: &AtmosphereBoundaryForcing) -> ClimateState`.
- Produces: `step_regional_weather(canon: &CanonLocked, tick: Tick, climate: &ClimateState, elevation: &Grid2<f64>, domain: &IslandDomain, atmosphere: &AtmosphereBoundaryForcing) -> WeatherState`.

- [ ] **Step 1:** Write tests for deterministic regional temperature/rain/wind, reference-latitude seasonality, finite boundary gradients and no spherical area weighting.
- [ ] **Step 2:** Run `cargo test -p mk_engine --test regional_atmosphere`; expect FAIL.
- [ ] **Step 3:** Extract/reuse upstream climate equations while replacing `GridSpec::lat_rad`/global-mean area assumptions with domain latitude and flat regional weighting; blend edge atmospheric forcing only through explicit boundary cells.
- [ ] **Step 4:** Adapt weather to the same latitude/edge contract; do not change imported global APIs.
- [ ] **Step 5:** Re-run the regional test and focused global climate/weather tests; expect PASS.
- [ ] **Step 6:** Commit `feat(engine): regionalize climate and weather`.

### Task 2: Regional ocean and tides

**Files:**
- Create: `crates/mk_engine/src/regional/ocean.rs`
- Create: `crates/mk_engine/src/regional/tides.rs`
- Test: `crates/mk_engine/tests/regional_ocean.rs`

**Interfaces:**
- Produces: `step_regional_ocean(canon: &CanonLocked, previous: &OceanState, climate: &ClimateState, weather: &WeatherState, elevation: &Grid2<f64>, domain: &IslandDomain, ocean_boundary: &OceanBoundaryForcing, dt_seconds: f64) -> OceanState`.
- Produces: `step_regional_tides(canon: &CanonLocked, sim_time_seconds: f64, domain: &IslandDomain, astronomy: &AstronomyForcing) -> TidalState`; the returned `TidalState` keeps the existing engine type while its regional grid dimensions come from `domain.medium_storage_spec()`.

- [ ] **Step 1:** Write tests asserting land cells contain no ocean column, all edge ocean cells receive boundary state, ocean currents do not wrap, and identical forcing produces identical bytes.
- [ ] **Step 2:** Run `cargo test -p mk_engine --test regional_ocean`; expect FAIL.
- [ ] **Step 3:** Reuse upstream seawater density/evaporation/heat equations and add deterministic edge relaxation/advection against `OceanBoundaryForcing`.
- [ ] **Step 4:** Keep astronomical tide phase analytic; map tide amplitude/state onto regional ocean cells without requiring a spherical ocean raster.
- [ ] **Step 5:** Re-run regional and existing ocean/tide tests; expect PASS.
- [ ] **Step 6:** Commit `feat(engine): regionalize ocean and tides`.

### Task 3: Regional hydrology and coastline exchange

**Files:**
- Create: `crates/mk_engine/src/regional/hydrology.rs`
- Test: `crates/mk_engine/tests/regional_hydrology.rs`

**Interfaces:**
- Produces: `step_regional_hydrology(canon: &CanonLocked, previous: &HydrologyState, weather: &WeatherState, climate: &ClimateState, topography: &Grid2<f64>, domain: &IslandDomain, dt_seconds: f64) -> HydrologyState`.
- Produces: non-wrapping `regional_downhill_neighbour(topography: &Grid2<f64>, row: usize, col: usize) -> Option<(usize, usize)>`.

- [ ] **Step 1:** Write tests for a west-edge slope proving no east-edge wrap, rain-to-river-to-ocean mass conservation, pit/lake accumulation, coastline movement and zero-time no-op.
- [ ] **Step 2:** Run `cargo test -p mk_engine --test regional_hydrology`; expect FAIL.
- [ ] **Step 3:** Port upstream bucket/routing equations, replacing spherical row area with `domain.cell_area_m2(Medium)` and treating outflow to sea/domain edge as `surface_to_ocean_kg`.
- [ ] **Step 4:** Re-run regional test and existing hydrology tests; expect PASS.
- [ ] **Step 5:** Commit `feat(engine): route island hydrology regionally`.

### Task 4: Coupled regional physical tick

**Files:**
- Create: `crates/mk_engine/src/regional/physical.rs`
- Test: `crates/mk_engine/tests/regional_physical_coupling.rs`

**Interfaces:**
- Produces: `RegionalPhysicalState { geophysics, climate, weather, hydrology, ocean, tides, climatology }`.
- Produces: `RegionalPhysicalState::bootstrap(canon: &CanonLocked, domain: &IslandDomain, boundaries: &RegionalBoundaryState, seed: [u8; 32]) -> Result<Self, RegionalPhysicalError>` and `RegionalPhysicalState::step(&mut self, canon: &CanonLocked, domain: &IslandDomain, boundaries: &RegionalBoundaryState, sim_time_seconds: f64, tick: Tick, dt_seconds: u64) -> Result<(), RegionalPhysicalError>`.

- [ ] **Step 1:** Write an acceptance test stepping 30 simulated days and asserting changing weather/ocean/hydrology, finite state, preserved land mask, and deterministic final serialized hash.
- [ ] **Step 2:** Run `cargo test -p mk_engine --test regional_physical_coupling`; expect FAIL.
- [ ] **Step 3:** Implement the physical step order: analytic astronomy/boundaries → tides → climate → weather → hydrology → ocean → slow geophysics cadence hook.
- [ ] **Step 4:** Re-run test twice and assert identical final blake3 hash.
- [ ] **Step 5:** Update `UPSTREAM.md` with regional physical wrappers and run `cargo fmt --all -- --check`.
- [ ] **Step 6:** Commit `feat(engine): couple island physical systems`.
