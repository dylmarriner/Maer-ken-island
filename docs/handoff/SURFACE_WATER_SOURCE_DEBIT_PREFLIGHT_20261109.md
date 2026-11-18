# Surface-water gathering debit preflight — 9 November 2026

## Scope
The material-ingestion atomicity work already preflights material-stock additions for roof-water capture, felled wood/log receipt and log seasoning. A separate standing-water/river gathering path still checked only that the destination stock stayed finite. It did not prove that the source reservoir actually decreased after binary64 subtraction.

A positive 1 kg transfer can disappear when subtracted from a very large finite surface-water depth. The old path could then add water to MaterialWater and record a flux while leaving the source unchanged.

## Implementation
- Added `checked_source_debit_result`, which validates finite non-negative source state, exact availability, and representable positive source reduction.
- `gather_water` now preflights standing-water debit, river-runoff debit, and destination MaterialWater stock before mutating either hydrology grid, material stock or flux ledger.
- Rejects a positive source amount whose depth conversion underflows to zero.
- Rejects an invalid/underflowed river area factor before division.
- Inserts the exact preflighted destination stock rather than recomputing an addition after source mutation.

## Regression coverage
Added `surface_water_gathering_rejects_unrepresentable_source_and_stock_debits_before_mutation`:
1. Seeds a (2^{53}) surface-water depth and attempts a 1 kg draw; verifies the source-debit rejection leaves surface water, runoff, material stock and flux count unchanged.
2. Seeds (2^{53}) kg MaterialWater and offers a normal 1 kg surface-water draw; verifies destination-stock rejection leaves both source grids, destination stock and flux count unchanged.

## Commits
- Production: `89b047709788eedbe2546c858d992837b1284ede`
- Regression: `2ffe373efadee8fcd91d2f533d2add892ccad867`

## Verification
- GitHub Actions is expected to run on the updated PR #135 head.
- No local Cargo test or Clippy result is claimed because custom OpenCommander device discovery is returning HTTP 429 in this run.
- Prior PR #135 exact-head workflow #42532858642 had dashboard, computer service, installed backend, dependency audit, repository scripts and week/year reference packs green; formatting/Clippy/fast tests and desktop acceptance were still in progress at inspection. Those results are for the prior head and do not certify these commits.

## Safety and architecture
No simulation state, real-world constants, replay formats, snapshots, LLM behaviour, production infrastructure or SCOS state changed. The implementation preserves the deterministic single-writer material ledger and append-only flux accounting.

## Next
Run exact-head formatting, Clippy, fast Rust tests, desktop/backend acceptance and week/year reference packs. Keep PR #135 draft until all required jobs pass.

## Exact-head CI follow-up

The first workflow for the implementation head `2bafa3ef61d1a8f61ddf669a11f40623fac9556a` reached `cargo fmt --all -- --check` and identified two formatting-only differences in `materials.rs`. No Clippy or fast Rust tests ran because formatting is the first gate. Corrected both exact rustfmt diffs in commit `0531b7cc1ff653058e9e28def4d32a2f5f31404e`. A fresh exact-head CI run is required; no formatting or test pass is claimed for the corrected head yet.

## Follow-up — felled-log soil source representability

A neighbouring timber-ingestion path had the same source-debit defect: `receive_felled_log` checked that the post-removal soil-water value was finite, but did not prove that a positive water transfer actually reduced the source. At a sufficiently large finite soil-water depth, subtracting the log's converted water depth can round back to the unchanged value while the material stock and flux ledger still receive the water.

- Replaced the finite-only result check with the shared `checked_source_debit_result` preflight.
- The exact source debit is now validated before log-ID allocation, hydrology mutation, stock/ledger changes or insertion.
- Added `receiving_felled_log_rejects_unrepresentable_soil_debit_before_allocating_or_booking_fluxes`, using a (2^{53}) mm source depth and a positive 8.2 kg log-water transfer.
- The regression asserts unchanged soil storage, carbon/water stocks, next log ID, log collection and flux-entry count.
- Production commit: `8df094d717fabe6e4c6d1bb913a8e20a2654902f`.
- Regression commit: `72eb626e523a163fa343640235b64e2322c3527d`.
- Exact-head CI is required. No local Cargo result is claimed because the authorised custom OpenCommander endpoint is currently rate-limited.

## Exact-head formatting follow-up — 9 November 2026

- Workflow [#42536830786](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42536830786) checked out the prior PR merge head and stopped at `cargo fmt --all -- --check`. The only reported diff was rustfmt layout of the new source-debit assignment; Clippy and fast tests did not run on that head.
- Corrected the formatting in commit `1b620f68811f1d5f7988ac677474c2b8143a5704`.
- New exact-head workflow [#42536914812](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42536914812) is running. Dependency audit, repository scripts, web dashboard and computer service have passed; formatting/Clippy/fast tests, desktop, installed backend and week/year reference packs remain in progress at the latest inspection.
- PR #135 remains draft. No full Rust test or desktop/backend pass is claimed yet.

## Felled-log source-depth conversion underflow follow-up — 9 November 2026

A second edge case remained after the positive source-debit check: a positive but subnormal `from_soil` quantity can underflow to exactly zero when converted from kilograms to millimetres (`from_soil / cell_area_m2`). The shared debit helper sees a zero amount and cannot distinguish that lost positive transfer.

- Added an explicit invariant: if `from_soil > 0`, the converted `removed_mm` must also be positive before the source-debit helper is called.
- Added `receiving_felled_log_rejects_soil_depth_underflow_before_allocating_or_booking_fluxes`, using the smallest positive binary64 subnormal water quantity and a cell area greater than 1 m².
- The test asserts unchanged soil storage, carbon/water stocks, next log ID, log collection and flux-entry count.
- Production commit: `0acf469543f519cf8c2f616eda5a3f8341bac8f8`.
- Regression commit: `ec352e3e84cef47f522429fa6cb013a680743896`.
- Exact-head CI must run on the final PR head; no local Cargo result is claimed while custom OpenCommander is rate-limited.

## Current exact-head evidence — 9 November 2026

- Final source guard: `0acf469543f519cf8c2f616eda5a3f8341bac8f8`.
- Underflow regression: `ec352e3e84cef47f522429fa6cb013a680743896`.
- Regression precondition proving the positive subnormal amount is available before unit conversion: `b767943c4718c2100caec6f2302e022baba9680e`.
- Current exact-code workflow: [#42537458249](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42537458249).
  - Rust formatting: passed.
  - Clippy: passed.
  - Fast Rust tests: running.
  - Desktop application, installed backend and week/year reference packs: running.
  - Dependency audit, repository scripts, web dashboard and computer service: passed.
  - Nightly slow tests: skipped by design.
- No full Rust test, desktop/backend or reference-pack pass is claimed until those jobs complete.
