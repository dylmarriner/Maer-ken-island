# Replay sequence-boundary handoff — 8 November 2026

## Scope

Repository: `dylmarriner/Maer-ken-island`  
PR: [#123 — same-tick replay sequence integrity](https://github.com/dylmarriner/Maer-ken-island/pull/123)  
Branch: `fix/replay-record-sequence-monotonic-20261107`

## Engineering change

Added an integration regression to `crates/mk_engine/tests/island_replay.rs` for the final representable same-tick sequence.

The test proves:

- An imported entry at `u64::MAX - 1` may be followed by exactly one append at `u64::MAX`.
- The next append returns `ReplaySequenceExhausted { tick }`.
- Exhaustion does not append a duplicate or mutate the retained log.

This complements the existing test that starts at `u64::MAX` and refuses the next append. No production code, serialized replay shape, snapshot, RNG, state digest or simulation rule changed.

## Commit

- `48e38e2b3f728e386723d6fdff64013d1ed1808c` — `test(replay): cover final available sequence boundary`

## Verification

Predecessor head `9c2f1d8c19238ee3814ed91b71e365117ff3abe3` had exact-head workflow [#42413861817](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42413861817) complete successfully: formatting, Clippy, fast Rust tests, week/year reference-pack acceptance, desktop application, installed backend, dashboard, computer service, repository scripts and dependency audit passed. Nightly slow tests were skipped by design.

The new test commit requires a fresh exact-head workflow. Do not attribute predecessor results to the new head.

## Next

1. Verify the new boundary regression on the exact current head.
2. Resolve the sequence-exhaustion and replay-index review threads after exact-head CI passes.
3. Keep PR #123 draft until shared-file overlap with timber PR #124 is reconciled.
4. Preserve deterministic ordering, append-only history and replay-hash equality.
