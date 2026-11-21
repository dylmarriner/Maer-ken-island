# D40 shared tip projection — 2026-11-10

PR #136. Commit `03b94c7c362e067cf75f31936ca4c165fea77251`.

Extracted `Log::tip_m()` and reused it in physical-state validation and cross-cut. The finite endpoint regression additionally asserts that the tip projection itself overflows in the malformed test case. This removes duplicate geometry arithmetic while preserving the deterministic formula, snapshot schema and event semantics.

Verification: GitHub commit confirmed; Rust fmt/Clippy/tests/replay and dependency audit NOT executed on this head. Run exact-head checks before merge. Previously observed SQLite advisory needs current dependency-tree review and remediation.

Remaining: exact-head CI, D40 terrain routing and persisted vehicle position, reconcile #134/#135, D31 nutrient work, full deterministic acceptance.
