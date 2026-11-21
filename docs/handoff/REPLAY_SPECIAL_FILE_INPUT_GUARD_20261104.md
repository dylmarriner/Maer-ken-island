# Replay special-file input guard — 4 November 2026

- Canonical repository: `dylmarriner/Maer-ken-island`
- Follow-up PR: stacked on https://github.com/dylmarriner/Maer-ken-island/pull/73
- Branch: `test/replay-special-file-refusal-20261104`
- Base branch: `fix/replay-sequence-validation-20261104`
- Test commit: ae57b1fdc04c1840af9bb9d961a91b3de25ceba5

## Change

The replay loader rejects symlinks and special files before parsing, uses non-following/non-blocking Unix open flags to close the metadata/open race, and re-checks the opened file type. The existing suite covered symlink refusal but not a FIFO.

Added a Unix-only regression that creates a named pipe with mode 0600 and asserts that `IslandReplayLog::load` returns `NotALog` with the regular-file explanation. This guards the rule that replay input is a bounded regular file, not a stream whose size and read completion are uncontrolled.

The test uses the crate's existing direct `libc` dependency and does not modify simulation state, replay formats, hashes, snapshots, or owner governance.

## Verification

- Test added to the Rust `island_replay` integration target.
- Local Cargo execution was not available through the connected GitHub interface.
- The follow-up PR is stacked on PR #73 so its in-progress CI run is not cancelled by a new commit.
- The follow-up CI must compile and execute the FIFO test on the exact new head before merge.

## Remaining checklist

- [ ] Verify the stacked PR's exact-head formatting, Clippy, and fast-test jobs.
- [ ] Complete PR #73's current fast and desktop checks.
- [ ] Run mandatory week/year reference-pack slow tests.
- [ ] Verify replay digest equality and persistence acceptance.
- [ ] Continue the remaining ecology, work, human agency, material-ledger and Phase 5 acceptance roadmap.
