# Deep-time geographic-range drift correction — 6 November 2026

Repository: `dylmarriner/Maer-ken-island`
Branch: `fix/deep-time-geographic-range-fraction-20261106`
Base: `main` at `373a1b71517b82352b40208e4546bc0d3e4d24a1`

## Finding

`Species::geographic_range` is a fraction of planetary surface. The founding catalogue uses values around 0.2–0.8, and the deep-time model's own documentation describes that scale. `apply_drift` previously assigned:

```rust
(sp.geographic_range * (1.0 + range_drift)).max(1.0)
```

Every valid positive range below one was therefore forced to exactly 1.0 on the first evolution step. That erased the species' geographic-range differences and changed the density derived later by the island species projection.

## Change

- Define the field explicitly as a finite planetary-surface fraction in `(0, 1]`.
- Validate every species range and the step duration before advancing the evolution clock or mutating species.
- Apply the keyed range-drift draw without forcing fractional ranges to one.
- Cap only at the physical upper bound of one planetary surface.
- Calculate and validate every next range before mutating any species in the drift phase.
- Preserve deterministic keyed RNG streams; no shared mutable RNG consumption was introduced.

## Regression coverage

- A one-year evolution step keeps a 0.4 starting range near 0.4 rather than forcing it to 1.0.
- Positive drift at the upper boundary is capped at 1.0.
- NaN geographic range fails before the clock, step index or population changes.
- NaN step duration fails before the clock advances.

## Verification

The implementation and tests are committed. Custom OpenCommander currently returns MCP HTTP 429, so no local Cargo test result is claimed. Exact-head GitHub Actions must pass formatting, Clippy, fast tests, desktop/backend builds and the reference-pack/replay acceptance jobs before merge.

## Invariants and scope

This is an intentional deterministic behaviour correction in deep-time biosphere evolution. It does not change RNG keys, event-log formats, snapshot schemas, append-only log semantics, replay ordering, material ledgers, fishery calibration, or external services. The exact-head CI and replay/digest acceptance remain mandatory because corrected geographic ranges can change downstream species-density outputs.

No deployment, live simulation, external LLM call, production data change or infrastructure mutation was performed.

## Next

Verify this exact branch head in CI. If replay/reference acceptance identifies a changed pinned output, inspect the diff and update a pin only after the corresponding audit passes; do not silently re-pin.

## CI iteration

- First PR workflow [#42139926212](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42139926212) checked out the branch and failed only at `cargo fmt --all -- --check`. The log identified two formatting differences in the new test module: one extra blank line and one assertion that rustfmt expands across lines.
- Both formatting differences were corrected in commit `4b4ac2e3ea020a827a5033e64d6e2a2cd8220143`.
- New exact-head workflow [#42140174141](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42140174141) is pending. No Clippy, fast-test or replay acceptance pass is claimed until that run completes.

## Rebase after PR #102 merge

PR #102 was merged into main as `2f11b5e9a3cd84a35f6cb69e1966cd3cd72391ed`. This branch is being rebased onto that commit so the deep-time correction does not carry a conflicting copy of the marine handoff/roadmap changes. The prior formatting correction remains in the source. The new exact-head CI run after the rebase is the merge gate; the older workflow #42140215387 was started against the pre-rebase branch head and is not sufficient evidence for the rebased commit.
