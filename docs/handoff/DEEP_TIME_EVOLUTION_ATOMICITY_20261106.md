# Deep-time evolution atomicity — 6 November 2026

**Repository:** `dylmarriner/Maer-ken-island`  
**Branch:** `fix/deep-time-evolution-atomicity-20261106`  
**Base:** PR #106 branch `fix/deep-time-geographic-range-fraction-20261106-rebased`  
**Stacked PR:** #107 (draft)

## Finding

`step_evolution_processes` advanced the evolution clock and step index, then mutated carrying capacities/populations/genomes before invoking fallible mutation and geographic-drift phases. If a later phase returned an error, the caller received a failed step while part of the step had already become visible. That violates the engine's fail-fast deterministic transition boundary.

## Change

- Stage `DeepTimeEvolution`, the species vector and `RngRegistry` on clones.
- Execute the existing ordered evolution pipeline against staged values.
- Commit all three mutable inputs only after every fallible phase succeeds.
- Preserve phase order, keyed RNG derivation, clock arithmetic, species ordering, snapshot schema and append-only log behavior.
- No LLM call, new ecological parameter or external side effect.

## Regression

A deterministic seed search uses a maximal finite horizon to force the late geographic-range drift validator to reject a negative candidate after population growth has changed the staged population. The test asserts the original evolution state, species vector and RNG registry remain byte-for-byte identical after the rejected transition.

Existing normal-step deterministic and fractional-range tests remain in place.

## Commits

- `bdc97507` — stage and atomically commit evolution transitions.
- `febdf236` — late-failure atomicity regression.
- `63d330ed` — exact rustfmt correction from the first PR workflow.

## Verification

- Exact-head GitHub Actions is required. The PR is stacked on #106 because the current range validator's fallible drift path is part of that branch.
- First exact-head workflow [#42171306926](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42171306926) failed at `cargo fmt --all -- --check` only; the log identified one multiline call and commit `63d330ed` applies the exact rustfmt output. Clippy and fast tests did not run in that attempt.
- No local Cargo result is claimed: custom OpenCommander device discovery returned HTTP 429.
- The latest parent PR #106 workflow is still running its Rust fast tests, desktop and week/year acceptance; this follow-up needs its own exact-head CI after PR #107 is opened.
- The staged approach clones the current species vector and evolution state per step. Review runtime impact under the repository's long-horizon profile before merge; correctness is the immediate goal, and the performance cost must be measured rather than guessed.

## Safety

No live simulation state, canon, physical parameters, snapshot format, production infrastructure or external service was changed.


## Follow-up: successful-path deterministic parity (6 November 2026)

The atomicity wrapper now has a companion success-path regression. With the same seed, species, clock and inputs, the public staged transition is compared against the ordered inner evolution pipeline. The test requires identical emitted events and byte-identical serialized evolution state, species/genomes and keyed RNG state. This guards against a correct rollback path accidentally changing successful deterministic output or consuming a different RNG stream.

- Test: `successful_staged_step_matches_ordered_pipeline_state_and_rng`.
- The previous exact-head workflow [#42172213109](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42172213109) passed formatting, Clippy, fast Rust tests, desktop, installed backend, dashboard, scripts, dependency audit and week/year reference acceptance on the prior head `27611fe9`; it does **not** verify this follow-up.
- A fresh exact-head CI run is required for this test. No local Cargo result is claimed.
