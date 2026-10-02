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
3. `mk_engine::io::encryption` — `parse_storage_key` validates length and ASCII hex instead of panicking; the key file is created atomically with mode `0600`; `EncryptionError` implements `Display`/`Error`. `HumanStorage::try_new`/`try_with_key_hex` propagate key errors, and `HumanStorage::new` logs instead of silently downgrading. `World::enable_persistent_humans` uses `try_new`.
4. `mk_engine::humans::computer_bridge` — retries go through `retry_with` and `RetryPolicy`. `SendEmail` is never retried on a timeout, and the final attempt's real error is returned (it used to be reported as `RateLimitError`).
5. `mk_engine::humans::registry` — who exists no longer depends on the disk. `add_human`, `create_human` and `create_named_human` reject duplicate ids (`HumanStorageError::DuplicateAgent`) before inserting, otherwise insert first and then write the folder. `create_human`/`create_named_human` now return `Result<(), _>`. `seed_founders`, `deliver_due_births`, `spawn_human` and `enable_persistent_humans` keep humans when a folder write fails; `sync_to_storage`, `sync_human_to_storage` and `record_event` return their errors.
6. `mk_engine::humans::registry` — an agent-id index (derived, never serialized) makes id lookups O(1); iteration order is unchanged; `get_all_humans_mut` returns a slice.
7. `mk_engine::humans::HumanSystem::step_dialogue` — pairing is O(n log n) and realistic: each human is in at most one conversation per step, partners are chosen by relationship, mutual approach and distance. Behaviour change: a child is no longer in conversations with both parents in the same step. Conversations still last one step; multi-step conversations are not modelled.
8. Clippy cleanups: removed the unused `HYDROLOGY_SPIN_UP_STEPS`/`HYDROLOGY_SPIN_UP_STEP_SECONDS` constants and `BrainRegionsSnapshot::resting_mean_activation`, and used `RangeBounds::contains` in `orbit`.

## Drift check (2026-10-02)

Upstream `dylmarriner/Maer-Ken` default-branch HEAD is `7c05f0dcf254387ffd7322dbb525fe4807228602`, equal to the pin: no upstream commits since the extraction, so upstream has not fixed the Phase 0 defects either. Re-check immediately before Phase 1.

**Owner decision needed:** keep the pin (program plan decision 2) or resync. Nothing is waiting on upstream today, so keeping the pin is the default.

Human schema or runtime changes should be compared against upstream first. Island-only rules belong in `apps/island_humans` unless the underlying human model itself is intentionally being changed.
