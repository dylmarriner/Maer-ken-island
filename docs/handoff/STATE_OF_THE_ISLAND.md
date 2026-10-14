# State of the island (2026-10-12, checked 2026-10-13)

This is the map. For the evidence behind each line (the tests and benchmarks that show it
works) see `docs/ISLAND_SYSTEM_STATUS.md`. For everything that departs from reality, see
`docs/island/DEVIATIONS.md`.

## The layers

```
canon (fixtures/island/canon.json, mk_core::canon)
  └─ zonal background: a 1-D latitude-band climate of the whole planet      regional/zonal.rs
       └─ boundary forcing at the domain's edges                            regional/boundary.rs, edge.rs
            └─ geophysics, generated once per seed                          regional/geophysics.rs
                 (tectonics, terrain, volcanism, seismicity, geology, deposits, shape)
                 └─ the coupled physical tick                               regional/physical.rs
                      climate (+ maritime air, + diurnal cycle) → synoptic systems → weather
                      → hydrology (+ plant diversions) → ocean, tides
                      └─ ecology: biomes, NPP, biomass                       regional/ecology.rs
                           └─ the estate patch: 200,000 trees            regional/local_vegetation.rs
                                └─ IslandLife: estate, founders, energy,      regional/life.rs
                                   materials, timber, audited every step
                                     └─ IslandWorld / scheduler / commands     regional/world.rs, scheduler.rs
                                          └─ apps: serve, run, replay, UI       apps/island, apps/island_ui
```

The domain (`mk_island::IslandDomain`) has three levels: coarse 12 km cells (climate, weather,
ocean: 160 x 200), medium 2 km cells (terrain, hydrology, ecology: 960 x 1,200), and the estate
patch, 4 km at 5 m (800 x 800). `regional/levels.rs` moves fields between them.

## Physical world (`crates/mk_engine/src/regional/`)

| System | File | Notes |
|---|---|---|
| Geophysics: tectonics, terrain, volcanism, seismicity, geology, deposits, island shape | `tectonics.rs`, `terrain.rs`, `volcanism.rs`, `seismicity.rs`, `geology.rs`, `deposits.rs`, `shape.rs`, `geophysics.rs` | Phase 1. Generated once per seed; `island_preview gallery` surveys seeds. |
| Climate | `climate.rs` | Upstream energy balance on the coarse grid plus the lapse rate, maritime air (`maritime.rs`, D37) and the **diurnal cycle**. Since 2026-10-12 the land's day-night range comes from boundary-layer heat storage (Panwar & Kleidon 2022) of the day's sunshine, less what the soil evaporates (D3). |
| Weather and synoptic systems | `weather.rs`, `synoptic.rs` | A vapour budget carries the sea's humidity inland (D38, D26). Travelling cyclones, anticyclones and fronts are parametric (D4). |
| Hydrology | `hydrology.rs` | Bucket soil, Horton infiltration, a flow network routed to the sea (D29). `Diversion`s take a plant's water out at its intake and return it at its tailrace. |
| Ocean, tides | `ocean.rs`, `tides.rs` | Upstream thermodynamics; the SST season lags (D25). |
| The coupled tick | `physical.rs` | `RegionalPhysicalState::bootstrap` settles, then spins up two orbits. **The last orbit's river flows become every perennial cell's flow-duration curve** (`river_flows`, Phase 4b). |

## Living world

| System | File | Notes |
|---|---|---|
| Ecology | `ecology.rs` | Köppen-Geiger biomes (D32), Miami-model NPP (D31: no nutrients), Whittaker & Likens residence times. |
| Trees on the estate patch | `local_vegetation.rs` | ~200,000 stems with Chave allometry; `fell_tree` books the stem's carbon. |
| The estate | `property.rs`, `estate_layout.rs` | Placed on a dry 2 x 2 block of medium cells **beside a river that can run the estate's micro-hydro plant**; laid out in metres (house, shed, workshop, armoury, computer room, garage). Inventory from upstream `PropertySystem` plus the forestry tools and gold pan the owner added. |
| Humans | `humans.rs`, `../humans/` | The full upstream human runtime: body, needs, cognition, emotion, memory, drives, a free-will decision loop (`humans/autonomy.rs`) that learns from prediction error, reproduction with mutual consent, Siler mortality fitted to SSA 2019 (D21). |
| Integrated life | `life.rs` | The household step: respiration charged at BMR x MET, meals and drinks when a founder chooses them (D10), lamps (D23), labour, timber. **Every step is audited**: `audited()` checks that the ledger's net flows equal the change in material carbon, human carbon and material water. |
| Materials | `materials.rs` | Stocks (wood, charcoal, food, water...), structures, bodies, and **logs**. Every movement is a flux-ledger entry. |
| Labour | `labour.rs` | Task durations and METs from the reference packs; walking and loaded walking; carrying limits. |
| Timber | `timber.rs` | Felled stems become green logs lying from the stump; a founder moves each to the building workshop by the quickest of the excavator, the ute (cross-cut into carryable pieces) or on foot. See D40. |
| Energy | `energy.rs` | Estate electricity: **hydro first**, then solar, battery and the diesel generator; fuel stores; vehicles burning fuel per km or per engine-hour; lamps. |

