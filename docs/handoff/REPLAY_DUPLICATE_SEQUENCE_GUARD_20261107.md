# Replay duplicate sequence guard

Updated 2026-11-07 on the replay-sequencing PR branch.

## Defect

A replay log is publicly constructible and its entries may be reordered. If two entries share the same `(tick, sequence)` pair, sorting cannot recover their original application order. The stable sort would preserve whichever vector order happened to arrive, so two reordered logs could apply conflicting commands in different orders.

## Production change

- `IslandReplayError::DuplicateSequence { tick, sequence }` identifies an ambiguous ordering position.
- `replay_island` filters to the requested `until_tick` horizon, sorts by `(tick, sequence)`, and rejects adjacent duplicate keys before island bootstrap.
- Duplicate entries beyond a partial replay's horizon remain ignored, preserving the existing 'leave future entries alone' contract.
- No replay format version, serialized entry, snapshot, simulation state, RNG, digest or single-writer behavior changed.

## Regression coverage

- Conflicting commands with the same tick and sequence are rejected even when the scenario digest is deliberately wrong, proving duplicate validation takes precedence over scenario interpretation.
- Duplicate keys beyond the requested partial replay horizon do not block an earlier replay.

## Verification

Code commit: `bb4e1622e731c79fd08ba2bcaac16bfeccacc4d0`.
Test commit: `06d913fc02fb898209fe6e6f01970a4b385b671a`.

Fresh exact-head CI is required on the latest PR head. No local Cargo pass is claimed.

## Exact-head formatting correction

Workflow [#42362560057](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42362560057) on the prior documentation head `64c3b66a` reached the Rust formatter and found only the expected multiline layout for the new `DuplicateSequence` struct variant. Commit `5aea0f30940cbe2a4663d984f02d75d1650fd1c2` applies that exact rustfmt output. Fresh exact-head workflow [#42362708042](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42362708042) is running; no source-test result is claimed until all required jobs finish.
