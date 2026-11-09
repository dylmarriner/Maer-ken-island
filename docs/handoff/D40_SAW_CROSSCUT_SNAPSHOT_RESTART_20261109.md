# D40 saw cross-cut snapshot/restart acceptance — 9 November 2026

## Change

Added `persisted_saw_crew_resumes_after_snapshot_restore_and_conserves_material` to `crates/mk_engine/src/regional/life.rs`.

The regression:
- creates one ground log and one persisted `CrossCutCrew` haul;
- advances half of the task and snapshots the island;
- restores the snapshot with the same canonical rules;
- completes the remaining task time;
- verifies the source log is replaced by the reserved pieces;
- checks exact conservation of length, oven-dry mass, carbon and water;
- verifies the piece-ID range is not reserved a second time;
- verifies the primary and assistant each pay labour energy.

The existing restore tests continue to reject duplicate assistant claims and an assistant who owns another active haul.

## Verification

Before this test-only acceptance slice, PR #136 head `07e650102d6443e036f77246c629e957ebc242c8` passed exact-head workflow #42552777537:
- formatting, Clippy and fast Rust tests;
- desktop application;
- installed backend;
- week/year reference-pack acceptance;
- web dashboard, computer service, repository scripts and dependency audit.
Nightly slow tests were skipped by design.

The new test commit `5899edb8d26371038f3964f10413771c17c9458e` triggered workflow #42558252972. That run failed only at `cargo fmt --all -- --check`, which requested the bootstrap assignment wrap at line 4891. The exact rustfmt output was applied in commit `801d6f6e9f7bfe29ccdfbcc731ae1dc682a279c7`. A fresh exact-head workflow is required before this regression is considered verified.

## Invariants

No production simulation logic, snapshot schema, append-only log format, replay hash, RNG, tick cadence, scientific constant or reference pack was changed by this test. No production deployment or live simulation state was modified.
