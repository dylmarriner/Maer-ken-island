# In-memory replay format-version guard

Date: 2026-11-07  
Canonical repository: `dylmarriner/Maer-ken-island`  
Branch: `fix/replay-inmemory-version-validation-20261107`  
Base: `main`

## Defect

`IslandReplayLog::load` rejects an unsupported replay format, but callers can construct the public `IslandReplayLog` struct directly and pass it to `replay_island`. The in-memory path previously skipped the version gate and interpreted entries using the current format.

## Change

- `replay_island` now checks `log.version` against `REPLAY_LOG_VERSION` before bootstrapping the island.
- Unsupported in-memory logs return the existing `IslandReplayError::Version`.
- File-loaded and directly constructed logs now share the same fail-fast format contract.
- No replay format, snapshot layout, simulation state, canonical digest, RNG, append-only history, material ledger or LLM behaviour changed.

## Regression

`an_in_memory_log_with_an_unsupported_version_is_refused_before_replay` constructs a future-version log directly and asserts the version error, even though its scenario digest is intentionally invalid. This proves the format gate wins before scenario interpretation.

## Verification

- Current branch head: `756b556c9c3f8c717af353c63b2b3b71db84b883`.
- Regression committed: `756b556c9c3f8c717af353c63b2b3b71db84b883`.
- Exact-head CI is required for formatting, Clippy, fast Rust tests, desktop/backend acceptance and week/year reference-pack checks.
- No local Cargo result is claimed.

## Remaining

- Verify the exact PR head.
- Keep this PR separate from the older open replay-sequence PR #73; if #73 merges first, rebase this small guard and rerun exact-head CI.
- Do not change the paused Maer-Ken Merge Completion job.
