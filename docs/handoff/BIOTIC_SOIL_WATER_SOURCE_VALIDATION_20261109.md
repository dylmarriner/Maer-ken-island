# Biotic and soil-water source validation — 9 November 2026

Repository: `dylmarriner/Maer-ken-island`
Branch: `fix/material-source-amount-boundaries-20261109`
Base: PR #129 squash merge `170e4b5f1c035bf71535294fa758bbd4b661a22b`

## Previous milestone

PR #129, material amount validation and atomic crafting, was merged after exact-head workflow #42473464844 passed all required non-nightly jobs. Formatting, Clippy, fast Rust tests, dependency audit, computer service, installed backend, desktop application, dashboard, repository scripts, and both week/year reference-pack acceptances passed. Nightly slow tests were skipped by design.

Merge: `170e4b5f1c035bf71535294fa758bbd4b661a22b`.

## Finding

The material amount audit still had source paths that silently clamped negative requests and could mutate ecological/hydrological sources before discovering that the destination stock would become non-finite:

- `gather_biotic` clamped negative/non-finite requests through `max(0.0)`.
- `gather_edible` had its own clamp and bypassed `gather_biotic` when the edible layer existed.
- `gather_soil_water` clamped invalid requests and drained soil before adding the water to material stock.

## Change

- Apply the shared finite, non-negative amount validation to biotic, edible and soil-water gathering.
- Remove the silent negative-quantity clamps from those paths.
- Preflight resulting Wood, PlantFood and Water stock before changing biomass, edible availability, soil storage or flux entries.
- Preserve existing zero-request/empty-source behavior and valid positive gathering semantics.
- No snapshot schema, append-only action log, replay hash, RNG, tick cadence, reference data or physical constants changed.

## Regression coverage

- Negative, NaN, positive-infinity and negative-infinity requests are rejected by all three paths.
- Invalid requests preserve biomass, edible availability, soil storage, material stock and flux-ledger entries.
- A biotic harvest that would overflow existing Wood stock is rejected before biomass changes.
- A soil-water transfer that would overflow existing Water stock is rejected before soil is drained.

## Commits

- Production: `ef4368bcf5fdf5ad5d915f70d8068148ae257901`.
- Regression tests: `4e2b3cc80fe9492161e8b026b51e1ecebc43af93`.

## Verification

Exact-head CI is required for formatting, Clippy, fast Rust tests, desktop, installed backend, dashboard, dependency audit, computer service, repository scripts and week/year reference-pack acceptance. The new PR must remain draft until its own exact-head checks finish green.

Custom OpenCommander device discovery returned HTTP 429, so no local Cargo result is claimed.

## Next

- Verify this branch's exact head.
- Continue auditing public material source/sink operations with typed errors and pre-mutation checks.
- Then advance D40 two-person saw cross-cutting with separate founder time/energy accounting.
- Keep deterministic single-writer simulation state, append-only logs, replay-hash equality, optional LLM behavior and offline/budget constraints unchanged.
