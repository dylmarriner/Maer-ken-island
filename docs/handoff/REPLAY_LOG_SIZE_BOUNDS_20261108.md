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
