# Refinery assay cut-label validation

Date: 2026-11-05
Repository: `dylmarriner/Maer-ken-island`
Branch: `fix/refinery-cut-label-validation-20261105`
Base: `main`

## Defect

The refinery reference loader parsed a cut label into an optional end temperature.
The `None` value is the valid light-gas cut, but parse failures also returned
`None`. A malformed or newly introduced reference-pack label could therefore be
silently classified as refinery gas instead of stopping the load.

## Change

- Recognise only the exact `IBP-C4` light-gas label as the no-end-temperature cut.
- Parse numeric atmospheric ranges and require finite, non-negative, increasing bounds.
- Recognise the numeric `C-FBP` residue label as the only infinite-end cut.
- Return a validation error for unknown or malformed labels; the reference loader now
  propagates it as `MissingReference`.
- Add regressions for an unknown gas label, an unknown label, a descending range and
  a malformed residue label.

No yield, density, energy, carbon, simulation constant, replay hash or snapshot value
was changed. The existing cited Troll assay and IPCC reference values remain untouched.

## Verification

- Unit regression added to `crates/mk_engine/src/regional/industry/refinery.rs`.
- Exact-head hosted CI is required for formatting, Clippy, fast tests, week/year
  reference-pack acceptance, desktop/backend, dashboard, computer service, scripts
  and dependency audit.
- Local Cargo checks were not run because custom OpenCommander is unavailable in the
  current connected tools.

## Next

1. Verify the exact PR head with the full required CI set.
2. Merge only after exact-head checks and review are green.
3. Continue the remaining refinery reformer, oil-field water-cut/spill and energy-ledger
   requirements in `WORK_REMAINING.md`.


## First exact-head CI correction

- Workflow [#41970492431](https://github.com/dylmarriner/Maer-ken-island/actions/runs/41970492431) failed at `cargo fmt --all -- --check` only.
- Rustfmt requested multiline wrapping for two assertions and the invalid-label assertion.
- Commit `286ff3fe4765d64fd39f9c0178b19504d24fdc44` applies those exact formatter changes.
- Clippy and fast tests were skipped after formatting failed; no code assertion failure was observed.
- A fresh exact-head workflow is required. The failed run does not certify the corrected head.
