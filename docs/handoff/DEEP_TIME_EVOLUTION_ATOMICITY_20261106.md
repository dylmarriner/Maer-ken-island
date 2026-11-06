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
- The current source/test head is `5d14be7d21d811271099be8b346e32dd3d8f4ddb`. Exact-head workflow [#42180786470](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42180786470) was cancelled when a newer handoff commit superseded it; workflow [#42180945915](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42180945915) is pending on docs head `9c622ee3fda3144d9e2cf2e2dce7184a0b25931e`, which contains the same source/test head; prior successful workflow [#42172213109](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42172213109) predates these tests and does not verify them.
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


## Exact-head CI correction (6 November 2026)

Workflow [#42181591105](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42181591105) on head `af28f807481b065f950f6529866b7320ee1d58ef` passed `cargo fmt --all -- --check`, then Clippy failed on one test-only `unused_mut` warning at the deterministic seed probe. Fast tests were skipped because Clippy runs first.

- Removed the unnecessary `mut` from the probe registry; `gen_f64_01` takes an immutable reference.
- Fix commit: `be427c097f4e4aa9112f152218878c905e5ed9a6`.
- The other completed jobs in that run passed: web dashboard, computer service, repository scripts and dependency audit. Desktop, installed backend and week/year acceptance were still running when inspected; no result is claimed for them here.
- Fresh exact-head CI is required for `be427c0`. The prior workflow is a confirmed Clippy failure, not a passing test run.


## Follow-up: validate persisted deep-time numeric state (6 November 2026)

The snapshot restore path calls `DeepTimeEvolution::validate`, but the validator previously checked the simulation clock, geographic ranges, speciation times and intelligence ceiling while leaving other persisted numeric inputs unchecked. NaN or invalid carrying-capacity/recovery values could therefore survive restore and poison later population or speciation calculations.

- Validate top-level and recovery adaptive-radiation values, recovery time, recovery-rate multiplier, and every persisted carrying capacity.
- Validate append-only extinction-event start/duration values and reject an overflowing event end time.
- Evolution steps now validate their starting persisted state before staging and reject recovery-clock overflow before any mutation.
- Regression tests cover NaN, infinities, negative/out-of-domain values, invalid capacities, malformed extinction history, and byte-identical state/species/RNG preservation when preflight rejects a step.
- No snapshot schema, canon, RNG derivation, append-only log format, physical parameter, or simulation ordering change.

Exact-head CI is required; no local Cargo pass is claimed while the custom OpenCommander connector is returning HTTP 429.


## Follow-up: exact-head rustfmt correction (6 November 2026)

Workflow [#42183421772](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42183421772) reached the new persisted-state validation and reported only rustfmt differences in `deep_time_evolution.rs`. The formatter's exact line wraps are now applied. Clippy and fast tests were skipped after the formatting gate failed; no source-test pass is claimed. A fresh exact-head workflow is required for the corrected head.


## Follow-up: keep append-only history validation off the evolution hot path (6 November 2026)

The new persisted-state preflight initially reused the full snapshot validator on every evolution step. That validator scans both append-only extinction events and speciation timestamps. On long-horizon runs this makes per-step validation grow with the accumulated history.

- Split mutable-state checks into `validate_evolution_step_state` and append-only history checks into `validate_append_only_history`.
- `step_evolution_processes` now checks the live clock, recovery state, carrying capacities, species ranges and intelligence ceiling without rescanning historical vectors on every 10-year step.
- The full public `validate` still performs both checks. `WorldState` snapshot validation calls it for current and legacy snapshot restore, so persisted histories remain fail-closed at the trust boundary.
- Full history validation now also rejects a speciation timestamp later than the persisted simulation clock.
- Regression coverage proves the hot-path validator excludes append-only histories while the full validator rejects malformed history, and that future speciation timestamps cannot be restored.
- The explicit staging initializer/commit block is documented as a paired maintenance point for future mutable fields.
- Commit: `fe5c22efed7ca370e3363ba8d46eebfd6943b5a5`.
- Previous exact head `ab106d75` passed workflow [#42183725408](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42183725408), including formatting, Clippy, fast Rust tests, desktop, installed backend, dashboard, computer service, scripts, dependency audit and week/year reference acceptance. Nightly slow tests were skipped by design. The new head requires its own exact-head run; no new pass is claimed yet.


## Follow-up: enforce the latest parent speciation cooldown (6 November 2026)

A second inspection of the append-only speciation history found that the evolution pipeline used `iter().find`, which selected the **first** event for a parent. After a parent's second speciation, the old timestamp could let it speciate again only ten years later even though the latest event was still inside the 18,000-year cooldown.

- The hot path now searches the append-chronological history from the end and uses the most recent timestamp for that parent.
- Full persistence-boundary validation now rejects globally out-of-order speciation history, making the reverse lookup assumption explicit and fail-closed.
- A deterministic regression seeds two parent events at 18,000 and 36,000 years, advances the clock to 36,010 years, and proves the parent remains in cooldown: no child, event or RNG draw is consumed.
- Commit: `c312a74ed28f27badae69f15a85f57886a71e9ef`.
- Exact-head CI for this follow-up is required; no pass is claimed yet.
