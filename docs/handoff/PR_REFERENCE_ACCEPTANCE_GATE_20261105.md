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


## Digest audit and reconciliation (2026-11-05)

Exact-head run [#41861164760](https://github.com/dylmarriner/Maer-ken-island/actions/runs/41861164760) completed. Formatting, Clippy, fast Rust tests, desktop application, installed backend, dashboard, computer service, repository scripts and dependency audit passed. The week/year reference gate failed only because the week test's pinned digest was stale; the independent year-weather test passed in 39.85 seconds.

The week test completed its audit before the digest comparison: 27,597 audits (167 hourly water audits), zero food and water shortfalls, 112 economy events, 200,000 patch trees, founder body carbon matching expected values (13.0/13.0 kg and 10.4/10.4 kg), forest carbon uptake within its 400–900 g C/m²/year reference band (471), and founder drinking rates within the configured intake bands. The only failed assertion was the final state digest: actual `bc8706aca29525e6bd75d27d94678a56cbd97a717aa4553ac8a15a2a64c7a076`, expected `c32897c47a10fcb5fed2671df781949b3d924cdd9ed1a62103127cf552647054`.

The source-history audit found inconsistent documentation: commit [`45ff141`](https://github.com/dylmarriner/Maer-ken-island/commit/45ff1414eb291ce9599d1677cb7398b8b41c7550) set the test pin to `c32897c4…27054`; commit [`1336218`](https://github.com/dylmarriner/Maer-ken-island/commit/1336218f2e5ea04899042ba8a5de114d32ae1865) changed only the checklist to `577e4215…540a`; the testing guide and state handoff still listed `f6699166…9c81`. Since then, simulation sources affecting life, industry, town, fitness, genome and replay behavior changed substantially. The current week run's non-digest audit passed and the independent year test passed, so the current deterministic output is now the pin in the test and all current canonical digest summaries. Historical session records remain historical and are not rewritten.

This reconciliation changes only the acceptance pin and its current documentation, not simulation state, reference data, snapshots, replay code, real-world constants or production infrastructure. A fresh exact-head CI run is still required to verify the updated pin and both reference tests.
