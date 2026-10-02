# Island Domain & Geophysics Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Introduce a deterministic regional spatial domain and generate one NZ-scale, uniquely shaped tectonic island with volcanism, bathymetry, rock types and geologically formed mineral deposits (metals, gems, crystals, stone, fuels, salts) inside a guaranteed surrounding-ocean buffer, and add the headless preview tool every later phase extends.

**Architecture:** A small `mk_island` crate owns pure geometry/profile/forcing *data* contracts and depends only on `mk_core` (plus serde). `mk_engine::regional` depends on `mk_island` and owns everything that needs engine code: astronomy sampling, tectonics, volcanism and terrain. The dependency direction is fixed: `mk_engine → mk_island → mk_core`; `mk_island` must never depend on `mk_engine`. Existing global functions remain intact until final pruning so imported tests continue to protect upstream behaviour.

**Tech Stack:** Rust 2021, `mk_core`, `mk_engine`, serde, serde_json, blake3, rand_chacha; preview tool adds `png = "0.17"` and `hex` (both added to `[workspace.dependencies]`) and hand-rolled argument parsing (no `clap`).

**Spec:** `docs/superpowers/specs/2026-10-01-maer-ken-island-regional-world-design.md`

**Depends on:** Phase 0 green baseline and Phase 0c island canon.

## Global Constraints

