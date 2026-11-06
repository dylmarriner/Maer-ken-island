# Marine biomass cell inspector integration

Date: 2026-11-06
Repository: `dylmarriner/Maer-ken-island`
Branch: `feat/marine-aquatic-cell-inspector-20261106`
Base: PR #92 head `cb88658f8bebed9b380b3ded8640a8296f58e882`.

## Change

- Retain a static `IslandBiota` projection in the server's immutable `Terrain` view, rebuilt deterministically from the same canon and scenario seed.
- Extend the shared `mk_island_api::Cell` wire type with optional marine species count, upstream-derived individual count and live biomass in tonnes.
- Populate those fields from `marine_aquatic_nodes` only for cells with marine aquatic species. Land/river cells omit the fields rather than reporting fictional zero stock.
- Display the values in the desktop cell inspector with an explicit "uncalibrated upstream estimate" label.
- Add server integration tests for a sea cell, land-cell exclusion, and an unchanged canonical digest/tick across a paused read-only cell query.
- Add an old-payload compatibility test for clients reading a backend that predates the new optional fields.
- Update `WORK_REMAINING.md` to distinguish the now-visible read-only projection from the still-unimplemented working fishery.

## Invariants

The projection is rebuilt from canon + scenario seed + fixed terrain/ecology. It is not stored in `IslandLife`, snapshots, event logs or replay state. It does not mutate biomass, create a second population, infer resilience/carrying capacity/CPUE, assign work to humans or alter ledger flows. No reference digest, simulation cadence, random stream or LLM behaviour changes.

## Verification

Source and regression tests are committed. Workflow [#42045662490](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42045662490) ran on code head `97e3970d`; at the last inspection formatting and Clippy had passed, while fast tests, desktop build/render, installed backend and week acceptance were still running. A subsequent docs-only roadmap-baseline commit `e79baf7` and the digest-readonly regression commit `d994da7` are newer than that run, so the run is not exact-head evidence for the latest branch. A fresh run is required. Custom OpenCommander returned HTTP 429, so no local Cargo result is claimed.

## Next

1. Verify the PR #92 base projection and this stacked API/UI change on exact-head CI.
2. Keep the fields explicitly uncalibrated until sourced species-level resilience, unexploited biomass/carrying capacity and catch-per-unit-effort data exist.
3. Only then connect the fishery to IslandLife, human-chosen fishing effort and ledger-balanced catches.

No live island state, production infrastructure, external service, snapshot format or deterministic replay input changed.


## Follow-up: browser map inspector (2026-11-06)

The API fields and desktop inspector were present, but `apps/island/static/island.js:onMapClick` still rendered only coordinates and land/sea details. The browser click handler now renders marine species count, upstream-derived individuals and live biomass tonnes when all three API values are finite, with an explicit label that this is an uncalibrated upstream estimate and not harvestable stock or catch. `style.css` gives the estimate a separate, tabular-numeric line. `apps/island/tests/serve.rs` verifies the served browser handler contains the three fields and the safety label.

## Exact-head formatting finding and correction

Workflow [#42109939191](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42109939191) passed repository scripts, desktop application, dashboard, week/year reference acceptance, dependency audit, installed backend and computer service. The Rust fast-check job stopped at `cargo fmt --all -- --check` because there was one extra blank line before the browser cell-inspector test in `apps/island/tests/serve.rs`; Clippy and fast tests were skipped.

Commit `84dc5f5a` applies the exact rustfmt output. A fresh exact-head run must pass formatting, Clippy and fast tests before merge. The prior workflow's other green jobs are evidence for the prior head only.


## Review follow-up — browser DOM regression

The browser click path now delegates to the exported `handleMapCellClick` helper in `apps/island/static/app.js`. The web dashboard CI runs `scripts/tests/marine_cell_inspector.test.mjs`, which simulates a map click and asserts the actual inspector nodes contain the upstream species count, individuals, live biomass and the uncalibrated/not-harvestable disclaimer. It also verifies invalid marine projection fields are omitted without hiding ordinary cell details, and clicks before the view is ready make no request. This addresses the source-only assertion gap and the review request for visible DOM coverage. Exact-head CI is required; no local or CI pass is claimed until that workflow completes.


## Exact-head Rust integration failure and correction (2026-11-06)

Workflow [#42159513778](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42159513778) passed formatting and Clippy, but the fast Rust suite failed one assertion in `browser_map_cell_inspector_renders_the_uncalibrated_marine_estimate`. The test still searched for marine fields inside `island.js:onMapClick` after the click path had been extracted into the shared `app.js:handleMapCellClick` helper. The browser DOM test itself passed; the failing assertion was stale source-location coupling, not a missing UI field.

Correction on this branch:
- Commit `4bdb413b3349c472f66984a5513f16555235c825` updates the Rust integration test to assert that `island.js` wires `handleMapCellClick`, then inspect the served shared `app.js` helper for the three marine fields and safety disclaimer.
- Commit `04bf72a02becb24d16a625f4e46ea04cdc724675` repairs the browser-inspector entry in `WORK_REMAINING.md`; the previous insertion had split a sentence in the marine biomass bullet.
- A fresh exact-head CI run is required for the corrected test and documentation. The prior workflow is not evidence for this new head.

No simulation state, RNG, replay digest, snapshot, append-only log, physical parameter, external service or production infrastructure changed.
