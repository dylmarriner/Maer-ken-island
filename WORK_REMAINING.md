# Work remaining

Written 2026-10-13 against `main` at the merge of PR #6 (`05fd6e5`). This is the single list of
what is not done, and for each item, **what it takes to do it**: the sources or data it needs,
the decisions only the owner can make, the files it touches, the tests that prove it and when it
counts as finished. Read [`HANDOFF.md`](HANDOFF.md) first for what exists.

Where this list and a plan disagree, the plan (`docs/superpowers/plans/`) holds the full spec and
this list holds the current state. The evidence for what is *done* is in
`docs/ISLAND_SYSTEM_STATUS.md`; every departure from reality is in `docs/island/DEVIATIONS.md`.

## The rules every item below inherits

- **Every real-world number is sourced.** It goes in a cited pack under `fixtures/reference/`,
  loaded through `mk_engine::validation::ReferenceLibrary`, checked by
  `crates/mk_engine/tests/reference_packs.rs`. Never type a value from memory with a citation
  attached; `docs/island/REALISM.md` exists because that happened once (water viscosity
  remembered as 1.307 against a measured 1.3063).
- **Every material movement is a ledger entry.** `IslandLife::audited` must still close at every
  step (material carbon, human carbon, material water).
- **A known departure is a deviation row**, added or updated in `docs/island/DEVIATIONS.md` in the
  same change; `cargo test -p mk_core --test deviation_register` checks the format.
- **Changes to imported upstream code are recorded in `UPSTREAM.md`**, and human-runtime changes
  go upstream first where the plan says "upstream first".
- **The week digest is re-pinned only after the audit passes**: run
  `island_life_acceptance -- --ignored`, confirm the books close and the reference-pack checks
  pass, then update `WEEK_DIGEST` (procedure in `docs/handoff/WORKING_PRACTICES.md`).
- Behaviour-changing work runs the two reference-pack slow tests (the week and the year) before
  merging; the fast tier cannot see them.
- **Who lives where (owner, 2026-10-13).** The founders' house is the two founders' and their
  children's alone. Estate staff each have a separate 2-3 bedroom cottage on the estate; everyone
  else lives in the town. Nothing may put anybody else in the house, including test scaffolding:
  use `EstateLayout::spaces_outside_house` or a cell of the island.

## Status at a glance

| Area | Item | State | Blocked on |
|---|---|---|---|
| Timber | Splitting, crosscut, rigging, routes (D40); drying and seasoning done 2026-10-14 | Open | Splitting and sawing rates |
| Living world | Phase 3 Task 1b: species at island scale | Done (D55 open: upstream densities) | — |
| Living world | Phase 3 Task 2: every resource and material, placers, recipes | Open (partly started) | Cited placer decay, shaft size, abundances |
| Living world | D9: actions take one tick | Open (felling, hauling done) | Cited durations per task |
| Living world | D31: NPP ignores soil nutrients | Open | Regional soils model and data |
| Runtime | Phase 4 Task 2b: deep-time spin-up | **Done** (2026-10-14) | — |
| Industry | Phase 4b Task 1 remainder: reference tables | Open | Sources (petroleum, refinery, staffing, wages, demand) |
| Industry | Phase 4b Task 4: oil and gas field (D14, D19) | Open | Task 1 tables; a petroleum deposit |
| Industry | Phase 4b Task 5: refinery, gas plant, fuel depot (D16) | Open | Task 1 tables |
| Industry | Phase 4b Task 6: jobs, trades, maintenance (D39) | Open | Upstream human-runtime change first |
| Industry | Phase 4b Task 7: money, enterprises, payroll | Open | Owner: ownership split, policies |
| Industry | Phase 4b Task 7b: food and timber chains | Open | Task 2 catalogue; crop/livestock data |
| Industry | Phase 4b Task 8: town and workforce | Open | Demographic tables; owner: town size |
| Industry | Phase 4b Task 9: dashboard pages, year acceptance | Open | Tasks 4-8 |
| Climate | D3 rest: sea/land breezes, katabatic winds | Open | A diurnal wind solver |
| Climate | D25: SST season lags too long | Diagnosed 2026-10-14: 0.281 of the year against 0.18-0.21 observed | A prognostic marine boundary layer (with D29) |
| Climate | D4: parametric weather systems | Open | Structural; large |
| Climate | D38: a quarter of the land too dry | Diagnosed; specific humidity carried (2026-10-14) | A flux-driven rain-out (with D4) |
| Climate | D29: energy audited per module | Open | End-to-end energy ledger |
| App | Phase 5 Tasks 3-8 | Partly done | See below; one owner pick (portraits) |
| Gates | Gate 1, 2, 3 owner reviews | **Closed** (owner, 2026-10-13) | — |
| Upstream | PRs for every divergence in `UPSTREAM.md` | Open | Owner's upstream access |
| Realism | R0-R10: humans, work and the living world replicate Earth (section 8) | R0, R1 and R1b (fetching water by hand) done; R3 next | Engineering and sourcing |
| CI | Nightly slow job and dependency audit | Green after PR #15 | See Housekeeping |

