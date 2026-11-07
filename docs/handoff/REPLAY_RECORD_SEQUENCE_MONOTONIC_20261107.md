# Replay command-sequence allocation handoff

Date: 2026-11-07
Repository: `dylmarriner/Maer-ken-island`
Branch: `fix/replay-record-sequence-monotonic-20261107`
Base: `main` after PR #121 merge `522aba3d860435c92a4938cf5ffba2b427941cd7`

## Defect

`IslandReplayLog::record` counted only the contiguous same-tick suffix at the end of `entries`. If a public in-memory log was re-ordered or extended with a tick that already appeared earlier in the vector, the next command could reuse sequence zero even though that tick already had sequence values. The log would then contain ambiguous same-tick ordering.

## Change

- Allocate the next sequence as one greater than the maximum sequence already present for that tick, regardless of vector order.
- Preserve the existing log format, entry fields, append-only semantics and replay sorting by `(tick, sequence)`.
- No snapshot layout, RNG, canonical digest, simulation state, single-writer ownership or LLM behaviour changed.

## Regression

`recording_after_an_out_of_order_tick_uses_the_next_sequence_for_that_tick` records tick 5, then tick 6, then another tick-5 command, and asserts the tick-5 sequence set is exactly [0, 1].

## Commits

- Production: `3352d0070a486f360249bcfb6423271ba6560d35`
- Regression: `0eab8a485d50a917c9a45ade123124fdc8263851`



Additional regression: `a_refused_command_does_not_consume_the_next_replay_sequence` submits an invalid unnamed-person request, verifies no person or replay entry is created, then proves the next accepted control command still receives sequence zero. This protects the live index from advancing on a command that the island refuses.

## Verification

Exact-head CI is required for formatting, Clippy, fast tests, desktop, installed backend, dashboard, computer service, dependency audit and week/year reference-pack acceptance. No local Cargo result is claimed.

## Next

- Verify this exact branch head.
- Keep the PR separate from the paused replay merge-completion work and from existing PR #73–76.


## Review follow-up: fail closed on sequence exhaustion and remove the live history scan

The public in-memory log still derives its next sequence from the maximum sequence already present for that tick. This is deliberate: callers can construct and reorder its public `entries` vector, so the standalone `IslandReplayLog::record` path must inspect the actual entries. It uses `checked_add` and returns `ReplaySequenceExhausted` rather than reusing `u64::MAX`.

The live `IslandLife` path is different. Its replay log is now restricted to the regional module and exposed externally only through an immutable accessor. `IslandLife` maintains a runtime-only `BTreeMap<tick, last_sequence>`; `apply_command` reads the next sequence from that index and updates it only after the command succeeds. This removes the O(history) scan from every live command (O(log distinct command ticks) lookup/update), while preserving public-log out-of-order semantics. The index is not snapshotted, serialized or included in the canonical state digest; bootstrap and restore both start with a fresh log and empty index.

Regression coverage:
- Public out-of-order log recording allocates tick 5 sequences 0 and 1 even with a tick 6 entry between them.
- Public sequence exhaustion at `u64::MAX` returns a typed error without appending.
- 4,096 live same-tick control commands receive the contiguous sequence range 0–4,095.
- Existing live replay test proves a tick change restarts the sequence at zero.

## Verification

The prior exact-head workflow [#42358501995](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42358501995) failed only at `cargo fmt --all -- --check`; Clippy and fast tests were skipped. The formatter output was applied before this follow-up. The new exact-head workflow must pass formatting, Clippy, fast Rust tests, desktop, installed backend, dashboard, computer service, dependency audit and week/year reference-pack acceptance before merge. No local Cargo result is claimed.
