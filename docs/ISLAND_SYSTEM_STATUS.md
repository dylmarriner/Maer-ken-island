# Island system status

Updated 2026-10-08. This records what is implemented, measured and verified, and the known
fidelity limits. The project is **not complete**: see "Not done".

## Implemented

| System | State | Evidence |
|---|---|---|
| Baseline, human creator, canon and realism suite (Phases 0, 0b, 0c) | Done | `crates/mk_engine/tests/human_realism.rs`, `fixtures/reference/` |
| Island domain and geophysics (Phase 1) | Done; owner's island provisional | `fixtures/island/default_profile.json`, `docs/previews/phase1/` |
| Climate, weather, storms, ocean, tides, hydrology, coupled tick (Phase 2) | Done; owner review pending | `crates/mk_engine/src/regional/`, `docs/previews/phase2/`, year-long reference test |
| **Integrated island life**: physics + ecology + estate + 200,000 trees + founders + energy + materials, audited (Phase 3 Task 9) | Done; one week measured | `regional/life.rs`, `tests/island_life_acceptance.rs`, `benchmarks/island_week.md` |
| Island ecology fields: biomes, NPP, producer biomass (Phase 3 Task 1a) | Done | `regional/ecology.rs`, `tests/regional_ecology.rs` |
| Grid topology for human/organism movement, births and perception (Phase 3 Task 6) | Done; planetary results bit-identical | `topology.rs`, `tests/topology_hash_baseline.rs` |
| Founders' estate laid out in metres (Phase 3 Task 5) | Done | `regional/estate_layout.rs`, `tests/island_estate_layout.rs` |
| Canonical founders on the island: observation, bedrooms, computer rule (Phase 3 Task 8) | Done except Task-3-dependent hooks | `regional/humans.rs`, `humans/observation.rs`, `tests/island_human_runtime.rs` |
| Physical materials: gather, hunt, burn, calcine, eat, respire, drink, build through the flux ledger (Phase 3 Task 3) | Model done and audited inside the island tick. A meal is the carbon its eater respired since the last one, so adults hold at 1.00x of their expected body carbon over a week. Who harvests and when is still the routine's choice, not each human's (D10) | `regional/materials.rs`, `regional/life.rs`, `tests/island_material_flows.rs`, `tests/island_life_acceptance.rs` |
| Timed, energy-costed work: tasks, BMR x MET, walking, load limits (Phase 3 Task 3b) | Model done. Energy is wired: the island tick costs resting metabolism at the body's BMR times the MET of sleeping or sitting, from the circadian clock, and `humans/lifecycle.rs` charges walking, mining and building their Compendium METs. Durations are not: every action still takes one tick, and the actions with no pack row cost the baseline (D9, D22) | `regional/labour.rs`, `regional/life.rs`, `humans/lifecycle.rs`, `tests/island_labour.rs`, `tests/human_realism.rs` |
| Estate fuel and electricity (Phase 3 Task 4b) | Done | `regional/energy.rs`, `tests/island_energy.rs` |
| Individual trees and stands on the estate patch (Phase 3 Task 7) | Done | `regional/local_vegetation.rs`, `tests/island_local_vegetation.rs` |
| Island scenario and estate placement (Phase 3 Task 4) | Done | `mk_island/src/scenario.rs`, `regional/property.rs`, `fixtures/island/default_scenario.json` |
| Complete island snapshot: save, restore, tamper and canon rejection (Phase 4 Task 3) | Done. A full island — a dozen 1,152,000-cell grids, 200,000 stems, the founders and the RNG streams — writes atomically to one deflated, digest-prefixed file and restores to the same state digest, then runs on identically. The domain, grid topology and labour table are derived again on load rather than trusted from the file | `io/island_snapshot.rs`, `regional/life.rs`, `tests/island_snapshot.rs` |
| A folder for every human, under a run of its own (Phase 4 Task 3b) | Done. `enable_human_store` claims a run id from a counter in the save root, writes a folder per person, and syncs on the store cadence, at every snapshot save and the moment somebody dies. Failed writes are counted and reported, never discarded. The state digest is identical with the store on, off, or failing | `regional/human_store.rs`, `regional/life.rs`, `tests/island_human_store.rs` |
| External commands and deterministic replay (Phase 4 Task 4) | Done. Every command reaching the island from outside is recorded with the tick and order it applied at, and replaying the log reaches the same canonical digest — measured on a live dashboard session: three people created through the API at ticks 480, 544 and 609, replayed on their own to the same digest `628d33bc`. `IslandCommand::Intervention` carries upstream's `InterventionAction` itself, so a log records what was actually asked for; a run with climate edits, a water injection, a spawn and a biomass injection in it replays to the same digest, through a file as well as in memory | `regional/commands.rs`, `regional/replay.rs`, `regional/interventions.rs`, `tests/island_replay.rs`, `tests/island_interventions.rs` |
| Operator interventions, adapted to the island (Phase 4 Task 4) | Done. **Applied:** `Pause`/`Resume`/`Step`, `ModifyClimate` (all four parameters, heat booked from `OperatorIntervention` as upstream books it), `InjectBiomass{Producers}`, `InjectResource{Water}`, `ConstructStructure`, `SpawnHuman`, `RemoveHuman`. **Refused by name and reason**, never a silent no-op: terrain edits (canon, hashed into the digest), the animal biomass types (no species at island scale), the non-water resources (the stores are named materials), `InjectEnergy`, `TriggerDisturbance`, `ModifyScenario`, `Scrub`/`Branch`/`Fork`. The table is in `UPSTREAM.md` item 27 and a test walks every variant. A `Location` off the domain is refused rather than clamped; `RemoveHuman` clears the registry, the estate table, the body in the material ledger and the two hashed accumulators | `regional/interventions.rs`, `tests/island_interventions.rs`, `UPSTREAM.md` |
| An island buildability rule (Phase 4 Task 4) | Done. Upstream's `physics::MAX_CLIMB_HEIGHT_M` is an **absolute** 2 m of relief between neighbouring cells — a statement about a grid's resolution, not terrain. On the island's 2 km cells that is a gradient of 0.1%, and measured it admits **0 of 66,116 land cells**, the founders' estate at 8.6% included. `MAX_BUILD_GRADIENT` asks the same question as a gradient, at 1 in 3 (~18°), where ground is conventionally classed very steep. It admits 99.9% of this island, which is the island being gentle at 2 km rather than the rule being lax, and it cannot judge a building plot — only exclude mountainside, which is all it claims | `regional/geophysics.rs`, `resource_economy.rs` (`place_structure`), `tests/island_interventions.rs`, `UPSTREAM.md` item 28 |
| Headless runner and Phase-4 acceptance (Phase 4 Task 5) | Done, `island replay` included. `island run` advances a scenario or a saved island and prints the canonical digest; `island inspect` describes a snapshot without running it. Two runs of one scenario and seed agree, and stopping at 60 steps and resuming reaches the same digest as 120 straight through, measured on the full island from the command line | `apps/island/src/run.rs`, `tests/island_runtime_acceptance.rs` |
| The island behind the dashboard, read and write (Phase 4 Task 6) | Done, but for a map. `island serve --scenario` runs `IslandLife` on its own thread and publishes a projection after every step; `GET /api/world` and the overview read it. `POST /api/world/humans` queues a creation applied between steps, and the creator page picks a room from the estate's own layout. A person created there is in the world and breathing. The roster page shows the island's own people with their full records and where they are, and the overview can pause the island or change its speed — measured: pause held the clock still at tick 1080. A person can be put in a room or on a cell of the island, with the sea refused by name, and `--import-0b` carries a Phase-0b population in as ordinary creations. `/api/properties`, `/api/economy` and `/api/timeline` are served, and the `/island` page reads them: the estate's buildings and its eighty-odd things, what has been built, and everything that has reached the island from outside. (This row previously said they were not served "because the projection does not carry the data". That was wrong, and measuring the island is what showed it: the properties and the replay log were there all along, and the economy is genuinely empty rather than absent — it says so in the reply, and fills as soon as somebody builds.) The overview can also step the island exactly n steps and then hold, and the island page can intervene in it, refusals included | `serve/sim.rs`, `serve/projection.rs`, `serve/server.rs`, `regional/create_human.rs`, `tests/island_create_human.rs`, `apps/island/tests/serve.rs` |
| Web dashboard: overview, roster with a readable per-person record, the island's own page, Human Creator, JSON API (`island serve`) | Done for the human-only bootstrap; the Phase 5 desktop app is separate. Every path has been driven in Chromium against the full island with no console errors and no failed requests | `apps/island/src/serve/`, `apps/island/static/`, `apps/island/tests/serve.rs` |

