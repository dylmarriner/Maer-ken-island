# Seasoning compensated-sum sign correction

Date: 2026-11-09
Repository: `dylmarriner/Maer-ken-island`
Branch: `fix/material-ingestion-atomicity-20261109`
PR: [#135](https://github.com/dylmarriner/Maer-ken-island/pull/135)

## Defect

`season_stored_logs` uses Neumaier compensated summation to aggregate the wood mass and water in a deterministic batch of stored logs. The correction term is signed. The previous representability assertion incorrectly required the corrected sum to be greater than the naive sum whenever the correction was nonzero.

A valid batch can produce a negative correction. Adding `3 * 2^-53` to a running total of 1.0 kg rounds the running sum up by two ULPs and records a negative half-ULP correction. The final correctly rounded binary64 result can equal the naive running sum. That is not a lost transfer or an invalid batch, but the old assertion panicked before mutation.

The same sign error existed in the water batch assertion.

## Implementation

- Require the corrected wood total to be finite and positive.
- Require the corrected water total to be finite and non-negative.
- Do not treat the sign of the Neumaier correction as an error.
- Preserve existing per-material stock-addition preflight, material-carbon audit, water-flux reconciliation, deterministic log ordering and all-before-mutation validation.

## Regression coverage

- Carbon batch with a negative Neumaier correction; seasoning succeeds, produces the correctly rounded wood total, removes the stored logs and closes the material-carbon audit.
- Water batch with a negative Neumaier correction; source soil debits remain representable, seasoning succeeds, removes stored logs and closes the material-water audit.

## Verification

- Source and regression changes are committed on PR #135.
- Exact-head Rust formatting, Clippy, fast tests, desktop/backend and week/year reference-pack acceptance are required.
- No local Cargo result is claimed because the authorised custom OpenCommander endpoint returned HTTP 429.
- Previous PR #135 workflow [#42537458249](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42537458249) verifies the predecessor code head only. PR #134 workflow [#42543151327](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42543151327) belongs to a different branch and does not verify this change.

## Architecture and safety

No simulation tick, RNG, snapshot schema, append-only flux format, replay hash, canonical scientific constant, reference dataset, LLM behaviour or live simulation state was changed. No deployment or external service was modified.

## Next

1. Verify this exact PR #135 head through the full required CI matrix.
2. Reconcile PR #134 and #135 only after both branches' code and tests are reviewed together; do not merge overlapping material-ledger changes blindly.
3. Continue D40 transport cross-cut and ute route/energy accounting.
## Formatting follow-up — 9 November 2026

- Exact-head workflow [#42545264928](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42545264928) checked out commit `2de5d6c` and stopped at `cargo fmt --all -- --check`; rustfmt reported only the extra blank line before the new regression tests and the missing final newline.
- Corrected both formatting findings. The workflow's other jobs are independent and continue; this formatting failure is not a test assertion result.
- A fresh exact-head CI run is required on the formatting-corrected commit. No local Cargo test is claimed while OpenCommander remains rate-limited.
