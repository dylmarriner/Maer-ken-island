# Maer-Ken Island engineering handoff: initial roster clock seeding (2026-11-03)

## Canonical scope
- Verified remote `dylmarriner/Maer-ken-island`, draft PR #68 (`fix/island-client-human-id-20261101`), fetched from head `08de299`. This is **not** the separate Maer-Ken repository.
- Canonical acceptance sources: `WORK_REMAINING.md`, `HANDOFF.md`, `docs/IMPLEMENTATION_READY_ROADMAP.md`, `docs/ISLAND_SYSTEM_STATUS.md`, `docs/island/DEVIATIONS.md`.
- No changes to deterministic Rust single-writer logic, append-only logs/snapshots, ledger/replay algorithms, optional LLM, or live simulation state.

## Verified change
- The initial `/api/world/humans` load displayed an accepted roster but did not initialize the live poller's monotonic tick/projection clocks. The first delayed/cache-backed poll could therefore rewind the already displayed people.
- `apps/island/static/people-selection.mjs` adds `seed(roster)`, validating initial roster data and preserving both independent clocks. It refuses stale re-seeding.
- `apps/island/static/people.js` seeds the poller after successful initial live-roster validation and before the UI adopts the roster.
- `apps/island/tests/people-roster.test.mjs` proves a first poll with an older world tick, or newer world tick with older record projection, cannot replace the initially accepted snapshot; a truly newer poll is accepted.
- RED: regression failed with `TypeError: poller.seed is not a function` (19/20 focused roster tests passed).
- GREEN on OpenCommander `amd`: focused roster **20/20**; full dashboard **61/61**; Python repository scripts **9/9**; JS syntax, `cargo fmt --all -- --check`, `git diff --check` passed.
- Code commit: `d502b84`.
- Previous exact-head hosted CI `41532335477` completed successfully for `08de299`; new head still requires hosted CI before merge. Nightly slow acceptance is not covered by fast PR CI.

## Remaining checklist
- [ ] Confirm full exact-head hosted CI (Rust, desktop, installed backend, dependency audit); keep draft PR #68 unmerged until reviewed and green.
- [ ] Run disposable offline deterministic week/year replay with matching hashes, water/carbon/material ledger closure and reference-pack acceptance.
- [ ] Exercise initial roster and delayed HTTP responses against an actual Rust island backend.
- [ ] Continue sourced ecology, industry, workforce, energy and climate roadmap; update deviations for real-world departures.
- [ ] Reconcile PR #70 separately, preserving upstream import rules and owner-governed decisions.

## Next engineering slice
Use a disposable state directory for the slow week/year acceptance and ledger audit, then implement the next sourced roadmap milestone without changing production state.