## Industry (`regional/industry/`, Phase 4b)

| File | What |
|---|---|
| `hydro.rs` | Turbine choice by head (ESHA 2004), RETScreen part-load efficiency curves, generator efficiency, Darcy-Weisbach penstock sizing, `design_hydro`, `HydroDesign::output_kw`. |
| `flow_duration.rs` | `FlowDuration` per perennial medium cell from a simulated year; firm flow at 95% exceedance. |
| `hydro_sites.rs` | `site_for_powerhouse`: walk upstream along the main stem to the shortest penstock that supports a design at the firm flow with 30% of the mean left in the river; `site_on_same_river` for the 2 MW town plant. |
| `grid.rs` | Overhead lines (BS 215 ACSR, IEC 60038 voltages, AS/NZS 3000 5% drop), I²R losses, and a radial `Grid` that dispatches in merit order and sheds the lowest-priority loads. |

The default island: a 25 kW Francis on 138 m of head beside the estate, an 11 kV line to the
house, and a 2 MW town site 129 m lower on the same river.

## Persistence, determinism, the outside world

| System | File |
|---|---|
| World composition, cadence scheduler, deep-time spin-up | `world.rs`, `scheduler.rs`, `deep_time.rs` (1,000 local years of plants, grazers and soil before the founders arrive; cached under `<save_root>/spinup/`) |
| Snapshot (one deflated, digest-prefixed file) | `crates/mk_engine/src/io/island_snapshot.rs` |
| A folder per human | `human_store.rs` |
| External commands, replay, interventions | `commands.rs`, `replay.rs`, `interventions.rs`, `create_human.rs` |
| Backend, dashboard, API | `apps/island/src/serve/`, `crates/mk_island_api/`, `crates/mk_island_client/` |
| Desktop app (Bevy 0.19) | `apps/island_ui/` |
| Headless previews and galleries | `apps/island_preview/` (`gallery --hydro` surveys estates and hydro sites) |
| Opt-in real-internet computer service | `services/computer-service/` |

Determinism: every random draw is keyed (`mk_core::rng`, `RngRegistry`, or a blake3 hash of
ids, as `timber::fall_heading_rad` does). The project's realignment allows real randomness for
agent cognition, but the island's acceptance tests pin digests, so anything that feeds the
digest must stay reproducible.

## Reference data (`fixtures/reference/`)

Every number the model uses that describes the real world lives in a cited pack, loaded by
`mk_engine::validation::ReferenceLibrary` and checked by `tests/reference_packs.rs`:

| Domain | Packs |
|---|---|
| climate | `midlatitude_maritime.json`: lapse rates, DTR by surface, cloud's cut, orographic rain, SST range, synoptic periods, NZ annual ranges, boundary-layer lapse (Panwar & Kleidon 2022), albedo (Oke 1987), clear-sky transmittance (FAO-56), Priestley-Taylor |
| hydrology | `catchments.json` |
| ecology | `productivity_and_allometry.json` |
| geology | `material_properties.json`, `seismic_volcanic_mineral.json` |
| humans | `physiology.json` (METs from the Compendium, water, sleep, loads...), `life_history.json` (SSA 2019 life table, fertility) |
| labour | `productivity.json` (felling, digging, mining, panning, building, vehicles, lamps), `timber.json` (green wood, the ute, the CAT 330, loader-crane cycles) |
| industry | `hydro.json` (turbines, generators, penstocks, river flows, lines) |

## The digests that pin behaviour

| Pin | Where | Current value |
|---|---|---|
| The founders' week | `crates/mk_engine/tests/island_life_acceptance.rs` `WEEK_DIGEST` | `2c9db940…e414` (re-pinned with D9's mining sites on top of R3's edible food, `5b01ed92…6d13`) |
| The planetary topology | `crates/mk_engine/tests/topology_hash_baseline.rs` | `e4ece246…73e2` (moved only by starter-shed data: the R1b jerrycans) |

`WORKING_PRACTICES.md` says how and when to re-pin.
