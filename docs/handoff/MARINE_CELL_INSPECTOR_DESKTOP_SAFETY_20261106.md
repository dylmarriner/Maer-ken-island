# Desktop marine inspector safety parity — 6 November 2026

**Repository:** `dylmarriner/Maer-ken-island`  
**Branch:** `fix/desktop-marine-inspector-safety-20261106`  
**Base:** `main` at `3057ea4274f8d6a131fa44a24380e0c7c145057a`

## Finding

The browser cell inspector already labels the marine values as an uncalibrated upstream estimate and explicitly says they are not harvestable stock or catch. The desktop inspector displayed the uncalibrated estimate but omitted the operational warning. It also rendered negative, NaN or infinite individual/biomass values whenever all three optional fields were present.

## Change

- Desktop marine values now render only when individual count and live biomass are finite and non-negative.
- The desktop panel uses the same wording as the browser: `Uncalibrated upstream estimate; not harvestable stock or catch.`
- Missing or invalid projection fields omit both the projection and its disclaimer rather than presenting a partial or misleading value.
- No marine stock, species parameters, harvest model, simulation state, API schema or snapshot format changed.

## Regression coverage

- Valid marine projection renders its values and the explicit non-harvestable disclaimer.
- NaN, positive infinity and negative individual values are omitted.
- NaN, positive infinity and negative biomass values are omitted.
- Partial projections remain omitted.

## Verification

The exact-head Rust formatting, Clippy, fast tests, desktop build/rendering, installed backend and week/year reference-pack acceptance are required. They have not yet run for this branch. Custom OpenCommander device discovery currently returns HTTP 429, so no local Cargo result is claimed.

## Safety

No live simulation state, physical parameters, production infrastructure, deployment, external service, budget or offline limit was changed.


## Verification update — 6 November 2026

Exact-head workflow [#42194076317](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42194076317) on code head `39d9d13b591e30343f2fd0431f1e39af3ca340f8` passed Rust formatting, Clippy, fast Rust tests, desktop build and unit tests, installed backend, week/year reference acceptance, web dashboard, computer service, repository scripts and dependency audit. Nightly slow tests were skipped by design.

PR #107 was merged into main after this run, so the desktop branch is being reconciled with the new main head. A fresh exact-head run is required before merge. Custom OpenCommander returned HTTP 429; no local Cargo result is claimed.
