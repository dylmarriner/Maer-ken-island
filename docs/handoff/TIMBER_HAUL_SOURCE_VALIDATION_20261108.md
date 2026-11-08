# Timber haul source-state validation — 8 November 2026

## Change
The public `haul_plans_over` boundary now checks the source log's physical invariants before any geometry, load calculation or cross-cut materialisation. Invalid logs return typed `LabourError::InvalidTimberLogState` rather than reaching the assertion in `Log::cross_cut`.

The regression covers zero dry mass, zero carbon, carbon greater than dry mass, and a non-finite position.

## Evidence
- PR: https://github.com/dylmarriner/Maer-ken-island/pull/128
- Source/test commit: `0684370c95d151d9d9324f1c45be2cef0f6d8d9e`
- Updated `WORK_REMAINING.md`: the D40 two-person saw workflow for ute log cross-cutting remains explicitly open.

## Verification
- Local `cargo fmt`, Clippy and Rust tests were not run: custom OpenCommander device discovery returned HTTP 429.
- Exact-head GitHub Actions is required for PR #128.
- No serialized action/snapshot shape, replay hash, RNG, material ledger, reference data or owner decision changed.

## Remaining
- Pass exact-head formatting, Clippy, fast Rust tests, desktop/backend and week/year reference-pack acceptance.
- D40 still needs a real two-person saw workflow for cross-cutting logs for the ute; this change does not claim that feature complete.
- Next Maer-Ken slice: inspect and implement the saw crew participation/energy accounting without treating the two-person productivity rate as one worker's rate.


