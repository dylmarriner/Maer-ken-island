# Fishery MSY under low-biomass depensation — 6 November 2026

Repository: `dylmarriner/Maer-ken-island`  
Branch: `fix/fishery-msy-depensation-20261106`  
Parent: PR #97, subnormal-safe fishery surplus

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

The bounded biomass fraction is applied to `K` before multiplying by `r`, preserving the existing overflow-aware arithmetic. The default reference threshold (0.25) retains the existing `rK/4` result.

## Regression coverage

The new test checks thresholds 0.25, 0.6, 0.75 and 1.0. For each it:
- evaluates the analytic peak biomass;
- asserts the reported MSY equals the model's surplus at that peak;
- samples 1,001 biomass fractions and asserts no point on the piecewise curve exceeds the reported MSY.

## Commits

- Fishery implementation and regression: `571b33603c3f86ea9546d39561a84d24eb4a6d9b`.
- Remaining-work checklist: `39a695a5e6f12219b4da73407c55fe91c7634500`.
- System-status evidence: `70825e113f702ca87c9b11ab1a227f4eb64b33de`.

## Verification

- Source and regression are committed on a branch stacked on PR #97.
- No local Cargo test is claimed because custom OpenCommander device access returned HTTP 429 during this run.
- Exact-head CI must pass formatting, Clippy, fast Rust tests, backend/desktop builds and required week/year acceptance before merge.

## Invariants

The default depensation threshold and default MSY are unchanged. No stock capacity, resilience class, CPUE, catch, reference value, species density, simulation state, RNG, snapshot, append-only log, material ledger, replay hash or digest was changed. The fishery remains standalone; IslandLife integration still requires sourced marine stock and effort data.
