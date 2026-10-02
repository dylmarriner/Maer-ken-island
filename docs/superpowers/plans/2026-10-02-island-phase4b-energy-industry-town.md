# Island Phase 4b: Hydro, Petroleum, Town and Economy Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A river beside the estate drives a hydro plant that powers the house and later a town; an oil and gas field, an oil refinery and a gas processing plant make the island's fuel; farms, a fishery, food processing and a supermarket feed the island; forestry, a sawmill and a building company supply and build its houses; a town houses the people who run all of it; and an economy with money, wages, prices and ownership pays every worker and pays Gem-D and Gem-K the profits.

**Owner decisions (2026-10-02):** river with hydro by the estate, powering the house and eventually a town; oil production, an oil refinery and a gas plant for fuel; houses and humans to operate and maintain them; Gem-D and Gem-K own the enterprises and receive the profit.

**Architecture:** Everything is built from the existing physical world, not placed by decree. The river is a real Phase-2 river; the oil and gas are a real Phase-1 petroleum deposit; electricity obeys `P = η·ρ·g·Q·H`; fuel comes out of crustal carbon and burns into the atmosphere; workers are complete humans created through the Phase-0b/4 creator path; money is double-entry and conserved. New human behaviours (going to work, buying, being paid) are added to the upstream human runtime first (Phase 0c rule) and synced.

**Upstream baseline (audited):** no money, prices, wages, jobs or trade qualifications (`SkillMatrix` holds cognitive aptitudes — analytical, verbal, physical… — not trades); settlements are `SiteKind::{Shelter, Settlement}` clusters of structures; property has no power or fuel.

**Tech Stack:** Rust, `mk_engine::regional`, the Phase-3 material ledger, labour and energy systems, the Phase-4 world, scheduler and command path.

**Spec:** `docs/superpowers/specs/2026-10-01-maer-ken-island-regional-world-design.md`

**Depends on:** Phase 4 (world, scheduler, commands, creator), Phase 3 Tasks 2–4b (materials, labour, energy).

## Global Constraints

- **Physical first.** Hydro output comes from simulated river discharge and real head; oil and gas output from a reservoir model with pressure decline; refinery products from a crude assay and mass balance; electricity from generation minus losses equals consumption every tick (or loads are shed).
- **Ledgers close.** Water diverted through a hydro plant returns to the river at the tailrace (the bypassed reach is drier in between). Oil and gas move `CrustCarbon → MaterialCarbon`; combustion, flaring and refinery fuel move carbon to `AtmosCO2`; vented and leaked methane goes to a new `AtmosCH4` reservoir with its own climate forcing (Myhre et al. 1998), so venting warms the climate realistically. Money is conserved: every payment debits one account and credits another.
- **Declared deviation:** turbines, generators, drilling rigs, refinery and gas-plant units cannot be manufactured on the island; they exist at scenario start as equipment imported before the simulation began. Spare parts are finite; wear and breakdowns consume them.
- **Right-sized.** Plants are sized to the island's real demand with the smallest equipment that exists in reality; where that equipment is far larger than demand (the refinery), it runs in campaigns and products are stored.
- **People are complete humans.** Every worker, spouse and child is a full upstream human with their own folder, created deterministically through `build_authored_human`. Their work, pay and spending go through their own cognition and needs, not scripts.

## Defaults (owner may change; recorded in the scenario)

