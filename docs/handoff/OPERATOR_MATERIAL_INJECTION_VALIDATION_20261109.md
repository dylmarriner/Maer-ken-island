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
