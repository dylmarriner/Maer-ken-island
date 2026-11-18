# PR #75: in-memory replay digest preflight (18 November 2026)

PR #75 was a stacked replay branch based on the original PR #74 code. After PR #74's 65 commits were reconciled against modern `main` and pushed as `f0db79d`, PR #75 was merged with that updated parent. The two overlapping replay source/test files select the newer parent implementation and retain sparse-sequence preservation, the 64 MiB I/O bound, runtime sequence index, and no-follow/owner-only file security. Original PR #75 commits remain in the merge history.

## Missing boundary validation

Direct in-memory callers can bypass `IslandReplayLog::load` and construct a log with an invalid digest. `replay_island` already rejects unsupported versions first and duplicate `(tick, sequence)` positions within the requested horizon, but it did not check canonical digest text until after the island was bootstrapped, treating malformed identity as a foreign scenario. The fix invokes the existing `validate_scenario_digest` after the version gate and **before** scanning commands or bootstrapping. The old full-history contiguous-sequence check is **not** restored: current replay legitimately supports sparse imported identifiers and horizon-limited duplicate validation.

## Regression evidence on Kubuntux backup-dev-ext4

- Added `replay_memory_boundary_rejects_invalid_digest_before_sequence_inspection`, using duplicate positions to prove validation precedence without a slow bootstrap. It failed before the fix and passed after it.
- Updated the existing wrong-island fixture to a canonical but distinct digest (`b` repeated 64 times), retaining the separate `WrongScenario` behavior. `a_log_from_another_island_is_refused` passed after the change.
- All three replay file-boundary regressions inherited from PR #74 passed again.
- `cargo fmt --all -- --check` and `git diff --check` pass. Exact-head GitHub CI and reference-pack acceptance remain required before merging PR #75.
