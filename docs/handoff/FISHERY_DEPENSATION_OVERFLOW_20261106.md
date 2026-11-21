# Fishery depensation overflow

Date: 2026-11-06
Repository: `dylmarriner/Maer-ken-island`
Branch: `fix/fishery-depensation-overflow-20261106`
Base: `main` after PRs #89 and #90 were merged.

## Defect

`surplus_t_per_year()` calculated `r * (1 - B/k) * B` before applying
the low-biomass depensation factor `B / (k * threshold)`. For a valid finite
model and stock, the unscaled intermediate could overflow even when applying
the depensation factor first yields a finite, representable final surplus.
That incorrectly returned `FisheryError::NonFiniteGrowth`.

## Change

- In the depensation branch, combine the bounded dimensionless factors first:
  `(1 - p) * (p / depensation_below)`.
- Scale biomass by that factor before multiplying by the growth rate.
- Preserve the existing non-depensated expression for stocks at or above the
  threshold.
- Add a regression with finite extreme inputs where the old intermediate
  overflows but the mathematically depensated surplus is approximately
  `3.96e307`.

## Invariants

No reference data, fishery constants, event logs, replay hashes, snapshot
fields, scheduler cadence, live island state or external systems are changed.
The public error contract and stock mutation boundary remain unchanged.

## Verification

Source and regression are committed. Exact-head CI must run formatting,
Clippy, fast tests, desktop/backend, dashboard, computer service, dependency
audit and week/year reference acceptance. Custom OpenCommander currently
returns HTTP 429, so no local Cargo result is claimed.

## Next

1. Run exact-head CI and fix only evidence-backed failures.
2. Keep this PR draft until all required jobs pass.
3. Then continue wiring the fishery model into IslandLife with typed errors
   propagated at the simulation step boundary and ledger-balanced catches.

No production service, live island state, owner data or external system changed.
