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
