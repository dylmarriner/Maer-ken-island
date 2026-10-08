# Planetary-only pruning inventory (Phase 5, Task 7 step 4)

Computed 2026-10-09 by following `crate::` references between the 35 top-level modules of
`crates/mk_engine` from the modules the production apps (`apps/island`, `apps/island_preview`,
`apps/island_humans`) use directly (`humans`, `io`, `organisms`, `regional`, `tectonics`). It is a
source-level approximation (a module counts as reachable if any file of a reachable module names it),
so it can only over-count what is reachable.

**Result: 26 of 35 modules are reachable; 9 are not, 5,619 of 66,469 lines (8.5%).**

| Module | Lines | What it is |
|---|---|---|
| `interventions` | 2,485 | the planetary operator-intervention engine (the island's commands use `mk_interventions` types, not this) |
| `transparent_ui` | 1,163 | a planetary inspection UI layer |
| `long_horizon_verification` | 951 | planetary long-horizon test support |
| `mki_declaration` | 350 | upstream declaration/manifest tooling |
| `replay` | 341 | the planetary replay (the island has its own, `regional/replay.rs`) |
| `verification` | 181 | verification helpers |
| `phase7_release_artifacts` | 104 | release-artifact tooling |
| `runtime` | 41 | planetary runtime shell |
| `sim` | 3 | stub |

The other 26 modules, including the planetary climate, ocean, hydrology, tides, insolation and
biosphere, stay reachable because the island calls their geometry-aware cores (`step_climate_on`,
`step_ocean_on`, `step_tides_on`, `HumanSystem::step_on`, ...). They cannot be pruned without
rewriting the island.

**Recommendation: do not prune.** The 9 unreachable modules are tooling, not runtime: pruning them
removes no memory or run time (they are never constructed by the island; the allocation-bound test
shows no planetary world is built), saves under 10% of the source, and makes the upstream sync
(`UPSTREAM.md`) harder, because every removed module is a divergence to carry. If the owner wants a
smaller tree anyway, remove them in small commits, running the workspace after each; `transparent_ui`
and `interventions` are the only two with meaningful size, and the planetary tests that use them
(`mk_engine/tests`) would go with them.
