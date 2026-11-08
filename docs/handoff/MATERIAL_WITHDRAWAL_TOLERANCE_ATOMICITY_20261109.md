# Material withdrawal tolerance and atomicity — 9 November 2026

Repository: \`dylmarriner/Maer-ken-island\`
Branch: \`fix/material-withdrawal-representability-20261109\`
PR: [#133](https://github.com/dylmarriner/Maer-ken-island/pull/133)

## Finding

The previous withdrawal guard retained a \`1e-12 kg\` availability tolerance and then clamped the debit with \`kg.min(have)\`. A positive request slightly larger than a positive stock could therefore succeed while removing less than the amount that callers transferred to a body, soil store or flux ledger.

The same tolerance in \`craft\` preflight could allow the recipe to pass its all-input check, so a later debit could be short after earlier inputs had already been removed.

## Production change

- Added one shared \`checked_withdrawal_result\` boundary.
- Requires exact available stock (\`have >= requested\`) before every debit.
- Rejects positive withdrawals that round away.
- Uses the same validation in \`take\` and \`craft\` preflight.
- Writes the precomputed resulting stock only after validation.
- Preserves deterministic material ordering, ledger order, replay format and single-writer ownership.

## Regression coverage

- Direct Water withdrawal: stock 1e-13 kg, request 2e-13 kg. Requires typed \`Insufficient\` and unchanged material/body water.
- Multi-input craft: valid 10 kg Wood stock is ordered before Water stock 1e-13 kg, while the recipe requests 2e-13 kg Water. Requires rejection before Wood, Water, carbon, water or flux entries change.

## Commits

- Production: see the \`fix(materials): reject tolerance-based partial withdrawals\` commit on PR #133.
- Regression: see the \`test(materials): cover tolerance shortfall and craft atomicity\` commit on PR #133.

## Verification

Exact-head PR CI is required: \`cargo fmt --all -- --check\`, Clippy, fast Rust tests, desktop/backend acceptance and week/year reference packs. No local Cargo pass is claimed; the authorised custom OpenCommander device connector returned HTTP 429 during this run.

## Invariants

No snapshot schema, append-only replay format, replay hash, RNG, tick cadence, scientific constant, reference pack or live simulation state changed.
