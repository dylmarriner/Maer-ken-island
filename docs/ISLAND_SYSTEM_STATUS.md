# Island system status

Updated 2026-10-08. This records what is implemented, measured and verified, and the known
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
| Physical materials: gather, hunt, burn, calcine, eat, respire, drink, build through the flux ledger (Phase 3 Task 3) | Model done; not yet driving the economy | `regional/materials.rs`, `tests/island_material_flows.rs` |
| Timed, energy-costed work: tasks, BMR x MET, walking, load limits (Phase 3 Task 3b) | Model done; not yet driving the economy | `regional/labour.rs`, `tests/island_labour.rs` |
| Estate fuel and electricity (Phase 3 Task 4b) | Done | `regional/energy.rs`, `tests/island_energy.rs` |
| Individual trees and stands on the estate patch (Phase 3 Task 7) | Done | `regional/local_vegetation.rs`, `tests/island_local_vegetation.rs` |
| Island scenario and estate placement (Phase 3 Task 4) | Done | `mk_island/src/scenario.rs`, `regional/property.rs`, `fixtures/island/default_scenario.json` |
| World composition, scheduler, snapshot, commands and replay (Phase 4 Tasks 1-4) | Done | `regional/{world,scheduler,commands,replay}.rs`, `io/island_snapshot.rs`, `tests/island_{world_bootstrap,scheduler,snapshot,replay}.rs` |
| Per-human folders through upstream storage, written in rotating shards (Phase 4 Task 3b) | Done | `regional/store.rs`, `tests/island_human_store.rs` |
| Births on the island: newborn age and WHO growth, adoption into the household books (mother's carbon, mother's position), body-mass rations, under-fives stay with their mother | Done; a year run shows the child as a 5 kg infant at 3 months | `humans/growth.rs`, `regional/life.rs` (`adopt_newborns`), `tests/island_newborn.rs`, UPSTREAM.md #27, `benchmarks/island_week.md` |
| Meals and drinks booked where the person stands (plants on their cell or around it; water within two cells, or the household stream near home) | Done; a year of physics shows no dry draw for the household | `regional/life.rs` (`six_hourly`), `regional/materials.rs` (`cells_around`), `tests/island_foraging.rs`, `tests/island_water_supply.rs` (slow) |
| Headless runner and run inspection (Phase 4 Task 5) | Done except nothing pending | `apps/island/src/runner.rs`, `tests/island_runtime_acceptance.rs`, `apps/island/tests/runner_cli.rs` |
| Live island server, pacing, creator with authored people and locations (Phase 4 Task 6) | Nearly done; see "Not done" | `apps/island/src/serve/island_{sim,api}.rs`, `regional/pacing.rs`, `tests/island_{pacing,create_human}.rs` |

## Measured (release build, development machine)

| Quantity | Result | File |
|---|---|---|
| Island physics, one simulated year | 22 s, 213 MB peak | `benchmarks/phase2_physical.md` |
| Human runtime, 5,000 people | 23 us/human-step, 153 MB peak | `benchmarks/humans_population.md` |
| **Combined island, one simulated week** (physics, ecology, estate, 200,000 trees, 2 founders, energy, materials) | 49 s, **307 MB peak** | `benchmarks/island_week.md` |

| Island with 1,002 / 5,002 people (360 / 120 minute-steps) | 594 MB / 1,017 MB peak; 115 / 511 ms per step | `benchmarks/island_population.md` |
| Snapshot of 5,002 people | 923 MB file before compression, 30 s save, 10 s load, 1.7 GB peak (was OOM-killed at 3.6 GB before the streaming fix); snapshots are now compressed about 7x | `benchmarks/island_population.md` |
| **Regional baseline** (`island_bench`, two founders, 1,000 steps) | 12.9 simulated days per real minute, 3.2 ms per step, 466 MB peak, snapshot 49 MB (save 1.9 s, load 2.2 s); no speedup from more cores (0.96x) | `benchmarks/2026-10-08-regional-baseline.json`, `benchmarks/README.md` |
| Two founders, one simulated year | about 32 min wall clock, 397 MB peak including the final snapshot; a birth occurred | `benchmarks/island_week.md` |
| **Two founders, five simulated years** | about 164 min wall clock, **321 MB peak, no growth trend** (sampled every minute); 5 people alive at the end (the founders and three children born on the WHO curves); 0 food and 0 water shortfalls | `benchmarks/island_week.md` |

