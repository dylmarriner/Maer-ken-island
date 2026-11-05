# Petroleum finite-interval and drilling-input guards

Date: 2026-11-05
Repository: `dylmarriner/Maer-ken-island`
Branch: `fix/petroleum-finite-interval-guards-20261105`
Base: `07ad027532901b304dd339548d270e10e121a0a6` (after refinery assay validation merge)

## Defects

`PetroleumField::produce_up_to` rejected NaN and non-positive elapsed time but
accepted positive infinity. The exponential decline formula then treated an
infinite interval as a request to produce the entire remaining reserve in one
step. Its public `max_oil_m3` argument also accepted NaN, which Rust's floating
point `max` treated as the other operand, silently converting an invalid cap
into a zero-production request.

`drill_for` is public and accumulated blowout expectation from non-finite
elapsed time or a non-finite/negative rate. Positive infinity could therefore
poison the cumulative expectation and saturate the integer blowout count.

## Change

- Reject non-finite or non-positive production intervals before changing field state or ledgers.
- Reject a NaN offtake cap while preserving positive infinity as the intentional unbounded cap used by `produce`.
- Reject non-finite/negative drilling rates and non-finite/negative elapsed intervals.
- Preserve the valid zero-duration drilling case: a well may be drilled immediately, with no well-year hazard accrued.
- Add regressions for positive/negative infinity, NaN offtake, invalid drilling rates/intervals and zero-duration well creation.

## Invariants

For valid finite intervals and valid inputs, the existing decline formula, production values,
carbon flows, methane leakage, formation-water accounting and well-hazard model are unchanged.
No reference values, simulation constants, replay hashes or snapshot formats were changed.

## Verification

Exact-head GitHub CI must pass formatting, Clippy, fast tests, week/year reference-pack acceptance,
desktop/render, installed backend, dashboard, computer service, scripts and dependency audit.
No local Cargo result is claimed because custom OpenCommander is unavailable.

## Next

1. Verify this exact head through all required CI jobs.
2. Merge only after the full exact-head gate passes.
3. Continue the sourced petroleum water-cut/spill model; do not invent field histories or spill volumes.
4. Keep the catalytic reformer blocked until mass, volume, carbon and by-product data can be closed from cited sources.

No live island state, production infrastructure, network service or owner data was changed.
