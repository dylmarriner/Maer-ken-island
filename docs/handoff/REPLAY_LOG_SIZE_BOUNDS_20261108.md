# Bounded replay log file I/O

Date: 2026-11-08
Repository: `dylmarriner/Maer-ken-island`
Branch: `fix/replay-record-sequence-monotonic-20261107`
PR: #123

## Finding

`IslandReplayLog::load` used `std::fs::read`, which allocates the entire input before parsing. A corrupt or externally supplied replay file could therefore consume unbounded memory. The save path also serialized the complete log into an unbounded intermediate `Vec<u8>` before writing it.

## Change

- Set an explicit 64 MiB replay-log file budget.
- Check regular-file metadata and reject files already over the limit before reading.
- Read at most limit + 1 bytes to protect against a file growing after the metadata check.
- Stream pretty-JSON serialization through a limited writer; an oversized save removes its temporary file and leaves the destination untouched.
- Return a typed `IslandReplayError::TooLarge` with the path and configured limit.
- Preserve the replay JSON format, sequence identifiers, deterministic state, snapshots, RNG and hashes.

## Regression

- Integration test creates a sparse file one byte over the limit and verifies fail-closed rejection from metadata.
- Unit test proves the bounded writer accepts exactly the configured byte count and never writes beyond it.

## Verification

- Production and regression commits pushed to the existing replay-integrity PR branch.
- OpenCommander returned HTTP 429, so no local Cargo result is claimed.
- Exact-head CI must pass formatting, Clippy, fast Rust tests, desktop acceptance, installed backend and reference-pack acceptance before merge.

## Safety

No simulation tick, snapshot, replay digest, RNG, append-only state or LLM path changed. No live island state was loaded, advanced or modified. The paused replay PRs #73–76 were not changed.

## Next

Verify the exact PR #123 head. If green, merge only after the review remains clean. Continue the remaining roadmap from `WORK_REMAINING.md`, not by treating replay I/O hardening as completion of the island project.


## Collision-safe replay temporary files — 8 November 2026

The bounded save path previously used `File::create` on a predictable PID-only sibling path. That could truncate a stale temporary file or follow a pre-existing symlink before the atomic rename.

- Replay saves now reserve a temporary sibling with `OpenOptions::create_new(true)`, which refuses existing files and symlinks instead of replacing/following them.
- A bounded 32-name suffix search allows concurrent saves in the same process to reserve distinct paths; if all candidates are occupied, save fails closed.
- The destination is still replaced only after complete bounded serialization. Existing temporary files are never modified.
- Regression `saving_uses_a_fresh_temporary_path_without_clobbering_existing_files` pre-creates the first candidate with sentinel bytes, saves a replay, verifies the sentinel remains byte-for-byte unchanged, and reloads the saved log.

Verification: no local Cargo result is claimed. The exact-head CI workflow must rerun formatting, Clippy, fast tests, desktop acceptance, installed backend and reference-pack checks after this code/test change. No live island state was loaded or modified.
