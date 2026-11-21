# D40 full Ute cross-cut restore acceptance — 9 November 2026

Repository: `dylmarriner/Maer-ken-island`
Branch: `feat/d40-two-person-saw-ute-crosscut-20261109`
PR: [#136 — two-person saw cross-cut for Ute transport](https://github.com/dylmarriner/Maer-ken-island/pull/136)
Base: `main` at `6b57ad96273c2320910f93902963bc8800974175`

## Engineering slice

Added `two_person_ute_crosscut_restores_and_completes_the_full_material_haul` to `crates/mk_engine/src/regional/life.rs`.

The earlier persisted-crew regression used a small synthetic OnFoot job. The new acceptance test exercises the actual 20 m ground-log Ute plan, with the excavator disabled and the Ute fuelled, then:

1. Starts the real two-founder `CrossCutCrew` task.
2. Advances it partially and snapshots the active job.
3. Restores the island and verifies the reserved piece-ID range is unchanged.
4. Completes the crew cut and verifies source removal plus exact conservation of length, dry mass, carbon and water.
5. Keeps the one material-owning primary haul and completes its Ute load/drive/unload route.
6. Verifies every reserved piece is in `LogPlace::Stored`, Ute fuel was consumed, both founders paid their own labour energy, and no additional piece IDs were allocated.

This closes the acceptance gap between the existing synthetic snapshot-resume test and the real Ute transport path.

## Commit and verification

- Test commit: `f05eb6424f22368b62a8c7b07d2830da77434b9e`.
- PR: [#136](https://github.com/dylmarriner/Maer-ken-island/pull/136), still draft.
- Exact-head CI for this new commit is pending. The previous head's workflow is not evidence for this new test.
- No local Cargo result is claimed because custom OpenCommander access remains rate-limited.

## D40 checklist

- [x] Real Ute plan selects a two-person saw cross-cut.
- [x] Partial crew task survives snapshot restore.
- [x] Reserved piece IDs survive restore without reallocation.
- [x] Full cross-cut conserves length, dry mass, carbon and water.
- [x] Primary and assistant energy are accounted separately.
- [x] Full Ute load/drive/unload finishes with all pieces stored.
- [ ] Exact-head format, Clippy and fast Rust tests.
- [ ] Exact-head desktop, installed-backend and week/year reference-pack acceptance.
- [ ] Terrain-aware Ute routing and persistent vehicle positions.

## Architecture and safety

No snapshot schema, append-only log format, replay hash, RNG, tick cadence, scientific constant, reference pack or live simulation state changed. The test only constructs an in-memory island fixture. No deployment, production data mutation, migration or external service call was performed.

## Next

- Verify the exact-head CI for this test.
- Reconcile material-ingestion PRs #134 and #135 only after controlled integration and green exact-head checks.
- Continue D40 route modelling, then D31 only with sourced regional nitrogen/phosphorus data.


## Exact-head fast-test failure and correction — 9 November 2026

- Exact-head workflow [#42564181241](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42564181241) passed formatting and Clippy, then ran the fast Rust suite: **897 passed, 1 failed**.
- The only failure was this acceptance test's arbitrary 100 × 60-second loop cap. The persisted saw task remained valid but its pack-derived duration exceeded 6,000 seconds; the assertion treated that longer task as a hang.
- Replaced the fixed minute cap with task-duration-driven advancement: the test reads the restored front task's exact remaining seconds and advances to that task boundary. The Ute route now advances each persisted haul task by its own remaining duration and bounds progress by the persisted task count, rather than an unrelated 500-minute ceiling.
- Production simulation behaviour, reference values, snapshot schema, replay hashes, RNG, cadence and material calculations are unchanged. This corrects the acceptance test's time horizon, not the saw duration or the assertion about completion.
- Source/test correction commit: `0b46fbda85ab6621585d5d6866ea59a625c46213`.
- Fresh exact-head formatting, Clippy, fast tests, desktop/backend and week/year reference-pack acceptance are required. No pass is claimed until the new head completes.


## Follow-up rustfmt correction — 9 November 2026

- The fresh workflow [#42569581446](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42569581446) ran on the duration-driven test correction. The fast-test job stopped at `cargo fmt --all -- --check` before Clippy or fast tests.
- Rustfmt identified one line-wrap difference in the new `matches!` assertion. Applied the exact formatter output, with no semantic change.
- Formatting correction commit: `2255b67bc2e6cb5161ca7a395be95ea0f4f4d2cd`.
- The same workflow's week/year reference-pack, installed-backend and desktop jobs were still running on the preceding head; they do not certify this formatting correction. A fresh exact-head workflow is required.


## Fresh exact-head verification after formatting correction — 9 November 2026

- Current head: `8e5f4b882ba9b7be8f3fa3ee724b0917b0f27d9f`.
- Fresh workflow [#42569801043](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42569801043) is now running on the handoff head.
- Repository scripts passed. Web dashboard checks passed through the workforce assertion and were still running at inspection.
- Desktop, installed backend, dependency audit and week/year reference-pack jobs are running. The fast Rust format/Clippy/test job is queued.
- This fresh run supersedes the previous rustfmt-only failure for acceptance purposes; no format/Clippy/fast-test pass is claimed until its queued job completes.
- The prior run [#42569581446](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42569581446) failed only at one formatter line-wrap before Clippy/tests. The exact output is committed at `2255b67bc2e6cb5161ca7a395be95ea0f4f4d2cd`.


## Current acceptance status — 9 November 2026

- Source/test head for the latest acceptance workflow: `00a76298ad05744adb5002d736e79b36f86d6f42`; workflow [#42569859195](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42569859195).
- Later commits `524104dc` and `618a3459` only update this handoff; no source or test files changed after the workflow started.
- Verified on the workflow head: Rust formatting, Clippy, week/year reference-pack acceptance, desktop build and tests, installed-backend build/smoke test, web dashboard, computer service, repository scripts and dependency audit.
- Still running at last inspection: fast Rust tests and the desktop render/interaction/performance acceptance steps.
- Do not treat the in-progress jobs as passes; inspect #42569859195 before continuing.
