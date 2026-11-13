# D40 cross-cut source-log invariant — 2026-11-10

PR #136. Production and regression commit `bb81790d47e6b8c23e0a6d13e7dd35210481de41`.

A restored haul's `CrossCut` and `CrossCutCrew` task must reference the job's source `log_id`. The persisted-state guard now rejects cross-cut tasks for a different log. Regression covers both single-founder and crew variants. No snapshot schema, replay hash, RNG, material ledger or simulation constants were changed.

**Verification:** GitHub accepted commit; exact-head rustfmt/Clippy/tests/replay have not been run yet. Obtain fresh exact-head CI and fix any failures before merging. Review the previously reported libsqlite3-sys advisory via dependency tracing and a compatible upgrade rather than suppression. Reconcile overlapping #134/#135; D40 terrain-aware routes/persistent vehicle positions and D31 sourced regional soil nutrients remain unfinished. Do not deploy or mutate live simulation state.
