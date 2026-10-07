# Island Life, Property & Humans Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Populate the regional island with retained Maer-Ken habitats, vegetation, resources, canonical property/equipment laid out in real metric space, individually simulated trees around the estate, and the full Gem-D/Gem-K human runtime moving on the island.

**Architecture:** Derive regional biome/ecology/resource state from the Phase-2 physical world, then instantiate property and humans through the already imported canonical engine types. Add regional adapters for placement, observation and interaction. Human cognition, biology and behaviour code is not forked; the one engine change is geometry — the human and organism runtimes currently assume the planetary grid (spherical cell sizes, east/west wrap, lat/lon from grid rows), so Task 6 threads a topology abstraction through them that reproduces upstream exactly on the planet and uses flat, non-wrapping geometry on the island.

Two representation levels exist:

- **Regional (medium cells, 2 km):** habitat, NPP, a prognostic standing-biomass field and resources as per-cell fields; species populations as upstream. Upstream living biomass carbon is `Σ species population × body mass` (`conservation.rs:255-285`) with no spatial distribution, so `biomass_kgc_m2` is the *spatial distribution* of producer carbon: it grows with NPP and loses to turnover, harvest and disturbance, and after each ecology step it is renormalised so `Σ cell biomass × area` equals producer species carbon. Species carbon stays the authoritative total; the field says where it is. Upstream `VegetationSystem::seed_from_biomes` materializes about one plant per 11 cells; that remains a sparse view, not the forest.
- **High-detail estate patch:** 4 km × 4 km at 5 m cells (800 × 800), aligned exactly to a 2 × 2 block of 2 km medium cells (the estate block), so patch quantities disaggregate whole cells. Buildings, rooms, doors and items have metric positions; trees within 1 km of the estate centre are individual stems, the rest of the patch is stand-level cover. Upstream property stores the whole estate as one grid cell with room *labels* only; the layout adds geometry without copying the inventory.

**Tech Stack:** Rust 2021, existing `mk_core::flux` ledger, `mk_engine::biosphere`, `conservation`, `organisms`, `resource_economy`, `humans`, `perception`, `mk_island` scenario/profile types.

**Spec:** `docs/superpowers/specs/2026-10-01-maer-ken-island-regional-world-design.md`

**Depends on:** Phase 2 (`RegionalPhysicalState` including `insolation`).

## Global Constraints

- Gem-D/Gem-K constructors and human cognition/biology modules remain sourced from the pinned Maer-Ken runtime; geometry changes (Task 6) must leave every existing upstream test result unchanged.
- The founders' estate retains every building upstream defines — House, Shed, Workshop, Armoury, ComputerRoom, Garage (`PropertyBuildingKind`) — plus rooms, named vehicles, attachments, tools, furniture, computers and network accounts.
- Only the founders' estate (the `StarterProperty` whose `owner_agent_ids` contains `"Gem-D"`) is laid out and placed. The unowned homestead template that `PropertySystem::new` always includes stays unplaced. Placements are keyed by `(property_id, item_id)` because item ids restart at 1 per property.
- Only regional island/ocean cells may seed living populations or resources.
- The computer room is part of the House. Upstream data models it as its own `PropertyBuildingKind::ComputerRoom` building; that data is kept unchanged (so upstream tests and access gates still work), and the layout places it as a room inside the House footprint, reached through the House, with no outside door.
- Computer access requires the upstream gate (a ComputerRoom building and a matching `NetworkAccount`) *and*, on the island, that the human is in the computer room space (Task 8).
- Every founders'-estate `PropertyItem` appears exactly once in the layout; the layout derives from `PropertySystem` and never defines items of its own.
- At seed time, patch tree + stand biomass equals the four estate cells' biomass within ±2%, with the tree cap active; afterwards patch biomass and the cell field change together.
- **Materials are physical.** Every gathered, crafted, built, burned, eaten or drunk material moves real carbon, oxygen and water through the `mk_core::flux` ledger (Task 3). Nothing gathered appears from nowhere: biotic resource nodes are views of cell biomass, not free-regenerating counters. Upstream couples none of this; the coupling is an island divergence recorded in `UPSTREAM.md` and suitable for upstreaming.

## Review Focus

- All-ocean or no-valid-estate scenarios must fail clearly instead of inventing a property location.
- Founder placement must be on dry, non-dominantly-volcanic, buildable land.
- Aquatic cells must not receive terrestrial resource nodes/vegetation.
- Re-seeding vegetation/ecology must be idempotent for already initialized state.
- Building footprints must not overlap each other, the patch edge, water or slopes above the buildable limit; trees must not stand inside footprints or the cleared yard.
- Every room/zone must be reachable from outdoors through the door graph.
- Planet-topology results must be bit-identical before and after Task 6.
- After any step, the conservation stock audit closes for carbon, oxygen and water including the new material and human reservoirs.

---

### Task 1: Regional biome, habitat, NPP and vegetation initialization

**Files:**
- Create: `crates/mk_engine/src/regional/ecology.rs`
- Modify: `crates/mk_engine/src/biosphere/mod.rs`, `crates/mk_engine/src/biosphere/habitat.rs` (extract geometry inputs; see Step 3)
- Modify: `crates/mk_engine/src/regional/mod.rs`
- Test: `crates/mk_engine/tests/regional_ecology.rs`

**Interfaces:**
- Produces: `RegionalEcologyState { biome_grid: Grid2<BiomeType>, biosphere: BiosphereState, organisms, vegetation: VegetationSystem, biomass_kgc_m2: Grid2<f64> }` on Medium.
- Produces: `bootstrap_regional_ecology(canon: &Arc<CanonLocked>, physical: &RegionalPhysicalState, domain: &IslandDomain, seed: [u8; 32]) -> Result<RegionalEcologyState, RegionalEcologyError>` and `step_regional_ecology(&mut self, …, dt_seconds)`.
- `biomass_kgc_m2` is initialised at equilibrium `npp_kgc_m2_yr × residence_years(biome)` (per-biome residence-time table in code, citing a source for each value), then stepped as `dB/dt = NPP − B / residence − harvest − disturbance`, and renormalised to producer species carbon after each ecology step.

