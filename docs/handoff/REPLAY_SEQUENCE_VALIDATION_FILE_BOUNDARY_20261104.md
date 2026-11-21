# Replay sequence validation at file boundaries — 4 November 2026

- Canonical repository: `dylmarriner/Maer-ken-island`
- PR: [#74](https://github.com/dylmarriner/Maer-ken-island/pull/74), stacked on PR #73
- Branch: `test/replay-special-file-refusal-20261104`
- Code commit: `08bac8ab5c72ba3a3d8b9512d4fddbd474d20a10`
- Regression commit: `3f5895c7c74d5f2db25dbd0b063a6938de53bef1`

## Change

Centralised replay command sequence validation in `ordered_replay_entries`. The shared boundary now sorts references by (tick, sequence), then rejects duplicate or missing zero-based sequence IDs without rewriting the persisted vector order.

The same validation is applied when saving, loading, and replaying. This prevents the writer from creating a file the reader or replay engine would later reject, while still accepting a valid log whose entries were reordered in transit.

## Regression coverage

- Load refuses a serialized log with duplicate same-tick sequence IDs before returning it.
- Save refuses a sequence gap and leaves no output file.
- Save/load accepts a shuffled but contiguous log and replay reaches the same state digest as replaying the original log.

The change does not alter simulation state, command effects, canonical digest rules, snapshot formats, material ledgers, or owner governance.

## Verification

- Source and regression tests are pushed to the stacked PR branch.
- The new exact-head CI must confirm formatting, Clippy, fast tests, desktop and installed-backend checks.
- Earlier run #41781700984 passed formatting, Clippy, fast tests, desktop application and installed-backend checks on the prior head; the nightly slow-test job was skipped. Those results do not certify this new head.
- No local Cargo execution was performed through the connected GitHub interface.

## Remaining checklist

- [ ] Verify all exact-head fast and desktop checks.
- [ ] Run mandatory week and year reference-pack slow tests.
- [ ] Verify replay digest equality and save/load acceptance on the new head.
- [ ] Continue `WORK_REMAINING.md`: material-ledger closure, sourced ecology and industry, climate, human agency, fitness, genetics, free will and Phase 5 acceptance.
- [ ] Merge PR #73 only after its required acceptance gates are satisfied, then rebase/retarget PR #74 to main and re-verify.
- [ ] Leave separate PR #68 and the paused merge-completion job untouched.
