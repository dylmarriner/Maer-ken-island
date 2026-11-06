# FishStock binary snapshot compatibility — 7 November 2026

## Finding

PR #109 replaced the derived `Serialize` implementation for `FishStock` with a manual fail-closed serializer that validates physical state before writing. The implementation intentionally preserves field order, but the existing round-trip test alone could not prove compatibility with bytes written by the former derived serializer: a serializer and deserializer that changed together could still round-trip successfully.

## Implementation

Added a fixed golden bincode 1.3 fixture for a representative valid stock:

- `Resilience::Medium` enum discriminant;
- `k_t = 1000.0`;
- `biomass_t = 420.0`;
- `landed_t = 17.0`.

The test asserts the exact 28-byte little-endian representation emitted by the previous derived struct serializer. This pins the persisted field order and encoding independently of the current deserializer.

## Regression and verification

- Added `fish_stock_bincode_snapshot_preserves_the_legacy_derived_field_layout` in `crates/mk_engine/src/regional/industry/fishery.rs`.
- Existing JSON/bincode round-trip and invalid-state serialization/deserialization tests remain unchanged.
- No stock parameters, simulation state, reference values, replay hashes, append-only histories, schema version or budget/offline limits changed.
- Exact-head CI is required after this commit. No local Cargo test is claimed.

## Next

Run the exact PR #109 fast Rust suite and the required desktop/backend/week/year acceptance jobs. Keep the PR draft until those pass on this exact head.