## Restored-ledger materialisation regression — 8 November 2026
- Added `cross_cut_refuses_restored_invalid_source_without_consuming_reserved_ids_or_changing_stocks` in `crates/mk_engine/tests/island_material_flows.rs`.
- The test creates a valid source log and a two-ID reservation, corrupts the serialized snapshot so carbon exceeds dry mass, restores it, and verifies `cross_cut_log` refuses materialisation without changing the source, piece slots, carbon/water stocks, allocator watermark or reserved IDs.
- It then repairs only the snapshot's corrupted carbon field, restores the same ledger and proves the original reservation can still be consumed exactly once with binary64-exact dry-mass and carbon totals.
- Test commit: `60732ba2c5041662432d2c360d5834f9ca7e6bf8`.
- Previous exact-head workflow [#42448302191](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42448302191) passed formatting, Clippy, fast Rust tests, desktop/backend, and both week/year reference-pack acceptance jobs on the earlier head. A new exact-head workflow is required for this added regression.


## Exact-head formatting correction — 8 November 2026
- New workflow [#42452344302](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42452344302) found rustfmt-only differences in the added restored-ledger regression. The Format step failed before Clippy or fast tests ran.
- Applied the formatter's exact four layout changes in commit `9d75733bba9a99bf04ef5b45a8c12b1461f7b38d`.
- No logic was changed by this correction. A fresh exact-head workflow is required to verify formatting, Clippy, tests, desktop/backend and week/year acceptance.


## Snapshot repair test indexing refinement — 8 November 2026
- Changed the regression to retrieve the serialized source log through `Map::get_mut(&source_key)` before repairing the test fixture, avoiding reliance on a `String` index implementation for `serde_json::Value`.
- This is a test-code compatibility refinement only; no simulation or ledger behavior changed.
- Source and handoff are committed together in the current PR branch head.


## Exact-head restored-ledger regression correction — 8 November 2026

- Exact-head workflow [#42452538734](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42452538734) passed Rust formatting and Clippy, then fast tests found one failure in the newly added restored-ledger regression: 23 passed / 1 failed in `island_material_flows`.
- The failing assertion compared `MaterialLedger::stock_of(MaterialCarbon)` after deliberately corrupting a serialized log (carbon 11 kg) with the valid pre-corruption stock (5 kg). `stock_of` intentionally derives material carbon from the current log records, so the restored corrupted snapshot correctly reports 11 kg before any cross-cut operation. The assertion was testing the wrong baseline; no production cross-cut mutation was demonstrated.
- Corrected the regression to capture carbon and water stock immediately after restoring the deliberately corrupted snapshot, then prove the refused cross-cut leaves that restored state unchanged.
- After repairing only the corrupted carbon field, the test now captures the valid restored stock and verifies the successful cross-cut preserves carbon and water stock as well as exact binary64 dry-mass/carbon totals and the allocator watermark.
- Test correction commit: `dc965bd3e0c69c309986c21ef773cbccce7d968a`.
- A new exact-head workflow is required to verify the corrected test. The previous run's formatting, Clippy, desktop/backend, reference-pack and auxiliary checks passed, but the fast-test job failed on the incorrect assertion above.

## Remaining
- Pass exact-head fast Rust tests after the corrected stock baseline.
- Reconfirm formatting, Clippy, desktop/backend and week/year reference packs on the same head.
- Keep PR #128 draft until all required checks pass; D40 two-person saw participation remains open.


## Fail-fast validation on persisted ledger restore — 8 November 2026

- The first exact-head run after the restored-ledger regression, [#42452538734](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42452538734), passed Format and Clippy but found one incorrect test baseline: 23 tests passed and the new test failed because it compared `stock_of` after deliberate snapshot corruption with the valid pre-corruption stock. `stock_of` correctly includes carbon in the current log records.
- The follow-up now moves the invariant to the production restore boundary. `MaterialLedger` retains derived serialization but implements validated deserialization through a private snapshot type. Restore rejects a log whose map key is not its one-based ID or whose physical state fails `Log::has_valid_physical_state()`.
- The regression now proves three paths: malformed persisted log state fails during restore; an invalid in-memory log is refused by `cross_cut_log` without changing its source, stock, allocator or reservation; and the untouched valid snapshot restores the same reservation and cross-cuts with exact binary64 dry-mass/carbon conservation and unchanged carbon/water stock.
- Production deserializer commit: `4f86a959da89d2667be5a9a5d9624c8870ad5faa`.
- Regression commit: `45ee8145d18e1086cff7cc07674f8e7e24c281b4`.
- This preserves the existing serialized field names and legacy defaults for snapshots without logs/reservations. No action-log format, replay digest, RNG, reference data or live state was changed intentionally.
- A new exact-head workflow is required for this production restore invariant and regression.

## Remaining
- Verify exact-head formatting, Clippy, fast Rust tests, desktop/backend and week/year reference packs.
- Keep PR #128 draft until all required checks pass.
- Next independent feature slice remains D40: two-person saw cross-cut participation and both founders' time/energy accounting.


## Persisted log allocator validation — 8 November 2026

- The restored-ledger review found that physical log validation alone did not prove allocator consistency. A corrupt snapshot could set the exclusive next-log watermark at or below an existing log, reserve an id already occupied by a log, or reserve the watermark itself (an id not yet allocated).
- MaterialLedger deserialization now rejects all three states before constructing the live ledger. One-based ids, the exclusive allocator watermark, materialised logs and pending reservations are checked together.
- Regression coverage verifies stale watermarks, reservation/log collisions, reservations at or beyond the watermark, and compatibility with pre-log snapshots that omit the three newer fields.
- This does not change the append-only action log, replay digest, RNG, snapshot field names, valid reservation behaviour or material accounting.
- Exact-head workflow must verify formatting, Clippy, fast tests, desktop/backend and week/year reference packs. No pass is claimed for this new head.
- D40 two-person saw participation remains the next feature slice; this change is allocator/recovery hardening, not D40 completion.


## Allocator restore regression formatting correction — 8 November 2026

- Exact-head workflow #42456976777 on the d0f9f7a6 head failed only at cargo fmt --all -- --check.
- Rustfmt identified four layout-only differences in the new allocator regression: three wrapped error-string assertions and the legacy snapshot construction/object lookup.
- Applied the formatter's exact layout in commit 8a48006a751351cb29539d3bde34aceba3a02ec0.
- No allocator logic, snapshot compatibility behaviour or test assertions changed.
- Workflow #42457183791 was created for the corrected 8a48006 head and is in progress. A handoff-only commit advances the branch again and requires a fresh exact-head run. No fast-test or desktop pass is claimed yet.
