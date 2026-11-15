# Marine species stack reconciliation — 6 November 2026

Repository: `dylmarriner/Maer-ken-island`
Branch: `fix/marine-species-stack-reconciliation-20261106`
PR: #102
Base: `main` after fishery PR #101 merged at `373a1b71517b82352b40208e4546bc0d3e4d24a1`

## Finding

The marine species lookup/index work was split across PRs #95, #96 and #98. PR #95 had diverged from current main after the fishery work landed, while #96 and #98 were stacked on earlier #95 heads. The stack was therefore stranded behind a dirty parent branch.

## Reconciled implementation

- Keep the species catalogue private and expose read-only `species()`.
- Route catalogue changes through `replace_species_catalogue`, rebuilding marine membership and producer/grazer weights before the replacement is visible.
- Validate unique `species_id` values during survey and controlled replacement; a rejected replacement leaves the catalogue and derived cache unchanged.
- Keep the marine aquatic index identity-checked and metadata-checked.
- Keep the recoverable `try_marine_aquatic_nodes` projection checks for cell bounds, cell area, marine abundance, finite non-negative density, positive finite body mass and finite population/biomass conversion.
- Validate shared cell area before the empty-habitat early return, so an empty cell cannot hide invalid geometry.
- Preserve the compatibility projection's fail-fast behaviour for invalid derived state.

## Regression evidence added

- Retain integration coverage for category/affinity changes, newly eligible aquatic species, catalogue atomicity, duplicate-ID rejection and producer/grazer cache rebuilding.
- Add a unit regression: zero cell area plus empty habitat returns `InvalidCellArea`; positive area plus empty habitat remains `Ok(Vec::new())`.

## Invariants

The catalogue and marine index remain derived, not persisted. No fishery calibration, carrying capacity, resilience, CPUE, catch, workforce decision, simulation tick, RNG stream, snapshot layout, append-only log, replay hash, material ledger or external system is changed. No unsourced physical values are introduced.

## Verification

- Source is based on the existing #95/#98 implementation and the #96 cell-area correction, applied to current main after PR #101 merged.
- Exact-head GitHub Actions must pass formatting, Clippy, fast Rust tests, desktop/backend builds, web dashboard, scripts, dependency audit and week/year reference-pack acceptance.
- Custom OpenCommander returned HTTP 429 in this run; no local Cargo test pass is claimed.
- Keep PR #102 draft until its exact head passes required CI. Superseded PRs #95/#96/#98 should only be closed after #102 merges.

## Next

After this stack is merged, continue sourced marine ecology/stock calibration and the fishery-to-IslandLife integration. Do not infer carrying capacity or CPUE from the current upstream-density projection.
