# Maer-Ken Island engineering handoff: conversation attribution integrity (2026-11-03 NZDT)

## Canonical scope
Repository `dylmarriner/Maer-ken-island`, ongoing draft PR #68 `fix/island-client-human-id-20261101`. Do not conflate this with the separate Maer-Ken repo or revive its paused merge job. Sources: `WORK_REMAINING.md`, `docs/IMPLEMENTATION_READY_ROADMAP.md` and the island reference/deviation registers.

## Implemented and evidence
- `apps/island/static/people-selection.mjs`: the authenticated conversation feed now validates operator-visible relationship labels, speaker names and optional speaker IDs with the same invisible-control/dot-segment/size constraints as inspected human identities. Legacy feeds without `speaker_id` remain supported; simulation dialogue text is unchanged.
- `apps/island/tests/people-selection.test.mjs`: regression rejects bidi/zero-width control metadata, dot-segment IDs, null and non-string IDs while accepting a real valid conversation. RED 1/21 failed; GREEN 21/21 focused and 57/57 complete dashboard Node tests.
- 9/9 Python repository tests, JavaScript syntax, `cargo fmt --all -- --check` and `git diff --check`: passed.
- A supplemental local `cargo test --offline --locked -p island --test client` attempt exceeded the 210-second remote command limit during compilation, so no new local Rust pass is claimed. Prior exact-head hosted CI #41503853231 passed all non-nightly jobs; slow nightly acceptance was skipped.

## Exact remaining checklist
- [ ] Verify exact new-head GitHub CI, including Rust, installed backend, desktop, dependency audit and dashboard.
- [ ] Run capacity-safe ignored week/year deterministic replay, replay-hash equality, and material/water/carbon ledger acceptance on an adequately provisioned runner.
- [ ] Continue sourced ecology, industry and workforce milestones, track any deviations and keep single-writer/snapshots/optional-LLM/budget invariants.
- [ ] Review PR #70 (Town dashboard) separately; do not silently merge into PR #68.
- [ ] Obtain owner acceptance only after documented roadmap criteria are verified.

No production simulation, governance, data deletion, live infrastructure or autonomous state changes.
