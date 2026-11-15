# Fishery unrepresentable surplus and Schaefer intermediate underflow

Date: 2026-11-07
Repository: `dylmarriner/Maer-ken-island`
Branch: `fix/fishery-unrepresentable-surplus-20261107`
Stacked on PR #117 at `b2ae59600cbd34b20faf34bd65a9bda3c5474154`.

## Defects

The non-depensated Schaefer path evaluated `r * (1 - B/K) * B` left-to-right. A subnormal positive `r * (1 - B/K)` could round to zero before a large biomass made the final mathematical surplus normally representable. The method also accepted a zero result when positive rate and sub-capacity biomass imply a mathematically positive surplus smaller than the smallest binary64 subnormal, contradicting its documented zero semantics.

## Change

- Re-evaluate the non-depensated expression in log space when its rate/fraction intermediate is subnormal or the direct result is zero.
- Add `FisheryError::UnrepresentableGrowth` when positive rate and positive biomass below carrying capacity produce a mathematical surplus that cannot be represented as a positive binary64 value.
- Preserve zero for genuine zero-rate, zero-biomass or at-capacity states.
- Keep the mutation boundary in `fish()`: the error is returned by the read-only growth calculation before biomass or landed totals change.

## Regression coverage

1. A minimum-subnormal growth rate multiplied by a 0.5 remaining fraction rounds to zero, but multiplying the mathematical result by a large biomass yields a normal, representable surplus. The test compares the production result to a log-space reference within 1e-12 relative error.
2. Both the depensated and ordinary Schaefer branches reject a positive mathematical surplus below binary64 range with `UnrepresentableGrowth`; `fish()` leaves the entire stock unchanged.

## Commits

- `daff952a8a8115bfc28754fa8eb8397fd77bd743` — production correction and two regressions.
- `470cfc6f37b17d55f9a9e63f26b0ed2baddca60f` — rustfmt alignment.

## Verification

- Exact-head CI is required on the final PR #118 head.
- No local Cargo result is claimed because custom OpenCommander access is currently rate-limited.
- PR #117 remains a separate parent change; this branch is stacked on its exact head and must be retargeted/reverified after #117 merges.

## Invariants

No reference values, species parameters, snapshot fields/layout, append-only logs, replay hashes, RNG, material ledger, IslandLife state or workforce assignments changed. Fishery remains standalone pending sourced species-specific stock/effort data and person-owned workforce decisions.
