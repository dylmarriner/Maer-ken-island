# Maer-Ken Island handoff: bounded conversation feed (2026-11-03 NZDT)

## Canon and scope
- Repository: `dylmarriner/Maer-ken-island`, draft PR #68 `fix/island-client-human-id-20261101`. This is **not** the separate `Maer-Ken` repository. PR #70 remains independent.
- Read `WORK_REMAINING.md`, `docs/IMPLEMENTATION_READY_ROADMAP.md` and the prior `docs/handoff/AUTOMATION_ENGINEERING_HANDOFF.md`.

## Implemented and verified
- `apps/island/static/people-selection.mjs`: conversation snapshot validation now rejects more than 40 conversations or 12 visible lines per conversation **before DOM rendering**, matching the Rust projection constants `CONVERSATIONS_SHOWN` and `LINES_SHOWN` in `apps/island/src/serve/projection.rs`. This bounds browser work on malformed/unexpected authenticated feeds without changing simulation output.
- New regression verifies the boundary (40 x 12 accepted, 41 conversations and 13 lines rejected). RED before fix: 16/17 focused Node tests passed; GREEN: 53/53 complete dashboard Node tests passed.
- `node --check` for changed module, `cargo test -q -p mk_island_client --lib --offline --locked` 8/8, `cargo fmt --all -- --check`, and `git diff --check` passed.
- Previous exact-head CI #41457747084 passed all non-nightly jobs. Slow week/year simulation acceptance remained skipped; new-head CI still required.

## Remaining
- [ ] Verify exact-new-head CI including dashboard, Rust, desktop, installed backend, dependency audit.
- [ ] Run capacity-safe ignored week/year deterministic replay, exact replay-hash equality and water/carbon/material ledger closure without changing pinned digests prematurely.
- [ ] Complete sourced ecology, industry, town/workforce, reference-pack/deviation milestones in `WORK_REMAINING.md` and owner acceptance.
- [ ] Coordinate PR #70 separately, preserving deterministic Rust single-writer, append-only log/snapshot, optional LLM and offline/budget limits.

Next engineering slice: capacity-safe replay/ledger acceptance on an isolated adequately resourced runner; no live simulation, canon, replay digest, ledger or governance changes authorized.
