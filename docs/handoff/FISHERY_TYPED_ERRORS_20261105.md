# Fishery typed-error contract

Date: 2026-11-05
Repository: `dylmarriner/Maer-ken-island`
Branch: `fix/fishery-typed-errors-20261105`
Base: current `main` after PR #88 merge (`8505cc286c2894a3e3aaf48cb65761e9556bf0d5`)

## Defect

The fishery API returned `0.0` for malformed model/input/state and arithmetic overflow. A caller could not distinguish a legitimate zero catch from an invalid simulation step, so bad state could be silently treated as a valid no-op.

The public `unexploited` constructor also accepted zero, negative and non-finite carrying capacities, creating invalid stocks that failed only on a later step.

## Change

- Add a typed `FisheryError` covering invalid model, carrying capacity, catch request, time step, stock state, non-finite growth and landed-total overflow.
- Make `FishStock::unexploited` return `Result` and reject non-finite or non-positive carrying capacity immediately.
- Make `FishStock::fish` return `Result<f64, FisheryError>`.
- Keep validation and all arithmetic checks before mutation; every error path leaves the stock unchanged.
- Update the module's unit tests to assert exact failure categories and preserve valid finite-step behavior.

## Compatibility and invariants

Repository search found no production call sites for `FishStock::unexploited` or `FishStock::fish`; existing use is confined to this module's tests. This changes the public API before the fishery is wired into the island runtime, rather than leaving an ambiguous silent-zero contract for future callers.

No reference values, physical constants, ledger flows, replay hashes, snapshot formats or serialized `FishStock` fields are changed. The typed error is not persisted.

## Verification

- Source and regression tests are committed on this branch.
- Exact-head CI must pass formatting, Clippy, fast tests, desktop/backend, dashboard, computer-service and reference-pack acceptance.
- No local Cargo test is claimed because custom OpenCommander is not exposed in this execution.

## Next

1. Verify the exact-head workflow.
2. When fishery runtime integration begins, propagate `FisheryError` to the island step boundary rather than converting it back to zero catch.
3. Continue species-specific stock calibration and effort/catch-per-trip modelling only from the verified fisheries reference pack.

No live island state, production service, owner data or external system was changed.


## Exact-head formatting correction

The first exact-head workflow [#42014892960](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42014892960) reached the Rust formatting check and failed only on rustfmt line wrapping in the new tests. Clippy and fast tests were skipped by that formatting failure. The exact formatter layout was applied in commit `592c00a8b0638ce93d92e29b702d9195eef940cf`. A fresh exact-head run is required; the failed run does not verify the code.


## Second exact-head formatting correction

Fresh workflow [#42015041970](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42015041970) checked the merged PR ref and found one remaining rustfmt wrap in the `InvalidCarryingCapacity` Display arm. The other new source/test formatting differences from the first run were clean. The exact wrap was applied in commit `1317a154eb86915836bf9c2a825be41cbf8d265f`; the run is superseded and a fresh exact-head CI is mandatory.


## Clippy/compiler correction

Exact-head workflow [#42015513952](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42015513952) passed formatting and reached Clippy, where the second invalid-model regression still compared the new `Result<f64, FisheryError>` API to `0.0`. This is a test compile error, not a production-code diagnostic. The assertion now expects `Err(FisheryError::InvalidModel)` in commit `9fa08df45a47ca3cffb1364875c5ca13cb1e5a78`. A fresh exact-head run is required.


## Latest corrected exact-head CI

Workflow [#42016144545](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42016144545) is the latest exact-head run after the missed invalid-depensation assertion was corrected. At handoff update, dashboard, computer service, dependency audit and repository scripts have passed; formatting/Clippy/fast tests, desktop, installed backend and week/year reference acceptance remain in progress. Do not merge until every required job on this head passes.


## Public diagnostic API hardening (2026-11-06)

The typed-error contract now covers the public diagnostic methods as well as
`fish()`:

- `surplus_t_per_year()` and `msy_t()` return `Result<f64, FisheryError>`.
- Invalid models, carrying capacity and stock state no longer masquerade as a
  legitimate zero surplus/MSY.
- Non-finite diagnostic arithmetic returns `NonFiniteGrowth`.
- MSY computes `r * (k / 4)`, avoiding an overflowing intermediate when the
  mathematically correct result remains representable.
- `fish()` reuses the same stock-state validation and propagates diagnostic
  errors before mutating biomass or landed totals.

Regressions cover invalid model/state errors and the representable large-MSY
case. Repository search found these diagnostics only in this module's tests,
so no runtime caller or serialized field is changed.

Exact-head CI is required for the updated PR head. No local Cargo result is
claimed in this run; custom OpenCommander device discovery is returning HTTP
429. Do not merge until formatting, Clippy, fast tests, desktop/backend and
week/year acceptance all pass on the new head.
