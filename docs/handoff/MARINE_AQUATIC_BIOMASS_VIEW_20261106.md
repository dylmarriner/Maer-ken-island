# Marine aquatic biomass projection

Date: 2026-11-06
Repository: `dylmarriner/Maer-ken-island`
Branch: `feat/marine-aquatic-biomass-view-20261106`
Base: `main` at `d6f6ce7a1c5f01bf5a14d158fad32dbbcdaffb0a` (after fishery PR #91 merged).

## Change

- Add `MarineAquaticNode`, a derived per-cell view for upstream `Aquatic` species in `OpenOcean` and `ReefSea` habitats.
- Expose the same per-cell individual count already represented by `IslandBiota::animal_nodes`, plus biomass in tonnes (`individuals × body_mass_kg / 1,000`).
- Keep the projection read-only and deterministic. It adds no retained state, snapshot field, event, ledger entry, random draw or replay input.
- Exclude river-affinity aquatic species from the inshore marine projection; they remain available through the general animal-node API.
- Add an integration regression proving per-cell counts match `animal_nodes`, per-cell biomass follows the unit conversion, and sums by species equal the whole-island population and biomass.

## Deliberate boundary

This is a population-to-biomass bridge, not a working fishery. It does not infer a species' resilience class, unexploited carrying capacity, catch-per-unit-effort, vessel fuel burn, crew effort or harvest. The upstream species remain at their documented D55 densities. Those inputs need sourced species-level data before the Schaefer `FishStock` model can be connected without inventing ecology.

## Verification

Required exact-head checks: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, focused `cargo test -p mk_engine --test island_species`, fast workspace tests, and week/year reference-pack acceptance. No local Cargo execution is claimed in this run; hosted exact-head CI must provide the evidence.

## Next

1. Run the focused species test and full required CI.
2. Source a marine species-to-resilience mapping and an evidence-backed unexploited biomass/catch-per-unit-effort model.
3. Only then connect `FishStock` to IslandLife and the human autonomy/fishing-effort path, with ledger-balanced catch and deterministic replay coverage.

No simulation state, snapshot format, replay hash, event log, reference constant, production infrastructure or live island state changed.