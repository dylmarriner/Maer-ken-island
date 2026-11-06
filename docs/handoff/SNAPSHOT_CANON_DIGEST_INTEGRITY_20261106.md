# Snapshot canon-digest integrity

**Date:** 6 November 2026  
**Repository:** `dylmarriner/Maer-ken-island`  
**PR:** #106

## Defect

Both snapshot restore paths verified the outer BLAKE3 digest and validated `CanonLocked`, then recomputed `CanonDerived`. They did not compare the persisted `canon_derived.canon_digest` with the digest derived from the restored canon. A self-consistent payload could therefore contain an internally contradictory canon identity and be accepted while the loader silently replaced the derived metadata.

## Change

- File-based `io::snapshot::load_snapshot` now returns `SnapshotError::CanonMismatch` when the stored inner digest differs from the validated canon.
- Replay `WorldState::from_snapshot` now returns a typed deserialization error for the same mismatch.
- Derived values are recomputed and assigned only after the stored canon identity matches.
- Outer payload-digest verification and canon validation remain in their existing order.
- No snapshot layout, schema version, append-only log, RNG, or simulation-state format changed.

## Regression tests

- A file snapshot with a recomputed valid outer BLAKE3 digest but a forged inner canon digest is rejected.
- A replay snapshot with the same internally inconsistent identity is rejected.
- Existing valid snapshot round-trip tests continue to assert the canonical digest.

## Verification

Fresh exact-head CI is required. The current PR workflow must run formatting, Clippy, fast Rust tests, file/replay snapshot tests, desktop/backend checks and week/year reference acceptance. No local Cargo result is claimed.

## Safety

No persisted customer/world snapshot was rewritten, no migration or data repair was performed, and no live simulation state was changed.
