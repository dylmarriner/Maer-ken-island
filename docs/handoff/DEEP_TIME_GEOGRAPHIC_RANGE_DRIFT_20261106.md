# Deep-time geographic-range and logistic-growth corrections — 6 November 2026

Repository: `dylmarriner/Maer-ken-island`
Branch: `fix/deep-time-geographic-range-fraction-20261106`
Base: `main` at `373a1b71517b82352b40208e4546bc0d3e4d24a1`

## Finding 1: geographic-range drift

`Species::geographic_range` is a fraction of planetary surface. The founding catalogue uses values around 0.2–0.8, but the imported `apply_drift` previously assigned:

```rust
(sp.geographic_range * (1.0 + range_drift)).max(1.0)
```

Every valid positive range below one was forced to exactly 1.0 on the first evolution step, erasing species range differences and changing later island species-density projections.

## Change 1

- Define the field as a finite planetary-surface fraction in `(0, 1]`.
- Validate every species range and the step duration before advancing the evolution clock or mutating species.
- Apply keyed range drift without forcing fractional ranges to one.
- Cap only at the physical upper bound of one planetary surface.
- Precompute and validate every next range before mutating species in the drift phase.
- Preserve deterministic keyed RNG streams; no shared mutable RNG consumption was introduced.

## Finding 2: logistic population-growth overflow

The imported closed-form logistic update used `exp(+r·dt)`. For sufficiently long but finite deep-time horizons, that exponential overflows to infinity; the numerator and denominator then become `inf/inf`, producing NaN and allowing the population cast to collapse to one. The function's comment claimed stability for any finite horizon, but the implementation did not provide it.

## Change 2

The same analytic logistic solution is now expressed with the decaying exponential:

```text
N(t) = K / (1 + (K / N0 - 1) * exp(-r * t))
```

This keeps the calculation finite as long horizons drive the exponential toward zero, without changing the model's growth rate, carrying-capacity policy, RNG keys or step ordering. The imported-code divergence is recorded as item 8 in `UPSTREAM.md`; the upstream repository itself was not modified.

## Regression coverage

Geographic-range tests:
- A one-year evolution step keeps a 0.4 starting range near 0.4 rather than forcing it to 1.0.
- Positive drift at the upper boundary is capped at 1.0.
- NaN geographic range fails before the clock, step index or population changes.
- NaN, infinity and negative step durations fail before the clock advances.

Logistic-growth tests:
- A one-million-year finite horizon converges to carrying capacity from below.
- The same horizon converges to carrying capacity from above.
- A 100-year horizon retains the expected closed-form result (124,492 individuals for the pinned fixture).
- A zero-duration update leaves population unchanged.

## Commits

- `6422300c56b9b60e9fddd68bab7cb94cac0102d6` — stable logistic growth and regression tests.
- `aeb9a0b2d4eabbe7324e4d3a846bc7a17a9b2efb` — record imported-code divergence in `UPSTREAM.md`.
- The current handoff commit records this combined slice.

## Verification

No local Cargo result is claimed because the authorised custom OpenCommander connector returned HTTP 429. Exact-head GitHub Actions must pass formatting, Clippy, fast tests, desktop/backend builds and week/year reference-pack acceptance before merge.

Earlier workflow [#42141050362](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42141050362) passed formatting, Clippy, dependency audit, installed backend, dashboard, computer service, repository scripts and both week/year reference-pack tests on the earlier geographic-range head; fast tests and desktop build/render were still running at last inspection. It does not verify the new logistic-growth commits.

## Invariants and scope

These are intentional deterministic behaviour corrections in imported deep-time biosphere code. No RNG key, append-only event-log format, snapshot schema, replay ordering, material ledger, fishery calibration, real-world parameter, external service or live simulation was changed. Because geographic range and long-horizon growth affect downstream state, exact-head replay/reference acceptance remains mandatory. Do not silently re-pin a digest; inspect the audited output first.

No deployment, live simulation, external LLM call, production data change or infrastructure mutation was performed.

## Next

1. Verify this exact branch head in GitHub Actions.
2. If week/year or replay digests move, inspect the audited state and reference evidence before changing any pin.
3. Continue from `WORK_REMAINING.md`; the fishery remains standalone until sourced species-specific stock and effort data are available.

## Exact-head CI iteration

- Workflow [#42143127232](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42143127232) checked the updated PR #103 merge head and failed only at `cargo fmt --all -- --check`.
- The formatter identified one multiline `assert_eq!` argument layout and one extra trailing blank line in the new population-growth test module. No Rust compile, Clippy or fast-test result is available from that job because formatting stopped the lane.
- Commit `bdac1c7376819796dc75bbf46a3497379138213b` applies the exact rustfmt output and removes the extra trailing blank line.
- The week/year reference-pack and desktop/backend jobs from #42143127232 were still running at inspection; they predate the formatting fix and do not verify the latest head.
- A fresh exact-head workflow is required for `bdac1c73`. Keep PR #103 draft until formatting, Clippy, fast tests, desktop/backend and week/year reference acceptance all pass.
