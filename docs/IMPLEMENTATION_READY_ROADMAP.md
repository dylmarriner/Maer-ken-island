# Maer-Ken Island — Implementation-Ready Roadmap

**Status:** Approved execution roadmap

**Purpose:** This file is the top-level execution map for building Maer-Ken Island. It does not replace the detailed phase plans. It defines the locked scope, dependency order, implementation contract, acceptance gates, and the exact plan files agents must follow.

## 1. Locked product scope

Build a **single New-Zealand-scale Maer-Ken island** surrounded by ocean, not a full rendered planet.

The island project must retain the depth of the Maer-Ken simulation while bounding detailed simulation to one regional domain:

- approximately `268,000 km²` of contiguous primary land;
- ocean on every side;
- minimum `300 km` coastline-to-domain-edge ocean buffer;
- deterministic tectonics, terrain, volcanism and geology;
- ocean, tides, atmosphere, climate, weather and hydrology;
- soils, vegetation, ecology, resources and material economy;
- canonical Maer-Ken humans and their full runtime;
- Gem-D and Gem-K as canonical founders;
- the current house, shed, rooms, computers, tools, vehicles, stored items and related property systems;
- persistence, save/load, replay, hashing and deterministic scheduling;
- a regional map/free-camera UI with inspectors;
- no normal-runtime dependency on unrelated whole-planet grids.

This is **not** a reduced toy simulation. The reduction is spatial scope, not simulation depth.

## 2. Source authority and non-negotiable constraints

1. `dylmarriner/Maer-Ken` remains the behavioural/source authority for imported Maer-Ken systems.
2. The current import pin is Maer-Ken commit `7c05f0dcf254387ffd7322dbb525fe4807228602` until explicitly changed through the upstream-sync process.
3. Gem-D and Gem-K must use the canonical imported Maer-Ken human runtime. Do not create an island-only fork of cognition, biology, reproduction, lifecycle, memory or social behaviour.
4. Same canon + seed + scenario + actions + build must produce the same deterministic world/replay hash.
5. Deterministic state may not depend on wall-clock time, hidden network input, unordered iteration or renderer state.
6. Simulation coordinates are regional Cartesian/metre coordinates. Do not rebuild detailed world state as a longitude/latitude whole-planet raster.
7. Presentation state is one-way. Bevy transforms, camera state and UI state must never feed back into deterministic simulation state.
8. Invalid regional profiles fail before large allocations.
9. Regional east/west boundaries must not accidentally inherit planetary wrapping.
10. Save/load and replay must reject incompatible canon/profile/schema versions rather than silently normalising them.
11. Do not add mocks, stubs or TODO implementations in place of required runtime behaviour.
12. A phase is not complete because code exists. It is complete only when its tests, deterministic checks and gate criteria pass.

## 3. Regional architecture

The regional world is built around these concepts:

- `IslandDomain`: dimensions, coordinate conversion, grid resolution, coastline/buffer limits and regional boundary contract.
- Regional geophysics: deterministic plates/stress, terrain, uplift/subsidence, erosion and volcanism projected into the bounded island domain.
- Regional water/air systems: ocean, atmosphere, weather, hydrology and tides with explicit boundary forcing rather than planetary wraparound.
- Regional living world: ecology, vegetation, resources, property and canonical humans mapped onto regional cells.
- `IslandWorldState`: owns the retained simulation state without bootstrapping unrelated whole-planet state.
- Deterministic scheduler: explicit cadence/order for coupled systems.
- Snapshot/replay layer: versioned persistence, state hashing and deterministic replay.
- `IslandView`: read-only presentation projection of simulation state.
- Bevy app: regional terrain/ocean/life/property rendering and inspectors, with headless simulation remaining independent of display initialisation.

Initial design baseline:

- regional domain: approximately `2,400 km × 1,920 km`;
- at least `300 km` ocean buffer around the generated coastline;
- coarse grid baseline: `24 km` cells;
- medium grid baseline: `8 km` cells;
- high-detail local patches only where required;
- target land area acceptance window: `254,600..=281,400 km²` (±5% around 268,000 km²).

Grid values are implementation baselines, not sacred numbers. Change them only from measured performance/fidelity evidence in the performance phase.

## 4. Execution order

The project is intentionally sequential. Each phase establishes contracts consumed by the next.

### Phase 1 — Domain and geophysics

**Execution plan:** `docs/superpowers/plans/2026-10-01-island-domain-geophysics.md`

Deliver:

- validated `IslandDomain` and regional profile;
- deterministic regional coordinate conversion;
- explicit non-wrapping regional boundaries;
- one contiguous primary island;
- land-area fitting around the NZ-scale target;
- 300 km minimum ocean buffer;
- deterministic tectonics/geology/terrain/volcanism;
- bounded coarse/medium grid allocation;
- tests proving repeatability, geometry and buffer/area constraints.

**Gate 1:** fixed-seed generation is deterministic, produces one valid island within target-area tolerance, preserves the ocean buffer and does not allocate unrelated planetary world grids.

### Phase 2 — Ocean, atmosphere and hydrology

**Execution plan:** `docs/superpowers/plans/2026-10-01-island-water-atmosphere.md`

Deliver:

