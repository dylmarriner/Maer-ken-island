# Maer-Ken Island Program Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the approved full-depth Maer-Ken simulation as a deterministic New-Zealand-scale single-island regional world surrounded by ocean.

**Architecture:** A baseline phase makes the imported workspace green and gated, then five dependent implementation plans replace whole-planet detailed grids with `IslandDomain`, regional boundary forcing, a regional `IslandWorldState`, and a flat/regional UI while retaining Maer-Ken physics, ecology, human, property, resource, persistence, and audit behaviour. Each plan ends in a buildable, independently testable milestone, produces a visible preview of its output, and becomes the input contract for the next.

**Tech Stack:** Rust 2021 workspace, serde/serde_json, blake3, rand_chacha, existing Maer-Ken engine modules, `png` for headless previews, Bevy 0.19 + the matching bevy_egui release (0.42 at time of writing) for the final regional UI.

**Spec:** `docs/superpowers/specs/2026-10-01-maer-ken-island-regional-world-design.md`

## Global Constraints

- Default target land area is approximately `268,000 km^2`; Phase 1 pins acceptance tolerance to ±5% (`254,600..=281,400 km^2`).
- The normal world contains one contiguous primary island and ocean on every side.
- Minimum ocean buffer is `300,000 m` from coastline to every domain edge.
- Detailed simulation coordinates are regional Cartesian/metre-based, not a full longitude/latitude planet raster. Initial coarse/medium resolutions are implementation baselines and may change only from measured Phase-5 benchmark evidence, not convenience.
- Maer-Ken canonical astronomy and physical constants remain authoritative unless explicitly overridden by island profile data.
- Same canon + seed + scenario + actions + build must produce the same state/replay hash.
- Deterministic state must not depend on wall-clock time, hidden network input, or unordered iteration.
- Gem-D and Gem-K use the canonical imported Maer-Ken human runtime; no island-only fork of human cognition/biology is permitted.
- Imported code remains attributable to `dylmarriner/Maer-Ken` at the commit pinned in `UPSTREAM.md` (currently `7c05f0dcf254387ffd7322dbb525fe4807228602`) until an explicit upstream sync changes the pin.
- Normal island execution must not allocate unrelated whole-planet terrain, ocean, atmosphere, biosphere, settlement, or resource grids. Compact 1-D zonal (latitude-band) forcing models permitted by spec §5 are not "planetary grids" and are allowed.
- Every phase ends with a headless, deterministic preview (`island_preview`, introduced in Phase 1) that renders that phase's new state to PNG/JSON so progress is inspectable without waiting for the Phase-5 UI.
- Each task commits on its own once its focused tests pass. Main must never be red: CI from Phase 0 gates every push.

## Review Focus

- Invalid/degenerate regional profiles must fail validation before any grid allocation.
- A generated coastline must never breach the 300 km edge buffer even after target-area fitting.
- Regional edge algorithms must not accidentally retain longitude-style east/west wrapping.
- No regional climate/weather/ocean step may depend on a planet-wide area-weighted mean computed from regional cells; global context comes only from the zonal background forcing.
- Save/load or replay must reject incompatible profile/canon versions rather than silently normalizing them.
- UI/render projection must never feed presentation state back into deterministic simulation state.

---

## Ordered plan set

0. `2026-10-01-island-phase0-baseline.md` — make the imported workspace green, split slow tests, add CI, fix inherited defects, check upstream drift.
0b. `2026-10-01-island-phase0b-early-human-creator.md` — browser dashboard where the owner creates complete humans, each with their own folder; no island or time yet.
1. `2026-10-01-island-domain-geophysics.md` — regional coordinate contract, deterministic boundaries, tectonics/terrain/volcanism, one-island land-area fitting, headless preview tool.
2. `2026-10-01-island-water-atmosphere.md` — dependency inventory, zonal background forcing, regional climate/weather/ocean/hydrology/tides and boundary exchange.
3. `2026-10-01-island-life-property-humans.md` — ecology/vegetation/resources, dense local vegetation, canonical property inventory with a metric estate layout, founders and human interactions.
4. `2026-10-01-island-runtime-persistence.md` — regional world composition, scheduler, deterministic replay, save/load, state hashing, per-human folders, and the web dashboard with the Human Creator.
5. `2026-10-01-island-app-ui-performance.md` — regional Bevy application, assets/inspectors, benchmarks, upstream sync and planetary-code pruning.

