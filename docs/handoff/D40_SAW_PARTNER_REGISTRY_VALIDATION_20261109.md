# D40 persisted saw-partner registry validation — 9 November 2026

Repository: `dylmarriner/Maer-ken-island`
Branch: `feat/d40-two-person-saw-ute-crosscut-20261109`
Pull request: [#136](https://github.com/dylmarriner/Maer-ken-island/pull/136)

## Finding

Snapshot restore checked that a persisted `CrossCutCrew` partner ID was non-empty, was not another active haul owner, and was not claimed by another crew. It did not prove that the partner existed in the snapshot's human registry. A malformed snapshot could therefore restore a crew with a dangling participant reference; the work loop would treat that participant as unavailable and silently convert the saw task to the solo axe fallback.

## Production correction

- Restore now requires every persisted saw partner to resolve in `snapshot.humans.registry`.
- Unknown partner IDs fail closed with `InvalidHaulJobState` before the island is published.
- Existing ownership, duplicate-assistant and solo-fallback behaviour remains unchanged.
- Corrected four 102-column test setup lines that caused the preceding exact-head `cargo fmt --check` failure.

## Regression coverage

`restore_rejects_a_saw_crew_partner_missing_from_the_human_registry` creates a valid source log and reserved piece-ID range but names a non-existent partner. Restore must return the typed invalid-haul error.

## Verification

Fresh exact-head CI is required for formatting, Clippy, fast tests, desktop/backend acceptance and week/year reference packs. The preceding workflow [#42559079714](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42559079714) failed the format job on the prior head; its successful independent dashboard, backend, computer-service, repository-script and dependency-audit jobs are not evidence for this new commit. No local Cargo pass is claimed because custom OpenCommander is currently rate-limited.

## Invariants

No snapshot schema, append-only event log, replay hash, RNG, simulation cadence, scientific constants, reference data or material-ledger format changed. No production deployment, live migration, destructive operation or external communication occurred.
