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
| Timber | Routes, sawing logs for the ute (D40); drying and seasoning done 2026-10-14, splitting, two-person crosscut felling and excavator rigging 2026-10-23 (D63) | Open | — |
| Living world | Phase 3 Task 1b: species at island scale | Done (D55 open: upstream densities) | — |
| Living world | Phase 3 Task 2: every resource and material, placers, recipes | Catalogue, mineral sites, gold placers, ironsand done (2026-10-14); diamond and platinum placers, animal, fish and plant nodes, salt works (2026-10-23); other gem and tin placers, tools, recipes open | Cited gem/tin placer models, shaft depth and rate |
| Living world | D9: actions take one tick | In part: felling, hauling, splitting, shed, panning, foraging and water trips timed, learned by practice (R11); Craft, Explore and social actions one tick | Cited durations per task |
| Living world | D31: NPP ignores soil nutrients | Open | Regional soils model and data |
| Runtime | Phase 4 Task 2b: deep-time spin-up | **Done** (2026-10-14) | — |
| Industry | Phase 4b Task 1 remainder: reference tables | Done 2026-10-14 (nine packs); gaps listed in §4a | A few items (see §4a) |
| Industry | Phase 4b Task 4: oil and gas field (D14, D19) | Fields, decline and carbon done 2026-10-14 (D58); in `IslandLife` with the diesel depot 2026-10-22; wells, water cut, spills open | Per-well rate, water-cut series |
| Industry | Phase 4b Task 5: refinery, gas plant, fuel depot (D16) | Topping refinery done 2026-10-14; gas plant, reformer, depot open | Reformate yield; gas-plant own use |
| Industry | Phase 4b Task 6: jobs, trades, maintenance (D39) | Open | Upstream human-runtime change first |
| Industry | Phase 4b Task 7: money, enterprises, payroll | Money, payroll and metered bills done 2026-10-18 (`regional/economy/`, D61, D62; `tests/island_economy.rs`). In `IslandLife` 2026-10-22 (`economy/island.rs`: deposits, Gem Petroleum selling the depot's diesel, monthly loans, closes, dividends, money audit); the petroleum business's staff and payslips, household bills, buying and selling homes open | Task 6 staff; upstream buying |
| Industry | Phase 4b Task 7b: food and timber chains (forestry, sawmill, house framing, farm, livestock, food processing and fishery models 2026-10-23) | Models done; not yet on the map or wired into island life | Task 8 workforce: who works a farm, mill or salt works, chosen by the people themselves (R14), never assigned |
| Industry | Phase 4b Task 8: town and workforce | Founding households, the smallest practice and a staffed school done 2026-10-23 (`regional/town.rs`, D67); other enterprises' minimum staff, the town in `IslandLife`, homes and growth open | A sourced minimum staff for the oil, gas, power, bank and store businesses |
| Industry | Phase 4b Task 9: dashboard pages, year acceptance | Industry (with payroll), Power, Timber and Food panels done 2026-10-23; Town page open (needs Task 8) | Tasks 4-8 |
| Climate | D3 rest: sea/land breezes, katabatic winds | Open | A diurnal wind solver |
| Climate | D25: SST season lags too long | Diagnosed 2026-10-14: 0.281 of the year against 0.18-0.21 observed | A prognostic marine boundary layer (with D29) |
| Climate | D4: parametric weather systems | Open | Structural; large |
| Climate | D38: a quarter of the land too dry | Diagnosed; specific humidity carried (2026-10-14) | A flux-driven rain-out (with D4) |
| Climate | D29: energy audited per module | Water audited end to end every hour (2026-10-15); energy open | End-to-end energy ledger (with D3's prognostic surface energy balance) |
| App | Phase 5 Tasks 3-8 | Partly done | See below; one owner pick (portraits) |
| Gates | Gate 1, 2, 3 owner reviews | **Closed** (owner, 2026-10-13) | — |
| Upstream | PRs for every divergence in `UPSTREAM.md` | Open | Owner's upstream access |
| Realism | R0-R10: humans, work and the living world replicate Earth (section 8) | R0, R1 and R1b (fetching water by hand) done; R8 in part and R11 (learning by doing) started 2026-10-23; R12-R14 with the other session; R3 next | Engineering and sourcing |
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

- **Splitting firewood (done 2026-10-23, D63):** once the shed stands, founders who choose to
  build split the store's air-dry wood by the house at Höldrich et al. 2007's 2.33 labour-hours
  per solid m³ (`labour/timber.json`), at the Compendium's chopping and splitting MET; the
  cooking fire burns only split wood. Left: a split-only axe rate for a long day, and stacking.
  (Checked 2026-10-23: the Finnish Work Efficiency Institute's comparison of five firewood
  methods, ETDEWEB 5174992, gives 0.95 m³ per effective hour for chainsaw cross-cutting plus axe
  splitting together, m³ not stated as solid or stacked, so it is not a split-only rate.)
- **The two-person crosscut saw (felling done 2026-10-23, D40):** a founder gathering with no
  tree of their own takes the other end of the shed's saw at a tree one other founder fells,
  and two at one tree fell it at the pack's two-person crosscut time (`labour/productivity.json`, 8-25 min for
  30 cm; FAO Forest Harvesting Case Study 18 measured two-cutter crosscut crews at 1.11-1.99
  m³/h). Left: cross-cutting a log for the ute with the saw rather than the axe.
- **Excavator rigging (done 2026-10-23, D40):** the CAT 330 has no grapple, so its operator
  slings each log by hand before the pick-up and frees it after the set-down, at Borz & Mititelu
  2022's measured hook and unhook times for hand-rigged winch logs (13.59 s and 9.30 s, 278 logs,
  `labour/timber.json`), and the operator's walk from the cab to the log and back each time, the
  log lying at the 6 m radius the CAT's lift is rated at (2026-10-23).
