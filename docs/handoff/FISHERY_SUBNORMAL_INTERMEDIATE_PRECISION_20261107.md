# Fishery subnormal intermediate precision

Date: 2026-11-07
Repository: `dylmarriner/Maer-ken-island`
Branch: `fix/fishery-subnormal-intermediate-precision-20261107`
Base: PR #116 merge `e3e7a92dad458b125150f1c6e712345b71c56487`

## Defect

PR #115 added a log-space fallback when the depensated surplus calculation
produced an exact zero from positive biomass and growth-rate inputs. A further
case remained: `B/K` and the depensation factor can both be normal, while the
intermediate `B × factor` is positive but subnormal. Multiplying that rounded
subnormal by a large growth rate can produce a normal final surplus with
material relative error.

The regression inputs demonstrate the issue before the fix:

- Carrying capacity: 4,000 t.
- Biomass: 1e-160 t.
- Growth rate: 1e308 per year.
- Depensation threshold: 0.25.
- `B × factor`: 1e-323, positive but subnormal.
- Direct result: approximately 9.881312916824931e-16 t/year.
- Log-space result: approximately 1e-15 t/year.

The direct path is about 1.19% low even though the final result is normal and
representable.

## Change

- Compute the biomass product once and retain it for both the direct path and
  the precision check.
- Use log-space evaluation when `B/K`, the depensation factor, or the
  `B × factor` product is subnormal (including zero).
- Keep the ordinary direct calculation for normal intermediates.
- Add a regression that proves the pre-fix direct path loses more than 0.1%
  while the corrected result matches the log-space reference within 1e-12
  relative error.

No reference data, stock schema, snapshot layout, append-only logs, replay
hashes, RNG, material ledger or IslandLife state changed.

## Commits

- `008887938cbb7998cb65aa56b0e42c620cfc0b89` — source correction and regression.
- `b5ab0ce7e68d9c08fbfb7a039fda097240fb2781` — exact rustfmt correction after workflow #42321774856.

## Verification

- Initial exact-head workflow [#42321774856](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42321774856) reached checkout and failed only at `cargo fmt --all -- --check`; Clippy and fast tests were skipped by the workflow dependency.
- The formatter identified two line-wrapping differences in the new source/test. Both were corrected in `b5ab0ce7`.
- No local Cargo pass is claimed because custom OpenCommander device discovery returned transient HTTP 429.
- PR #117 exact-head CI must pass formatting, Clippy, fast Rust tests, desktop, installed backend, dashboard, computer service, repository scripts, dependency audit and week/year reference-pack acceptance.
- PR #117 must remain unmerged until every required check passes on its exact head.

## Next

1. Verify the new exact head after the rustfmt correction.
2. Address only evidence-backed failures.
3. Merge only after all required checks pass.
4. Keep the fishery standalone until sourced species-specific stock/effort
   data and person-owned workforce decisions permit IslandLife integration.
