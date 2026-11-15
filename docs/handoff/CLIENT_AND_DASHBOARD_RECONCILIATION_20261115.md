# Client / dashboard integration against current main, 15 November 2026

## Purpose

PR #68 contains source-backed web and Rust client protections for human-ID path/query injection, scoped owner tokens, response budgets, live People roster and inspector refresh. Those changes remained on an old draft branch. Integration with current `main` produced six conflicts in CI, historical handoff documents, map UI and styling. The Rust client security changes merged without conflict.

## Resolution

- Preserve the modern globally applied CI debug/incremental settings, the fast-test `CARGO_BUILD_JOBS=2` gate and the newer town and marine dashboard checks. Retain PR #68's full dashboard tests and cell-environment tests.
- Keep the newer shared `handleMapCellClick` helper and marine projection; compose it with PR #68's local biome/annual-climate display and latest-click-wins selection gate. Stale map results and errors no longer replace a later click.
- Keep both cell-environment and marine CSS rules. Preserve both sides' historical status/handoff information rather than erasing recently merged physics/material progress.
- Adjust the old CI disk-budget regression to accept `CARGO_PROFILE_TEST_DEBUG` and `CARGO_PROFILE_DEV_DEBUG` set globally, or locally on the fast-test job, without weakening the resource budget.
- Keep the Rust client human-ID, endpoint, token/redirect and bounded-response guards. Do not change simulation RNG, tick cadence, scientific constants, snapshot format or deterministic replay.

## Red-to-green evidence on AMD

- Two added `scripts/tests/marine_cell_inspector.test.mjs` cases failed before the shared-helper repair (stale click overrode the latest and environmental details were missing), then passed.
- `node --test apps/island/tests/*.test.mjs`: 65 passed.
- `node --test scripts/tests/*.test.mjs`: 12 passed, including the two new regressions.
- `python3 -m unittest discover -s scripts/tests -v`: 9 passed after updating the stale configuration assumption.
- `cargo test -p mk_island_client --locked`: 8 unit tests and 1 doc test passed.
- `cargo clippy -p mk_island_client --all-targets --locked -- -D warnings`: passed.
- `cargo fmt --all -- --check`: passed.

Full exact-head GitHub CI, including application/backend and reference-pack acceptance, must pass before PR #68 merges.