- **Routes:** legs are straight lines. Walks climb the ground's rises along them (Naismith,
  `EstateLayout::ground_m`, 2026-10-14); obstacles are ignored, vehicle times ignore slope, and
  vehicles have no position of their own, though one a founder is hauling with is no longer
  free for another (2026-10-23). A route over the patch terrain with a
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
animal and fish resource nodes (done 2026-10-23, §2b).

### 2b. Phase 3 Task 2: every resource and material

- **Done (2026-10-14):** the material catalogue (`materials/catalogue.rs`, densities and hardness
  cited) and `regional/resources.rs`: every primary deposit as a site; placer gold below each
  gold-bearing deposit where the river's gradient flattens (Orris & Bliss 1986, USGS model 39a,
  read from the bulletin); ironsand at the mouths of rivers draining arc andesite (Christie et al.
  2024, GNS 2024/62A); sites deplete and never refill; panning with a pan or sluice through the
  labour model. The deposit models' medians and spreads now match the bulletin's figures (Fe skarn
  50% Fe, phosphorite 330 Mt at 25% P2O5, placer gold 0.2 g/t were corrected), and a unit test
  holds them to the pack. D56 records what the sites leave out.
- **Left:**
  - Diamond placers below kimberlites are sited (2026-10-23) from Bliss's (1992, USGS OFR 92-437)
    model 39d grade and volume, inland only; working them needs a jig or a pan's concentrate sorted
    by hand, with cited rates. Platinum placers are sited below ultramafic bodies (2026-10-23)
    from Singer and Page's model 39b (USGS Bulletin 1693, 83 Urals placers), inland, platinum
    only, and refused like the diamonds until a working rate is cited. Placer tin (39e) has only
    a descriptive model, no grade or tonnage. Gem placers below the other gem deposits still need
    a source.
  - Hand mining of primary ore: a cited artisanal shaft or adit depth and rate (refused by name
    until then).
  - Animal and fish nodes (2026-10-23): `IslandBiota::animal_nodes` gives each cell's animals
    as views of their species' abundance there (a grazer's share of the cell's grazer carbon,
    any other animal at upstream's density over the cell), adding up to the species on the
    island; hunting and fishing them, and the fishery model on the sea's fish, are still to
    come, and the densities stay upstream's (D55).
  - Salt (2026-10-23): `industry/salt_works.rs` makes solar salt at Dominion Salt's Lake
    Grassmere yield per hectare of production area (pack, 42.4-49.4 t/ha/yr); it ignores the
    rain that swings Grassmere's harvest from nil to 130,000 t, and no works is sited yet (a
    sourced siting rule for coastal flats and a dry climate is needed).
  - Plant nodes (2026-10-23): `IslandBiota::plant_nodes` gives each plant species' share of a
    cell's living biomass carbon, adding up to the species on the island.
  - Water sources: already in the model, not as a separate node. `physical.river_flows`
    (`industry/flow_duration.rs`) holds a flow-duration curve for every perennial medium cell,
    and `RiverFlowRecord::is_river` marks those at the hydrology's 1 m³/s river threshold;
    `FlowNetwork::lake_depth_m` gives the lakes, and each house has its roof tank
    (`household_water.rs`). People already walk to the nearest liquid water, drink and carry
    jerrycans home for the tank (`WaterTrip`, R1b).
  - The new tools in `resource_economy.rs`, smelting and the other recipes, mass-balanced through
    the material ledger.
  - Serving the sites to the map and the founders' choices.
