# Week/year reference acceptance on simulation PRs

Date: 2026-11-05

## Change
- Added a PR-only `reference-acceptance` job to `.github/workflows/ci.yml`.
- The job diffs the PR base and head and runs the week and year reference-pack tests when crate source/tests, reference packs, island scenario fixtures, `UPSTREAM.md`, or the deviation register change.
- The existing nightly slow tier remains intact. A path-scoped skip is explicitly reported and is not described as a test pass.
- Updated `docs/TESTING.md` and `WORK_REMAINING.md` to document the gate.

## Evidence
- Workflow commit: f6c15eb19672be5311ae5858e6633b608253b09c
- Testing guide commit: 4bda7a4ae9919bb6146be643cadc3d0ce6dac0f5
- Work remaining commit: 739113f1a225db323b625c42145cc84fba3e4eb9
- Pull request: https://github.com/dylmarriner/Maer-ken-island/pull/76
- PR #76 changes replay engine source and tests, so the exact-head run should execute both reference tests. CI logs, not this handoff, determine whether they pass.

## Remaining
- Inspect the exact-head workflow result.
- If the week test fails, distinguish audit/reference-pack failures from a digest-pin mismatch. Do not re-pin unless book-closure and reference-pack requirements pass.
- Run the full slow tier before final acceptance and preserve deterministic replay-hash equality.


## Follow-up: week digest mismatch and independent year test (2026-11-05)

- Exact-head PR workflow: [#41851124118](https://github.com/dylmarriner/Maer-ken-island/actions/runs/41851124118), merge checkout `abce6a3445cbcd5ebae559b41559453dcd6b0519`.
- Formatting, Clippy, fast tests, desktop application, installed backend, dashboard, computer service, repository scripts and dependency audit passed.
- The new week reference-pack test executed and passed its physical/ledger assertions, then failed only at the final pinned digest comparison: actual `bc8706aca29525e6bd75d27d94678a56cbd97a717aa4553ac8a15a2a64c7a076`, expected `c32897c47a10fcb5fed2671df781949b3d924cdd9ed1a62103127cf552647054`.
- The year test was skipped by GitHub Actions' default fail-fast step condition. This run fixes that evidence gap: both tests now run independently, and a final gate fails if either test fails.
- Do **not** re-pin the week digest from this result alone. Existing docs disagree about the canonical value, and the year-weather test still needs to execute before the digest audit can be completed.
- Next: inspect the independent year-test result, reconcile the week digest against the audit/reference-pack evidence and documented behavior changes, then rerun both tests on the resulting exact head.

- Independent execution and aggregate gating: [3abedc8](https://github.com/dylmarriner/Maer-ken-island/commit/3abedc8d3e41ac1b415d9749cb64d8c8816ace2f); testing-guide update: [eb41923](https://github.com/dylmarriner/Maer-ken-island/commit/eb4192335e9ce05fde136f13e9fed8afe000e892). The final workflow prints both outcomes on separate lines and fails if either is not successful.



## Digest reconciliation ownership (2026-11-05)

The week audit and independent year-weather test results from run [#41861164760](https://github.com/dylmarriner/Maer-ken-island/actions/runs/41861164760) are documented in the dedicated [PR #77](https://github.com/dylmarriner/Maer-ken-island/pull/77), which owns the canonical digest update, active-document consistency regression and current digest summaries.

This replay-budget PR intentionally leaves the digest pin and its active summaries unchanged to avoid duplicating PR #77's scope. PR #76 remains responsible for replay horizon enforcement and the path-scoped week/year acceptance gate. Do not merge this stack until PR #77's exact-head week/year tests pass and the replay stack is rebased/reconciled.

Run [#41879960540](https://github.com/dylmarriner/Maer-ken-island/actions/runs/41879960540) was started before this scope cleanup and may not represent the final restored head. Verify the exact-head workflow after this commit; no post-cleanup pass is claimed.
