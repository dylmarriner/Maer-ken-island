# Fishery subnormal intermediate surplus handoff

Date: 2026-11-07
Repository: `dylmarriner/Maer-ken-island`
Branch: `fix/fishery-subnormal-intermediate-surplus-20261107`
Base: `main` after PR #114 merge `78f30aacaa733fe46fe6571f0751e0e100ed9c34`

## Defect

The low-biomass depensation path evaluated:

`rate * (biomass * ((1 - p) * (p / depensation)))`

where `p = biomass / carrying_capacity`.

The previous log-space fallback handled only `p == 0`. A representable subnormal `p` can still leave the subsequent `biomass * factor` intermediate at zero even when the final mathematical surplus is representable. That silently reported no recruitment for a positive stock.

## Change

- Compute the direct result first.
- If the direct result is zero while biomass and growth rate are both positive, recompute the equivalent Schaefer expression in log space.
- Include the `(1 - p)` factor in the log expression.
- Preserve the ordinary direct path for nonzero results and for zero biomass/rate.
- No model parameters, reference data, snapshots, replay hashes, RNG, ledgers or IslandLife integration changed.

## Regression

`low_biomass_surplus_uses_log_space_when_intermediate_product_underflows` constructs:
- carrying capacity `1.0e308`,
- biomass `5.0e-16`,
- medium-resilience growth rate `1.0e308`,
- depensation threshold `0.25`.

The test proves `B/K` and the depensation factor are positive, `B * factor` underflows to zero, and the final surplus is positive and agrees with the log-space result within `1e-12` relative error.

## Commits

- `05c2f4c357518fc8956e9bd0d2ba2379cd94df77` — source fix and regression.
- `4422c0130db3f086d6bd458d425d09adf2e308db` — roadmap/handoff status reconciliation.

## Verification

- Local Cargo tests were not run; custom OpenCommander device discovery has returned transient MCP 429/404 errors.
- First exact-head workflow #42307400642 identified only rustfmt differences in the new source/test lines; those exact formatter changes were applied in `302dcd59`. The first run's other jobs are not evidence for the new head.
- A fresh exact-head GitHub Actions run is required for formatting, Clippy, fast Rust tests, desktop, installed backend, dashboard, computer service, scripts, dependency audit and week/year reference-pack acceptance.
- Do not merge until the exact branch head passes all required jobs.

## Next work

1. Verify exact-head CI.
2. Merge only if all required checks pass and the head remains unchanged.
3. Continue sourced fishery-to-IslandLife integration only after species-specific resilience, unexploited biomass and catch-per-unit-effort inputs are supported by cited reference data and workforce decisions remain person-owned.
