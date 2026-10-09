# Island system status

Updated 2026-10-09. This records what is implemented, measured and verified, and the known
fidelity limits. The project is **not complete**: see "Not done".

The island stopped being one program on 2026-10-09. It is now a backend that simulates and
serves, and frontends that read it: a web dashboard that can be hosted on any machine, and a
desktop application. `docs/island/DEPLOYMENT.md` is how to run them apart.

The desktop application **draws the island**, verified by a frame rather than by argument:
`scripts/render-smoke.sh` runs it on a virtual screen against a real island, photographs a frame
and asserts the picture is of something. It runs in CI. What it still cannot say is how the
thing feels to use or what it costs on a real GPU — every frame so far was rasterised on the
CPU. See `docs/island/RENDER_STACK.md`, which also records that an earlier version of this
document was wrong about whether rendering here was possible at all.

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
| Web dashboard: overview, roster with a readable per-person record, the island's own page, Human Creator, JSON API (`island serve`) | Done. Every path has been driven in Chromium against the full island with no console errors and no failed requests | `apps/island/src/serve/`, `apps/island/static/`, `apps/island/tests/serve.rs` |
| **The island as a backend, read by frontends on other machines** | Done. `ISLAND_READ_TOKEN` gates every `/api` endpoint in front of the routes, so an unauthenticated caller cannot tell a path that exists from one that does not; `--allow-origin` names the browsers that may ask; `/api/version` says what this server speaks and can do, and `/api/elevation.bin` serves the terrain as numbers for a client that builds a mesh. Measured on a real server bound to `0.0.0.0` with both tokens: version and health answer unauthenticated, everything else 401s without a token and 200s with either, an unnamed origin gets a 403 naming `--allow-origin`, and forty concurrent readers all got 200 | `serve/auth.rs`, `serve/server.rs`, `apps/island/tests/client.rs` |
| **One wire schema, shared** | Done. Every shape that crosses the network is defined once, in a crate that depends on `serde` and nothing else, so a client links it without building the island. Two tests keep it honest: one compares the key sets both ways (the silent direction is a field added to the server's projection and not to the schema, which would reach no frontend and look like a server that never sent it — verified to bite), and one puts every intervention constructor through the real `InterventionAction` | `crates/mk_island_api/`, `apps/island/tests/schema.rs` |
| **A typed client** | Done. `mk_island_client` draws the four distinctions a frontend needs and HTTP's status codes do not: nobody answered, somebody answered and said no, this server is a different version, and the answer was unreadable. Tested against the real backend rather than a mock, which found three field-name disagreements and one real gap in the server | `crates/mk_island_client/`, `apps/island/tests/client.rs` |
| **The web dashboard, hosted away from its island** | Done. `island serve --frontend-only --backend URL` serves the pages for an island elsewhere; `island export --to DIR` writes the same bytes for any web server. Driven in Chromium against two processes and again against `npx http-server`: every page loads with no console errors and no failed requests, arriving with no token shows the island's own words and opens the token field, and the map draws the real island — 4,604 distinct colours over 2,401 km — from both | `apps/island/static/app.js`, `serve/pages.rs` |
| **The desktop application (Phase 5)** | Draws the island and the estate. `island-ui` on Bevy 0.19 reads an island over HTTP, remote or embedded, through one code path; its projection, terrain chunking, camera, model table and data path are tested headlessly, and `scripts/render-smoke.sh` renders a frame on a virtual screen and asserts the picture is of something (CI runs it and uploads the frame). A rendered frame shows the landmass and its relief, the sea floor with the trench and the outer rise, both panels, and the clock running at 59.9x of the 60x asked for. **Two defects were in the first frame it ever drew** and neither was visible to any unit test: every terrain triangle was wound to face downward and so was backface-culled, and `bevy_egui` without `default_fonts` had no font data, so every panel drew as an empty box. `--view estate` opens on the founders' estate at one unit to the metre: five buildings at their real footprints, fourteen rooms as floors, 84 things, and 52 of them carrying a Maer-Ken model. The models are decoration on the simulation's own geometry rather than a substitute for it, so an empty `assets/` costs the decoration and not the estate -- measured at 3,457 colours against 2,172, which the picture alone would not have caught, so the check reads the log for named missing models too. A mouse and a keyboard are answered and measured -- a held drag moves 39.8% of the pixels, a scroll 59.0%, the `e` key 32.5%. Unverified still: what it costs on a real GPU -- every frame so far was rasterised by lavapipe on the CPU, and the application asks for the high-performance adapter in its own code and names the one it got | `apps/island_ui/`, `scripts/render-smoke.sh`, `docs/island/RENDER_STACK.md` |
| **The opt-in computer service** | Done, and off. A founder at their own machine in a powered computer room can search the web and send email for real, through a vendored Node service, when `COMPUTER_ACTIONS_ENABLED=1`. It makes the island irreproducible on purpose, which is why `run`, `replay` and `inspect` never attach it. Off, and on-but-failing, both leave the state digest identical — the costs are charged on success only | `services/computer-service/`, `crates/mk_engine/tests/island_computer_service.rs` |

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
    cargo test --workspace --release --no-fail-fast
    cargo test --workspace --release --no-fail-fast -- --ignored slow_

    cd services/computer-service && npm ci && npm test
    for f in apps/island/static/*.js; do cp "$f" /tmp/x.mjs && node --check /tmp/x.mjs; done

`--workspace` now includes `island_ui`, which pulls in Bevy: 392 crates and about nine minutes
from cold on four cores. For a run that only needs the island, `--exclude island_ui` skips it,
and that is what CI's fast job does — a formatting mistake should be reported in two minutes
rather than after a Bevy build. `island_ui` has its own CI job, with its own clippy, build and
tests.

`cargo test --workspace` in debug builds a large target directory (tens of GB with debug info and
incremental compilation on). On a machine with a small disk, `CARGO_INCREMENTAL=0` and
`CARGO_PROFILE_DEV_DEBUG=0` keep it to a fraction of that. With Bevy in the tree that matters
more than it did: a debug build of this workspace filled this container's disk once.

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

`a_new_person_can_be_read_the_moment_they_exist` in `apps/island/tests/serve.rs` **used to fail
between one run in three and every run**, depending on load. It is fixed, and the fix is worth
describing because three attempts before it were not.

The cause was never that test. Every test in that file bootstraps its own island on its own sim
thread, and this container has **four cores**. The dominant cost was not the stepping but the
grids: two 1,152,000-cell grids built on every bootstrap and hashed on every `DIGEST_EVERY`,
which `tree_cap` does not shrink. A dozen of those starves the runtime answering the HTTP polls
the tests make, and a sixty-second deadline goes by.

The fix is `IslandProfile::test_small` — 240 x 192 medium cells, **25x fewer** — which seven other
test files already use. It needs its own seed: `IslandLife` validates the coastline it generates
against the profile's shape rules, and the default scenario's seed makes a small island with
"0 major headlands, need 3". Seed 16 is the first that passes, found by trying 0, 1, 2 ... through
`RegionalPhysicalState::bootstrap`, which runs that check before the expensive spin-up so bad
seeds cost nothing: **17 tries, 1.3 seconds**. Two tests also had `-41.03, 173.56` written into
them, which is land on the default island and sea on this one; they now read the estate's position
from `/api/world` instead of pinning themselves to one geography.

**Measured: five consecutive runs, 19 passed, 0 failed, about 18 seconds each**, against roughly
130 seconds and a one-in-three failure before.

Three things were tried first and are recorded so nobody repeats them. Pacing the dashboards at
`Times(600)` instead of `AsFastAsPossible` thinned the digests and did not fix it. Turning the
digest off in the tests did not fix it either, and left behind an unreachable knob that review
caught. Blaming the test count was wrong: removing the test this branch added left the same
failure. Each of those was a guess that looked right; the profile swap was the first thing that
was measured before it was believed, and it is also the only one that worked.

**The refactor this paragraph used to leave for later is done.** The seven tests in that file
that only *read* an island now share one, built once on a thread of its own and served for the
life of the process; the eleven that create a person, pause the clock or intervene keep an
island each, because the next test to read a shared one would see what they did. That is seven
fewer islands and seven fewer simulation threads.

Measured, median of three: **22.0 s to 20.2 s idle, and 57.4 s to 44.1 s under three times
oversubscription** (eight busy loops on four cores). The gain is where it should be — 23% where
the threads compete, 8% where they do not.

**It is not established that this fixes the flake, and the honest reason is that the flake did
not reproduce.** On the old fixture, under those same eight busy loops, the suite passed 25 of
25 twice. The failure seen on 2026-10-09 was under a Bevy build, which is memory and I/O as well
as CPU, and synthetic load did not stand in for it. So: the cause named above is gone and the
suite is faster and less contended, and whether that was sufficient is still a question. A
failure of this one test alone, with the others passing, remains the thing to re-run before
believing.

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

Every known departure from reality is in `docs/island/DEVIATIONS.md` (D1-D36). The largest open
ones: no sea/land breezes (D3); storm structure is parametric (D4); river channels have no
in-channel storage (D28); the regional tick audits tidal heat only (D29); biomass residence times
are round estimates (D30); production ignores soil nutrients (D31); kin repeat themselves,
because a spoken line is the speaker's internal monologue and that is deliberately the same
whenever their state is (D36); every human action still takes exactly one
tick whatever it is (D9); the actions with no Compendium row cost the resting baseline (D22);
who harvests and when is the routine's choice rather than each person's (D10); machine use off
the estate's electrical system is still free (D13).

## Not done

- Phase 3 Task 1b (island-scale species) and Tasks 2 and 9 (and the economy/lifecycle wiring of Tasks 3 and 3b): materials and resources, physical
  materials, time and energy of actions, acceptance.
- Phase 4 is **done**: world composition, the cadence scheduler, the island snapshot, a folder
  per human, external commands and replay, operator interventions, the headless runner, and the
  dashboard with a world behind it. **The map it lacked is drawn, and it is clickable**:
  `/api/map.png` serves the elevation render `island_preview` already made for the headless
  galleries, `/api/cell/<row>/<col>` answers for one cell, and `/island` shows the picture and
  reads the cell you click. A click names the cell, its coordinates, whether it is land or sea,
  its elevation and whether anything could be built there, and offers it to the intervention
  form — which until now could target **only the estate**, leaving the rest of the island
  unreachable from the page. Verified in Chromium against the real island: clicking the top of
  the picture gives row 941 at 38.2° S and the bottom row 20 at 43.8° S, and an intervention
  aimed at a clicked cell came back "put 100 kg of water into the island's stores, asked for at
  cell (624, 575)".

  **It now zooms, and it draws what is on it.** The `<img>` is a canvas holding the view in
  domain metres, and zooming in improves the picture in place rather than magnifying one render:

  | Zoom | What is actually drawn | Where it comes from |
  | --- | --- | --- |
  | Whole island | elevation, or standing vegetation, at one pixel per 2 km cell | `/api/map.png`, `/api/vegetation.png` |
  | The estate's 4 km patch | stand cover at one pixel per 5 m cell, 400× finer | `/api/patch.png` |
  | Within 807 m of the estate | ~200,000 individual stems, each at its own trunk diameter | `/api/trees?x0..` |
  | Any zoom | the founders at their own metres, structures and the estate at their cells | `/api/world`, `/api/economy` |

  Verified in Chromium against the real island: the whole island at 2,401 km across, the patch
  at 4 km with 3,904 of 186,718 stems drawn, and 40 m across with 100 stems, "which is every one
  of them".

  What it still does not show, and does not pretend to:

  - **Species.** `TreeInstance` carries `PlantKind` — Tree, Shrub or Grass — and nothing else.
    The engine's species system in `organisms/runtime.rs` is not wired into the island's
    vegetation; that is Phase 3 Task 1b, above, and still open. The page's key says so in those
    words rather than colouring three kinds and calling them species.
  - **Crowns.** A stem is a position, a height and a trunk diameter. Each is drawn at its own
    diameter with a floor so a narrow one stays visible, and the key says that is what the dot
    size is.
  - **Individual trees anywhere but the wood.** Beyond `individual_radius_m` — which is 807 m on
    the default scenario, not the scenario's requested 1,000, because the engine shrinks it to
    stay under `tree_cap` — vegetation is stand cover per cell. The viewer draws the circle, and
    the key says an empty box is not bare ground.

  Two things the viewer had to be told about the island rather than guess, both found by
  looking at what it drew: the estate's yard is cleared, so the middle of the wood is a hole
  and "to the trees" aims at a stem rather than at the centre; and inside the circle the stand
  cover is drawn paler because the stems' carbon has been taken out of it and put into the
  trees — the same carbon, counted once.

  Still missing from a map proper: the Creator page does not use it yet, only the intervention
  form.
- Phase 4b: energy, industry, town, economy. **Blocked at its own first step, and the block is
  the network rather than the work.** Task 1 Step 1 is "assemble cited reference values" --
  turbine efficiency curves by type and head range, Darcy-Weisbach penstock friction, oil
  reservoir recovery factors and Arps decline parameters, crude assays, refinery energy per
  barrel. Verified from this container on 2026-10-08, not assumed:

  ```
  https://doi.org               000
  https://www.ieahydro.org      000
  https://canyonhydro.com       000
  ```

  `000` is curl for no connection at all. Writing those values from memory and attaching
  citations to them would look exactly like sourced data and would not be any -- the provenance
  would be invented even where the number happened to be right, which is the failure
  `REALISM.md` exists to prevent. Everything downstream (sizing, staffing, the town) hangs off
  that table, so the phase waits on reachable sources or on the values being supplied.
- Phase 5, partly. `island-ui` exists, builds on Bevy 0.19 with no system graphics packages
  (9 min 19 s from cold, 392 crates), reads an island remotely or embedded through one code
  path, and **draws it** -- verified by a photographed frame in CI rather than by argument.

  A mouse and a keyboard are answered, and measured: `scripts/interaction-smoke.sh` drives the
  running application with xdotool and compares frames. A held drag moves 39.8% of the pixels,
  a scroll 59.0%, the `e` key 32.5% and clicking the estate row 59.5%. CI runs it and uploads
  the frames.

  What rendering has **not** answered is what it costs on a GPU, because every frame so far was
  rasterised by lavapipe on the CPU. The application asks for the high-performance adapter in
  its own code and says which one it got, but no frame-rate figure from real hardware is
  claimed anywhere. Other window sizes are untested too.

  The estate is drawn too, from `EstateLayout` rather than from the model table:
  `/api/world/estate/layout` serves the patch, every building footprint, every room and zone
  and every item at the metres it stands on, and `--view estate` opens on it. The Maer-Ken
  models hang on that geometry rather than standing in for it, so a missing `assets/` costs
  the decoration and not the estate. CI photographs that frame as well as the island's, and
  reads the log for models the asset server could not find -- measured, an empty `assets/`
  takes the frame from 3,457 colours to 2,172 and names thirteen missing files, and only the
  log half of the check fails it.

  Still open from that phase's plan: individual stems rendered in 3D (the web map draws them;
  the desktop application does not yet), the human inspector's full detail sections, the
  foundry panel, the resolution gate and its benchmark, `island_bench`, the allocation-bound
  test, retiring `apps/island_humans`, and the planetary-only pruning. None of those is
  blocked; none is done.
- Owner reviews: Gate 1 (island) and Gate 2 (weather previews).
- Upstream pull requests for every divergence in `UPSTREAM.md`.
