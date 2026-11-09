# D40 saw cross-cut snapshot/restart acceptance — 9 November 2026

## Production change

`IslandLife::restore` now reconstructs and validates pending cross-cut ID ranges before accepting an active haul snapshot.

For both solo `CrossCut` and `CrossCutCrew` tasks, restore now requires:
- the task's source log ID to equal the owning haul job's log ID;
- the source log to still exist;
- the reserved piece-ID range to be valid, unoccupied and below the allocator watermark.

The existing ledger helper also reconstructs ranges for older snapshots whose allocator watermark was persisted before pending-ID tracking existed. Invalid state now returns `InvalidHaulJobState` during restore instead of reaching a later assertion/panic when the task completes.

The existing crew-ownership checks remain: an assistant cannot be claimed by two crews, own another active haul, or assist their own haul.

## Regression coverage

- `restore_rejects_a_saw_crew_whose_source_log_is_missing`
- `restore_rejects_a_saw_cut_that_targets_another_jobs_log`
- Existing duplicate-assistant and assistant-own-haul restore tests now use real source logs and valid reserved ranges, so they isolate the ownership invariants.
- `persisted_saw_crew_resumes_after_snapshot_restore_and_conserves_material` advances a crew cut halfway, snapshots/restores, finishes the remaining work, and checks exact length, oven-dry mass, carbon and water totals, one-time piece-ID reservation, and separate energy expenditure by both founders.

## Verification

The earlier exact-head workflow #42552777537 passed formatting, Clippy, fast Rust tests, desktop application, installed backend, week/year reference packs, dashboard, computer service, repository scripts and dependency audit. That run predates the new restore guard and regression tests.

On the test-only head, workflow #42558437059 passed formatting and Clippy; fast Rust tests and the remaining application/reference-pack jobs were still running when the production guard was added. That predecessor run does not verify the new code.

Production guard commit: `a05f1ccbc65dc3765f6f94e91c8cc6ff2b1899b9`.
Roadmap update: `79a73fcb5b484b3be308780048b063982e81606d`.

A fresh exact-head workflow is triggered by this handoff commit. No test pass is claimed for the new restore guard until that run completes green.

## Invariants

No snapshot schema, append-only log format, replay hash, RNG, tick cadence, scientific constant or reference pack was changed. The change tightens validation of existing persisted state and preserves the single-writer material mutation model. No production deployment or live simulation state was modified.
