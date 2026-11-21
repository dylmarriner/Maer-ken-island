# Replay hardening handoff (2026-11-12)

Branch: `test/replay-special-file-refusal-20261104`. Local changes are intentionally not pushed until runtime tests pass.

## Verified
- `1aee88f` compares opened inode mtime/ctime before and after reading, detecting ordinary equal-size in-place rewrites; `cargo fmt --all` and `git diff --check` passed before commit.
- Prior `d26f960` tests rejection of BOM-prefixed JSON. Previous focused Cargo job `85bbbffe3bb1d7a9d5f33332605efef3` exited its wrapper successfully but Cargo itself timed out (124) while compiling `mk_engine`; **not a test pass**.

## Pending
- Focused Cargo job `c1af9c5ec6a3ffa5d432762831717004`: retrieve status and inspect underlying `CARGO_EXIT` (wrapper exit 0 alone is insufficient).
- Run `cargo test --offline -p mk_engine --lib` and replay-hash equality tests once compilation resources permit.
- Inspect current PRs/CI, rebase safely and push branch only after tests pass; keep the unrelated checkout untouched.
- Verify deterministic single-writer, snapshot/log invariants and offline/budget acceptance against canonical roadmap.

## Blockers
- Host reported 7.8 GiB swap nearly exhausted and repeated Cargo compilation timeouts. Avoid killing unrelated workloads or deleting build artifacts without approval.
- Metadata timestamps are best-effort race detection, not cryptographic integrity against adversaries able to forge timestamps.

## Follow-up (2026-11-12)
- Job `c1af9c5ec6a3ffa5d432762831717004` returned `CARGO_EXIT:124`: mk_engine compilation timed out again; no runtime test passed.
- Commit `2112ef8` adds a duplicate same-tick sequence loader regression. `cargo fmt --all -- --check` and `git diff --check` passed; runtime test remains blocked.
- Local branch is 26 commits ahead of remote after the test commit. Do not push or merge until runtime and deterministic replay-hash checks pass.
- Next: arrange a resource-capable isolated Rust build runner without deleting caches or interrupting unrelated processes, run the complete replay suite, inspect PR/CI, then push only verified commits.
