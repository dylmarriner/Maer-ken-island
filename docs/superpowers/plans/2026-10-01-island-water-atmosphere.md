# Island Water & Atmosphere Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Run Maer-Ken climate, weather, ocean, tides and hydrology on the bounded island domain with explicit deterministic edge exchange.

**Architecture:** Keep existing state types and global functions as upstream baselines, but add regional stepping functions in `mk_engine::regional` that use `IslandDomain` for cell size/latitude/longitude and `RegionalBoundaryState` for external ocean/atmosphere forcing. Regional routing never wraps across domain edges; flows reaching ocean/edges are explicit budget terms. `mk_island` stays data-only (Phase 1 dependency rule); everything here lives in `mk_engine`.

The upstream climate is a zonal energy-balance model with planet-wide terms a regional window cannot compute from its own cells:

- meridional transport relaxes each cell to `0.6·T_rad + 0.4·mean(T_rad)`, the cos-latitude-weighted mean of radiative-equilibrium temperature (`climate/mod.rs:484, 503-504`);
- CO₂ is stepped from `previous.global_temperature()` (`climate/mod.rs:446`);
- `ClimateForcing::volcanic_co2_mol_yr` is a planet-wide total;
- `mean_absorbed_flux_w_m2` is a global average (`climate/mod.rs:488`);
- weather rescales precipitation so the grid mean equals global evaporation from `mean_absorbed_flux_w_m2` (`weather/mod.rs:244-260`) and assumes row spacing `π / nrows` (`weather/mod.rs:195`);
- hydrology reads humidity from `GridSpec::lat_rad` and hard-codes a 50 km slope distance (`hydrology/mod.rs:284-288`);
- ocean sets SST from climate surface temperature every step (`ocean/mod.rs:223`).

This phase therefore adds a compact 1-D **zonal background model** — latitude bands only, no longitude, no surface detail — that supplies every one of those planet-wide quantities, and from which atmospheric and ocean edge forcing are derived. It is compact analytic forcing under spec §5, not a planetary grid.

**Schedule risk:** this is the phase most likely to overrun. Task 1 exists to size it before code is written; re-estimate after Task 1, not after Task 3.

**Tech Stack:** Rust 2021, `mk_core`, `mk_island`, `mk_engine` climate/weather/ocean/hydrology/tides modules, `island_preview` from Phase 1.

**Spec:** `docs/superpowers/specs/2026-10-01-maer-ken-island-regional-world-design.md`

## Global Constraints

- Detailed atmosphere/ocean state exists only inside the regional domain.
- Canonical orbit/rotation/day/season values remain analytic inputs.
- Edge forcing is deterministic and serializable.
- Regional cell area is flat `cell_size_m(level)²`, never spherical row area; row spacing is `cell_size_m(level)`, never `π·R/nrows`.
- Hydrology reaching ocean or a domain edge is conserved and ledger/audit visible.
- No regional step computes a planet-wide quantity from regional cells; global/zonal context comes only from `ZonalBackgroundState`.
- The zonal background has a fixed, small band count (default 64) and is part of persisted deterministic state.
- **Grid levels:** zonal background is 1-D; climate, weather and ocean run on `DomainLevel::Coarse` (160 × 200 at 12 km); hydrology and tides run on `DomainLevel::Medium` (960 × 1,200 at 2 km). Crossing levels goes only through `resample_coarse_to_medium` (bilinear) and `aggregate_medium_to_coarse` (area mean, flux-conserving), introduced in Task 3.
- **Land-only fine grid.** Only ~6% of the domain is land (~67,000 of 1,152,000 medium cells). Medium-level processes (hydrology, ecology, resources, local vegetation) iterate `domain.active_cells(Medium)` — land plus the coastal band — and never the open ocean, which is handled on Coarse. Storage stays dense `Grid2`; only iteration is restricted. Measured basis: upstream world physics costs ~6 µs per cell per step (32 × 64 grid, release), so iterating all 1.15 M medium cells would cost ~7 s per step against ~0.4 s for active cells.
- **Coriolis:** `f = 2Ω·sin(latitude_rad_for_row)`, Ω from the canon rotation period, computed once per level from the domain.

## Review Focus

