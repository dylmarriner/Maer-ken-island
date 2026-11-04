# Replay sequence exhaustion guard

Date: 2026-11-04

## Change
- Replaced saturating same-tick sequence allocation with checked addition.
- A loaded log containing sequence `u64::MAX` for a tick now reports sequence exhaustion instead of appending a duplicate sequence.
- `IslandLife::apply_command` reserves the next sequence before any create/intervention/control mutation. This prevents an unrecordable command from changing simulation state without a corresponding replay entry.
- Exposed a distinct `CommandError::ReplaySequenceExhausted` failure and a replay-log `SequenceExhausted` error.
- No tick, simulation state, snapshot format, replay hash, ledger, upstream human behavior or owner governance changed.

## Regression coverage
- Existing recorder tests now assert the checked `record` result.
- Added a boundary test with a preloaded same-tick `u64::MAX` sequence; recording refuses and leaves the entry vector unchanged.
- Existing replay sequence ordering and interleaved-tick cases remain in the target integration suite.

## Verification
- Source, command error and test changes pushed to `fix/replay-sequence-validation-20261104`.
- No local Cargo runtime test was run in this environment. Exact-head CI is required; prior build timeouts are not treated as passes.
- No production deployment, simulation run, owner-governance change or destructive operation was performed.

## Next
1. Run `cargo fmt --all -- --check`.
2. Run `cargo test -p mk_engine --test island_replay --locked --offline`.
3. Run `cargo clippy -p mk_engine --all-targets --locked --offline -- -D warnings`.
4. Verify exact-head GitHub Actions and the required ignored week/year reference-pack tests.
