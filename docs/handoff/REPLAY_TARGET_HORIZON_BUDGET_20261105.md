# Replay target-horizon budget regression — 5 November 2026

Canonical repository: `dylmarriner/Maer-ken-island`
PR: [#76](https://github.com/dylmarriner/Maer-ken-island/pull/76)
Branch: `fix/replay-execution-budget-20261105`

## Defect

The replay budget guarded included commands but not the requested `until_tick`. A caller could request a horizon beyond the configured limit with an empty log or with all commands earlier than the limit, then force the simulation loop to advance beyond the budget.

## Change

- Validate the complete log header and sequence contract first.
- Reject `until_tick > max_replay_ticks` before island bootstrap.
- Preserve partial replay when a future command lies beyond the requested tick.
- Generalize the error text so it describes both a requested horizon and a command tick.

## Regression

`replay_refuses_requested_horizon_beyond_explicit_budget` uses a small explicit limit and proves the requested target is rejected with `ReplayBudgetExceeded`. The existing far-future-command and partial-replay tests remain in place.

## Verification

Source and regression are committed to PR #76. Exact-head GitHub Actions must pass before this change is accepted. No local Cargo test pass is claimed in this run.

## Remaining

- Verify the new exact-head fast tests, Clippy, formatting, desktop and backend checks.
- Run the required week/year reference-pack tests and fixed-seed replay digest audit.
- Reconcile the existing canonical week-digest disagreement before any re-pin.
- Keep PR #76 stacked on #75 → #74 → #73; do not merge out of order.

No simulation state, snapshot format, replay-log serialization, real-world constants, owner governance, production infrastructure or SCOS state changed.


## CI correction — 5 November 2026

- Exact-head run [#41841623263](https://github.com/dylmarriner/Maer-ken-island/actions/runs/41841623263) exposed one rustfmt-only difference in the new explicit-budget assertion; no Clippy or test failure was reported because the check job stopped at formatting.
- Applied the formatter's exact multiline layout in commit [`c4ce89e`](https://github.com/dylmarriner/Maer-ken-island/commit/c4ce89e84ae05acd36e6dc1f9102d2afd14c0857).
- A new exact-head run is required. Desktop/backend checks from the prior head are not evidence for this corrected head.
