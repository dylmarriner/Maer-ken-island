# Upstream provenance

Source repository: `dylmarriner/Maer-Ken`
Source commit: `7c05f0dcf254387ffd7322dbb525fe4807228602`
Extraction date: 2026-10-01

Imported directly from that commit:

- `crates/mk_core`
- `crates/mk_engine`
- `crates/mk_interventions`
- `fixtures/human`
- canonical human schema documents under `docs/canon`
- `tools/blender/generate_human_model.py`
- Gem-D and Gem-K GLB assets

Intentional island divergence at extraction:

1. `mk_engine::humans` is public in this repository so the island application can use the human runtime directly.
2. `apps/island_humans` is new and owns island population bootstrapping.
3. A small amount of rustfmt-only normalization occurred in copied source files.

## Phase 0 divergences (2026-10-02)

Each of these changes imported code. None has been proposed upstream yet; per the roadmap, human behaviour and biology changes should be made upstream first and synced, so items 4-7 need an upstream PR before the next resync.

1. `mk_core::flux` — the `reservoir_all` test now checks every `Reservoir` variant exactly once through an exhaustive `match` (the old hard-coded count of 31 was stale; there are 32).
2. `mk_engine::world_integration::step_ledger_clear` no longer clears `audit_trail.entries`. `step_world` already clears them on entry, so the trail stays bounded to one tick and is observable after `step_world` returns. Snapshots now carry one tick of audit entries.
3. `mk_engine::io::encryption` — `parse_storage_key` validates length and ASCII hex instead of panicking; the key file is created atomically with mode `0600` (temp file plus hard link, falling back to exclusive direct creation on filesystems without hard links); `EncryptionError` implements `Display`/`Error`. `HumanStorage::try_new`/`try_with_key_hex` propagate key errors, and `HumanStorage::new` logs instead of silently downgrading. `World::enable_persistent_humans` uses `try_new`.
4. `mk_engine::humans::computer_bridge` — retries go through `retry_with` and `RetryPolicy`. `SendEmail` is never retried on a timeout, and the final attempt's real error is returned (it used to be reported as `RateLimitError`).
5. `mk_engine::humans::registry` — who exists no longer depends on the disk. `add_human`, `create_human` and `create_named_human` reject duplicate ids (`HumanStorageError::DuplicateAgent`) before inserting, otherwise insert first and then write the folder. `create_human`/`create_named_human` now return `Result<(), _>`. `seed_founders`, `deliver_due_births`, `spawn_human` and `enable_persistent_humans` keep humans when a folder write fails; `sync_to_storage`, `sync_human_to_storage` and `record_event` return their errors.
6. `mk_engine::humans::registry` — an agent-id index (derived, never serialized) makes id lookups O(1); iteration order is unchanged; `get_all_humans_mut` returns a slice.
7. `mk_engine::humans::HumanSystem::step_dialogue` — pairing is O(n log n) and realistic: each human is in at most one conversation per step, partners are chosen by relationship, mutual approach and distance. Behaviour change: pairing is one greedy matching in which family pairs (founders, child–parent at any distance, adjacent siblings) outrank neighbour pairs and rotate by a per-tick hash, so a child talks to one parent per step rather than both, and the founders still talk to each other and to their children over time. Conversations still last one step; multi-step conversations are not modelled.
8. Clippy cleanups: removed the unused `HYDROLOGY_SPIN_UP_STEPS`/`HYDROLOGY_SPIN_UP_STEP_SECONDS` constants and `BrainRegionsSnapshot::resting_mean_activation`, and used `RangeBounds::contains` in `orbit`.

## Phase 0b divergences (2026-10-02)

1. `mk_engine::humans::spawn` (new) — the human-building core of `interventions::spawn_human` is extracted into `build_spawned_human`, `build_authored_human` and `agent_id_for_name` (now taking a `HumanRegistry` instead of a `WorldState`), with `SPAWN_HUMAN_EPOCH` moved alongside. `spawn_human` calls them, so intervention spawns are unchanged (all intervention tests pass unmodified); the island population uses the same functions to create people without a planetary world. Suitable for upstreaming as a pure refactor.

## Phase 0c divergences (2026-10-07)

