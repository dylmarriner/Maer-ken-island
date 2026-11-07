# Replay command-sequence allocation handoff

Date: 2026-11-07
Repository: `dylmarriner/Maer-ken-island`
Branch: `fix/replay-record-sequence-monotonic-20261107`
Base: `main` after PR #122 merge `1bc91d7a09778891a4f2b4d25b9b3cb400cbfcdf`

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

## Verification

Exact-head CI is required for formatting, Clippy, fast tests, desktop, installed backend, dashboard, computer service, dependency audit and week/year reference-pack acceptance. No local Cargo result is claimed.

## Next

- Verify this exact branch head.
- Keep the PR separate from the paused replay merge-completion work and from existing PR #73–76.
