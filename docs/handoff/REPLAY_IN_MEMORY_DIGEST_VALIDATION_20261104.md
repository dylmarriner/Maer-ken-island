# In-memory replay scenario digest validation — 4 November 2026

Canonical repository: `dylmarriner/Maer-ken-island`
Branch: `fix/replay-inmemory-digest-validation-20261104`
Parent: PR #74 head `f64e6339ffd5de769db1e34858a18ad63363ed04`
Stacked PR base: `test/replay-special-file-refusal-20261104`

## Change

The replay file loader and writer already reject a scenario digest that is not exactly 64 lowercase hexadecimal characters, but the public `replay_island` API could receive an in-memory `IslandReplayLog` directly and classify a malformed digest as `WrongScenario`.

The replay entry point now applies the same canonical digest validation before bootstrapping the island or comparing scenario identity. This keeps malformed input distinct from a valid digest belonging to another scenario and avoids bootstrapping for an invalid log.

## Regression coverage

- Direct replay of an in-memory log with a malformed digest returns `InvalidScenarioDigest`.
- The existing foreign-scenario test now uses a canonical but different 64-character digest and continues to require `WrongScenario`.

## Verification

- Source and test changes are committed on this branch.
- The parent PR #74 CI was left untouched; this branch is stacked on its current head.
- Exact-head CI for this branch must verify formatting, Clippy, fast tests and desktop/backend checks.
- No local Cargo test execution is claimed.

## Remaining

- [ ] Verify exact-head CI for this branch.
- [ ] Complete PR #74 and its required checks first, then retarget/rebase this follow-up and verify again.
- [ ] Run mandatory week/year reference-pack slow tests.
- [ ] Preserve replay digest equality, snapshot formats, deterministic single-writer state and offline/budget constraints.