Human biology changes made here to pass `crates/mk_engine/tests/human_realism.rs` against `fixtures/reference/humans/`. Like Phase 0 items 4-7, each needs an upstream PR before the next resync; cognition and behaviour code is unchanged.

1. `mk_engine::humans::circadian` (new) — the Forger–Jewett–Kronauer (1999) light-driven circadian pacemaker with a per-human intrinsic period (mean 24.15 h observed, SD 0.2 h), light-suppressed melatonin, and a two-process (Borbély) sleep gate. `HumanBeing` gains `circadian` (`#[serde(default)]`); `step_lifecycle` steps it before `needs`.
2. `mk_engine::humans::needs` — `fatigue` is now the homeostatic sleep pressure, owned by the clock (rises with an 18.2 h time constant awake, falls with 4.2 h asleep, slower in poor shelter). Rates are per day instead of per year: water is lost at 0.2 of the reserve per day plus sweating above 25 °C (death in 4-5 days without water), glycogen lasts a day, and the new `energy_reserve` (fat and protein, `#[serde(default)]` = full) carries a fasting human for ~60 days. Eating capacity is ~3x a day's expenditure, so food access above ~0.35 sustains a person (before, under 0.77 was a slow death). Drinking capacity is now ~10x the resting loss (about 1 L/h), so water access above ~0.1 keeps a person hydrated; before, anywhere under 0.77 was a slow death.
3. `mk_engine::humans::neurochemistry` — melatonin is no longer a two-year relaxation toward darkness; `apply_circadian` takes it and the sleep state from the clock, and an asleep human is in `BrainState::Fatigued` (which `autonomy` already treats as sleep). New `asleep` field, `#[serde(default)]`.
4. `mk_engine::humans::reproduction::GESTATION_YEARS` — 268 days from conception (Jukic et al. 2013) instead of 280 (40 weeks counts from the last menstrual period, not conception).
5. `mk_engine::humans::lifecycle` — the Gompertz constants are `pub` so the realism suite can check them.

## Phase 1 divergences (2026-10-07)

Phase 1 adds the regional island domain and its geophysics. Almost all of it is new code; only item 2 changes imported code, and that change is a refactor that keeps behaviour the same.

1. `crates/mk_island` (new) — the regional domain contract: `IslandProfile` (version, size, target land area, ocean buffer, shape requirements, basement, optional seed), `IslandDomain` (flat metre grids at coarse/medium/fine levels; row 0 is south, col 0 is west, and edges never wrap), regional boundaries and `ShapeMetrics`. It depends only on `mk_core`; `mk_engine` depends on it (`mk_engine → mk_island → mk_core`). `mk_engine` also gains `rayon` for deterministic multi-core cell maps (`regional::par`).
2. `mk_engine::volcanism` — `step_volcanism` now delegates to `step_volcanism_on` over a `VolcanismGeometry` trait (cell height/width and neighbours). `SphericalVolcanismGeometry` gives the same cell sizes and `rem_euclid` longitude wrapping as before, so the global path is unchanged and the upstream volcanism tests pass unmodified. Suitable for upstreaming as a pure refactor.
3. `mk_engine::regional` (new) — island-scale geophysics built on the imported models without changing them: tectonics (`regional_plates`, `step_regional_tectonics` on a flat, non-wrapping domain), seismicity from regional fault stress, volcanism through the item-2 geometry, terrain (a continental plateau, forearc and main range, trench and outer rise, arc and rift volcanoes, domain-warped noise, a stand-in for erosion), sea-level fitting to the target land area, with an edge ceiling relative to sea level that keeps coasts off the ocean buffer, then lithology and primary mineral deposits. Upstream has no rock-type map and places resources by biome. Here deposits form only where their geology allows, and petroleum only under water no deeper than 500 m.
4. `apps/island_preview` (new) — renders the geophysics maps and a seed gallery to PNG/JSON.

## Phase 2 divergences (2026-10-07)

