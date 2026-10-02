# Maer-Ken Island — Implementation-Ready Roadmap

**Status:** Approved execution roadmap (updated 2026-10-02 to match the detailed plan set)

**Purpose:** This file is the top-level execution map for building Maer-Ken Island. It does not replace the detailed phase plans. It defines the locked scope, dependency order, implementation contract, acceptance gates, and the exact plan files agents must follow.

## 1. Locked product scope

Build a **single New-Zealand-sized Maer-Ken island** surrounded by ocean, not a full rendered planet, as a **realistic simulation of a planet and its people**.

The island project must retain the depth of the Maer-Ken simulation while bounding detailed simulation to one regional domain:

- approximately `268,000 km²` of contiguous primary land with a **unique, procedurally generated shape** (irregular coastline with headlands and bays; not New Zealand's outline, not a blob), chosen by the owner from a seed gallery;
- ocean on every side, with a minimum `300 km` coastline-to-domain-edge buffer;
- the planet is **Earth-like Marr'Kena**: Marr'Kena's 19,113 km radius, 36-hour day, 27° tilt and two moons, with Earth gravity, atmosphere and sunlight, and orbit, year (288 local days) and outer-moon period recomputed for physical consistency;
- deterministic tectonics, earthquakes, terrain, volcanism, rock types and mineral deposits (gold, silver, platinum, base metals, gems including diamonds, crystals, stone, industrial minerals, salts, coal, oil and gas);
- ocean, tides, atmosphere, climate with a real day/night cycle, travelling weather systems, and hydrology with real rivers;
- soils, vegetation (individual trees around the estate), ecology, every resource as a node and every material as an item, with carbon, oxygen and water moving physically through gathering, crafting, burning and eating;
- canonical Maer-Ken humans and their full runtime; Gem-D and Gem-K as founders; every human stored in their own folder; a dashboard **Human Creator**;
- the founders' estate (house with the computer room inside it, shed, workshop, armoury, garage, rooms, computers, tools, vehicles, stored items) laid out in metres, with fuel and electricity;
- a river-side hydro plant powering the house and later a town, an oil and gas field, refinery and gas plant, farms, fishery, supermarket, forestry, sawmill and builders, a founding town of complete humans who work, are paid and buy what they need, and enterprises owned by Gem-D and Gem-K that pay them the profits;
- persistence, save/load, replay, hashing, deterministic scheduling and speed control from real time to as fast as possible;
- a web dashboard and a regional map/free-camera desktop UI with inspectors;
- no normal-runtime dependency on unrelated whole-planet grids.

This is **not** a reduced toy simulation. The reduction is spatial scope, not simulation depth.

## 2. Source authority and non-negotiable constraints

1. `dylmarriner/Maer-Ken` remains the behavioural/source authority for imported Maer-Ken systems.
2. The current import pin is Maer-Ken commit `7c05f0dcf254387ffd7322dbb525fe4807228602` until explicitly changed through the upstream-sync process.
3. Gem-D and Gem-K must use the canonical imported Maer-Ken human runtime. Do not create an island-only fork of cognition, biology, reproduction, lifecycle, memory or social behaviour. Changes to human behaviour or biology (body clock, energy use, work, money, conversation pairing) are made **upstream first** and synced.
4. Same canon + seed + scenario + actions + build must produce the same deterministic world/replay hash, at any simulation speed.
5. Deterministic state may not depend on wall-clock time, hidden network input, unordered iteration, disk success/failure or renderer state.
6. Simulation coordinates are regional Cartesian/metre coordinates. Do not rebuild detailed world state as a longitude/latitude whole-planet raster.
7. Presentation state is one-way. Bevy transforms, camera state and UI state must never feed back into deterministic simulation state.
8. Invalid regional profiles fail before large allocations.
9. Regional east/west boundaries must not accidentally inherit planetary wrapping.
10. Save/load and replay must reject incompatible canon/profile/schema versions rather than silently normalising them.
11. Do not add mocks, stubs or TODO implementations in place of required runtime behaviour.
12. A phase is not complete because code exists. It is complete only when its tests, deterministic checks and gate criteria pass.
13. **Realism standard** (`docs/island/REALISM.md`, Phase 0c): every parameter cites a source or derivation; every system is validated against real-world reference data scaled for the planet; every remaining simplification is recorded in `docs/island/DEVIATIONS.md`; no free regeneration, instant actions, infinite fuel or unpowered machines.
14. Carbon, oxygen, water, methane and money ledgers close every step.

## 3. Regional architecture

The regional world is built around these concepts:

- Island canon: the Earth-like Marr'Kena canon file with a physical-consistency validator (upstream canon unchanged).
- `IslandDomain`: dimensions, coordinate conversion, grid levels, active land cells, coastline/buffer limits and regional boundary contract (`mk_island`, data-only, depends only on `mk_core`).
- Regional geophysics (`mk_engine::regional`): plates, faults and earthquakes, terrain, uplift/subsidence, volcanism, rock types and mineral deposits.
- Regional water/air systems: zonal background forcing, ocean, atmosphere with diurnal cycle and synoptic systems, hydrology and tides with explicit boundary forcing rather than planetary wraparound.
- Regional living world: ecology, vegetation, materials and resources, property and canonical humans on a non-wrapping grid topology, with timed, energy-costed work.
- High-detail patches: the estate (4 km × 4 km at 5 m), the town and the refinery site, holding buildings, rooms, items, individual trees and positions in metres.
- `IslandWorldState`: owns all retained simulation state without bootstrapping unrelated whole-planet state.
- Deterministic scheduler: explicit cadence/order for coupled systems; speed control changes how often fixed steps run, never their size.
- Snapshot/replay layer: versioned persistence, canonical state hashing, external-command replay, per-human folders.
- Industry and economy: hydro and grid, petroleum, refinery, food and timber chains, jobs and trades, double-entry money, enterprises.
- `IslandProjection`/`IslandView`: read-only presentation projections for the web dashboard and the Bevy app.
- Bevy app (`apps/island_ui`, Bevy 0.19): regional terrain/ocean/life/property/industry rendering and inspectors; the headless binary (`apps/island`) never links Bevy.

Design baseline:

- regional domain `2,400 km × 1,920 km`;
- at least `300 km` ocean buffer around the generated coastline;
- coarse grid `12 km` (atmosphere/ocean, `160 × 200`), medium grid `2 km` (terrain/rivers/ecology/resources, `960 × 1,200`), with medium processes iterating land plus a 20 km coastal band only;
- documented fallback `24 km` / `4 km` only if Phase-5 benchmarks require it;
- target land area acceptance window `254,600..=281,400 km²` (±5% around 268,000 km²).

Grid values are implementation baselines, not sacred numbers. Change them only from measured performance/fidelity evidence in the performance phase.

## 4. Execution order

The project is intentionally sequential. Each phase establishes contracts consumed by the next.

### Phase 0 — Green baseline

**Execution plan:** `docs/superpowers/plans/2026-10-01-island-phase0-baseline.md`

Deliver: the five inherited test failures fixed; slow and deep test tiers; pinned toolchain; CI; storage-key and bridge-retry fixes; humans never lost to a disk error; realistic O(n log n) conversation pairing and an indexed registry; upstream drift check.

**Gate 0:** fmt, clippy `-D warnings`, the fast tier and the slow tier are green and enforced by CI.

### Phase 0b — Early Human Creator

**Execution plan:** `docs/superpowers/plans/2026-10-01-island-phase0b-early-human-creator.md`

Deliver: `island serve` web dashboard; Human Creator built on upstream's `SpawnHuman` path; every human in their own folder; restart reloads everyone.

**Gate 0b:** the owner creates a human in the browser; they appear in the roster, have their own folder with a `created` event, and survive a restart.

### Phase 0c — Island canon and realism standard

**Execution plan:** `docs/superpowers/plans/2026-10-02-island-phase0c-canon-and-realism.md`

Deliver: Earth-like Marr'Kena canon and consistency validator; realism standard and deviation register; cited real-world reference packs; human realism validation (including an intrinsic ~24.2 h body clock), fixed upstream first.

**Gate 0c:** the island canon passes the validator with only the declared density exception; reference packs load; human realism results pass or are accepted by the owner.

### Phase 1 — Domain and geophysics

**Execution plan:** `docs/superpowers/plans/2026-10-01-island-domain-geophysics.md`

Deliver: validated `IslandDomain` and profile; non-wrapping boundaries; tectonics and earthquakes; volcanism; one uniquely shaped island fitted to the area target inside the buffer; rock types and mineral deposits; headless preview tool and seed gallery (shape, deposits, later hydro and petroleum potential); the owner's chosen seed pinned.

**Gate 1:** fixed-seed generation is deterministic, produces one valid island within area tolerance and shape requirements, preserves the ocean buffer, has geologically placed deposits, and allocates no unrelated planetary grids.

### Phase 2 — Ocean, atmosphere and hydrology

**Execution plan:** `docs/superpowers/plans/2026-10-01-island-water-atmosphere.md`

Deliver: planetary-dependency inventory; zonal background forcing; regional climate with diurnal cycle and orographic rain; synoptic weather systems; ocean and tides; hydrology with real rivers; coupled physical tick; year-long realism test against reference data.

**Gate 2:** local water/air systems couple deterministically with explicit boundary forcing, no wraparound, a calibrated zonal background, and climate/hydrology statistics within reference tolerances.

### Phase 3 — Ecology, resources, property and humans

**Execution plan:** `docs/superpowers/plans/2026-10-01-island-life-property-humans.md`

Deliver: ecology and prognostic biomass; complete resource nodes and material catalogue; physical carbon/oxygen/water flows; timed, energy-costed work; estate placement by a hydro-capable river; estate fuel and electricity; metric estate layout (computer room inside the house); grid topology for humans; individual trees; founders on the island with observation and computer access gated by room and power.

**Gate 3:** ecology, resources, founders and property operate on regional state without a parallel human model, and carbon, oxygen and water budgets close every step.

### Phase 4 — Runtime, scheduling, persistence and replay

**Execution plan:** `docs/superpowers/plans/2026-10-01-island-runtime-persistence.md`

Deliver: `IslandWorldState`; cadence-aware scheduler; complete snapshots; per-human folders on a sync cadence; external-command replay; headless runner; speed control (real time to as fast as possible); dashboard and Human Creator on the live island.

**Gate 4:** two identical fixed-seed runs end in identical canonical hashes at any speed, save/load round-trips, every human has a folder, and the owner can create a human from the dashboard and see them in the world and on disk.

### Phase 4b — Hydro, petroleum, town and economy

**Execution plan:** `docs/superpowers/plans/2026-10-02-island-phase4b-energy-industry-town.md`

Deliver: river hydro for the estate and town; island grid; oil and gas field; refinery and gas plant; jobs, trades and shifts (upstream first); money, enterprises, payroll every 14 roster days and dividends to Gem-D and Gem-K; food and timber supply chains; founding town and workforce; Power, Petroleum, Town, Food, Timber, Payroll and Finance dashboard pages.

**Gate 4b:** for one simulated local year the estate and town run on hydro, the refinery makes the island's fuel, workers keep the plants running and are paid, the town is fed and housed from its own production, every ledger closes, and Gem-D and Gem-K receive dividends equal to their shares of profit.

### Phase 5 — App, UI, performance and pruning

**Execution plan:** `docs/superpowers/plans/2026-10-01-island-app-ui-performance.md`

Deliver: `island-ui` on Bevy 0.19; `IslandView`; regional terrain and ocean with real sun, moons and weather; humans, vegetation, property, resources, industry and town rendering; human inspector, detail views and foundry; opt-in computer service; inspectors and controls including speed; benchmark with resolution gate; upstream-sync tooling; retirement of `apps/island_humans`; removal of planetary-only runtime; final status documentation.

**Gate 5:** headless simulation remains independent of the UI, retained world state is inspectable and rendered, benchmark evidence is recorded and meets the targets (or the fallback is recorded), and normal island execution no longer bootstraps whole-planet-only runtime state.

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
10. Record any intentional deviation from the detailed plan in this roadmap or the relevant plan before relying on it downstream, and any departure from reality in `docs/island/DEVIATIONS.md`.

## 6. Definition of done for every implementation task

A task is done only when:

- required source files exist and contain production implementation;
- no placeholder path is being relied on;
- focused tests pass;
- deterministic behaviour is preserved;
- error handling fails visibly rather than silently corrupting state;
- imported Maer-Ken behaviour remains attributable;
- new parameters cite a source or derivation;
- relevant docs are updated when interfaces/contracts changed;
- the repository builds for the affected targets.

## 7. Final repository acceptance

Before the project can be called complete, all nine phase gates (0, 0b, 0c, 1, 2, 3, 4, 4b, 5) must be green and the following final checks must pass:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo test --workspace --release -- --ignored slow_
```

Also required:

- fixed-seed replay executed twice with identical final hashes;
- snapshot save/load round-trip verified;
- measured benchmark output committed under `benchmarks/`;
- realism validation results recorded and every open deviation listed in `docs/island/DEVIATIONS.md`;
- final `UPSTREAM.md` import/divergence record updated;
- final `docs/ISLAND_SYSTEM_STATUS.md` records implemented systems, benchmark numbers, verification commands and known fidelity limits;
- README reflects `apps/island` and `apps/island_ui` as the project entry points once Phase 5 replaces the initial human-only bootstrap.

## 8. Canonical detailed plan set

The detailed implementation instructions live here, in execution order:

0. `docs/superpowers/plans/2026-10-01-island-phase0-baseline.md`
0b. `docs/superpowers/plans/2026-10-01-island-phase0b-early-human-creator.md`
0c. `docs/superpowers/plans/2026-10-02-island-phase0c-canon-and-realism.md`
1. `docs/superpowers/plans/2026-10-01-island-domain-geophysics.md`
2. `docs/superpowers/plans/2026-10-01-island-water-atmosphere.md`
3. `docs/superpowers/plans/2026-10-01-island-life-property-humans.md`
4. `docs/superpowers/plans/2026-10-01-island-runtime-persistence.md`
4b. `docs/superpowers/plans/2026-10-02-island-phase4b-energy-industry-town.md`
5. `docs/superpowers/plans/2026-10-01-island-app-ui-performance.md`

Program-level contract, owner decisions and test tiers:

- `docs/superpowers/plans/2026-10-01-maer-ken-island-program-plan.md`

Design authority:

- `docs/superpowers/specs/2026-10-01-maer-ken-island-regional-world-design.md`

## 9. Roadmap maintenance rule

This file should remain concise enough to orient a new coding agent without forcing the full detailed plan set into context at once.

When implementation changes the order, interface contract or acceptance gate, update this roadmap and the affected detailed phase plan in the same change. Routine task-level progress belongs in the detailed plan/checklist or status document, not by expanding this file into another implementation dump.
