# Fishery material biomass-increment precision — 7 November 2026

## Canonical target

Repository: `dylmarriner/Maer-ken-island`

This work is a separate fishery correctness slice, stacked on PR #120's MSY diagnostic correction. It does not revive or modify the paused Maer-Ken Merge Completion job or replay PRs #73–76.

## Defect

The existing zero-catch guard rejects a positive remaining growth increment when adding it to the stored biomass rounds all the way back to the original biomass. It did not reject a materially wrong non-zero result.

At (B = 2^{53}), binary64 spacing is 2 tonnes. A positive increment of 3 tonnes rounds to a stored delta of 4 tonnes; an increment of 5 tonnes also rounds to 4 tonnes. Both distort the material ledger even though biomass changes.

## Production change

In `crates/mk_engine/src/regional/industry/fishery.rs`:

- Rename the private landed-total tolerance to `MATERIAL_INCREMENT_RELATIVE_TOLERANCE` because it now governs both stored mass accumulators.
- Keep `BiomassRemainderUnrepresentable` for a positive remainder that disappears completely.
- Add `BiomassIncrementUnrepresentable` for a positive remainder whose stored delta is materially rounded up or down.
- Compare `biomass_t - previous_biomass` with the unharvested increment before mutating either stock field.
- Reject the step before mutation when the relative error exceeds 1e-12.

## Regression

`biomass_increment_rounded_up_or_down_is_rejected_without_mutation` constructs exact Schaefer growth increments of 3 and 5 tonnes at a stored biomass of (2^{53}), proves both become a 4-tonne stored delta, then asserts the typed error and full stock immutability.

Existing zero-remainder, partial-harvest, cumulative landed precision, full-harvest, ordinary-rounding and snapshot-layout regressions remain in the same test module.

## Commits

- `15a4f24b` — production guard and regression.
- `33d284c8` — roadmap/checklist update.

## PR and dependency

PR #121: `fix(fishery): reject materially rounded biomass increments`.

Branch: `fix/fishery-biomass-increment-precision-20261107`.

Base branch: `fix/fishery-unrepresentable-msy-diagnostics-20261107` (PR #120). This is intentionally stacked: merge #120 first, then rebase #121 onto current main and rerun all required checks.

## Verification

No local Cargo result is claimed. Custom OpenCommander returned HTTP 429 in this run. The exact-head CI must pass formatting, Clippy, fast Rust tests, desktop build/render/input/frame-cost acceptance, installed backend, web dashboard, computer service, repository scripts, dependency audit and week/year reference-pack acceptance. Do not merge based on a previous head's green workflow.

## Safety and architecture

No reference data, species calibration, snapshot field, serialization layout, append-only history, replay hash, RNG, material ledger schema, IslandLife state or workforce assignment changed. The guard is deterministic and single-writer; it rejects an unrepresentable state transition before mutating the stock. No production simulation state was changed.

Fishery remains standalone until sourced species-specific stock/effort inputs and person-owned workforce integration are available.
