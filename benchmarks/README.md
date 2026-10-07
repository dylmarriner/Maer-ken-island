# Benchmarks

Every number here is measured on the real regional paths; nothing is estimated.

## Repeatable baseline

```bash
cargo build --release -p island
target/release/island_bench --ticks 1000 --dt 60 --tmp target/bench --compare-threads
target/release/island_bench --ticks 120 --people 5000 --tmp target/bench      # a populated island
```

`--tmp` must be on a real disk: `/tmp` is often RAM-backed, which hides disk time and competes with
the memory being measured. The output is JSON: `bootstrap_ms`, `ticks_per_second`,
`simulated_days_per_real_minute`, `subsystem_ms` (humans, households, hourly physics, six-hourly
ecology, human-store cadence), `snapshot_bytes`, `save_ms`, `load_ms`, a check that the snapshot
round-trips to the same state hash, `human_store_sync_ms`, `peak_rss_bytes` (`VmHWM`, Linux),
`threads` and, with `--compare-threads`, the single-thread against all-cores speedup.
`spin_up_1000y_seconds` is `null` until a deep-time spin-up exists.

Wall-clock numbers depend on what else the machine is doing; on the shared development machine
they vary by 10-50% between runs, and a one-off I/O stall once added 8 s to a 3 s run. Repeat a
measurement before trusting a single figure. Peak memory is the stable column.

## Files

| File | What |
|---|---|
| `2026-10-08-regional-baseline.json` | the `island_bench` baseline for the default island, two founders |
| `island_week.md` | one simulated week and one simulated year of the founders |
| `island_population.md` | 200 to 5,000 people: time, memory, snapshots, the human store |
| `island_snapshot.md` | snapshot size and time (see the compression note in `island_population.md`) |
| `phase2_physical.md` | the island physics alone |
| `humans_population.md` | the human runtime alone |
