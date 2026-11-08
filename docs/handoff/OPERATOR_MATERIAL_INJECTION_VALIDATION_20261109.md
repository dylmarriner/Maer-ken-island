# Operator material injection validation — 9 November 2026

Repository: `dylmarriner/Maer-ken-island`
Branch: `fix/operator-material-injection-validation-20261109`
Base: `main` at `170e4b5f1c035bf71535294fa758bbd4b661a22b`

## Finding

`MaterialLedger::inject` was the remaining public material source-inflow boundary that silently clamped negative and non-finite quantities, then added directly to stock without checking whether the resulting inventory overflowed. A rejected or malformed operator intervention could therefore be represented as a successful zero/invalid inflow, or corrupt material stock and its conservation ledger.

## Change

- Change `inject` to return `Result<f64, MaterialError>`.
- Reject negative, NaN and infinite amounts with the existing typed `InvalidAmount` error.
- Reject invalid existing stock and non-finite resulting stock before mutation.
- Preflight the carbon conversion before stock or ledger mutation.
- Preserve the explicit `OperatorIntervention` boundary entries for material carbon and water.
- Propagate injection errors through the island's `InjectResource(Water)` intervention instead of applying `max(0.0)`.
- Update the existing overflow fixture to assert the valid seed injection succeeds.

## Regression coverage

- Negative, NaN, positive-infinity and negative-infinity inflows are rejected without changing stock or flux entries.
- A second finite maximum-sized water injection is rejected before stock or ledger mutation.
- Valid wood and water injections book carbon and water from `OperatorIntervention`, and the audit closes.

## Commits

- Production material boundary: `d101e85bbf7324eb0a335513afc393c0d14e9341`.
- Intervention refusal propagation: `7794e81aa40050254bfca4bad0a88867e01a3d1a`.
- Regression tests: `1a377f5f2ddcd903834feef75351dfa576d8245c`.
- Roadmap handoff: `1be4e45ee2bfbf791b855594b635afbeff668b4a`.

## Verification

- Exact-head GitHub Actions is required for Rust formatting, Clippy, fast tests, desktop/backend, and week/year reference-pack acceptance.
- Custom OpenCommander device discovery returned HTTP 429; no local Cargo test result is claimed.
- No simulation tick, snapshot schema, append-only log, replay hash, RNG, physical constant or reference pack changed.

## Next

- Verify exact-head CI and merge only after every required non-nightly job is green.
- Continue the D40 two-person saw-based log cross-cutting workflow and the D31 sourced regional soil-nutrient model.

## Producer-carbon field injection atomicity — 9 November 2026

A second operator inflow path bypassed `MaterialLedger::inject`: `InjectBiomass(Producers)` wrote directly into the regional biomass field and booked the total carbon only after looping through cells. It silently clamped negative values and did not preflight corrupt source cells or increments that round away.

### Production change

- Added typed `IslandInterventionError::InvalidProducerInjection`.
- Reject invalid amount, cell area, total area and per-cell allocation before mutation.
- Preflight every target land cell for finite, non-negative existing biomass.
- Reject non-finite results and positive increments that are not representable at a target cell.
- Collect all resulting cell values first; only mutate the grid and append the boundary flux after every target cell passes.
- Preserve sea exclusion and the existing `OperatorIntervention -> BiomassCarbon` flux.

### Regression coverage

- Negative, NaN and infinite amounts are rejected by the upstream validator without changing biomass, flux entries or replay log.
- Negative, NaN and infinite source biomass values are rejected with the typed island error.
- A finite `f64::MAX` source whose requested positive increment rounds away is rejected.
- Full-grid bit comparisons prove no earlier target cell is partially changed when a later target is invalid.
- Flux-ledger length and replay-log length remain unchanged on refusal.

### Commits

- Production: `1f088b8079f9f870fa7c59898ff618e2649b0313`.
- Regression tests: `3559e07ea702a389e9f69a4218429de10cddb564`.

### Verification

A fresh exact-head CI run is required after the production and test commits. No local Cargo result is claimed because custom OpenCommander access is unavailable. No replay schema, snapshot, RNG, tick cadence, physical constant or reference pack changed.

### Exact-head formatting correction — 9 November 2026

The first exact-head workflow after the producer-carbon implementation reached `cargo fmt --all -- --check` and failed only on four formatter layout differences in the new error display, preflight lookup, grid update and two regression assertions. No Clippy or Rust test result was reached on that head.

Applied the formatter's exact requested layout:

- Source formatting: `befca7646469af3cfbd5cd30f8c1a07f42778005`.
- Test formatting: `b4c12729e0343f71477d725acd886842196a9d54`.

Fresh exact-head workflow [#42487854063](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42487854063) was queued for the formatted head. This handoff update itself will create another head and requires its own exact-head CI. No Cargo test or Clippy pass is claimed for the producer-carbon slice yet.

## Rebase after PR #130 merge — 9 November 2026

PR #130 merged to `main` as `7dbcc2721b9103016b296f559e2a60147248bf81` after exact-head workflow [#42483265161](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42483265161) passed all required non-nightly checks. PR #131 had been based on the previous main head, so its branch was updated with a two-parent merge commit whose first parent is the prior PR #131 head and second parent is the new main commit. The merge tree preserves PR #130's biomass/soil-water source validation while applying PR #131's material-ledger and producer-carbon injection changes.

The current PR #131 branch therefore contains both histories without dropping either implementation. A fresh exact-head CI run is required on the new merge head. No local Cargo result is claimed.

### Producer summary formatting correction — 9 November 2026

Reviewing the merged tree exposed an escaped line-continuation typo in the new producer-injection summary: it would have emitted a literal backslash and newline. Corrected the summary to retain the original single-line display and added a regression assertion that the result contains no literal backslash and exactly one line.

- Production correction: `bfbebb3e53ede444aff454a7c7e9aa0efff17bf9`.
- Regression: `2bb6009157ad98f983f6b22bd45dea29f19cc4ac`.

This is included in the rebased PR #131 branch. Fresh exact-head CI is required after the follow-up handoff/work-register commits.

### Final assertion formatting correction — 9 November 2026

The first post-rebase workflow reached `cargo fmt --all -- --check` and found only two line-wrap differences in the new one-line-summary regression assertions. Applied the exact formatter layout in `eb4f83e8d8040a659dadbafdd831cffa99adbdc1`. No Clippy or fast-test result was reached on that head. The final handoff/work-register commit below triggers a fresh exact-head workflow.


## Clippy correction — 9 November 2026

Exact-head workflow [#42488473527](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42488473527) passed formatting and then failed Clippy because two overflow-test setup calls ignored the new `Result<f64, MaterialError>` returned by `MaterialLedger::inject`.

The fixtures now explicitly assert that the finite maximum-sized Wood and Water seed injections succeed and return the requested stock quantity. This preserves the intended overflow test setup while making the new fallible API contract explicit.

- Test correction: `08b9278ace557ccc622de60119f1209145ce263f`.
- No production behavior changed in this correction.
- Fresh exact-head CI is required; fast tests were skipped after Clippy failed on the previous head.


## Follow-up rustfmt correction — 9 November 2026

Fresh exact-head workflow [#42491631214](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42491631214) ran on the Clippy-fixture correction. The fast-check job stopped at formatting because rustfmt requires the two `materials.inject(...).expect(...)` chains to be wrapped across method-call lines. Applied the exact formatter layout:

- Formatting correction: `3987778c8dac1a8fdbe47d9aa39c3c58a1b91a91`.
- The previous Clippy error is not re-evaluated on that run because formatting failed first.
- Fresh exact-head CI must now pass formatting, Clippy and fast tests.
