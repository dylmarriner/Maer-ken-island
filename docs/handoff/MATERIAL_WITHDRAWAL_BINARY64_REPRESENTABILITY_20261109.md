# Material withdrawal binary64 representability — 9 November 2026

Repository: `dylmarriner/Maer-ken-island`
Branch: `fix/material-withdrawal-representability-20261109`
Base: PR #132 merge commit `e6304cc4423cd09e03b61922d61c8b31bfabaccd`

## Finding

`MaterialLedger::inject` now rejects a positive inflow that rounds away when added to a large existing stock. The withdrawal path had the symmetric hole: `take` accepted a finite positive amount even when `have - amount == have` in binary64. `craft` could then record the carbon transfer while leaving the source material inventory unchanged.

The recipe preflight also checked availability and output overflow, but did not check that each positive debit was representable before beginning the first mutation.

## Production change

- `MaterialLedger::take` now validates the existing stock and resulting stock before mutation.
- A positive withdrawal is rejected with typed `MaterialError::InvalidAmount` when the resulting stock is not strictly lower than the existing stock.
- `craft` preflights every required debit against a projected stock map before mutating any input.
- The same representability check is repeated at the private debit boundary, so non-craft callers cannot bypass it.
- Existing small absolute availability tolerance, output-overflow checks, ledger flow ordering and material composition remain unchanged.

## Regression

`material_withdrawals_reject_positive_amounts_rounded_away_before_mutation` seeds Wood stock to `f64::MAX`, attempts to craft with a 1 kg Wood input and no output, and requires typed rejection. It asserts unchanged Wood stock, MaterialCarbon stock and physical flux-entry count.

## Commits

- Production: `0386e035da5dbd27d65fa4cc2ec9e73a3675e4f2`.
- Regression: `067c254c627d90e2cb2529361f1cc26574791986`.

## Verification

A fresh exact-head workflow is required after the production and regression commits. The green PR #132 workflow [#42497043271](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42497043271) verifies only the preceding water-injection branch head, not this new withdrawal change. No local Cargo result is claimed because the authorised OpenCommander connector is returning HTTP 429.

## Safety and architecture

No snapshot schema, append-only replay format, replay hash, RNG, tick cadence, scientific constant, reference pack or live simulation state changed. The deterministic single-writer model is unchanged.
