# Fishery landed-total overflow atomicity — 7 November 2026

Repository: `dylmarriner/Maer-ken-island`
Branch: `test/fishery-landed-overflow-atomicity-20261107`
Base: `main` after PR #109 merge `0613dff`

## Invariant under test

`FishStock::fish` is a deterministic single-writer step. It must either commit the complete next stock state or leave the prior state untouched. A late arithmetic failure must not commit the biomass calculation while rejecting the cumulative landed total.

## Regression

The test constructs a physically valid stock with:
- finite carrying capacity `1.0e308`;
- biomass `5.0e307`;
- finite prior landed total `f64::MAX`;
- a valid Medium resilience model and one-year step.

The requested catch is capped by interval growth. Adding that positive catch to the already-large landed total overflows, so `fish` must return `FisheryError::LandedTotalOverflow`.

The regression clones the stock before the call and asserts the entire `FishStock` remains equal after the error. This protects both biomass and cumulative landed mass from a partial commit.

## Scope and invariants

- No successful-step formula or fishery calibration changed.
- No species values, reference packs, carrying capacities used by the island, CPUE, RNG, append-only logs, snapshots, replay hashes, material ledgers or live simulation state changed.
- No IslandLife integration is claimed; that remains blocked on sourced species stock/effort data and owner-approved workforce choices.

## Verification

- Regression and repository handoff/status updates are committed on this branch.
- Exact-head CI must pass formatting, Clippy, fast Rust tests, desktop/backend, web/computer service, dependency audit and week/year reference-pack acceptance.
- No local Cargo result is claimed because custom OpenCommander currently returns HTTP 429.

## Follow-up — cumulative precision loss

A separate numerical boundary is now tracked: a positive catch can be smaller than half an ULP of a large but finite cumulative landed total. In that case IEEE-754 addition can leave `landed_t` unchanged even though `fish()` returns a positive catch. This would report material as landed without persisting it in the stock's cumulative total.

Implemented on this branch:

- Added `FisheryError::LandedTotalUnrepresentable` with an explicit display message.
- `FishStock::fish` now rejects a positive catch when adding it leaves the cumulative `landed_t` unchanged at binary64 precision, before either biomass or landed total is committed.
- Added a regression with `landed_t = 2^53` and a one-tonne catch. It proves the addition rounds back to the existing total, the error is returned, and the entire stock remains equal to its pre-step state.
- No compensation field or snapshot schema change; the serialized `FishStock` layout remains unchanged.

No compensation field or snapshot schema change is proposed; preserving the existing serialized field layout is preferred. Exact-head Rust and acceptance CI is required.
