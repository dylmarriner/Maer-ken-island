# Island Water & Atmosphere Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Run Maer-Ken climate, weather, ocean, tides and hydrology on the bounded island domain with explicit deterministic edge exchange.

**Architecture:** Keep existing state types and global functions as upstream baselines, but add regional stepping functions that use `IslandDomain` for cell area/latitude and `RegionalBoundaryState` for external ocean/atmosphere forcing. Regional routing never wraps across domain edges; flows reaching ocean/edges are explicit budget terms.

The upstream climate is a zonal energy-balance model whose meridional transport relaxes every cell toward the **area-weighted global mean temperature** (`climate/mod.rs`, Budyko-style term). A regional window has no global mean, so the regional climate cannot be produced by substituting latitude/area functions alone. This phase therefore adds a compact 1-D **zonal background model** — latitude bands only, no longitude, no surface detail — that supplies the global-mean and zonal-mean context the upstream equations need, and from which atmospheric and ocean edge forcing are derived. This is compact analytic forcing under spec §5, not a planetary grid.

**Schedule risk:** this is the phase most likely to overrun. Task 1 exists to size it before code is written; re-estimate after Task 1 rather than after Task 3.

**Tech Stack:** Rust 2021, `mk_core`, `mk_island`, `mk_engine` climate/weather/ocean/hydrology/tides modules, `island_preview` from Phase 1.

**Spec:** `docs/superpowers/specs/2026-10-01-maer-ken-island-regional-world-design.md`

## Global Constraints

- Detailed atmosphere/ocean state exists only inside the regional domain.
- Canonical orbit/rotation/day/season values remain analytic inputs.
- Edge forcing is deterministic and serializable.
- Regional cell area is flat `cell_width_m × cell_height_m`, never spherical row area.
- Hydrology reaching ocean or a domain edge is conserved and ledger/audit visible.
- No regional step computes a planet-wide mean from regional cells; global/zonal context comes only from `ZonalBackgroundState`.
- The zonal background has a fixed, small band count (default 64) and is part of persisted deterministic state.

## Review Focus

- West/east hydrology and current calculations must not wrap.
- Extreme but finite edge forcing must stay numerically finite and clamped to physical model ranges.
- Zero-length steps must not mutate prognostic state.
- Rain/runoff entering ocean must be reflected in water budgets rather than disappearing.
- Regional latitude mapping must be consistent across climate, weather and Coriolis calculations.
- The zonal background must reproduce the upstream global model's zonal-mean temperatures within a documented tolerance, or the regional island will drift to a different climate than Maer-Ken.

---

### Task 1: Planetary-dependency inventory

**Files:**
- Create: `docs/island/PLANETARY_DEPENDENCIES.md`

- [ ] **Step 1:** For `climate`, `weather`, `ocean`, `hydrology`, `tides` and `insolation`, list every use of global means, spherical row area (`cell_area_at_row_m2`, `cos(lat)` weights), `GridSpec::lat_rad`, longitude wrap (`% nlon`, `nlon - 1` neighbours), and pole handling, with file:line.
- [ ] **Step 2:** Classify each as: replace with domain geometry, replace with zonal-background input, replace with edge forcing, or unaffected.
- [ ] **Step 3:** Re-estimate Tasks 3–6 from the inventory and record the estimate in the document.
- [ ] **Step 4:** Commit `docs(island): inventory planetary dependencies in physical systems`.

### Task 2: Zonal background forcing model

**Files:**
- Create: `crates/mk_island/src/zonal.rs`
- Modify: `crates/mk_island/src/boundary.rs`
- Test: `crates/mk_island/tests/zonal_background.rs`

**Interfaces:**
- Produces: `ZonalBackgroundState { bands: Vec<ZonalBand>, global_mean_surface_temperature_k: f64, sim_time_seconds: f64 }`, `ZonalBand { lat_rad, surface_temperature_k, ocean_fraction, ... }`.
- Produces: `ZonalBackgroundState::bootstrap(canon: &CanonLocked, band_count: usize) -> Self` and `step(&mut self, canon: &CanonLocked, sim_time_seconds: f64, dt_seconds: f64)` reusing the upstream insolation, greenhouse, transport and thermal-inertia equations on bands.
- `RegionalBoundaryState::sample` derives atmospheric edge temperature/humidity/wind and ocean edge temperature/salinity from the background instead of free parameters.

