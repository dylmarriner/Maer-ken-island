# PR #74: reconcile 65 locally preserved replay commits against modern main

Date: 18 November 2026. Branch: `test/replay-special-file-refusal-20261104`.

The original 65 unpublished commits were pushed without force as `f64e633..57f51da` and copied into the independent backup-drive worktree `mk-replay-65-commits-20261118`. This reconciliation worktree merges current `main` (after PR #73's security fix) into that history without rewriting or dropping any original commits.

`commands.rs`, `replay.rs`, and `island_replay.rs` overlapped with newer upstream work. Preserve the newer runtime-only per-tick sequence index, sparse strictly increasing imported sequence semantics, 64 MiB bounded I/O, no-follow source protections, and owner-only atomic temporary files. The old version's contiguous-zero-based sequence checks were **not** restored because current replay legitimately accepts sparse sequences.

## Missing boundary logic carried forward

- `save` refuses unsupported replay versions and noncanonical scenario digests **before creating or replacing any file**.
- `load` refuses noncanonical scenario digests after version decoding but before accepting imported state.
- Canonical digests are exactly 64 ASCII lowercase hexadecimal characters. The existing JSON format, replay index, tick order, randomness, and scientific constants are unchanged.
- The PR #73 security tests' fixture digest was corrected to canonical hex to match the stronger file format boundary.

## Verification

Three independent integration regressions failed against the modern unfixed loader: version save refusal, digest save refusal (including non-clobbering), and invalid digest import refusal. After the fix, the same 3/3 tests passed; both existing Unix replay file-security tests also passed. Complete exact-head CI and reference-pack acceptance are still required before merging this PR. Legacy standalone runner scripts and prior audit handoffs remain for historical provenance; they are not claims of current replay gap semantics.
