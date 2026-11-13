# Maer-Ken haul snapshot saw-crew ownership — 9 November 2026

## Scope

Repository: `dylmarriner/Maer-ken-island`  
Branch: `feat/d40-two-person-saw-ute-crosscut-20261109`  
Pull request: [#136](https://github.com/dylmarriner/Maer-ken-island/pull/136)

## Finding

Live work planning keeps saw assistants out of subsequent haul plans during the same work tick. Snapshot restore validated each `HaulJob`, unique log ownership and the single-vehicle constraints, but did not validate crew-participant ownership across persisted jobs. A malformed snapshot could therefore assign one founder to multiple saw crews or make a founder assist while also owning another active haul.

## Production correction

Restore now builds the set of active haul owners and rejects a `CrossCutCrew` partner when that partner is already an active haul owner or has already been claimed by another persisted crew. This applies the same single-writer labour ownership rule at the snapshot boundary as live planning.

No snapshot fields, serialization tags, replay hashes, event logs, RNG, simulation cadence or material ledger formats changed.

## Regression coverage

- Two distinct persisted crew jobs cannot claim the same assistant.
- A persisted saw assistant cannot simultaneously own another active haul.

Both cases assert fail-closed `InvalidHaulJobState` restoration.

## Verification state

Fresh exact-head CI is required for formatting, Clippy, fast Rust tests, desktop/backend acceptance and week/year reference packs. No local Cargo pass is claimed because custom OpenCommander device discovery is currently rate-limited.

## Safety

No production deployment, live migration, destructive action, paid service or external communication was performed.
