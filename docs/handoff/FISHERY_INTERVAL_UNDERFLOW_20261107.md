# Fishery interval-growth underflow guard

Date: 2026-11-07
Repository: `dylmarriner/Maer-ken-island`
PR: #118
Branch: `fix/fishery-unrepresentable-surplus-20261107`
Base: `main` after PR #117 merged as `6da6e254f75146a2b3feeae3b0dab7e67eaf089d`.

## Defect

`surplus_t_per_year` now rejects a positive annual surplus that itself rounds to zero. However, a positive annual surplus multiplied by a positive, minimum-subnormal time step can still produce a mathematically positive interval increment below binary64's representable range. `growth_capacity_t` previously returned `Ok(0.0)`, silently discarding that positive growth from the stock/material ledger.

## Change

- After calculating raw interval growth and finite stock headroom, detect `raw_growth == 0` while annual surplus, time step and headroom are all positive.
- Return `FisheryError::UnrepresentableGrowth` before the caller can mutate stock state.
- Preserve genuine zero for zero annual surplus, zero time step and no remaining carrying-capacity headroom.

## Regression

`positive_interval_growth_below_binary64_range_is_rejected_without_mutation`:
- Uses a valid default fishery model and a 1,000 t stock at 500 t biomass.
- Confirms annual surplus is positive.
- Uses `f64::from_bits(1)` as the positive minimum-subnormal interval.
- Asserts both the read-only sustainable-catch diagnostic and `fish()` return `UnrepresentableGrowth`.
- Asserts biomass and landed total remain byte-for-byte equal to the pre-call stock value.

## Commit

- `e9856040fb13247a046dec0b896497c37834cb44` — production guard and regression.

## Verification

The exact-head workflow [#42331233817](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42331233817) was queued when this handoff was written. No local Cargo result is claimed because custom OpenCommander returned HTTP 429. Required CI remains formatting, Clippy, fast Rust tests, desktop/backend, dashboard, computer service, dependency audit and week/year reference-pack acceptance.

## Invariants

No reference values, species parameters, snapshot fields/layout, append-only logs, replay hashes, RNG, material ledger, IslandLife state or workforce assignments changed. Fishery remains standalone pending sourced species-specific stock/effort data and person-owned workforce decisions.

## Next

- Verify the exact final PR #118 head.
- Merge only after all required exact-head checks pass.
- Continue sourced fishery integration and remaining IslandLife roadmap work.


## Status-document reconciliation — 7 November 2026

Commit `77ae69dbc311d13583aa335a15b00239e09512f1` updates `docs/ISLAND_SYSTEM_STATUS.md` to record the merged and verified fishery PRs #110, #111, #114, #115 and #117, and the pending PR #118 interval-underflow guard. Fishery remains standalone pending sourced species/effort data and person-owned workforce integration.