| Item | Default | Basis |
|---|---|---|
| House hydro (stage 1) | run-of-river micro-hydro, 25 kW, Pelton or Turgo by head | estate peak load ~10–15 kW plus margin |
| Town hydro (stage 2) | small hydro on the same river, 2 MW, built when town peak demand exceeds 80% of stage 1 + backup | ~200-person town with workshops, sawmill, food processing, refrigeration and refinery loads |
| Environmental flow | ≥ 30% of mean discharge always left in the river | common minimum-flow practice |
| Oil field | 3 producing wells, separator, storage tanks, flowline to the refinery | smallest viable development |
| Refinery | skid-mounted atmospheric distillation unit, ~500 bbl/day nameplate, plus a small catalytic reformer for petrol octane | smallest real modular refineries |
| Gas plant | separation of methane, LPG (propane/butane) and condensate; methane to a gas-fired backup generator and town supply; LPG bottled | |
| Town | ~200 people in ~60 households: plant operators, electricians, mechanics, drillers and field crew, refinery operators; farmers, farm hands, fishers, a butcher, a miller/baker, supermarket manager and staff; forestry crew, sawmill hands, builders, carpenters, a plumber; a nurse and a teacher; with spouses and children | staffing from reference tables (Task 1) |
| Food | mixed farm (grain, vegetables, orchard, pasture with sheep and cattle, dairy, poultry) sized to the town's energy and protein needs, an inshore fishing boat, abattoir/butchery, mill and bakery, dairy shed, supermarket | ~2,300 kcal and ~60 g protein per person per day |
| Timber | forestry crew (chainsaws, skidder, log truck) on a sustainable-yield harvest plan with replanting, sawmill (logs → framing, weatherboards, flooring; offcuts and sawdust → firewood and boiler fuel) | |
| Currency | island dollar, integer cents | |
| Shares | Gem-D 50%, Gem-K 50% of every enterprise | owner decision |
| Prices | cost-based: levelised cost + 20% margin, reviewed each local month | |
| Wages | relative occupational wage ratios from the reference labour pack, scaled so the lowest full-time wage covers a household's food, rent, fuel and electricity with a margin; paid every 14 roster days for hours actually worked | |

---

### Task 1: Reference data and sizing

**Files:**
- Create: `fixtures/reference/industry/{hydro,petroleum,refinery,gas,staffing,wages,demand}.json`
- Create: `crates/mk_engine/src/regional/industry/sizing.rs`
- Test: `crates/mk_engine/tests/industry_sizing.rs`

- [ ] **Step 1:** Assemble cited reference values: turbine efficiency curves by type (Pelton, Turgo, Francis, Crossflow, Kaplan) and head range; penstock friction (Darcy–Weisbach); oil reservoir recovery factors and Arps decline parameters; crude assays (product yields for light and medium crude); refinery energy use and emissions per barrel; gas composition and processing yields; staffing per plant type and size; occupational wage ratios; household electricity, cooking and transport fuel demand.
- [ ] **Step 2:** Implement `size_plants(demand, river_reach, petroleum_deposit) -> PlantDesign` choosing equipment from the reference tables; test that the defaults above fall out for a ~200-person town.
- [ ] **Step 3:** Commit `feat(industry): reference data and plant sizing`.

### Task 2: River-side estate and hydro sites

**Files:**
- Modify: `crates/mk_engine/src/regional/property.rs` (`choose_regional_estate_location`)
- Create: `crates/mk_engine/src/regional/industry/hydro_sites.rs`
- Modify: `apps/island_preview` (gallery reports hydro and petroleum potential)
- Test: `crates/mk_engine/tests/hydro_sites.rs`

**Interfaces:**
- Produces: `HydroSite { intake_cell, powerhouse_cell, head_m, penstock_length_m, design_flow_m3_s, firm_flow_m3_s (95% exceedance), mean_flow_m3_s }` found by scanning Phase-2 river reaches for head and flow; `firm_flow` comes from a simulated year of discharge, not a single snapshot.
- Estate placement now also requires a perennial river within 1 km of the estate block whose reach supports the stage-1 design with environmental flow left, and a site on the same river (up- or downstream) for stage 2. If none exists the scenario fails clearly (`NoHydroSite`) rather than inventing a river.
- The Phase-1 gallery adds, per candidate island: best estate-with-hydro site, stage-2 capacity possible, and whether a petroleum basin exists.

- [ ] **Step 1:** Tests: estate block lies within 1 km of the chosen river; stage-1 and stage-2 sites satisfy head/flow with environmental flow; an island with no suitable river returns `NoHydroSite`; gallery JSON includes the new fields.
- [ ] **Step 2:** Implement; run `cargo test -p mk_engine --test hydro_sites` and `cargo test -p island_preview`; expect PASS.
- [ ] **Step 3:** Commit `feat(industry): place the estate by a river that can run hydro`.

### Task 3: Hydro plants and the electricity grid

**Files:**
- Create: `crates/mk_engine/src/regional/industry/hydro.rs`, `crates/mk_engine/src/regional/industry/grid.rs`
- Modify: `crates/mk_engine/src/regional/energy.rs` (Phase 3 Task 4b: the estate draws from the grid; diesel generator becomes backup)
- Modify: `crates/mk_engine/src/regional/hydrology.rs` (diversion between intake and tailrace)
- Test: `crates/mk_engine/tests/hydro_grid.rs`

