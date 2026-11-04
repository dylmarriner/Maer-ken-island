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

- Exact-head workflow [#41928721548](https://github.com/dylmarriner/Maer-ken-island/actions/runs/41928721548) completed successfully on `0004b1e2382be58c69edf41dda77f8703961d837`.
- Passed: formatting, Clippy, fast Rust tests, both week/year reference-pack acceptance tests, desktop build/render tests, installed backend, dashboard, computer service, repository scripts and dependency audit.
- The nightly slow-test job was skipped by design; the path-scoped week/year acceptance job ran and passed both tests.
- Local Cargo checks were not run because the custom OpenCommander device session was unavailable.
- PR #81 was marked ready after exact-head checks passed and merged by squash as `7af8bec18ddbada7ff6f537d73684d0b805dc329`.
- No simulation constants, snapshot formats, append-only logs, canonical digest inputs or LLM behaviour changed.


## First exact-head CI correction

Workflow [#41928451248](https://github.com/dylmarriner/Maer-ken-island/actions/runs/41928451248) failed only at `cargo fmt --all -- --check`. Rustfmt requested a multiline layout for the first accessor assertion. No Clippy or test compilation was reached in that job. The exact formatter change is committed as `15367d4001c687a133371cf774bd1dc9e30ebbf2`; a fresh exact-head workflow is required. No test pass is claimed from the failed run.


## Corrected exact-head CI

- Formatting correction commit: `15367d4001c687a133371cf774bd1dc9e30ebbf2`.
- Handoff update commit: `22cd6a34b6eac8b0d001518979635a1b666069cf`.
- Fresh exact-head workflow [#41928602373](https://github.com/dylmarriner/Maer-ken-island/actions/runs/41928602373) is pending.
- The prior run #41928451248 failed only at rustfmt; no Clippy or Rust tests ran in that job. The corrected run must establish all required checks before merge.
