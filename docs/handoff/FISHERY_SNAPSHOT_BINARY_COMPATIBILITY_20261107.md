# FishStock binary snapshot compatibility — 7 November 2026

## Finding

PR #109 replaced the derived `Serialize` implementation for `FishStock` with a manual fail-closed serializer that validates physical state before writing. The implementation intentionally preserves field order, but the existing round-trip test alone could not prove compatibility with bytes written by the former derived serializer: a serializer and deserializer that changed together could still round-trip successfully.

## Implementation

Added a fixed golden bincode 1.3 fixture for a representative valid stock:

- `Resilience::Medium` enum discriminant;
- `k_t = 1000.0`;
- `biomass_t = 420.0`;
- `landed_t = 17.0`.

The test asserts the exact 28-byte little-endian representation emitted by the previous derived struct serializer, then deserializes those fixed legacy bytes into `FishStock`, compares every field and re-serializes to the identical byte sequence. This pins both the writer layout and the reader's backward compatibility; a same-version round trip alone would not catch both sides drifting together.

## Regression and verification

- Added `fish_stock_bincode_snapshot_preserves_the_legacy_derived_field_layout` in `crates/mk_engine/src/regional/industry/fishery.rs`.
- Existing JSON/bincode round-trip and invalid-state serialization/deserialization tests remain unchanged.
- No stock parameters, simulation state, reference values, replay hashes, append-only histories, schema version or budget/offline limits changed.
- Exact-head CI is required after this commit. No local Cargo test is claimed.

## Next

Run the exact PR #109 fast Rust suite and the required desktop/backend/week/year acceptance jobs. Keep the PR draft until those pass on this exact head.


## Legacy snapshot read regression — 7 November 2026

The golden fixture now verifies the read path as well as the write path:

- The current serializer must emit the fixed 28-byte legacy representation.
- The current deserializer must read those exact bytes and restore the expected `FishStock`.
- Re-serializing the restored value must reproduce the same bytes.

This closes the gap where the new serializer could match the old byte layout while a future deserializer change silently stopped reading historical snapshots.

The preceding exact-head workflow [#42230618280](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42230618280) passed fast Rust tests, formatting, Clippy, desktop build/render/input/frame-cost acceptance, installed backend, week/year reference acceptance, web dashboard, computer service, repository scripts and dependency audit. This new legacy-read assertion is a follow-up after that green head; it requires a fresh exact-head workflow. No local Cargo pass is claimed.

No snapshot format, schema version, simulation state, replay hash, reference value, stock parameter, offline limit or budget limit changed.


## Subnormal MSY preservation — 7 November 2026

A further review found that `msy_t` avoided overflow by multiplying carrying capacity by the peak fraction before applying the resilience rate. For subnormal carrying capacities this intermediate product can round down (or to zero), even when the final MSY is representable.

- `msy_t` now detects a subnormal scaled capacity and applies the resilience rate to the small carrying capacity before applying the bounded peak fraction.
- The branch is safe from intermediate overflow because the piecewise MSY surplus fraction is bounded below by `4/27`; if the fraction-scaled capacity is subnormal, the unscaled `rate * K` remains far below `f64::MAX`.
- Regression covers 1, 3 and 7 units of the smallest subnormal value with `r = 4` and peak fraction `0.25`, where the exact result is the original carrying capacity.
- No reference values, stock calibration, snapshot format, replay hash, simulation state, offline limit or budget limit changed.
- The preceding exact-head workflow [#42237465516](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42237465516) passed all required jobs on the legacy-snapshot-read head. This new source/test change requires its own exact-head CI run.


## Exact-head formatting correction — 7 November 2026

Workflow [#42248051888](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42248051888) reached the new Rust source and stopped at `cargo fmt --all -- --check` on one chained assertion in the subnormal MSY regression. The formatter's exact line wrap is applied in the follow-up commit. Clippy and fast tests did not run in that attempt; the other parallel jobs were still in progress when the branch was superseded. A fresh exact-head workflow is required.
