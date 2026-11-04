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