- West/east hydrology and current calculations must not wrap.
- Extreme but finite edge forcing must stay numerically finite and clamped to physical model ranges.
- Zero-length steps must not mutate prognostic state.
- Rain/runoff entering ocean must be reflected in water budgets rather than disappearing, including across the medium→coarse aggregation.
- Regional latitude mapping must be consistent across climate, weather and Coriolis calculations.
- The zonal background must reproduce the upstream global model's zonal-mean temperatures within a documented tolerance, or the regional island will drift to a different climate than Maer-Ken.

---

### Task 1: Planetary-dependency inventory

**Files:**
- Create: `docs/island/PLANETARY_DEPENDENCIES.md`

- [x] **Step 1:** For `climate`, `weather`, `ocean`, `hydrology`, `tides` and `insolation` (this phase), and `biosphere`, `biosphere::habitat`, `organisms::runtime`, `perception` and `humans` movement/lifecycle (Phase 3 Tasks 1 and 6), list every use of global means, spherical row area (`cell_area_at_row_m2`, `cos(lat)` weights), `GridSpec::lat_rad`/`lon_rad`, row spacing from planet radius, longitude wrap (`% nlon`, `rem_euclid`, `nlon - 1` neighbours), hard-coded cell distances, and pole handling, with file:line. Start from the list in this plan's Architecture section; it is known to be incomplete.
- [x] **Step 2:** Classify each as: replace with domain geometry, replace with zonal-background input, replace with edge forcing, or unaffected.
- [x] **Step 3:** Re-estimate Tasks 2–6 from the inventory and record the estimate in the document.
- [x] **Step 4:** Commit `docs(island): inventory planetary dependencies in physical systems`.

### Task 2: Zonal background forcing model

**Files:**
- Create: `crates/mk_engine/src/regional/zonal.rs`
- Modify: `crates/mk_engine/src/regional/boundary.rs`
- Modify: `crates/mk_engine/src/regional/mod.rs`
- Test: `crates/mk_engine/tests/regional_zonal_background.rs`

**Interfaces:**
- Produces: `ZonalBackgroundState { climate: ClimateState, grid: GridSpec, global_mean_surface_temperature_k, mean_absorbed_flux_w_m2, global_mean_precipitation_mm_day, atmospheric_co2_ppm, planetary_volcanic_co2_mol_yr, sim_time_seconds }`.
- Produces: `ZonalBackgroundState::bootstrap(canon: &CanonLocked, band_count: usize) -> Self` and `step(&mut self, canon: &CanonLocked, sim_time_seconds: f64, dt_seconds: f64)`.
- Produces: `sample_regional_boundaries_with_background(seed, canon, domain, background: &ZonalBackgroundState, sim_time_seconds) -> RegionalBoundaryState` — same output type as Phase 1's `sample_regional_boundaries`, with atmospheric edge temperature/humidity/wind and ocean edge temperature/salinity taken from the background bands at the domain's edge latitudes, and `provisional: false`. Phase 1's function is kept for geophysics-only use.

- [x] **Step 1:** **Try reuse first:** a `GridSpec::new(band_count, 1)` grid makes upstream `step_climate` already zonal. Implement the background as upstream `step_climate` on that grid with an aquaplanet surface (all-ocean elevation, canon geothermal flux, canon volcanic CO₂), so no equations are copied. Only if that fails a test below, extract the shared equations from `climate/mod.rs` into functions both paths call, so upstream and island cannot silently diverge.
- [x] **Step 2:** Spin-up: upstream `spin_up_climate` is private (`world_integration.rs:130`). Make it `pub(crate)` (recorded divergence) and reuse it rather than copying.
- [x] **Step 3:** Write a calibration test: run upstream `step_climate` on the default `WorldState` grid (32 × 64) and the zonal background at 32 bands with matching canon for one orbital period (`canon.orbital_period_s`); compare per-row zonal means. Assert agreement within ±1.5 K per band and ±0.5 K global mean (initial tolerances; tighten or justify from evidence and record the rationale in the test). Then assert the 64-band default stays within ±0.5 K of the 32-band run on global mean. The calibration test belongs in the slow tier if it exceeds ~30 s debug.
- [x] **Step 4:** Write determinism, zero-dt no-op and finite-extreme-input tests, and a test that boundary samples with background differ from the provisional Phase-1 values and match background bands at the edge latitudes.
- [x] **Step 5:** Run `cargo test -p mk_engine --test regional_zonal_background`; expect FAIL; implement; re-run plus `cargo test -p mk_engine --lib climate::`; expect PASS.
- [x] **Step 6:** Commit `feat(engine): add zonal background forcing`.

