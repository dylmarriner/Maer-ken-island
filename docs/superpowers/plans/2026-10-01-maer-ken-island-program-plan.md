# Maer-Ken Island Program Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the approved full-depth Maer-Ken simulation as a deterministic New-Zealand-scale single-island regional world surrounded by ocean.

**Architecture:** A baseline phase makes the imported workspace green and gated, then five dependent implementation plans replace whole-planet detailed grids with `IslandDomain`, regional boundary forcing, a regional `IslandWorldState`, and a flat/regional UI while retaining Maer-Ken physics, ecology, human, property, resource, persistence, and audit behaviour. Each plan ends in a buildable, independently testable milestone, produces a visible preview of its output, and becomes the input contract for the next.

**Tech Stack:** Rust 2021 workspace, serde/serde_json, blake3, rand_chacha, existing Maer-Ken engine modules, `png` for headless previews, Bevy + bevy_egui for the final regional UI (version decided at the start of Phase 5; see Owner decisions).

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
1. `2026-10-01-island-domain-geophysics.md` — regional coordinate contract, deterministic boundaries, tectonics/terrain/volcanism, one-island land-area fitting, headless preview tool.
2. `2026-10-01-island-water-atmosphere.md` — dependency inventory, zonal background forcing, regional climate/weather/ocean/hydrology/tides and boundary exchange.
3. `2026-10-01-island-life-property-humans.md` — ecology/vegetation/resources, dense local vegetation, canonical property inventory with a metric estate layout, founders and human interactions.
4. `2026-10-01-island-runtime-persistence.md` — regional world composition, scheduler, deterministic replay, save/load and state hashing.
5. `2026-10-01-island-app-ui-performance.md` — regional Bevy application, assets/inspectors, benchmarks, upstream sync and planetary-code pruning.

## Program gates

- [ ] **Gate 0:** `cargo test --workspace` (fast set) passes with 0 failures, the slow set passes in release, and CI enforces fmt/clippy/fast tests on every push.
- [ ] **Gate 1:** Phase 1 tests prove deterministic one-island generation within land-area tolerance and ocean-buffer constraints; `island_preview` emits the elevation/land-mask PNG.
- [ ] **Gate 2:** Phase 2 tests prove local ocean/atmosphere/hydrology coupling with explicit non-wrapping edge forcing and a zonal background calibrated against the upstream global model; preview emits temperature/rain/river maps.
- [ ] **Gate 3:** Phase 3 tests prove the retained ecology, founders, property inventory, metric estate layout, dense local vegetation, computers and material economy operate on regional cells; preview emits the estate plan and local tree map.
- [ ] **Gate 4:** Phase 4 tests prove complete snapshot/replay determinism and scheduler cadence behaviour.
- [ ] **Gate 5:** Phase 5 tests/benchmarks prove the regional app renders/inspects the retained world and normal execution no longer depends on planetary-only modules.

## Test tiers

- **Fast tier** (`cargo test --workspace`): every test that completes in under ~30 s in a debug build. Runs on every push.
- **Slow tier** (`cargo test --workspace --release -- --ignored`): long-horizon, benchmark and multi-year acceptance tests, each marked `#[ignore = "slow: ..."]`. Runs nightly and before each gate is ticked.
- New acceptance tests that advance more than ~30 simulated days at fine cadence belong in the slow tier.

## Owner decisions

These change scope or appearance and are the repository owner's to make; implementation uses the stated default until decided.

1. **Island shape.** Default: unconstrained generated landmass (area-fitted only). Option: set `IslandProfile::shape` to an elongated NZ-like form (aspect ratio and orientation) — see Phase 1 Task 4.
2. **Upstream resync before Phase 1.** Default: keep the current pin unless Phase 0 Task 6 finds human/property changes upstream worth taking.
3. **Bevy version.** Default: match upstream `mk_ui` so render code ports directly; upgrading is a separate decision at the start of Phase 5.
4. **Detail-patch size.** Default: one 4 km × 4 km high-detail patch centred on the founders' estate (Phase 3). Larger or multiple patches cost memory and tick time and should follow Phase-5 benchmarks.

## Final verification

- [ ] Run `cargo fmt --all -- --check` and expect exit 0.
- [ ] Run `cargo clippy --workspace --all-targets -- -D warnings` and expect exit 0 after resolving inherited warnings rather than suppressing new ones.
- [ ] Run `cargo test --workspace` and `cargo test --workspace --release -- --ignored`; expect 0 failures in both tiers.
- [ ] Run the fixed-seed replay command defined in Phase 4 twice and require byte-identical final hashes.
- [ ] Run the benchmark command defined in Phase 5 and commit the baseline report under `benchmarks/`.
- [ ] Update `UPSTREAM.md` with every imported path and island divergence introduced by the phases.
- [ ] Tick the program complete only after all six gates are green.
