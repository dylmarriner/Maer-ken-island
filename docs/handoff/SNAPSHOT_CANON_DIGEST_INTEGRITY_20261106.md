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


## Replay envelope tick integrity

The replay snapshot envelope carries a public `tick` beside the digested state payload. That tick was not compared with the deserialized world's tick, so a valid payload digest could accompany contradictory envelope metadata. `WorldState::from_snapshot` now rejects that mismatch before schema validation or state exposure. A regression changes only the envelope tick and proves rejection while leaving the payload digest valid.


## Mixed legacy ranges after speciation

Codex identified a legacy ordering edge case: the old drift floor ran before speciation, so parents could be floored to `1.0` while a newly created child retained half the parent's range. The previous all-species-at-least-one detector missed this mixed state. Schema-0 validation now treats a non-empty speciation history plus any fractional species range as ambiguous and rejects it rather than silently upgrading. Regression tests exercise both file-based and replay snapshot loaders. Because schema 0 has no persisted parent-child provenance, a lossless reconstruction cannot be proven.


## Exact-head formatting correction — mixed legacy regression

Workflow [#42169111579](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42169111579) reached the latest mixed-range code and failed only at `cargo fmt --all -- --check`. Rustfmt requested three line wraps in the new file/replay regression tests. Clippy and fast tests were skipped in that run. The formatter output is applied in the next commit; fresh exact-head CI is required.
