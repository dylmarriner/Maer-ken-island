# Fishery MSY under low-biomass depensation — 6 November 2026

Repository: `dylmarriner/Maer-ken-island`  
Branch: `fix/fishery-msy-depensation-main-20261106`  
Base: `main` at `1d70e9e2941675aef4d10fb8f94816cfe4610c70` (PR #97 merged)

## Finding

`FisheryModel::is_valid` permits depensation thresholds throughout `0..=1`, but `FishStock::msy_t` always returned `rK/4`. That is correct only when the depensation threshold `d` is at or below `0.5`.

For `d > 0.5`, the ordinary Schaefer peak at `B = K/2` lies inside the depensated region. The actual maximum moves:
- to `B = dK` for `0.5 < d <= 2/3`;
- to `B = 2K/3` for `d > 2/3`.

The old method therefore overstated the maximum sustainable surplus for valid models with higher thresholds.

## Change

The MSY calculation now uses the analytic maximum of the same piecewise surplus function used by `surplus_t_per_year`:

- `d <= 0.5`: peak fraction `1/4`;
- `0.5 < d <= 2/3`: peak fraction `d(1-d)`;
- `d > 2/3`: peak fraction `4/(27d)`.

The bounded biomass fraction is applied to `K` before multiplying by `r`, preserving overflow-aware arithmetic. The default reference threshold (0.25) retains the existing `rK/4` result.

## Regression coverage

The test checks thresholds 0.25, 0.6, 0.75 and 1.0. For each it:
- evaluates the analytic peak biomass;
- asserts the reported MSY equals the model's surplus at that peak;
- samples 1,001 biomass fractions and asserts no point on the piecewise curve exceeds the reported MSY.

## Commits

- Fishery implementation and regression: `c617b9f232e0b8bb73246bac6ba623586a6036b1`.
- Remaining-work checklist: `a1ed7beba24cfcb0e5955e587f21b3181240c482`.
- System-status evidence: `8f085da6125bfb10aad0614009918a8261549f9a`.

## Verification

- This branch starts from main after PR #97's subnormal-safe surplus correction was merged at `1d70e9e2941675aef4d10fb8f94816cfe4610c70`.
- No local Cargo test is claimed because custom OpenCommander device access returned HTTP 429.
- Exact-head CI must pass formatting, Clippy, fast Rust tests, backend/desktop builds and required week/year acceptance before merge.

## Invariants

The default depensation threshold and default MSY are unchanged. No stock capacity, resilience class, CPUE, catch, reference value, species density, simulation state, RNG, snapshot, append-only log, material ledger, replay hash or digest was changed. The fishery remains standalone and is not connected to IslandLife until sourced species-specific stock and effort data are available.
