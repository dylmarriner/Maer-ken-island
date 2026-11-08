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

- Added a successful duplicate-input recipe case as well: two gold inputs totalling 5 kg against 6 kg stock are aggregated, debited and re-added as 5 kg output with the final stock unchanged. This complements the insufficient-stock no-partial-mutation case.

- Extended the same finite/non-negative contract to `bank_mined`; negative and positive-infinity mined quantities are now rejected instead of being clamped or entering inventory. The regression verifies the existing 10 kg gold stock remains unchanged.
- Call-site search found the simulation's only `bank_mined` caller in the gold-mining completion path, which supplies the non-negative `work.product_kg` output; no caller relies on negative clamping.


## Finite resulting-stock preflight — 8 November 2026

### Finding
Finite, non-negative input amounts are not sufficient to guarantee a finite inventory result. Adding a finite amount to an already-large finite stock can overflow to infinity. Crafting could therefore pass amount validation and still write a non-finite stock value; duplicate outputs could also overflow after earlier output additions.

### Change
- `bank_mined` now checks the resulting stock total before mutation.
- `craft` projects post-debit stock and every output addition in the original output order before the first debit.
- Any non-finite resulting stock returns the existing typed `InvalidAmount` error without changing inventory or the flux ledger.
- Valid recipe ordering, snapshot schema, replay hashes and normal stock arithmetic are unchanged.

### Regression coverage
- A second `f64::MAX` mined deposit is rejected while preserving the first deposit.
- Crafting `f64::MAX` onto an existing `f64::MAX` stock is rejected atomically.
- Two individually finite duplicate outputs that overflow their combined stock are rejected before any stock write.

### Verification
- Added to PR #129 after review of its first green exact-head run.
- A fresh exact-head CI run is required for the updated branch head; no local Cargo result is claimed because custom OpenCommander is still returning HTTP 429.


## Rustfmt correction — 8 November 2026

- Exact-head workflow [#42469507804](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42469507804) reached `cargo fmt --all -- --check` and found one formatting-only difference in the new stock-overflow regression.
- Applied the formatter's exact single-line call layout. The run did not reach Clippy or fast tests; those checks remain unverified for this head.
- A fresh exact-head workflow is required after this correction.
