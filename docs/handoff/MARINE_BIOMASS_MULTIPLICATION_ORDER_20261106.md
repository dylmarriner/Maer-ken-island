# Marine biomass multiplication order — 6 November 2026

## Scope

Avoid a false overflow in the checked marine population-to-biomass projection.

The projection converts individuals × body mass (kg) into tonnes. Multiplying the full kilogram values before dividing by 1,000 can overflow an intermediate `f64` even when the final value in tonnes is finite.

## Implementation

- Scale body mass to tonnes first, then multiply by the individual count.
- Retain the existing finite/non-negative validation on the final biomass.
- Do not clamp or silently saturate an actually unrepresentable final biomass.
- No simulation state, stock, carrying capacity, harvest, RNG, snapshot, event log, ledger or replay hash changes.

## Regression

A deliberately extreme but finite input uses (10^{308}) individuals and 10 kg per individual. The intermediate kilogram product would overflow, while the mathematically equivalent final biomass is (10^{306}) tonnes and remains representable. The test asserts that the checked projection returns the finite result.

## Commit

- `3af9f0e234bcd4139eb6bcdcc4d7aa35b0b43ce0` — scale before multiplication and add regression.

## Verification

- GitHub file update succeeded on the existing PR #95 branch.
- No local Cargo test was run because custom OpenCommander device discovery is returning HTTP 429.
- The prior exact-head workflow [#42073656984](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42073656984) had passed formatting, Clippy, both week/year reference-pack acceptance tests, the installed backend, computer service, dashboard, repository scripts and dependency audit; fast Rust tests and the desktop application were still running at the last inspection.
- This new commit requires a fresh exact-head workflow. No result from the prior head certifies this change.

## Safety

No live island state, simulation parameters, real-world constants, upstream pin, replay input or digest was changed.
