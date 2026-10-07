# Phase 2 physical simulation: memory and time

Measured 2026-10-07 on the development machine (~15 GB RAM, heavily swapped, release build),
owner's provisional island (`fixtures/island/default_profile.json`).

Command:

    /usr/bin/time -v island_preview physical --profile fixtures/island/default_profile.json --days 360 --out <dir>

| Quantity | Result |
|---|---|
| Simulated span | 360 local days (one canon year) after bootstrap |
| Bootstrap | island geophysics + two-orbit spin-up (included below) |
| Wall clock, total | 22.0 s |
| Peak resident memory | 218,516 kB (213 MB) |
| Exit status | 0 |

Coarse (12 km) grids carry climate, weather, ocean and tides; the medium (2 km) grid carries
hydrology over the land cells only. No planetary grid is allocated. Full-workspace and
ecology/human memory are not yet measured (Phase 5).
