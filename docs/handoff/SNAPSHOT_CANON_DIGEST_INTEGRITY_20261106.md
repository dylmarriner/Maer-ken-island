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


## Exact-head formatting correction

Workflow [#42167966575](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42167966575) reached the new head and failed only at `cargo fmt --all -- --check`: three long temporary-directory expressions in `io/snapshot.rs` needed rustfmt line wrapping. No Clippy or Rust tests ran in that workflow. Commit `e2f38185c6dfb776475960cadd07183e02b3e4cf` applies the exact formatting output. A fresh exact-head workflow is required; the failed run does not verify the implementation.


## Exact-head compile correction

Workflow [#42168311597](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42168311597) passed formatting and reached Clippy. Compilation then found two test-only E0382 partial moves: the `matches!` assertions moved the error message before the diagnostic formatted the whole error. Both patterns now borrow the message with `ref`; production restore logic was unaffected. Fresh exact-head CI is required.
