# Replay session backward-target rejection

Date: 2026-11-05
Repository: `dylmarriner/Maer-ken-island`
Branch: `fix/replay-session-backward-target-20261105`
Base: `ae6757754e7e969406dd782a7a076c6cfab267c2`

## Defect

`ReplaySession::replay_to` is an advancing replay API. When a caller requested a tick earlier than
`current_state.tick`, saturating subtraction produced a zero-length replay and the method returned
`Ok(())` while the state remained ahead of the requested target. It also overwrote
`target_tick`, leaving success metadata inconsistent with the actual world tick.

## Change

- Add `ReplayError::TargetBeforeCurrentTick`.
- Reject a backward target before budget evaluation or metadata mutation.
- Preserve current state, target metadata and history on rejection.
- Prove that the rejection does not poison the session and a later forward replay still succeeds.
- Record the rule in the canonical remaining-work and island-status documents.

## Verification

- Regression test added in `crates/mk_engine/src/replay.rs`:
  `backward_target_is_rejected_without_mutating_or_poisoning_session`.
- Local Cargo checks were not run because the custom OpenCommander session is terminated.
- Exact-head hosted CI is required for fmt, Clippy, fast Rust tests, week/year reference acceptance,
  desktop, backend, dashboard, scripts and dependency audit.
- No live simulation service, persisted world, snapshot, append-only log or digest was changed.

## Next

1. Verify the exact PR head and the new regression test.
2. Keep the PR draft until all required exact-head jobs pass.
3. Continue the remaining Phase 5 application and realism requirements in `WORK_REMAINING.md`.
