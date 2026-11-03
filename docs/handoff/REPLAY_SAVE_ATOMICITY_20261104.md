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