**Interfaces:**
- Produces: `HydroPlant { site, turbine: TurbineKind, rated_kw, condition, spare_parts }` with output `P = η(Q/Q_design)·ρ·g·Q·H_net`, `Q = min(design_flow, river_flow − environmental_flow)`, `H_net = H − penstock friction loss`; zero output when the river is too low; intake closes in floods above a design threshold.
- Produces: `Grid { nodes, lines: Vec<Line { from, to, kv, length_km, resistance }>, generators, loads }` solving a simple per-tick balance: generation dispatch (hydro first, then gas backup, then diesel), line losses `I²R`, and load shedding by priority (refinery, workshops, households, street lighting) when supply falls short.
- Wear: turbines and generators lose condition with running hours and sediment; maintenance labour (Task 6) restores it using spare parts; at zero condition the unit fails until repaired.

- [ ] **Step 1:** Tests: output matches `η·ρ·g·Q·H` for test flows; output falls in drought and stops below environmental flow; diverted water reappears at the tailrace and the water ledger closes; the bypassed reach carries reduced flow; grid balance holds within losses every tick; shortfall sheds the lowest-priority loads first; an unmaintained turbine eventually fails.
- [ ] **Step 2:** Implement; run `cargo test -p mk_engine --test hydro_grid`; expect PASS.
- [ ] **Step 3:** Commit `feat(industry): hydro power and island grid`.

### Task 4: Oil and gas field

**Files:**
- Create: `crates/mk_engine/src/regional/industry/petroleum.rs`
- Modify: `crates/mk_core/src/flux/mod.rs` (`AtmosCH4` reservoir), `crates/mk_engine/src/climate/` via `regional/climate.rs` (methane radiative forcing `0.036(√M − √M₀) − overlap`, Myhre et al. 1998)
- Test: `crates/mk_engine/tests/petroleum_field.rs`

**Interfaces:**
- Requires a Phase-1 petroleum deposit (mudstone/sandstone basin with mature source rock). If the chosen island has none, the oil and gas enterprises are not created and the dashboard says why; the gallery (Task 2) shows this before the owner chooses.
- Produces: `PetroleumReservoir { oil_in_place_m3, gas_in_place_m3, pressure_mpa, recovery_factor, gas_oil_ratio, water_cut }` and `Well { cell, rate_m3_day, decline: ArpsDecline, condition }`; production declines with reservoir pressure; produced water rises with time; wells need workover crews.
- Produced gas not used is flared (CO₂) or, by default, not vented; a small leak fraction from reference data goes to `AtmosCH4`.
- Spills and blowouts at reference frequencies; a spill releases oil to land or sea cells and harms ecology (Phase-3 ecology mortality hook).

- [ ] **Step 1:** Tests: cumulative production never exceeds recoverable volume; production follows the Arps curve within tolerance; carbon moves `CrustCarbon → MaterialCarbon`; flaring books CO₂, leaks book CH₄ and methane raises radiative forcing; a spill kills exposed shellfish beds; deterministic per seed.
- [ ] **Step 2:** Implement; run `cargo test -p mk_engine --test petroleum_field`; expect PASS.
- [ ] **Step 3:** Commit `feat(industry): oil and gas field`.

### Task 5: Refinery, gas plant and fuel distribution

**Files:**
- Create: `crates/mk_engine/src/regional/industry/refinery.rs`, `crates/mk_engine/src/regional/industry/gas_plant.rs`, `crates/mk_engine/src/regional/industry/fuel.rs`
- Modify: `crates/mk_engine/src/regional/energy.rs` (vehicles and generators buy fuel from the depot instead of only finite stores)
- Test: `crates/mk_engine/tests/refinery.rs`

**Interfaces:**
- Produces: `Refinery { units: [Distillation, Reformer], capacity_bbl_day, tanks, condition }` running in campaigns when storage allows: crude in, products out by assay (LPG, petrol after reforming, kerosene, diesel, fuel oil residue), refinery fuel use and CO₂ per barrel from reference data; mass and carbon balance close.
- Produces: `GasPlant` separating methane, LPG and condensate; methane to the grid's gas generator and a town gas line; LPG to bottles.
- Produces: `FuelDepot` with tanks and a pump; vehicles, generators and households buy fuel and LPG there at the enterprise's price.

