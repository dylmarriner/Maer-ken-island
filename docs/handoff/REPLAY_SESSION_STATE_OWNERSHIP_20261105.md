# ReplaySession single-writer state ownership

Date: 2026-11-05
Repository: `dylmarriner/Maer-ken-island`
Branch: `fix/replay-session-readonly-state-20261105`
Base: PR #80 merge `e73a1bcec4c19e3691b9c45f847fe4e74c044d99`

## Defect

`ReplaySession` was exported from `mk_engine` while exposing mutable public fields for its
initial snapshot, current world state, target tick, replay options and history. External callers
could mutate deterministic replay state or metadata without passing through `replay_to`, bypassing
the session's budget, shadow-state and failure-poisoning invariants.

A repository-wide usage search found no in-repository consumer relying on those public fields.

## Change

- Make the five session-owned fields private.
- Add immutable accessors for the initial snapshot, current world state, target tick, options and
  replay history.
- Keep `shadow_state` and the latched failure private.
- Add a regression verifying the public accessors report the initial snapshot/state/target,
  default budget and empty history.
- Record the single-writer rule in `WORK_REMAINING.md` and `docs/ISLAND_SYSTEM_STATUS.md`.

This does not change replay stepping, hash inputs, snapshots, append-only island logs, canonical
digests, physical constants or LLM behaviour.

## Verification

- Exact-head hosted CI required: fmt, Clippy, fast Rust tests, week/year reference acceptance,
  desktop, installed backend, dashboard, computer service, repository scripts and dependency audit.
- Local Cargo checks were not run because the custom OpenCommander device session is unavailable.
- Exact-head workflow must prove no workspace consumer depended on the formerly public fields.

## Next

1. Verify the exact PR head and all required CI jobs.
2. Merge only after green exact-head checks and review.
3. Continue the remaining Phase 5 application and realism requirements in `WORK_REMAINING.md`.


## First exact-head CI correction

Workflow [#41928451248](https://github.com/dylmarriner/Maer-ken-island/actions/runs/41928451248) failed only at `cargo fmt --all -- --check`. Rustfmt requested a multiline layout for the first accessor assertion. No Clippy or test compilation was reached in that job. The exact formatter change is committed as `15367d4001c687a133371cf774bd1dc9e30ebbf2`; a fresh exact-head workflow is required. No test pass is claimed from the failed run.
