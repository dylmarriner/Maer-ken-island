# Maer-Ken Island engineering handoff: hidden-tab clock monotonicity (2026-11-03 NZDT)

Canonical repository: `dylmarriner/Maer-ken-island`, PR #68 `fix/island-client-human-id-20261101`, distinct from `Maer-Ken` and the independent town-dashboard PR #70. Read `HANDOFF.md`, `WORK_REMAINING.md`, `docs/IMPLEMENTATION_READY_ROADMAP.md` and `docs/island/DEVIATIONS.md`.

## Completed engineering slice
- Found that visibility/pagehide called `createLiveRosterPoller.invalidate()`, resetting the accepted simulation clock. After a tab was hidden and shown again, an older proxy/server roster could replace a newer visible roster in the *same* simulation epoch.
- Split `suspend()` (abort in-flight requests and invalidate old responses, preserving accepted tick and records-at-tick) from explicit `invalidate()` (reset monotonic guard for a verified new simulation epoch).
- Updated visibility/pagehide call sites to suspend. Retained explicit new-epoch reset semantics.
- RED: 18/19 focused roster tests passed before the fix, new suspend regression failed. GREEN: 52/52 complete dashboard Node tests; JS syntax, Cargo formatting and Git whitespace checks passed on OpenCommander AMD.
- Previous exact-head #41447626735 passed all seven non-nightly hosted jobs; slow deterministic simulation was skipped. New exact-head CI is not yet verified.

## Remaining acceptance
- [ ] Verify exact-new-head Rust, desktop, installed-backend, dashboard, scripts and dependency audit CI.
- [ ] Run capacity-safe ignored week/year deterministic replay and material/water/carbon ledger closure, recording replay-hash equality without re-pinning digests prematurely.
- [ ] Continue sourced ecology, industry, workforce and reference-pack/deviation work in `WORK_REMAINING.md`; reconcile PR #70 independently and obtain owner acceptance.

Next: verify new CI, then execute slow deterministic replay/ledger acceptance on an adequately resourced isolated runner. No Rust single-writer, append-only logs, snapshots, replay hashes, simulation tick, canon, live simulation or governance changes were made.
