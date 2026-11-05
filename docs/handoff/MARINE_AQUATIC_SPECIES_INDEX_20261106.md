# Marine aquatic species lookup index

Date: 2026-11-06
Repository: `dylmarriner/Maer-ken-island`
Branch: `perf/index-marine-aquatic-species-20261106`
Base: `main` at `7bc452d1b6bced84cdc6c8f1515b726a495456b4`.

## Defect

`IslandBiota::marine_aquatic_nodes(cell)` scanned all 4,096 founding species for every cell-inspector request, even though only the OpenOcean/ReefSea aquatic subset can contribute to this projection. The result was correct, but the request path repeatedly paid the full species scan.

## Change

- Build a stable, species-order vector of eligible marine aquatic species indices once during `IslandBiota::survey`.
- Use that index for per-cell marine-node lookup, then apply the cell's existing habitat mask.
- Preserve species ordering and the exact eligibility predicate: aquatic category plus OpenOcean or ReefSea affinity.
- Keep the index derived in memory only. It is not persisted, serialized, included in snapshots, event logs, replay hashes or the canonical digest.

## Regression coverage

- New unit test verifies the index includes only OpenOcean and ReefSea aquatic species and preserves original species order.
- Existing `marine_aquatic_nodes_project_population_to_biomass_without_a_second_stock` integration test still walks every medium cell and checks population conservation, biomass conversion, habitat membership and absence of non-marine species.

## Verification

- Source commit: `70bb8d65e854b1c91844b72430a6738addfa2225`.
- No local Cargo test pass is claimed. Exact-head CI must pass formatting, Clippy, fast tests, desktop/backend builds, and week/year reference acceptance before merge.
- This is a query-path optimization only; no reference data, fishery calibration, harvest, ledger, random stream, LLM behaviour or live island state changes.

## Next

1. Verify the exact PR head on all required CI jobs.
2. Merge only after review and all required checks pass.