Implementation notes: an all-ocean 1-column background missed polar land by 4.6 K at 73° S (radiative T goes as flux^¼, so land through polar night averages colder). The background therefore has 64 columns per band, with each band's land columns taken once at bootstrap from upstream tectonics' land fraction; with that it meets ±1.5 K per band and ±0.5 K global mean against the upstream 32 × 64 world over one orbit. `WorldState::new` rejects the island canon (declared density exception), so the test builds the upstream world's fields directly.

### Task 3: Regional climate, weather and level resampling

**Files:**
- Create: `crates/mk_engine/src/regional/levels.rs`
- Create: `crates/mk_engine/src/regional/climate.rs`
- Create: `crates/mk_engine/src/regional/weather.rs`
- Modify: `crates/mk_engine/src/regional/mod.rs`
- Test: `crates/mk_engine/tests/regional_atmosphere.rs`, unit tests in `levels.rs`

**Interfaces:**
- Produces: `resample_coarse_to_medium<T: Lerp>(coarse: &Grid2<T>, domain) -> Grid2<T>` and `aggregate_medium_to_coarse(medium: &Grid2<f64>, domain) -> Grid2<f64>` (area mean).
- Produces: `regional_coriolis(domain, level, canon) -> Grid2<f64>`.
- Produces: `step_regional_climate(previous: &ClimateState, forcing: &ClimateForcing<'_>, domain: &IslandDomain, background: &ZonalBackgroundState, atmosphere: &AtmosphereBoundaryForcing, ocean_edge: &OceanBoundaryForcing) -> ClimateState` on Coarse. `forcing.volcanic_co2_mol_yr` is the island's own volcanic output from Phase 1; CO₂ concentration, the meridional-transport target `mean(T_rad)`, and `mean_absorbed_flux_w_m2` come from `background`, not regional cells. Ocean-edge SST forcing is applied here, because upstream ocean copies SST from climate (`ocean/mod.rs:223`).
- Produces: `step_regional_weather(canon: &CanonLocked, tick: Tick, climate: &ClimateState, elevation_m: &Grid2<f64>, domain: &IslandDomain, background: &ZonalBackgroundState, atmosphere: &AtmosphereBoundaryForcing) -> WeatherState` on Coarse, using `regional_coriolis`, flat row spacing, and `background.global_mean_precipitation_mm_day` for the precipitation normalisation.
- Produces (new physics, island divergence): `apply_diurnal_cycle(climate: &mut ClimateState, insolation: &InsolationState, surface: &SurfaceProperties, domain)` — upstream climate is a daily mean; the island resolves day and night across the 36-hour day: instantaneous insolation, surface energy balance with land/ocean heat capacity, a diurnal temperature range that is larger inland and under clear skies, nocturnal cooling and inversions in valleys, and land/sea breezes. The daily mean is preserved, so the zonal-background coupling is unchanged.
- Produces (new physics, island divergence): `apply_orographic_precipitation(weather: &mut WeatherState, elevation_m: &Grid2<f64>, domain)` — upwind moisture budget along the wind vector: rain enhancement on windward slopes proportional to upslope gradient, depletion of the carried moisture, and a rain shadow downwind; total precipitation over the domain is preserved within ±1% so the normalisation still holds.

- [x] **Step 1:** Write `levels.rs` tests: coarse→medium→coarse round-trip within interpolation tolerance; medium→coarse conserves `Σ value·area` exactly.
- [x] **Step 2:** Write `regional_atmosphere.rs` tests: deterministic temperature/rain/wind; reference-latitude seasonality (southern-hemisphere phase); warmer at the northern edge than the southern; finite boundary gradients; no spherical area weighting; an all-ocean domain whose mean temperature matches the background at the same latitudes within ±1 K; a single west–east ridge under a westerly wind gets more rain on its west flank than its east flank, and domain-total precipitation is preserved within ±1%; the diurnal temperature range over inland valleys exceeds that over the ocean by the reference-pack ratio, and the daily mean is unchanged by the diurnal cycle.
- [x] **Step 3:** Run `cargo test -p mk_engine --test regional_atmosphere`; expect FAIL.
- [x] **Step 4:** Implement climate reusing upstream equations with domain latitude, flat weighting, and background-sourced global terms; blend atmospheric and ocean-edge forcing only through explicit boundary cells.
- [x] **Step 5:** Implement weather on the same contract, then `apply_orographic_precipitation`. Do not change imported global APIs; extract shared helpers where an upstream function hard-codes spherical geometry, as Phase 1 Task 4 did for volcanism.
- [x] **Step 6:** Run `cargo test -p mk_engine --test regional_atmosphere` and `cargo test -p mk_engine --lib -- climate:: weather::`; expect PASS.
- [x] **Step 7:** Record the orographic precipitation step and any extracted helpers in `UPSTREAM.md`. Commit `feat(engine): regionalize climate and weather`.

