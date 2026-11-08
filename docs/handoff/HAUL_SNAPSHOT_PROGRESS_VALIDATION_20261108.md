# Persisted timber-haul progress validation — 8 November 2026

Repository: `dylmarriner/Maer-ken-island`
PR: #128
Branch: `fix/timber-haul-log-validation-20261108`

## Finding

`IslandLife::restore` accepted the persisted `hauling` map without validating its progress cursor or task inputs. A malformed snapshot could therefore restore NaN/out-of-range progress, non-finite task durations/positions, or an overflowing cross-cut ID range and defer the failure until simulation execution.

The live haul loop also subtracted the completed task duration after allowing a 1 ns completion tolerance. A task that completed inside that tolerance could leave a tiny negative cursor.

## Change

- Added `HaulJob::has_valid_persisted_state` to validate active job identity, non-empty task queues, finite bounded progress, finite non-negative durations/METs, finite task destinations and action-specific identifiers/quantities.
- `IslandLife::restore` now rejects malformed persisted haul jobs with typed `LabourError::InvalidHaulJobState` before exposing the restored world.
- Completed-task subtraction clamps only the tolerance-sized negative remainder to zero while preserving positive carry-over into the next task.
- Added snapshot regressions for NaN progress, progress beyond the front task, infinite duration, non-finite destination coordinates and cross-cut ID overflow.

## Compatibility and determinism

- No snapshot fields or version changed.
- No replay log, hash, RNG, reference data, material stocks or task scheduling cadence changed.
- Existing snapshots with a negative progress remainder within the historical 1 ns completion tolerance remain accepted; the runtime now normalizes that remainder on the next task completion.

## Verification

- Exact-head GitHub Actions must pass formatting, Clippy, fast Rust tests, installed backend, desktop acceptance and week/year reference packs.
- The previous exact-head workflow was still running fast tests when this slice began. This commit supersedes that run and requires fresh exact-head verification.
- Custom OpenCommander is required for local Cargo execution; no local test result is claimed.

## Next

- Finish D40: source-backed saw-based log cross-cutting for the ute and estate routes.
- Continue D31 regional soil nutrient modelling only with sourced nitrogen/phosphorus data and explicit deviations.
