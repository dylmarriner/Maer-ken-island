# Populated island: time and memory

Test: `crates/mk_engine/tests/island_population_scale.rs` (slow tier,
`cargo test --release -p mk_engine --test island_population_scale -- --ignored --nocapture`;
`ISLAND_SCALE_HUMANS`, `ISLAND_SCALE_STEPS`). The whole integrated island
(regional physics, ecology, estate, energy, materials, households) with N
settlers added through the command path, stepped in 60 s substeps. Release
build, one machine (Linux), peak resident memory from `VmHWM`.

| Humans | Steps (60 s each) | Bootstrap | Run | Per step | Peak RSS |
|---:|---:|---:|---:|---:|---:|
| 2 (founders; `island run`, 1,500 steps) | 1,500 | n/a | 17.4 s | 11.6 ms | 287 MB |
| 202 | 360 | 12.0 s | 9.0 s | 25 ms | 369 MB |
| 1,002 | 360 | 12.0 s | 41.4 s | 115 ms | 594 MB |
| 5,002 | 120 | 12.1 s | 61.3 s | 511 ms | 1,017 MB |

Reading: memory is about 0.3 MB per human on top of a ~290 MB island, so an
8 GB PC has room for tens of thousands of people by memory alone; time is the
limit. 5,000 people still run at about 117x real time (60 s simulated per
0.51 s), so a person-scale day is minutes, not hours.

Caveats, so the numbers are not over-read:

- Short spans (a quarter of a day to six hours). No births, deaths or
  long-run population growth are exercised; a lifetime-scale population
  run is still to be measured (Phase 5).
- Settlers all arrive at the yard and are not given jobs or a larger
  property; the household economy is sized for the founders. This measures
  the human runtime cost on the island, not a realistic village.
- Per-human folders are off here (`--save-root` adds ~30 ms per full sync
  for two humans; at thousands of humans the hourly sync will cost
  proportionally more and needs its own measurement).
- Snapshot size grows with population; 336 MB at two humans is dominated
  by the physical grids (`benchmarks/island_snapshot.md`).