---

## 1. Timber the estate can actually use (D40)

**Now:** a felled tree leaves a green log where it fell (0.82 kg water per kg dry wood for
hardwoods, 1.06 for softwoods, from `fixtures/reference/labour/timber.json`). A founder moves it
to the building workshop by the quickest of the CAT 330 excavator, the ute (after cross-cutting
it into pieces they can lift) or on foot. The log is stored green and stays green.

### 1a. Drying (done 2026-10-14)

`regional/drying.rs`: every log approaches the Wood Handbook's equilibrium moisture content
(Eq. 4-5, Table 4-2 reproduced) for the estate's hourly air, at a rate fitted to Simpson & Hart
(2000, FPL-GTR-121) Table 2 and scaled by vapour pressure and by thickness squared; the water
leaves `MaterialWater` for the atmosphere through the audit. Tests: `regional::drying` (the
equation against the table, the fit re-checked against all 72 Madison times, warm dry air
faster than cold wet, never below equilibrium, thickness scaling, steps compose) and
`tests/island_timber_drying.rs` (a stored log on the island dries, the books close). Left in
D40: end grain, bark, species and re-wetting.

### 1b. Seasoned wood joins the stock (done 2026-10-14)

A stored log that has dried to the stock's air-dry moisture (15% wet basis, 0.176 kg per kg
dry) becomes `Material::Wood` with its carbon exact (`MaterialLedger::season_stored_logs`,
every hour after drying). The stock's wood carries its bound water outside the water books, so
the log's last water is booked to the air at that moment rather than when the wood burns
(D40). Test: `tests/island_timber_drying.rs::a_seasoned_stored_log_joins_the_wood_stock`.

### 1c. Splitting, the crosscut saw, rigging and routes

