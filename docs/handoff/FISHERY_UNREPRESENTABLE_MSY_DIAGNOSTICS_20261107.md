# Unrepresentable MSY diagnostics

Date: 2026-11-07
Repository: `dylmarriner/Maer-ken-island`
PR: #120
Branch: `fix/fishery-unrepresentable-msy-diagnostics-20261107`
Base: `main` after PR #119 merge `9cc2b15808c57bd5f61d305bca6122e79820c6bc`

## Finding

The fishery's annual and interval growth paths fail closed when a positive mathematical result rounds to zero. The two read-only MSY diagnostics did not:

- `msy_t()` could return `Ok(0.0)` for positive growth rate and positive carrying capacity when the mathematical maximum sustainable yield was below the smallest representable binary64 value, including cases where an intermediate multiplication rounded to zero even though the exact final result rounded to one subnormal unit.
- `msy_biomass_t()` could return `Ok(0.0)` when the positive analytic peak-biomass target rounded to zero.

Zero is misleading here: every valid piecewise peak fraction is positive for the allowed depensation thresholds.

## Change

- Added `FisheryError::UnrepresentableMsy` for positive maximum yield that rounds to zero.
- Added `FisheryError::UnrepresentableMsyBiomassTarget` for a positive analytic target that rounds to zero.
- Preserved genuine zero MSY when the selected growth rate is zero.
- If the first MSY multiplication order rounds to zero, retry with the growth rate scaled by the peak fraction; reject only when the alternate order also rounds to zero.
- Preserved representable subnormal biomass targets, including the upper depensation branch.
- Added doc comments to both new public error variants.
- Rebased the change onto current `main` after PR #119 merged; the previous branch conflicted because both changes touched `fishery.rs` and the fishery status documents.

## Regression

`positive_but_unrepresentable_msy_diagnostics_fail_closed` verifies:

- Smallest-subnormal carrying capacity with positive default growth rate returns the typed MSY error when both multiplication orders round to zero.
- The same stock returns the typed biomass-target error rather than zero.
- A zero selected growth rate remains a valid zero MSY.
- Two subnormal capacity units produce unrepresentable MSY but a representable one-unit biomass target.
- Three subnormal capacity units retain the nearest non-zero target.
- The upper depensation branch retains a representable one-unit target.
- For `r = 1.25`, `d = 0.51`, and `K = 2` minimum-subnormal units, the first multiplication order rounds to zero but the alternate order returns one unit.
- For `r = 2.5`, `d = 0.6`, and `K = 1` minimum-subnormal unit, the alternate order also recovers one unit.

## Rebased commits

- `1d7d9789` — production guards and regression.
- `6c786636` — checklist and fishery work-remaining reconciliation.
- `46785b65` — canonical status reconciliation.
- `ea553f10` — alternate-order recovery and two subnormal regressions.
- Handoff/documentation commits follow.

## Verification

- Parent PR #119 merged as `9cc2b15808c57bd5f61d305bca6122e79820c6bc`.
- Exact-head workflow [#42336338097](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42336338097) passed formatting, Clippy, fast Rust tests, desktop build/tests/rendering/input/frame-cost acceptance, installed backend, dashboard, computer service, repository scripts, dependency audit and week/year reference-pack acceptance; nightly slow tests were skipped by design.
- The prior PR #120 head `9468d8f9c3f6c02f2c08e197676b9bacab776e6d` was based on pre-#119 main and was not mergeable after #119 landed. Its workflow [#42338493268](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42338493268) had passed formatting, Clippy, desktop, backend, dashboard, computer service, repository scripts, dependency audit and week/year acceptance; fast Rust tests were still running when the branch reconciliation began. That run does not verify the rebased head.
- Exact-head workflow [#42341044056](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42341044056) passed its required jobs on the prior head `7bae0d02`, but Codex and CodeRabbit identified the alternate-order case above. That run does not verify the corrected head.
- Exact-head CI and a fresh code review are required on the corrected head. Resolve both inline review findings before merge; do not merge until fast Rust tests and all required acceptance jobs pass on that exact SHA.
- Custom OpenCommander still returns HTTP 429; no local Cargo result is claimed.

## Invariants

No reference values, species parameters, snapshot fields/layout, append-only logs, replay hashes, RNG, material ledger schema, IslandLife state or workforce assignments changed. Fishery remains standalone pending sourced species-specific stock/effort data and owner-approved workforce integration.

## Next

1. PR #120's existing branch has been updated to the rebased tree after PR #119 merged.
2. Verify the final docs-reconciled PR head in exact-head CI; the earlier workflow on head `23ed51ef` is superseded and does not certify the final head.
3. Resolve the Codex/CodeRabbit subnormal multiplication-order finding and terminology comment; rerun exact-head CI and review.
4. Merge only after every required check passes.
5. Continue sourced fishery-to-IslandLife integration only after species stock/effort data and owner-approved workforce decisions exist.
## Final documentation reconciliation

- `WORK_REMAINING.md` now records PR #118 and PR #119 as merged and verified, with PR #120 as the remaining MSY diagnostic gate (`d3b8d2d1`).
- `docs/ISLAND_SYSTEM_STATUS.md` now records PR #119's merge evidence and PR #120's rebased pending state (`5a0eb382`).
- The previous exact-head workflow [#42340666071](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42340666071) on head `23ed51ef` was in progress when the final documentation reconciliation began; it does not certify the final docs-reconciled head.
- Codex and CodeRabbit both identified the same alternate-order subnormal MSY defect and the terminology issue; the production fallback, two regressions, public-variant docs and terminology correction are committed. Their three inline threads are resolved.
- Workflow [#42345147384](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42345147384) was queued on the review-corrected code head `02c2c40b`; this handoff update creates a newer head, so a fresh exact-head workflow is still required.