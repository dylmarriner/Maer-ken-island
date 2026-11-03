# Maer-Ken Island engineering handoff: live inspector identity binding (2026-11-03 12:46 NZDT)

## Canonical repository and architecture
- Confirmed `dylmarriner/Maer-ken-island`, draft PR #68 `fix/island-client-human-id-20261101`; fetched and fast-forwarded from `08de299` to `e3ed26f` before editing. Not the separate `Maer-Ken` repository.
- Acceptance sources: `WORK_REMAINING.md`, `HANDOFF.md`, `docs/IMPLEMENTATION_READY_ROADMAP.md`, `docs/ISLAND_SYSTEM_STATUS.md`, `docs/island/DEVIATIONS.md`.
- Only browser inspector and its tests changed. Rust deterministic single-writer, append-only event logs/snapshots, replay hashes, ledger, LLM and budgets untouched.

## Verified code change
- `apps/island/static/people-selection.mjs`: live inspector checks `record.summary.agent_id` equals the currently selected human before rendering or advancing monotonic clocks. Rejects mismatched and absent identities, preventing misrouted/cached records from appearing under another person's name.
- `apps/island/tests/people-selection.test.mjs`: regression for a correct record, a wrong human, a missing identity, then a correct newer record; existing live test fixtures now carry the real Rust inspector identity shape.
- RED: 24/25 focused tests passed; regression displayed ticks 8 and 9 for wrong/missing identities. GREEN: 25/25 focused; 62/62 dashboard JS tests, 9/9 Python repository tests, JavaScript syntax, Cargo formatting and Git diff checks passed on OpenCommander `amd`.
- Code commit `37095d8`. Previous exact-head hosted CI `41541218688` passed all required jobs at `e3ed26f`; new head requires its own CI. Slow week/year tests not run this slice.

## Remaining checklist
- [ ] Confirm exact-head hosted CI and review; keep draft PR #68 unmerged until verified.
- [ ] Run disposable offline week/year deterministic replay, replay-hash equality and water/carbon/material ledger closure; reference-pack checks.
- [ ] Exercise live person selection against actual Rust backend under delayed/misrouted HTTP responses.
- [ ] Continue sourced ecology, industry, town/workforce, energy and climate roadmap and deviation register.
- [ ] Reconcile PR #70 independently, preserving upstream import and owner-governed constraints.

## Next engineering slice
Run slow acceptance in disposable state with bounded resources; then take the next sourced roadmap item without modifying live simulation data.
