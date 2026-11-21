# Bounded replay serialization — 4 November 2026

- Canonical repository: dylmarriner/Maer-ken-island
- Follow-up branch: test/replay-special-file-refusal-20261104
- PR: #74, stacked on PR #73
- Scope: replay-log save boundary and regression coverage

## Change

The replay save path previously serialized the entire log into a Vec and only then checked the 128 MiB limit. A too-large in-memory log could therefore allocate its full serialized representation before being rejected.

Save now performs an allocation-free counting serialization first. It returns the existing TooLarge error with the exact serialized byte count before allocating the output buffer; only logs within the limit are serialized to a Vec. The exact-limit boundary remains inclusive, and oversized logs still fail before scenario-digest validation.

## Regression coverage

A unit test verifies exact byte counting, rejection one byte below the serialized size, and acceptance at the exact size limit.

## Verification and blocker

- Current PR #74 head 865251b had passed its fast Rust/formatting CI before this change; that does not verify this new commit.
- Local Cargo test is blocked by the Pi's installed Cargo 1.65.0 / rustc 1.63.0. It fails dependency resolution in the workspace before compiling mk_engine because this old Cargo cannot resolve the current Warp 0.4 feature metadata. No dependency manifest was changed to work around the outdated local toolchain.
- Exact-head hosted CI must verify formatting, Clippy, fast tests and desktop/backend checks for the new commit. Week/year reference-pack tests remain mandatory before final acceptance.
- No simulation state, replay digest rules, snapshot format, material ledger, owner governance, production infrastructure or SCOS job was changed.

## Next

1. Verify exact-head CI.
2. Run the mandatory week/year reference-pack tests on a supported Rust toolchain.
3. Continue the remaining WORK_REMAINING.md acceptance checklist.
