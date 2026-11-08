# Operator material injection validation — 9 November 2026

Repository: `dylmarriner/Maer-ken-island`
Branch: `fix/operator-material-injection-validation-20261109`
Base: `main` at `170e4b5f1c035bf71535294fa758bbd4b661a22b`

## Finding

`MaterialLedger::inject` was the remaining public material source-inflow boundary that silently clamped negative and non-finite quantities, then added directly to stock without checking whether the resulting inventory overflowed. A rejected or malformed operator intervention could therefore be represented as a successful zero/invalid inflow, or corrupt material stock and its conservation ledger.

## Change

- Change `inject` to return `Result<f64, MaterialError>`.
- Reject negative, NaN and infinite amounts with the existing typed `InvalidAmount` error.
- Reject invalid existing stock and non-finite resulting stock before mutation.
- Preflight the carbon conversion before stock or ledger mutation.
- Preserve the explicit `OperatorIntervention` boundary entries for material carbon and water.
- Propagate injection errors through the island's `InjectResource(Water)` intervention instead of applying `max(0.0)`.
- Update the existing overflow fixture to assert the valid seed injection succeeds.

## Regression coverage

- Negative, NaN, positive-infinity and negative-infinity inflows are rejected without changing stock or flux entries.
- A second finite maximum-sized water injection is rejected before stock or ledger mutation.
- Valid wood and water injections book carbon and water from `OperatorIntervention`, and the audit closes.

## Commits

- Production material boundary: `d101e85bbf7324eb0a335513afc393c0d14e9341`.
- Intervention refusal propagation: `7794e81aa40050254bfca4bad0a88867e01a3d1a`.
- Regression tests: `1a377f5f2ddcd903834feef75351dfa576d8245c`.
- Roadmap handoff: `1be4e45ee2bfbf791b855594b635afbeff668b4a`.

## Verification

- Exact-head GitHub Actions is required for Rust formatting, Clippy, fast tests, desktop/backend, and week/year reference-pack acceptance.
- Custom OpenCommander device discovery returned HTTP 429; no local Cargo test result is claimed.
- No simulation tick, snapshot schema, append-only log, replay hash, RNG, physical constant or reference pack changed.

## Next

- Verify exact-head CI and merge only after every required non-nightly job is green.
- Continue the D40 two-person saw-based log cross-cutting workflow and the D31 sourced regional soil-nutrient model.
