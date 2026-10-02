# Island Domain & Geophysics Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Introduce a deterministic regional spatial domain and generate one NZ-scale tectonic island with volcanism, bathymetry and a guaranteed surrounding-ocean buffer, and add the headless preview tool every later phase extends.

**Architecture:** Add a small `mk_island` crate that owns geometry/profile/boundary contracts while `mk_engine::regional` adapts existing Maer-Ken tectonic and volcanic state to a planar regional grid. Existing global functions remain intact until final pruning so imported tests continue to protect upstream behaviour.

**Tech Stack:** Rust 2021, `mk_core`, `mk_engine`, serde, blake3, rand_chacha, `png` (preview tool only).

**Spec:** `docs/superpowers/specs/2026-10-01-maer-ken-island-regional-world-design.md`

## Global Constraints

- Default land target `268,000 km^2`, acceptance tolerance ±5%.
- Minimum coastline-to-edge ocean buffer `300,000 m` on north/south/east/west edges.
- Initial benchmark profile dimensions `2,400,000 m × 1,920,000 m`; coarse cell `24,000 m`; medium cell `8,000 m`; reference latitude `-41.0°`; all values remain configurable and validated rather than canon-locked. This yields exactly `100 × 80` coarse cells and `300 × 240` medium cells. Phase 5 benchmarks may justify a different default, but any change must preserve the profile contract and acceptance tests.
- No periodic east/west wrap in regional neighbor queries.
- Boundary forcing is deterministic from seed, canon digest and simulation time.
- Existing `mk_core::grid::Grid2<T>` remains the storage container; regional geometry lives in `IslandDomain`, not in fake spherical `GridSpec` methods.
- Island shape defaults to unconstrained (area-fitted only). An optional `IslandShape` profile field may bias the landmass toward an elongated form; it never relaxes the area, connectivity or buffer constraints.

## Review Focus

- Zero/negative extent, cell size, target land area, or ocean buffer must return `IslandProfileError` before allocation.
- Domain dimensions not divisible by configured cell sizes must be rejected rather than truncated.
- Same profile + seed must generate byte-identical boundary and elevation state.
- Disconnected satellite land must not survive the final primary-island fitting pass.
- No land cell may exist within the configured edge-buffer band.

---

### Task 1: Regional profile and coordinate contract

**Files:**
- Create: `crates/mk_island/Cargo.toml`
- Create: `crates/mk_island/src/lib.rs`
- Create: `crates/mk_island/src/profile.rs`
- Create: `crates/mk_island/src/domain.rs`
- Modify: `Cargo.toml`
- Test: `crates/mk_island/tests/domain_contract.rs`

**Interfaces:**
- Produces: `IslandProfile::default_nz_scale() -> Self`, `IslandProfile::validate() -> Result<(), IslandProfileError>`.
- Produces: `IslandShape::{Unconstrained, Elongated { aspect_ratio: f64, orientation_deg: f64 }}` as `IslandProfile::shape`, default `Unconstrained`; validation rejects `aspect_ratio < 1.0`, non-finite values, and any elongated envelope whose length cannot fit inside the buffered domain at the given orientation.
- Produces: `IslandDomain::from_profile(profile: IslandProfile) -> Result<Self, IslandDomainError>`.
- Produces: `DomainLevel::{Coarse, Medium}`, `IslandDomain::{coarse_storage_spec, medium_storage_spec}() -> mk_core::grid::GridSpec`, `cell_center_m(level: DomainLevel, row: usize, col: usize) -> (f64, f64)`, `cell_area_m2(level: DomainLevel) -> f64`, `latitude_rad_for_row(level: DomainLevel, row: usize) -> f64`, `is_edge_buffer_cell(level: DomainLevel, row: usize, col: usize) -> bool`.
- Produces: `LocalPatchSpec { origin_x_m: f64, origin_y_m: f64, width_m: f64, height_m: f64, cell_size_m: f64, rows: usize, cols: usize }` and `IslandDomain::local_patch(center_x_m: f64, center_y_m: f64, extent_m: f64, cell_size_m: f64) -> Result<LocalPatchSpec, IslandDomainError>` for high-detail property/interior/navigation windows without allocating a world-wide fine grid.

