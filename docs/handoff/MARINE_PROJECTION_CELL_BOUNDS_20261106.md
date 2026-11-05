# Checked marine projection cell bounds

Date: 2026-11-06
Repository: `dylmarriner/Maer-ken-island`
Branch: `perf/index-marine-aquatic-species-20261106`
Base PR: #95, marine aquatic species index and validated projection.

## Defect

The checked marine projection used `habitat.get(cell).unwrap_or(0)`. An out-of-range cell therefore looked identical to a valid cell with no marine habitat and returned an empty projection. This hid caller indexing defects instead of returning a typed validation error.

## Change

- Add `MarineProjectionError::InvalidCellIndex { cell }`.
- Make `try_marine_aquatic_nodes` reject a cell outside the habitat grid before evaluating its habitat mask.
- Preserve the old infallible `marine_aquatic_nodes` compatibility behavior: out-of-range cells still return an empty vector.
- Add a regression for both checked and compatibility paths.

## Invariants

No simulation state, snapshot, append-only event log, RNG stream, replay input/hash, ledger, ecology parameter or LLM behaviour changes. The index remains derived and read-only.

## Verification

Regression is committed. Cargo tests were not run in this environment; exact-head GitHub Actions must complete before merge. No test pass is claimed.
