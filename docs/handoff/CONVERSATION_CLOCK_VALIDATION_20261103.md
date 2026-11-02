# Maer-Ken Island handoff: conversation projection clock validation (2026-11-03)

## Canonical source and scope
- Repository: `dylmarriner/Maer-ken-island`, draft PR #68, branch `fix/island-client-human-id-20261101`.
- Parent before this slice: `caa1e92`. This is not the similarly named Maer-Ken repository.
- `WORK_REMAINING.md` is the canonical unfinished-work register; `HANDOFF.md`, `docs/ISLAND_SYSTEM_STATUS.md`, and `docs/island/DEVIATIONS.md` define current state and realism constraints.

## Implemented
- The authenticated `/api/conversations` endpoint includes the published simulation tick, but the browser's feed validator did not check it. Malformed/future feed clocks or exchanges could appear to be valid dialogue.
- `validateConversationFeed` now requires any supplied projection tick to be a nonnegative safe integer, and prevents a conversation's first/last tick from exceeding that projection tick. Legacy stored feeds without a top-level tick remain supported.
- No Rust simulation, single-writer logic, append-only logs, snapshots, replay hashes, LLM budgets or live state changed.

## Verified on OpenCommander amd
- RED before implementation: new projection-clock regression failed (missing exception).
- GREEN after implementation: **55/55** dashboard Node tests; **8/8** Rust `mk_island_client` tests offline/locked.
- JavaScript syntax, `cargo fmt --all -- --check`, and `git diff --check`: passed.
- Previous exact-head CI for `caa1e92`, run #41477508688, passed all non-nightly jobs; slow/nightly acceptance was skipped. New commit requires its own CI.

## Remaining acceptance checklist
- [ ] Confirm new exact-head CI (Rust, dashboard, dependency audit, desktop, installed backend).
- [ ] Run capacity-safe week/year deterministic replay-hash and material water/carbon ledger acceptance.
- [ ] Continue sourced ecology, industry, timber and workforce milestones in `WORK_REMAINING.md`; record any deviation in `docs/island/DEVIATIONS.md`.
- [ ] Reconcile PR #68 and separate town dashboard PR #70 only after safe green reviews.

## Next engineering slice
Run the slow week/year replay and ledger gates on a machine with sufficient resources, then select the highest-impact unblocked sourced realism task. Do not repin digests before audits pass.
