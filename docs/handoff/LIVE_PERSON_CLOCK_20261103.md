# Maer-Ken Island engineering handoff: live person inspection clocks (2026-11-03)

## Canonical scope
- Repository: `dylmarriner/Maer-ken-island`, branch `fix/island-client-human-id-20261101`, draft PR #68. Do **not** confuse with the separate Maer-Ken repository.
- Acceptance: `WORK_REMAINING.md`, `HANDOFF.md`, `docs/IMPLEMENTATION_READY_ROADMAP.md`, `docs/ISLAND_SYSTEM_STATUS.md`, and `docs/island/DEVIATIONS.md`.
- Preserve deterministic Rust single-writer, append-only log/snapshot, replay hash, optional LLM, fail-fast invariants, offline and budget limits. This slice changes dashboard reads only, not the simulation.

## Implementation and verification
- Live person inspector now rejects malformed `tick`/`records_at_tick` and projections older than the last accepted person snapshot. Equal world ticks cannot regress the record-copy tick. A different person/source starts a fresh clock; reselecting the same person retains the guard; invalidate resets it.
- `records_at_tick` and `tick` are fields of the real `/api/world/humans/:id` response in `apps/island/src/serve/server.rs`, not invented client-only metadata.
- RED regression: stale and impossible projections were all rendered before the fix. GREEN: 60/60 dashboard Node tests and 9/9 Python repository tests; JavaScript syntax and `git diff --check` passed.
- The checkout and tests ran through OpenCommander `kubuntux` on `/dev/shm`, avoiding its full root filesystem. No live simulation state or persistent data changed.
- Code and handoff commit SHAs are recorded in git history; exact-head hosted CI must still be checked after push.

## Remaining checklist (not 100%)
- [ ] Confirm exact-head CI including Rust/desktop/installed backend/audit; do not count skipped nightly tests as passes.
- [ ] Execute and document week/year deterministic replay, reference-pack, water/carbon/material ledger and replay-hash acceptance; repin digests only after audited proof.
- [ ] Complete sourced ecology, timber routes, industrial workflows, workforce, energy and climate gaps in `WORK_REMAINING.md`; record every realism deviation.
- [ ] Verify live dashboard with a real running Rust island and delayed/reordered HTTP projections.
- [ ] Resolve upstream divergences and owner-dependent reviews; do not alter simulation governance or deploy without separate authorization.

## Next engineering slice
Verify exact-head hosted CI, then run a deterministic week/year acceptance on a disposable offline state directory and record audit results before any physics or industry change.