- regional ocean state;
- atmosphere/climate/weather state;
- hydrology, runoff, rivers/lakes/groundwater as defined by the detailed plan;
- tides and relevant Maer-Ken astronomical forcing;
- explicit edge/boundary exchange;
- deterministic coupling to Phase-1 elevation/geology;
- conservation/sanity checks appropriate to the retained models.

**Gate 2:** local water/air systems couple deterministically, use explicit regional boundary forcing, and contain no accidental longitude-style wraparound.

### Phase 3 — Ecology, resources, property and humans

**Execution plan:** `docs/superpowers/plans/2026-10-01-island-life-property-humans.md`

Deliver:

- vegetation and ecology on regional cells;
- soils/resources/material economy required by retained systems;
- canonical property inventory;
- house, shed and room structure;
- tools, stored items, computers/network-account representations and vehicles defined by the imported/current property model;
- Gem-D and Gem-K using the canonical Maer-Ken human runtime;
- human/environment/property interactions mapped to regional coordinates;
- deterministic local placement and interaction tests.

**Gate 3:** retained ecology, resources, founders and property systems operate on regional state without a parallel island-only human model.

### Phase 4 — Runtime, scheduling, persistence and replay

**Execution plan:** `docs/superpowers/plans/2026-10-01-island-runtime-persistence.md`

Deliver:

- composed `IslandWorldState`;
- deterministic simulation scheduler and subsystem cadence;
- world stepping without renderer ownership;
- versioned save/load;
- deterministic state hashing;
- replay/action journal;
- compatibility rejection for mismatched profile/canon/schema versions;
- fixed-seed deterministic replay verification.

**Gate 4:** two identical fixed-seed runs with the same scenario/actions end in byte-equivalent or specified hash-equivalent state, save/load round-trips correctly, and scheduler cadence/order is covered by tests.

### Phase 5 — App, UI, performance and pruning

**Execution plan:** `docs/superpowers/plans/2026-10-01-island-app-ui-performance.md`

Deliver:

- `IslandView` read-only presentation projection;
- regional terrain and surrounding ocean rendering;
- free/map camera, not a planetary globe renderer;
- humans, vegetation, buildings, property and vehicles rendering;
- inspectors for world, terrain, humans, property and environment;
- pause/resume, deterministic single-step and save controls;
- repeatable benchmark command and committed baseline;
- measured allocation/performance evidence;
- upstream-sync tooling;
- removal of planetary-only runtime dependencies proven unnecessary;
- final system status documentation.

**Gate 5:** headless simulation remains independent of UI, retained world state is inspectable/rendered, benchmark evidence is recorded, and normal island execution no longer bootstraps whole-planet-only runtime state.

## 5. Agent execution contract

Agents implementing this roadmap must:

1. Read this roadmap first.
2. Read `docs/superpowers/plans/2026-10-01-maer-ken-island-program-plan.md`.
3. Read the detailed plan for the active phase in full before editing code.
4. Work phase-by-phase and task-by-task. Do not skip dependency gates.
5. Preserve deterministic behaviour and source provenance.
6. Write or update tests before declaring behaviour complete.
7. Run focused tests during each task, then the phase gate suite.
8. Commit in small coherent units matching the detailed plan where practical.
9. Never describe planned behaviour as implemented until verification proves it.
10. Record any intentional deviation from the detailed plan in this roadmap or the relevant plan before relying on it downstream.

## 6. Definition of done for every implementation task

A task is done only when:

- required source files exist and contain production implementation;
- no placeholder path is being relied on;
- focused tests pass;
- deterministic behaviour is preserved;
- error handling fails visibly rather than silently corrupting state;
- imported Maer-Ken behaviour remains attributable;
- relevant docs are updated when interfaces/contracts changed;
- the repository builds for the affected targets.

## 7. Final repository acceptance

Before the project can be called complete, all five phase gates must be green and the following final checks must pass:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace -- --test-threads=1
```

Also required:

- fixed-seed replay executed twice with identical final hashes;
- snapshot save/load round-trip verified;
- measured benchmark output committed under `benchmarks/`;
- final `UPSTREAM.md` import/divergence record updated;
- final `docs/ISLAND_SYSTEM_STATUS.md` records implemented systems, benchmark numbers, verification commands and known fidelity limits;
- README reflects the regional island application as the project entry point once Phase 5 replaces the initial human-only bootstrap.

## 8. Canonical detailed plan set

The detailed implementation instructions live here:

1. `docs/superpowers/plans/2026-10-01-island-domain-geophysics.md`
2. `docs/superpowers/plans/2026-10-01-island-water-atmosphere.md`
3. `docs/superpowers/plans/2026-10-01-island-life-property-humans.md`
4. `docs/superpowers/plans/2026-10-01-island-runtime-persistence.md`
5. `docs/superpowers/plans/2026-10-01-island-app-ui-performance.md`

Program-level contract:

- `docs/superpowers/plans/2026-10-01-maer-ken-island-program-plan.md`

Design authority:

- `docs/superpowers/specs/2026-10-01-maer-ken-island-regional-world-design.md`

## 9. Roadmap maintenance rule

This file should remain concise enough to orient a new coding agent without forcing the full detailed plan set into context at once.

When implementation changes the order, interface contract or acceptance gate, update this roadmap and the affected detailed phase plan in the same change. Routine task-level progress belongs in the detailed plan/checklist or status document, not by expanding this file into another implementation dump.
