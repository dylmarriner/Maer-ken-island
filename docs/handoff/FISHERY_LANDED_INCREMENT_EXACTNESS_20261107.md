# Exact cumulative landed-increment accounting — 7 November 2026

Repository: `dylmarriner/Maer-ken-island`
Branch: `fix/fishery-landed-increment-rebased-20261107`
Base: `main` at `b41bb5cf24bddca5df2bc556612c5edfc8b91bde` (PR #111 merge)

## Finding

The PR #110 guard rejected a positive catch only when binary64 addition left the cumulative landed total unchanged. That prevents a zero-recorded increment, but it does not prevent a rounded-up or rounded-down increment from recording a different mass than was actually caught.

For example, at a cumulative total of (2^{53}), adding 3 tonnes in binary64 rounds to a 4-tonne increase. The fishery would otherwise reduce biomass by 3 tonnes while recording 4 tonnes in its cumulative material total.

## Implementation

`FishStock::fish` now computes the persisted increment as `recorded_delta = landed_t - self.landed_t` after checking the projected total is finite. For a positive catch it rejects when the recorded delta is non-positive or when `abs(recorded_delta - catch) > catch * 1e-12`. This allows ordinary accumulated binary64 rounding while rejecting vanished or materially distorted mass before either biomass or landed mass is committed. The existing `LandedTotalUnrepresentable` error covers both cases. The serialized `FishStock` layout is unchanged.

## Regression

`landed_increment_rounded_to_a_different_amount_is_rejected_without_mutation` uses a valid Medium-resilience model and a stock with `landed_t = 2^53`. A requested 3-tonne catch is within the interval growth limit, but the projected cumulative total increases by 4 tonnes. The test expects `LandedTotalUnrepresentable` and asserts the entire stock remains unchanged.

- `landed_increment_rounded_down_is_rejected_without_mutation` uses the same stock scale with a requested 5-tonne catch. The tie-to-even binary64 addition records only 4 tonnes; the guard rejects it before either biomass or landed mass changes. Regression commit `b6d6e11ccbbae169008c451e955258bc60d835f5`.

## Verification

- Source and regression are committed in this PR.
- PR #110 is merged as `148ac98c95e58aac7b2065dd0cc68884790f4168`; PR #111 is merged as `b41bb5cf24bddca5df2bc556612c5edfc8b91bde`.
- Exact-head workflow [#42275024667](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42275024667) passed formatting, Clippy, fast Rust tests, desktop, installed backend, web dashboard, computer service, repository scripts, dependency audit and week/year reference-pack acceptance; nightly slow tests were skipped by design.
- This follow-up changes the fishery source and requires a fresh exact-head workflow. No local Cargo result is claimed because the required custom OpenCommander runner access was unavailable.
- No reference values, species calibration, RNG, append-only logs, snapshots, replay hashes, material ledger schema, IslandLife state, budget or offline limits were changed.
- Fishery remains standalone pending sourced species-specific stock/effort data and owner-approved workforce integration.


## Rebase after PR #111 merge — 7 November 2026

PR #111 was merged to main as `b41bb5cf24bddca5df2bc556612c5edfc8b91bde`. The original PR #112 branch became unmergeable because both branches changed `fishery.rs` and the same handoff/checklist sections. The landed-delta tolerance and its two regressions were reapplied to a fresh branch from the new main; the PR #111 residual-growth invariant is retained. Documentation was reconciled to the new main and the previous exact-head green run is explicitly treated as pre-rebase evidence only.

Rebased source/test commit: `186766a19108338f0bd75a28dd00ef2bd1cc8b76`.

No snapshot layout, replay hash, RNG, reference data, material ledger schema, IslandLife state or production state changed.
