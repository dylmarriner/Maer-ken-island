# What Phase 4 costs the island

Measured 2026-10-08, cloud container, release build, commit `3414d76`.

    cargo test --release -p mk_engine --test island_phase4_cost -- --ignored slow_ --nocapture

`benchmarks/island_week.md` measures the Phase 1-3 island and then says "Not in this number:
Phases 4, 4b and 5 (persistence, replay, the app…)". Phase 4 is built, so this is that number.

The full island from `fixtures/island/default_scenario.json` — 199,997 stems, both founders, every
grid — bootstrapped fresh for each case, 360 steps of 60 s (six hours of island time, long enough
to average over the hourly and six-hourly cadences).

| Case | Per step | Added |
|---|---|---|
| The island as Phases 1-3 left it | **4,296 µs** | — |
| …with a folder per human syncing (Task 3b) | **5,028 µs** | **+732 µs, +17%** |
| …with a command recorded every hour (Task 4) | 4,547 µs | +251 µs |
| Bootstrap | 15.7 s | |
| Snapshot save (Task 3) | **27.4 s** for 43.5 MB on disk | |
| Snapshot load | 3.7 s | |

**Nothing Phase 4 added changes the world.** All four canonical digests are identical —
`a2900f14cce76306…`: the island with nothing attached, with folders syncing, with commands
recorded, and after a trip through a file. That is the half of this benchmark that matters most,
and it is asserted rather than printed.

## Three things worth acting on

**A snapshot stalls the simulation for 27 seconds.** `ControlCommand::Snapshot` runs on the thread
that owns the island, so a dashboard operator asking for one freezes the world for nearly half a
minute with no indication beyond the clock stopping. 43.5 MB written from 338 MB of JSON through
deflate is most of it. Nothing is wrong with the result — the digest survives the round trip — but
this wants either doing off-thread from a cheap clone, or saying plainly in the UI that it will
take that long.

**A folder per human costs 17% of a step for two people.** `set_auto_sync(false)` already stopped
the per-step full rewrite, and what remains is the store's own cadence. At two founders that is
732 µs; the cost is per human, so Phase 4b's ~200-person town would multiply it. Worth re-measuring
against population before the town arrives rather than after.

**The command path is nearly free**, and the figure above is an upper bound: a command every hour
is far more than any real session, and six commands over 360 steps moved the step by 251 µs — most
of which is the pause command's own bookkeeping rather than the log.

## A correction

`serve/sim.rs`'s `slow_what_a_step_and_a_digest_cost` reports **a step at 1.2 ms**, and that figure
reached `docs/ISLAND_SYSTEM_STATUS.md`, the `DIGEST_EVERY` doc comment and a pull request. It is
measured on an island with `tree_cap = 50` — fifty trees, not 199,997. The comment says so for the
*digest* ("that measurement is on a 50-tree patch") but not for the step.

The real island steps at **4.3 ms**, which `island_week.md` already implied: 49 s for 10,080 human
steps is 4.9 ms each. So the step figure was roughly **3.5× optimistic**, and the claim that a
digest is "650× the step" is wrong — on the real island an 800 ms digest is about **186×** a
4.3 ms step.

The decision it supported does not change. A digest every step would still cost 800 ms against
4.3 ms of simulation, and at `DIGEST_EVERY = 60` the amortised 13.3 ms plus a 4.3 ms step caps the
island near 3,400× real time, which is what the comment claimed by a different route.