1. `mk_engine::world_integration::spin_up_climate` is `pub(crate)` (was private) so the zonal background reuses the world's own two-orbit spin-up instead of copying it. No behaviour change.
2. `mk_engine::regional::zonal` (new) — the zonal background: upstream `step_climate` and `step_weather` unchanged on a 64-band × 64-column grid, where each band's land columns match upstream tectonics' land fraction at that latitude. Calibrated against the upstream 32 × 64 world (±1.5 K per band, ±0.5 K global mean).
3. `mk_engine::climate` — `step_climate` now delegates to `step_climate_on`, which takes row latitudes and optional `RegionalClimateTerms` (CO₂, planetary mean radiative temperature and absorbed flux from the zonal background, a per-cell equilibrium offset for the lapse rate); the radiative field is `radiative_field`, shared with `planetary_radiative_terms`. With `None` and the grid's own latitudes it is exactly the old function (all upstream climate, weather, ocean and hydrology tests pass unmodified). `weather::{zonal_wind, meridional_wind_from_gradient, rainout_weight, VAPOUR_COLUMN_FRACTION}` are `pub(crate)` for the regional weather. Suitable for upstreaming as a pure refactor.
4. `mk_engine::regional::{levels, edge, climate, weather}` (new) — coarse↔medium resampling (bilinear / area mean, exactly conservative), edge-cell relaxation toward boundary forcing, regional Coriolis, a 6.5 K/km lapse rate, a diurnal cycle, regional weather with per-row rain normalised to the background, and orographic rain. Upstream has none of the lapse rate, diurnal cycle or orographic rain (D3, D26).
5. `mk_engine::regional::synoptic` (new) — travelling cyclones, anticyclones and fronts from the Eady growth rate, deformation radius and thermal-wind steering; modulates the regional weather and conserves its long-run mean. No upstream equivalent (D4).
6. `mk_engine::ocean::step_ocean` delegates to `step_ocean_on` (row latitudes as a parameter) and `mk_engine::tides::step_tides` to `step_tides_on` (row latitudes, column longitudes and a `power_share` of the planet's tidal heating); with the grid's own geometry and `1.0` they are exactly the old functions (upstream ocean and tides tests pass unmodified). `ocean::ocean_density` is `pub(crate)`. Suitable for upstreaming as a pure refactor.
7. `mk_engine::regional::{ocean, tides}` (new) — the regional ocean (upstream equations on domain latitudes, then edge relaxation of salinity and normal inflow, clamped to 0-45 PSU and ±5 m/s) and tides (analytic phase; degree-2 potential at the domain's latitudes/longitudes; dissipation booked as the ocean's share of the planet's surface). The tide field is on the coarse grid, not the medium one the plan named: it varies over thousands of km, and a medium grid would spend ~46 MB per snapshot on near-constant fields; consumers resample with `levels::resample_coarse_to_medium`.
8. `mk_engine::hydrology` — `infiltration_capacity`, `potential_evaporation` and the bucket constants are `pub(crate)`; nothing else changes (upstream hydrology is untouched).
9. `mk_engine::regional::hydrology` (new) — upstream's bucket equations on the medium grid with flat area, `cell_size_m` slopes, domain latitudes and no wrap, over a once-per-terrain `FlowNetwork` (priority-flood depression filling, steepest-descent receivers): channel water is delivered downstream within the step (upstream moves it one cell per step, ~75 days for a 150 km river on 2 km cells), depressions deeper than 0.5 m are lakes that fill to their spill level, only land cells are iterated, and the coarse climate and weather are sampled at each land cell instead of being resampled to medium grids. Water leaving into the sea or off an edge is `surface_to_ocean_kg`; river losses (evaporation, transmission) and the within-step channel delay are not modelled (D28).
10. `mk_engine::insolation::step_insolation` delegates to `step_insolation_on` (row latitudes, column longitudes, per-row cell area); with the grid's own geometry it is exactly the old function. Suitable for upstreaming as a pure refactor.
11. `mk_engine::regional::physical` (new) — `RegionalPhysicalState`: the coupled tick (background → boundaries → insolation → tides → climate + diurnal → synoptic → weather + orographic → hydrology → river water → ocean). The regional systems book only the tidal-heat ledger entry; the planetary world's climate-step energy bookkeeping (`conservation::book_climate_step`) has no regional counterpart yet and belongs with Phase 4's audit (D29). River water enters the ocean as extra precipitation on the coarse cell containing each outlet, which may be a land cell where the coast is narrower than 12 km; that water is then not counted by the ocean.
12. `mk_engine::ocean::surface_current_from_wind` — **behaviour change, needs an upstream PR.** Upstream gave `0.008·|w|·w·cos 45°/100`: 0.28 cm/s for a 7 m/s wind (50-100x below observed), with no Ekman turning and the Coriolis parameter ignored. It is now wind drift at 3% of the wind speed (Wright & Thompson 1983) turned 45° right of the wind where `f > 0` and left where `f < 0`, capped at 5 m/s. It changes the global ocean's currents (nothing else in the repository reads their magnitude).
13. `mk_engine::regional::ecology` (new) — island biomes (`classify_biome`), NPP (`miami_npp_kgc_m2_yr`) and standing biomass on the medium grid from the physical state's annual climatology, hydrology and geophysics, with a sparse `VegetationSystem` view; no upstream code changes (both functions were already pure). Species populations are not yet seeded at island scale.
14. `mk_engine::topology` (new) and its threading — `GridTopology::{planetary, regional}` holds the geometry the human and organism runtimes use (cell area and width, east/west wrap, neighbours, shortest column step, lat/lon and cell lookup). `HumanSystem::step_on`, `OrganismRuntime::step_on`, `Occupancy::new_on` and `seed_runtime_position_from_birthplace_on` are the cores; `step`, `Occupancy::new` and `seed_runtime_position_from_birthplace` keep their signatures as planetary wrappers. `step_toward`, `step_by`, `someone_within_reach`, `step_toward_nearest`, `cell_crossing_probability`, `apply_movement`, `directed_seek_step`, `deliver_birth`, `deliver_due_births` and `grid_position_to_birthplace` take `&GridTopology` instead of `&GridSpec`. The planetary topology reproduces the old behaviour exactly (a pinned 10-step `WorldState` hash and every humans/perception/organisms test are unchanged), including the old birthplace mapping (row 0 at 90° N, mirrored against `GridSpec::lat_rad`; kept on the planet, correct on the island). Geometry only; suitable for upstreaming.
15. `mk_engine::regional::estate_layout` (new) — the founders' estate in metres on the 4 km patch (5 m cells): deterministic site (nearest the patch centre where all five footprints fit on dry, non-lake, non-river ground at most 2 m per 5 m steep), House with hall, rooms and the computer room inside it (its only door into the hall), Shed zones by tool category, single-space Workshop/Armoury/Garage, doors, a lattice position for every `PropertyItem`, and `route` over the door graph. Reads `StarterProperty` and defines no items; upstream property code is untouched. Building sizes (5 × 5 m rooms, 2.5 m hall, floor area per item) are the layout's own plausible dimensions, not upstream data.
16. `mk_island::scenario` (new) and `mk_engine::regional::property` (new) — the island scenario (`IslandScenario`: seed, canon path, profile, cadences 60 s / 1 h / 6 h / 1 d / 1 h, 4 km × 5 m estate patch, estate energy defaults, founders/estate switches; `fixtures/island/default_scenario.json`) and estate placement: upstream's year-round-climate scoring (temperature, seasonal swing from the canon obliquity, rain, height penalty) over 2 × 2 medium-cell blocks that are dry, non-volcanic, not lake/river/wetland/alpine/ice, with a river of at least 1 m³/s within 1 km (Phase 4b sizes the micro-hydro plant against the real flow). The inventory is upstream's `PropertySystem::new(Some(location))`, unchanged. The best-scoring site is tried first; the layout must also fit its patch, else the next site is tried.
17. `mk_engine::regional::energy` (new) — estate fuel and electricity: loads (computers 0.2 kW, power tools 1.5 kW) run only while in use and only if supplied (solar from insolation and cloud, then battery at 95% each way, then a diesel generator at 0.30 efficiency on 9.97 kWh/L); vehicles have tanks (road: reference 4WD L/100 km by surface; tractors and excavator: 0.27 L/kWh over rated power × load) and do not move when empty; every burn books carbon (crust → atmosphere, kg C) and the bound free oxygen (`O2_PER_CARBON`) from the fuel's stoichiometry. Upstream property and vehicles are untouched. The hydrogen's water, and the carbon/oxygen of materials, wait for Task 3's material ledger; no refinery exists (D16).
18. `mk_engine::regional::local_vegetation` (new) — individual stems (>= 10 cm, within 1 km of the estate, at most `tree_cap`) and 5 m stand cover on the estate patch, carrying exactly the four estate cells' biomass over the ground outside the yard. Carbon follows Chave et al. (2014) pantropical allometry (a temperate/boreal wood density of 0.55 g/cm³ is assumed, not the model's own species data); stems are placed by thinned jittered-lattice sampling with a reverse-J size distribution; the cap shrinks the radius (halving the excess area per pass) and the displaced carbon stays in stands; the patch follows the ecology field (growth) with a 1/150 per year background mortality that moves dead carbon into stands; felling debits the cell field. Felled carbon is booked `BiomassCarbon -> DetritusCarbon` until Task 3's `MaterialCarbon` exists, and wood is returned as a `WoodYield` (not yet an economy item). `VegetationSystem` upstream is untouched.
19. `mk_engine::humans::observation` (new, extracted) and `mk_engine::regional::humans` (new) — the observation's pure parts (biome shelter `canopy*0.6+0.2`, biome hazard, the computer gate, `build_observation`) moved out of `WorldState::step_humans_period` unchanged (the planetary world is bit-identical: pinned ten-step hash, 7 world-integration tests, 302 humans/perception/world lib tests). The island adapter builds Gem-D and Gem-K from upstream's constructors in their own bedrooms, runs upstream `HumanSystem::step_on` over the medium topology with an island observation (coarse climate plus diurnal cycle, medium hydrology, ecology NPP through the same saturating forage curve, estate building shelter), and keeps a serialized side-table of metric positions for humans inside the patch (entries drop when a human leaves the 2 × 2 block). **Computer access (island divergence)** = upstream's gate AND standing in the House's computer room AND the estate able to supply power (battery charge, generator fuel, or sun). Within the estate, movement between spaces follows `EstateLayout::route` (instantaneous until Task 3b). Not yet done: felling and eating hooks through Task 3's material ledger (felling exists in `local_vegetation`; eating's respired CO₂ needs Task 3).
20. `mk_engine::regional::labour` (new) and `humans::needs::EffortFocus::activity` — **a needs-model change, needs an upstream PR.** `EffortFocus` gains `activity` (serde default 0, treated as the 1.0 baseline): the energy expenditure of the current activity as a multiple of the baseline the drain rates are calibrated for (light daily activity, 1.5 MET). `NeedsSnapshot::step` multiplies the glucose drain by it (clamped 1-8) and the water loss by `1 + 0.5 (activity - 1)` (breathing and sweat; the 0.5 is an assumption, low confidence). Every existing caller uses the baseline, so planetary behaviour is unchanged (pinned hash, realism suite, 282 humans tests). The labour module builds timed tasks from the reference packs: felling by tool (time scaled by trunk cross-section from the 30 cm figures), digging, hand mining, panning (yield = grade × throughput × recovery), timber-frame building, Mifflin-St Jeor BMR from the packs' terms, energy `BMR × MET × hours`, walking (1.4 m/s, Naismith ascent, slowdown past the sustainable load, refusal above the maximum), and `ActiveTask` progress. **Not done:** routing the economy's actions (`resource_economy.rs`) through timed tasks, and `lifecycle` setting `activity` from the action: both change planetary human behaviour and belong with the integration (Task 9) after an upstream decision.
21. `mk_core::flux` — new reservoirs `MaterialCarbon`, `MaterialWater`, `HumanCarbon` (appended last, so every earlier index and any serialized ledger is unchanged; the exhaustive `reservoir_all` test now covers 35; all three are interior reservoirs). `mk_engine::regional::materials` (new) — `MaterialLedger`: gathering biotic material debits the cell's biomass (`BiomassCarbon → MaterialCarbon`) and cannot exceed it (regrowth is the ecology's NPP); coal and limestone come from `CrustCarbon`; hunting kills whole animals through a fractional accumulator (meat carbon to material, carcass to detritus); burning and charcoal-making release carbon to `AtmosCO2` binding 2.664 kg O₂ per kg C; calcination's CO₂ carries the carbonate's own oxygen; eating moves carbon into the body, respiration returns it at a respiratory quotient of 0.85 (≈0.35 g CO₂/kcal) binding oxygen, death sends the body's carbon to detritus; water comes from standing water or 1% of river flow (`SurfaceWater`/`Rivers → MaterialWater`) and returns to the air and soil; structures hold carbon until demolition or 60-year decay. Every composition value carries its source in code (Lamlom & Savidge 2003 for wood, FAO 1987 for charcoal, USDA for food and meat, CaCO₃ stoichiometry for limestone, Heymsfield et al. 2007 for body carbon). **Not done:** species populations are not debited when biomass is gathered (needs Task 1b's island species), `conservation.rs`'s stock audit does not yet read `MaterialLedger::stock_of` (the tests audit each reservoir's stock against the ledger directly), and `resource_economy.rs`/`humans::lifecycle` do not call the hooks yet (planetary behaviour is unchanged); embedded water in foods and metabolic water's oxygen are not booked. Wiring is Task 9.
22. `mk_engine::regional::life` (new) — `IslandLife` bundles every Phase 1-3 system and advances them at the plan's cadences (60 s humans, hourly physics, 6-hourly ecology and households), calling the step functions directly until Phase 4's scheduler exists. The founders' household routine is scripted, not autonomous: each hour they respire at 1.3 MET and the estate steps its electricity; every 6 h each takes 0.6 kg of plant food from the estate block's richest cell and 0.65 kg of water from the nearest river cell, eaten and drunk through the material ledger; each such step is audited (the ledger's net flow into `MaterialCarbon`, `HumanCarbon` and `MaterialWater` must equal the change in each stock). The patch trees' height now uses the reference pack's own Chave et al. (2014) height-diameter relation. The canonical state digest (JSON `Value`, sorted maps, hashed) of a simulated week is pinned and identical across processes. **Still open:** autonomous economy actions through the material and labour hooks, species-carbon debiting (Task 1b), Task 2's catalogue.
23. `mk_engine::regional::ecology` island corrections (D32) and `island_preview life` — **a fidelity fix found by reviewing the preview:** upstream `classify_biome` labels ground under 283 K and 1 mm/day an ice sheet; on the island that was 34,000 of 66,000 land cells at 270-280 K (-3 to +7 °C). The island keeps `IceSheet` only below 260 K (upstream's own hard limit) and otherwise calls cold dry ground tundra (< 278 K) or shrubland; and a cell is `River` only when the river's width (Leopold & Maddock, 3 Q^0.5 m) fills half of it (previously every 1 m³/s stream made a 2 km-wide River cell, 10% of land with no vegetation). The estate moved as a result (best site now medium cell (474, 544) on the wet west coast, 200,000 trees). `island_preview life` writes `biomes.png`, `resources.png` (Phase 1 deposits and water; the full catalogue needs Task 2), `estate_plan.png` (8 px/m: footprints, spaces, doors, items, founders) and `estate_trees.png`; the default is committed under `docs/previews/phase3/`.
24. `mk_engine::regional::boundary` — `sample_regional_boundaries_with_background` (`provisional: false`) takes ocean and atmosphere edges from the background's bands; the Phase 1 sampler is kept for geophysics and now shares its loop.

## Phase 4 divergences (2026-10-07)

25. `mk_engine::io::island_snapshot` (new) — `save_island_snapshot`/`load_island_snapshot` write and
    read a whole island as `magic ‖ blake3(rest) ‖ canon digest ‖ deflated JSON`, atomically
    (temp file, fsync, rename, fsync the directory). The canon is supplied by the caller and
    checked against the file's digest rather than read from it, and `IslandLife::restore`
    validates the snapshot's scenario as if it had come off disk and then derives the domain,
    grid topology and labour table again instead of trusting the file. `mk_engine` gains
    `flate2` for this (already in the tree through `ureq`): a full island is a dozen grids of
    1,152,000 cells and comes to 338 MB written plainly. Island-only: upstream's own
    `io::snapshot` for `WorldState` is untouched, and the new serde derives are all on island
    types (`regional::{physical, ecology, property, local_vegetation, estate_layout, materials}`),
    not on imported ones.

26. `mk_engine::humans::HumanSystem::set_auto_sync` — **an upstream behaviour switch, needs an
    upstream PR.** `HumanSystem::step` rewrites every human's full-state files on every call
    (`humans/mod.rs`). `auto_sync` defaults to `true`, so the planetary `WorldState` is unchanged,
    and it is `#[serde(skip)]` so no serialized world and no state digest moves. The island sets
    it `false`: at a 60-second human step that is 1,440 full rewrites per simulated day. Births,
    deaths and events are still written when they happen either way.
    `mk_engine::regional::human_store` (new) — one run's folder tree:
    `<save root>/<run id>/humans/`, `run id = blake3(scenario digest ‖ seed ‖ counter)[..12]` with
    the counter in `<save root>/runs.json`, opened through `HumanStorage::try_new` so a missing
    storage key refuses rather than writing records in plaintext. Island-only; upstream
    `HumanStorage` and `HumanRegistry` are used as they are.

27. `mk_engine::regional::interventions` (new) — upstream's `InterventionAction` executor adapted
    to `IslandLife`. `IslandLife::intervene` validates with upstream's own
    `validate_intervention` and then applies, or refuses with
    `IslandInterventionError::NotSupportedOnIsland { action, reason }`; `IslandCommand::Intervention`
    carries it into the replay log, so an intervened-in run replays to the same canonical digest
    (`a_run_somebody_intervened_in_replays_to_the_same_island`). Island-only: upstream's
    `interventions::InterventionExecutor` against `WorldState` is untouched, and `mk_interventions`
    is unchanged — the island speaks upstream's action vocabulary rather than a translation of it,
    so a log records what was actually asked for.

    **Applied on the island:** `Pause`, `Resume`, `Step` (directives, as upstream);
    `ModifyClimate` (all four parameters, against the island's own coarse climate and weather
    grids, with the heat booked from `Reservoir::OperatorIntervention` exactly as upstream books
    it); `InjectBiomass { Producers }`; `InjectResource { Water }`; `ConstructStructure` (the island's
    economy is upstream's `ResourceEconomyState` and the recipes are the same, through the island's
    own buildability rule — item 28); `SpawnHuman`; `RemoveHuman`.

    **`NotSupportedOnIsland`,** with the reason each refusal gives. The list is
    `regional::interventions::refusal` and `every_action_is_supported_or_refused_by_name` holds
    the two together by walking every variant:

    | Action | Why the island has no meaning for it |
    |---|---|
    | `SculptTerrain`, `SmoothTerrain` | The island's terrain is generated from its canon-locked scenario and hashed into the state digest. An edited coastline would put an island out of agreement with its own canon, and every snapshot and replay of it is checked against that. |
    | `InjectBiomass` — `Consumers`, `Apex`, `Decomposers` | The island has no animal populations. `regional::ecology` carries standing producer carbon and NPP; species are not seeded at island scale (item 13). `Producers` is applied, against the biomass field. |
    | `InjectResource` — `Minerals`, `Nutrients`, `Energy`, `Organic` | The island's stores are nine named materials (wood, charcoal, coal, limestone, quicklime, plant food, meat, fibre, water). Only water names the same thing in both vocabularies; the rest would have to be guessed at. |
    | `InjectEnergy` | The island's energy is the estate's own plant — fuel stores, generators, solar arrays, batteries — not an energy field over the ground, so there is nowhere a quantity of energy at a coordinate would land. |
    | `TriggerDisturbance` | The island runs no disturbance system; `crate::disturbance` is planetary state its physical step does not carry. |
    | `ModifyScenario` | The island's scenario is an `IslandScenario`, whose digest every snapshot and replay is checked against. Upstream's three settable parameters are planetary and name nothing in it. |
    | `Scrub`, `Branch`, `Fork` | The island can be saved and loaded (`io::island_snapshot`), but its runner keeps no branch registry for a fork to be rooted in. |

    Two island differences inside the actions that *are* applied. A `Location` off the domain is
    **refused** rather than clamped: upstream's planet has a cell for every coordinate, and
    clamping here would quietly move an intervention aimed at the open sea onto the nearest coast.
    And `RemoveHuman` touches four places where upstream touches one — the registry, the estate
    position table, the body in the material ledger (out through the boundary, not to detritus:
    a removal is not a death) and the two per-person accumulators, which are hashed, so a ghost
    left in any of them would make two islands that agree about who is alive disagree about their
    digest.

    `mk_engine::regional::materials` gains `inject` and `remove_body` for the two crossings above,
    both booked against `Reservoir::OperatorIntervention`, and `RegionalPhysicalState` gains an
    `elevation_coarse_m()` accessor. **Note on the energy booking:** the island's physical state
    rebuilds its ledger at the start of every physical step — it is a record of that step's fluxes,
    not a running total — and the island keeps no cumulative energy ledger. So a climate
    intervention's joules are readable on the step the intervention lands on and gone after the
    next one. The material injections are different: `MaterialWater` and `MaterialCarbon` are
    audited against the ledger on every household step, so those go through `IslandLife::audited`
    and the audit closes on the tick they land.


28. `mk_engine::regional::geophysics::{MAX_BUILD_GRADIENT, is_buildable_cell}` (new) and
    `mk_engine::resource_economy::ResourceEconomyState::place_structure` (new) — **the island's
    buildability rule, and the seam it needed.**

    Upstream's `physics::is_buildable` gates on `MAX_CLIMB_HEIGHT_M`: an **absolute** 2 m of
    relief between a cell and each of its four neighbours, ported from a game with small cells
    and carrying its own note that it "may need tuning against Maer-Ken's actual elevation
    scale". That is a statement about a grid's resolution, not about terrain — the same hillside
    is 2 m per cell on one grid and 200 m per cell on another. On the island's 2 km medium cells
    it is a gradient of 0.1%, and measured across the default island it admits **0 of 66,116 land
    cells**. Not almost none: none, the founders' estate at 8.6% included, where the house
    already stands.

    So the island asks the same question as a gradient. `MAX_BUILD_GRADIENT` is 1 in 3 (about
    18°), where ground is conventionally classed very steep and building stops being a footing on
    a slope and becomes engineered terracing. It is a planning threshold, not a physical one —
    dry soil stands far steeper, its angle of repose nearer 30–35° (a gradient of 0.58–0.70) — so
    it is about what can be built on cheaply rather than what stands up. Sea neighbours are
    skipped rather than counted as a drop to the sea floor, which would refuse every coast, and
    the coast is where people build.

    Measured gradient distribution over the default island's land, which is what the threshold
    was chosen against rather than guessed at:

    | at or under | share of land |
    |---|---|
    | 0.1% (upstream's gate here) | **0.0%** |
    | 2% | 22.9% |
    | 5% | 79.6% |
    | 10% | 95.0% |
    | 20% | 99.4% |
    | 30% | 99.9% |
    | steepest land on the island | 42.5% (849 m over 2 km) |

    **What it cannot do, stated rather than left to be discovered.** At 2 km per cell this is a
    mean gradient across kilometres, so it cannot judge a building plot: a 15% cell holds flat
    benches and steep faces and this sees neither. It excludes mountainside, and that is all it
    claims. It admits 99.9% of this island's land — which is the island being gentle at this
    scale, not the rule being lax. A gate tightened to look strict would turn down ordinary
    ground, which is the failure mode it exists to end.

    `place_structure` is `construct_for_operator` with the terrain gate lifted out in front of
    it: upstream's path is unchanged and still gates, and the island calls the inner one after
    making its own judgement. The recipe check and the event-log record are shared, so a
    structure placed by the island is as visible in the economy's books as any agent-built one;
    the only thing a caller takes on is the terrain judgement. Suitable for upstreaming as a pure
    refactor, and the gradient rule is a candidate for replacing `MAX_CLIMB_HEIGHT_M` upstream
    too — the same resolution argument applies to any grid.


## Drift check (2026-10-02)

Upstream `dylmarriner/Maer-Ken` default-branch HEAD is `7c05f0dcf254387ffd7322dbb525fe4807228602`, equal to the pin: no upstream commits since the extraction, so upstream has not fixed the Phase 0 defects either. Re-check immediately before Phase 1.

**Owner decision needed:** keep the pin (program plan decision 2) or resync. Nothing is waiting on upstream today, so keeping the pin is the default.

Human schema or runtime changes should be compared against upstream first. Island-only rules belong in `apps/island_humans` unless the underlying human model itself is intentionally being changed.
