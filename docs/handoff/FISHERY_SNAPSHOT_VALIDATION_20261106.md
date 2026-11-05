# Fishery snapshot validation

Date: 2026-11-06
Repository: `dylmarriner/Maer-ken-island`
Branch: `fix/fishery-validate-deserialized-stock-20261106`
Stacked on: `fix/fishery-typed-errors-20261105` at `2b98fd2499a19bc1980f57c17056186b8c21e733` (PR #89)

## Defect

`FishStock` derived `Deserialize` directly. A persisted or imported stock with non-positive/non-finite carrying capacity, biomass outside `[0, k]`, or negative/non-finite landed total could therefore be materialised as an invalid model object. The typed fishery methods would reject it later, but the invalid state crossed the restore boundary.

## Change

- Retain the same four serialized fields and field order.
- Deserialize through a private wire struct, then call the shared `validate_state()` before returning a `FishStock`.
- Convert invalid stock state into a serde deserialization error; do not clamp, repair or silently zero corrupt values.
- Keep unknown-field handling compatible with the prior derived deserializer.

## Regression coverage

- Valid JSON round-trip asserts the exact existing field names and values.
- Valid bincode round-trip asserts the binary field order remains compatible.
- Invalid carrying capacity, biomass above capacity, negative biomass and negative landed total are rejected at deserialization.

## Commit

- `f5d92368249ba096c5dd814e9db8c3a62ffcb4cb` — validate fish stock snapshots before restore.

## Verification

- Source and regression tests are committed.
- No local Cargo test or formatter result is claimed; custom OpenCommander device discovery returned HTTP 429.
- PR #89's exact-head workflow #42019071845 has passed formatting, Clippy, week/year reference-pack acceptance, installed backend, repository scripts, web dashboard, computer service and dependency audit; fast Rust tests and the desktop application's remaining acceptance steps were still running at last inspection.
- This PR is stacked on #89 and must not merge independently. Run exact-head formatting, Clippy, fast tests, desktop/backend, and week/year reference-pack acceptance after the parent branch is reconciled.

## Invariants

No fishery constants, reference data, event logs, replay hashes, snapshot field names/order, simulation tick, LLM behavior or live island state are changed. Invalid persisted stock state now fails closed instead of being accepted and deferred.

## Next

1. Finish and merge PR #89 only after all required checks pass.
2. Rebase this branch onto the merged parent and run its own exact-head checks.
3. Propagate typed fishery errors at the eventual runtime integration boundary; do not convert them back to zero catch.


## Parent arithmetic correction synchronized (2026-11-06)

- PR #89 exposed and fixed an avoidable intermediate overflow in annual growth scaling: `surplus * dt_s / YEAR_S` is now `surplus * (dt_s / YEAR_S)`.
- Parent source/test commit: `18a36a1b40bbad5a9abc91e76dad4506023515b2`; parent handoff commit: `83e6145da6c9911365ef4c9d1ee85c69f309f755`.
- This branch now carries the corrected parent fishery source plus the snapshot-deserialization validation, so the next exact-head run covers both changes together.
- Parent exact-head workflow [#42021998151](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42021998151) is running; formatting has passed and Clippy, fast tests, desktop/backend and week/year acceptance were still in progress at inspection.
- PR #90 remains stacked on PR #89. Do not merge until the parent passes and is merged; then retarget/rebase and rerun this branch.


## Binary non-finite regression (2026-11-06)

- Added a bincode fixture containing positive-infinite carrying capacity and asserted that custom deserialization rejects it. JSON cannot represent NaN/infinity, so the earlier JSON cases alone did not prove the binary snapshot boundary.
- Test commit: `ee24cd0b069525da79f4c65ae17ef94f42d54f97`.
- The new exact-head run must include this commit; the previous #90 run predates it.


## Exact-head rustfmt correction (2026-11-06)

- Workflow [#42022708165](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42022708165) checked out the correct PR #90 merge ref and found one formatter-only line-wrap mismatch in the JSON round-trip assertion. Clippy and fast tests were skipped by the format gate.
- Applied the exact one-line rustfmt output in commit `09846ca352e24b65751d7b89814c1752cab3e7eb`.
- The failed run is not a pass. A fresh exact-head workflow is required for this corrected head, including the binary non-finite regression.


## Parent growth-scaling refinement synchronized (2026-11-06)

- PR #89 refined annual-growth scaling again after identifying that the simple ratio-first expression could underflow a subnormal time interval while the final growth remained representable.
- Current parent implementation divides whichever factor is at least one year before multiplying; when both are below one year, their product is bounded and is divided afterward.
- Parent source/test commit: `f92c6103ee665026d6774003b4d704576a83ff36`; parent handoff: `767d4333efd4c614fb71e14195b3d85a809f5159`.
- This branch now includes the robust parent implementation and its tiny-time-step regression plus the snapshot validation and binary non-finite regression. Combined source/test commit: `f54008fb62dfca6db8cec9f830cb45d87ce31ab2`.
- A fresh exact-head workflow is required for the refreshed PR #90 head; the prior #42022876589 run predates this latest parent refinement.
