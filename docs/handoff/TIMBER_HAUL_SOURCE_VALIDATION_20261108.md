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
