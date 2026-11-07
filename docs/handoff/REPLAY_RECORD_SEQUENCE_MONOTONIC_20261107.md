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


## Exact-head formatting correction

Workflow [#42366006668](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42366006668) on the new refused-command regression reached `cargo fmt --all -- --check` and found only the multiline layout for the final sequence assertion in `island_replay.rs`. Commit `b12718f419c0bc9c483f52242141712076265047` applies the exact formatter output. The format failure is a real source-level CI failure; a fresh exact-head run must pass formatting, Clippy, fast tests and all remaining acceptance jobs before merge.


## Preserve source sequence identifiers during replay

A further replay audit found that `replay_island` sorted imported entries by `(tick, sequence)` but then called the live `apply_command` allocator, which renumbered sparse imported sequences contiguously. State replay remained deterministic, but the returned IslandLife replay log no longer preserved the source audit identifiers.

The replay path now calls a private `apply_command_with_sequence` helper. It accepts strictly non-decreasing per-tick identifiers (gaps are valid), rejects backwards identifiers before command mutation with `CommandError::ReplaySequenceOutOfOrder`, and records the original sequence. The public live path still allocates the next contiguous sequence from the runtime-only BTreeMap. No snapshot, digest, RNG, command payload or log format changed.

Regression: `replay_preserves_sparse_recorded_sequence_numbers` supplies shuffled tick-0 entries at sequences 12 and 7 and asserts the returned log is ordered as 7, 12 with the original identifiers intact.

Commits: `60e592120fb2980bc1b2117b6c0e77bdab90aad2` (error contract), `23120a75af4c6b48a3ec1dff5d9f5549cd3cdab4` (replay path), `6803a75b2068c63792fdd384e13c6023eed4a885` (regression). Fresh exact-head CI is required.


## Public replay-log append performance and immutability — 8 November 2026

A current Codex review found that the public `IslandReplayLog::record` convenience API still scanned every retained entry for each append. The live `IslandLife::apply_command` path already used its own runtime per-tick index, but repeated public log construction could still become O(N²).

Correction:
- Made the replay entry vector private and exposed an immutable `entries()` slice. This makes the documented append-only log boundary enforceable instead of relying on callers not to mutate a public vector.
- Added `from_entries(version, scenario_digest, entries)` for importing or testing in-memory logs; replay still validates version, duplicate positions and the requested horizon.
- Added a runtime-only `BTreeMap<tick, max_sequence>` to `IslandReplayLog`. Each append now reads/updates the per-tick maximum rather than scanning the full history.
- Marked the index as skipped in serde. It does not change JSON, snapshot data, replay hashes or persisted format.
- Rebuild the index once when loading a file, and lazily on the first append after direct serde deserialisation.
- Updated the island CLI and integration tests to use the read-only accessor.
- Added regressions for interleaved ticks, imported out-of-order entries, sequence exhaustion, and appending after serde deserialisation. The JSON regression asserts the runtime index never appears in the persisted log.

Commits:
- `53cd6ade10ef49ba695043612eb7fa09acd40b99` — per-tick sequence index and immutable log entries.
- `8ae4a23b5f19ad25f99dcb2bd954977781ab6570` — indexed append and deserialisation regressions.
- `17f205fcd55664bc5a839c55e3b216f5e4c31862` — update replay tests to the immutable accessor.
- `269186ec2eff37e72a13772f407a8f0ae84093f4` — update island CLI read paths.
- `317824d2cb24905a34b99674864ac486e197f95f` — update intervention integration tests.

Fresh exact-head formatting, Clippy, fast Rust tests, desktop/backend and week/year reference acceptance are required. The previous green workflow for PR #123 predates this change and is not evidence for this head. No local Cargo result is claimed while custom OpenCommander access returns HTTP 429.


## Exact-head formatting correction — 8 November 2026

Workflow [#42413421404](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42413421404) caught a test-placement mistake before Clippy or fast Rust tests ran: the new deserialisation regression had been inserted before the closing brace of the preceding test, nesting it inside that function. Rustfmt reported the required module-level placement.

Commit:
- `9bbe128be14f3465712091a14febe5682bd723c3` — close the preceding test before declaring the deserialisation regression.

This was a test-source formatting/placement defect, not a production-code failure. The preceding workflow did not run Clippy or fast tests for this head. A fresh exact-head workflow is required; no source-level pass is claimed.
