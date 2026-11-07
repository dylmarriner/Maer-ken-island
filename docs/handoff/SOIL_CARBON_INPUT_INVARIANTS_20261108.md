# Soil-carbon fail-closed input invariants

Date: 2026-11-08
Repository: `dylmarriner/Maer-ken-island`
Branch: `fix/soil-carbon-input-invariants-20261108`

## Defect

The active RothC soil-carbon model relied on well-formed caller data without enforcing its public boundaries:

- `SoilCarbon::steady_state` used chained `zip` calls, silently truncating mismatched per-cell vectors.
- Negative annual production input was silently clamped to zero.
- `SoilCarbon::step` could mutate earlier cells before discovering a malformed input in a later cell.
- Direct `RothC::step_cell` calls accepted negative/non-finite durations and malformed pools or inputs.
- Deserialised or manually constructed RothC parameters were not checked before they could mutate soil state.

## Change

- Validate RothC rate constants, cover factors, partition fractions and carbon-conserving manure fractions.
- Require equal per-cell vector lengths before steady-state construction or stepping.
- Reject negative/non-finite annual inputs, soil inputs, temperature factors, durations and pool values.
- Preflight every caller-supplied input and temperature vector before mutating the first cell.
- Keep the grid-step hot path efficient by validating the parameter set once per grid step and using a private validated single-cell kernel.
- Preserve the existing RothC equations and all valid-input results.

## Regression coverage

- Mismatched steady-state vectors fail rather than truncate.
- A NaN input in a later cell is rejected before any earlier cell is mutated.
- A negative single-cell duration is rejected without changing its pools.
- Existing carbon-conservation and steady-state equilibrium tests remain in place.

## Commits

- `2acd7c4b745e6d099dc49cdcf2832936e3b1fc6a` — fail-closed RothC and soil input validation.
- `965666ac8d05d1481bf640889a7df42e85a5de29` — validate parameters once per grid step.

## Verification

- Source and regression changes are committed and pushed.
- Exact-head Rust formatting, Clippy, fast tests, desktop/backend and week/year reference acceptance are pending.
- No local Cargo result is claimed; the current environment has no authorized OpenCommander execution path exposed.
- No NPP value, soil nutrient parameter, snapshot schema, replay entry, RNG stream or digest pin was changed.

## Next

1. Complete exact-head CI for this branch.
2. Review any formatter/compiler/test findings and correct them on this branch.
3. Continue D31 with sourced mineral nitrogen and phosphorus pools and a soil-limited NPP response. Do not invent nutrient concentrations or response coefficients.


## Formatting correction — 8 November 2026

The first exact-head workflow stopped at `cargo fmt --all -- --check`. It identified only rustfmt line-wrapping differences in the new assertions and one test assertion; no compiler or test job ran on that head.

Commit `35d895d68fce9a341bc9fa57b8956fd0a8f7dc93` applies the formatter's reported layout exactly. A fresh exact-head workflow is required; the failed predecessor is not counted as code verification.


## Grid-wide soil-step atomicity follow-up — 8 November 2026

The initial fail-closed pass validated caller vectors before mutation, but derived-value failures could still occur after earlier cells had advanced: a later cell's temperature-factor × cover overflow, or the grid-wide respiration accumulator exceeding finite range.

- Added a full preflight over every cell's current pools, incoming carbon, derived decay modifier and cumulative grid carbon budget before the mutation loop.
- The grid carbon budget includes prior cumulative respiration and bounds the maximum respiration the step can add; existing post-step assertions remain as invariant checks.
- Added regressions proving both a late-cell decay-modifier overflow and an unrepresentable multi-cell carbon budget leave the entire SoilCarbon value unchanged.
- Production change commit: 4228af75ed2bd1bd203d70c6303813f6caa2229b.
- Exact-head formatting, Clippy, fast Rust tests, desktop/backend and week/year acceptance must be rerun. No local Cargo result is claimed.
- Valid RothC equations, reference coefficients, snapshots, replay format, RNG and digest pins remain unchanged.


## Exact-head fast-test fixture correction — 8 November 2026

Workflow [#42408189828](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42408189828) completed the reference-pack acceptance successfully: both week and year packs passed. Formatting and Clippy also passed. The fast Rust suite ran 879 tests and reported **878 passed, one failed**.

The failure was in `step_preflights_late_derived_decay_overflow_before_mutating_any_cell`. Its second cell is non-vegetated, so the production preflight correctly multiplies its temperature factor by `cover_bare`; the fixture had changed `cover_vegetated` instead. The test therefore never constructed the intended overflow and its `result.is_err()` assertion failed.

Correction:
- Set `soil.params.cover_bare = 2.0` for the non-vegetated second cell.
- Document why the bare cover factor is the relevant input.
- Leave the production preflight unchanged; the implementation was not weakened or altered to satisfy a malformed fixture.

Commit:
- `ab6aac5ea328e53e32a034ef758610194e3217a4` — correct the late-cell derived-overflow test fixture.

The prior workflow proves the week and year reference packs passed on the previous code head, but it does not verify this corrected test head. Fresh exact-head formatting, Clippy, fast tests, desktop/backend and reference-pack checks are required. No local Cargo result is claimed because custom OpenCommander is returning HTTP 429.


## Canonical handoff reconciliation note — 8 November 2026

The root `HANDOFF.md` on `main` is stale: it still presents PRs #120 and #121 as current open work, but GitHub confirms both are merged (#120 merge commit `66bf099f36c567ab344fcc9b268a9a33a656011d`; #121 merge commit/current main `522aba3d860435c92a4938cf5ffba2b427941cd7`).

Current active engineering paths are:
- PR #123 — replay sequence/index correctness.
- PR #124 — timber cross-cut conservation and reservation.
- PR #125 — RothC soil input atomicity.

These branches all touch some combination of `HANDOFF.md`, `WORK_REMAINING.md` and `docs/ISLAND_SYSTEM_STATUS.md`. Do not independently overwrite those shared files from competing branches. Keep each dedicated `docs/handoff/` record current, then reconcile the shared root handoff/status documents once the code branches are integrated.
