# Maer-Ken Island: inspected-person URL boundary (2026-11-02)

Canonical repository: `dylmarriner/Maer-ken-island`, PR #68 (`fix/island-client-human-id-20261101`), not the separate `Maer-Ken` repository. Roadmap: `WORK_REMAINING.md`, `HANDOFF.md`, `docs/IMPLEMENTATION_READY_ROADMAP.md`.

## Defect and implementation

`personRecordPath` previously used `encodeURIComponent(agentId)` without validating the ID. JavaScript does not percent-encode `.` or `..`; the browser normalizes those URL segments and can turn `/api/world/humans/..` into `/api/world/` while the authenticated client fetches the wrong endpoint. The shared path builder now rejects empty, non-string, whitespace-padded, dot-segment and control-character identifiers before constructing either live or stored inspection URLs. Valid IDs containing reserved URL characters remain encoded as one segment.

## Evidence

- RED: the new regression failed before the fix with a missing exception for `.` (15/16 tests passed).
- GREEN: `node --test apps/island/tests/*.test.mjs` **46/46 passed**; both changed-module JavaScript syntax checks, `cargo fmt --all -- --check`, and `git diff --check` passed on OpenCommander AMD.
- Previous exact-head [CI #41396138200](https://github.com/dylmarriner/Maer-ken-island/actions/runs/41396138200) passed all seven non-nightly jobs; slow replay tests were skipped. New exact-head CI remains required.

## Remaining

- [ ] Exact-head CI on PR #68, including Rust, desktop, installed backend and web.
- [ ] Capacity-safe ignored week/year deterministic replay-hash acceptance and water/carbon/material ledger closure; do not re-pin digests without full audit.
- [ ] Sourced ecology, industry, workforce milestones, reference packs/deviations and owner acceptance.

No Rust tick, single-writer, append-only log, snapshot, replay, budget or governance behavior changed. No live simulation or deployment occurred. Next: verify new CI and execute slow deterministic acceptance on a capacity-safe runner.
