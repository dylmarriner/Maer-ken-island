# Material amount validation and craft atomicity — 8 November 2026

Repository: `dylmarriner/Maer-ken-island`
Branch: `fix/material-amount-boundaries-20261108`
Base: `main` at merge commit `5ea337d5f5931f9349869b39f0f16e7edbcd31c6`

## Finding

The shared `MaterialLedger::take` boundary accepted negative and non-finite quantities. A negative burn, calcination, charcoal, eating, drinking or structure-build request could therefore reverse a debit, create inventory or move negative carbon through the flux ledger.

The crafting path also checked duplicate inputs independently. Two 5 kg inputs could each pass against 6 kg in stock, then the second debit fail after the first had already mutated the ledger. Negative/non-finite outputs were not rejected before the carbon balance calculation.

## Change

- Added typed `MaterialError::InvalidAmount` for negative and non-finite material quantities.
- Centralized the finite, non-negative check in `MaterialLedger::take`, protecting every withdrawal path that uses the shared boundary.
- `craft` now validates every input and output before mutation.
- Repeated input materials are aggregated into a deterministic `BTreeMap` and availability is checked against the total requirement before any debit.
- Craft debits occur only after all material requirements pass, preventing partial inventory mutation on duplicate-input shortages.

## Regression coverage

- Negative burn and eating quantities are rejected.
- NaN calcination quantities are rejected.
- Positive-infinity charcoal and negative-infinity drinking quantities are rejected.
- Negative structure builds are rejected.
- Rejections preserve stock, body carbon/water, structure count and flux-ledger entries.
- Duplicate 5 kg gold inputs against 6 kg inventory return one 10 kg insufficiency error without consuming any stock.
- Negative craft outputs are rejected without debiting inputs.

## Commits

- Production guard and deterministic aggregate preflight: `b3f644d87741fb9503ce897589c9d87a151ff844`.
- Regression coverage: `2dad19f75318ee37195bbf8f62e6e96abe7edd02`.

## Compatibility and determinism

- No snapshot schema, version, action log, replay hash, RNG, tick cadence, reference pack or real-world parameter changed.
- Valid zero and positive quantities retain their existing semantics.
- The aggregation uses the existing ordered `BTreeMap` key type, keeping the preflight deterministic.
- No external provider, payment, production service or live database was touched.

## Verification

- Custom OpenCommander device discovery returned HTTP 429; local Cargo commands could not be run.
- Exact-head GitHub Actions is required for formatting, Clippy, fast Rust tests, desktop/backend and week/year reference-pack acceptance.
- No test pass is claimed until that workflow completes.

## Next

- Verify this exact branch head in CI.
- Continue D40's two-person saw cross-cut workflow with both founders' time and energy accounted separately.
- Continue D31 only with sourced regional nitrogen/phosphorus data and explicit deviations.

## Exact-head formatting correction — 8 November 2026

- Workflow [#42464939304](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42464939304) checked out merge ref `d6c9ad45d27d8ac16de501dacd4cb86a280ce8a0` and failed only at `cargo fmt --all -- --check`.
- Rustfmt requested the multi-line layout for `MaterialError::InvalidAmount`; no semantic or test failure was reached.
- Applied the exact formatter layout in commit `fb86622003566e8857dc19c4918f9da61d4d6e55`.
- The corrected branch head requires a fresh exact-head workflow. Fast tests, Clippy, desktop/backend and reference-pack results are not yet available for this slice.