- **Splitting firewood:** the MET is already in `fixtures/reference/humans/physiology.json`
  (the Compendium's `chopping_splitting_wood`); a cited splitting rate (kg or pieces per hour)
  is needed for the duration.
- **The two-person crosscut saw:** needs both founders at the log together, which is a
  coordination change in `life.rs`, plus a cited two-person sawing rate.
- **Excavator rigging:** the CAT 330 lifts with the shed's rope; attaching a sling is untimed
  because no cited figure was found. Either source a rigging time or the owner adds a grapple.
- **Routes:** legs are straight lines; slope (`EstateLayout::terrain_m`) and obstacles are
  ignored, and vehicles have no position of their own. A route over the patch terrain with a
  slope cost, and vehicles that stay where they were parked, would close this.
- **Files:** `regional/timber.rs`, `regional/life.rs` (`work`, `plan_haul`,
  `finish_haul_task`), `regional/labour.rs`.

---

## 2. Living world

### 2a. Phase 3 Task 1b: species at island scale

Done 2026-10-14 (`regional/species.rs`, D55): upstream's founding species placed on the island's
habitats, plants and grazers as shares of the island's carbon fields, other guilds at upstream's
density times the island's habitat area. **Left:** Earth-calibrated densities and a carbon pool
for each consumer guild (realism R8, R10); serving the species to the map and the cell inspector;
animal and fish resource nodes (§2b), which can now read the habitats.

### 2b. Phase 3 Task 2: every resource and material

- **Now:** upstream's nine node kinds; `node_kind_for_resource` names every resource kind that has
  no node yet (an exhaustive match was started); the estate's shed has an axe, a crosscut saw and
  a gold pan. Nothing else from the task exists: no `materials/catalogue.rs`, no
  `regional/resources.rs`, no placers.
- **Needs (sources):** a cited downstream decay of placer-gold grade along a river; a cited
  artisanal shaft size for hand mining; for every deposit, an abundance and grade source (the
  Phase 1 deposits already carry their own). Densities and Mohs hardness per catalogue item
  from a cited table.
- **Files:** create `crates/mk_engine/src/materials/{mod.rs,catalogue.rs}`,
  `crates/mk_engine/src/regional/resources.rs`
  (`bootstrap_regional_resources(geophysics, ecology, physical, domain)`); modify
  `resource_economy.rs` (`ResourceNodeKind`, `ToolKind` gaining `GoldPan`, `Sluice`, `Chisel`,
  `LapidaryWheel`, `HuntingWeapon`, `FishingGear`, `Kiln`, `Crucible`, `Anvil`, `Drill`, recipes),
  `agents/mod.rs` (`ItemKind`). Add, never rename, upstream variants.
- **Tests:** `crates/mk_engine/tests/regional_resources.rs` and catalogue unit tests, as listed in
  the plan's Task 2 Steps 1-2 (placers only downstream of a source, ironsand only on suitable
  coasts, deposits never regenerate, a pan is needed to pan, smelting yields gold in proportion
  to grade, deterministic ids).
- **Then:** record the enums, tool map and every abundance source in `UPSTREAM.md`; re-render
  `island_preview life` so `resources.png` shows the nodes.

### 2c. D9: most actions still take one tick

- **Now:** felling and hauling are timed and costed; everything else completes in one tick.
- **Needs:** a cited duration for each action (`fixtures/reference/labour/productivity.json`
  holds some already), and the same task machinery felling uses (`life.rs` `work`) extended to
  them. Actions with no Compendium row cost the resting baseline (D22, accepted).
- **Done when:** every action a founder can choose takes a sourced time and MET.

### 2d. D31: NPP ignores soil nutrients

- **Needs:** a regional soils model (parent rock from `geology.rs`, age from the deep-time
  spin-up in §3, nitrogen and phosphorus) and a cited nutrient limitation on NPP. Large; it
  follows the spin-up.

---

## 3. Phase 4 Task 2b: deep-time spin-up (done 2026-10-14)

`regional/deep_time.rs` runs the plants, grazers and RothC soil monthly for the scenario's
`spin_up_years` (1,000 by default) before the founders arrive, and caches the result under
`<save_root>/spinup/<key>.snap`. The full island takes 10.8 s on 8 threads in release. The plan's
evidence is in `docs/ISLAND_SYSTEM_STATUS.md` and the departures (weather, rivers and terrain not
stepped) in D20. What is left grows out of other items: once a soils model with nutrients (D31,
§2d) or succession and disturbance (R10) exists, it belongs in the same per-cell loop, and its
equilibrium is what the spin-up will then be for.

---

## 4. Phase 4b: oil and gas, refinery, trades, money, the town

Plan: `docs/superpowers/plans/2026-10-02-island-phase4b-energy-industry-town.md`. Its defaults
table is the owner's. Hydro (Tasks 1-3) is done; each later task needs its slice of Task 1's
reference tables, which were not fetched.

### 4a. Task 1 remainder: the reference tables