## Program gates

- [ ] **Gate 0:** `cargo test --workspace` (fast set) passes with 0 failures, the slow set passes in release, and CI enforces fmt/clippy/fast tests on every push.
- [ ] **Gate 0b:** The owner creates a human in the browser dashboard; they appear in the roster, have their own folder with a `created` event, and survive a restart.
- [ ] **Gate 1:** Phase 1 tests prove deterministic one-island generation within land-area tolerance and ocean-buffer constraints; `island_preview` emits the elevation/land-mask PNG.
- [ ] **Gate 2:** Phase 2 tests prove local ocean/atmosphere/hydrology coupling with explicit non-wrapping edge forcing and a zonal background calibrated against the upstream global model; preview emits temperature/rain/river maps.
- [ ] **Gate 3:** Phase 3 tests prove the retained ecology, founders, property inventory, metric estate layout (computer room inside the house), dense local vegetation, computers and material economy operate on regional cells, with carbon, oxygen and water budgets closing every step; preview emits the estate plan and local tree map.
- [ ] **Gate 4:** Phase 4 tests prove complete snapshot/replay determinism and scheduler cadence behaviour; every human has a folder; the owner can create a human from the dashboard and see them in the world and on disk.
- [ ] **Gate 5:** Phase 5 tests/benchmarks prove the regional app renders/inspects the retained world and normal execution no longer depends on planetary-only modules.

## Test tiers

- **Fast tier** (`cargo test --workspace`): every test that completes in under ~30 s in a debug build. Runs on every push.
- **Slow tier** (`cargo test --workspace --release -- --ignored`): long-horizon, benchmark and multi-year acceptance tests, each marked `#[ignore = "slow: ..."]`. Runs nightly and before each gate is ticked.
- New acceptance tests that advance more than ~30 simulated days at fine cadence belong in the slow tier.

## Owner decisions

Decided (2026-10-02):

1. **Island shape:** uniquely shaped and procedurally generated from the island's own tectonics — irregular coastline with headlands and bays, explicitly *not* New Zealand's outline and not a blob. Enforced by shape metrics; the owner picks the final seed from a Phase-1 preview gallery.
2. **Upstream resync:** not needed — upstream `dylmarriner/Maer-Ken` HEAD equals the pinned commit `7c05f0d` as of 2026-10-02. Phase 0 Task 6 re-checks before Phase 1 starts.
3. **Bevy:** upgrade to Bevy 0.19 (current) rather than upstream's 0.13. Upstream `mk_ui`/`mk_view` render and UI code is a behavioural reference to port, not code to copy unchanged.
4. **Detail patch:** one 4 km × 4 km high-detail patch centred on the founders' estate holds the house, shed, garage, workshop, rooms, vehicles, tools, computers and individually simulated trees.
5. **Computer room:** part of the house — laid out as a room inside the House, reached through it (Phase 3 Task 5).
6. **Physical materials:** gathered wood, food, fibre, resin, coal and water move real carbon, oxygen and water; burning and human metabolism release CO₂; biotic resources regrow only as biomass regrows (Phase 3 Task 3).

Population:

7. **Population.** The island starts with Gem-D and Gem-K (the only humans upstream defines, running the complete runtime). The owner adds people through the dashboard Human Creator (first in Phase 0b, then with island placement in Phase 4 Task 6); further people are born through the reproduction system. Every human, however created, gets their own folder.

## Final verification

- [ ] Run `cargo fmt --all -- --check` and expect exit 0.
- [ ] Run `cargo clippy --workspace --all-targets -- -D warnings` and expect exit 0 after resolving inherited warnings rather than suppressing new ones.
- [ ] Run `cargo test --workspace` and `cargo test --workspace --release -- --ignored`; expect 0 failures in both tiers.
- [ ] Run the fixed-seed replay command defined in Phase 4 twice and require byte-identical final hashes.
- [ ] Run the benchmark command defined in Phase 5 and commit the baseline report under `benchmarks/`.
- [ ] Update `UPSTREAM.md` with every imported path and island divergence introduced by the phases.
- [ ] Tick the program complete only after all seven gates are green.
