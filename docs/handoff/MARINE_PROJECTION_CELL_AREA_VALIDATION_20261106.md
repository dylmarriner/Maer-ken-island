# Marine projection cell-area validation — 6 November 2026

Repository: `dylmarriner/Maer-ken-island`  
Branch: `fix/marine-projection-area-validation-20261106`  
Stacked on PR #95: marine aquatic species index and checked projection.

## Finding

`try_marine_aquatic_nodes` checked the habitat mask and returned an empty vector for a valid non-marine cell before validating the projection's shared `cell_area_m2`. A non-finite or non-positive area therefore escaped the typed `InvalidCellArea` invariant whenever the queried cell had no marine habitat.

Cell area is a projection-wide physical input, not a property of a particular habitat mask. An empty cell must not conceal invalid global geometry.

## Change

- Keep the cell-index bounds check first.
- Validate `cell_area_m2` immediately after resolving the cell mask and before the empty-habitat early return.
- Preserve `Ok(Vec::new())` for a valid-area cell with no marine habitat.

## Regression

- Zero cell area plus an empty habitat returns `MarineProjectionError::InvalidCellArea`.
- Positive cell area plus an empty habitat remains an empty successful projection.

## Commit

- `23c2571d362fc8c0b9853f905abe03eea5883744` — validate shared cell area before habitat short-circuit.

## Verification

- Source and regression committed on the stacked PR #96 branch.
- No local Cargo test was run because custom OpenCommander device discovery continues to return HTTP 429.
- Exact-head GitHub Actions must pass formatting, Clippy, fast Rust tests, backend/desktop builds and required week/year acceptance before merge.
- PR #95's existing exact-head workflow remains separate and is not cancelled by this stacked branch.

## Invariants

No simulation state, species abundance, RNG, snapshot, append-only log, replay hash, material ledger, reference pack, ecological parameter or live island state changed.