- **Then:** record the enums, tool map and every abundance source in `UPSTREAM.md`; re-render
  `island_preview life` so `resources.png` shows the nodes.

### 2c. D9: most actions still take one tick

- **Now (2026-10-23):** felling (axe or two-person crosscut saw), hauling (ute or excavator with
  rigging), splitting firewood, building the shed, panning a placer for gold, foraging (R3, Kelly
  2013 return rates) and water trips (R1b) are timed and costed, and each is slowed for a
  beginner and quickened by practice (R11, D64). Still one tick: Craft (upstream's instant
  recipes: planks, masonry, rope, fuel, ingots, tools; on the island nobody carries their inputs
  yet, since timber and gold go to the estate's store), Explore and SocialApproach (moves paced by
  the cell crossing only), Transfer, Harm and Intimacy. Timing Craft is not a matter of a rate:
  upstream's recipes are unit counts with no masses (3 wood make 2 planks), so a duration per
  recipe would have nothing to scale with. It needs upstream recipes in kilograms first, then a
  cited rate per kg (checked 2026-10-23).
- **Needs:** a cited duration for each action (`fixtures/reference/labour/productivity.json`
  holds some already), and the same task machinery felling uses (`life.rs` `work`) extended to
  them. Actions with no Compendium row cost the resting baseline (D22, accepted).
- **Done when:** every action a founder can choose takes a sourced time and MET.

### 2d. D31: NPP ignores soil nutrients

- **Needs:** a regional soils model (parent rock from `geology.rs`, age from the deep-time
  spin-up in §3, nitrogen and phosphorus) and a cited nutrient limitation on NPP. Large; it
  follows the spin-up.
- **Checked 2026-10-23:** Porder and Ramachandran (2013, Plant Soil 367:41-55,
  doi:10.1007/s11104-012-1490-2) give rock phosphorus from 263,539 EarthChem analyses. Their
  abstract states medians for granite (436 ppm), andesite (1,000), mudstone (1,135), siltstone
  (698), sandstone (500), several ultramafic rocks (120) and several alkali basalts (>3,000), and
  that parent rock explains 42% of soil total P across 62 sites. The full table (basalt, schist,
  gneiss, greywacke, limestone, marble, rhyolite: the rest of `geology.rs`'s lithologies) is
  behind the paywall and not read. Still missing as well: no source found yet turns soil P into
  a limit on NPP, so a phosphorus field alone would change nothing.

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

**Done 2026-10-14:** nine cited packs, every value read at its source with a quote or table
reference in its note:

- `industry/petroleum.json`: recovery factors by drive, gas-oil ratios, the world water-oil ratio,
  decline rates (Höök et al. 2009; IEA WEO 2008 via UKERC), Arps b (low), methane loss (Alvarez et al.
  2018), blowout frequencies per well and well-year (BOEM 2014 from SINTEF), well spacing, and New
  Zealand's field sizes (MBIE 2019 via Wikipedia).
- `industry/refinery.json`: the Equinor Troll Blend assay (cuts, densities, sulphur),
  hydroskimming against cracking yields (low), refinery energy and CO2 (Jing et al. 2020;
  Concawe 2018), NZ diesel density, IPCC 2006 calorific values and carbon contents.
- `industry/gas.json`: Kapuni raw gas, NZ sales-gas ranges, Kupe's products, IPCC calorific values,
  US NGL shrinkage.
- `industry/staffing.json`, `industry/wages.json`, `industry/demand.json`: dairy and sheep-and-beef
  labour, GP list size, Marsden Point's staff and throughput, supermarket staff; Stats NZ wages by
  occupation and industry, the 2026 minimum wage, leave and KiwiSaver; household electricity, the
  HEEP end-use and fuel split, driving per year and vehicles per person.
- `industry/agriculture.json`, `industry/construction.json`, `humans/demography.json`: NZ dairy,
  sheep, beef and pig production, pasture growth, crop yields, maize thermal time, Pukekohe's
  urban area; sawmill recovery (low), a reference house's areas, framing labour rates, new-house
  floor areas; NZ age-sex structure, household sizes and labour-force participation.

**Not found, still needed:** a per-well production rate and water-cut history (Task 4); a primary
refinery energy figure per tonne and gas-plant own use (Task 5); staffing for hospitals, sawmills,
gas plants and hydro stations, Marsden Point's nameplate capacity (Task 6); transport fuel per
vehicle in litres (Task 8); a minimum breeding herd, lamb slaughter age, NZ pig carcass and intake,
beef intake, wheat and pasture degree days (Task 7b); framing m³ per m² and whole-house labour by
trade, a 2-3 bedroom cottage's floor area (Task 6 and 7b); the Pukekohe growing district's area.

### 4b. Task 4: oil and gas field (D14, D19)

- **Done 2026-10-14** (`regional/industry/petroleum.rs`, `tests/petroleum_field.rs`): every
  petroleum deposit is a field the size of one of New Zealand's nine (the owner's island has three,
  like Kapuni, McKee and Maari), declining exponentially at 4.4% a year, exact for any step length;
  its carbon comes out of the crust into the field's tanks, and 2.3% of its gas leaks as methane into
  the new `AtmosCH4` reservoir (`UPSTREAM.md`).