## Measured (release build, development machine)

| Quantity | Result | File |
|---|---|---|
| Island physics, one simulated year | 22 s, 213 MB peak | `benchmarks/phase2_physical.md` |
| Human runtime, 5,000 people | 23 us/human-step, 153 MB peak | `benchmarks/humans_population.md` |
| **Combined island, one simulated week** (physics, ecology, estate, 200,000 trees, 2 founders, energy, materials) | 49 s, **307 MB peak** | `benchmarks/island_week.md` |
| Complete island snapshot, one day in | **44 MB** on disk (338 MB before deflate) | `tests/island_snapshot.rs::slow_a_full_island_survives_a_trip_through_a_file` |
| One island step, and what the dashboard publishes after it | On a **50-tree** test island: step 1.2 ms, projection **3.6 ms**, every human's full record **336 us** (two people), canonical digest **800 ms**. The digest is the two 1,152,000-cell grids and the humans' JSON rather than the stems, so it carries over; **the step does not**. This row previously gave 1.2 ms as the island's step and called the digest "650x" it | `serve/sim.rs::slow_what_a_step_and_a_digest_cost` |
| One step of the **real** island (199,997 stems) | **4.3 ms**, so a digest is about **186x** a step, not 650x. A folder per human adds **+732 us (+17%)** for two people; a command an hour adds 251 us; a snapshot save **stalls the simulation thread for 27 s** for 43.5 MB, and loads in 3.7 s. Every one of those leaves the canonical digest identical | `tests/island_phase4_cost.rs::slow_what_phase_4_costs`, `benchmarks/phase4_cost.md` |

