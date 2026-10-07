# Human runtime: memory and cost by population

Measured 2026-10-07, development machine (heavily swapped), release build.

    cargo test --release -p mk_engine --test human_population_memory -- --ignored slow_ --nocapture

Adults spread over a 32 x 64 grid, 4 steps of 60 simulated seconds, flat world, no economy load.

| Humans | us per human-step | Peak RSS of the test process |
|---|---|---|
| 200 | 11 | 11 MB |
| 1,000 | 12 | 35 MB |
| 5,000 | 23 | 153 MB |

Roughly linear (the test fails if per-human cost grows more than 4x from 200 to 5,000).
This is the human runtime alone: no regional world, no cognition-heavy interaction load,
no persistence. The island's population is expected in the tens to low hundreds, so the
margin is large; the integrated measurement belongs to Phase 5.
