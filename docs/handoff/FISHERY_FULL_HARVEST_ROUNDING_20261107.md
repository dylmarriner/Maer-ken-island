# Fishery full-harvest rounding — 7 November 2026

## Finding

When a caller requests all interval growth, the fishery correctly caps catch at growth, but the previous biomass calculation evaluated:

`(biomass + grown) - catch`.

For large stocks, floating-point cancellation can round that result one ULP below the original biomass even when `catch == grown`. The model then reports an apparent stock loss caused solely by arithmetic ordering.

A deterministic reproducer uses:
- biomass = `2^62`;
- carrying capacity = `2 × biomass`;
- medium resilience rate = `0.0012/year`;
- depensation threshold = `0.25`;
- interval = `YEAR_S`.

The old expression loses 512 tonnes at this scale despite harvesting exactly the calculated growth.

## Implementation

`FishStock::fish` now computes `remaining_growth = grown - catch` first, then adds only that remainder to the original biomass. A full-growth harvest therefore adds exactly zero to biomass; a partial harvest adds only the unharvested growth. The existing finite-growth and carrying-capacity checks remain in place.

## Regression

Added `harvesting_all_interval_growth_does_not_reduce_biomass_from_rounding`, which compares the read-only sustainable-catch limit with the actual catch and proves that:
- the full requested catch is capped to the reported growth;
- biomass is bit-for-bit unchanged when all growth is harvested;
- cumulative landed biomass increases by exactly the catch.

## Verification and safety

- Implementation and regression are committed on PR #109's branch.
- The preceding exact-head workflow [#42213034085](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42213034085) passed formatting, Clippy, fast Rust tests, desktop, installed backend, web dashboard, computer service, repository scripts, dependency audit and week/year reference-pack acceptance.
- A new exact-head workflow is required for this follow-up.
- No reference values, stock parameters, snapshot/replay formats, hash baselines, live simulation state, budget or offline limits were changed.
- Fishery remains standalone pending sourced species-level stock/effort data and human-chosen workforce integration.