- [ ] **Step 1:** Write a calibration test: run upstream `step_climate` on its default planet and the zonal model with matching canon for one simulated year; assert annual zonal-mean temperatures agree within ±1.5 K per band and global mean within ±0.5 K (initial tolerances; tighten or justify from evidence and record the rationale in the test).
- [ ] **Step 2:** Write determinism, zero-dt no-op and finite-extreme-input tests.
- [ ] **Step 3:** Run `cargo test -p mk_island --test zonal_background`; expect FAIL.
- [ ] **Step 4:** Implement by extracting shared equations from `climate/mod.rs` into functions both paths call, so upstream and island cannot silently diverge.
- [ ] **Step 5:** Re-run zonal tests and existing `climate::` tests; expect PASS.
- [ ] **Step 6:** Commit `feat(island): add zonal background forcing`.

### Task 3: Regional climate and atmospheric edge forcing

**Files:**
- Create: `crates/mk_engine/src/regional/climate.rs`
- Create: `crates/mk_engine/src/regional/weather.rs`
- Modify: `crates/mk_engine/src/regional/mod.rs`
- Test: `crates/mk_engine/tests/regional_atmosphere.rs`

**Interfaces:**
- Produces: `step_regional_climate(previous: &ClimateState, forcing: &ClimateForcing<'_>, domain: &IslandDomain, background: &ZonalBackgroundState, atmosphere: &AtmosphereBoundaryForcing) -> ClimateState`.
- Produces: `step_regional_weather(canon: &CanonLocked, tick: Tick, climate: &ClimateState, elevation: &Grid2<f64>, domain: &IslandDomain, atmosphere: &AtmosphereBoundaryForcing) -> WeatherState`.

- [ ] **Step 1:** Write tests for deterministic regional temperature/rain/wind, reference-latitude seasonality, finite boundary gradients, no spherical area weighting, and an all-ocean regional domain whose mean temperature matches the zonal background at the same latitudes within ±1 K.
- [ ] **Step 2:** Run `cargo test -p mk_engine --test regional_atmosphere`; expect FAIL.
- [ ] **Step 3:** Reuse the shared climate equations from Task 2, using domain latitude and flat regional weighting; take the meridional-transport target from `ZonalBackgroundState` (not a regional mean); blend edge atmospheric forcing only through explicit boundary cells.
- [ ] **Step 4:** Adapt weather to the same latitude/edge contract; do not change imported global APIs.
- [ ] **Step 5:** Re-run the regional test and focused global climate/weather tests; expect PASS.
- [ ] **Step 6:** Commit `feat(engine): regionalize climate and weather`.

### Task 4: Regional ocean and tides

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

### Task 5: Regional hydrology and coastline exchange

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

### Task 6: Coupled regional physical tick

**Files:**
- Create: `crates/mk_engine/src/regional/physical.rs`
- Test: `crates/mk_engine/tests/regional_physical_coupling.rs`

**Interfaces:**
- Produces: `RegionalPhysicalState { geophysics, zonal_background, climate, weather, hydrology, ocean, tides, climatology }`.
- Produces: `RegionalPhysicalState::bootstrap(canon: &CanonLocked, domain: &IslandDomain, boundaries: &RegionalBoundaryState, seed: [u8; 32]) -> Result<Self, RegionalPhysicalError>` and `RegionalPhysicalState::step(&mut self, canon: &CanonLocked, domain: &IslandDomain, boundaries: &RegionalBoundaryState, sim_time_seconds: f64, tick: Tick, dt_seconds: u64) -> Result<(), RegionalPhysicalError>`.

- [ ] **Step 1:** Write an acceptance test stepping 30 simulated days and asserting changing weather/ocean/hydrology, finite state, preserved land mask, and deterministic final serialized hash.
- [ ] **Step 2:** Run `cargo test -p mk_engine --test regional_physical_coupling`; expect FAIL.
- [ ] **Step 3:** Implement the physical step order: analytic astronomy → zonal background → boundaries → tides → climate → weather → hydrology → ocean → slow geophysics cadence hook.
- [ ] **Step 4:** Re-run test twice and assert identical final blake3 hash.
- [ ] **Step 5:** Update `UPSTREAM.md` with regional physical wrappers and run `cargo fmt --all -- --check`.
- [ ] **Step 6:** Commit `feat(engine): couple island physical systems`.
- [ ] **Step 7:** Add `island_preview physical --profile <path> --seed <hex> --days <n> --out <dir>` writing surface temperature, annual-mean rainfall, river/lake, ocean surface temperature and current-vector PNGs plus `summary.json`; same determinism test pattern as Phase 1 Task 5. This runs in the slow tier if it exceeds ~30 s.
- [ ] **Step 8:** Commit the default-seed 30-day preview under `docs/previews/phase2/` and commit `feat(preview): render island physical state`.
- [ ] **Step 9:** Gate 2 review: the owner checks the previews for plausibility (warmer north/cooler south at the reference latitude, rain shadow behind ranges, rivers reaching the sea) before Phase 3.
