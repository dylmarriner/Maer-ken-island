# Replay session default execution budget

Date: 2026-11-05

## Change

The Phase 5 `ReplaySession` API previously defaulted `ReplayOptions.max_ticks` to zero, which means unlimited work. That bypassed the bounded default already used by the regional replay-log API. The default is now 600,000 world steps, enough for a year at a 60-second cadence. A caller may still explicitly set `max_ticks: 0` for an intentionally unbounded run.

The existing `replay_to` guard rejects a requested horizon beyond the configured limit before stepping the world. Regression coverage asserts the default is bounded, the over-budget request leaves the tick and history unchanged, and unlimited replay requires an explicit zero override.

## Verification

- Source commit: e3c9847dc450bf834191468c25b6d941bf656d1d
- Exact-head CI is required for formatting, Clippy, fast tests, desktop/backend checks and the reference acceptance gates.
- No local Cargo test pass is claimed; the custom remote session is unavailable in this run.
- No simulation state, replay-log format, snapshot format, physical constants or production infrastructure changed.

## Remaining

- Inspect the exact-head workflow result before review or merge.
- Ensure no callers depend on the previous implicit-unlimited default; callers requiring a longer horizon must now set an explicit budget.


## CI follow-up (2026-11-05)

- Initial exact-head workflow [#41890163537](https://github.com/dylmarriner/Maer-ken-island/actions/runs/41890163537) failed only at `cargo fmt --all -- --check`; Clippy and fast tests were skipped by the workflow after formatting failed.
- Corrected the test module spacing and fixture-path wrapping in source commit 28ef5226f66bb6a897b083aea1505f0d1f3704e6.
- The current branch handoff was reconciled with the latest `main` handoff after PR #77 merged as `750840e3160893888bcf2efab25ac7cc34d8fdbe`. The replay-session change remains separate in PR #78.
- A fresh exact-head workflow must pass before review/merge; no Rust test pass is claimed yet.