- [ ] **Step 1:** Tests: product yields match the assay within tolerance; mass and carbon in = out + losses; the refinery stops when product tanks are full and restarts when they drain; petrol is unavailable if the reformer is down (straight-run naphtha is not sold as petrol); a vehicle refuels at the depot and pays.
- [ ] **Step 2:** Implement; run `cargo test -p mk_engine --test refinery`; expect PASS.
- [ ] **Step 3:** Record the imported-equipment deviation. Commit `feat(industry): refinery, gas plant and fuel depot`.

### Task 6: Jobs, trades, shifts and maintenance (upstream first)

**Files:**
- Upstream (`dylmarriner/Maer-Ken`), then sync: `crates/mk_core/src/human/trades.rs` (`TradeQualification`), `crates/mk_engine/src/humans/` (`ActionKind::Work`, job attendance in the autonomous mind)
- Create: `crates/mk_engine/src/regional/industry/jobs.rs`
- Test: `crates/mk_engine/tests/industry_jobs.rs`

**Interfaces:**
- Produces: `TradeQualification::{PlantOperator, Electrician, Mechanic, Driller, WellServiceHand, ProcessOperator, Farmer, FarmHand, DairyHand, Fisher, Butcher, Baker, RetailWorker, StoreManager, Forester, ChainsawOperator, SawmillHand, Builder, Carpenter, Plumber, Nurse, Teacher, Labourer}` with proficiency (0–1) that grows with practice; learning speed scales with the upstream `SkillMatrix` aptitudes.
- Produces: `Job { employer, role, qualification, shift: ShiftPattern, wage_cents_per_hour, workplace }`. Shifts run on a 24-hour human roster, not the 36-hour sun, because human body clocks are ~24.2 h (Phase 0c); night shifts carry the realistic fatigue cost the circadian model produces.
- Work is a real upstream action: a human goes to work when their own decision-making chooses it (pay, obligation, needs, personality), travels there (Phase 3 labour), performs timed tasks (operate, inspect, maintain, repair) that change plant condition and output, and can be late, absent, sick or quit.
- Maintenance: plants request work orders when condition falls; qualified workers complete them using spare parts; unqualified workers take longer and make errors at reference rates.

- [ ] **Step 1:** Tests: a qualified operator on shift keeps the hydro plant running; with nobody on shift the refinery stops; an untreated work order lets condition fall to failure; proficiency rises with practice; a worker whose needs are unmet sometimes misses shifts; night-shift workers accumulate circadian sleep debt.
- [ ] **Step 2:** Land the human-runtime changes upstream with tests; sync; implement the regional job system; run `cargo test -p mk_engine --test industry_jobs` and `cargo test -p mk_engine --lib -- humans::`; expect PASS.
- [ ] **Step 3:** Commit `feat(industry): jobs, trades, shifts and maintenance`.

### Task 7: Money, enterprises, ownership and profit

**Files:**
- Create: `crates/mk_engine/src/regional/economy/{money.rs,enterprise.rs,market.rs}`
- Upstream then sync: `ActionKind::Buy` and money awareness in human decisions
- Test: `crates/mk_engine/tests/island_economy.rs`

**Interfaces:**
- Produces: double-entry `Ledger` of `Account { owner: Human | Enterprise, balance_cents }` and `Transaction { tick, from, to, amount_cents, memo }`; total money is constant except explicit issuance at scenario start (documented initial balances).
- Produces: `Enterprise { name, shareholders: Vec<(agent_id, share)>, assets, accounts, price_list, payroll }` for **Gem Hydro** (electricity), **Gem Petroleum** (oil, gas, refinery, fuel depot), **Gem Foods** (farm, fishery, abattoir, mill, bakery, dairy), **Gem Supermarket** (retail), **Gem Forestry** (forest and sawmill), **Gem Construction** (builders) and **Gem Housing** (owns the town houses and collects rent), each owned 50/50 by Gem-D and Gem-K by default (owner to confirm per enterprise). Enterprises trade with each other at their price lists (the farm sells to the supermarket, the sawmill to the builders, everyone buys electricity and fuel).
- **Pay:** each job records a timesheet from actual attendance (Task 6); every 14 roster days payroll pays each worker hours worked × wage (overtime at 1.5×, paid sick leave and annual leave per a default employment policy the owners can change) into the worker's own account and writes a payslip event to their folder. If an enterprise cannot cover payroll it borrows from Gem-D's and Gem-K's accounts (owner approval command) or pays late, and workers' decisions react to late or missing pay.
- Each local month: meters bill households and businesses for kWh; the depot bills fuel at the pump; Gem Housing charges rent; costs (spare parts, materials, purchases from other enterprises) are booked; profit = revenue − costs; a dividend policy (default: pay out 50% of profit each local quarter, retain the rest) credits Gem-D's and Gem-K's accounts by share.
- Humans buy food, fuel, LPG and electricity with their money through their own decisions; a household that cannot pay is disconnected after a grace period (owners may change the policy).

