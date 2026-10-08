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

## Snapshots and the human store at scale

Measured with `ISLAND_SCALE_STORE=1`, 60 steps, files on disk (the test uses
`CARGO_TARGET_TMPDIR`, not the RAM-backed `/tmp`):

| Humans | Snapshot file | Save | Load | Peak RSS during snapshot | Full folder sync |
|---:|---:|---:|---:|---:|---:|
| 1,002 | 452 MB | 13.6 s | 3.5 s | 718 MB | 17.6 s |
| 5,002 | 923 MB | 30.0 s | 10.2 s | 1,667 MB | 48.9 s |

**A real defect found and fixed here.** The first 5,002-human snapshot was
killed by `earlyoom` at 3.6 GB RSS on a 16 GB machine: `state_digest` turned
the whole population into one `serde_json::Value` and a string, and the save
built the whole JSON payload as one buffer. Now the digest hashes one human at
a time (`HumanSystem::canonical_chunks`), the save streams JSON to disk
through the hasher and patches the digest into the header, and the load
verifies the digest in 1 MB chunks and parses from a buffered reader. At 1,002
humans the snapshot peak fell from 1,943 MB to 718 MB. The state digest value
changed with the method, so the week digest was re-pinned.

Honest limits:

- The machine was under heavy memory and swap pressure from other
  applications during these runs, so wall-clock figures (and the 1.8 s/step in
  the 5,002 run, against 0.5-0.7 s in quieter runs) are pessimistic and noisy;
  peak memory is the reliable column.
- A **full folder sync of 5,000 humans takes about 49 s**, against about 31 s
  of wall clock per simulated hour at 117x, so the hourly cadence writes a
  rotating shard of at most 1,000 humans (`HumanStoreStatus::max_per_sync`,
  about 17 s per firing here); each human is rewritten at least every
  `ceil(n / 1000)` hours. Deaths are still written at once and a snapshot save
  writes everyone. The per-firing cost at 5,000 (one shard of 1,000) has not
  been re-measured separately.
- Short spans, no births, deaths or long-run growth (unchanged).

## Snapshot compression (format 2) and the regional baseline

Snapshots are now zlib-compressed JSON (flate2, pure-Rust backend, level 1): the two-founder
snapshot went from 338 MB to about 49 MB (7x; a one-year snapshot from 357 MB to about 50 MB), and
format 1 files still load. An unbatched first version was *slower* (32 s to save): the JSON
writer fed the compressor a few bytes at a time; batching the writes through a 1 MB buffer fixed
it. Measured by `island_bench` (`benchmarks/2026-10-08-regional-baseline.json`, two founders,
1,000 steps): save 1.9 s, load 2.2 s, round trip hash-equal. Earlier uncompressed saves took 11-18
s on this machine, so most of that was writing 338 MB. The large-population snapshot figures
above (923 MB for 5,002 people) are the uncompressed numbers and were not re-measured; expect
roughly a seventh of the size.

`island_bench` also showed that **stepping does not use the extra cores**: 200 steps run at 976
steps/s with one thread and 932 with eight (speedup 0.96). The island step is effectively
single-threaded for a small population, so a faster CPU core helps more than more cores. A populated island does
not either: with 1,002 people, 60 steps run at 11.6 steps/s on one thread and 11.4 on eight (0.98), and
the people step is the main cost (about 86 ms per step, 5.2 s of 12 s) and is single-threaded. One
hourly human-store firing for a 1,000-person shard took about 5.3 s on this disk. Parallelising the
people step would be the first lever if a larger population ever needs to run faster than the
measured 117x real time at 5,000 people; it would have to keep results identical at any thread count.

## Correction: long-run memory per person (found 2026-10-09)

The populated-island figures above (1,002 people 594 MB; 5,002 people 1,017 MB) cover **two to six
simulated hours**. A person's record is small when fresh (about 0.06 MB as JSON) and was found to
grow without limit: `reproduction_system.sexual_activities`, an audit log of every reproductive
attempt, gained about 37 entries a day and reached 15,451 entries (5.7 MB) in 413 simulated days,
about 5 MB per person per simulated year, with no ceiling (UPSTREAM.md #29). Those short runs could
not show it, and neither could the founders-only runs (2 to 5 people: tens of MB over years). It is
fixed: the log keeps its 128 most recent entries.

Measured with a 26-person run (24 settlers plus the founders), a settler's full record at day 145:

| | Fresh | After the first month | Day 145 | Day 413 |
|---|---|---|---|---|
| Unfixed | 0.06 MB | - | - | 7.0 MB (growing about 5 MB a year) |
| Fixed | 0.06 MB | 1.67 MB | 1.67 MB (flat) | 1.68 MB at day 277 after the births, still flat (day 413 pending) |

Process memory for the same 26 people: 401 MB fixed against 521 MB unfixed at day 145, and 398 MB against 591 MB at day 277 with 32 people (the
unfixed run was growing; its reading then moved around as the allocator returned memory, so record
size is the cleaner measure). Taking the fixed run's 401 MB less the roughly 226 MB of the world
itself gives about 6.7 MB of process memory per person once their state has settled (the in-memory
form is larger than the JSON record). **So the honest long-run capacity is on the order of 1,000 people on 8 GB and
2,000 on 16 GB, not 5,000**; the 5,002-person figure is a short-run cost only. A long run at that
size has not been done.
