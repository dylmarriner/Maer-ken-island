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
3. `mk_engine::regional::boundary` — `sample_regional_boundaries_with_background` (`provisional: false`) takes ocean and atmosphere edges from the background's bands; the Phase 1 sampler is kept for geophysics and now shares its loop.

## Drift check (2026-10-02)

Upstream `dylmarriner/Maer-Ken` default-branch HEAD is `7c05f0dcf254387ffd7322dbb525fe4807228602`, equal to the pin: no upstream commits since the extraction, so upstream has not fixed the Phase 0 defects either. Re-check immediately before Phase 1.

**Owner decision needed:** keep the pin (program plan decision 2) or resync. Nothing is waiting on upstream today, so keeping the pin is the default.

Human schema or runtime changes should be compared against upstream first. Island-only rules belong in `apps/island_humans` unless the underlying human model itself is intentionally being changed.
