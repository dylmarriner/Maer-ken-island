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

## What predictive processing costs

    cargo test --release -p mk_engine --test human_population_memory -- --ignored slow_formal --nocapture

Measured 2026-10-07, same machine and release build, 1,000 adults over 4 steps of 60 simulated
seconds.

| Quantity | Result |
|---|---|
| Whole human step | 20 us per human-step |
| `formal_predictive_processing::step` alone | 1.6 us per human-step |
| Its share of the step | **8%** |

`docs/canon/HUMAN_SYSTEM_STATUS.md` records that nothing outside that module reads the action it
selects: `AutonomousMind` is what drives behaviour. So 8% of every human step, for every human,
every tick, buys a hierarchical-inference and actor-critic update whose output is consumed only by
its own next update. At the island's expected tens to low hundreds of people that is affordable;
it is recorded here so that wiring the output into decisions, or stopping the work, is a decision
made on a number. The measurement is not a budget: it will move with the model.
