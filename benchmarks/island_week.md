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
| Peak resident memory | 308 MB (315,436 kB), re-measured with scheduler, snapshot and commands |
| Stock audits (material carbon, body carbon, water) | 196, all closed |
| Founders | both alive, both in the estate |
| Food and water shortfalls | 0 |
| State digest, two separate processes | identical (`f97c4ca2...1f52`), re-pinned after the command state joined the digest |

Not in this number: Phases 4, 4b and 5 (persistence, replay, the app, a larger population, the
refinery and town). The population is two humans; `benchmarks/humans_population.md` measures the
human runtime alone at 5,000.

Human store (Task 3b), one simulated day, two humans: 18.2 s with folders vs 17.4 s without (25 syncs, about 30 ms each); peak 293 MB vs 287 MB; state hash identical.

## One simulated year, two founders (`island run --steps 525600 --dt 60`)

| Quantity | Result |
|---|---|
| Simulated span | 365 days (525,600 human steps) |
| Wall clock | 32 min 47 s (about 16x real time while sharing the machine with builds) |
| Peak resident memory | 713 MB (includes the final snapshot save, 357 MB file) |
| Exit | success; snapshot saved and run log written |
| Population | 2 founders at the start, **3 at the end**: a birth happened and the child is in the world |

Run with the release binary built before the digest-streaming change, so its
state hash (`dd7ee529...1f1d`) is not comparable with the pinned week digest.
One run only: not re-run in a second process, and what the household economy
did over the year (food and water shortfalls, the child's health) was not
inspected beyond the population count. Memory over the year was not sampled
over time, so growth cannot be separated from the snapshot save peak.

### Re-run on the current code (newborn fix, streamed snapshot)

| Quantity | Result |
|---|---|
| Wall clock | 32 min 11 s (about 16x real time while sharing the machine) |
| Peak resident memory | 397 MB (includes the final snapshot save) |
| People at day 365 | 3: both founders alive and a child, **age 0.266 y, Infant, 5.1 kg, 56 cm** (the earlier run showed the same child as 25 y, 59 kg) |
| Food shortfalls | 0 |
| Water shortfalls | 10 (before the neighbour-cell fix below) |
| State hash | `27772f9e...361d` (binary built before the water fix; not comparable with the pinned week digest) |

Water shortfalls explained (`tests/island_water_supply.rs`, slow tier, physics only,
about 6 min): the household's stream cell holds a thin film of water (a few
millimetres over a 4 km2 cell on a typical day, 10^7 kg) but reads exactly zero
for one continuous day (days 202.75-203.75, five 6-hourly draws). People now try the
eight neighbouring cells when the usual one is dry; with that, the household had
water at every one of 1,460 draws in the year (lowest supply 372 tonnes).

## Five simulated years, two founders (`island run --steps 2628000`)

Run on a copy of the release binary at commit `303bf0c`, memory sampled every minute from outside
(`target/longrun/y5.rss`).

| Quantity | Result |
|---|---|
| Simulated span | 1,825 days (2,628,000 human steps) |
| Wall clock | about 164 min (about 11 simulated days per real minute, on a shared machine) |
| Peak resident memory | 321 MB (252 MB for the first simulated year, 310 MB after the first step up at about 1.1 years, 313 MB for the next three, 321 MB at the end, which includes the snapshot save): no growth trend |
| Snapshot | 53 MB (compressed) |
| People at the end | 5, all alive: Gem-D and Gem-K at 30.0 years; three children of Gem-K at 4.3, 3.2 and 2.0 years (18.0, 10.5 and 11.8 kg; 105, 92 and 77 cm), the oldest a Child, the youngest an Infant |
| Food and water shortfalls | 0 and 0 |
| State hash | `f2484f1d...b8ad` (one run, not repeated in a second process) |

What this does and does not show. It does show that a household of founders runs for five
simulated years inside a flat 250-320 MB, gains three children born on the WHO growth curves, and
never runs short of food or water. It does **not** show that the household is realistic:

- **Where people stand does not matter for food or water.** The household eats and drinks at the
  estate's fixed cells wherever its members are. At the end Gem-D was 2 cells (4 km) from the
  estate, but Gem-K and all three children were 22 cells (44 km) north of it, still "Nourished"
  with no shortfall. People wander (upstream autonomy) and nothing yet ties eating to place.
  This is the unfinished economy wiring (Phase 3 Tasks 3, 3b and 9).
- The children are slightly small for age (3.2 years at 10.5 kg is well under the WHO median of
  about 14 kg); the model grows each child toward a sampled adult body, so a small-framed child
  stays small. Not investigated further.
- No one died in five years (two adults of 25-30 and three children) and the founders did not
  have a child with each other's partner beyond Gem-K's three; a decades-long run with deaths and
  a second generation has not been done.
