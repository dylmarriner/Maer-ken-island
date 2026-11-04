# Replay save atomicity handoff

Date: 2026-11-04
Canonical repository: dylmarriner/Maer-ken-island
PR: https://github.com/dylmarriner/Maer-ken-island/pull/73
Branch: fix/replay-sequence-validation-20261104
Base observed: main at 5aaf51f; prior PR head 8f5c372
Code commit: 30ef734c06104a2562e69387e1ba6b521de3cbdc

## Change

IslandReplayLog save now allocates a unique staging file using exclusive create, a process ID and a bounded local suffix. It writes and syncs the complete log before atomic rename, and removes the staging file on write/rename failure. A stale temp file or a concurrent save can no longer be truncated through the old shared PID-only temp path.

The round-trip regression seeds both the legacy temp path and the first new candidate path, verifies both are preserved, then reloads the saved log and checks replay state-digest equality. Simulation state, replay format, material ledgers and owner-governed behavior are unchanged by this save-path fix.

## Verification

- Cargo formatting check: passed.
- Offline locked cargo check for mk_engine and the island_replay integration test target: passed.
- Clippy for all mk_engine targets with warnings denied: passed.
- Git whitespace check: passed.
- Targeted runtime test attempted twice, each reaching the 300-second command timeout while code-generating mk_engine. No assertion failure was reported, but runtime execution is not verified locally.
- Hosted CI for the new pushed head: pending; do not merge before exact-head CI is green.

## Remaining checklist

- Verify exact-head hosted CI and the targeted replay runtime suite.
- Run both slow reference-pack tests (week and year) before merge, as required by WORK_REMAINING.md for behavior-changing replay work.
- Recheck PR #73 mergeability and base conflicts; do not rebase or merge the separate conflict-blocked PR #68.
- Continue the broader roadmap in WORK_REMAINING.md: replay/hash acceptance, snapshots, material ledger closure, sourced ecology/industry, climate fidelity, town/workforce and year acceptance.
- Do not mark the island complete until documented acceptance gates are evidenced.

## Follow-up: replay directory-entry durability (2026-11-04)

- After syncing the staged replay file and atomically renaming it into place, Unix builds now sync the containing directory so the rename metadata is durable across a crash or power loss. Relative filenames resolve their parent to `.`. Non-Unix builds retain the synced-file plus atomic-rename behavior because portable directory handles are not exposed by `std` on every target.
- A parent-directory sync failure is returned as an I/O error after the rename; the file may be visible, but the save does not claim durability it could not confirm.
- The existing `a_log_round_trips_through_a_file` integration test exercises this save path and checks replay state-digest equality. On exact head `ab6054b`: `cargo fmt --all -- --check`, `git diff --check`, `CARGO_BUILD_JOBS=2 cargo check -p mk_engine --test island_replay --locked --offline` (40.64 s), and `CARGO_BUILD_JOBS=2 cargo clippy -p mk_engine --all-targets --locked --offline -- -D warnings` (50.75 s) passed.
- The targeted offline runtime command `CARGO_BUILD_JOBS=2 cargo test -p mk_engine --test island_replay --locked --offline` timed out twice at the 300-second command limit while compiling `mk_engine`; no assertion failure was reported, but runtime tests are not verified. The hosted exact-head CI run is the remaining independent runtime/build check; week/year slow reference-pack acceptance is still required before merge.

## Follow-up: owner-only replay file permissions (2026-11-04)

- Commit `90e52bb` creates Unix replay staging files with mode `0600` before writing any serialized content. Atomic rename preserves the restrictive mode on the final replay log. Non-Unix builds retain the platform's inherited ACL behavior because portable ACL controls are not exposed by `std`.
- The reason is data classification: replay logs include simulated people's names, birth timestamps and physical details. The regression `saved_replay_files_are_owner_only` verifies that group/world permission bits are absent and the owner can read the file.
- `cargo fmt --all -- --check` and `git diff --check` passed before commit. The targeted local command `CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0 cargo test -p mk_engine --test island_replay --locked --offline saved_replay_files_are_owner_only` reached its 300-second limit and was terminated with exit `-9` while compiling `mk_engine`; no test assertion ran, so the runtime regression is **not locally verified**.
- The earlier exact-head CI run #41752413427 was still running on `a205e10` at the last inspection: installed-backend, dependency-audit and computer-service jobs passed; desktop tests and fast tests were still in progress; slow week/year tests were skipped. The new `90e52bb` push requires a fresh exact-head CI run.

## Current remaining checklist

- [ ] Verify exact-head CI for `90e52bb`, including the new Unix permission regression.
- [ ] Run the week and year reference-pack slow tests before merge, as required by `WORK_REMAINING.md` for replay behavior changes.
- [ ] Obtain a completed runtime result for the new test; the local timeout is not a pass.
- [ ] Recheck PR #73 mergeability; leave separate conflict-blocked PR #68 untouched.
- [ ] Continue the remaining `WORK_REMAINING.md` acceptance work: replay/hash, snapshots, material ledgers, sourced ecology/industry, climate, town/workforce and year acceptance.
