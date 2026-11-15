# Maer-Ken Island engineering handoff: monotonic roster projection clock (2026-11-03)

## Canonical scope
- Remote `dylmarriner/Maer-ken-island`, branch `fix/island-client-human-id-20261101`, draft PR #68. Not the separate Maer-Ken repository.
- Roadmap: `WORK_REMAINING.md`, `HANDOFF.md`, `docs/IMPLEMENTATION_READY_ROADMAP.md`, `docs/ISLAND_SYSTEM_STATUS.md`, `docs/island/DEVIATIONS.md`.
- Preserve deterministic Rust single-writer, append-only snapshots/logs, replay-hash equality, optional LLM and offline/budget limits. This change is read-only dashboard logic, not simulation physics or governance.

## Engineering slice and test evidence
- Previously, a live roster poll with a later world tick but an older `records_at_tick` was accepted, replacing newer inspected roster records with stale population data.
- Enforce monotonic `tick` and `records_at_tick` independently. Retain the last accepted snapshot on stale data and accept the next genuinely newer projection; explicit `invalidate()` still permits a known new simulation epoch, while `suspend()` preserves clocks.
- RED: modified Node regression observed stale `[13,11]` rendered after accepted `[12,12]`. GREEN: 19/19 focused roster tests, 60/60 dashboard tests, 9/9 Python repository tests, JS syntax, `cargo fmt --all -- --check` and `git diff --check` passed on OpenCommander `amd`.
- The preceding PR head `efa0895` had completed hosted CI successfully: https://github.com/dylmarriner/Maer-ken-island/actions/runs/41522888536. New-head hosted CI is still required; no week/year acceptance claimed.
- Python tests generated only bytecode cache changes; restored those test artifacts before commit. No simulation, live data or infrastructure modified.

## Remaining checklist
- [ ] Verify full exact-head CI including Rust, desktop, installed backend and dependency audit; do not count skipped nightly checks as passes.
- [ ] Execute disposable offline week/year deterministic replay, reference-pack and water/carbon/material ledger acceptance; verify replay hashes.
- [ ] Validate roster behavior against an actual Rust island with delayed/out-of-order HTTP responses.
- [ ] Continue sourced ecology, industry, workforce, energy and climate milestones; record deviations.
- [ ] Review upstream divergence and PR #70 separately before merging.

## Next slice
Run deterministic week/year and ledger acceptance on a disposable offline state directory, capture exact SHA and audit evidence, then advance the next roadmap milestone. No deployment or simulation-governance changes were authorised.
