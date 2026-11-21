# Replay scenario digest validation — 4 November 2026

Canonical repository: `dylmarriner/Maer-ken-island`
PR: https://github.com/dylmarriner/Maer-ken-island/pull/74
Branch: `test/replay-special-file-refusal-20261104`
Parent inspected: `055c672b86de34b6a651161c3f4be925cf134064`

## Engineering slice

- Exact-head CI run [#41791619666](https://github.com/dylmarriner/Maer-ken-island/actions/runs/41791619666) failed at `cargo fmt --all -- --check`; the logs identified formatting-only differences in `replay.rs` and `island_replay.rs`. Those differences are corrected in this commit.
- Replay logs now require a canonical 32-byte lowercase hexadecimal scenario digest at load and save boundaries.
- Oversized serialized logs are still rejected as `TooLarge` before digest validation, preserving the existing bounded-output contract.
- Added load/save regression tests for malformed digests and updated the owner-only file permission test to use a real scenario digest.
- Existing in-memory replay mismatch behavior remains unchanged; replay still returns `WrongScenario` for a digest belonging to another island.

## Verification

- Previous run #41791619666: desktop, installed backend, repository scripts, dependency audit, computer service and web dashboard passed; the fast Rust job failed at rustfmt. No compile/test result was produced by that job.
- The first exact-head run after this patch found one additional rustfmt-only difference in the new error display arm; this follow-up formats that arm to the CI formatter's exact output. No compile or test result has yet been produced for the new code.
- The next exact-head CI run must pass formatting, Clippy, fast tests, desktop/backend checks and the mandatory week/year reference-pack tests before merge.
- The PR remains stacked on PR #73; do not retarget or merge this branch before its base is reconciled.

## Remaining checklist

- [ ] Verify exact-head format, Clippy and fast Rust tests.
- [ ] Verify malformed-digest load/save regressions.
- [ ] Run the required week and year reference packs and compare replay digests.
- [ ] Continue the outstanding `WORK_REMAINING.md` acceptance items.
- [ ] After PR #73 is accepted, rebase/retarget PR #74 to `main` and rerun required checks.

No production deployment, simulation-state mutation, snapshot-format change, owner-governance change, or remote host operation was performed.