Add to `fixtures/reference/industry/` (new packs alongside `hydro.json`), each value cited:

| Table | For | Notes |
|---|---|---|
| Reservoir recovery factors, gas-oil ratios, water cut, Arps decline parameters | Task 4 | Published field studies or textbooks |
| Methane leak fraction from production | Task 4, D14 | An inventory figure (e.g. a national GHG inventory) |
| Spill and blowout frequencies | Task 4 | An offshore/onshore incident database |
| Crude assays and product yields; refinery energy and CO₂ per barrel | Task 5 | A published assay for a light crude; refinery benchmarks |
| Gas composition and processing yields | Task 5 | |
| Staffing per plant type and size | Task 6 | |
| Occupational wage ratios | Task 7 | e.g. a national statistics office's wage tables |
| Household electricity, cooking and transport-fuel demand | Task 8 | e.g. a national energy-use survey |
| Crop growth (growing degree days, water stress), yields per hectare, livestock feed and growth | Task 7b | FAO tables are the natural source |
| Sawmill recovery rates; house bill of materials and labour hours by trade | Task 7b | |
| Demographic structure for the founding town | Task 8 | A national census age/sex table |

### 4b. Task 4: oil and gas field (D14, D19)

- **Requires** a Phase 1 petroleum deposit on the chosen island; if there is none, the
  enterprises are not created and the dashboard says why.
- **Files:** create `regional/industry/petroleum.rs` (`PetroleumReservoir`, `Well`,
  `ArpsDecline`); modify `crates/mk_core/src/flux/mod.rs` (an `AtmosCH4` reservoir, appended last
  so earlier indices are unchanged; record in `UPSTREAM.md`) and `regional/climate.rs` (methane
  forcing per Myhre et al. 1998 as the plan states; transcribe the formula from the paper).
