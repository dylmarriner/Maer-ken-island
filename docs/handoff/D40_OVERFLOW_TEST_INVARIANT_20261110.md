# Maer-Ken Island handoff: overflow regression invariant

PR #136. Commit `65521c50638b062f584d50c6810a0580a4446914` corrects an internally contradictory test: finite inputs whose projected endpoint overflows are invalid physical state. The test now asserts `!has_valid_physical_state()` and expects the existing physical-state panic before cross-cutting. Production geometry is unchanged.

Verification: GitHub write accepted; exact-head rustfmt, Clippy, Rust tests, replay/reference packs and dependency audit **not run**. Do not merge until green.

Remaining: investigate libsqlite3-sys security advisory and compatible dependency upgrade; reconcile PRs #134/#135; implement terrain-aware Ute routing, persistent positions and D31 nutrient modelling. Next: run exact-head Rust suite and audit, then address failures.