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

The no-Result material-ingestion paths are deliberately tracked separately in PR #135 (`receive_felled_wood`, `receive_felled_log`, `capture_roof_water`, `season_stored_logs`). PR #134 does not claim those paths complete and no longer carries duplicate roof-water code/tests.

## Next

1. Verify PR #134 exact head.
2. Continue D40: cross-cut felled logs for ute transport using the two-person saw, with both founders' labour/energy accounted for and route modelling.
3. Continue D31 regional soil-nutrient modelling.


## Exact-head formatting correction — 9 November 2026

Workflow [#42517525917](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42517525917) ran on the prior PR head `1ddc817030bcc2a966bfcf99d7f1b89df7fb5764`. `cargo fmt --all -- --check` found two test-file formatting differences: one extra blank line before the appended cases and one long edible-cell predicate. Clippy and fast tests were skipped by the format gate; no compile/test result is claimed from that job.

Both findings were corrected in `f408d171e7e1af4239aa6ff170996ce52d2fe98d`. A fresh exact-head workflow is required. The same old-head run had computer service, repository scripts, web dashboard and dependency audit passing; installed backend, desktop application and week/year reference packs were still running at the last inspection. Those results do not verify the corrected head.


## PR #134 Clippy correction — 9 November 2026

- Exact-head workflow [#42517618293](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42517618293) on head `f9a493610a138515fc8b6d5569ff4f537594424e` passed formatting, then Clippy rejected the test's `Option::map_or(false, ...)` as `unnecessary_map_or`. Fast tests were skipped by the Clippy gate.
- Corrected the predicate to `is_some_and` in `ac50da421a9f7acbaddee2c867e7f3efbfc1a3ae`.
- On that old-head run, dependency audit, repository scripts, web dashboard and computer service passed; installed backend, desktop and week/year reference packs were still running at last inspection. Those results do not verify the corrected head.
- Fresh exact-head CI must target the current PR head. A run on `ac50da42` or any earlier commit does not verify later changes.


## Week reference-pack drift investigation — 9 November 2026

- Exact-head workflow [#42517754858](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42517754858) on `eda79a8e1a56b910cb04d62a71d93426421e44f2` passed formatting, Clippy, fast Rust tests, desktop application, installed backend, computer service, dashboard, repository scripts and dependency audit.
- The required week reference-pack failed: actual digest `a3b31f72ba1a20f3b4232988c2ce350d3f04eb1e625100fe53fead3a55440719`, expected `29ed53bd7769eb3025e70135bab8b7ea07718e26e7fca4cbf77ab379d62b7784`. The test reported `shortfalls food 1 water 0`; the year reference pack did not run after the week failure.
- Do not update the expected digest until the state divergence is explained. The run is not acceptable evidence for merge.
- Temporary diagnostic logging was added in `life.rs` at commit `69ff4411359f625f09290311866c71f4105ec993` to distinguish no-edible-cell, gather and eat failures in the week simulation. Remove the diagnostic after identifying the cause.
- Exact-head diagnostic workflow [#42522720849](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42522720849) is in progress. Its result must be inspected before changing simulation logic or the reference pack.

## Week digest audit and re-pin — 9 November 2026

The diagnostic/epsilon follow-up changed the week test's canonical digest. Exact-head workflow [#42523987454](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42523987454) reported:

- Actual digest: `61ffaa6a559b5a8f503b7b6c707ca5f8cf634be9eebec64a1af6652706c2478a`.
- Previous expected digest: `29ed53bd7769eb3025e70135bab8b7ea07718e26e7fca4cbf77ab379d62b7784`.
- The week run's physical assertions all passed: 27,651 ledger audits, 167 hourly water audits, zero food and water shortfalls, 111 economy events, 200,000 estate-patch trees, Gem-D body carbon 13.0/13.0 kg, Gem-K 10.4/10.4 kg, and temperate-forest NPP 471 g C/m2/yr within the 400-900 reference band.
- Sleep, eating, glycogen and water-intake checks also passed; the independent year-weather reference-pack test passed.
- The only failed assertion was the previous digest pin.

The digest moved because the completed-meal path now canonicalises a residual `respired_since_meal` balance at or below 1e-12 kg C to exactly zero. The old positive binary64 remainder could not produce a representable physical transfer. No scientific constant, reference-pack bound, RNG, tick cadence, snapshot schema or replay format was changed.

The week digest is now re-pinned to the audited actual value in test commit `180d1c8f48247236511cf75b9cc04b2f5f4a1a1a`. The audit comment records the measured physical outputs. A fresh exact-head CI run on the new PR head is mandatory; workflow #42523987454 itself remains failed because it ran against the old pin. Do not merge PR #134 until the new week and year acceptance jobs, fast tests and all required checks pass.

## Scope split — 9 November 2026

A separate draft PR, [#135 — Preflight timber and roof-water material ingestion](https://github.com/dylmarriner/Maer-ken-island/pull/135), was discovered during the current repository-wide PR audit. It already owns the no-Result ingestion paths and contains the more complete aggregate-audit preflights for felled wood/logs, roof-water capture and seasoning. To avoid duplicate implementations, PR #134 has been narrowed back to resource inflow/conversion precision and the audited week-digest pin.

- Removed the overlapping roof-water implementation from PR #134 in commit `eaddfc8a0103795abe5808f0b9bf9906a4d8e8a0`.
- Removed the duplicate roof-water regression from PR #134 in commit `9a79154a5f9d4f3f93908ae21d29ee9b436d2205`.
- PR #135 remains the owner of roof-water source/destination representability and the remaining timber ingestion audit. Its current head has an additional source-soil debit regression; verify its own exact-head CI before merge.
- The two PRs remain separate and unmerged. No snapshot schema, replay format, RNG, cadence, scientific constant or reference pack was changed by this scope split.


## Exact-head documentation-gate correction — 9 November 2026

Exact-head workflow [#42528864382](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42528864382) passed formatting and Clippy, and its desktop/backend, dashboard, computer-service, repository-script, dependency-audit and week/year reference-pack jobs passed. Fast Rust tests failed only at `active_week_digest_documentation_matches_the_acceptance_pin`: the acceptance test's audited `WEEK_DIGEST` had been updated to `61ffaa6a559b5a8f503b7b6c707ca5f8cf634be9eebec64a1af6652706c2478a`, but the three published guidance files still contained the prior pin.

- Updated `docs/TESTING.md`, `docs/handoff/STATE_OF_THE_ISLAND.md` and `HANDOFF.md` to publish the audited current pin.
- No simulation code, snapshot format, append-only log, RNG, tick cadence or reference bounds changed in this correction.
- Fresh exact-head fast tests and full required acceptance are still required; no pass is claimed for the corrected head.