- **Tests:** `tests/petroleum_field.rs` (the plan's Step 1 list).
- **Decision:** D19 (no drilling rigs can be made on the island) stays open or is accepted by the
  owner as imported equipment (D15 already accepts imported plant).

### 4c. Task 5: refinery, gas plant, fuel depot (D16)

- **Files:** create `regional/industry/{refinery.rs,gas_plant.rs,fuel.rs}`; modify
  `regional/energy.rs` so vehicles and generators buy fuel at the depot.
- **Tests:** `tests/refinery.rs` (yields by assay, mass and carbon close, campaigns stop on full
  tanks, no petrol without the reformer, a vehicle refuels and pays).

### 4d. Task 6: jobs, trades, shifts, maintenance (D39) — upstream first

- **Upstream first:** `TradeQualification` in `mk_core::human`, `ActionKind::Work` and job
  attendance in the autonomous mind go into `dylmarriner/Maer-Ken`, then sync. If upstream is
  not available, record each change as a divergence in `UPSTREAM.md` and say so.
- **Files:** create `regional/industry/jobs.rs`; shifts run on a 24 h roster, not the 36 h sun.
- **Staff housing (owner, 2026-10-13):** every estate staff member -- maids, kitchen staff, farm
  hands, the staff manager or supervisor -- gets a separate 2-3 bedroom cottage on the estate,
  added to `EstateLayout` as a building kind of its own with its own bedrooms, sited on
  buildable ground clear of the house. Staff sleep in their own cottage (the bedroom rule in
  `regional/humans.rs` reads a `"<id>'s Bedroom"` label) and never in the house. Needs: a
  sourced floor area for a 2-3 bedroom cottage.
- **Estate staff (owner, 2026-10-13):** 1 maid, about 2 kitchen staff or butlers, and 1
  supervisor with his wife; the supervisor organises the farm workers and all estate staff. Farm
  workers in the numbers the farms' work needs (sourced labour per animal and per hectare). Every
  worker is free to quit (Task 8's free will).
- **D39** closes here: the hydro plant's wear, overhaul and failure need **cited overhaul
  intervals**, and maintenance labour restores it; flood closure of the intake needs a **cited
  flow threshold**.
- **Tests:** `tests/industry_jobs.rs` and `cargo test -p mk_engine --lib -- humans::`.

### 4e. Task 7: money, enterprises, payroll

- **Decided (owner, 2026-10-13):** the founders own every home and business in the town, the
  hospital and doctors, a bank and the power companies; profits are paid **monthly**, most of
  each month's profit, with a reserve kept by each business; **Gem-D and Gem-K decide for
  themselves** whether to offer homes and businesses for sale to villagers, and a villager who
  can afford one decides whether to buy (upstream first: buying and selling in the human mind).
  Wages come with benefits appropriate to the job and workplace, from a cited source.
- **Owner decisions still open:** the ownership split per enterprise (default 50/50), the exact
  payout share, initial balances, the disconnection grace period, and what the bank does
  (deposits only, or loans too).
- **Files:** create `regional/economy/{money.rs,enterprise.rs,market.rs}`; upstream first for
  `ActionKind::Buy` and money in human decisions.
- **Tests:** `tests/island_economy.rs` (money conserved, bills from meters, payroll every 14
  roster days with payslips in the person's folder, dividends equal profit × payout × share).

### 4f. Task 7b: food and timber chains

- **The farms are on the founders' estate (owner, 2026-10-13):** beef, dairy, sheep and pigs,
  and market gardens growing feed for the animals and vegetables for people. They **start small
  and grow**: each herd starts at no less than a cited minimum breeding herd, so it can keep
  breeding while animals are slaughtered, eaten or sold as meat across Maer-Ken, and grows as
  profits allow. Each farm has the vehicles and equipment its work needs, from a cited list. The
  estate's land is about the area of Pukekohe (cited), with room to expand for business and
  leisure; `regional/estate_layout.rs` and the estate patch grow to match.
- **Needs:** Task 2's catalogue (grain, flour, bread, milk, meat, logs, sawn timber and so on),
  Task 1b's species (the fishery fishes real populations), the crop and livestock tables above.
  The estate's timber work (§1) is the seed of the forestry chain: felling, hauling and drying
  already exist at estate scale.
- **Files:** create `regional/industry/{farm.rs,fishery.rs,food_processing.rs,supermarket.rs,
  forestry.rs,sawmill.rs,construction.rs}`.
- **Tests:** `tests/food_chain.rs`, `tests/timber_chain.rs` (harvest never exceeds regrowth,
  carbon conserved from tree to house).

### 4g. Task 8: town and workforce

- **Owner decision:** the founding town's size (the plan offers a smaller one if performance
  misses its targets).
- **Residence (owner, 2026-10-13):** everyone who is not a founder, a founder's child or estate
  staff lives in the town. The town's homes and businesses, its doctors and hospital, its bank
  and its power company all belong to Gem-D and Gem-K.
- **A growing country, not built towns (owner, 2026-10-13):** the simulation founds no further
  towns. People have free will: they may quit their jobs, leave their rented home, travel the
  island and live where they choose -- in a stick hut they build themselves if that is what they
  can manage (the materials model's shelter building is the start). Any new settlement grows
  from the original one by people's own choices. Upstream first: quitting, leaving and settling
  are decisions in the human mind. Once it exists, the Creator should offer the town (and, for staff,
  a cottage) as where a person lives, rather than only a spot on the estate.
- **Files:** create `regional/town.rs`, `fixtures/island/workforce.json` (generated from the seed);
  generalise `regional/estate_layout.rs` to any high-detail site.
- **Tests:** `tests/island_town.rs`.

### 4h. Task 9: dashboard pages and the year's acceptance

- **Pages:** Power, Petroleum, Town, Food, Timber, Payroll, Finance; owner controls as replayable
  commands.
- **Acceptance:** `tests/island_industry_acceptance.rs` (slow tier): one local year, every ledger
  closes (water, carbon, oxygen, methane, money), snapshot mid-year and resume to the same hash.
- **Benchmark** the full town and record it in the Phase 5 baseline.

---

## 5. Climate fidelity

| Item | What is needed |
|---|---|
| **D3 (rest)**: sea and land breezes, katabatic winds | A diurnal wind solver on the coarse grid (land-sea temperature contrast driving an onshore/offshore flow; cold-air drainage downslope at night), which would also let the coastal damping of the day-night range come from air carried off the sea instead of the observed coastal/inland ratio. The land's range itself is already physical (Panwar & Kleidon 2022). **Sources gathered 2026-10-14** (to be transcribed into the climate pack when used): poleward of 30° the linear response is coast-trapped with scale `l = N h / sqrt(f^2 - omega^2)` (Rotunno 1983, as printed in Crosman & Horel 2010, BLM 137, eq. 2b, p. 12); typical sea-breeze surface winds 6-7 m/s (Simpson 1994 via Miller et al. 2003, Rev. Geophys. 41, para 20); inflow depth 300-2500 m, ~700 m in Britain (Miller et al. 2003, para 44); New Zealand sea breezes "usually extend from the coast inland for 20 miles or more" in settled summer weather (Te Ara 1966, Winds); a land breeze "rarely achieves half of sea breeze wind speed" (Thompson 1998 via Khan 2010). **The difficulty:** with the island's own N (7 K/km) and a 0.5-1 km layer, `l` is 190-370 km, five to ten times the observed reach, because friction limits real breezes; and no amplitude scaling that includes rotation was verified. A solver needs a sourced friction or a calibration against the 6-7 m/s and ~30 km observations, stated as such. |
| **D25**: the SST season peaks 0.281 of the year after the solstice against 0.179-0.205 observed (ERSSTv5, derived 2026-10-14) | Diagnosed 2026-10-14: coupling the mixed layer to the air by bulk sensible and latent conductance shortens the lag to 0.145 but swells the annual range from 4.8 to 42.8 K, because it pulls the sea toward the energy balance's radiative equilibrium, which swings tens of kelvin. The owner question (regional or shared) does not decide it: either way the fix is a prognostic marine boundary layer whose air has its own heat capacity and is carried from upwind, which belongs with D29. The year test now measures the lag and holds it within 0.08 of the observed range. |
| **D4**: weather systems are parametric | No occlusion, no warm-front structure. A structural change to `synoptic.rs`; large. |
| **D38**: a quarter of the land gets under 460 mm/yr where most of NZ gets 600-1600 (NIWA) | Diagnosed 2026-10-14 (`tests/regional_rain_budget.rs`): before orography land gets 58% of the sea's rain, because upstream's rain-out weight follows the local column's vapour and relative humidity. The wind now carries specific humidity (median 533 → 657 mm, highlands 340 → 668). What is left needs rain driven by the vapour flux of the systems crossing the land, which belongs with D4. |
| **D29**: energy is audited per module | An end-to-end energy ledger across the coupled tick. |

---

## 6. Phase 5: the application

Plan: `docs/superpowers/plans/2026-10-01-island-app-ui-performance.md` (its checkboxes were never
ticked; `docs/ISLAND_SYSTEM_STATUS.md` is the evidence). Done: Tasks 0-2 (the crate on Bevy 0.19,
the projection, terrain and camera), 4b (the computer service), and parts of 3 (the island and
the estate drawn, the Maer-Ken models on the estate) and 5 (pause, speed and step from the
dashboard; the desktop panels).

| Task | Remaining | Needs |
|---|---|---|
| 3 | Individual stems in 3D around the yard are drawn (2026-10-14, `stems.rs`: up to 3,000 trunks within 250 m, at their own height and diameter); left of 2a: stand cover and instanced trees beyond the patch from biomass. Interiors (2b), industry and town (2d, after Phase 4b), resource visuals and inventory icons (2c, after Phase 3 Task 2), tests for every building kind and vehicle | Engineering |
| 4 | Human detail view, inspector, foundry panel; portraits | **Owner picks the two portraits** from upstream `apps/mk_ui/assets/portraits/`, and confirms their provenance before any distribution |
| 5 | Done 2026-10-14 for the clock, controls and terrain cell (`apps/island_ui/src/inspectors.rs`): six speeds with requested and achieved, Save, the local date (year, day of 288, time of a 36-hour day) with Earth time elapsed, and the cell under the view with its biome, climate and production. A click on the estate picks a building, room or item and the right panel describes it (`estate.rs` `pick`). Left: picking a tree, once the stems are drawn | Engineering |
| 6 | `island_bench`, the allocation-bound test (`construction-counters` feature), the resolution gate | **Owner confirms targets** (default: headless one simulated day per real minute; real time with the UI open) and **decides on formal predictive processing** (wire it into decisions upstream, or stop running it) |
| 7 | The sync script is done (2026-10-14, `scripts/sync_maerken_upstream.py`, `UPSTREAM.md` § Sync procedure). The creator moved into `apps/island` (`island::population`) and `apps/island_humans` is retired. Pruning is the owner's call (the inventory recommends not to) | Engineering |
| 8 | Final acceptance: every design criterion run and recorded | All of the above |

Also: **a GPU frame-rate figure** for `island-ui` (every frame so far was rasterised on the CPU,
0.6 frames/s for the island view and 10.8 for the estate), and other window sizes. The Creator
page does not yet use the map.

---

## 7. Owner gates and upstream

- **Gates 1, 2 and 3 are closed** (owner, 2026-10-13): the island, its weather and the estate
  plan are the ones the project has been built on, and that is the approval. The island is no
  longer provisional. Do not reopen them as blockers; `resources.png` is still re-rendered when
  Phase 3 Task 2 lands, as a record rather than a gate.
- **Upstream pull requests** for every divergence in `UPSTREAM.md` (Phase 0 items 4-7, Phase 0c's
  biology, and the 2026-10-12 human-runtime changes listed there). The owner kept the pin at
  `7c05f0d` on 2026-10-09 because resyncing before these merge would mean reapplying them by
  hand.

---

## 8. Realism programme: humans, work and the living world (research 2026-10-13)

The owner's standard (2026-10-13): **everything replicates Earth life.** How humans function,
think, feel, sense, behave, invent and create, and buildings and resources really built,
gathered and carried. Three research passes compared the code with the real world, with
sources: [`docs/research/2026-10-13/`](docs/research/2026-10-13/) (`humans.md`, `work.md`,
`earth.md`, plus the raw source notes beside them). Their headline findings:

- **The body is realistic; the mind barely drives anything.** `choose_action`
  (`humans/autonomy.rs`) picks from needs and a few cell averages. Senses, memory, emotion,
  learning, culture and language are computed every tick and nothing reads them to act. There
  is no smell or taste; pain does not follow injury; skill never improves with practice;
  "invention" is a timer.
- **Most actions do nothing on the island.** The resource economy starts empty and the founders
  hold no tools or containers, so Gather, Mine, Craft and Build fail; only felling and hauling
  run, through `life.rs` `work`. The real labour models in `labour.rs` (dig, mine, pan, build a
  timber frame) are called only by tests. Food and water are taken from up to 2-6 km away the
  instant someone eats or drinks; carrying has no mass except for logs; buildings appear free.
- **The living world is one plant-carbon field and one assumed grazer pool.** NPP is set at
  bootstrap and ignores season and soil; people eat whole-cell biomass with no edible fraction;
  grazer faeces and deaths leak out of the books; nothing decomposes.

Rules for every step: each number is re-read at its source before it enters a pack (many
research figures came from search summaries and are tagged so); a departure is a deviation row;
changes to imported human code are recorded in `UPSTREAM.md` for the owner's upstream PRs; the
week digest is re-pinned only after the audit and both reference-pack slow tests pass.

Ordered by what most changes the founders' daily life:

| # | Step | Research | State |
|---|---|---|---|
| R0 | Honest register: deviation rows for every mind and sense gap; book grazer faeces and deaths to detritus; remove or wire dead code | humans §1.11, earth §4 Step 0 | Done: D44-D54 record the mind and sense gaps; litter, faeces and carcasses feed RothC soil carbon (`regional/soil.rs`, D43); the unused sparse plant view is gone |
| R1 | Water at home: a rainwater tank filled by simulated rain on the house roof, sized from the estate's own climate (ESR 2021 use, Harrison Grierson 2011 inflow model); people away from home drink where they find water; a dry tank means fetching by hand | work §3.2 | Tank done (PR #10, D42); R1b fetching by hand with two 20 L jerrycans when dry done (PR #12) |
| R2 | Mass-based carrying and real containers: load limits by body mass, pace and energy by load | work §3.1, §2.2 | Open |
| R3 | An edible food base: edible fractions, foraging with a food store at home, seasonal and soil-limited plant production (D31) | earth Steps 1-2, work §3.3 | Open |
| R4 | Firewood, drying (D40), fire and cooking | work §3.4, §2.5-2.6 | Open |
| R5 | Every action takes its real time (D9): gather, mine, build with delivered materials and visible progress, craft chains; skill improves with practice (power law) | work §3.5-3.8 | Open |
| R6 | Senses and perception as things (sight, hearing, smell, taste, touch and pain from injury), memory of places, and choices that use memory, emotion, goals and personality | humans ranks 1, 3 | Open |
| R7 | Emotion by appraisal with mood; per-person bonds and cooperation; teaching; dependent children who learn and acquire language; invention from experience, not a timer | humans ranks 2, 4-6 | Open |
| R8 | Game and hunting, fish and shellfish, farming and livestock | earth Steps 3-4, §2.5 | Open |
| R9 | Thermoregulation from sources, clothing and shelter; injury, accident and illness by mechanism; sleep stages | humans ranks 7-9 | Open |
| R10 | Disturbance and succession, fire, predators, pollinators, disease of plants and animals | earth Steps 5-7 | Open |

Still unsourced and needing a second pass before use (not to be typed in): edible and carcass
fractions, fruit and seed yields, foraging return rates, firewood per person per day, cordage
rates, novice-to-expert ratios, smell and taste thresholds, clothing insulation, bird and
invertebrate densities, fire danger indices (see each report's "unsourced" section).

## Housekeeping

- **The repository is public.** The licence stays all rights reserved (`LICENSE`), so the code is
  readable but not licensed for reuse. Two things follow:
  - The founders' GLB models (`assets/humans/GemD.glb`, `GemK.glb`) have unverified provenance
    (`assets/CREDITS.md`) and are now distributed. **The owner should confirm their origin**, or
    replace them.
  - The Phase 5 portraits must have their provenance confirmed **before** they are copied in.
  - A scan of the whole git history on 2026-10-13 found no keys, tokens or private keys;
    `deploy/island.env` holds empty token placeholders. Keep secrets in environment variables.

- **Nightly slow job:** green on 2026-10-13 (58 minutes), `island_ui` included: Bevy builds
  without ALSA or libudev headers now that its audio and gamepad features are off. The same
  night's `dependency audit` was red on RUSTSEC-2026-0344 (ruzstd), fixed in PR #15; that PR's
  `.cargo/audit.toml` ignore is to be removed once a `bevy_image` release takes ruzstd 0.9.1.
- **Remove the Snyk GitHub App** (owner's action). It is out of quota and posts an error on every
  push; the `dependency audit` CI job replaced it.
- **Firecrawl credits ran out** on 2026-10-12. WebSearch and the TinyFish fetcher still work for
  sources; the container's own network blocks most hosts.
- **Persist digests.** `WEEK_DIGEST` is `54b48e5b…bb18`; the planetary topology pin is
  `e4ece246…73e2`. Any change that moves either is a behaviour change and must say so.
