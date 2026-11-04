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
- Keep session storage, replay state, hash chains, snapshots and simulation behavior unchanged.
- Update the canonical status and work ledger with PR #81's completed exact-head verification and
  merge evidence.

## Commits

- `edffa9a3300e3843ceca28bb1d8e5ba322b9720a` — deterministic session-name ordering.
- `59981d8741211181c77d1ff543f9fc573116fa73` — regression test.
- `5e9419b3c720ca2ada29c72754d237d30acc3e5d` — apply exact rustfmt output from the first CI run.
- `639b87a2e4175cad1a96a818a4cd9f71d8ccb915` — update work ledger with PR #81 evidence.
- `39a940f6d0de61c4cccbe83be9c89780eaa418e5` — update system status.
- `4fd24740f92aa91fce1d4e253e9e1368594594b6` — correct PR #81 handoff verification.

## Verification

- First exact-head workflow [#41936832743](https://github.com/dylmarriner/Maer-ken-island/actions/runs/41936832743) caught two rustfmt layout differences in the new test import and constructor expression. The formatter output was applied in commit `5e9419b3c720ca2ada29c72754d237d30acc3e5d`; the first run's check job stopped before Clippy/tests, so no Rust test pass is claimed from it.
- A fresh exact-head workflow is required for `5e9419b3c720ca2ada29c72754d237d30acc3e5d`, including fmt, Clippy, fast Rust tests, week/year reference-pack acceptance, desktop/backend, dashboard, computer service, scripts and dependency audit.
- Local Cargo checks were not run because the custom OpenCommander service is returning HTTP 429; no local pass is claimed.
- No simulation constants, canonical digests, snapshots, append-only logs, LLM behavior or live
  systems changed.

## Next

1. Verify exact-head CI and ensure the regression runs in the fast Rust test job.
2. Keep the PR draft until the exact-head workflow is green.
3. Continue the remaining sourced realism, industry and Phase 5 acceptance requirements in
   `WORK_REMAINING.md`.