- [ ] **Step 1:** Write tests asserting ocean/land biome consistency, at least one tree/shrub/grass on eligible land, no terrestrial plant in ocean cells, NPP higher in warm wet lowland than cold alpine cells, `biomass_kgc_m2` zero over ocean, `Σ biomass × area` equal to producer species carbon after a step, and deterministic bootstrap.
- [ ] **Step 2:** Run `cargo test -p mk_engine --test regional_ecology`; expect FAIL.
- [ ] **Step 3:** Upstream `update_primary_production` derives latitude as `(row+0.5)/nlat·π−π/2` and uses planet radius and `dlon` (`biosphere/mod.rs:628-660`); `habitat_cells` takes `&WorldState` (`habitat.rs:55`). Extract the latitude/area inputs as parameters (global path filled exactly as today; regional path from `domain.latitude_rad_for_row` and flat area) and add a habitat function over explicit grids. Reuse biome classification and biosphere/vegetation systems against regional climatology/hydrology/elevation resampled to Medium.
- [ ] **Step 4:** Run `cargo test -p mk_engine --test regional_ecology` and `cargo test -p mk_engine --lib -- biosphere:: organisms::vegetation`; expect PASS.
- [ ] **Step 5:** Commit `feat(engine): seed island ecology`.

Implementation notes (part 1a, done): `regional/ecology.rs` runs upstream's pure `classify_biome` and `miami_npp_kgc_m2_yr` on the medium grid instead of refactoring `update_primary_production`/`habitat_cells` (no upstream code changes were needed for the field layer). Mean NPP on the owner's island is 0.48 kgC/m²/yr in the lowland and 0.15 on the heights. Rivers (discharge >= 1 m³/s) are `River`; lakes are `CoastalWaters`. **Part 1b is open:** seeding species populations at island scale (upstream's are planetary totals, so `BiosphereSystem` needs island-scaled initial populations and a producer-carbon total for `renormalise_to_total`), plus the `biosphere::`/`organisms::vegetation` regression run of Step 4. Steps 1-2 are done for the field layer; Step 3 is replaced by the pure-function reuse; Steps 4-5 close with part 1b.


### Task 2: Complete resource nodes and material catalogue

Upstream has 9 node kinds (`resource_economy.rs:13-23`) placed by biome, maps only 12 of the 25 biome `ResourceKind`s to nodes (`node_kind_for_resource`, `:639-654` — `Gem`, `Salt`, `Sulfur`, `Herbs`, `Fish`, `Shellfish`, `Meat`, `Hide`, `Bone`, `Wax`, `Feather` and `Water` are dropped), and lumps ores into `MetalOre`/`Minerals`. This task makes every resource and material in the world gatherable from a real source: geological deposits from Phase 1 Task 5, river and beach deposits from Phase-2 hydrology, plants from cell biomass, animals and fish from species populations, and water from the hydrology stores.

**Files:**
- Create: `crates/mk_engine/src/materials/mod.rs`, `crates/mk_engine/src/materials/catalogue.rs`
- Modify: `crates/mk_engine/src/agents/mod.rs` (`ItemKind` gains the catalogue's items)
- Modify: `crates/mk_engine/src/resource_economy.rs` (`ResourceNodeKind`, `ToolKind`, `RecipeId`, exhaustive `node_kind_for_resource`)
- Create: `crates/mk_engine/src/regional/resources.rs`
- Test: `crates/mk_engine/tests/regional_resources.rs`, unit tests in `materials/catalogue.rs`

**Interfaces:**
- Produces: `MaterialCategory::{PreciousMetalOre, BaseMetalOre, Gem, Crystal, Stone, Sediment, IndustrialMineral, FossilFuel, Salt, Plant, Animal, Marine, Water, Processed}` and the catalogue — one `ItemKind` per material with name, category, density, hardness (Mohs where meaningful) and source. Required items:

| Category | Items |
|---|---|
| Precious metals | gold ore (quartz-gold), placer gold (nuggets/flakes), silver ore, platinum-group concentrate |
| Base and other metals | iron ore (hematite/magnetite), ironsand (titanomagnetite), copper ore, tin ore (cassiterite), lead ore (galena), zinc ore (sphalerite), nickel ore, chromite, tungsten ore (wolframite), molybdenite, cinnabar (mercury), uraninite, lithium ore, bauxite (only if Phase-3 climate produces laterite) |
| Gems | diamond, ruby, sapphire, emerald, topaz, tourmaline, garnet, opal, nephrite jade |
| Crystals and silica | quartz crystal, amethyst, agate, chalcedony, obsidian |
| Stone | granite, basalt, limestone, marble, sandstone, slate, flint/chert, pumice, stone (generic rubble, upstream `Stone`) |
| Sediments | clay (upstream `Clay`), kaolin, sand (upstream `Sand`), silica sand, gravel, peat |
| Industrial minerals | sulfur, gypsum, phosphate rock, feldspar, mica, graphite, zeolite, serpentine, kyanite, pyrite |
| Fossil fuels | coal (upstream `Coal`), crude oil, natural gas |
| Salts | rock salt, sea salt |
| Plants | wood (upstream `Wood`), resin, fibre (upstream `Fiber`), herbs (upstream `Herbal`), fruit, nuts, fungi, plant food (upstream `Food`), honey, beeswax, seaweed |
| Animals | meat, hide, bone, feather |
| Marine and freshwater | fish, shellfish |
| Water | water (upstream `Water`) |
| Processed | planks, masonry, rope, fuel, charcoal, quicklime, glass, gold ingot, silver ingot, copper ingot, tin ingot, lead ingot, zinc ingot, iron ingot, steel, bronze, brass, cut gem (per gem kind), polished stone, tools, weapons, structures (existing upstream items keep their names) |

- Produces: `ResourceNodeKind` covering every source above: `Deposit(DepositKind)` for every Phase-1 deposit; `Placer { gold | diamond | gem(GemKind) }` and `Ironsand` from rivers and beaches; `Tree`, `FruitPlant`, `NutPlant`, `FungiPatch`, `HerbPatch`, `FibrePatch`, `FoodPatch`, `BeeColony`, `SeaweedBed` from cell biomass by biome; `GameAnimals`, `Fishery`, `ShellfishBed` from fauna and marine species in the cell; `SaltPan` (coastal, from sea water) and `WaterSource` (from hydrology stores).
- `node_kind_for_resource` becomes an exhaustive `match` with no `_` arm, so a new upstream `ResourceKind` cannot be silently dropped again.
- Secondary deposits, computed from Phase-2 rivers and coast: placers accumulate downstream of eroding primary gold, diamond and gem deposits, with grade decaying by distance along the river; ironsand concentrates on beaches downwind and downcurrent of basaltic/andesitic coasts.
- Yields: a deposit node yields ore or rough gem at its deposit's grade, removing tonnage until exhausted; it never regenerates. Placers deplete too, but slowly recharge from upstream erosion. Biotic and animal nodes follow Task 3 (biomass and species populations, no free regeneration). Hunting and fishing reduce the hunted species' population.
- Tools: `ToolKind` gains `GoldPan`, `Sluice`, `Chisel`, `LapidaryWheel`, `HuntingWeapon`, `FishingGear`, `Kiln`, `Crucible`, `Anvil`, `Drill`. Each node kind lists the tools that can work it (gems and hard-rock ore need a pickaxe or better; placer gold needs a pan or sluice; crude oil and natural gas need a drill rig that no current recipe makes, so they exist but cannot yet be extracted). Founders' estate tools (shed tool categories, workshop anvil and forge tools, armoury weapons) grant the matching `ToolKind` capabilities while the human is at the estate.
- Recipes (each through Task 3's material ledger, mass-balanced): smelt gold, silver, copper, tin, lead, zinc and iron ores to ingots; bronze (copper + tin) and brass (copper + zinc); steel (iron + carbon from charcoal or coal); charcoal (wood → charcoal + CO₂); quicklime (limestone → lime + CO₂ from crustal carbon, as real calcination does); glass (silica sand + lime + fuel); cut gem (rough → cut, with documented mass loss); polished stone.
- Produces: `bootstrap_regional_resources(geophysics: &RegionalGeophysics, ecology: &RegionalEcologyState, physical: &RegionalPhysicalState, domain: &IslandDomain) -> ResourceEconomyState`. Upstream `from_terrain` stays for the planetary path.
- Buildability: upstream `is_buildable` allows at most `MAX_CLIMB_HEIGHT_M = 2.0` between neighbouring cells (`physics.rs:34, 76`), which almost no 2 km cell passes. Inside the estate patch, buildability uses patch terrain at 5 m; outside it, `regional_is_buildable` applies the same 2 m rise over the 5 m patch spacing as a slope limit (0.4) to medium-cell slope. Record as a divergence.

- [ ] **Step 1:** Write catalogue tests: every required item exists with a category and density; every `ResourceKind` maps to a node kind (exhaustive match compiles); every `DepositKind` maps to at least one node kind and item.
- [ ] **Step 2:** Write regional tests: deposit nodes only where Phase 1 put deposits; gold and diamond nodes exist on the owner's chosen island where its gallery entry said they would; placer gold appears only downstream of a gold source; ironsand only on suitable coasts; no terrestrial nodes in aquatic cells; deposit nodes never regenerate and run out at their tonnage; panning a placer without a pan fails and with one succeeds; smelting gold ore yields gold in proportion to grade; a gentle lowland medium cell is buildable while a steep alpine one is not; deterministic IDs/order.
- [ ] **Step 3:** Run `cargo test -p mk_engine --test regional_resources` and `cargo test -p mk_engine --lib materials::`; expect FAIL.
- [ ] **Step 4:** Implement the catalogue, node kinds, tools, recipes and regional placement. Keep upstream variants and names; add, don't rename.
- [ ] **Step 5:** Run both tests and `cargo test -p mk_engine --lib -- resource_economy:: agents::`; expect PASS, with existing upstream tests unchanged.
- [ ] **Step 6:** Record the extended enums, tool mapping and every abundance/grade source in `UPSTREAM.md`. Commit `feat(engine): nodes for every resource and material on the island`.

### Task 3: Physical materials — carbon, oxygen and water through the economy

Upstream gathering, crafting, building, smelting, eating and drinking move no carbon, oxygen or water: a Tree node yields `Wood` without touching biomass, `regenerate()` refills nodes from nothing, `Fuel` (made from coal, `resource_economy.rs:730-734`) is consumed by smelting (`:712-713`) without CO₂, and humans eat `Food` and drink `Water` (`humans/lifecycle.rs:556-565`) with no metabolism in the ledger. This task makes every material transfer physical.

**Files:**
- Modify: `crates/mk_core/src/flux/mod.rs` (new reservoirs `MaterialCarbon`, `MaterialWater`, `HumanCarbon`; covered by Phase 0's exhaustive `reservoir_all` test)
- Modify: `crates/mk_engine/src/conservation.rs` (stock audit includes the new reservoirs)
- Create: `crates/mk_engine/src/regional/materials.rs`
- Modify: `crates/mk_engine/src/resource_economy.rs` (hooks: gather, craft, build, smelt, demolish/decay; biotic node stock as a view of cell biomass)
- Modify: `crates/mk_engine/src/humans/lifecycle.rs` (eat/drink hook in `consume_carried_rations`)
- Test: `crates/mk_engine/tests/island_material_flows.rs`

**Interfaces:**
- Produces: `MaterialComposition` table, one row per `ItemKind` in Task 2's catalogue: dry mass (kg per item unit), carbon fraction, water content, and origin (`Biotic(BiomeClass)`, `Animal`, `Fossil`, `Mineral`, `Carbonate`). Initial values with a cited source each (e.g. dry wood ≈ 50% carbon by mass, limestone ≈ 12% carbon as CaCO₃); a test asserts every `ItemKind` has a row. Mineral mass itself is conserved through deposit tonnage (Task 2), not the ledger.
- Produces: `MaterialLedger` hooks called by the regional economy with the cell and amount:
  - **Gather biotic** (Tree/FoodPatch/Fiber/Herbal/Resin nodes): `BiomassCarbon → MaterialCarbon`, debiting the cell's `biomass_kgc_m2` and producer species carbon by the same amount. Species are debited in proportion to their carbon share, with a per-species fractional accumulator so integer populations stay exact. A biotic node's available yield is `min(node cap, cell biomass available for harvest)`; biotic `regenerate()` is replaced by biomass regrowth from NPP. Abiotic nodes (stone, clay, sand, ore) keep upstream behaviour.
  - **Gather coal, oil, gas or limestone:** `CrustCarbon → MaterialCarbon`.
  - **Hunt or fish:** consumer species carbon (`BiomassCarbon`, consumer class) `→ MaterialCarbon`, reducing the hunted species' population with a fractional accumulator.
  - **Calcine limestone (quicklime) / make charcoal:** the carbon released goes `MaterialCarbon → AtmosCO2`.
  - **Gather water:** the cell's hydrology store (rivers/lakes/groundwater) `→ MaterialWater`; refused if the store is empty.
  - **Craft:** carbon follows mass; any recipe mass loss (e.g. offcuts) goes to `DetritusCarbon` at the cell.
  - **Build:** carbon stays in `MaterialCarbon`, attributed to the structure; demolition or decay over a documented lifetime moves it to `DetritusCarbon`.
  - **Burn / smelt:** fuel carbon `MaterialCarbon → AtmosCO2` with stoichiometric O₂ `AtmosO2 →` consumed (C + O₂ → CO₂, 2.664 kg O₂ per kg C).
  - **Eat:** food carbon `MaterialCarbon → HumanCarbon`; respiration returns it `HumanCarbon → AtmosCO2` and consumes O₂ at a documented respiratory quotient, at the rate set by the human's metabolic expenditure. On death, remaining `HumanCarbon` for that body goes to `DetritusCarbon` at the cell.
  - **Drink:** `MaterialWater → ` the human, returned to `Atmosphere`/`SoilWater` at the cell by respiration, sweat and excretion so the human water store stays bounded.
- Wildfire and other disturbances keep upstream's existing biomass carbon handling; harvested `MaterialCarbon` in structures at a burned cell burns too.

- [ ] **Step 1:** Write tests: felling/gathering moves exactly the item's carbon from biomass (cell field and species) to `MaterialCarbon`; a cell harvested to zero yields nothing until NPP regrows it; smelting with coal fuel raises atmospheric CO₂ and lowers O₂ by the stoichiometric amounts; calcining limestone and making charcoal release their carbon as CO₂; hunting a game animal reduces its species population and moves its carbon; a human eating and living one day moves food carbon through `HumanCarbon` to atmospheric CO₂; drinking from a dry cell is refused; a built shelter holds its carbon until demolished; after each step the carbon, oxygen and water stock audits close.
- [ ] **Step 2:** Run `cargo test -p mk_engine --test island_material_flows`; expect FAIL.
- [ ] **Step 3:** Add the reservoirs and audit terms; implement the composition table and hooks; route regional economy actions and human eating/drinking through them.
- [ ] **Step 4:** Run the test plus `cargo test -p mk_core --lib flux::`, `cargo test -p mk_engine --lib -- conservation:: resource_economy:: humans::` and `cargo test -p mk_engine --test conservation_audit`; expect PASS. Planetary `WorldState` keeps upstream behaviour unless the hooks are explicitly enabled, so existing tests are unaffected.
- [ ] **Step 5:** Record the coupling and every composition value's source in `UPSTREAM.md`. Commit `feat(engine): make gathered materials move real carbon, oxygen and water`.

### Task 3b: Real time and real energy for every action

Upstream actions have no duration or energy cost: one decision gathers, crafts or builds instantly. Real work takes hours and burns calories.

**Files:**
- Create: `crates/mk_engine/src/regional/labour.rs`
- Modify: `crates/mk_engine/src/resource_economy.rs` (actions become timed tasks)
- Modify (upstream first, per Phase 0c): `crates/mk_engine/src/humans/lifecycle.rs` / `needs.rs` (energy expenditure hook)
- Test: `crates/mk_engine/tests/island_labour.rs`

**Interfaces:**
- Produces: `TaskSpec { action, tool, duration_s, met, output_per_hour }` for every action × tool from the Phase-0c labour and human packs (Compendium of Physical Activities MET values; documented productivity — e.g. hand-axe vs chainsaw felling, hand mining, panning throughput, smelting batches, construction hours per m²).
- Actions become `ActiveTask { spec, progress_s }` carried across human ticks; output is delivered as work progresses; walking to the node takes real time at the reference walking speed, slowed by slope and load.
- Energy: each human's expenditure per tick is `BMR (Mifflin–St Jeor from their body) × MET of the current activity`; it draws down glucose/energy reserves in upstream needs, so heavy labour makes humans hungry and tired at realistic rates; carried load is limited by body mass and fitness.

- [x] **Step 1:** Write tests: felling a tree takes the reference duration for the tool used; a day of heavy labour expends energy within the reference range for an adult of the founder's body; panning a placer yields grams per day within the reference range for its grade; walking 10 km on flat ground takes ~2 h; carrying above the load limit is refused.
- [x] **Step 2:** Run `cargo test -p mk_engine --test island_labour`; expect FAIL; implement; re-run; expect PASS.
- [x] **Step 3:** Record in `UPSTREAM.md` and offer upstream. Commit `feat(engine): timed, energy-costed human work`.

Implementation notes: the labour model is a new module driven entirely by the reference packs (every duration, rate, recovery fraction, MET and BMR term is read, not hard-coded; tests check results against the packs' own ranges). The upstream hook is one field, `EffortFocus::activity`, defaulting to the baseline, so planetary behaviour is bit-identical. **Open, deliberately:** the plan's `resource_economy.rs` change (actions as timed tasks) and lifecycle mapping each `ActionKind` to a MET would change planetary human behaviour and the realism calibration, so they are left for Task 9's integration and an upstream decision (Phase 0c rule: human biology changes go upstream first). A day of heavy labour comes to 2.1 x BMR for the founder's body, inside the pack's 2.0-2.4; an invented 4 h of hard felling plus 4 h walking gave 2.42 and was rejected as an unrealistic duty cycle, not fitted to.


### Task 4: Island scenario and canonical estate placement

**Files:**
- Create: `crates/mk_island/src/scenario.rs`
- Modify: `crates/mk_island/src/lib.rs` (export `scenario`)
- Create: `fixtures/island/default_scenario.json`
- Create: `crates/mk_engine/src/regional/property.rs`
- Test: `crates/mk_island/tests/scenario.rs`, `crates/mk_engine/tests/island_property_scenario.rs`

**Interfaces:**
- Produces: `IslandCadenceProfile { human_seconds: u64, weather_ocean_seconds: u64, hydrology_ecology_resource_seconds: u64, geophysics_seconds: u64, human_store_seconds: u64 }` with defaults `60`, `3_600`, `21_600`, `86_400`, `3_600`; `validate()` rejects zero values and any cadence that is not a multiple of `human_seconds`.
- Produces: `EstatePatchConfig { extent_m: f64, cell_size_m: f64, individual_tree_radius_m: f64, tree_cap: usize }` with defaults `4_000.0`, `5.0`, `1_000.0`, `200_000`.
- Produces: `IslandScenario { version: u32, seed: [u8; 32], canon_path: PathBuf, profile: IslandProfile, cadences: IslandCadenceProfile, estate_patch: EstatePatchConfig, estate_energy: EstateEnergyConfig, founders_enabled: bool, estate_enabled: bool }` (`canon_path` defaults to the Phase-0c `fixtures/island/canon.json`; `EstateEnergyConfig` is defined here with the Task 4b defaults) (serde) and `IslandScenario::load(path: &Path) -> Result<Self, IslandScenarioError>`; the default fixture embeds `fixtures/island/default_profile.json`'s values and seed.
- Produces: `choose_regional_estate_location(canon: &CanonLocked, physical: &RegionalPhysicalState, ecology: &RegionalEcologyState, domain: &IslandDomain) -> Option<(usize, usize)>` (Medium cell; scoring follows upstream's year-round-climate placement, which uses `canon.obliquity_deg`, and additionally requires a perennial river within 1 km of the estate block whose reach can run a stage-1 micro-hydro plant with environmental flow left — Phase 4b Task 2 sizes and builds it), `estate_patch_spec(domain, cell, &EstatePatchConfig) -> Result<LocalPatchSpec, …>` (the patch is the 2 × 2 medium-cell block whose south-west cell is `cell`; `extent_m` must equal 2 × `cell_size_m(Medium)` and the block must lie on land inside the domain), and `bootstrap_regional_property(location) -> PropertySystem` = upstream `PropertySystem::new(Some(location))`.

- [x] **Step 1:** Write scenario tests: defaults, JSON round-trip, unknown version rejected, invalid cadences rejected. Write property tests: deterministic estate placement on dry buildable land; patch exactly covers the 2 × 2 estate block; founders' estate has all six `PropertyBuildingKind`s, Gem-D's and Gem-K's bedrooms, named vehicles, all shed tool categories, computers and two administrator network accounts; an all-ocean physical state returns `None`.
- [x] **Step 2:** Run `cargo test -p mk_island --test scenario` and `cargo test -p mk_engine --test island_property_scenario`; expect FAIL.
- [x] **Step 3:** Implement scoring from real physical/climatology state; instantiate upstream `PropertySystem::new(Some(location))` without copying inventory definitions.
- [x] **Step 4:** Run both tests and `cargo test -p mk_engine --lib organisms::property`; expect PASS.
- [x] **Step 5:** Commit `feat(island): place canonical founders estate`.

Implementation notes: the scenario seed is written as 64 hex characters (as the profile's). `choose_regional_estate_location` returns the best-scoring block; `place_regional_estate` additionally lays the estate out and falls back down the ranking (up to 64 sites) if the buildings do not fit the patch, so placement and layout always agree. On the owner's island the best site (medium cell 321, 884) is the one placed. Wetland blocks are excluded along with lake, river, volcanic, alpine and ice ground. The river requirement is a minimum discharge for now; Task 4b/Phase 4b sizes micro-hydro against flow and environmental flow.


### Task 4b: Fuel and electricity

Upstream property has vehicles, power tools and computers but no fuel or power. In reality none of them run without energy.

**Files:**
- Create: `crates/mk_engine/src/regional/energy.rs`
- Modify: `crates/mk_engine/src/regional/property.rs`
- Test: `crates/mk_engine/tests/island_energy.rs`

**Interfaces:**
- Produces: `EstateEnergy { fuel_stores: Vec<FuelStore { fuel: Diesel | Petrol, litres }>, generators: Vec<Generator { rated_kw, litres_per_kwh }>, solar: Vec<SolarArray { rated_kw }>, batteries: Vec<Battery { capacity_kwh, charge_kwh }>, loads: Vec<Load { item, kw }> }`, initialised from `EstateEnergyConfig` in the scenario (owner decision; default below).
- Electricity: computers, network equipment, power tools and household appliances draw rated power only while in use; supply is solar (from the regional insolation and cloud at the estate), then battery, then generator burning fuel; when supply runs out, loads stop (computers lose access, power tools fall back to hand tools).
- Vehicles: each named vehicle has a fuel tank and consumption per km and per engine-hour on terrain (reference labour pack); a vehicle with an empty tank does not move.
- Burning fuel books CO₂ and O₂ through Task 3 (`MaterialCarbon → AtmosCO2`). There is no refinery: fuel is finite until a later phase adds a production route (e.g. biodiesel), which is recorded in the deviation register.
- Default `EstateEnergyConfig` (owner to confirm): one 10 kW diesel generator, 2,000 L diesel and 400 L petrol in storage, a 5 kW rooftop solar array and a 10 kWh battery. Phase 4b adds the river micro-hydro plant as the estate's main supply (the generator becomes backup) and a fuel depot supplied by the island refinery.

- [x] **Step 1:** Write tests: a computer runs on solar at midday and on battery at night until the battery is empty, then on the generator; the generator consumes fuel at its rated efficiency and stops when fuel runs out; a vehicle driven 100 km uses its reference fuel; with no power, computer access is denied even in the computer room; fuel burned raises atmospheric CO₂ by the stoichiometric amount.
- [x] **Step 2:** Run `cargo test -p mk_engine --test island_energy`; expect FAIL; implement; re-run; expect PASS.
- [x] **Step 3:** Commit `feat(island): fuel and electricity for the estate`.

Implementation notes: vehicles start at a quarter tank (a full tank each would empty the 2,000 L diesel store into the tractors and leave the generator none). Battery losses are 5% each way; the generator does not recharge the battery. Tank sizes and engine ratings are rounded manufacturer figures; consumption comes from the labour reference pack (a test pins the reference midpoints inside the pack's ranges). Burning is booked as `CrustCarbon -> AtmosCO2` (kg C) and `AtmosO2 -> AtmosCO2` (kg O₂ bound), following `conservation.rs`; the tests check fuel + O₂ = CO₂ + H₂O per litre. The plan's `MaterialCarbon -> AtmosCO2` entry is Task 3's; until then the carbon comes from the crust, as fossil fuel is.


### Task 5: Metric estate layout and navigation

**Files:**
- Create: `crates/mk_engine/src/regional/estate_layout.rs`
- Modify: `crates/mk_engine/src/regional/property.rs`
- Test: `crates/mk_engine/tests/island_estate_layout.rs`

**Interfaces:**
- Produces: `EstateLayout { property_id, patch: LocalPatchSpec, terrain_m: Grid2<f64>, buildings: Vec<BuildingFootprint>, spaces: Vec<SpaceLayout>, doors: Vec<Door>, items: Vec<ItemPlacement>, yard: Rect }`.
- `BuildingFootprint { building_id, kind: PropertyBuildingKind, rect_m, rotation_deg }`.
- `SpaceLayout { id: SpaceId, building_id, label, kind: SpaceKind, rect_m }` where `SpaceKind::{Room, Zone, WholeBuilding}`. Mapping from upstream labels: House items with `room` = `"Kitchen"`, `"Lounge"`, `"Bathroom"`, `"<name>'s Bedroom"` → `Room`; House `"General"` → `Zone` (hall); Shed items' tool categories (`"Construction Tools"`, `"Digging Tools"`, `"Repair Tools"`, `"Exploration Tools"`, `"Smithing Tools"`) → `Zone`s inside the Shed; Workshop, Armoury and Garage items (`room: None`) → one `WholeBuilding` space each, each its own footprint. ComputerRoom building items → a `Room` labelled `"Computer Room"` **inside the House footprint**, with its only door opening into the House (`"General"` hall). Footprints: House (including the computer room), Shed, Workshop, Armoury, Garage.
- `Door { a: Space, b: Space, position_m }` where `Space::{Outdoors, Inside(SpaceId)}`; `ItemPlacement { property_id, item_id, space, position_m }` (vehicles in the Garage or yard; computers in the ComputerRoom).
- Produces: `layout_estate(property: &StarterProperty, physical: &RegionalPhysicalState, domain: &IslandDomain, patch: &LocalPatchSpec, seed: [u8; 32]) -> Result<EstateLayout, EstateLayoutError>`, `EstateLayout::space_at(position_m) -> Space`, `EstateLayout::route(from: Space, to: Space) -> Option<Vec<Space>>`.
- Patch terrain is the medium-cell elevation bilinearly interpolated plus bounded deterministic detail; it never changes the regional elevation.

- [x] **Step 1:** Write tests: every founders'-estate item placed exactly once in the space its label/building maps to; the computer room lies inside the House footprint and is reachable from `Outdoors` only through the House; there are five footprints; homestead items not placed; no footprint overlaps; footprints on dry land under the slope limit; every space routable from `Outdoors`; `space_at` round-trips each placement; identical bytes for identical inputs; a patch with no buildable site returns `EstateLayoutError::NoBuildableSite`.
- [x] **Step 2:** Run `cargo test -p mk_engine --test island_estate_layout`; expect FAIL.
- [x] **Step 3:** Implement deterministic site selection inside the patch, rectangle packing per building kind sized from its space/item count, space subdivision and door placement.
- [x] **Step 4:** Run the test and `cargo test -p mk_engine --lib organisms::property`; expect PASS.
- [x] **Step 5:** Commit `feat(engine): lay out the founders estate in metres`.

Implementation notes: site selection scans every patch position against a 2-D prefix sum of unbuildable cells (O(1) per footprint), so a 640 k-cell patch costs milliseconds; a cell is buildable if >= 1 m above sea level, not in a lake or river (Phase-2 network and discharge) and no steeper than 2 m over 5 m. The Computer Room space keeps the upstream ComputerRoom building id while lying inside the House footprint. The plan's `regional/property.rs` change belongs with Task 4's placement. Tests run on the owner's island (first gentle lowland site the layout accepts) and cover item placement, the computer-room rule, routing, slope and ground, determinism, the sea-patch error and a House-less property.


### Task 6: Grid topology for the human and organism runtimes

**Files:**
- Create: `crates/mk_engine/src/topology.rs`
- Modify: `crates/mk_engine/src/humans/mod.rs` (`HumanSystem::step`, `seed_runtime_position_from_birthplace`, `cell_crossing_probability`, `step_toward`, `step_toward_nearest`, `someone_within_reach`, `step_by`, `grid_position_to_birthplace`, `birthplace_to_grid_position`)
- Modify: `crates/mk_engine/src/humans/lifecycle.rs` (`deliver_due_births`, `deliver_birth`)
- Modify: `crates/mk_engine/src/perception.rs` (`Occupancy::social_density`)
- Modify: `crates/mk_engine/src/organisms/runtime.rs` (wrapping neighbour steps)
- Modify: `crates/mk_engine/src/world_integration.rs` (pass `GridTopology::planetary`)
- Modify: `crates/mk_engine/src/regional/mod.rs`
- Test: unit tests in `topology.rs`; existing human/organism tests

**Interfaces:**
- Produces: `GridTopology { rows, cols, wraps_east_west: bool, … }` with `cell_area_m2(row)`, `cell_width_m(row)`, `cell_height_m()`, `neighbour(row, col, drow, dcol) -> Option<(usize, usize)>`, `col_delta(from, to) -> i32` (shortest, wrap-aware only when wrapping), `lat_lon(row, col) -> (f64, f64)`, `cell_for_lat_lon(lat, lon) -> Option<(usize, usize)>`.
- `GridTopology::planetary(grid_spec: &GridSpec, planet_radius_m: f64)` reproduces today's behaviour exactly (spherical area via `cell_area_at_row_m2`, `rem_euclid` wrap, lat/lon from `GridSpec`).
- `GridTopology::regional(domain: &IslandDomain, level: DomainLevel)` uses flat cells, no wrap, `domain.latitude_rad_for_row`/`longitude_rad_for_col`; `cell_for_lat_lon` returns `None` outside the domain.
- Every listed function takes `&GridTopology` where it took `&GridSpec`. A child born on the island gets a birthplace from `topology.lat_lon` of the mother's cell.

- [x] **Step 1:** Run `cargo test -p mk_engine --lib -- humans:: perception:: organisms::` and `cargo test -p mk_engine --test humans_world_integration`; record results and the `state_hash` of a 10-step default `WorldState`.
- [x] **Step 2:** Write topology tests: planetary area/neighbours/lat-lon equal the current functions for every row of a 32 × 64 grid; regional west-edge cells have no east-edge neighbours; regional `lat_lon` ↔ `cell_for_lat_lon` round-trips.
- [x] **Step 3:** Introduce `GridTopology` and thread it through the listed functions; `WorldState` passes `GridTopology::planetary`.
- [x] **Step 4:** Re-run Step 1; expect identical results and identical 10-step hash.
- [x] **Step 5:** Record in `UPSTREAM.md` as a geometry-only divergence suitable for upstreaming. Commit `refactor(engine): grid topology for human and organism movement`.

Implementation notes: baseline before the refactor was 309 humans/perception/organisms lib tests, 7 `humans_world_integration` tests and the 10-step `WorldState` hash `c771907a…6e73` (pinned in `tests/topology_hash_baseline.rs`); after, 312 lib tests (3 new), 7, and the identical hash. To keep every existing caller, the cores are `step_on`/`new_on`/`..._on` and the old names wrap them with the planetary topology. A birthplace outside the island leaves a human unplaced (the caller must place them). Deviation from the plan's file list: `world_integration.rs` needed only the `cell_crossing_probability` call updated (it keeps calling the planetary wrappers).


### Task 7: Dense vegetation in the estate patch

**Files:**
- Create: `crates/mk_engine/src/regional/local_vegetation.rs`
- Test: `crates/mk_engine/tests/island_local_vegetation.rs`

**Interfaces:**
- Produces: `LocalVegetationPatch { trees: Vec<TreeInstance>, stands: Grid2<StandCover>, individual_radius_m: f64, cap: usize }`, `TreeInstance { id, kind: PlantKind, position_m, height_m, stem_diameter_m, biomass_kgc, alive }`, `StandCover { biome, biomass_kgc, stem_density_per_ha }` per 5 m cell outside the individual radius (and under footprints/yard: zero).
- Individual stems are trees with stem diameter ≥ 0.10 m within `individual_radius_m` of the estate centre. If seeding would exceed `cap`, the radius shrinks deterministically (halving the excess area per iteration) until it fits; the displaced biomass stays in `stands`, so totals are unchanged.
- Patch biomass and the estate block's `biomass_kgc_m2` (the cell under each patch position) stay consistent: patch growth/mortality and felling change the cell field by the same carbon.
- Produces: `seed_local_vegetation(ecology: &RegionalEcologyState, layout: &EstateLayout, domain: &IslandDomain, config: &EstatePatchConfig, seed: [u8; 32]) -> Result<LocalVegetationPatch, LocalVegetationError>` and `step_local_vegetation(&mut self, ecology: &RegionalEcologyState, dt_seconds: u64)` (growth/mortality scaled to the NPP of the estate-block cell under each tree).
- Produces: `fell_tree(&mut self, tree_id, economy: &mut ResourceEconomyState, materials: &mut MaterialLedger) -> Result<WoodYield, LocalVegetationError>` — the stem is removed, wood enters the economy through the same item kinds a Tree node yields, and its carbon moves `BiomassCarbon → MaterialCarbon` through Task 3, debiting the biomass of the estate-block cell under the tree and producer species.

- [x] **Step 1:** Write tests: stem density follows biome/biomass (forest ≫ grassland, none in water); trees + stands sum to the four estate cells' biomass within ±2% with the cap both inactive and forced active (cap = 1,000); no tree in footprints or yard; count never exceeds `cap`; felling removes the stem, adds wood and moves exactly its carbon; deterministic bytes.
- [x] **Step 2:** Run `cargo test -p mk_engine --test island_local_vegetation`; expect FAIL.
- [x] **Step 3:** Implement deterministic Poisson-disc placement with spacing from biome/kind, size distribution from biomass, stands for the remainder, and growth/mortality tied to NPP.
- [x] **Step 4:** Run the test and `cargo test -p mk_engine --lib organisms::vegetation`; expect PASS.
- [x] **Step 5:** Commit `feat(engine): simulate individual trees around the estate`.

Implementation notes: `fell_tree` takes the ecology and a ledger instead of the plan's `ResourceEconomyState`/`MaterialLedger` (Task 3 does not exist yet) and returns a `WoodYield`; wiring the wood into economy items and `MaterialCarbon` is Task 3's. Per medium cell the stems are scaled to exactly their stem share of the cell's carbon over the ground inside the radius, so totals match the field apart from the yard's tiny share (tests assert +/-2%). A rich forest (9 kgC/m²) seeds ~241,000 stems within 1 km uncapped, so the default 200,000 cap is real. The 10 cm minimum is enforced by scaling only the excess over the minimum stem and dropping the smallest stems if even minimum stems exceed the cell's stem carbon.


### Task 8: Founders on the island, observation and interaction

**Files:**
- Create: `crates/mk_engine/src/humans/observation.rs` (extracted)
- Modify: `crates/mk_engine/src/world_integration.rs` (`step_humans_period` uses the extracted builder)
- Create: `crates/mk_engine/src/regional/humans.rs`
- Modify: `apps/island_humans/src/lib.rs` (delegate to the adapter where it overlaps; the app is retired in Phase 5)
- Test: `crates/mk_engine/tests/island_human_runtime.rs`

**Interfaces:**
- Produces: `build_observation(inputs: &ObservationInputs<'_>, position: &GridPosition, agent_id: &str) -> AgentWorldObservation`, extracted unchanged from the closure in private `WorldState::step_humans_period` (`world_integration.rs:2066-2150`). `ObservationInputs` borrows the climate, weather, insolation (`daylight_fraction`), hydrology, biome, property, economy and occupancy data the closure reads today.
- Produces: `HumanEstatePositions(BTreeMap<String, EstatePosition { space: Space, position_m: (f64, f64) }>)` — serialized side-table for humans inside the patch; humans outside the patch have no entry.
- Produces: `bootstrap_regional_humans(property: &PropertySystem, layout: &EstateLayout, topology: &GridTopology) -> Result<(HumanSystem, HumanEstatePositions), RegionalHumanError>`; founders start in their own bedrooms.
- Produces: `step_regional_humans(humans: &mut HumanSystem, positions: &mut HumanEstatePositions, economy: &mut ResourceEconomyState, property: &PropertySystem, layout: &EstateLayout, vegetation: &mut LocalVegetationPatch, physical: &RegionalPhysicalState, ecology: &RegionalEcologyState, topology: &GridTopology, tick: Tick, dt_seconds: u64, rng: &RngRegistry) -> Result<(), RegionalHumanError>` — calls upstream `HumanSystem::step` with the regional topology and `build_observation`; movement inside the patch follows `EstateLayout::route`.
- Computer access (island divergence): `computer_access` is the upstream gate (ComputerRoom on the cell and a matching `NetworkAccount`, `world_integration.rs:2130-2146`) AND `positions[agent].space` is the House's computer room AND the computer has power (Task 4b). The network-account gate itself is unchanged.

- [x] **Step 1:** Run `cargo test -p mk_engine --test humans_world_integration` and record results; extract `build_observation`; re-run; expect identical results.
- [x] **Step 2:** Write tests: exactly Gem-D/Gem-K by default, each starting in their own bedroom; IDs/profiles match canonical constructors; a founder routed to the computer room (through the House) gains access, a founder in the Kitchen does not, and a non-account human in the computer room remains denied; a founder felling a patch tree adds wood and moves its carbon through Task 3's hooks; a founder eating from their supplies produces respired CO₂; a founder walking out of the patch loses their `HumanEstatePositions` entry and moves on medium cells without wrapping.
- [x] **Step 3:** Run `cargo test -p mk_engine --test island_human_runtime`; expect FAIL.
- [x] **Step 4:** Implement the adapter without changing human cognition/body modules.
- [x] **Step 5:** Run the test, `cargo test -p mk_core human`, and `cargo test -p mk_engine --lib humans::`; expect 0 failures.
- [x] **Step 6:** Record the observation extraction and room-level computer access in `UPSTREAM.md`. Commit `feat(engine): run canonical humans on island`.

Implementation notes: the extraction is of the observation's pure formulas (`humans/observation.rs`) rather than the whole closure, because the planetary and island worlds read different data types; the planetary closure now calls them and the hash and tests are unchanged. `step_regional_humans` takes a `RegionalHumanContext` (property, layout, physical, ecology, energy, domain, solar kW) instead of the plan's long argument list, and has no `vegetation` argument: felling is `LocalVegetationPatch::fell_tree`, called by whoever drives the action. **Not done in this task, because they need Task 3:** the felling-adds-wood test and the eating-produces-CO₂ test; they move to Task 3/9. Four tests run on the owner's island: founders in their own bedrooms with canonical profiles, the computer rule (account + computer room + power), plausible observations and house shelter, and walking out of the patch with no wrap.


### Task 9: Phase-3 integrated life/property acceptance

**Files:**
- Create: `crates/mk_engine/tests/island_life_acceptance.rs`
- Modify: `UPSTREAM.md`

**Interfaces:**
- Consumes all Phase-3 APIs.

- [ ] **Step 1:** Bootstrap fixed-seed physical + ecology + resources + property + layout + local vegetation + humans, advance one simulated week at 60 s human steps (hourly physical, 6-hourly ecology — call the Phase-2/3 step functions directly; the scheduler arrives in Phase 4), and assert living vegetation (regional and patch), resources, both founders inside the layout, intact estate inventory, economy events, closed carbon/oxygen/water stock audits every step, and deterministic final serialization. Add slow-tier realism checks against the Phase-0c packs: NPP by biome, forest stem density and allometry in the patch, founders' daily energy expenditure and food/water use, and labour productivity. Hash a canonical form (serialize to `serde_json::Value`, whose maps are sorted, then to bytes): upstream state such as `AdvancedMemorySnapshot` holds `HashMap`s (`advanced_memory.rs:70-85`) whose iteration order is not stable across processes. Slow tier.
- [ ] **Step 2:** Run the acceptance test twice in separate processes; expect identical blake3 digests and PASS.
- [ ] **Step 3:** Update provenance for scenario/adapter files; run fmt, clippy `-D warnings` and Phase-3 focused tests.
- [ ] **Step 4:** Commit `test(island): lock life property human acceptance`.
- [ ] **Step 5:** Add `island_preview life` writing `biomes.png`, `resources.png`, `estate_plan.png` (footprints, spaces, doors, item markers, founder positions) and `estate_trees.png`, with the Phase-1 determinism test pattern; commit the default preview under `docs/previews/phase3/`.
- [ ] **Step 6 (Gate 3 review):** The owner checks the estate plan and tree map before Phase 4.
