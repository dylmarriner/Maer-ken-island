# Replay sequence continuation after shuffled log entries — 4 November 2026

Canonical repository: `dylmarriner/Maer-ken-island`
PR: https://github.com/dylmarriner/Maer-ken-island/pull/73
Branch: `fix/replay-sequence-validation-20261104`
Parent head inspected: `11a39821b276a181e679e62ca9a9d31e642decaa`

## Change

`IslandReplayLog::record` now chooses the maximum existing sequence for a tick, rather than the last matching entry in the vector. Replay intentionally sorts by (tick, sequence), so a loaded or transported log whose vector order has changed could previously cause the recorder to reuse a sequence number and emit a log its own validator would reject.

A regression test reverses two same-tick entries, records a third command, and asserts that the sequence set remains contiguous at [0, 1, 2].

The change does not alter simulation state, tick behavior, canonical hashes, snapshots, material ledgers, append-only file format, optional LLM behavior, or owner governance.

## Verification

- Previous exact-head CI on `11a3982` completed successfully (run #41764151806), but the new code/test commit requires its own exact-head CI.
- The new runtime regression has not been run locally in this automation turn. Do not count it as passed until hosted or local test output confirms it.
- Required slow week/year reference-pack tests remain merge gates as specified in `WORK_REMAINING.md`.

## Remaining checklist

- [ ] Verify exact-head hosted CI for this commit.
- [ ] Run the required week and year reference-pack replay tests and confirm digest equality and ledger closure.
- [ ] Continue `WORK_REMAINING.md`: sourced ecology/industry, timber, resource recipes, climate and year acceptance.
- [ ] Keep PR #68 and the paused merge-completion work separate and untouched.