- [ ] **Step 1:** Write `domain_contract.rs` tests asserting default numeric values, valid dimensions, flat cell area, coordinate round-trip, bounded local-patch geometry and all invalid profile/local-patch cases.
- [ ] **Step 2:** Run `cargo test -p mk_island --test domain_contract`; expect compile/test failure because the crate/types do not exist.
- [ ] **Step 3:** Add the workspace member and implement the profile/domain interfaces without changing `Grid2` storage semantics.
- [ ] **Step 4:** Re-run `cargo test -p mk_island --test domain_contract`; expect PASS.
- [ ] **Step 5:** Commit `feat(island): add regional domain contract`.

### Task 2: Deterministic regional boundary forcing

**Files:**
- Create: `crates/mk_island/src/boundary.rs`
- Modify: `crates/mk_island/src/lib.rs`
- Test: `crates/mk_island/tests/boundary_determinism.rs`

**Interfaces:**
- Produces: `OceanBoundaryForcing`, `AtmosphereBoundaryForcing`, `TectonicBoundaryForcing`, `AstronomyForcing`, `RegionalBoundaryState`.
- Produces: `RegionalBoundaryState::sample(seed: [u8;32], canon: &CanonLocked, domain: &IslandDomain, sim_time_seconds: f64) -> Self`.

- [ ] **Step 1:** Write tests asserting identical samples for identical inputs, changed samples for changed seed/time, finite forcing values on every edge, and no wall-clock fields.
- [ ] **Step 2:** Run `cargo test -p mk_island --test boundary_determinism`; expect FAIL.
- [ ] **Step 3:** Implement sampling with named blake3-derived deterministic streams and analytic canonical astronomy; store only serializable forcing data.
- [ ] **Step 4:** Re-run the test; expect PASS.
- [ ] **Step 5:** Commit `feat(island): add deterministic regional boundaries`.

### Task 3: Regional tectonics and non-wrapping neighbors

**Files:**
- Create: `crates/mk_engine/src/regional/mod.rs`
- Create: `crates/mk_engine/src/regional/tectonics.rs`
- Modify: `crates/mk_engine/src/lib.rs`
- Test: `crates/mk_engine/tests/regional_tectonics.rs`

**Interfaces:**
- Consumes: `IslandDomain`, `TectonicBoundaryForcing` from Tasks 1-2.
- Produces: `step_regional_tectonics(canon: &CanonLocked, tick: Tick, domain: &IslandDomain, forcing: &TectonicBoundaryForcing) -> TectonicsState`.
- Produces: `regional_neighbours(row,col,rows,cols) -> impl Iterator<Item=(usize,usize)>` with no wrap.

- [ ] **Step 1:** Write tests asserting deterministic plate assignment, multiple local plates, realistic heat-flow range, and that west-edge neighbours never include east-edge cells.
- [ ] **Step 2:** Run `cargo test -p mk_engine --test regional_tectonics`; expect FAIL.
- [ ] **Step 3:** Implement planar deterministic plate seeds/velocities and classify boundaries using regional neighbours plus edge tectonic forcing; reuse `PlateCell`, `PlateType`, `BoundaryType`, `TectonicsState`.
- [ ] **Step 4:** Re-run the test; expect PASS and existing `cargo test -p mk_engine tectonics:: --lib` remains green.
- [ ] **Step 5:** Commit `feat(engine): step tectonics on island domain`.

### Task 4: One-island terrain, target area and volcanism

**Files:**
- Create: `crates/mk_engine/src/regional/terrain.rs`
- Create: `crates/mk_engine/src/regional/geophysics.rs`
- Modify: `crates/mk_engine/src/regional/mod.rs`
- Test: `crates/mk_engine/tests/regional_geophysics.rs`

**Interfaces:**
- Produces: `RegionalGeophysics { tectonics, volcanism, elevation_m, bathymetry_m, land_area_m2 }`.
- Produces: `bootstrap_regional_geophysics(canon: &CanonLocked, domain: &IslandDomain, boundaries: &RegionalBoundaryState, seed: [u8;32]) -> Result<RegionalGeophysics, RegionalGeophysicsError>`.
- Produces: `primary_land_component(elevation_m: &Grid2<f64>, rows: usize, cols: usize) -> Vec<bool>` and `fit_sea_level_to_target(raw_elevation_m: &Grid2<f64>, domain: &IslandDomain, target_land_area_m2: f64, tolerance_fraction: f64, minimum_ocean_buffer_m: f64) -> Result<(Grid2<f64>, f64), RegionalGeophysicsError>`, returning the fitted elevation grid and sea-level offset.

