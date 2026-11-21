# Replay security reconciliation (15 November 2026)

## Why PR #73 could not be merged as written

The original November 4 PR introduced a replay parser and stricter zero-based contiguous tick sequences. Since then `main` independently added a bounded 64 MiB loader, duplicate-position refusal, sparse sequence preservation, deterministic ordering and an indexed recording path. Restoring the older PR implementation would regress those guarantees and change current replay semantics. This reconciliation merges current `main` and intentionally retains its `commands.rs`, `replay.rs` and `island_replay.rs` implementation as the base, carrying forward only separately verified missing protections.

## Security changes

- Replay imports reject symbolic links by checking the path and, on Unix, opening with `O_NOFOLLOW`. `O_NONBLOCK` also avoids blocking on a path replaced with a FIFO; the opened descriptor is separately checked to be a regular file.
- Replay temporary output files request Unix mode `0600` rather than inheriting a potentially permissive umask. Existing exclusive temporary-path creation and atomic rename remain unchanged.
- The loader continues to enforce the existing 64 MiB limit, version check, and order/duplicate checks; the wire format, RNG, tick cadence and scientific constants are unchanged.

## Local verification on AMD

1. Added two focused Unix regressions and saw both fail against the previously merged implementation: symlink redirect was accepted, and a replay save under `umask 022` was group-readable.
2. After the fix, both new tests passed.
3. `cargo test -p mk_engine --test island_replay --locked`: **21 passed, zero failed**.
4. `python3 -m unittest scripts.tests.test_ci_rust_profile -v`: **3 passed**.
5. `cargo fmt --all -- --check` and `git diff --check` passed. Full exact-head GitHub CI must pass before merge.

The original November 4 handoffs remain historical records, not statements of current replay gap semantics. Sparse, strictly increasing sequence numbers are supported by current `main`; duplicate positions remain invalid.
