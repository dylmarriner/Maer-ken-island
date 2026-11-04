# Replay execution budget — 5 November 2026

Canonical repository: `dylmarriner/Maer-ken-island`
Follow-up PR: #76 (stacked on PR #75 → #74 → #73)
Branch: `fix/replay-execution-budget-20261105`

## Defect

A structurally valid replay log can contain a command at an extremely distant tick. Replaying to that tick advances the simulation once per human tick from bootstrap; a tiny input file could therefore request effectively unbounded CPU work even though the file-size limit is satisfied.

## Change

- Add a default per-call replay horizon of 600,000 human ticks. The canonical scenario uses 60-second ticks, so the default permits a full year (525,600 ticks) plus margin.
- Return `ReplayBudgetExceeded` before island bootstrap when an included command exceeds the default horizon.
- Add `replay_island_with_budget` for callers that intentionally need a longer replay and explicitly choose a higher absolute tick ceiling.
- Keep full-log header and command-sequence validation before filtering to `until_tick`; future commands do not consume the current partial replay's execution budget.

The budget is an execution guard, not a change to the serialized log format or simulation state. Longer histories should use a verified snapshot or explicitly raise the budget.

## Regression coverage

- A far-future command is refused by the default replay API.
- A partial replay ending before that command remains valid.
- Existing replay digest, save/load and sequence tests remain in the same suite.

## Verification

No local Rust test pass is claimed. Exact-head CI must compile and execute the new regression before acceptance. The current parent PR #75 remains untouched while its CI runs.

## Remaining

1. Verify fast Rust tests, Clippy, formatting, desktop and backend jobs on this branch.
2. Run the required week/year reference-pack tests and fixed-seed replay digest checks.
3. Rebase this follow-up after PRs #73–#75 are accepted, then rerun exact-head CI.
4. Keep the default limit aligned with supported replay budgets; do not re-pin the week digest for this execution-only guard.

No simulation state, snapshot format, real-world constants, owner governance, production infrastructure or SCOS state was changed.