Limits of this evidence: populated runs are short (six hours or less) and add no births, deaths or
growth; the year run has only the founders and its household outcomes were not inspected in
detail; the refinery and town (Phase 4b) are not built. The development machine is shared and
often under heavy memory pressure, so peak memory is the reliable column and wall-clock figures
are pessimistic.

## Verification commands

    cargo fmt --all -- --check
    cargo clippy --workspace --all-targets -- -D warnings
    cargo test --workspace --release
    cargo test --workspace --release -- --ignored slow_

Last full run (2026-10-09, commit `01ed5f9`, release build, `--test-threads 2`, one build job):

| Check | Result |
|---|---|
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean (same tree, run first in the same sequence) |
| `cargo test --workspace --release --no-fail-fast` | **1,154 passed, 0 failed**, 19 ignored |
| `cargo test --workspace --release -- --ignored slow_` | **7 passed, 0 failed**: human runtime to 5,000 people, crowded cell, island week, day-in-one-step equals 1,440 steps, long-horizon evolution, island year of weather, ten thousand years of seismicity |

The previous full run (commit `56de476`) had caught a real failure the per-module runs had missed (a
deviation-register row edited in the wrong place); it was fixed there. Ignored tests that are not `slow_`-named (the
population scale test, the year of drinking-water physics) are run by hand, see `benchmarks/`.
The memory-heavy tests need several GB: on a small machine build with `-j 1`, run with
`--test-threads 2`, and keep `/tmp` (often RAM-backed) clear of large files.

## Known fidelity limits

Every known departure from reality is in `docs/island/DEVIATIONS.md` (D1-D31). The largest open
ones: no sea/land breezes (D3); storm structure is parametric (D4); river channels have no
in-channel storage (D28); the regional tick audits tidal heat only (D29); biomass residence times
are round estimates (D30); production ignores soil nutrients (D31); humans do work instantly and
at no energy cost until Phase 3 Task 3b (D9, D22); materials and fuel are not physical until
Phase 3 Tasks 3 and 4b (D10, D13).

## Not done

- Phase 3 Task 1b (island-scale species) and Task 2 (the full resource and material catalogue),
  and the economy/lifecycle wiring of Tasks 3 and 3b (so people do timed, energy-costed work
  through the materials model; today the models exist and are tested but do not drive the
  household economy).
- Phase 4 Task 2b (deep-time spin-up): blocked, not skipped. There is no soil, erosion or
  tectonics model to spin up, and ecology already starts at its equilibrium (plan, Task 2b).
- Phase 4 Task 6, remainder: a map picker for island cells,
  and `agent_id` slugging from a display name. The page's read path was checked in headless Chrome; its Add form was not clicked in a browser.
- The five-year run was made before meals and drinks were booked where people stand; it has not been repeated on the new bookkeeping, so its end state describes the earlier one.
- Children: they are small, grow on the WHO curves, take a body-mass share of food and water, and under five years old stay with their mother. Older children choose their own actions, but not an adult's: under 12 they do not mine, build, craft, code or use email and search, and under 16 they take no part in harm or intimacy (`humans/autonomy.rs` `age_substitute`; the thresholds are round numbers, not sourced); a child whose mother has died is not cared for; schooling and play are not modelled.
- Phase 4b: energy, industry, town, economy beyond the estate.
- Phase 5: pruning, lifetime-scale runs (births, deaths and growth over decades), final
  benchmarks, README.
- Owner reviews: Gate 1 (island), Gate 2 (weather previews), Gate 3 (integrated life).
- Upstream pull requests for every divergence in `UPSTREAM.md`.