The combined island (without persistence, the scheduler, a larger population, the refinery/town and the app) is measured above; those remain unmeasured until Phases 4, 4b and 5.

## Verification commands

    cargo fmt --all -- --check
    cargo clippy --workspace --all-targets -- -D warnings
    cargo test --workspace --release
    cargo test --workspace --release -- --ignored slow_

`cargo test --workspace` in debug builds a large target directory (tens of GB with debug info and
incremental compilation on). On a machine with a small disk, `CARGO_INCREMENTAL=0` and
`CARGO_PROFILE_DEV_DEBUG=0` keep it to a fraction of that.

The last full-workspace run was **1,226 passing, 0 failing, 22 ignored, across 88 binaries**
(2026-10-08, commit `94b97b1`), with `cargo fmt --all -- --check` and
`cargo clippy --workspace --all-targets -- -D warnings` clean.

Run it with `--no-fail-fast`. Without it cargo stops at the first failing binary, and the run
before this one reported "50 passed" across four binaries — a partial result that reads exactly
like a whole-workspace one.

The **slow tier passes** — `cargo test --workspace --release --no-fail-fast -- --ignored slow_`,
**12 tests, 0 failures**, on `8564846`: a simulated week on the island against the reference
packs, a simulated year of weather against them too, ten thousand years of seismicity, a
full-size island through a file, a day in one step against 1,440 minute steps, the human runtime
to five thousand people, Phase 4's cost and its long-horizon evolution, the share of a human step
that predictive processing takes, the cost of a crowded cell, and what publishing conversations
costs.

The two reference-pack tests are the ones that matter most when reading a diff like this one:
they compare a simulated week and a simulated year against recorded values, so they are what
would catch a change to simulation behaviour that the fast suite cannot see.

