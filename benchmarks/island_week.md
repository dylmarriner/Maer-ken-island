# Combined island: one simulated week

Measured 2026-10-07, development machine (~15 GB RAM, heavily swapped), release build.

    cargo test --release -p mk_engine --test island_life_acceptance -- --ignored slow_ --nocapture

The whole Phase 1-3 island at once, from `fixtures/island/default_scenario.json`:

- geophysics, climate, weather, storms, ocean, tides and hydrology (`RegionalPhysicalState`);
- ecology (biomes, NPP, standing biomass);
- the founders' estate laid out in metres, with 200,000 individual trees and stand cover on its patch;
- Gem-D and Gem-K as canonical humans, 60 s steps;
- the estate's energy, and every food, water and respiration flow through the material ledger.

Cadences: humans 60 s, physics hourly, ecology and households 6-hourly.

| Quantity | Result |
|---|---|
| Simulated span | 7 days (10,080 human steps, 168 physics steps, 28 ecology steps) |
| Wall clock, including bootstrap | 49 s |
| Peak resident memory | 307 MB (314,484 kB), re-measured after the biome fix (D32) |
| Stock audits (material carbon, body carbon, water) | 196, all closed |
| Founders | both alive, both in the estate |
| Food and water shortfalls | 0 |
| State digest, two separate processes | identical (`bf836cb8...6e3f`) |

Not in this number: Phases 4, 4b and 5 (persistence, replay, the app, a larger population, the
refinery and town). The population is two humans; `benchmarks/humans_population.md` measures the
human runtime alone at 5,000.
