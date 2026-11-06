# Exact cumulative landed-increment accounting — 7 November 2026

Repository: `dylmarriner/Maer-ken-island`
Branch: `fix/fishery-landed-increment-exactness-20261107`
Base: `main` at `148ac98c95e58aac7b2065dd0cc68884790f4168`

## Finding

The PR #110 guard rejected a positive catch only when binary64 addition left the cumulative landed total unchanged. That prevents a zero-recorded increment, but it does not prevent a rounded-up or rounded-down increment from recording a different mass than was actually caught.

For example, at a cumulative total of (2^{53}), adding 3 tonnes in binary64 rounds to a 4-tonne increase. The fishery would otherwise reduce biomass by 3 tonnes while recording 4 tonnes in its cumulative material total.

## Implementation

`FishStock::fish` now requires:

```rust
landed_t - self.landed_t == catch
```

for every positive catch, after checking that the cumulative total is finite and before committing either biomass or landed mass. The existing `LandedTotalUnrepresentable` error covers both zero-recorded and distorted increments. The serialized `FishStock` layout is unchanged.

## Regression

`landed_increment_rounded_to_a_different_amount_is_rejected_without_mutation` uses a valid Medium-resilience model and a stock with `landed_t = 2^53`. A requested 3-tonne catch is within the interval growth limit, but the projected cumulative total increases by 4 tonnes. The test expects `LandedTotalUnrepresentable` and asserts the entire stock remains unchanged.

## Verification

- Source and regression are committed in this PR.
- PR #110 is merged as `148ac98c95e58aac7b2065dd0cc68884790f4168`.
- Exact-head workflow [#42275024667](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42275024667) passed formatting, Clippy, fast Rust tests, desktop, installed backend, web dashboard, computer service, repository scripts, dependency audit and week/year reference-pack acceptance; nightly slow tests were skipped by design.
- This follow-up changes the fishery source and requires a fresh exact-head workflow. No local Cargo result is claimed because the required custom OpenCommander runner access was unavailable.
- No reference values, species calibration, RNG, append-only logs, snapshots, replay hashes, material ledger schema, IslandLife state, budget or offline limits were changed.
- Fishery remains standalone pending sourced species-specific stock/effort data and owner-approved workforce integration.
