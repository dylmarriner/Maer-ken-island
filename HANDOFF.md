# Handoff

Written 2026-10-13 against `main` at `05fd6e5` (the merge of PR #6). This is the front door for
whoever picks the island up next, human or agent. It says what the project is, where it stands,
how to build and check it, and where everything else is written down.

**What is left to do, and what each item needs, is in [`WORK_REMAINING.md`](WORK_REMAINING.md).**

## What this is

Maer-Ken Island simulates a New-Zealand-sized island on Marr'Kena, an Earth-like planet with a
36-hour day, a 288-day year and 27° of obliquity (`fixtures/island/canon.json`), realistically:
every number that describes the real world is sourced and lives in a cited reference pack, and
every known departure from reality is a numbered row in `docs/island/DEVIATIONS.md`.

On it live the founders, Gem-D and Gem-K: full simulated humans (body, needs, cognition, emotion,
memory, drives and a free-will decision loop, from upstream `dylmarriner/Maer-Ken`) who eat,
drink, sleep, work and love by their own choices on an estate with a house, a shed, a workshop,
an armoury, a computer room and a garage. Every kilogram of carbon and water they and their
estate move is booked in a flux ledger that is audited at every step.

## Where it stands

| Phase | Scope | State |
|---|---|---|
| 0, 0b, 0c | Baseline, early human creator, canon and the realism standard | Done |
| 1 | Island domain and geophysics | Done; the owner's island is accepted (Gate 1 closed 2026-10-13) |
| 2 | Climate, weather, ocean, tides, hydrology, the coupled tick | Done (Gate 2 closed 2026-10-13) |
| 3 | Ecology, estate, 200,000 trees, founders, materials, labour, energy, timber | Mostly done. Open: species (Task 1b), every resource and material (Task 2). Gate 3 closed 2026-10-13 |
| 4 | World, scheduler, snapshot, per-human folders, commands, replay, interventions, runner, live dashboard | Done, including Task 2b, the deep-time spin-up (2026-10-14) |
| 4b | Energy, industry, town, economy | Hydro done (Tasks 1-3). Oil and gas, refinery, trades, money, food and timber chains, the town: not started |
| 5 | The application and performance | The web dashboard and the desktop app run and draw the island and estate; inspectors, benchmarks, upstream sync and pruning remain |

The newest work (merged 2026-10-12, PR #6):

- Köppen-Geiger biomes (D32).
- Maritime air and a vapour budget for the island's rain (D37, D38).
- A day-night energy balance for the land (D3, in part).
- Siler mortality fitted to SSA 2019 (D21).
- Founders who eat and drink when they choose to (D10), with desire that is mutual and has satiety.
- Lamps.
- Founders who fell trees with an axe.
- River micro-hydro for the estate: a 25 kW Francis plant on 138 m of head, sited from a simulated
  year of river flows, with a 2 MW town site lower on the same river (Phase 4b Tasks 1-3).
- Felled timber: a tree comes down as a green log, and a founder moves it by excavator, ute or
  hand to the workshop (D40).

`docs/handoff/SESSION_2026-10-12.md` is the full account.

The session of 2026-10-13 (production readiness, the realism research and programme, water at
home, soil carbon) is recorded in `docs/handoff/SESSION_2026-10-13.md`.

## Read in this order

1. **This file.**
2. **[`WORK_REMAINING.md`](WORK_REMAINING.md):** every open item with its sources, decisions,
   files, tests and finish line.
3. **[`docs/handoff/STATE_OF_THE_ISLAND.md`](docs/handoff/STATE_OF_THE_ISLAND.md):** the layer map,
   every system and its file, the reference packs, the pinned digests.
4. **[`docs/handoff/WORKING_PRACTICES.md`](docs/handoff/WORKING_PRACTICES.md):** the realism rules,
   the ledger, how and when to re-pin a digest, CI, the environment's traps, the owner's
   standing decisions.
5. **[`docs/ISLAND_SYSTEM_STATUS.md`](docs/ISLAND_SYSTEM_STATUS.md):** the evidence (tests and
   measurements) for every system that is done.
6. **[`docs/island/DEVIATIONS.md`](docs/island/DEVIATIONS.md):** D1-D40, every departure from
   reality and its status.
7. The plan for whatever you pick up, in `docs/superpowers/plans/`.

## Build and check

```bash
cargo fmt --all -- --check
cargo clippy --workspace --exclude island_ui --all-targets -- -D warnings
cargo test --workspace --exclude island_ui                       # fast tier (what CI runs)
cargo test --release -p mk_engine --test island_life_acceptance -- --ignored      # the week, audited
cargo test --release -p mk_engine --test regional_physical_coupling -- --ignored  # the year of weather
cargo test --workspace --exclude island_ui --release -- --ignored slow_            # the whole slow tier
cargo run -p island -- serve --scenario fixtures/island/default_scenario.json      # run it
```

- `island_ui` (Bevy) needs ALSA and graphics headers. CI builds it in its own job; a plain cloud
  container cannot.
- The fast tier takes about 28 minutes on CI.
- `docs/TESTING.md` has the tiers and every slow and deep test.

## Things that will bite you

- **The week digest.** `WEEK_DIGEST` in `crates/mk_engine/tests/island_life_acceptance.rs`
  (`bbbc144d…cb7a`) pins one simulated week. Any change to what the founders or their world do
  moves it. Re-pin only after the audit and the reference-pack checks pass, and say in the
  commit that behaviour changed.
- **The planetary topology pin** (`e4ece246…73e2`, `tests/topology_hash_baseline.rs`) must not move
  because of island work. Shared ocean and climate code is shared with the planet; that is why
  D25 needs an owner decision.
- **Sources.** Never type a real-world value from memory. Fetch the source, transcribe it into the
  pack with its citation. Firecrawl credits are exhausted; WebSearch and the TinyFish fetcher
  work. The container's network blocks most hosts directly.
- **Upstream.** Changes to imported code are divergences: record each in `UPSTREAM.md`. The
  human-runtime work of PR #6 changed `humans/{autonomy,circadian,lifecycle,needs,reproduction}`
  and `climate/mod.rs`; those are listed there and need upstream pull requests.
- **Disk.** A debug build of the whole workspace with Bevy can fill a small disk.
  `CARGO_INCREMENTAL=0` and `CARGO_PROFILE_DEV_DEBUG=0` help, and `--exclude island_ui` helps
  more.
- **Stale release artefacts** once showed a field as missing that was present
  (`cargo clean --release -p <crate>` fixed it).

## Owner decisions outstanding

- Gates 1, 2 and 3 are **closed** (owner, 2026-10-13): the work built on the island, its weather
  and the estate plan is the approval. Do not ask for them again.
- D25: change the sea's heat exchange regionally, or in shared code and re-pin the planet.
- Phase 4b: enterprise ownership splits, employment and dividend policies, the town's size.
- Phase 4b, decided 2026-10-13 (`docs/handoff/WORKING_PRACTICES.md`): who lives where, the
  estate's staff, farms on the estate that start small and grow on Pukekohe-sized land, the
  founders owning the town (hospital, bank and power included), monthly profits with a reserve,
  founders deciding asset sales, and free will in place of built towns. Still open: the exact
  payout share and what the bank does.
- Phase 5: the two founder portraits, the performance targets, and whether formal predictive
  processing is wired into decisions or stopped.
- Whether the excavator gets a grapple (D40 rigging) and whether imported drilling rigs are
  accepted (D19).
- Remove the Snyk GitHub App (out of quota).

## Process

- Work on a branch; open a pull request; CI must be green (fmt, clippy, fast tests, desktop,
  computer service, web, dependency audit) before merging to `main`.
- After every feature or fix, update `docs/ISLAND_SYSTEM_STATUS.md`, the deviation register, the
  plan's status notes, and `WORK_REMAINING.md`.