- **Also done:** `regional/industry/chain.rs` runs the fields and the refinery as one plant, choked to
  the plan's default 500 bbl/day nameplate, its material carbon auditable step by step; leaked
  methane oxidises with its 11.8-year lifetime (IPCC AR6). Packs gained reformate yields (Speight),
  gas-plant fuel and shrinkage (EIA), methane lifetimes, alluvial diamond grades (low), Lake
  Grassmere solar salt and hand-dug shaft depth and sinking rate (a well analogue, low).
- **Done 2026-10-22:** `IslandLife` carries `IndustryChain` (state, snapshot, digest), stepping it
  inside the material audit, which now counts the chain's tanks and products and island-refined
  fuel; a depot fills the estate's diesel back to the scenario's opening stock from the refinery's
  distillate, and generators and vehicles burn that share as material carbon
  (`energy::IslandFuel`, `book_island_burn`).
- **Left (D58):** a plateau, reservoir pressure and water cut; wells, workover crews and blowouts
  (the frequencies are in the pack, but there is no per-well rate to count wells by); spills and their
  harm to shellfish; methane warming (negligible at island scale, Etminan et al. 2016);
  who runs the field (Task 6). D19 (drill rigs cannot be made on the island) stays the owner's call;
  the default is imported equipment, as D15 accepts imported plant.

### 4c. Task 5: refinery, gas plant, fuel depot (D16)

- **Done 2026-10-14** (`regional/industry/refinery.rs`, `tests/refinery.rs`): a topping refinery
  distils crude into the Troll assay's atmospheric cuts and names each by where it boils, against
  New Zealand's fuel specifications (petrol's 210 °C final boiling point, diesel's 360 °C T95). It
  burns its own refinery gas, then residue, for the world's average 606 MJ a barrel (Jing et al.
  2020): about 11% of the crude and 43 kg of CO2 a barrel, inside Downey's 8-12% and Jing et al.'s
  13.9-62.1 kg. Mass and carbon close (IPCC carbon contents, scaled by one factor of 0.974). Crude
  comes from a field's tanks (`PetroleumField::take_oil`).
- **Left:** the gas plant (LPG and condensate from raw gas; gas-plant own use is not sourced), a
  reformer for petrol (needs a cited reformate yield), the fuel depot and vehicles and generators
  buying fuel (`regional/energy.rs`), campaigns that stop on full tanks, and payment (Task 7).

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
- **Decided (owner, 2026-10-15):** every enterprise is owned **50/50** by Gem-D and Gem-K;
  **50%** of each month's profit is paid out and the other half kept by the business to grow;
  the **bank takes deposits and lends** (to villagers buying homes or businesses, at a sourced NZ
  rate); each founder starts with **NZ median household net worth** (sourced, from Stats NZ).
- **Disconnection (owner, 2026-10-15):** a household whose power and water bills have gone a
  **month** unpaid is cut off from both, **unless** it is still making regular payments and
  trying to keep up, in which case the supply stays on. So the test is the payment record over
  the month, not the balance owed: a household paying something each billing period keeps its
  supply while it catches up; one that has stopped paying for a month loses it.
- **Files:** create `regional/economy/{money.rs,enterprise.rs,market.rs}`; upstream first for
  `ActionKind::Buy` and money in human decisions.
