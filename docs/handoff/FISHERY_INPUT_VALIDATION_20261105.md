# Fishery input and state validation

Date: 2026-11-05
Repository: `dylmarriner/Maer-ken-island`
Branch: `fix/fishery-input-validation-20261105b`
Base: `main` at `4bbf75af71ad2fa6ad6c748842b262a732c15fb2`

## Defect

`FishStock::fish` used `wanted_t.clamp(0.0, grown)` without validating
`wanted_t`, `dt_s`, the model or the stock state. A negative or NaN time step
can make the clamp bounds invalid and panic; non-finite catch/time/model values
can otherwise poison `landed_t` or biomass.

## Change

- Validate all reference-derived growth rates and the Schaefer depensation threshold
  before accepting a `FisheryModel`.
- Fail closed on non-finite/negative catch requests and time intervals.
- Reject invalid carrying capacity, biomass or landed stock state without mutation.
- Detect non-finite growth and accumulation before writing either stock field.
- Replace panic-prone `clamp` with a bounded `min` after validation.
- Add regressions for NaN, both infinities, negative values, invalid model/state,
  and arithmetic overflow.

Valid finite fishing inputs retain the existing Schaefer surplus-production model,
depensation rule, catch ceiling and stock cap. No reference pack, simulation digest,
snapshot schema or valid-input result is intentionally changed.

## Verification

- Unit regressions are included in `crates/mk_engine/src/regional/industry/fishery.rs`.
- Exact-head GitHub Actions must pass formatting, Clippy and the required fast suite.
- Local Cargo checks were not run because the custom OpenCommander device session
  terminates before command execution.

## Next

After exact-head CI passes, continue the sourced fishery-to-human food chain work:
species-specific stock calibration and human fishing choices/return rates remain
separate acceptance requirements. This validation guard does not claim those
features are implemented.