- Default land target `268,000 km^2`, acceptance tolerance ±5% (`254,600..=281,400 km²`).
- Minimum coastline-to-edge ocean buffer `300,000 m` on north/south/east/west edges.
- Initial benchmark profile dimensions `2,400,000 m` (east–west) × `1,920,000 m` (north–south); coarse cell `12,000 m` (atmosphere/ocean, comparable to regional climate models); medium cell `2,000 m` (terrain, rivers, ecology, resources — fine enough for real catchments and valleys; the island has ~67,000 land cells); reference latitude `-41.0°` and reference longitude `174.0°` at the domain centre; all values configurable and validated rather than canon-locked. This yields exactly `160` rows × `200` cols coarse and `960` rows × `1,200` cols medium. If the Phase-5 benchmark shows the default cannot run in real time on the reference machine, the documented fallback is `24,000 m` / `4,000 m` (`80 × 100` / `480 × 600`); any change must preserve the profile contract and acceptance tests. The planet canon is the island canon from Phase 0c.
- **Orientation:** row 0 is the southern edge and latitude increases with row (matching upstream `GridSpec::lat_rad`, where row 0 is the south pole, and weather's `row + 1` = north); col 0 is the western edge and longitude increases with col. `latitude_rad_for_row` is linear in metres about the reference latitude; `longitude_rad_for_col` likewise about the reference longitude, scaled by `cos(reference latitude)`.
- All regional grids are constructed with explicit `GridSpec::new(rows, cols)` (`GridSpec::new(nlat, nlon)`). Do not call `TectonicsState::new(width, height)`: it passes its arguments to `GridSpec::new` as `(nlat, nlon)`.
- No periodic east/west wrap in regional neighbour queries.
- Boundary forcing is deterministic from seed, canon digest and simulation time.
- Existing `mk_core::grid::Grid2<T>` remains the storage container; regional geometry lives in `IslandDomain`, not in fake spherical `GridSpec` methods. Upstream code that calls `GridSpec::lat_rad`, `cell_area_at_row_m2` or wraps longitude must not be fed regional grids directly.
- The island must have a distinctive, irregular, procedurally generated shape — not a blob, ellipse or circle, and not New Zealand's outline. No real-world coastline data may be used as a template. Shape quality is enforced by measurable metrics (Task 4) and the default seed is chosen by the owner from a preview gallery (Task 5); shape requirements never relax the area, connectivity or buffer constraints.

## Review Focus

- Zero/negative extent, cell size, target land area, or ocean buffer must return `IslandProfileError` before allocation.
- Domain dimensions not divisible by configured cell sizes must be rejected rather than truncated.
- Same profile + seed must generate byte-identical boundary and elevation state.
- Disconnected satellite land must not survive the final primary-island fitting pass.
- No land cell may exist within the configured edge-buffer band.

---

### Task 1: Regional profile and coordinate contract

**Files:**
- Create: `crates/mk_island/Cargo.toml` (deps: `mk_core`, `serde`, `serde_json`; no `mk_engine`)
- Create: `crates/mk_island/src/lib.rs`
- Create: `crates/mk_island/src/profile.rs`
- Create: `crates/mk_island/src/domain.rs`
- Modify: `Cargo.toml` (workspace member; add `png = "0.17"` to `[workspace.dependencies]` — `hex` is already there)
- Test: `crates/mk_island/tests/domain_contract.rs`

**Interfaces:**
- Produces: `IslandProfile { version: u32, width_m, height_m, coarse_cell_m, medium_cell_m, reference_latitude_deg, reference_longitude_deg, target_land_area_m2, land_area_tolerance_fraction, minimum_ocean_buffer_m, shape: ShapeRequirements }` with `Serialize`/`Deserialize`; `IslandProfile::default_nz_scale() -> Self` (`version = 1`); `IslandProfile::validate() -> Result<(), IslandProfileError>`; `IslandProfile::load(path: &Path) -> Result<Self, IslandProfileError>` which rejects unknown `version`.
- Produces: `GeologyProfile { ancient_basement: bool, basement_age_ma: f64, basement_area_fraction: f64 }` as `IslandProfile::geology`, defaults `true`, `2_700.0`, `0.08` — a fragment of Archean continental crust so that kimberlite pipes (and therefore diamonds) are geologically possible (Clifford's rule: diamondiferous kimberlites occur on cratons older than ~2.5 Ga). With `ancient_basement = false` the island is a young arc and has no primary diamonds, as a real young arc would not.
- Produces: `ShapeRequirements { max_compactness: f64, max_convexity: f64, min_major_headlands: u32, min_major_bays: u32 }`, defaults `0.30`, `0.80`, `3`, `3` (initial values; tune from the gallery). Validation rejects non-finite or out-of-range values.
- Produces: `IslandDomain::from_profile(profile: IslandProfile) -> Result<Self, IslandDomainError>`.
- Produces: `DomainLevel::{Coarse, Medium}`, `IslandDomain::{rows, cols}(level) -> usize`, `storage_spec(level) -> mk_core::grid::GridSpec` (`GridSpec::new(rows, cols)`), `cell_size_m(level) -> f64`, `cell_center_m(level, row, col) -> (f64, f64)`, `cell_area_m2(level) -> f64`, `latitude_rad_for_row(level, row) -> f64`, `longitude_rad_for_col(level, col) -> f64`, `lat_lon_at_m(x_m, y_m) -> (f64, f64)` (for positions inside cells, e.g. estate metres), `is_edge_buffer_cell(level, row, col) -> bool`, and `active_cells(level) -> &[(usize, usize)]` — set after Task 4's land mask is known to land cells plus a coastal band of `coastal_band_m` (default 20 km) on Medium, and all cells on Coarse.
- Produces: `LocalPatchSpec { origin_x_m, origin_y_m, width_m, height_m, cell_size_m, rows, cols }` and `IslandDomain::local_patch(center_x_m, center_y_m, extent_m, cell_size_m) -> Result<LocalPatchSpec, IslandDomainError>` for high-detail property/interior/navigation windows without allocating a world-wide fine grid. A patch may span several medium cells; it must lie fully inside the domain.

- [ ] **Step 1:** Write `domain_contract.rs` tests asserting default numeric values, `160 × 200` / `960 × 1,200` dimensions, the fallback profile's `80 × 100` / `480 × 600`, flat cell area, row 0 = southernmost latitude, latitude/longitude at the centre equal the reference values, coordinate round-trip, bounded local-patch geometry, profile JSON round-trip, unknown `version` rejected, and all invalid profile/local-patch cases.
- [ ] **Step 2:** Run `cargo test -p mk_island --test domain_contract`; expect compile/test failure because the crate/types do not exist.
- [ ] **Step 3:** Add the workspace member and implement the profile/domain interfaces without changing `Grid2` storage semantics.
- [ ] **Step 4:** Re-run `cargo test -p mk_island --test domain_contract`; expect PASS.
- [ ] **Step 5:** Commit `feat(island): add regional domain contract`.

### Task 2: Deterministic regional boundary forcing

**Files:**
- Create: `crates/mk_island/src/boundary.rs` (forcing data types only)
- Modify: `crates/mk_island/src/lib.rs`
- Create: `crates/mk_engine/src/regional/mod.rs`
- Create: `crates/mk_engine/src/regional/boundary.rs` (sampling)
- Modify: `crates/mk_engine/src/lib.rs` (`pub mod regional;`)
- Modify: `crates/mk_engine/Cargo.toml` (add `mk_island = { path = "../mk_island" }`)
- Test: `crates/mk_engine/tests/regional_boundary_determinism.rs`

**Interfaces:**
- Produces (in `mk_island::boundary`, serde data only): `OceanBoundaryForcing`, `AtmosphereBoundaryForcing`, `TectonicBoundaryForcing`, `AstronomyForcing { solar_declination_rad, sub_solar_longitude_rad, moon_sub_longitudes_rad: Vec<f64>, season_phase, … }`, `RegionalBoundaryState { ocean, atmosphere, tectonic, astronomy, sim_time_seconds }`.
- Produces (in `mk_engine::regional::boundary`): `sample_regional_boundaries(seed: [u8; 32], canon: &CanonLocked, domain: &IslandDomain, sim_time_seconds: f64) -> RegionalBoundaryState`. Astronomy comes from the existing `orbit` and `rotation` modules (analytic). Ocean/atmosphere fields in Phase 1 are seed-derived placeholders flagged `provisional: true`; Phase 2 Task 2 replaces them with zonal-background-derived values through a second function rather than changing this signature.

- [ ] **Step 1:** Write tests asserting identical samples for identical inputs, changed samples for changed seed/time, finite forcing values on every edge, declination matching `orbit`/`rotation` at the same time, and no wall-clock fields.
- [ ] **Step 2:** Run `cargo test -p mk_engine --test regional_boundary_determinism`; expect FAIL.
- [ ] **Step 3:** Implement sampling with named blake3-derived deterministic streams and analytic canonical astronomy; store only serializable forcing data.
- [ ] **Step 4:** Re-run the test; expect PASS. Confirm `cargo tree -p mk_island` contains no `mk_engine`.
- [ ] **Step 5:** Commit `feat(island): add deterministic regional boundaries`.

### Task 3: Regional tectonics and non-wrapping neighbours

**Files:**
- Create: `crates/mk_engine/src/regional/tectonics.rs`
- Modify: `crates/mk_engine/src/regional/mod.rs`
- Test: `crates/mk_engine/tests/regional_tectonics.rs`

**Interfaces:**
- Consumes: `IslandDomain`, `TectonicBoundaryForcing` from Tasks 1–2.
- Produces: `step_regional_tectonics(canon: &CanonLocked, tick: Tick, domain: &IslandDomain, forcing: &TectonicBoundaryForcing) -> TectonicsState` on the coarse grid.
- Produces: `regional_neighbours(row, col, rows, cols) -> impl Iterator<Item = (usize, usize)>` (Moore neighbourhood, no wrap).

- [ ] **Step 1:** Write tests asserting deterministic plate assignment, at least 3 local plates, at least one convergent and one divergent boundary, heat flow finite and within `20..=200` mW/m² (upstream units; default 50), and that west-edge neighbours never include east-edge cells.
- [ ] **Step 2:** Run `cargo test -p mk_engine --test regional_tectonics`; expect FAIL.
- [ ] **Step 3:** Implement planar deterministic plate seeds/velocities and classify boundaries using regional neighbours plus edge tectonic forcing; reuse `PlateCell`, `PlateType`, `BoundaryType`, `TectonicsState`.
- [ ] **Step 4:** Run `cargo test -p mk_engine --test regional_tectonics` and `cargo test -p mk_engine --lib tectonics::`; expect PASS.
- [ ] **Step 5:** Commit `feat(engine): step tectonics on island domain`.

### Task 3b: Earthquakes and fault slip

The spec promises earthquakes; the engine has none. Earthquakes are the release of the tectonic stress Task 3 accumulates.

**Files:**
- Create: `crates/mk_engine/src/regional/seismicity.rs`
- Test: `crates/mk_engine/tests/regional_seismicity.rs`

**Interfaces:**
- Produces: `FaultSystem { faults: Vec<Fault { id, trace_cells, kind: Thrust | Normal | StrikeSlip, slip_rate_mm_yr, locked_depth_km, stress_mpa }> }` derived from plate boundaries and their relative velocities.
- Produces: `step_seismicity(faults: &mut FaultSystem, dt_seconds, rng: &RngRegistry) -> Vec<Earthquake { fault_id, epicentre_m, depth_km, magnitude_mw, slip_m, rupture_length_km }>`: stress loads at the fault's slip rate; events nucleate when stress exceeds a threshold drawn deterministically; magnitude from rupture area and slip (Wells & Coppersmith 1994 scaling); aftershocks follow Omori–Utsu decay; long-run magnitude–frequency follows Gutenberg–Richter with b ≈ 1.
- Earthquakes feed back into the world: co-seismic uplift/subsidence at the fault (elevation change), landslides on steep slopes (Phase 2 hydrology sediment), shaking intensity per cell (Phase 3 structures and humans).

- [ ] **Step 1:** Write tests: fault slip rates match plate relative velocities; a 10,000-year run's magnitude–frequency has b within 0.8–1.2 and moment release matches the tectonic loading budget within 10%; aftershock rate decays as Omori–Utsu (p ≈ 1); events are deterministic per seed; no event on an aseismic interior.
- [ ] **Step 2:** Run `cargo test -p mk_engine --test regional_seismicity`; expect FAIL; implement; re-run; expect PASS (the 10,000-year run is a slow-tier test).
- [ ] **Step 3:** Commit `feat(engine): earthquakes from regional fault stress`.

### Task 4: Volcanism, one-island terrain, target area and shape

**Files:**
- Create: `crates/mk_engine/src/regional/volcanism.rs`
- Create: `crates/mk_engine/src/regional/terrain.rs`
- Create: `crates/mk_engine/src/regional/shape.rs`
- Create: `crates/mk_engine/src/regional/geophysics.rs`
- Modify: `crates/mk_engine/src/regional/mod.rs`
- Modify: `crates/mk_engine/src/volcanism/mod.rs` (extract cell-geometry/neighbour inputs; see Step 1)
- Test: `crates/mk_engine/tests/regional_geophysics.rs`, `crates/mk_engine/tests/regional_shape.rs`

**Interfaces:**
- Produces: `step_regional_volcanism(canon, tick, domain, tectonics: &TectonicsState, previous_degassed_fraction: f64, dt_seconds: f64, rng: &RngRegistry) -> VolcanismState`.
- Produces: `RegionalGeophysics { tectonics, volcanism, elevation_m, bathymetry_m, land_mask, land_area_m2, sea_level_offset_m, shape: ShapeMetrics }`.
- Produces: `bootstrap_regional_geophysics(canon: &CanonLocked, domain: &IslandDomain, boundaries: &RegionalBoundaryState, seed: [u8; 32]) -> Result<RegionalGeophysics, RegionalGeophysicsError>`; the `RngRegistry` is built internally with `RngRegistry::new(seed)`.
- Produces: `primary_land_component(elevation_m: &Grid2<f64>) -> Grid2<bool>` and `fit_sea_level_to_target(raw_elevation_m: &Grid2<f64>, domain: &IslandDomain, target_land_area_m2: f64, tolerance_fraction: f64, minimum_ocean_buffer_m: f64) -> Result<(Grid2<f64>, f64), RegionalGeophysicsError>`.
- Produces: `ShapeMetrics { compactness, convexity, major_headlands, major_bays, coastline_length_m, principal_axis_ratio }` and `measure_shape(land_mask: &Grid2<bool>, domain: &IslandDomain) -> ShapeMetrics`.
- Error variants include `AreaUnreachable`, `BufferViolated`, `ShapeRequirementsUnmet(ShapeMetrics)`.

- [ ] **Step 1 (volcanism):** Upstream `step_volcanism` derives cell size from planet radius and `lat_rad` (`volcanism/mod.rs:294-298`), wraps longitude with `rem_euclid` (`:310`) and uses `cell_area_at_row_m2` (`:239`). Extract those into a parameter (`VolcanismGeometry { cell_height_km(row), cell_width_km(row), cell_area_m2(row), neighbours(row, col) }`) that the global path fills exactly as today and the regional wrapper fills with flat domain sizes and `regional_neighbours`. Write tests: existing `cargo test -p mk_engine --lib volcanism::` unchanged; regional volcanism places activity near convergent boundaries and never reads across the east/west edge.
- [ ] **Step 2 (raw terrain):** Build raw elevation from the tectonic and volcanic state rather than a template: uplift along convergent boundaries and volcanic arcs, subsidence at divergent margins, multi-octave deterministic noise for coastline detail, and coarse erosion-like smoothing, so peninsulas, bays, ranges and inlets emerge from geology. Medium elevation is coarse uplift bilinearly resampled plus medium-scale noise.
- [ ] **Step 3 (fit):** Fit sea level on flat regional cell area, deterministically retain the largest land component, submerge disconnected land, raise edge-band cells below sea level, and iterate sea level until area tolerance is met without violating the buffer.
- [ ] **Step 4 (shape metrics):** Implement `measure_shape`. Perimeter is the length of the marching-squares coastline contour (not the raster staircase, which inflates perimeter by ~4/π); compactness is `4πA/P²` (circle = 1); convexity is land area / convex-hull area; headlands and bays are counted from convex-hull deficits and protrusions larger than `max(2 coarse cells, 1% of hull area)`.
- [ ] **Step 5 (tests):** Write `regional_shape.rs`: a disc and a 4:1 ellipse fixture both fail `ShapeRequirements` (the ellipse fails on convexity); a synthetic indented fixture passes. Write `regional_geophysics.rs`: land area `254,600..=281,400 km²`, one connected component, ocean in the complete 300 km edge band, negative offshore bathymetry, deterministic elevation bytes, and `ShapeRequirementsUnmet` (with metrics) for a seed that fails. Use a provisional passing seed found by a deterministic search over `[0u8;32]`, `[1u8;32]`, … and recorded as a constant in the test; Task 7 replaces it with the owner's seed.
- [ ] **Step 6:** Run `cargo test -p mk_engine --test regional_geophysics --test regional_shape` and `cargo test -p mk_engine --lib volcanism::`; expect PASS.
- [ ] **Step 7:** Commit `feat(engine): generate nz-scale island geophysics`.

### Task 5: Rock types and mineral deposits

Upstream places every resource by biome and has no rock-type map; gold, silver, gems and crystals do not exist. This task gives the island real geology: a lithology map from the tectonic and volcanic state, and primary mineral deposits that form only where their geological conditions exist. Secondary (river/beach) deposits need Phase-2 rivers and are added in Phase 3 Task 2.

**Files:**
- Create: `crates/mk_engine/src/regional/geology.rs`
- Create: `crates/mk_engine/src/regional/deposits.rs`
- Modify: `crates/mk_engine/src/regional/geophysics.rs` (`RegionalGeophysics` gains `lithology`, `deposits`)
- Modify: `crates/mk_engine/src/regional/mod.rs`
- Test: `crates/mk_engine/tests/regional_geology.rs`

**Interfaces:**
- Produces: `Lithology::{OceanicBasalt, ArcAndesite, Rhyolite, Granite, Pegmatite, Greywacke, Schist, Gneiss, Marble, Limestone, Sandstone, Mudstone, CoalMeasures, Evaporite, Ultramafic, CratonicGneiss, Kimberlite}` and `generate_lithology(tectonics, volcanism, elevation_m, domain, profile, seed) -> Grid2<Lithology>` on Medium. Rules: oceanic crust → basalt; volcanic arcs → andesite/rhyolite; granite plutons behind convergent margins with pegmatite margins; greywacke/schist/gneiss belts in the collision zone by metamorphic grade; marble where limestone meets metamorphism; sedimentary basins (sandstone, mudstone, limestone in shallow marine, coal measures in low wet basins, evaporites in arid basins) on subsiding crust; ultramafic slivers along sutures; the ancient basement fragment as cratonic gneiss; kimberlite pipes only inside cratonic gneiss where lithosphere thickness ≥ 150 km.
- Produces: `DepositKind` and `MineralDeposit { id, kind, cell: (usize, usize), depth_m, tonnage_t, grade, host: Lithology }` (grade in g/t for precious metals, % for base metals and industrial minerals, carats/100 t for diamonds, ct or kg/t for gems and crystals). Required deposit kinds and their formation rules:

| Deposit kind | Yields | Forms in |
|---|---|---|
| Orogenic lode gold (quartz veins) | gold, quartz | Greywacke/Schist belts near convergent boundaries |
| Epithermal gold–silver | gold, silver, cinnabar, quartz, amethyst | ArcAndesite/Rhyolite near volcanic centres |
| Porphyry copper–molybdenum–gold | copper, molybdenum, gold | Granite intrusions in arcs |
| Volcanogenic massive sulfide | copper, zinc, lead, silver, pyrite | OceanicBasalt / submarine volcanics |
| Sediment-hosted lead–zinc | lead (galena), zinc (sphalerite), silver | Limestone/Mudstone basins |
| Tin–tungsten greisen and veins | tin (cassiterite), tungsten (wolframite) | Granite |
| Pegmatite | quartz crystal, feldspar, mica, beryl/emerald, topaz, tourmaline, lithium | Pegmatite |
| Skarn | iron (magnetite), copper, tungsten, garnet | Granite–Limestone contacts |
| Ultramafic | chromite, nickel, platinum-group metals, nephrite jade, serpentine | Ultramafic |
| Metamorphic gems and minerals | garnet, ruby/sapphire (corundum, in marble), kyanite, graphite | Schist/Gneiss/Marble |
| Banded iron formation | iron (hematite/magnetite) | CratonicGneiss |
| Kimberlite | diamond | Kimberlite |
| Volcanic silica and gems | obsidian, opal, agate, chalcedony, amethyst geodes, pumice, zeolite | Rhyolite/ArcAndesite/OceanicBasalt |
| Volcanic sulfur | sulfur | active volcanic centres (fumaroles) |
| Coal | coal | CoalMeasures |
| Petroleum and natural gas | crude oil, natural gas | Mudstone/Sandstone basins above maturation depth |
| Evaporite | rock salt, gypsum | Evaporite |
| Phosphate | phosphate rock | Limestone/Mudstone (marine) |
| Uranium | uraninite | Granite/CratonicGneiss |
| Building and industrial stone | granite, basalt, limestone, marble, sandstone, slate, flint/chert, kaolin clay | the matching lithology at the surface |

- Abundance: each kind's deposit count, size and grade are drawn deterministically from distributions documented in code with a cited source, scaled by the area of suitable host rock — precious metals and gems are rare, building stone is everywhere its rock is. Bauxite (needs tropical laterite weathering) is evaluated in Phase 3 from climate and is expected to be absent at the reference latitude.
- Produces: `generate_primary_deposits(lithology, tectonics, volcanism, domain, profile, seed) -> Vec<MineralDeposit>` (deterministic order by cell, then kind).

- [ ] **Step 1:** Write tests: lithology is deterministic; arc volcanics lie along convergent boundaries; no kimberlite outside cratonic gneiss and none at all with `ancient_basement = false`; every `DepositKind` has a formation rule and appears for at least one of 32 fixed test seeds; no deposit sits in a host rock its rule forbids; gold and diamond deposits are rarer than building-stone deposits by at least two orders of magnitude of tonnage; deposit list is byte-identical for identical inputs.
- [ ] **Step 2:** Run `cargo test -p mk_engine --test regional_geology`; expect FAIL.
- [ ] **Step 3:** Implement the lithology rules and deposit generation; record every abundance distribution's source in code.
- [ ] **Step 4:** Re-run; expect PASS.
- [ ] **Step 5:** Commit `feat(engine): generate island rock types and mineral deposits`.

### Task 6: Headless preview tool and island gallery

**Files:**
- Create: `apps/island_preview/Cargo.toml` (deps: `mk_island`, `mk_engine`, `mk_core`, `serde`, `serde_json`, `hex`, `png`)
- Create: `apps/island_preview/src/lib.rs` (renderers, testable), `apps/island_preview/src/main.rs` (argument parsing)
- Create: `apps/island_preview/src/font.rs` (embedded 5×7 bitmap digit/letter font for gallery labels)
- Create: `fixtures/island/default_profile.json` (`IslandProfile::default_nz_scale()` serialized, with the provisional seed)
- Modify: `Cargo.toml` (workspace member)
- Test: `apps/island_preview/tests/preview_determinism.rs` (calls the lib directly)

**Interfaces:**
- CLI: `island_preview geophysics --profile <path> --seed <64 hex chars> --out <dir>` writes `elevation.png` (hypsometric land, depth-shaded ocean, 300 km buffer outline), `land_mask.png`, `plates.png`, `geology.png` (lithology), `deposits.png` (deposit markers by kind) and `summary.json` (land area km², sea-level offset, component count, min/max elevation, plate count, `ShapeMetrics`, deposit counts and contained tonnage per `DepositKind`).
- CLI: `island_preview gallery --profile <path> --seeds <n> --out <dir>` derives `n` candidate seeds deterministically (`blake3("island-gallery" ‖ index)`), writes `seed-<hex>.png` thumbnails, `gallery.png` (contact sheet with each tile labelled by index using the bitmap font) and `gallery.json` (index → seed hex, metrics, deposit counts per kind — including whether the island has gold, diamonds and each gem — pass/fail reason). Thumbnails mark gold and diamond deposits so the owner can choose an island by its resources as well as its shape.
- Later phases add subcommands (`physical`, `life`, `world`) to the same tool; it never mutates or persists simulation state.

- [ ] **Step 1:** Write tests asserting identical PNG bytes and `summary.json` for the same profile/seed, that `summary.json` area equals `RegionalGeophysics::land_area_m2`, and that `gallery.json` lists failing seeds with reasons.
- [ ] **Step 2:** Run `cargo test -p island_preview`; expect FAIL.
- [ ] **Step 3:** Implement the renderers with fixed palettes and no timestamps or variable PNG metadata.
- [ ] **Step 4:** Re-run; expect PASS. Generate a 24-seed gallery and commit it under `docs/previews/phase1/gallery/`.
- [ ] **Step 5:** Commit `feat(preview): render island geophysics and seed gallery`.
- [ ] **Step 6 (owner):** The owner picks an island from the gallery by index — by shape and by resources (e.g. one with diamonds) — and the chosen seed hex is recorded.

### Task 7: Phase-1 acceptance fixture

**Files:**
- Modify: `fixtures/island/default_profile.json` (owner's seed)
- Modify: `crates/mk_engine/tests/regional_geophysics.rs` (replace provisional seed constant)
- Create: `crates/mk_engine/tests/island_phase1_acceptance.rs`
- Modify: `UPSTREAM.md`

**Interfaces:**
- Consumes all Phase-1 public interfaces; produces no new runtime API.

- [ ] **Step 1:** Set the owner-chosen seed in the fixture (profile `version` stays `1`) and in the geophysics test constant.
- [ ] **Step 2:** Add an acceptance test that loads the fixture, bootstraps geophysics twice and asserts identical serialized results, area/buffer/connectivity/shape constraints, finite tectonic/volcanic state, and the deposit list the owner saw in the gallery.
- [ ] **Step 3:** Run `cargo test -p mk_engine --test island_phase1_acceptance -- --nocapture`; expect PASS.
- [ ] **Step 4:** Generate the chosen island's full preview into `docs/previews/phase1/`.
- [ ] **Step 5:** Update `UPSTREAM.md` with `mk_island`, the volcanism geometry extraction and regional tectonic/terrain divergences.
- [ ] **Step 6:** Run `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings` and the fast test tier; expect 0 failures.
- [ ] **Step 7:** Commit `test(island): lock regional geophysics acceptance`.
- [ ] **Step 8 (Gate 1 review):** The owner looks at `docs/previews/phase1/elevation.png` before Phase 2 starts. A landmass that is wrong in kind (shape, relief, coast) is fixed here, not after climate is built on it.
