# Maer-Ken Island Program Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the approved full-depth Maer-Ken simulation as a deterministic New-Zealand-scale single-island regional world surrounded by ocean.

**Architecture:** Five dependent implementation plans replace whole-planet detailed grids with `IslandDomain`, regional boundary forcing, a regional `IslandWorldState`, and a flat/regional UI while retaining Maer-Ken physics, ecology, human, property, resource, persistence, and audit behaviour. Each plan ends in a buildable, independently testable milestone and becomes the input contract for the next.

**Tech Stack:** Rust 2021 workspace, serde/serde_json, blake3, rand_chacha, existing Maer-Ken engine modules, Bevy 0.13 + bevy_egui 0.25 for the final regional UI.

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
- Imported code remains attributable to `dylmarriner/Maer-Ken` commit `7c05f0dcf254387ffd7322dbb525fe4807228602` until an explicit upstream sync changes the pin.
- Normal island execution must not allocate unrelated whole-planet terrain, ocean, atmosphere, biosphere, settlement, or resource grids.

## Review Focus

- Invalid/degenerate regional profiles must fail validation before any grid allocation.
- A generated coastline must never breach the 300 km edge buffer even after target-area fitting.
- Regional edge algorithms must not accidentally retain longitude-style east/west wrapping.
- Save/load or replay must reject incompatible profile/canon versions rather than silently normalizing them.
- UI/render projection must never feed presentation state back into deterministic simulation state.

---

## Ordered plan set

1. `2026-10-01-island-domain-geophysics.md` — regional coordinate contract, deterministic boundaries, tectonics/terrain/volcanism, one-island land-area fitting.
2. `2026-10-01-island-water-atmosphere.md` — regional climate/weather/ocean/hydrology/tides and boundary exchange.
3. `2026-10-01-island-life-property-humans.md` — ecology/vegetation/resources, canonical property inventory, founders and human interactions.
4. `2026-10-01-island-runtime-persistence.md` — regional world composition, scheduler, deterministic replay, save/load and state hashing.
5. `2026-10-01-island-app-ui-performance.md` — regional Bevy application, assets/inspectors, benchmarks, upstream sync and planetary-code pruning.

## Program gates

- [ ] **Gate 1:** Phase 1 tests prove deterministic one-island generation within land-area tolerance and ocean-buffer constraints.
- [ ] **Gate 2:** Phase 2 tests prove local ocean/atmosphere/hydrology coupling with explicit non-wrapping edge forcing.
- [ ] **Gate 3:** Phase 3 tests prove the retained ecology, founders, property inventory, computers and material economy operate on regional cells.
- [ ] **Gate 4:** Phase 4 tests prove complete snapshot/replay determinism and scheduler cadence behaviour.
- [ ] **Gate 5:** Phase 5 tests/benchmarks prove the regional app renders/inspects the retained world and normal execution no longer depends on planetary-only modules.

## Final verification

- [ ] Run `cargo fmt --all -- --check` and expect exit 0.
- [ ] Run `cargo clippy --workspace --all-targets -- -D warnings` and expect exit 0 after resolving inherited warnings rather than suppressing new ones.
- [ ] Run `cargo test --workspace -- --test-threads=1` and expect 0 failures.
- [ ] Run the fixed-seed replay command defined in Phase 4 twice and require byte-identical final hashes.
- [ ] Run the benchmark command defined in Phase 5 and commit the baseline report under `benchmarks/`.
- [ ] Update `UPSTREAM.md` with every imported path and island divergence introduced by the five phases.
- [ ] Commit only after all five plan gates are green.
