# Fishery interval-growth capacity cap

Date: 2026-11-06
Repository: `dylmarriner/Maer-ken-island`
Branch: `fix/fishery-interval-growth-cap-20261106`
Base: `main` at `d6f6ce7a1c5f01bf5a14d158fad32dbbcdaffb0a`.

## Defect

`FishStock::fish` rejected an interval when the unconstrained time-integrated growth overflowed `f64`, even though the stock's remaining capacity was finite and the model already caps growth at that headroom. A sufficiently long finite time step can make the raw growth amount unrepresentable while the physically relevant stock result remains representable.

This is distinct from an annual surplus that is itself unrepresentable. `surplus_t_per_year` continues to reject that invalid diagnostic; this change handles overflow only after a finite annual surplus has been scaled over a finite interval.

## Change

- Treat positive infinity from the interval-growth multiplication as proof that unconstrained growth exceeds every finite stock headroom.
- Cap that case at `k_t - biomass_t`, preserving the existing carrying-capacity invariant.
- Continue to reject NaN and negative growth as `NonFiniteGrowth`.
- Keep catch bounded by grown biomass and preserve the existing all-or-nothing stock mutation order.
- Refresh `docs/ISLAND_SYSTEM_STATUS.md` to 2026-11-06 and record the standalone fishery model's current boundary: implemented, not yet wired into IslandLife or sourced marine stocks.

## Regression coverage

- Uses a finite high-resilience stock at half of `f64::MAX` and a finite `f64::MAX` time step, causing only the unconstrained interval-growth result to overflow.
- With maximum requested catch, harvest is capped at finite remaining headroom and the stock returns to its starting biomass.
- With zero requested catch, the stock reaches exactly its finite carrying capacity without a non-finite state.
- Existing test still rejects an annual surplus that is itself unrepresentable.

## Verification

Exact-head CI is required for formatting, Clippy, fast tests, desktop/backend, and week/year reference-pack acceptance. No local Cargo test pass is claimed in this run. The change does not wire fishery behaviour into IslandLife, change reference constants, snapshots, event logs, replay hashes, RNG streams, ledger flows or live simulation state.

## Next

1. Verify the exact PR head on all required CI jobs.
2. Continue the marine biomass bridge in PRs #92 and #93; those read-only projections do not establish a calibrated fishery.
3. Source species-specific resilience, unexploited carrying capacity and catch-per-unit-effort before connecting `FishStock` to IslandLife and human-chosen fishing effort.
