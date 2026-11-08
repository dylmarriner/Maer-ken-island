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
