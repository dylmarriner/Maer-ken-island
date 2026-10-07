# Island system status

Updated 2026-10-07. This records what is implemented, measured and verified, and the known
fidelity limits. The project is **not complete**: see "Not done".

## Implemented

| System | State | Evidence |
|---|---|---|
| Baseline, human creator, canon and realism suite (Phases 0, 0b, 0c) | Done | `crates/mk_engine/tests/human_realism.rs`, `fixtures/reference/` |
| Island domain and geophysics (Phase 1) | Done; owner's island provisional | `fixtures/island/default_profile.json`, `docs/previews/phase1/` |
| Climate, weather, storms, ocean, tides, hydrology, coupled tick (Phase 2) | Done; owner review pending | `crates/mk_engine/src/regional/`, `docs/previews/phase2/`, year-long reference test |
| Island ecology fields: biomes, NPP, producer biomass (Phase 3 Task 1a) | Done | `regional/ecology.rs`, `tests/regional_ecology.rs` |

## Measured (release build, development machine)

| Quantity | Result | File |
|---|---|---|
| Island physics, one simulated year | 22 s, 213 MB peak | `benchmarks/phase2_physical.md` |
| Human runtime, 5,000 people | 23 us/human-step, 153 MB peak | `benchmarks/humans_population.md` |

Combined ecology, economy, humans and persistence on the island are **not yet measured**.

## Verification commands

    cargo fmt --all -- --check
    cargo clippy --workspace --all-targets -- -D warnings
    cargo test --workspace --release
    cargo test --workspace --release -- --ignored slow_

The last full-workspace run was at the end of Phase 1 (1,026 tests passing). Later phases were
verified per module and by their own acceptance tests.

## Known fidelity limits

Every known departure from reality is in `docs/island/DEVIATIONS.md` (D1-D31). The largest open
ones: no sea/land breezes (D3); storm structure is parametric (D4); river channels have no
in-channel storage (D28); the regional tick audits tidal heat only (D29); biomass residence times
are round estimates (D30); production ignores soil nutrients (D31); humans do work instantly and
at no energy cost until Phase 3 Task 3b (D9, D22); materials and fuel are not physical until
Phase 3 Tasks 3 and 4b (D10, D13).

## Not done

- Phase 3 Task 1b (island-scale species) and Tasks 2-9: materials and resources, physical
  materials, time and energy of actions, estate placement, fuel and electricity, estate layout,
  grid topology for human and organism runtimes, dense estate vegetation, founders on the island.
- Phase 4: runtime scheduling, persistence, replay.
- Phase 4b: energy, industry, town, economy.
- Phase 5: app and UI replacing the human-only bootstrap, performance work, pruning, final
  benchmarks.
- Owner reviews: Gate 1 (island) and Gate 2 (weather previews).
- Upstream pull requests for every divergence in `UPSTREAM.md`.