- **Done 2026-10-18:** `money.rs` (accounts, issuance, the bank's loans as created money, annuity
  instalments at RBNZ's 4.82%, the disconnection rule, dividends; D61), `enterprise.rs` (hiring at
  no less than the 23.95/h minimum wage, Stats NZ median rates by industry and occupation, payroll
  every 14 roster days with payslips, 3.5% employer KiwiSaver, four weeks' leave accruing with
  hours, wages owed when a business cannot pay, monthly close on cash profit; D62) and `market.rs`
  (meters and monthly bills at MBIE's 42.1 c/kWh and Watercare's 2026-27 water and wastewater
  charges, `industry/prices.json`). `tests/island_economy.rs` runs a local year: money conserved
  every day, payroll every 14 roster days, dividends equal profit x payout x share, a household
  that stops paying is cut off.
- **Done 2026-10-22:** `IslandLife` carries `IslandEconomy` (`regional/economy/island.rs`) in its
  state, snapshot and digest. The books open with each founder's sourced deposit and a Gem
  Petroleum business, owned 50/50, when the island has oil. Each local month the bank takes its
  instalments, every business closes its month and pays dividends, and the money audit must close.
  The estate pays Gem Petroleum for each depot delivery at MBIE's weekly adjusted retail diesel
  price (16 October 2026, 321.7 c/L, `industry/prices.json`), the founders half each; the depot
  delivers only what they can pay for.
