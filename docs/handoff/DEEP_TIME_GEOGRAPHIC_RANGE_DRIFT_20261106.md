# Deep-time geographic-range and logistic-growth corrections — 6 November 2026

Repository: `dylmarriner/Maer-ken-island`
Branch: `fix/deep-time-geographic-range-fraction-20261106-rebased`
Base: `main` at `5a0410f87a9a1b947c1bf481dc1f83202573b17b` (after the verified fishery diagnostic merge, PR #105)

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

## Exact-head Clippy iteration

- The fresh workflow [#42143299829](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42143299829) passed formatting, then Clippy failed on three unnecessary `mut` bindings in the new population-growth tests (seeds 11, 12 and 13). Fast tests were skipped after Clippy failed.
- This was a source lint failure, not an environment failure. The three test-only bindings now use immutable `RngRegistry` values in commit `1a910cd167fc17cbae34657de9c45e9dcd3e4c94`.
- The same run's repository scripts, computer service, dependency audit and web dashboard passed. Week/year reference acceptance, installed backend and desktop jobs were still running at inspection; they predate the Clippy correction and do not verify the latest head.
- A fresh exact-head workflow is required for `1a910cd1`. Keep PR #103 draft until formatting, Clippy, fast tests, desktop/backend and week/year acceptance pass.

## Latest exact-head workflow

- Code-bearing head: `1560e6cbbc194cc69393d5b8e323482a442e38a6`.
- Workflow [#42143529000](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42143529000) has passed `cargo fmt --all -- --check` and Clippy after the earlier formatter and unused-mut corrections.
- Fast Rust tests, week/year reference-pack acceptance, desktop application build/tests/rendering and installed-backend build/serve were still running at the latest inspection.
- Repository scripts, computer service, web dashboard and dependency audit have passed. The nightly slow tier is skipped by design on this PR workflow.
- The later `2ed66705` commit updates only the canonical `WORK_REMAINING.md` biosphere row; no application code or tests changed after the code-bearing head.
- No local Cargo result is claimed because the authorised custom OpenCommander connector still returns HTTP 429. Keep PR #103 draft until the remaining exact-head jobs complete successfully.

## Exact-head fast-test finding and correction

Workflow [#42144167785](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42144167785) passed formatting and Clippy but failed the fast Rust tests: 800 passed and 17 failed. The failures shared one root cause: the imported founding-species generator computed `base_range + spread` without capping at one planetary surface. Species 43 (Pelagic) was generated with range `1.0175`, so the new fail-fast deep-time validation correctly rejected it before evolution.

Correction:
- Cap the generated founding range at `1.0` in `BiosphereSystem::generate_founding_species`.
- Add a regression over all 4,096 founding species, including an explicit assertion that species 43 is exactly `1.0`.
- Record the imported-code correction as `UPSTREAM.md` item 9.

This is not a reason to weaken the new validation. A geographic range cannot exceed the entire planetary surface; the invalid value must be corrected at its source. The new exact-head workflow is required to verify all previously failing callers and the week/year reference packs. Do not re-pin any digest without reviewing the measured output.


## Deterministic world-hash baseline correction

Workflow [#42147228604](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42147228604) passed formatting, Clippy, the week/year reference-pack acceptance tests, desktop/backend builds, and the other non-nightly jobs. The fast Rust suite had one remaining failure: `ten_planetary_steps_hash_after_island_reconciliation`. It reported `66d0b56023d72ff7079eb74499820dbfd31a0f44ecbddbd69e493fa320e0bdfe` instead of the prior `bd75bc7d…966285` pin.

The divergence is explained by the intentional founding-species correction in this PR: species 43 previously began with geographic range 1.0175, greater than the entire planetary surface. Capping the generated fraction at 1.0 changes the default biosphere state and therefore the ten-step deterministic world hash. The validation was not weakened. The baseline test now records the measured hash and this reason.

- The deterministic hash test and this handoff were carried onto current main during the PR #106 rebase. The code-bearing rebase head was `b5e6893d1c83e62d61db52c5387d6eac5e3f3d3a`; the later commit in this handoff is documentation-only.
- Fresh exact-head workflow [#42154339053](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42154339053) is running. Dashboard, computer service and repository scripts have passed; formatting/Clippy/fast tests, desktop, installed backend, week/year acceptance and dependency audit are still running at latest inspection.
- No pass is inferred from workflow #42147228604, which predates the hash correction and rebase. Keep PR #106 draft until #42154339053 completes green.


## Follow-up: fail closed on deep-time clock and RNG-step overflow (6 November 2026)

The same fail-fast boundary now checks the existing simulation clock and its next value before mutating the evolution clock, step index or species. It also uses checked addition for the RNG step index so a saturated `u64` cannot wrap and reuse deterministic RNG keys.

- Reject a non-finite or negative persisted evolution clock.
- Reject a finite duration whose addition would overflow the clock.
- Reject `step_index == u64::MAX` before incrementing.
- Add regressions proving both overflow paths leave the clock, step index, population and geographic range unchanged.
- Extend `DeepTimeEvolution::validate` to reject invalid persisted clock state.
- No RNG stream, key derivation, step ordering, snapshot format or simulation parameter changes.

Exact-head CI must rerun formatting, Clippy, fast tests, desktop/backend and both week/year reference-pack acceptance jobs. The previous run is superseded by this follow-up commit; no local Cargo result is claimed while custom OpenCommander returns HTTP 429.


## Exact commits for the clock guard follow-up

- `56c6777cd3f7b1884cb431ad139e8b1be6365e4c` — preflight clock addition and checked RNG step index; add overflow regressions.
- `c0204d3a61416aaf8a50d7137645d773dcac313c` — update the canonical remaining-work checklist.
- `365a76c96dfdcd394731ff0f24cc5f3ea4265f6c` — record the imported-code divergence in `UPSTREAM.md` and update PR #106.
- The persisted-clock validation regression is added in this follow-up commit; exact-head CI remains mandatory.


## Snapshot compatibility review — explicit fail-fast policy (6 November 2026)

Codex review identified that an older `WorldState` snapshot can contain geographic ranges written before the new invariant. Inspection confirmed both `WorldState::from_snapshot` and the file-based `io::snapshot::load_snapshot` restore the persisted `BiosphereSystem::species` catalogue.

I did **not** clamp those saved ranges. The previous drift implementation applied `max(1.0)`, so after evolution it could force every species to at least a whole-planet range and could leave many species above one. The original fractional values cannot be reconstructed losslessly from the saved state alone. Silently replacing them with 1.0 would rewrite scientific state and make the replay origin differ from the saved state. The implementation-ready roadmap's persistence rule requires incompatible schema/state to be rejected rather than silently normalised.

Implemented:
- Add persisted `snapshot_schema_version` (current version 1) to `WorldState`; old snapshots without the field deserialize as schema 0.
- Both snapshot loaders verify the original digest first, then call the same schema/state validator.
- A schema-0 snapshot whose physical state already satisfies current deep-time invariants upgrades only the metadata to version 1; the species, RNG, clock, hash chain and event history are unchanged.
- A schema-0 snapshot with an invalid geographic range is rejected at load with an explicit compatibility error, before replay or a later evolution step can fail.
- Unknown future schema versions are rejected.
- New snapshot creation and file persistence refuse non-current schema state.
- `DeepTimeEvolution::validate` now checks geographic ranges, and `WorldState::validate` includes deep-time invariants.

Regression coverage:
- Valid legacy snapshot upgrades metadata and produces the same next-step hash as the original state.
- Invalid legacy species 43 range is rejected by `WorldState::from_snapshot`.
- Future schema version is rejected.
- The file-based loader upgrades valid legacy state and rejects the same invalid legacy range.
- Existing digest/tamper checks remain before schema validation.

This is an explicit compatibility boundary, not a data migration. No saved files or append-only logs were rewritten, no RNG keys/order changed, and `snapshot_schema_version` is persistence metadata excluded from the subsystem replay hash chain. Exact-head CI must verify the new loader tests before merge.


Follow-up implementation commits: `e857b030` (schema metadata/replay loader), `ac8a72aa` (geographic validation), `a3d91eaa` (file loader), `d6f01314` (replay snapshot tests), `73a2cf00` (file snapshot tests), `964275b8` (world validation), `50e0a442`/`c9431c99` (canon validation order), and `2001fc8b` (finite/non-negative speciation-time preflight before sorting). Rust formatting and checklist/handoff follow-ups are recorded in the PR description.

Latest code/test head: `2001fc8b819cc2dff68e12b7b12a0bda5a341141`. Exact-head workflow [#42166054878](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42166054878) is pending. The prior all-green workflow #42156312276 predates these compatibility changes and does not verify them. Custom OpenCommander remains HTTP 429; no local Cargo result is claimed. Keep PR #106 unmerged until the new exact-head formatting, Clippy, fast Rust tests, both snapshot restore paths, desktop/backend and week/year reference acceptance all pass.