Implementation notes: `step_climate` was split into `step_climate_on` (row latitudes + optional `RegionalClimateTerms`) rather than copying its equations; the lapse rate enters as an equilibrium offset so it relaxes with the heat capacity. Regional weather normalises each row's mean rain to the background's rate at that latitude (a window cannot compute the planetary mean). Edge cells relax linearly over 5 coarse cells; edge SST/air are clamped to 150-400 K and wind to ±60 m/s. The diurnal cycle uses the reference pack's ranges scaled by √(day/24 h) and is a zero-mean sinusoid, so the daily mean is exactly preserved; land/sea breezes are not built (D3). Tests spin four orbits: the ocean's ~1.4-orbit relaxation means two leave ~1 K of cold-start residual. The upstream ocean's seasonal lag is ~a quarter orbit (D25).

### Task 3b: Synoptic weather systems

Upstream weather is diagnostic from climate; real mid-latitude weather is a sequence of travelling fronts, highs and lows. A full dynamical atmosphere is beyond this phase; instead, synoptic systems are generated from the physics that creates them and steered by the background flow.

**Files:**
- Create: `crates/mk_engine/src/regional/synoptic.rs`
- Test: `crates/mk_engine/tests/regional_synoptic.rs`

**Interfaces:**
- Produces: `SynopticState { systems: Vec<SynopticSystem { kind: Cyclone | Anticyclone | Front, centre_m, velocity_m_s, central_pressure_pa, radius_m, age_s }> }` and `step_synoptic(state, background: &ZonalBackgroundState, boundaries, domain, canon, dt_seconds, rng) -> SynopticState`.
- Physics: genesis rate from the Eady baroclinic growth rate (`0.31·f·|∂u/∂z|/N`, from the background's meridional temperature gradient and the canon's rotation rate); system scale from the Rossby deformation radius (`N·H/f`), which differs from Earth's because of the 36-hour day; systems enter at the upwind domain edge, move with the steering flow, deepen and decay with documented lifetimes. Weather (wind, cloud, rain) is the diagnostic upstream field modulated by the systems: fronts bring rain bands and wind shifts, lows bring storms, highs bring clear calm spells.

- [x] **Step 1:** Write tests: system frequency, size and lifetime fall within the reference climate pack's mid-latitude storm-track ranges after scaling by the planet's deformation radius and rotation rate; a front's passage produces a pressure trough, wind shift and rain band; long-run mean rainfall and wind still match Task 3's climatology within 5%; deterministic per seed.
- [x] **Step 2:** Run `cargo test -p mk_engine --test regional_synoptic`; expect FAIL; implement; re-run; expect PASS.
- [x] **Step 3:** Commit `feat(engine): travelling synoptic weather systems`.