- **Left:** the petroleum business hires and pays staff (Task 6), and writes payslips to the person's folder;
  household billing once the town has households that buy power and water (the founders' estate
  makes its own, Task 8); Gem-D and Gem-K offering
  homes and businesses for sale and villagers deciding to buy (upstream first).
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
- **Done 2026-10-23:**
  - `forestry.rs`: even-aged radiata pine stands clearfelled at the NEFD's 28.2-year average age
    for 486 m³/ha (Lane 2007) and replanted, so harvest never exceeds regrowth (unit tests).
  - `sawmill.rs`: logs sawn at New Zealand's 1.81 m³ of roundwood per m³ of sawnwood (FAO/ITTO/UN
    2020). The rest is shared in North America's residue proportions as a stand-in, since New
    Zealand reports no material balance. The pack item had mislabelled those shares as New
    Zealand's and is corrected.
  - Carbon: wood carries radiata's basic density 0.38 t/m³ times a conifer's 0.51 carbon fraction
    (IPCC 2006 Tables 4.14, 4.3). `tests/timber_chain.rs` checks a century of plantation and mill:
    no overcut, and no carbon lost from felling to the mill's gate.
  - `construction.rs`: the reference house's wall frames (thinkstep 2022: 325 m² of framed wall,
    18% timber, 90 mm deep) built from the mill's timber, carbon kept from tree to house.
  - `farm.rs`: dairy land at the national 2.76 cows/ha and 400 kg of milksolids a cow (LIC and
    DairyNZ 2023-24, which checks against their 1,105 kg/ha), crops at FAOSTAT's 2023 New Zealand
    yields per hectare (vegetables are FAO estimates).
  - `livestock.rs`: ewes and cows at Beef + Lamb's Class 5 lambing (140.3%) and calving (87.5%);
    ewe lambs replace the 30% of ewes lost each year (Ridler's 25-35%, Country-Wide 2019) and the
    other lambs are killed the same season at 19.93 kg; calves at 20 months at the steer's 316 kg.
    No cattle replacement rate has a source yet (RB35's 13% counts only heifers calving at two),
    so every calf is finished and the cows are held constant; cull ewes' mutton is not counted.
    Sows farrow all year at NZ Pork's 1.7 t of carcass per sow (2022 annual report) at 71.1 kg a
    pig, killed at 5.5 months; the sow herd is held constant (no sow replacement rate read).
  - `food_processing.rs`: the farms' wheat, barley and oats milled and malted, and their whole milk
    (FAOSTAT 2023: 4,593 kg a cow) made into cream, butter, cheese, powder and so on, at New
    Zealand's extraction rates in FAO's Technical Conversion Factors (wheat 78% flour and 22%
    bran, butter 6.1% of milk). `tests/food_chain.rs` follows wheat to flour and milk to butter.
  - `fishery.rs`: a stock in Schaefer surplus production with depensation below a quarter of
    unexploited biomass, `r` the middle of Froese et al. (2017) Table 2's prior for its FishBase
    resilience class; the catch is capped at the surplus, so the fishery never takes more than
    grows. Not yet fed by the island's fish: their densities are upstream's (D55) and no species
    carries a resilience class.
  - The reference house's floor is a concrete slab on grade (thinkstep 2022 Table 2-2), so it has
    no floor timber. Its roof is "timber roof trusses (70x45mm SG8 H1.2 treated purlins)" over
    263 m², but the report gives the roof only as kg CO2e per m² (Table 6-7), not a timber volume.
  - Still open: roof truss and other timber in the house (no volume source found), the plantation and
    farms on the map fed by the existing felling and hauling, a beef herd's replacement rate (the
    dairy herd's is DairyNZ's 22% a year; the ewe flock's Ridler's 25-35%; no beef-cow figure
    found), the fishery on the island's own fish, bread and other second-stage foods (neither
    FAO's conversion factors nor USDA AH-697 give a bread yield), the plants' energy use, and the supermarket.

### 4g. Task 8: town and workforce

- **Size (owner, 2026-10-15):** there is no target size. The town starts **tiny**: each
  enterprise opens with the smallest staff that can run it (two doctors for the whole town, for
  example, sourced per enterprise, not invented), plus their households. It grows as the
  businesses do: a business that needs more hands hires, and pays builders to renovate and expand
  its premises, so the town's size is an outcome, not a setting.
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
- **Founding households (done 2026-10-23):** `regional/town.rs` founds one household per
  opening from the seed: the worker 20-64 by Stats NZ's 2018 ERP, the household's size by the
  2023 Census, a partner from the worker's band and children under 18 weighted by natural
  fertility at the mother's age (D67). `found_from_reference` adds the smallest general practice
  (one GP, HTRHN 2024, and two nurses at the average rural practice's nurse-to-GP ratio, HTRHN
  2023) and a school with a teacher per 25 children of school age (Ministry of Education's
  small-school staffing). Tests in `regional::town` and `tests/island_town.rs`. **Left:** the
  oil field's, gas plant's, depot's, bank's, store's and power company's smallest staff (no
  headcount found: Kupe about 35 and Tongariro hydro 40-45 are unverified); the town's people
  as humans in `IslandLife` with homes, hired through `Enterprise::hire`; growth by hiring.
- **Where (2026-10-23):** `property.rs` `town_site` puts the town on the dry cell nearest the
  town hydro plant's powerhouse, outside the estate's 2 x 2 block, the same cell for the same
  island. The town follows its power; nobody chose the cell. Island life founds the town at
  bootstrap (`install_town`, also on restoring an island saved without one): `town.rs`'s first
  staff and households are sampled whole through upstream's spawn (`settle_in_town`, keyed by
  their ids, born their age before the island's clock), stand on the town cell, have their
  bodies on the material ledger, and live outside the founders' household (`townsfolk`). Gem
  Health and Gem School open, Gem-D and Gem-K's, and hire their staff at the industry's ordinary
  hourly rate (Stats NZ QES 2026 Q2: health care and social assistance, education and training).
  Staff hours: a member of staff who is awake in the town and whose own choice is a working
  action (gather, mine, build, craft, transfer, code; not rest, food, water, shelter or company)
  works that time at their job, on the books for the next pay run; nobody is rostered (R14).
  Left: houses laid out on the town cell (the townsfolk sleep on its open ground until then),
  the practice's and school's income (fees and government funding, unsourced) and so a pay run,
  and the town's other enterprises' openings.
- **Files:** create `regional/town.rs`, `fixtures/island/workforce.json` (generated from the seed);
  generalise `regional/estate_layout.rs` to any high-detail site.
- **Tests:** `tests/island_town.rs`.

### 4h. Task 9: dashboard pages and the year's acceptance

- **Pages:** an Industry panel (2026-10-23: `GET /api/industry`, the fields, the plant's store,
  the accounts, loans and each business's last month close, with the money audit) is served;
  so are Power, Timber and Food panels and the latest payslips (2026-10-23: `/api/power`,
  `/api/timber`, `/api/food`, `payslips` in `/api/industry`). The owner's controls (pause,
  resume, step, snapshot, speed) already reach the island only as `IslandCommand::Control`,
  which the replay log records (Phase 4 Task 4). The Town page (Task 8) remains, and so does any
  industry control the owner wants (a price, opening a business) as a new recorded command.
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
| 3 | Individual stems in 3D around the yard are drawn (2026-10-14, `stems.rs`: up to 3,000 trunks within 250 m, at their own height and diameter); left of 2a: stand cover and instanced trees beyond the patch from biomass. Interiors (2b), industry and town (2d, after Phase 4b), resource visuals and inventory icons (2c, after Phase 3 Task 2). Every building kind and vehicle is tested (2026-10-23): `property.rs` holds its kind lists to the founders' estate and checks each vehicle's model, and `tests/reading_an_island.rs` draws every building and item from a running island | Engineering |
| 4 | Done 2026-10-14: the person inspector (the backend's sections, with what they are doing and thinking, and family), the foundry (create somebody from the desktop), and the founders' portraits (the owner chose upstream's `new-gem-d` and `new-gem-k` and confirmed their provenance; `assets/humans/portraits/`). Nothing left | — |
| 5 | Done 2026-10-14 for the clock, controls and terrain cell (`apps/island_ui/src/inspectors.rs`): six speeds with requested and achieved, Save, the local date (year, day of 288, time of a 36-hour day) with Earth time elapsed, and the cell under the view with its biome, climate and production. A click on the estate picks a building, room, item or tree and the right panel describes it (`estate.rs` `pick`, `stems.rs` `describe`). Nothing left | — |
| 6 | Done; the owner accepted the default targets on 2026-10-14: `island_bench` and `benchmarks/2026-10-08-regional-baseline.json` (12.9 simulated days per real minute headless against a default target of one, so no fallback profile), the allocation-bound test (`tests/island_allocation_bounds.rs`, `construction-counters`), and formal predictive processing measured at 0.9% of the human step (`tests/island_dead_compute.rs`), so keeping it running costs little. The plan's Task 6 status has the detail | Targets confirmed: headless one simulated day per real minute, real time with the UI open. Formal predictive processing: the owner decided on 2026-10-14 to wire it into decisions, and it is (2026-10-15, `UPSTREAM.md` production item 6): what its critic learned each action is worth steers `AutonomousMind::choose_action`, and its critic learns from the action actually taken |
| 7 | The sync script is done (2026-10-14, `scripts/sync_maerken_upstream.py`, `UPSTREAM.md` § Sync procedure). The creator moved into `apps/island` (`island::population`) and `apps/island_humans` is retired. Pruning is the owner's call (the inventory recommends not to) | Engineering |
| 8 | Final acceptance: every design criterion run and recorded | All of the above |

A GPU frame-rate figure for `island-ui` is measured (2026-10-15, RX 470, 1600×1000, release,
without vsync): 255.6 frames/s on the island view and 508.9 on the estate, against 0.6 and 10.8
on the CPU (`docs/island/RENDER_STACK.md`). Other cards and window sizes are not measured.

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
| R2 | Mass-based carrying and real containers: load limits by body mass, pace and energy by load | work §3.1, §2.2 | In part (2026-10-14): load limits by body mass; pace under load from measured self-selected speeds; energy by load from Pandolf's equation for water fetched by hand. Open: a general mass inventory of what each person holds, and baskets, sacks and packs |
| R3 | An edible food base: edible fractions, foraging with a food store at home, seasonal and soil-limited plant production (D31) | earth Steps 1-2, work §3.3 | Started (#42, D57): people eat only a forest's fruit and seed, grown at NPP times Ward et al. 2025's reproductive allocation, not its wood. A meal is now gathered before it is eaten: the walk to the food and back, and the meal's energy over Kelly's (2013, Table 3-4) acorn return rate, 821 kcal/h, at the Compendium's foraging MET. At home the household keeps a food store: one trip gathers what everyone at home owes, carried back within the forager's sustainable load (R2), and the others eat from the store (2026-10-15). Search time on top of Kelly's post-encounter rate: 0.5 h per hour of handling, from O'Connell and Hawkes's (1984) Alyawara fruit trips (2026-10-22). Left: search that slows as the cell's food runs low, spoilage, bracken and raupo once their extent per cell is sourced, seasonality, soil limits (D31) |
| R4 | Firewood, drying (D40), fire and cooking | work §3.4, §2.5-2.6 | In part: logs dry (D40); the kitchen's gas stove burns the house's average cooking LPG from two bottles (D59, 2026-10-14); when the bottles are empty, a wood fire burns the store's air-dry wood at kitchen-performance-test rates (1.74-1.98 kg per adult equivalent per day). Open: meals cooked at times of day, fire tending and fetching time. Splitting done 2026-10-23 (D63): only split wood burns |
| R5 | Every action takes its real time (D9): gather, mine, build with delivered materials and visible progress, craft chains; skill improves with practice (power law) | work §3.5-3.8 | Started (2026-10-14): mining a placer is timed (D9); the first building, a 10 m² firewood shed, takes its timber from the store's seasoned wood and the packs' timber-frame hours with the powered saw (D60); the starting estate counts as pre-built (D60, the owner's ruling). Open: gathering clay, sand, gravel and stone, a covered stack's drying, craft chains, skill |
| R6 | Senses and perception as things (sight, hearing, smell, taste, touch and pain from injury), memory of places, and choices that use memory, emotion, goals and personality | humans ranks 1, 3 | Open |
| R7 | Emotion by appraisal with mood; per-person bonds and cooperation; teaching; dependent children who learn and acquire language; invention from experience, not a timer | humans ranks 2, 4-6 | Open |
| R8 | Game and hunting, fish and shellfish, farming and livestock | earth Steps 3-4, §2.5 | In part (2026-10-23): animal and fish nodes per cell from `regional/species.rs`, a Schaefer fish stock (`industry/fishery.rs`, Froese et al. 2017) and farm and livestock models (§4f), none worked by anyone yet. Hunting needs a choice to hunt from `humans/autonomy.rs` (no hunt action exists; R14), encounter rates from the nodes' densities, and post-encounter return rates, which Kelly (2013) Table 3-4 lists (deer 17,971-31,450 kcal/h, rabbit 8,983-9,800, trout by spear 17,700-24,400, shellfish 300-14,100) |
| R9 | Thermoregulation from sources, clothing and shelter; injury, accident and illness by mechanism; sleep stages | humans ranks 7-9 | Open |
| R10 | Disturbance and succession, fire, predators, pollinators, disease of plants and animals | earth Steps 5-7 | Open |
| R11 | Everything is learnt (owner, 2026-10-23): no one is born able to do a task; each skill rises with every repetition (a learning curve), making the task faster and its result better, and fades with disuse; free will picks what to do from what a person needs and wants. Takes over R5's "skill improves with practice". Distinct from Task 6's `TradeQualification` (a credential, not a skill) | Needs sources: learning-curve exponents per kind of task (only crosscut felling found), quality with skill, teaching | Started 2026-10-23 (D64): everyone learns felling, splitting, panning, foraging and building by doing, from one sourced curve, and forgets with disuse. Open: per-task curves, quality, teaching, other tasks |
| R12 | The body trains (owner, 2026-10-23): physical work builds stamina (aerobic capacity) and muscle; lifting raises the load a person can lift; inactivity and eating beyond need do the opposite (detraining, fat gain); body composition changes how a person looks. R2's load limits then follow each person's own strength | Needs sources: VO2max and strength gains with training load, detraining rates, muscle protein accretion and loss, energy surplus to fat mass | Open. Upstream first (human runtime) |
| R13 | Genetics (owner, 2026-10-23): a DNA-like genome, half from each parent, sets each person's starting point and limits (height, build, trainability, predispositions) | Needs sources: heritabilities of the traits modelled; touches birth (`reproduction.rs`) and how the population is stored | Open. Upstream first |

**Owner decision (2026-10-23), in the owner's words:** "the humans in this need to learn to do
things and have free will to decide what they need and want to do. all things must be learnt and
over time as they do something more often they get better and better. when they do physical
activities that increases stamina and muscle mass which changes physical appearance and if they
are lazy and do nothing but eat all day that does the opposite. if they lift things they get
stronger and can lift heavier things everything needs to be trained essentially like humans on
earth do. they also need to have a double helix like humans do." R11-R13 record it; each number
they need is sourced into `fixtures/reference/` before it is used.

Still unsourced and needing a second pass before use (not to be typed in): edible and carcass
fractions, fruit and seed yields, foraging return rates, firewood per person per day, cordage
rates, novice-to-expert ratios, smell and taste thresholds, clothing insulation, bird and
invertebrate densities, fire danger indices (see each report's "unsourced" section).

## Housekeeping

- **The repository is public.** The licence stays all rights reserved (`LICENSE`), so the code is
  readable but not licensed for reuse. Two things follow:
  - The founders' GLB models (`assets/humans/GemD.glb`, `GemK.glb`) and their two portraits:
    the owner confirmed on 2026-10-14 that all four are theirs to use and distribute
    (`assets/CREDITS.md`).
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
- **Persist digests.** `WEEK_DIGEST` is `679abc7e…8322`; the planetary topology pin is
  `e4ece246…73e2`. Any change that moves either is a behaviour change and must say so.
