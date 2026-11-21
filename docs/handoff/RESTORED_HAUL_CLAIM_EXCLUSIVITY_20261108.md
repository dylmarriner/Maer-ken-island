# Restored active-haul claim exclusivity — 8 November 2026

Repository: `dylmarriner/Maer-ken-island`
PR: https://github.com/dylmarriner/Maer-ken-island/pull/128
Branch: `fix/timber-haul-log-validation-20261108`

## Finding

`IslandLife::restore` validated each persisted `HaulJob` independently but did not validate conflicts across the active-haul map. A malformed snapshot could assign the same source log to two founders, or assign the island's single ute or single excavator to two simultaneous jobs. New planning already prevents these conflicts; restore must enforce the same invariant before exposing the world.

## Change

- Reject duplicate active source-log IDs across restored haul jobs.
- Reject more than one active Ute job.
- Reject more than one active Excavator job.
- Continue to allow multiple independent OnFoot jobs.
- Return the existing typed `LabourError::InvalidHaulJobState` before restoring the world.

Commits:
- Production invariant: `686c70d2bfbd2db8bbe7cdc9cc9f0f57f6aaa7fd`.
- Regression tests: `3cfdf01960bc5544ae9152ef7e0e4a1a59f398fa`.

## Regression coverage

- Two individually valid OnFoot jobs claiming the same log are rejected.
- Two different logs cannot both claim the single Ute.
- Two different logs cannot both claim the single excavator.
- Existing single-job and independent OnFoot behavior remains unchanged.

## Compatibility and determinism

- No snapshot fields or version changed.
- No replay log, hash, RNG, tick cadence, material stock, reference data or real-world parameter changed.
- The check runs only at restore and follows the exclusivity rules already used by new haul planning.

## Verification

- The preceding PR #128 head `fe05a6f6baebff9f8e9b15d5715832c9c2081377` passed Format, Clippy, installed-backend and week/year reference-pack checks while fast tests and desktop interaction acceptance were still running.
- The new production/test commits require a fresh exact-head workflow. No pass is claimed for this new head.
- Custom OpenCommander remains rate-limited; local Cargo execution was not available.

## Next

Verify exact-head formatting, Clippy, fast Rust tests, desktop/backend and week/year acceptance. Then implement D40's two-person saw workflow with separate founder participation and energy accounting.
