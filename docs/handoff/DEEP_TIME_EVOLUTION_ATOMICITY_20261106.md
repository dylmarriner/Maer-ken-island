# Deep-time evolution atomicity — 6 November 2026

**Repository:** `dylmarriner/Maer-ken-island`  
**Branch:** `fix/deep-time-evolution-atomicity-20261106`  
**Base:** `main` at `3057ea4274f8d6a131fa44a24380e0c7c145057a` (PR #106 is merged)  
**PR:** #107

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

- PR #106 is merged into `main` as `3057ea4274f8d6a131fa44a24380e0c7c145057a`. Its exact-head workflow [#42169416524](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42169416524) passed formatting, Clippy, fast Rust tests, desktop, installed backend, web dashboard, computer service, repository scripts, dependency audit and week/year reference-pack acceptance. Nightly slow tests were skipped by design.
- PR #107 is based directly on `main` at that merge commit; it is no longer stacked on the now-merged #106 branch.
- The current source/test head is `5d14be7d21d811271099be8b346e32dd3d8f4ddb`. Exact-head workflow [#42180786470](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42180786470) was created for the current head and is queued; prior successful workflow [#42172213109](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42172213109) predates these tests and does not verify them.
- No local Cargo result is claimed: custom OpenCommander discovery returned HTTP 429. Do not substitute another remote-desktop connector.
- The staging value copies only fields owned by the evolution pipeline; the append-only extinction history is not copied. Long-horizon performance still needs exact-head CI and the repository's acceptance profile before merge.

## Safety

No live simulation state, canon, physical parameters, snapshot format, production infrastructure or external service was changed.


## Follow-up: successful-path deterministic parity (6 November 2026)

The atomicity wrapper now has a companion success-path regression. With the same seed, species, clock and inputs, the public staged transition is compared against the ordered inner evolution pipeline. The test requires identical emitted events and byte-identical serialized evolution state, species/genomes and keyed RNG state. This guards against a correct rollback path accidentally changing successful deterministic output or consuming a different RNG stream.

- Test: `successful_staged_step_matches_ordered_pipeline_state_and_rng`.
- The previous exact-head workflow [#42172213109](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42172213109) passed formatting, Clippy, fast Rust tests, desktop, installed backend, dashboard, scripts, dependency audit and week/year reference acceptance on the prior head `27611fe9`; it does **not** verify this follow-up.
- A fresh exact-head CI run is required for this test. No local Cargo result is claimed.


## Follow-up: keep append-only extinction history out of per-step staging (6 November 2026)

Codex review identified that cloning the whole `DeepTimeEvolution` copied the accumulated `extinction_events` history on every evolution tick. Since the separate extinction phase appends to that history and long-horizon acceptance may execute up to a million evolution ticks, repeated history copies could make runtime scale roughly quadratically with the horizon.

- The staging value now copies only fields mutated by `step_evolution_processes_inner`; its extinction history is empty.
- Successful commit writes back only those owned fields, leaving the original append-only extinction history untouched.
- Success-path parity and late-drift rollback regressions now seed existing extinction history and assert it remains byte-identical.
- This removes per-tick copying of accumulated extinction events without changing event ordering, RNG derivation, snapshots or the extinction phase.
- Review finding: PR #107 thread `PRRT_kwDOU2IUOc60ZaJe`; keep unresolved until the exact-head performance correction and regression are verified.
- Exact-head CI after this change is required. No local Cargo result is claimed while custom OpenCommander is returning HTTP 429.


## Follow-up: environmental-stress fail-fast boundary (6 November 2026)

The climate integration normalises global environmental stress to the closed interval [0, 1]. Direct callers could previously pass NaN, infinities or out-of-range values; NaN could flow into selection arithmetic and silently collapse population values instead of producing an explicit error.

- Added a preflight before staging that requires finite environmental stress in [0, 1].
- Regression cases cover NaN, positive/negative infinity, negative stress and stress above one.
- Each rejected call asserts byte-identical evolution state, species and RNG registry.
- Source/test commit: `1e40b1df4df21a73bc2f83e981448373bd1d9653`.
- Constant-casing cleanup: `6ac9a66f86292a67595efe4ad98d7c8286067700`.

## Follow-up: successful speciation parity (6 November 2026)

The earlier success-parity fixture used a 2,500-year step and could not reach the 18,000-year speciation threshold. The new deterministic fixture searches a bounded seed range for a keyed draw below the fixture's minimum speciation probability, then compares staged and direct ordered execution.

- Requires the child species to exist and the parent's `last_speciation_times` entry to be committed.
- Compares emitted events, serialized evolution state, species/genomes and RNG state.
- Seeds existing append-only extinction history and requires it to remain identical.
- The test closes the review gap without changing the evolution phase order or random-key derivation.
- Exact-head CI remains pending; no test pass is claimed.


- Exact rustfmt correction from the new run's formatter diff: `5d14be7d21d811271099be8b346e32dd3d8f4ddb`. The current exact-head workflow is [#42180786470](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42180786470), queued at the time this handoff was updated.