Implementation notes: the Eady rate reduces to `0.31 g |∂T/∂y| / (N T)` (f cancels), so the 36 h day sets system size (`L_d` is 1.5x Earth's) but not frequency. The canon's 19,113 km radius makes `∂T/∂y` ~3x weaker, so the rate at 41° S is 0.12/day against Earth's 0.5 and systems are rarer and slower (D27). Steering is surface wind plus thermal-wind shear over 3 km. A 60-day running normaliser divides out the mean rain ratio, so the long-run mean matches Task 3's climatology within 5%. `step_synoptic` is `SynopticState::step`; it does not take `boundaries` (edge forcing already reaches the weather through Task 3).

### Task 4: Regional ocean and tides

**Files:**
- Create: `crates/mk_engine/src/regional/ocean.rs`
- Create: `crates/mk_engine/src/regional/tides.rs`
- Test: `crates/mk_engine/tests/regional_ocean.rs`

**Interfaces:**
- Produces: `step_regional_ocean(canon: &CanonLocked, previous: &Grid2<OceanColumn>, forcing: &OceanForcing<'_>, domain: &IslandDomain, ocean_boundary: &OceanBoundaryForcing) -> OceanState` on Coarse — same inputs as upstream `step_ocean` (`OceanForcing { wind, climate, precipitation_mm_day, coriolis, elevation_m, dt_seconds }`) with regional Coriolis and geometry. Edge relaxation here covers salinity and currents; SST comes from climate (Task 3).
- Produces: `step_regional_tides(canon: &Arc<CanonLocked>, sim_time_seconds: f64, dt_seconds: u64, domain: &IslandDomain, astronomy: &AstronomyForcing, ledger: &mut Ledger) -> TidalState` on Medium. Keeps upstream's dissipation ledger entry, scaled to the domain's ocean area, and uses `longitude_rad_for_col` against the moons' sub-lunar longitudes.

- [x] **Step 1:** Write tests asserting land cells contain no ocean column, every edge ocean cell receives boundary salinity/current forcing, currents do not wrap, tidal height varies across the domain with longitude, tidal dissipation is booked in the ledger, and identical forcing produces identical bytes.
- [x] **Step 2:** Run `cargo test -p mk_engine --test regional_ocean`; expect FAIL.
- [x] **Step 3:** Reuse upstream seawater density/evaporation/heat equations and add deterministic edge relaxation/advection against `OceanBoundaryForcing`.
- [x] **Step 4:** Keep tide phase analytic; map tide amplitude onto regional ocean cells.
- [x] **Step 5:** Run `cargo test -p mk_engine --test regional_ocean` and `cargo test -p mk_engine --lib -- ocean:: tides::` plus `cargo test -p mk_engine --test tides`; expect PASS.
- [x] **Step 6:** Commit `feat(engine): regionalize ocean and tides`.

Implementation notes: `step_ocean` and `step_tides` were split like `step_climate` (`step_ocean_on`, `step_tides_on`) so no equation is copied. Ocean currents are cm/s and the boundary inflow m/s, so edge currents are converted. Edge cells clear of the corners equal the boundary exactly; within 5 cells of a corner the two edges' targets blend by weight. The tide field is on the coarse grid (RAM: the medium grid would cost ~46 MB per snapshot for fields that vary over thousands of km). `step_regional_tides` takes the coarse elevation instead of `AstronomyForcing`: the sub-lunar longitudes come from the same analytic function the boundary sampler uses (a test pins them equal).

### Task 5: Regional hydrology and coastline exchange

**Files:**
- Create: `crates/mk_engine/src/regional/hydrology.rs`
- Test: `crates/mk_engine/tests/regional_hydrology.rs`

**Interfaces:**
- Produces: `step_regional_hydrology(canon: &CanonLocked, previous: &HydrologyState, weather_medium: &WeatherState, climate_medium: &ClimateState, topography: &Grid2<f64>, domain: &IslandDomain, dt_seconds: f64) -> HydrologyState` on Medium. Callers resample coarse climate/weather to medium with Task 3's `resample_coarse_to_medium` first; upstream hydrology indexes climate/weather at the same `(row, col)` (`hydrology/mod.rs:276-282`).
- Produces: non-wrapping `regional_downhill_neighbour(topography: &Grid2<f64>, row: usize, col: usize) -> Option<(usize, usize)>`.
- Slope distance is `domain.cell_size_m(Medium)` (not upstream's hard-coded 50,000 m); humidity latitude is `domain.latitude_rad_for_row(Medium, row)`.

- [x] **Step 1:** Write tests for a west-edge slope proving no east-edge wrap, rain-to-river-to-ocean mass conservation (including `aggregate_medium_to_coarse` of runoff into the coarse ocean), pit/lake accumulation, coastline movement and zero-time no-op.
- [x] **Step 2:** Run `cargo test -p mk_engine --test regional_hydrology`; expect FAIL.
- [x] **Step 3:** Port upstream bucket/routing equations with flat area `domain.cell_area_m2(DomainLevel::Medium)`, treating outflow to sea or a domain edge as `HydrologyBudget::surface_to_ocean_kg`.
- [x] **Step 4:** Run `cargo test -p mk_engine --test regional_hydrology` and `cargo test -p mk_engine --lib hydrology::`; expect PASS.
- [x] **Step 5:** Commit `feat(engine): route island hydrology regionally`.

Implementation notes: upstream's one-cell-per-step routing cannot make real rivers on 2 km cells, so this is a port, not a wrapper: a `FlowNetwork` (priority-flood fill, steepest-descent receivers on the filled surface) is built once per terrain and each cell's hillslope runoff is delivered down the channel within the step. Lakes (filled depressions deeper than 0.5 m) hold water to their spill level; `bootstrap_regional_hydrology` starts them full. `step_regional_hydrology_on` takes a prebuilt network and the **coarse** weather/climate (sampled at each land cell with `levels::sample_coarse_at_medium`), so no medium weather copies are built (the plan's `weather_medium`/`climate_medium` would be ~3 grids of 1.15 M cells); `step_regional_hydrology` builds the network itself, for one-off use. `routed_outflow_mm` stays empty (no water in transit), so every step's budget closes exactly. D28 records the channel simplifications.

### Task 6: Coupled regional physical tick

**Files:**
- Create: `crates/mk_engine/src/regional/physical.rs`
- Test: `crates/mk_engine/tests/regional_physical_coupling.rs`

**Interfaces:**
- Produces: `RegionalPhysicalState { geophysics, zonal_background, boundaries, insolation, climate, synoptic, weather, hydrology, ocean, tides, climatology, ledger }`. The state owns its `RegionalBoundaryState`; callers never pass one in. `insolation` is upstream `InsolationState` evaluated on the Coarse grid from `AstronomyForcing` with regional latitude/longitude; Phase 3's human observation reads `daylight_fraction` from it.
- Produces: `RegionalPhysicalState::bootstrap(canon: &Arc<CanonLocked>, domain: &IslandDomain, seed: [u8; 32]) -> Result<Self, RegionalPhysicalError>` (runs Phase 1 geophysics bootstrap, zonal spin-up and regional climate spin-up) and `RegionalPhysicalState::step(&mut self, canon: &Arc<CanonLocked>, domain: &IslandDomain, seed: [u8; 32], sim_time_seconds: f64, tick: Tick, dt_seconds: u64) -> Result<(), RegionalPhysicalError>`.
- Step order inside `step`: zonal background → `sample_regional_boundaries_with_background` (stored in `self.boundaries`) → insolation → tides → climate (+ diurnal) → synoptic systems → weather (+ orographic) → resample to medium → hydrology → aggregate runoff → ocean → slow geophysics cadence hook (no-op until Phase 4's scheduler).

- [x] **Step 1:** Write an acceptance test stepping 30 simulated days and asserting changing weather/ocean/hydrology, finite state, preserved land mask, water budget closure across the step, and deterministic final serialized hash. Add a slow-tier realism test over one simulated year comparing lapse rate, diurnal range, windward/leeward rainfall ratio, sea-surface temperature range, storm frequency and river runoff ratios against the Phase-0c climate and hydrology reference packs (scaled for the planet), with each tolerance stated.
- [x] **Step 2:** Run `cargo test -p mk_engine --test regional_physical_coupling`; expect FAIL.
- [x] **Step 3:** Implement `bootstrap` and `step` in the order above.
- [x] **Step 4:** Re-run the test twice and assert identical final blake3 hash.
- [x] **Step 5:** Update `UPSTREAM.md` with regional physical wrappers; run `cargo fmt --all -- --check` and `cargo clippy --workspace --all-targets -- -D warnings`.
- [x] **Step 6:** Commit `feat(engine): couple island physical systems`.
- [ ] **Step 7:** Add `island_preview physical --profile <path> --seed <hex> --days <n> --out <dir>` writing surface temperature, mean rainfall, river/lake, ocean surface temperature and current-vector PNGs plus `summary.json`; same determinism test pattern as Phase 1 Task 6.
- [ ] **Step 8:** Commit the default-seed 30-day preview under `docs/previews/phase2/` and commit `feat(preview): render island physical state`.
- [ ] **Step 9 (Gate 2 review):** The owner checks the previews for plausibility (warmer north/cooler south at the reference latitude, wetter windward and drier leeward of ranges, rivers reaching the sea) before Phase 3.