- [ ] **Step 1:** Write fixed-seed tests asserting land area `254,600..=281,400 km²`, one connected primary land component, ocean in the complete 300 km edge band, negative offshore bathymetry, and deterministic elevation bytes.
- [ ] **Step 2:** Run `cargo test -p mk_engine --test regional_geophysics`; expect FAIL.
- [ ] **Step 3:** Implement sea-level fitting on regional cell area, deterministically retain the largest land component, submerge disconnected land, and iterate sea level until tolerance is met without violating the buffer.
- [ ] **Step 3a:** For `IslandShape::Elongated`, apply a smooth deterministic elevation envelope along the configured axis before fitting; add a test that the fitted land's principal-axis aspect ratio is within ±25% of the requested value and all other constraints still hold.
- [ ] **Step 4:** Feed the regional plate/heat state into existing volcanism logic through a regional wrapper; confirm volcanic fields do not require a planetary grid.
- [ ] **Step 5:** Re-run `regional_geophysics` plus focused volcanism tests; expect PASS.
- [ ] **Step 6:** Commit `feat(engine): generate nz-scale island geophysics`.

### Task 5: Headless preview tool

**Files:**
- Create: `apps/island_preview/Cargo.toml`
- Create: `apps/island_preview/src/main.rs`
- Modify: `Cargo.toml`
- Test: `apps/island_preview/tests/preview_determinism.rs`

**Interfaces:**
- CLI: `island_preview geophysics --profile <path> --seed <hex> --out <dir>` writes `elevation.png` (hypsometric land, depth-shaded ocean, 300 km buffer outline), `land_mask.png`, `plates.png` and `summary.json` (land area km², sea-level offset, component count, min/max elevation, plate count).
- Later phases add subcommands (`physical`, `life`, `world`) to the same tool; it never mutates or persists simulation state.

- [ ] **Step 1:** Write tests asserting identical PNG bytes and `summary.json` for the same profile/seed, and that `summary.json` area equals `RegionalGeophysics::land_area_m2`.
- [ ] **Step 2:** Run `cargo test -p island_preview`; expect FAIL.
- [ ] **Step 3:** Implement the renderer with fixed palettes and no timestamps/metadata that vary between runs.
- [ ] **Step 4:** Re-run; expect PASS. Generate the default-seed preview and commit it under `docs/previews/phase1/`.
- [ ] **Step 5:** Commit `feat(preview): render island geophysics headlessly`.

### Task 6: Phase-1 acceptance fixture

**Files:**
- Create: `fixtures/island/default_profile.json`
- Create: `crates/mk_engine/tests/island_phase1_acceptance.rs`
- Modify: `UPSTREAM.md`

**Interfaces:**
- Consumes all Phase-1 public interfaces; produces no new runtime API.

- [ ] **Step 1:** Add a fixture with the exact default profile values and profile version `1`.
- [ ] **Step 2:** Add an acceptance test that bootstraps geophysics twice from `[7u8;32]` and asserts identical serialized results, area/buffer/connectivity constraints, and finite tectonic/volcanic state.
- [ ] **Step 3:** Run `cargo test -p mk_engine --test island_phase1_acceptance -- --nocapture`; expect PASS.
- [ ] **Step 4:** Update `UPSTREAM.md` with `mk_island` and regional tectonic/terrain divergences.
- [ ] **Step 5:** Run `cargo fmt --all -- --check` and Phase-1 focused tests; expect 0 failures.
- [ ] **Step 6:** Commit `test(island): lock regional geophysics acceptance`.
- [ ] **Step 7:** Gate 1 review: the owner looks at `docs/previews/phase1/elevation.png` before Phase 2 starts. A landmass that is wrong in kind (shape, relief, coast) is fixed here, not after climate is built on it.
