# Island system status

Updated 2026-10-07. This records what is implemented, measured and verified, and the known
fidelity limits. The project is **not complete**: see "Not done".

## Implemented

| System | State | Evidence |
|---|---|---|
| Baseline, human creator, canon and realism suite (Phases 0, 0b, 0c) | Done | `crates/mk_engine/tests/human_realism.rs`, `fixtures/reference/` |
| Island domain and geophysics (Phase 1) | Done; owner's island provisional | `fixtures/island/default_profile.json`, `docs/previews/phase1/` |
| Climate, weather, storms, ocean, tides, hydrology, coupled tick (Phase 2) | Done; owner review pending | `crates/mk_engine/src/regional/`, `docs/previews/phase2/`, year-long reference test |
| **Integrated island life**: physics + ecology + estate + 200,000 trees + founders + energy + materials, audited (Phase 3 Task 9) | Done; one week measured | `regional/life.rs`, `tests/island_life_acceptance.rs`, `benchmarks/island_week.md` |
| Island ecology fields: biomes, NPP, producer biomass (Phase 3 Task 1a) | Done | `regional/ecology.rs`, `tests/regional_ecology.rs` |
| Grid topology for human/organism movement, births and perception (Phase 3 Task 6) | Done; planetary results bit-identical | `topology.rs`, `tests/topology_hash_baseline.rs` |
| Founders' estate laid out in metres (Phase 3 Task 5) | Done | `regional/estate_layout.rs`, `tests/island_estate_layout.rs` |
| Canonical founders on the island: observation, bedrooms, computer rule (Phase 3 Task 8) | Done except Task-3-dependent hooks | `regional/humans.rs`, `humans/observation.rs`, `tests/island_human_runtime.rs` |
| Physical materials: gather, hunt, burn, calcine, eat, respire, drink, build through the flux ledger (Phase 3 Task 3) | Model done and audited inside the island tick. A meal is the carbon its eater respired since the last one, so adults hold at 1.00x of their expected body carbon over a week. Who harvests and when is still the routine's choice, not each human's (D10) | `regional/materials.rs`, `regional/life.rs`, `tests/island_material_flows.rs`, `tests/island_life_acceptance.rs` |
| Timed, energy-costed work: tasks, BMR x MET, walking, load limits (Phase 3 Task 3b) | Model done. Energy is wired: the island tick costs resting metabolism at the body's BMR times the MET of sleeping or sitting, from the circadian clock, and `humans/lifecycle.rs` charges walking, mining and building their Compendium METs. Durations are not: every action still takes one tick, and the actions with no pack row cost the baseline (D9, D22) | `regional/labour.rs`, `regional/life.rs`, `humans/lifecycle.rs`, `tests/island_labour.rs`, `tests/human_realism.rs` |
| Estate fuel and electricity (Phase 3 Task 4b) | Done | `regional/energy.rs`, `tests/island_energy.rs` |
| Individual trees and stands on the estate patch (Phase 3 Task 7) | Done | `regional/local_vegetation.rs`, `tests/island_local_vegetation.rs` |
| Island scenario and estate placement (Phase 3 Task 4) | Done | `mk_island/src/scenario.rs`, `regional/property.rs`, `fixtures/island/default_scenario.json` |
| Web dashboard: overview, roster with a readable per-person record, Human Creator, JSON API (`island serve`) | Done for the human-only bootstrap; the Phase 5 desktop app is separate | `apps/island/src/serve/`, `apps/island/static/` |

## Measured (release build, development machine)

| Quantity | Result | File |
|---|---|---|
| Island physics, one simulated year | 22 s, 213 MB peak | `benchmarks/phase2_physical.md` |
| Human runtime, 5,000 people | 23 us/human-step, 153 MB peak | `benchmarks/humans_population.md` |
| **Combined island, one simulated week** (physics, ecology, estate, 200,000 trees, 2 founders, energy, materials) | 49 s, **307 MB peak** | `benchmarks/island_week.md` |

The combined island (without persistence, the scheduler, a larger population, the refinery/town and the app) is measured above; those remain unmeasured until Phases 4, 4b and 5.

## Verification commands

    cargo fmt --all -- --check
    cargo clippy --workspace --all-targets -- -D warnings
    cargo test --workspace --release
    cargo test --workspace --release -- --ignored slow_

`cargo test --workspace` in debug builds a large target directory (tens of GB with debug info and
incremental compilation on). On a machine with a small disk, `CARGO_INCREMENTAL=0` and
`CARGO_PROFILE_DEV_DEBUG=0` keep it to a fraction of that.

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

- Phase 3 Task 1b (island-scale species) and Tasks 2 and 9 (and the economy/lifecycle wiring of Tasks 3 and 3b): materials and resources, physical
  materials, time and energy of actions, acceptance.
- Phase 4: persistence, replay and external commands. Tasks 1 and 2 (world composition, the
  cadence scheduler) are done. Task 3, the island snapshot, needs `Serialize`/`Deserialize` on
  five state types that do not have them yet — physical, ecology, the placed estate, the
  200,000-tree vegetation patch and the material ledger — plus canon re-derivation and tamper
  rejection on load; it is not a small change.
- Phase 4b: energy, industry, town, economy.
- Phase 5: the desktop app (`island-ui`, Bevy) replacing the human-only bootstrap, performance
  work, pruning, final benchmarks. The `island serve` web dashboard covers the human-only
  bootstrap in the meantime; it has no world, no clock and no map.
- Owner reviews: Gate 1 (island) and Gate 2 (weather previews).
- Upstream pull requests for every divergence in `UPSTREAM.md`.
