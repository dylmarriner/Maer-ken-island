# Fishery MSY biomass target — 6 November 2026

Repository: `dylmarriner/Maer-ken-island`
Branch: `feat/fishery-msy-biomass-target-20261106`
PR: #101 (to be opened)

## Finding

After the piecewise MSY correction, the fishery model could report the maximum sustainable surplus in tonnes/year but not the biomass at which that maximum occurs. A future harvest-decision layer needs both values; otherwise it cannot distinguish a stock below, at or above the model's sustainable-production peak.

## Change

- Centralise the piecewise peak calculation as one helper returning both the peak biomass fraction and peak surplus fraction.
- Keep `msy_t` on the same formula and preserve its existing result.
- Add read-only `FishStock::msy_biomass_t`, returning the biomass target in tonnes.
- Validate the model and stock state with the same fail-fast typed errors used by `msy_t`.
- Correct the module-level documentation, which previously implied `rK/4` at `K/2` for every allowed depensation threshold.
- Do not change FishStock serialization, snapshots, simulation state, harvest behaviour, material ledgers or replay hashes.

## Regression coverage

- Extend the piecewise peak test to include both exact branch boundaries, (d=0.5) and (d=2/3), plus the existing low/high thresholds.
- Assert the new biomass target matches the analytic peak for every threshold.
- Verify the target remains finite for an extreme but valid carrying capacity.
- Verify invalid model, carrying capacity and stock state return the existing typed errors.

## Commits

- Implementation and regression: `4ba8c01fc3d95aafffb7231e34b2fd690a4c0d4e`.
- Follow-up extreme-capacity assertion correction: `e356c66bbb392069c249308ebc48904fba879b5b`; the target is one quarter of `f64::MAX` when K is one half of `f64::MAX` and the default peak fraction is one half.
- Remaining-work ledger: `b830f170886ad7407df2d07ee319d1f8b8433518`.
- Root handoff: `bac57bdc74cbd63d05551af5faf831795642619f`.

## Verification

- GitHub file writes succeeded on the new branch created from main at `55851f7a10d6ab3653b5ef013e6eba8de8e13c52`.
- Exact-head hosted CI is required for formatting, Clippy, fast Rust tests, desktop, installed backend and week/year reference-pack acceptance.
- Custom OpenCommander device discovery returned HTTP 429, so no local Cargo test is claimed.
- Keep the PR draft until the exact-head required jobs pass.

## Remaining integration boundary

This is a diagnostic API only. It does not create a fishery stock from an island species, assign carrying capacity or resilience, infer CPUE, harvest fish, move food into material stocks or connect fishing labour to IslandLife. Those require cited species/effort data and the workforce choices already identified in `WORK_REMAINING.md`.

## Safety

No sourced reference values were added or changed. No live simulation, persistence format, owner decision, deployment, external system or budget was changed.