- [ ] **Step 1:** Tests: every transaction balances and total money is conserved; a month of operation produces correct bills from meter readings; dividends equal profit × payout × share; Gem-D and Gem-K's balances rise when the enterprises are profitable and fall when they lose money; a worker is paid exactly hours × wage (with overtime and leave) every 14 roster days and a payslip appears in their folder; the lowest full-time wage covers the reference household basket at default prices; a worker paid wages buys food at the supermarket; disconnection happens only after the grace period.
- [ ] **Step 2:** Implement; run `cargo test -p mk_engine --test island_economy`; expect PASS.
- [ ] **Step 3:** Commit `feat(economy): money, enterprises and owner profits`.

### Task 7b: Food and timber supply chains

A supermarket sells food; it does not make it. With no imports, every calorie the town eats and every board in its houses must be grown, caught, cut and processed on the island.

**Files:**
- Create: `crates/mk_engine/src/regional/industry/{farm.rs,fishery.rs,food_processing.rs,supermarket.rs,forestry.rs,sawmill.rs,construction.rs}`
- Modify: `crates/mk_engine/src/materials/catalogue.rs` (Phase 3 Task 2: grain, flour, bread, vegetables, fruit, milk, cheese, eggs, mutton, beef, chicken, fish, logs, sawn timber, weatherboards, firewood; crop seed and livestock as stock)
- Test: `crates/mk_engine/tests/food_chain.rs`, `crates/mk_engine/tests/timber_chain.rs`

**Interfaces:**
- **Farm:** fields and pasture on buildable cells near the town. Crop growth from the Phase-2 climate (temperature, rainfall, sunlight over the 36-hour day and 323-day year) using crop growth-degree-day and water-stress models from reference data; sowing and harvest are timed labour; yields per hectare within reference ranges; soil nutrients deplete and need rotation or manure. Livestock are real animals with feed (pasture biomass), growth, breeding and slaughter weights from reference data; dairy cows milked twice a roster day. Farm carbon, nitrogen and water go through the Phase-3 ledgers.
- **Fishery:** an inshore boat with a crew; catch per trip from the Phase-3 marine species populations and fishing effort (catch-per-unit-effort), burning diesel; overfishing depletes stocks.
- **Processing:** abattoir/butchery (carcass → cuts by reference yields), mill and bakery (grain → flour → bread), dairy (milk → milk, butter, cheese); each a workplace with jobs and electricity load.
- **Supermarket:** stock of every food item with quantity, purchase cost, shelf price and shelf life; refrigerated and frozen stock spoils if the power fails; reorders from Gem Foods when stock is low; households buy through their own decisions (Task 7); unsold food past its date becomes waste (detritus carbon).
- **Forestry:** a harvest plan whose annual cut does not exceed the forest's regrowth (sustainable yield from Phase-3 biomass and NPP); felling is timed labour with chainsaws (fuel) and a skidder; logs are trucked to the sawmill; cut blocks are replanted and regrow.
- **Sawmill:** logs → framing, weatherboards and flooring at reference recovery rates; offcuts and sawdust become firewood and boiler fuel; carbon follows the timber (Phase-3 `MaterialCarbon`, stored in buildings).
- **Construction:** Gem Construction builds houses and other buildings as projects with a bill of materials (sawn timber, iron roofing from stock or the Phase-3 smelter, glass, lime mortar/concrete from Phase-3 quicklime, fixings) and labour hours by trade from reference data; a house is habitable only when complete.

- [ ] **Step 1:** Food tests: the founding town's food production meets its energy and protein needs over a simulated year within ±10% (or the shortfall is reported, not hidden); a drought year lowers crop yields; overfishing reduces catch; a power cut spoils refrigerated stock; a household with money and need buys food and eats it (Phase-3 metabolism); food waste returns to detritus.
- [ ] **Step 2:** Timber tests: annual harvest never exceeds regrowth under the default plan; a felled block regrows after replanting; sawmill output matches log input × recovery; a house built from the bill of materials consumes exactly those materials and labour hours and becomes habitable on completion; carbon is conserved from tree to house.
- [ ] **Step 3:** Run `cargo test -p mk_engine --test food_chain --test timber_chain`; expect FAIL; implement; re-run; expect PASS.
- [ ] **Step 4:** Commit `feat(industry): food and timber supply chains`.

