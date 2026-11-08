# Material stock inflow precision — 9 November 2026

## Scope

Repository: `dylmarriner/Maer-ken-island`  
Branch: `fix/material-inflow-conservation-20261109`  
Next PR: #134

## Previous verified work

PR #133, binary64 withdrawal representability, merged to `main` as `6b57ad96273c2320910f93902963bc8800974175`.

Exact-head workflow [#42512313757](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42512313757) passed:
- Rust formatting and Clippy.
- Fast Rust tests.
- Desktop build, tests, render and input acceptance.
- Installed backend.
- Computer service.
- Web dashboard.
- Repository scripts.
- Dependency audit.
- Week and year reference-pack acceptance.

Nightly slow tests were skipped by design.

## Defect

The prior material-ledger hardening covered injection, mineral banking, craft input/output aggregation and withdrawals. Several other inflow paths still checked only for infinity. A finite positive amount could be added to a much larger finite stock and round away, while the source reservoir, animal population or conversion input was still debited.

## Production changes

Reuse `MaterialLedger::checked_stock_addition_result` before source mutation in:
- biotic gathering;
- edible-food gathering;
- soil-water gathering;
- surface-water gathering;
- fossil extraction;
- hunting.

Conversion paths now preflight both sides before mutation:
- calcining checks the limestone withdrawal and resulting Quicklime stock before debiting limestone;
- charcoal making checks the Wood withdrawal and resulting Charcoal stock before debiting Wood.

Successful additions insert the exact preflighted resulting stock rather than recomputing the addition after mutation.

## Regression coverage

Seven new tests:
1. Fossil extraction at maximum finite Coal stock.
2. Biotic gathering at maximum finite Wood stock.
3. Edible gathering at maximum finite PlantFood stock.
4. Soil-water gathering at maximum finite Water stock.
5. Surface-water gathering at maximum finite Water stock.
6. Calcining when Quicklime output rounds away.
7. Charcoal conversion when Charcoal output rounds away.
8. Hunting when Meat output rounds away.

The cases assert that rejected operations leave source state/population, destination inventory and flux-entry count unchanged.

## Commits

- Production: `fc2d5cdeba8bb59eaa89014c94107906d993f08b`.
- Conversion input preflight: `b20a5d7ac3b1d126ca8d1b717ec4a3a1af961d7f`.
- Regression suite: `8cab8693afd2587ad19aac05df6637a1e856b33d`.
- Test fixture correction: `8002856ad6dd3472b4e45b5e1395b098a9b8e5af`.
- Work register: `00e866054d473457d559fb55f96a8b4de7ba8b7c`.

## Verification

PR #134 exact-head CI is required for formatting, Clippy, fast Rust tests, desktop/backend acceptance, dependency audit and week/year reference packs. No local Cargo pass is claimed because custom OpenCommander returned HTTP 429.

## Invariants and limits

No snapshot schema, append-only replay format, replay hash, RNG, tick cadence, scientific constant, reference pack or live simulation state changed. Deterministic single-writer ownership is preserved.

The no-Result ingestion paths (`receive_felled_wood`, `receive_felled_log`, `capture_roof_water`, `season_stored_logs`) remain a separate audit item; this PR does not claim every stock inflow path is complete.

## Next

1. Verify PR #134 exact head.
2. Continue D40: cross-cut felled logs for ute transport using the two-person saw, with both founders' labour/energy accounted for and route modelling.
3. Continue D31 regional soil-nutrient modelling.


## Exact-head formatting correction — 9 November 2026

Workflow [#42517525917](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42517525917) ran on the prior PR head `1ddc817030bcc2a966bfcf99d7f1b89df7fb5764`. `cargo fmt --all -- --check` found two test-file formatting differences: one extra blank line before the appended cases and one long edible-cell predicate. Clippy and fast tests were skipped by the format gate; no compile/test result is claimed from that job.

Both findings were corrected in `f408d171e7e1af4239aa6ff170996ce52d2fe98d`. A fresh exact-head workflow is required. The same old-head run had computer service, repository scripts, web dashboard and dependency audit passing; installed backend, desktop application and week/year reference packs were still running at the last inspection. Those results do not verify the corrected head.
