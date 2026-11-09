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
