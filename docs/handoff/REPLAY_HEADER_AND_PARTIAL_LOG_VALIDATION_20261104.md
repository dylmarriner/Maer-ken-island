# Replay header and complete-log validation

**Date:** 2026-11-04  
**Repository:** `dylmarriner/Maer-ken-island`  
**PR:** #75, stacked on #74 → #73  
**Branch:** `fix/replay-inmemory-digest-validation-20261104`

## Change

The replay file loader already rejected unsupported `REPLAY_LOG_VERSION` values, but the writer and direct in-memory replay path did not enforce the same format-version contract. Direct replay also validated only entries at or before `until_tick`, allowing a partial replay to accept a malformed later portion of the same log.

The change centralises header validation (format version plus canonical 64-character lowercase-hex scenario digest) and applies it at load, save and in-memory replay boundaries. In-memory replay now validates the complete command sequence before bootstrapping the island, then filters the validated entries for the requested replay horizon.

The existing save size-preflight ordering is preserved: oversized serializations still report `TooLarge` before header-field validation.

## Regression coverage added

- In-memory replay rejects an unsupported log version.
- Loading rejects an unsupported log version.
- Saving rejects an unsupported log version and leaves no output file.
- A partial replay rejects an invalid sequence in entries beyond the requested tick.

## Commits

- Source: `31505098546e7d025852460821e0a66f0bd31ff0`
- Tests: `f92b9fbf40bf5916d473a3e821e84fb8adc37f5c`

## Verification

- Changes are pushed to the PR branch.
- Exact-head GitHub Actions must run after the test commit.
- No local Cargo test pass is claimed.
- The preceding exact-head run #41806794152 passed formatting, Clippy, fast tests, desktop build/tests/render smoke, installed backend, web dashboard, computer service, dependency audit and repository scripts. Its nightly slow job was skipped and it predates this change.

## Remaining

1. Verify the new exact-head fast test and all required CI jobs.
2. Run the mandatory week/year reference-pack slow tests on the supported pinned toolchain.
3. Re-run fixed-seed replay and compare canonical state digests.
4. Keep PR #75 stacked until PRs #73 and #74 are accepted; do not retarget or merge early.

No simulation state, scenario digest algorithm, snapshot format, owner governance, live infrastructure or SCOS state was changed.


## CI correction (2026-11-04)

The first run on the new head, #41816490579, failed at `cargo fmt --all -- --check` on one multiline assertion in the new unsupported-version save test. The formatter's exact diff was applied in commit `28488c39e7a8f57d9f0452fd0b87a6ca3cc6792f`. The follow-up exact-head run #41816993183 is pending. No Rust test pass is claimed for the new code until that run completes.