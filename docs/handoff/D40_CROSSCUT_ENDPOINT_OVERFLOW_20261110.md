# D40 cross-cut geometry overflow, 2026-11-10

## Change
- Commit `50de14c434c6da3b122ea0ecedf3f1c77a336ba5`: fail fast if a finite source log projects to non-finite butt/tip coordinates before materialising cross-cut pieces.
- Regression constructs finite input fields with an overflowing butt coordinate and expects an explicit panic before allocation.

## Verification
- GitHub accepted the source and regression commit on PR #136 branch. Rust formatting, Clippy, fast tests and replay acceptance **not yet verified** for this new head.

## Remaining
- [ ] Run `cargo fmt --all -- --check`, Clippy and focused cross-cut test, then full fast Rust tests.
- [ ] Verify deterministic replay and week/year packs, desktop and installed backend.
- [ ] Reconcile PRs #134/#135 with #136; preserve single-writer logs/snapshots and offline limits.
- [ ] Terrain-aware Ute routing, persistent positions and D31 nutrient work.

## Next
Check exact-head CI; fix any formatting or regression failures, then continue D40 transport modelling. No production deployment or destructive operation performed.
