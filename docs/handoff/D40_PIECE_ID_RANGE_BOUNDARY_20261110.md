# D40 piece-ID range boundary, 2026-11-10

PR #136. Commit `f84bb70c96f9642228091ac85d926594099b8046`.

The cross-cut planner now checks one-based piece-ID range overflow before calling the assertive `Log::cross_cut` routine. Regression covers zero and `u64::MAX` starting IDs. The existing typed `InvalidHaulJobState` error is returned without mutating material state.

Verification: GitHub accepted commit; exact-head Rust formatting, Clippy, fast tests and replay acceptance NOT yet verified. Prior CI had a dependency security audit failure concerning libsqlite3-sys; recheck the current dependency chain and advisory.

Remaining: exact-head CI and audit remediation, #134/#135 integration, D40 terrain-aware routing/vehicle persistence, D31 nutrient model, full deterministic replay and offline acceptance.
