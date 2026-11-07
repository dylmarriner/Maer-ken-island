# Replay-log input bound and CI disk budget — 4 November 2026

## Repository and change

- Canonical repository: `dylmarriner/Maer-ken-island`
- Pull request: [#73](https://github.com/dylmarriner/Maer-ken-island/pull/73)
- Branch: `fix/replay-sequence-validation-20261104`
- Code/test/CI commit: `3e27e6f7b910b7c0b0388efbfa859b52dd726c18`

## Implemented

- `IslandReplayLog::load` refuses serialized replay files larger than `MAX_REPLAY_LOG_BYTES` (128 MiB) before parsing them.
- The loader uses a bounded read as well as the metadata preflight, so a file that grows after the metadata check cannot cause an unbounded read.
- Added a `TooLarge` error and sparse-file regression test that exercises the refusal without allocating a 128 MiB payload.
- CI now sets `CARGO_PROFILE_DEV_DEBUG=0` and `CARGO_PROFILE_TEST_DEBUG=0` while keeping `CARGO_INCREMENTAL=0`. A Python regression test prevents those resource limits from silently disappearing.

The size cap is an explicit compatibility bound. If a documented reference pack legitimately exceeds 128 MiB, review that evidence before raising the cap or implementing streaming parsing. Simulation state, replay digests, snapshots, material ledgers, and owner governance were not changed.

## Verified locally

- `cargo fmt --all -- --check` — passed.
- `cargo check -p mk_engine --test island_replay --locked --offline` — passed.
- `python3 -m unittest discover -s scripts/tests -v` — **10/10 passed**.
- `git diff --check` — passed.
- Targeted runtime test `cargo test -p mk_engine --test island_replay oversized_replay_logs_are_rejected_before_reading --locked --offline` — **timed out after 300 seconds while compiling `mk_engine` test code**; no assertion result was obtained.

## Hosted CI failure and fix

- [CI run #41754767814](https://github.com/dylmarriner/Maer-ken-island/actions/runs/41754767814) failed in the `Fast tests` step while linking `mk_engine` integration-test executables. The runner reported only 93 MiB free, then `No space left on device`; `runtime_boundary` also saw a linker bus error. This was a runner disk-exhaustion failure, not a reported test assertion failure.
- The new profile limits target debug-artifact size. Exact-head hosted CI after this commit is still required to verify the mitigation.

## Remaining checklist

- [ ] Verify exact-head CI with the profile limits, including fast tests and desktop checks.
- [ ] Run the required week and year reference-pack slow replay tests before merging.
- [ ] Verify runtime replay digest equality and persistence behavior on the new head.
- [ ] Continue the canonical roadmap: snapshot/replay acceptance, material-ledger closure, sourced ecology/industry, climate fidelity, town/workforce, and year acceptance.
- [ ] Keep PR #68 separate and untouched; its merge conflicts remain a separate issue.

## Follow-up: replay source path and writer bounds — 4 November 2026

Code commit: c3e02ff8b68b29fbb8de9dd4359bbd89569e6814

- IslandReplayLog::load now rejects symlinks and non-regular files before parsing. On Unix it also opens with O_NOFOLLOW | O_NONBLOCK, then validates the opened file descriptor, closing the metadata/open symlink race and avoiding a FIFO hang if a path is swapped.
- The existing 128 MiB compatibility cap is retained. The opened file is checked again and read with a one-byte-over-limit bound; save refuses serialized logs above the same limit with IslandReplayError::TooLarge, so the writer cannot create a file its reader rejects.
- Added regressions for symlink rejection and oversized serialized saves. The existing sparse-file test continues to cover oversized reads.
- The fast Rust CI job now bounds compile/link concurrency with CARGO_BUILD_JOBS=2; root dev/test debug-info limits and disabled incremental compilation remain in place. The existing Python CI-profile test now guards the concurrency setting too.
- Verified locally: cargo fmt --all -- --check, repository Python tests 11/11, and git diff --check passed.
- Runtime island_replay tests are still pending on the updated code head; the prior hosted run #41769714575 passed on the previous head but does not verify these follow-up changes.

No simulation tick, state digest, snapshot schema, material ledger, imported human runtime, LLM policy or owner governance changed.
