# Unrepresentable MSY diagnostics

Date: 2026-11-07
Repository: `dylmarriner/Maer-ken-island`
PR: #120
Branch: `fix/fishery-unrepresentable-msy-diagnostics-20261107`
Base: `main` after PR #119 merge `9cc2b15808c57bd5f61d305bca6122e79820c6bc`

## Finding

The fishery's annual and interval growth paths fail closed when a positive mathematical result rounds to zero. The two read-only MSY diagnostics did not:

- `msy_t()` could return `Ok(0.0)` for positive resilience and positive carrying capacity when the mathematical maximum sustainable yield was below the smallest representable binary64 value.
- `msy_biomass_t()` could return `Ok(0.0)` when the positive analytic peak-biomass target rounded to zero.

Zero is misleading here: every valid piecewise peak fraction is positive for the allowed depensation thresholds.

## Change

- Added `FisheryError::UnrepresentableMsy` for positive maximum yield that rounds to zero.
- Added `FisheryError::UnrepresentableMsyBiomassTarget` for a positive analytic target that rounds to zero.
- Preserved genuine zero MSY when resilience is zero.
- Preserved representable subnormal biomass targets, including the upper depensation branch.
- Rebased the change onto current `main` after PR #119 merged; the previous branch conflicted because both changes touched `fishery.rs` and the fishery status documents.

## Regression

`positive_but_unrepresentable_msy_diagnostics_fail_closed` verifies:

- Smallest-subnormal carrying capacity with positive default resilience returns the typed MSY error.
- The same stock returns the typed biomass-target error rather than zero.
- Zero resilience remains a valid zero MSY.
- Two subnormal capacity units produce unrepresentable MSY but a representable one-unit biomass target.
- Three subnormal capacity units retain the nearest non-zero target.
- The upper depensation branch retains a representable one-unit target.

## Rebased commits

- `1d7d9789` — production guards and regression.
- `6c786636` — checklist and fishery work-remaining reconciliation.
- `46785b65` — canonical status reconciliation.
- Handoff commit follows.

## Verification

- Parent PR #119 merged as `9cc2b15808c57bd5f61d305bca6122e79820c6bc`.
- Exact-head workflow [#42336338097](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42336338097) passed formatting, Clippy, fast Rust tests, desktop build/tests/rendering/input/frame-cost acceptance, installed backend, dashboard, computer service, repository scripts, dependency audit and week/year reference-pack acceptance; nightly slow tests were skipped by design.
- The prior PR #120 head `9468d8f9c3f6c02f2c08e197676b9bacab776e6d` was based on pre-#119 main and was not mergeable after #119 landed. Its workflow [#42338493268](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42338493268) had passed formatting, Clippy, desktop, backend, dashboard, computer service, repository scripts, dependency audit and week/year acceptance; fast Rust tests were still running when the branch reconciliation began. That run does not verify the rebased head.
- Exact-head CI is required on the rebased PR head. Do not merge until fast Rust tests and all required acceptance jobs pass on that exact SHA.
- Custom OpenCommander still returns HTTP 429; no local Cargo result is claimed.

## Invariants

No reference values, species parameters, snapshot fields/layout, append-only logs, replay hashes, RNG, material ledger schema, IslandLife state or workforce assignments changed. Fishery remains standalone pending sourced species-specific stock/effort data and owner-approved workforce integration.

## Next

1. Move the verified rebased commit to PR #120's existing branch with an expected-head lease.
2. Verify exact-head CI on the updated PR.
3. Mark ready and merge only after every required check passes.
4. Continue sourced fishery-to-IslandLife integration only after species stock/effort data and owner-approved workforce decisions exist.