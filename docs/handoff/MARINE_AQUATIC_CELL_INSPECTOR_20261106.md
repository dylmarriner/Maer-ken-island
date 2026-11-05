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
- Add a server integration test for a sea cell, a land-cell exclusion assertion, and an old-payload compatibility test for clients reading a backend that predates the new optional fields.
- Update `WORK_REMAINING.md` to distinguish the now-visible read-only projection from the still-unimplemented working fishery.

## Invariants

The projection is rebuilt from canon + scenario seed + fixed terrain/ecology. It is not stored in `IslandLife`, snapshots, event logs or replay state. It does not mutate biomass, create a second population, infer resilience/carrying capacity/CPUE, assign work to humans or alter ledger flows. No reference digest, simulation cadence, random stream or LLM behaviour changes.

## Verification

Source and regression tests are committed. Exact-head CI is required for formatting, Clippy, fast tests, desktop application, installed backend, dashboard, computer service, dependency audit and the week/year reference-pack gates. Custom OpenCommander returned HTTP 429, so no local Cargo result is claimed.

## Next

1. Verify the PR #92 base projection and this stacked API/UI change on exact-head CI.
2. Keep the fields explicitly uncalibrated until sourced species-level resilience, unexploited biomass/carrying capacity and catch-per-unit-effort data exist.
3. Only then connect the fishery to IslandLife, human-chosen fishing effort and ledger-balanced catches.

No live island state, production infrastructure, external service, snapshot format or deterministic replay input changed.