`slow_the_human_runtime_stays_small_and_roughly_linear_to_five_thousand_people` used to fail on
a bad draw, and no longer does. It asserts `cost(5000) <= 4 x cost(200)` — a ratio against the
smallest and noisiest sample. Two hundred humans fit in cache and five thousand do not, so
per-head cost genuinely grows and the 4x is the tolerance for that. On a container where the
200-human run came in at 17 us instead of its usual 23-25, the bar dropped to 68.6 us and a
perfectly ordinary 77 us failed it.

**The fix is a median of nine runs per population**, and it corrects something this page used to
say. The earlier text claimed that averaging several runs "is a change to what the test claims,
not a tidy-up, so it is left for whoever decides what the budget should be." That was wrong. A
median changes nothing about what is compared: each run is still the same four steps on the same
population, so `HumanSystem::step`'s fixed per-call cost still lands on every population equally.
It only stops a single unlucky sample from setting the bar. Measured over three consecutive runs
the ratio came out at **1.69, 1.86 and 1.71** against a bar of 4, where single samples had
produced 3.2 and a failure. A warm-up step before the clock starts was a real but partial
improvement on the way here, and is still in place.

Printing every run rather than only the median turned up something a single figure hid: **the
runs climb, monotonically, every time.** A typical 200-human row reads 17, 25, 35, 41, 49, 65,
69, 71, 79. That is a trend, not jitter, and the likeliest reason is that these humans accumulate
state as they live — conversation history, memory, relationships — so a later step genuinely
costs more than an early one. It is why the medians rose when the repeat count went from five to
nine, and it means the absolute microsecond figures here are only comparable at a fixed
`REPEATS`. The ratio is unharmed, because every population is measured over the same history.
Whether that growth is acceptable at a lifetime's scale is a real question about the human
runtime, and nothing here answers it.

One thing not to try: scaling the step count inversely with population so each size does equal
total work. It was tried here and is wrong. `HumanSystem::step` has per-call cost that does not
depend on how many humans it holds, so giving 200 humans 100 steps while 5,000 get 4 amortises
that fixed cost over 25x fewer people — measured, it put the 200-human figure at 84 us against
its usual 17-25 and made the test compare two different things.

CI has not run any of it: every
job since 2026-10-06 completes in 1-4 seconds with `runner_id: 0`, an empty `runner_name` and no
`steps` array — no runner is ever assigned, so nothing executes to pass or fail, and `main` is
red the same way on a commit that passed three times before. The repository is private, so
Actions minutes are billable, and that signature is what an exhausted allowance or spending
limit looks like; it is the first thing to check. Until it is, every figure here is local.

## Known fidelity limits

Every known departure from reality is in `docs/island/DEVIATIONS.md` (D1-D34). The largest open
ones: no sea/land breezes (D3); storm structure is parametric (D4); river channels have no
in-channel storage (D28); the regional tick audits tidal heat only (D29); biomass residence times
are round estimates (D30); production ignores soil nutrients (D31); a parent and child converse
at any distance while siblings need adjacency (D34); every human action still takes exactly one
tick whatever it is (D9); the actions with no Compendium row cost the resting baseline (D22);
who harvests and when is the routine's choice rather than each person's (D10); machine use off
the estate's electrical system is still free (D13).

## Not done

- Phase 3 Task 1b (island-scale species) and Tasks 2 and 9 (and the economy/lifecycle wiring of Tasks 3 and 3b): materials and resources, physical
  materials, time and energy of actions, acceptance.
- Phase 4 is **done**: world composition, the cadence scheduler, the island snapshot, a folder
  per human, external commands and replay, operator interventions, the headless runner, and the
  dashboard with a world behind it. What Task 6 still lacks is a map: the island page describes
  the world in words and tables, and nothing draws it.
- Phase 4b: energy, industry, town, economy.
- Phase 5: the desktop app (`island-ui`, Bevy) replacing the human-only bootstrap, performance
  work, pruning, final benchmarks. The `island serve` web dashboard covers the human-only
  bootstrap in the meantime. With `--scenario` it now has a world and a clock; it still has no
  map.
- Owner reviews: Gate 1 (island) and Gate 2 (weather previews).
- Upstream pull requests for every divergence in `UPSTREAM.md`.
