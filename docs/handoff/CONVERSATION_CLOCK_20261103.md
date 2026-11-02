# Maer-Ken Island handoff: conversation clock monotonicity (2026-11-03 NZDT)

## Scope and architecture
- Canonical project: `dylmarriner/Maer-ken-island`, draft PR #68; this is not the similarly named Maer-Ken project.
- Source: `WORK_REMAINING.md`, `docs/IMPLEMENTATION_READY_ROADMAP.md`, `apps/island/src/serve/projection.rs` and `/api/conversations` in `server.rs`.
- The server publishes a simulation tick with each conversation projection. The dashboard previously only rejected out-of-order *requests*, not a later request returning an older simulation tick.

## Implemented and checked
- `apps/island/static/people-selection.mjs`: monotonic conversation clock guard; invalid ticks fail closed; stale projections are ignored. An observed HTTP 404 resets the guard for a new island epoch. Hidden-tab suspension does not reset it.
- `apps/island/static/people.js`: apply the guard before changing visible conversations. Preserve the last valid dialogue on stale success.
- Regression: `apps/island/tests/people-selection.test.mjs`, failed before implementation due to missing export; passed afterward.
- OpenCommander amd: 37/37 Node people-selection + roster tests passed; 8/8 offline Rust client tests passed; JS syntax checks, cargo fmt and git diff --check passed.
- Previous exact-head CI run #41468189918 on earlier commit 8ab4388 passed all non-nightly jobs; new-head hosted CI remains unverified.

## Remaining / next iteration
- [ ] Verify exact-head hosted CI; do not merge until green and reviewed.
- [ ] Run capacity-safe deterministic week/year replay and water/carbon/material ledger acceptance (slow nightly gate not run here).
- [ ] Continue sourced ecology, industry, workforce and reference-pack roadmap acceptance; review separate PR #70.
- [ ] Consider explicit simulation epoch ID in `/api/conversations` so an immediate restart without an observed 404 is distinguishable from a stale projection.

No Rust single-writer engine, logs, snapshots, replay digests, live simulation or governance changes. No deployment or destructive action.