### Task 8: Town, housing and the founding workforce

**Files:**
- Create: `crates/mk_engine/src/regional/town.rs`
- Modify: `crates/mk_engine/src/regional/estate_layout.rs` (generalise to any high-detail site: estate, town, refinery)
- Create: `fixtures/island/workforce.json` (generated deterministically from the seed and sizing; owner may edit)
- Test: `crates/mk_engine/tests/island_town.rs`

**Interfaces:**
- High-detail patches: the estate (Phase 3), the town (4 km × 4 km, on buildable land near the stage-2 hydro site and the refinery road), and the refinery/gas-plant site (2 km × 2 km); oil wells are point sites on medium cells; roads, flowlines, pipelines and power lines are network links with real lengths.
- Town layout: one house per household sized to household size (reference floor areas), owned by Gem Housing and rented to the household; supermarket, bakery, butchery, workshop, clinic, school room, fuel depot; the farm (fields, pasture, dairy shed, barns) and the sawmill and timber yard at the town edge; the fishing wharf on the coast; every building has an electricity meter and LPG supply.
- Workforce: households generated deterministically from the scenario seed with realistic demographics (age/sex structure, couples and children from reference tables), each adult assigned a role and an initial qualification; every person is created through `build_authored_human` and gets their own folder; family relationships are set through upstream relationship APIs.
- Town growth: new households form from births and young adults leaving home; when demand exceeds supply, the dashboard proposes stage-2 hydro, more houses, more farmland or a bigger sawmill as construction projects that Gem Construction builds with Task 7b's timber chain, Phase-3 labour and materials, and imported equipment from stock.

- [ ] **Step 1:** Tests: the founding town has the default number of households and roles; every person has a folder, a house and (if adult) a job or a reason not to; houses do not overlap and are routable; the town is connected to the grid and the depot; the workforce file reproduces the same people from the same seed.
- [ ] **Step 2:** Implement; run `cargo test -p mk_engine --test island_town`; expect PASS.
- [ ] **Step 3:** Commit `feat(island): founding town and workforce`.

### Task 9: Dashboard, persistence and acceptance

**Files:**
- Modify: `apps/island/src/serve/*`, `apps/island/static/*` (Phase 4)
- Modify: `crates/mk_engine/src/regional/world.rs` (state, scheduler cadences), `io/island_snapshot.rs`
- Test: `crates/mk_engine/tests/island_industry_acceptance.rs`

**Interfaces:**
- Dashboard pages: **Power** (river flow, hydro output, grid load, shed loads), **Petroleum** (well rates, reservoir pressure, refinery campaigns, tank levels), **Town** (households, people, jobs, vacancies, houses under construction), **Food** (production vs need, supermarket stock and prices), **Timber** (harvest vs regrowth, sawmill output), **Payroll** (each worker's hours, pay and payslips), **Finance** (each enterprise's revenue, costs, profit, dividends; Gem-D's and Gem-K's balances), with the Human Creator able to create a person directly into a town household and job.
- Owner controls (commands, replayable): set prices and tariffs, set wages, set dividend policy, approve construction projects, hire and fire.
- All new state is in `IslandWorldState`, snapshots and the state hash; cadences: electricity and jobs at `human_seconds`, petroleum and refinery at `weather_ocean_seconds`, billing and payroll monthly.

- [ ] **Step 1:** Acceptance (slow tier): bootstrap the default scenario, run one local year; assert the house and town are powered by hydro except during recorded low-flow periods, fuel is produced and sold, every ledger closes (water, carbon, oxygen, methane, money), Gem-D and Gem-K receive dividends equal to their shares of profit, every worker is paid for the hours they worked every 14 roster days, the town is fed from its own farms and fishery, houses are built from its own timber, and the state hash is deterministic; snapshot mid-year and resume to the same hash.
- [ ] **Step 2:** Run the acceptance test; add `island_preview world` overlays for the river, plants, grid and town.
- [ ] **Step 3:** Benchmark with the full town population and record it in the Phase-5 baseline; if the default scale misses the performance targets, record the measured cost and offer the owner a smaller founding town.
- [ ] **Step 4:** Commit `feat(island): hydro, petroleum, town and economy on the dashboard`.
