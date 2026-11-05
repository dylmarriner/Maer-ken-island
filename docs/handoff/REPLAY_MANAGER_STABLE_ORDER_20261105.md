# Deterministic replay-manager session listing

Date: 2026-11-05
Repository: `dylmarriner/Maer-ken-island`
Branch: `fix/replay-manager-stable-order-20261105`
Base: `main` after PR #81 merge `7af8bec18ddbada7ff6f537d73684d0b805dc329`

## Defect

`ReplayManager` stores sessions in a `HashMap`, but `list_sessions()` returned the map's
iteration order directly. Rust's `HashMap` iteration order is intentionally not stable. Identical
session sets could therefore produce different API/UI ordering between processes, making replay
session listings needlessly non-deterministic.

## Change

- Collect the session names and sort them with `sort_unstable()` before returning.
- Add a regression that inserts `zeta`, `alpha` and `middle` in non-sorted order and requires
  the returned list to be `alpha`, `middle`, `zeta`.
- Reject duplicate session names before insertion. The previous `HashMap::insert` silently replaced
  the existing session, potentially discarding its replay progress. `add_session` now returns
  `Result<(), ReplayError>` and reports `DuplicateSessionName` without mutating the manager.
- Add a regression that advances the original session, attempts a duplicate insert, and proves the
  original tick and session listing remain unchanged.
- Keep session storage, replay state, hash chains, snapshots and simulation behavior unchanged.
- Update the canonical status and work ledger with PR #81's completed exact-head verification and
  merge evidence.

## Commits

- `edffa9a3300e3843ceca28bb1d8e5ba322b9720a` — deterministic session-name ordering.
- `873d93f2ead41e77f13858f775b40862e6ddc8c5` — reject duplicate session names without replacing replay progress.
- `c70de1d648fa8424f596cd52cb4d803bde7a2fa3` — apply the exact rustfmt output required by the first exact-head run.
- `59981d8741211181c77d1ff543f9fc573116fa73` — regression test.
- `5e9419b3c720ca2ada29c72754d237d30acc3e5d` — apply exact rustfmt output from the first CI run.
- `639b87a2e4175cad1a96a818a4cd9f71d8ccb915` — update work ledger with PR #81 evidence.
- `39a940f6d0de61c4cccbe83be9c89780eaa418e5` — update system status.
- `4fd24740f92aa91fce1d4e253e9e1368594594b6` — correct PR #81 handoff verification.

## Verification

- Exact-head workflow [#41944400454](https://github.com/dylmarriner/Maer-ken-island/actions/runs/41944400454) completed successfully on the then-current head `350f350570a4ac279dd5048dcfb51677baa30834`.
- Passed: formatting, Clippy, fast Rust tests, both week/year reference-pack acceptance tests, desktop/render, installed backend, dashboard, computer service, repository scripts and dependency audit.
- The nightly slow-test job was skipped by design; the required path-scoped week/year acceptance job ran and passed both tests.
- The review follow-up documents the stable-order API contract and the regression test, and corrects the merge gate requested in review.
- Exact-head workflow [#41953353236](https://github.com/dylmarriner/Maer-ken-island/actions/runs/41953353236) failed at `cargo fmt --all -- --check`; the log identified formatting-only differences in the new regression. The exact formatter output was applied in `c70de1d648fa8424f596cd52cb4d803bde7a2fa3`.
- The first run's computer service, dependency audit, repository scripts and web dashboard passed; desktop, installed backend and week/year acceptance were still running when the formatting failure was recorded. None of those results certifies the corrected head.
- A fresh exact-head workflow is required for `c70de1d648fa8424f596cd52cb4d803bde7a2fa3` and the subsequent handoff commit. Earlier green runs do **not** certify it. Do not merge until all required jobs, including week/year acceptance, pass on the final current PR head.
- Local Cargo checks were not run because the custom OpenCommander session is terminating; no local pass is claimed.
- No simulation constants, canonical digests, snapshots, append-only logs, LLM behavior or live systems changed.

## Next

1. Keep this PR draft until the exact current head has green required CI, including week/year reference-pack acceptance.
2. Re-check the PR head SHA and unresolved review threads immediately before merge; any new commit requires a new exact-head check.
3. Merge only after those conditions pass, then continue sourced realism, industry and Phase 5 acceptance work in `WORK_REMAINING.md`.

## Follow-up: reject blank replay-session identifiers

`ReplayManager::add_session` now rejects empty or whitespace-only names before duplicate lookup or insertion. Blank identifiers are not useful stable keys and otherwise create a session that cannot be meaningfully addressed by callers. The new `ReplayError::InvalidSessionName` is explicit, and the regression proves the manager remains empty after rejection.

Commits:
- `cf9f0a40e038ef1c4652574546ecda27c25f4847` — reject blank manager session names.
- `42dc4eac573bf8fe4cec212b325a9137384f0bb8` — make the regression input actual Rust whitespace escapes.

Verification:
- The earlier CI run on the previous PR head does not certify these commits; a fresh exact-head run must pass formatting, Clippy, fast tests, desktop/backend, dashboard, computer service, dependency audit and both week/year reference-pack acceptance tests.
- Local Cargo checks were not run because the custom OpenCommander session terminated; no local pass is claimed.
- No simulation state, tick logic, replay hash, snapshot format, append-only log or LLM behavior changed.

## Exact-head formatting correction

- Workflow [#41957894866](https://github.com/dylmarriner/Maer-ken-island/actions/runs/41957894866) reached the formatter and failed only because rustfmt wanted the new test's `let session` assignment wrapped differently. Clippy and fast tests were skipped after the formatting failure; this is not a test failure.
- Commit `0d5ef91eaa8a523e78bb651c0c6977df361c3480` applies the exact line layout printed by the CI formatter.
- A fresh exact-head workflow is required for the formatter correction and this handoff update. No earlier CI result certifies the new head.
