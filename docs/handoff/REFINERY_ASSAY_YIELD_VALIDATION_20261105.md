# Refinery assay yield validation

Date: 2026-11-05
Repository: `dylmarriner/Maer-ken-island`
Branch: `fix/refinery-assay-yield-validation-20261105`
Base: current `main` after refinery cut-label validation merge.

## Defect

The refinery loader read each assay cut's weight percentage and immediately converted
it to a fraction. `distilled_fractions()` then normalised by the sum. A negative,
non-finite, over-100% or incomplete reference table could therefore be silently
normalised into plausible-looking product shares. A zero/empty table could produce
non-finite fractions.

## Change

- Validate that the assay has at least one cut.
- Require every weight fraction to be finite and within 0..=1.
- Require the total to be within 0.01 percentage points of 100%, allowing the source
  table's three-decimal rounding (the checked-in Troll assay totals 100.001%).
- Run validation in `RefineryModel::from_reference` before the model can be used.
- Add unit regressions for empty, negative, non-finite, underweight and overweight
  assay inputs.
- Assert the checked-in reference assay's total is within the documented tolerance.

No yields, density, carbon, energy, simulation constants, replay hashes, snapshots or
reference data were changed.

## Verification

- Exact-head hosted CI is required for formatting, Clippy, fast tests, week/year
  reference-pack acceptance, desktop/backend, dashboard, computer service, scripts
  and dependency audit.
- Custom OpenCommander returned HTTP 429 during this run, so no local Cargo test or
  formatting pass is claimed.

## Next

1. Verify the exact PR head through all required CI jobs.
2. Merge only after green exact-head CI and review.
3. Continue the cited catalytic reformer model only after its mass, volume, carbon and
   by-product basis is sourced and can be audited. The checked-in Speight table gives
   C5+ volume yield and RON, but by itself does not close mass or carbon.
4. Keep the petroleum water-cut/spill and workforce/industry requirements open.

No simulation state, live island, owner governance or production infrastructure changed.
