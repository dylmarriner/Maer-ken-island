# Replay sequence validation handoff

Date: 2026-11-04
Canonical repository: `dylmarriner/Maer-ken-island`
Base observed: `main` at `5aaf51f` (after PRs #70–#72).
Branch: `fix/replay-sequence-validation-20261104`.

## Change

`replay_island` now validates each included tick's command sequence after sorting by `(tick, sequence)`. Sequences must be zero-based and contiguous, matching `IslandReplayLog::record`. Duplicate sequence numbers and gaps return `IslandReplayError::InvalidSequence` before any logged command is applied. Shuffled but valid logs remain supported.

Regression coverage: duplicate same-tick sequence and missing same-tick sequence. This is a replay-integrity check only; it changes no simulation state, real-world constants, material ledgers, snapshot format or owner-governed behavior.

## Verification

- `cargo fmt --all -- --check`: passed.
- `git diff --check`: passed.
- `CARGO_BUILD_JOBS=2 cargo check -p mk_engine --test island_replay --locked --offline`: passed, including type-checking the new integration tests.
- `cargo test -p mk_engine --test island_replay --locked --offline`: local runtime execution was terminated by SIGTERM while code-generating `mk_engine`; no assertion failure was reported, but the runtime suite is **not verified locally**.
- Hosted CI on the pushed head: pending. Do not merge until exact-head CI is green.

## Remaining gates

- Run `cargo test -p mk_engine --test island_replay --locked --offline`.
- Run `cargo fmt --all -- --check` and targeted Clippy with warnings denied.
- Push the branch and open a focused PR after local checks pass.
- Existing draft PR #68 remains separate and conflict-blocked; this work does not rebase, resolve or merge it.
- Full island acceptance, reference-pack slow tests, year replay, ledger closure and the broader `WORK_REMAINING.md` roadmap remain open.
