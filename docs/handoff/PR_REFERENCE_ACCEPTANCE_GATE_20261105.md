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
