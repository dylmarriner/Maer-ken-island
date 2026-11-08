# Biotic and soil-water source validation — 9 November 2026

Repository: `dylmarriner/Maer-ken-island`
Branch: `fix/material-source-amount-boundaries-20261109`
Base: PR #129 squash merge `170e4b5f1c035bf71535294fa758bbd4b661a22b`

## Previous milestone

PR #129, material amount validation and atomic crafting, was merged after exact-head workflow #42473464844 passed all required non-nightly jobs. Formatting, Clippy, fast Rust tests, dependency audit, computer service, installed backend, desktop application, dashboard, repository scripts, and both week/year reference-pack acceptances passed. Nightly slow tests were skipped by design.

Merge: `170e4b5f1c035bf71535294fa758bbd4b661a22b`.

## Finding

The material amount audit still had source paths that silently clamped negative requests and could mutate ecological/hydrological sources before discovering that the destination stock would become non-finite:

- `gather_biotic` clamped negative/non-finite requests through `max(0.0)`.
- `gather_edible` had its own clamp and bypassed `gather_biotic` when the edible layer existed.
- `gather_soil_water` clamped invalid requests and drained soil before adding the water to material stock.

## Change

- Apply the shared finite, non-negative amount validation to biotic, edible and soil-water gathering.
- Remove the silent negative-quantity clamps from those paths.
- Preflight resulting Wood, PlantFood and Water stock before changing biomass, edible availability, soil storage or flux entries.
- Preserve existing zero-request/empty-source behavior and valid positive gathering semantics.
- No snapshot schema, append-only action log, replay hash, RNG, tick cadence, reference data or physical constants changed.

## Regression coverage

- Negative, NaN, positive-infinity and negative-infinity requests are rejected by all three paths.
- Invalid requests preserve biomass, edible availability, soil storage, material stock and flux-ledger entries.
- A biotic harvest that would overflow existing Wood stock is rejected before biomass changes.
- A soil-water transfer that would overflow existing Water stock is rejected before soil is drained.

## Commits

- Production: `ef4368bcf5fdf5ad5d915f70d8068148ae257901`.
- Regression tests: `4e2b3cc80fe9492161e8b026b51e1ecebc43af93`.

## Verification

Exact-head CI is required for formatting, Clippy, fast Rust tests, desktop, installed backend, dashboard, dependency audit, computer service, repository scripts and week/year reference-pack acceptance. The new PR must remain draft until its own exact-head checks finish green.

Custom OpenCommander device discovery returned HTTP 429, so no local Cargo result is claimed.

## Next

- Verify this branch's exact head.
- Continue auditing public material source/sink operations with typed errors and pre-mutation checks.
- Then advance D40 two-person saw cross-cutting with separate founder time/energy accounting.
- Keep deterministic single-writer simulation state, append-only logs, replay-hash equality, optional LLM behavior and offline/budget constraints unchanged.

## Exact-head formatting correction — 9 November 2026

- Initial PR #130 workflow [#42477804232](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42477804232) checked out the initial handoff head and failed only at `cargo fmt --all -- --check`.
- Rustfmt identified two line-wrap differences in the newly appended regression test; no Rust compilation, Clippy or fast-test result was reached in that job.
- Applied the exact formatter output in test commit `480284d57f4b1ff8ef27cae59477eeaeae01850e`.
- At the time of correction, the independent installed-backend, desktop, dependency-audit and week/year reference-pack jobs from #42477804232 were still running; their results are not evidence for the corrected head.
- A fresh exact-head workflow is required for the corrected branch head. No test pass is claimed until that run completes.

## Source-state and cell-area integrity follow-up — 9 November 2026

The source boundary was still incomplete even after request-amount validation: finite/non-negative requests could be applied against NaN, infinite or negative source reservoirs, and non-positive/non-finite cell areas. `f64::min`/`max` can mask NaN operands, so relying on the requested quantity alone was insufficient.

### Production changes

- Added typed `MaterialError::InvalidCellArea` and `MaterialError::InvalidSourceState`.
- Added a shared finite, strictly-positive cell-area check.
- `gather_biotic` now validates biomass density, area-converted biomass and available material before mutating ecology.
- `gather_edible` independently validates the edible layer and biomass before taking from either reservoir.
- `gather_soil_water` validates moisture fraction, soil storage and area-converted available water before draining soil.
- `gather_water` validates surface water, runoff, both area conversions and their sum before changing either hydrology source.
- Preserved the existing ledger reservoirs: surface water remains `SurfaceWater`, river runoff remains `Rivers`.
- No snapshot schema, append-only action log, replay hash, RNG, tick cadence, reference pack, physical constant, optional LLM path or offline/budget rule changed.

### Regression coverage

- Biotic biomass and edible stock reject negative, NaN and positive-infinity source values without changing source, stock or flux entries.
- Soil storage and surface-water source values reject the same invalid states before mutation; non-finite runoff is rejected too.
- All four cell-gathering operations reject zero, negative, NaN and infinite cell areas before mutation.
- The earlier stock-overflow tests remain in place.

### Commits

- Production source validation: `64883418984a2868881e04ba91f2d6cff1db4597`.
- Regression tests: `101ddc44d13f8d8464e5dd9f23fd675aa54b164f`.
- Corrected river reservoir identity after backend compilation exposed an invalid enum variant: `b89e64b74403fe74b47f967428afc259c813adc8`.
- Exact rustfmt corrections: `063c7ebac9cfc8030b9ddd1282e39a35019ae52f`, `1366320e396cf70aea2ee67464447503aebfed88`.

### Verification

- Workflow [#42479894903](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42479894903) on the pre-correction test head exposed a real compile error: `Reservoir::Runoff` does not exist. The code now uses the established `Reservoir::Rivers` identity; this is corrected in `b89e64b7`.
- Workflow [#42479978630](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42479978630) on the corrected source head found only rustfmt differences. The exact formatter output is committed above.
- Fresh exact-head workflow [#42480080363](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42480080363) is pending on test-format head `1366320e`. No current-head Rust test, Clippy, desktop or installed-backend pass is claimed yet.
- Custom OpenCommander continues to return HTTP 429, so no local Cargo run is claimed.

Keep PR #130 draft until exact-head formatting, Clippy, fast Rust tests, desktop, installed backend, dashboard, computer service, repository scripts, dependency audit and week/year reference-pack acceptance pass. Nightly slow tests remain skipped by design.
