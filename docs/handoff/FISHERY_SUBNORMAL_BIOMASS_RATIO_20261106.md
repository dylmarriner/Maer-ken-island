# Fishery subnormal biomass-ratio hardening — 6 November 2026

Repository: `dylmarriner/Maer-ken-island`
Branch: `fix/fishery-subnormal-biomass-ratio-20261106`
Base: `main`

## Finding

The low-biomass Schaefer depensation path first calculates `p = B / k`. For finite positive biomass and carrying capacity, that ratio can underflow to zero even when the final depensated surplus remains representable. Returning zero in that case silently suppresses real positive model growth.

For `p < d`, the existing model is:

`r * B * (1 - p) * (p / d)`

When `p` underflows to zero, the mathematically equivalent limiting expression is:

`r * B² / (k * d)`

## Change

- Keep the existing direct arithmetic for all ordinary representable `B / k` ratios.
- Use logarithmic evaluation only when `B / k == 0`, `B > 0`, and `r > 0`.
- Preserve the existing typed `NonFiniteGrowth` failure if the final result is outside the finite f64 range.
- Preserve legitimate zero growth for an empty stock or zero intrinsic growth rate.

## Regression

The new unit test uses finite extreme values where `B / k` rounds to zero but `r * B² / (k * d)` is approximately `4e-200` tonnes/year. It asserts the result remains finite, positive and within a tight relative tolerance, then exercises `FishStock::fish` for one year and verifies a positive `1e-200` tonne catch is recorded from that surplus.

## Verification

- Source change: `fa6024adf6dbf5747cd1da4aeada5803fb2e84e9`.
- `WORK_REMAINING.md`, `docs/ISLAND_SYSTEM_STATUS.md`, the Phase 4b plan and this handoff were updated.
- No local Cargo test is claimed because custom OpenCommander device discovery is currently returning HTTP 429.
- Exact-head CI must pass `cargo fmt`, Clippy, fast Rust tests, desktop/backend builds and required week/year reference acceptance before merge.

## Invariants

No species parameters, reference values, carrying capacities, resilience classes, CPUE assumptions, stock snapshots, simulation state, RNG, append-only logs, material ledgers, replay inputs or hashes changed. The fishery remains standalone and is not connected to IslandLife until sourced species-specific stock and effort data are available.
