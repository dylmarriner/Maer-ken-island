# Material ingestion and timber seasoning atomicity — 9 November 2026

## Scope

PR #135: [Preflight timber and roof-water material ingestion](https://github.com/dylmarriner/Maer-ken-island/pull/135)

This slice closes the no-Result material-ingestion paths identified by the previous stock-inflow audit. It keeps the public method signatures stable and fails fast before mutation when a requested transfer cannot be represented by the audited binary64 stocks.

## Production changes

- `receive_felled_wood`: rejects non-finite/negative carbon, positive carbon that underflows to zero wood mass, and rounded-away Wood or aggregate MaterialCarbon additions before writing stock or flux.
- `receive_felled_log`: validates the cell area and soil-water storage; preflights aggregate MaterialCarbon and MaterialWater plus the resulting soil storage before allocating the log ID, changing hydrology, appending fluxes or inserting the log.
- `capture_roof_water`: validates amount, area and soil storage; preflights both the Water inventory and aggregate MaterialWater before debiting soil.
- `season_stored_logs`: gathers eligible log IDs in deterministic map order, uses compensated batch aggregation, preflights the complete Wood addition and aggregate MaterialWater debit, then removes logs and appends water-to-atmosphere flows. A refusal leaves every log and ledger entry untouched.

## Regression coverage

Added tests in `crates/mk_engine/tests/island_material_flows.rs` for:

1. Felled wood that would round away against `f64::MAX` Wood stock.
2. Roof-water capture that would round away, preserving soil, stock and ledger.
3. Felled-log carbon that would round away in aggregate MaterialCarbon before ID allocation.
4. Felled-log water that would round away in aggregate MaterialWater before ID allocation.
5. Seasoning refusal when the Wood destination cannot represent the batch, preserving the stored log and ledger.
6. Two 1 kg logs aggregated into one representable 2 kg addition to a `2^53` kg Wood stock.
7. A successful roof-water transfer with a closed material-water audit.

## Verification

- PR #135 created as draft against `main` at base `6b57ad96273c2320910f93902963bc8800974175`.
- Head at creation: `aa026fc2a3c3c15315df4098d2ed729ef2cbbf01`.
- Exact-head GitHub Actions verification is pending.
- No local Cargo test or formatting pass is claimed. Custom OpenCommander access is unavailable in this run; no Remote Desktop Commander substitute was used.

## Architecture and safety

No snapshot schema, append-only replay format, replay hash, RNG, tick cadence, scientific constant, reference pack, live simulation state or external infrastructure was changed. All source/destination preflights occur before the first mutation in each method.

## Next

1. Complete PR #135 formatting, Clippy, fast tests, desktop/backend and required week/year acceptance.
2. Keep PR #135 draft until every required check passes; do not merge a red or partially verified branch.
3. Continue the separate PR #134 week-reference digest investigation; do not update the expected digest until the state drift is explained.
4. Resume D40's remaining two-person saw cross-cut for ute transport, then founder labour/energy and route modelling; continue D31 regional soil-nutrient modelling.
