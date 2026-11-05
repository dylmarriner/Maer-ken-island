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
- First exact-head workflow `#41997469485` ran the formatter and identified two formatting-only differences. No Clippy or fast-test pass is claimed from that run.
- Applied the exact rustfmt output in commit `27d865a97bd51a3ecf9d473a366df3a104697036`.
- A fresh exact-head workflow is required for the corrected branch head.
- Local Cargo checks were not run because the custom OpenCommander device session
  terminates before command execution.

## Follow-up: validate raw prior bounds

The first validation pass checked the derived midpoint, which could still hide an
invalid source range (for example, a negative lower bound paired with a larger
positive upper bound). The loader now validates both published bounds for
finiteness, non-negativity and ordering before deriving the midpoint. The
midpoint calculation subtracts before adding so large finite bounds cannot
overflow an intermediate sum. Regression tests cover negative, reversed,
non-finite and large finite bounds.

## Next

After exact-head CI passes, continue the sourced fishery-to-human food chain work:
species-specific stock calibration and human fishing choices/return rates remain
separate acceptance requirements. This validation guard does not claim those
features are implemented.


## Remaining checklist

- [ ] Finish the exact-head fast Rust test suite and resolve any failures.
- [ ] Keep the week/year reference-pack distinction explicit: the current fishery-only
      workflow path-skipped those slow tests; it did not execute them.
- [ ] Calibrate species-specific fish stocks from cited reference packs.
- [ ] Implement human fishing choices, effort and return-rate accounting without
      invented values or breaking deterministic replay.
- [ ] Continue the remaining `WORK_REMAINING.md` requirements: petroleum water-cut
      and spill behaviour, mass/carbon-balanced reformer, workforce/maintenance/town
      integration, remaining resource/material chains, climate/energy fidelity and
      Phase 5 application acceptance.

## Blockers and next task

Exact-head workflow [#41997671638](https://github.com/dylmarriner/Maer-ken-island/actions/runs/41997671638)
has passed formatting, Clippy, desktop build/tests, installed-backend checks,
dashboard, computer-service, repository scripts and dependency audit. Its fast
Rust tests are still running. The week/year reference job path-skipped this
fishery-only change. Custom OpenCommander `list_devices` returns
`Session terminated`, so no local Cargo run was possible.

Next: finish exact-head fast tests, merge only after the full required checks pass,
then implement the next source-backed fishery-to-human slice. Preserve the
single-writer/replay and append-only persistence invariants.


## Follow-up: bound aggregate-step fish growth

A second numerical edge was found after reviewing the discrete Schaefer step. The
instantaneous surplus rate multiplied by a very long `dt_s` could exceed the
stock's entire unoccupied carrying-capacity headroom. A single aggregate step
could then land more fish than could physically be added to that stock.

- Cap one step's estimated growth at `k_t - biomass_t`, after validating the
  raw growth estimate is finite and non-negative.
- Add a regression with a 100-year aggregate interval; the catch must not exceed
  the 500 t of remaining capacity in the 1,000 t stock.
- This is a conservative single-step upper bound, not a claim that the model
  resolves repeated multi-year spawning/harvest cycles. The simulation's normal
  hourly cadence is unchanged.
- No reference values, snapshots, replay hashes or canonical simulation inputs
  were changed by this guard.

The new exact-head CI run must verify this follow-up together with the existing
input-validation regressions before merge. The prior workflow run on
`88feff87` is superseded by this code commit.
