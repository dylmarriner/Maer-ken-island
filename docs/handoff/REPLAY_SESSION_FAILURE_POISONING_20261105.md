# ReplaySession fail-closed semantics

Date: 2026-11-05
Repository: `dylmarriner/Maer-ken-island`
Branch: `fix/replay-session-failure-poisoning-20261105`
Base: `589e4a2abdac8f2fa5e879f65b6225f3af2822bc`

## Defect

`ReplaySession::replay_to` logged a step/hash/state error and continued when
`stop_on_error=false`. A failed step can advance the primary world without advancing the shadow
world or recording a successful history entry. The loop could therefore finish with `Ok(())`
despite an unverifiable replay; repeated non-progressing errors could also spin until the target
was reached.

## Change

- Latch the first step failure in the session.
- Always return the first step failure, regardless of `stop_on_error`; the compatibility flag
  only controls whether the forced fail-closed behaviour is logged.
- Make subsequent `replay_to` and `verify` calls return the original failure without advancing.
- Count the latched failed attempt in replay statistics.
- Prove that an over-budget preflight rejection does not poison the session and a smaller retry
  still succeeds.
- Add a deterministic regression that corrupts only the shadow hash chain, then proves the session
  cannot continue or verify successfully after the mismatch.

## Invariants

- No simulation constants or world-state fields are changed.
- No append-only island replay-log entries, snapshot format, canonical digest input or week digest
  are changed.
- No LLM, network, external service or budgeted provider is involved.
- No deviation-register row is needed: this is replay-control failure handling, not a departure
  from physical reality.

## Verification

- Exact-head hosted CI is required for formatting, Clippy, fast tests, reference-pack acceptance,
  backend, desktop and audit jobs.
- Local Cargo checks were not run: the custom OpenCommander session is terminated. Do not use
  Remote Desktop Commander as a substitute.
- No live simulation service or persisted world data was changed.

## Next

1. Verify the exact PR head, especially the new replay unit tests.
2. Run the week/year reference-pack acceptance required by the project before merging.
3. Inspect reviews and merge only after all required exact-head jobs are green.
4. Continue the open realism and application acceptance items in `WORK_REMAINING.md`.
