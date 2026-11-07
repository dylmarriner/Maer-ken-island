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

- - Production guard commit: `81be08e9d0650bd48ec53f0054ef74c0d6a70e8e`.
- Regression commit: `756b556c9c3f8c717af353c63b2b3b71db84b883`.
- Formatting correction: `f0a95dd7ffe53f86721fa2e43fa8354ec6004396`.
- First exact-head workflow `#42353261552` found one rustfmt-only import layout mismatch. The import was corrected in the formatting commit above; that failed formatting run is retained as evidence.
- Replacement exact-head workflow `#42353406567` on head `7ff8b946a9082c10dc89bd6a1cb73d195233ea7d` is in progress. At last inspection, repository scripts, dashboard, dependency audit and computer service passed; desktop, installed backend, week/year acceptance and the fmt/Clippy/fast-test job were still running. Nightly slow tests were skipped by design.
- This handoff-only follow-up will trigger a fresh workflow on the final PR head. No local Cargo result is claimed.

## Remaining

- Verify the exact PR head.
- Keep this PR separate from the older open replay-sequence PR #73; if #73 merges first, rebase this small guard and rerun exact-head CI.
- Do not change the paused Maer-Ken Merge Completion job.
