# Fishery interval sustainable-catch limit — 6 November 2026

Repository: `dylmarriner/Maer-ken-island`
Branch: `feat/fishery-sustainable-catch-limit-20261106-main`
Base: `main` after PR #104 merge `14611890766631c83a51713d0a2aca77d1e4c5c3`
PR: draft #105

## Finding

The fishery model already capped each requested catch at the biomass growth calculated for that interval, but the cap was only available as a side effect of mutating the stock through `FishStock::fish`. A future harvest-decision layer could not inspect the exact bound without either duplicating the numerical calculation or advancing simulation state.

Duplicating it would risk the decision layer displaying a different cap from the one the fishery actually enforces.

## Change

- Add read-only `FishStock::sustainable_catch_limit_t(model, dt_s)`.
- Extract the existing overflow-aware interval-growth and carrying-capacity cap into one shared helper.
- Make `fish()` use that same helper, so the displayed cap and enforced cap cannot drift.
- Preserve the existing Euler interval estimate, growth rate, depensation curve, catch semantics and carrying-capacity bound.
- Reject invalid model, stock and timestep inputs with the existing typed errors.
- Keep the method strictly read-only: it does not change biomass, landed totals, clocks, RNG, event logs or snapshots.
- Document that this is an interval diagnostic, not a TAC, quota, stock assessment or future-recruitment forecast.

## Regression coverage

- At 50% biomass with the default Medium resilience model, a one-year diagnostic returns 125 t and leaves the stock unchanged.
- A subsequent request above that limit lands exactly the reported cap.
- A coarse interval is capped at remaining carrying-capacity headroom.
- NaN, infinite and negative timesteps return `InvalidTimeStep`.
- Invalid model and stock state return their existing typed errors.
- No stock state is mutated by diagnostic calls.

## Commits

- `a7b47da5` — read-only interval catch limit and shared growth-cap helper.
- Documentation and handoff commits follow.

## Verification

No local Cargo result is claimed because the authorised custom OpenCommander connector returned HTTP 429. Exact-head GitHub Actions must pass formatting, Clippy, fast Rust tests, desktop/backend builds and week/year reference-pack acceptance before merge. The slow nightly tier is intentionally separate.

The new API does not add stock calibration or change simulation behaviour unless a caller chooses to use the diagnostic. No reference digest re-pin is expected from the read-only query.

## Invariants

- No new carrying capacity, resilience class, CPUE, catch, effort or workforce values.
- No RNG keys or deterministic ordering changes.
- No snapshot schema, append-only log or replay hash changes.
- No material ledger or IslandLife tick changes.
- No external LLM call, deployment or live simulation mutation.

The fishery remains standalone until sourced species-specific stock and effort data exist and an owner-approved integration design connects harvest to IslandLife material flows.

## Next

1. Verify PR #105 exact-head Rust and acceptance checks.
2. Keep the fishery standalone until species-specific carrying capacity, resilience and effort data are sourced.
3. Design the IslandLife integration around deterministic single-writer state, append-only commands/logs, snapshot compatibility and replay-hash equality.

## Exact-head formatting correction

Workflow [#42146493685](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42146493685) checked the first PR head. Repository scripts, web dashboard, dependency audit and computer service passed; the week/year reference acceptance, installed backend and desktop build were still running. The fast Rust job stopped at `cargo fmt --all -- --check` on exactly two formatting differences in `fishery.rs` (the helper signature and one multiline assertion). Clippy and fast tests were skipped after formatting failed.

Commit `535ec4d5` applies the formatter's exact output. A fresh workflow on that exact head must pass formatting, Clippy, fast tests, both reference-pack acceptance tests, desktop/backend and the other required jobs before merge. Results from the pre-format head do not verify the new head.
