# Material withdrawal tolerance and atomicity — 9 November 2026

Repository: dylmarriner/Maer-ken-island
Branch: fix/material-withdrawal-representability-20261109
PR: https://github.com/dylmarriner/Maer-ken-island/pull/133

## Finding

The previous withdrawal guard retained a 1e-12 kg availability tolerance and then clamped the debit with kg.min(have). A positive request slightly larger than a positive stock could therefore succeed while removing less than the amount that callers transferred to a body, soil store or flux ledger.

The same tolerance in craft preflight could allow the recipe to pass its all-input check, so a later debit could be short after earlier inputs had already been removed.

## Production change

- Added one shared checked_withdrawal_result boundary.
- Requires exact available stock (have >= requested) before every debit.
- Rejects positive withdrawals that round away.
- Uses the same validation in take and craft preflight.
- Writes the precomputed resulting stock only after validation.
- Preserves deterministic material ordering, ledger order, replay format and single-writer ownership.

## Regression coverage

- Direct Water withdrawal: stock 1e-13 kg, request 2e-13 kg. Requires typed Insufficient and unchanged material/body water.
- Multi-input craft: valid 10 kg Wood stock is ordered before Water stock 1e-13 kg, while the recipe requests 2e-13 kg Water. Requires rejection before Wood, Water, carbon, water or flux entries change.

## Commits

- Production: 05de1c58ded881acf2912217edfd4852c392fb84.
- Regression: 6776ed061bd2d3881a3c99154ca3e1af484b1c23.
- Handoff file creation: 6763658a89cb037b9cb5c3712cfa52fcf3e34066.
- Work register: f0a179bf166086ed8127f7a61fb79e022c3c094a.

## Verification

Exact-head PR CI is required: cargo fmt --all -- --check, Clippy, fast Rust tests, desktop/backend acceptance and week/year reference packs. No local Cargo pass is claimed; the authorised custom OpenCommander device connector returned HTTP 429 during this run.

## Invariants

No snapshot schema, append-only replay format, replay hash, RNG, tick cadence, scientific constant, reference pack or live simulation state changed.


## Exact-head CI finding and regression correction — 9 November 2026

Workflow [#42507201745](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42507201745) passed formatting and Clippy. The fast Rust suite ran 42 tests in island_material_flows: 41 passed and one failed because the new zero-stock test expected InvalidAmount. The production path correctly returns Insufficient { have_kg: 0.0, need_kg: 1e-13 } at the exact-availability check before subtraction; the regression expectation was wrong, not the withdrawal guard. The test now asserts the precise typed Insufficient result and still checks unchanged stock, carbon and flux entries. A fresh exact-head workflow is required; the earlier run does not verify this correction.


## Material inflow representability and duplicate-output aggregation — 9 November 2026

A second binary64 conservation boundary was found in stock additions. A positive finite inflow could round away against an existing large stock. Operator injection already rejected that case, but mineral banking and craft outputs did not. Craft also applied duplicate outputs sequentially, so two individually rounded-away increments could be lost even when their combined amount was representable.

- Added shared checked_stock_addition_result validation for exact stock inflows and applied it to bank_mined and craft output preflight.
- Craft now aggregates duplicate outputs in deterministic BTreeMap order, validates the aggregate before any debit, computes carbon from the actual aggregated inputs/outputs, and installs the preflighted final stock values.
- Added regressions for a 1 kg mineral deposit rounded away at f64::MAX, a craft output rounded away before a valid Wood input is debited, and two 1 kg outputs at 2^53 kg that must aggregate into one representable 2 kg increment.
- The zero-stock withdrawal regression now expects the correct typed Insufficient error (have=0, need=1e-13); the prior CI failure was a test expectation defect, not a production withdrawal failure.
- Fresh exact-head CI is required for this expanded slice. No local Cargo result is claimed.


## Duplicate-input aggregation representability — 9 November 2026

- The same binary64 loss could occur while aggregating repeated craft inputs: adding a 1 kg requirement to a 2^53 kg requirement rounds the aggregate back to 2^53 kg.
- Input aggregation now rejects a positive component when the aggregate does not increase, before any stock debit. Added a regression that requests 2^53 kg plus 1 kg of Gold and verifies typed refusal with unchanged stock and ledger.
