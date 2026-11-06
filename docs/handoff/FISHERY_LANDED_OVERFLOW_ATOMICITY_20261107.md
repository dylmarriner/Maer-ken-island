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
