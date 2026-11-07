# Handoff

## Latest engineering handoff — timber cross-cut reserved-ID integrity (8 November 2026)

- **PR #124** remains draft on `fix/timber-cross-cut-exact-conservation-20261108`. Use [PR #124](https://github.com/dylmarriner/Maer-ken-island/pull/124) for the exact live head SHA; this handoff deliberately avoids hard-coding a SHA that changes when the handoff itself is refreshed.
- `MaterialLedger::cross_cut_log` now requires the complete one-based destination range to have been reserved already: its exclusive end must be at or below the ledger's `next_log` watermark. It still rejects overflow and occupied IDs before removing the source.
- This closes a public-boundary hole: an unoccupied future range could previously be materialised without advancing the allocator, allowing a later felled log to reuse and overwrite one of those piece IDs.
- Regression `cross_cut_refuses_colliding_or_overflowing_piece_ids_without_mutating_the_ledger` proves an unreserved range 10–11 is rejected without changing the source, occupied log, carbon/water stocks or allocator; the same cut succeeds after IDs 3–4 are reserved.
- The existing IslandLife planner remains the single writer: it previews the next ID, calculates the exact final conserved piece count, reserves the range, and the finisher fails fast if materialisation is refused.
- Code/test commits in this run: `b75a25d9131872ea16a8af09c48dd76b2d18a832` (reserved-range guard) and `a0ca75f567fef99dfc051c94ed80273d43b1dbb8` (regression). Checklist and handoff commits are recorded in the PR history.
- The predecessor exact-head workflow [#42386637414](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42386637414) was still running when the branch handoff was refreshed. A fresh exact-head workflow is required for the current PR head; do not treat predecessor-head results as current verification. Dependency audit, dashboard, computer service and repository scripts had passed; desktop, formatting/Clippy/fast Rust tests and week/year acceptance were still in progress at the last observation. Nightly slow tests are skipped by design.
- **PR #123** remains a separate replay/sequence branch at `1378a345020601f3cbde85ee9db4f8a44822c16e`; exact-head workflow [#42374551377](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42374551377) passed its required checks. PRs #123 and #124 both touch `HANDOFF.md`, `WORK_REMAINING.md`, `life.rs` and `docs/ISLAND_SYSTEM_STATUS.md`; reconcile the overlapping status/documentation hunks before merging either branch.
- PRs #73–76 remain untouched. This is independent timber-ledger hardening, not a restart of the paused Maer-Ken Merge Completion job.
- Custom OpenCommander device discovery still returns HTTP 429; no local Cargo result is claimed. No snapshot schema, append-only log format, replay hash, RNG, simulation tick, LLM behaviour or production infrastructure changed.
- Full technical detail: [TIMBER_CROSS_CUT_MASS_CONSERVATION_20261108.md](docs/handoff/TIMBER_CROSS_CUT_MASS_CONSERVATION_20261108.md).

Updated 2026-11-08 against `main`: PR #120 merged as `66bf099f36c567ab344fcc9b268a9a33a656011d` and PR #121 merged as `522aba3d860435c92a4938cf5ffba2b427941cd7`; both exact-head workflows passed their required jobs. PRs #123 and #124 remain separate draft branches pending final integration and reconciliation. This is the front door for whoever picks the island up next, human or agent.

**What is left to do, and what each item needs, is in [`WORK_REMAINING.md`](WORK_REMAINING.md).**

## Previous engineering handoff — 7 November 2026

- **PR #118** is merged as `134082a1c1bbcea837acc58344572476e8b14f74`; its exact-head workflow passed the required Rust, desktop, backend, dashboard, computer-service, dependency and week/year acceptance checks.
- **PR #119** is merged as `9cc2b15808c57bd5f61d305bca6122e79820c6bc`; exact-head workflow [#42336338097](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42336338097) passed all required checks, including fast Rust tests, desktop and reference-pack acceptance.
- **PR #120** (`fix/fishery-unrepresentable-msy-diagnostics-20261107`) is open against `main` at head `6aa0d644ceacde3474ab8e37540882f2513900d8`. Its review threads are resolved. Exact-head workflow [#42346006514](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42346006514) has passed formatting, Clippy, installed backend, web dashboard, computer service, repository scripts, dependency audit and week/year reference-pack acceptance; fast Rust tests and desktop rendering were still running at the latest check. Do not merge until every required job is green.
- **PR #121** (`fix/fishery-biomass-increment-precision-20261107`) is deliberately stacked on PR #120. It adds `BiomassIncrementUnrepresentable` for positive unharvested growth that is stored as a materially different non-zero biomass delta, with 3→4 and 5→4 tonne regressions at `2^53`; the stock must remain unchanged on error. The first exact-head run caught one stale test reference to the renamed shared tolerance; commit `9ea6a478` fixed it. Current head `bd31d34b854eb76c4d75b990626d480c5b0bf55b` is under exact-head workflow [#42346953607](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42346953607). Do not merge until all required Rust, desktop/backend and reference-pack checks pass.
- Fishery remains standalone pending sourced species-specific stock/resilience and effort/CPUE data, IslandLife integration, and person-owned workforce decisions. No ecological parameters are invented.
- Replay PRs #73–76 were not modified; do not revive the paused Maer-Ken Merge Completion job. Reconcile that replay stack separately against current `main` and the merged digest correction when the replay work is explicitly resumed.

## Previous engineering handoff — 7 November 2026

- [Fishery biomass remainder precision](docs/handoff/FISHERY_BIOMASS_REMAINDER_PRECISION_20261107.md) — PR #111 merged to `main` as `b41bb5cf24bddca5df2bc556612c5edfc8b91bde`. Exact-head workflow [#42292739398](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42292739398) passed formatting, Clippy, fast Rust tests, desktop, installed backend, dashboard, computer service, repository scripts, dependency audit and week/year reference-pack acceptance; nightly slow tests were skipped by design.

- [Fishery landed-increment exactness](docs/handoff/FISHERY_LANDED_INCREMENT_EXACTNESS_20261107.md) — rebased follow-up PR #114. Cumulative landed mass rejects zero or materially distorted recorded deltas, with regressions for both rounded-up (3→4) and rounded-down (5→4) increments at 2^53, while allowing ordinary repeated binary64 rounding; snapshot layout is unchanged. Source/test commits `186766a19108338f0bd75a28dd00ef2bd1cc8b76` and `b6d6e11ccbbae169008c451e955258bc60d835f5`; fresh exact-head CI pending.

- [Fishery landed-total atomicity and precision](docs/handoff/FISHERY_LANDED_OVERFLOW_ATOMICITY_20261107.md) — PR #110 merged to `main` as `148ac98c95e58aac7b2065dd0cc68884790f4168`. Exact-head workflow [#42275024667](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42275024667) passed formatting, Clippy, fast Rust tests, desktop build/tests/rendering/input acceptance, installed backend, web dashboard, computer service, repository scripts, dependency audit and week/year reference-pack acceptance; nightly slow tests were skipped by design.

- [FishStock binary snapshot compatibility](docs/handoff/FISHERY_SNAPSHOT_BINARY_COMPATIBILITY_20261107.md) — merged in PR #109 to `main` as `0613dffca65a87e4db147300d9d57805a744ea07`. A fixed 28-byte bincode 1.3 fixture pins the legacy field order and encoding. Exact-head workflow [#42257092937](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42257092937) passed the required Rust, desktop, backend, dashboard, computer-service, dependency and week/year acceptance jobs; nightly slow tests were skipped by design.

- [Marine biomass cell inspector](https://github.com/dylmarriner/Maer-ken-island/commit/6294504efe9261b4794dd0778c96ba4712fdd141) — PR #93 merged into main as `6294504efe9261b4794dd0778c96ba4712fdd141`. Exact-head workflow [#42166273507](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42166273507) passed the browser DOM click regression, Rust formatting/Clippy/fast tests, desktop, installed backend, week/year reference acceptance, computer service, repository scripts and dependency audit; nightly slow tests were skipped by design. The two prior Codex/Kilo findings were fixed and their review threads resolved.
- [Deep-time geographic-range, logistic-growth and snapshot compatibility corrections](docs/handoff/DEEP_TIME_GEOGRAPHIC_RANGE_DRIFT_20261106.md) — PR #106 merged into main as `3057ea4274f8d6a131fa44a24380e0c7c145057a`. Exact-head workflow [#42169416524](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42169416524) passed formatting, Clippy, fast Rust tests, desktop, installed backend, web dashboard, computer service, repository scripts, dependency audit and week/year reference-pack acceptance; nightly slow tests were skipped by design.
- [Deep-time evolution atomicity](docs/handoff/DEEP_TIME_EVOLUTION_ATOMICITY_20261106.md) — PR #107 merged into main as `0ddbd878eca26e11c114e0c434653316f4d7da96`. Exact-head workflow [#42193855127](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42193855127) passed formatting, Clippy, fast Rust tests, week/year reference-pack acceptance, desktop build/tests/rendering, installed backend, web dashboard, computer service, repository scripts and dependency audit; nightly slow tests were skipped by design. The append-only extinction and speciation histories remain in place during each tick; full history validation remains at restore/audit boundaries, while the hot path validates mutable state only. The outstanding O(history)-per-tick review thread was resolved after the exact-head workflow passed.
- [Desktop marine inspector safety parity](docs/handoff/MARINE_CELL_INSPECTOR_DESKTOP_SAFETY_20261106.md) — PR #108 merged into main as `42bf9cc43c8024644a452a00127ed39b676d19ff`. Exact-head workflow [#42226945535](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42226945535) passed formatting, Clippy, fast Rust tests, desktop build/render/input acceptance, installed backend, week/year reference acceptance, web dashboard, computer service, repository scripts and dependency audit; nightly slow tests were skipped by design.
- [Reference range midpoint overflow, subnormal rounding and double-rounding hardening](docs/handoff/REFERENCE_RANGE_MIDPOINT_OVERFLOW_20261106.md) — PR #109, branch `fix/reference-range-midpoint-overflow-20261106`. The helper sums only when both magnitudes are <= `f64::MAX / 2`; otherwise it halves both endpoints before addition. This preserves subnormal ties, avoids overflow and avoids a one-ULP error from halving a rounded difference. Regressions cover extreme bounds, adjacent subnormals and the wide-gap case `3×2^969` to `2^1023`. Commits `2471472c` and `f07a3d29` implement the algorithm and tests; documentation follow-up `203b0dee` records the exact-head verification gate. Workflow #42205227479 on code/test head `f07a3d29` was cancelled after later documentation-only pushes superseded the in-progress run; its fast Rust tests, week acceptance, desktop and backend checks did not complete. A fresh exact-head run is required before merge.

- [Fishery full-harvest rounding](docs/handoff/FISHERY_FULL_HARVEST_ROUNDING_20261107.md) — PR #109 follow-up. `FishStock::fish` now subtracts catch from interval growth before adding the remainder to biomass, preventing a fully harvested stock from appearing to lose biomass through floating-point cancellation. Regression uses a deterministic (2^{62})-tonne stock and asserts exact biomass preservation when catch equals the read-only interval limit. New exact-head CI is required; the preceding workflow #42213034085 passed all required jobs.




- [Snapshot canon-digest integrity](docs/handoff/SNAPSHOT_CANON_DIGEST_INTEGRITY_20261106.md) — PR #106 follow-up. Both the replay snapshot loader and file snapshot loader now compare the persisted `canon_derived.canon_digest` with a fresh digest derived from the validated `CanonLocked` before exposing restored state. A valid outer BLAKE3 payload digest no longer masks an internally inconsistent canon identity. Mismatches fail closed; derived values are recomputed only after the identity check. Regression tests cover both restore paths. Replay restore also requires the unhashed envelope tick to match the digested world-state tick. Schema-0 compatibility also rejects mixed fractional ranges when speciation history proves the old drift-before-speciation ordering may have erased parent fractions. Workflow #42169111579 reached the new code and failed only on three rustfmt line wraps in the mixed-state tests; the exact formatter output is applied in the next commit, with fresh exact-head CI required. Exact-head workflow #42167966575 caught three rustfmt-only line wraps in the new file-loader tests; commit `e2f38185` applies the formatter output. Workflow #42168311597 then passed formatting but Clippy compilation caught two test-only E0382 partial moves in error assertions; both now borrow the message with `ref`. Fresh exact-head CI is required.

- [Fishery interval sustainable-catch limit](docs/handoff/FISHERY_SUSTAINABLE_CATCH_LIMIT_20261106.md) — PR #105 merged into main as `5a0410f87a9a1b947c1bf481dc1f83202573b17b`. Exact-head workflow [#42146857592](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42146857592) passed formatting, Clippy, fast Rust tests, desktop build/tests/rendering, installed backend, week/year reference acceptance, dashboard, computer service, repository scripts and dependency audits; nightly slow tests were skipped by design. The read-only interval limit shares the exact growth-cap helper enforced by `FishStock::fish`.


- [Deep-time geographic-range, logistic-growth and snapshot compatibility corrections](docs/handoff/DEEP_TIME_GEOGRAPHIC_RANGE_DRIFT_20261106.md) — PR #106, rebased after PR #105. Preserves fractional ranges, uses stable decaying-exponential logistic growth, caps all 4,096 founding ranges at one planetary surface, and preflights clock/RNG overflow. The measured ten-step hash `66d0b56023d72ff7079eb74499820dbfd31a0f44ecbddbd69e493fa320e0bdfe` is pinned with its reason. A new `WorldState` snapshot schema version (1) now validates both replay and file-based loads: valid schema-0 snapshots upgrade metadata only; legacy snapshots with invalid ranges and future schemas fail at load. No lossy clamp is applied because the old drift could destroy the original fractional values. Exact-head CI is required for the compatibility tests.

- [Marine catalogue value validation](docs/handoff/MARINE_CATALOGUE_VALUE_VALIDATION_20261106.md) — PR #104 merged into main as `14611890766631c83a51713d0a2aca77d1e4c5c3` after exact-head workflow #42141381773 passed formatting, Clippy, fast Rust tests, desktop/backend builds, week/year reference acceptance, dashboard, computer service, repository scripts and dependency audit. Nightly slow tests were skipped by design.
- [Marine species stack reconciliation](docs/handoff/MARINE_SPECIES_STACK_RECONCILIATION_20261106.md) — PR #102 merged into main as `2f11b5e9a3cd84a35f6cb69e1966cd3cd72391ed`. Exact-head workflow [#42133095121](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42133095121) passed formatting, Clippy, fast tests, desktop/backend, week/year reference acceptance, dashboard, computer service, scripts and dependency audit; nightly slow tests were skipped by design. No local Cargo result is claimed.

- [Fishery MSY biomass target](docs/handoff/FISHERY_MSY_BIOMASS_TARGET_20261106.md) — PR #101. Adds a read-only `msy_biomass_t` diagnostic derived from the same piecewise peak as MSY, with boundary, invalid-state and extreme-capacity regressions.
- Follow-up commit `e356c66b` corrects the extreme-capacity target expectation in that regression; the prior workflow head predates this correction, so only the new exact-head run can verify it.
- PR #100 (piecewise MSY) was merged into main at `55851f7a10d6ab3653b5ef013e6eba8de8e13c52`. Exact-head workflow [#42117060148](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42117060148) passed every required job, including fast Rust tests, desktop build/tests/rendering, installed backend, week/year reference-pack acceptance, dashboard, computer service, scripts and dependency audit. The nightly slow tier was skipped by design.
- PR #97's subnormal-safe surplus correction remains in main at `1d70e9e2941675aef4d10fb8f94816cfe4610c70`.
- PR #101 (MSY biomass target) merged into main as `373a1b71517b82352b40208e4546bc0d3e4d24a1`. Exact-head workflow [#42117060148](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42117060148) passed required fast Rust, desktop/backend, week/year reference-pack, dashboard, computer service, scripts and dependency-audit checks; nightly slow tests were skipped by design. Custom OpenCommander returned HTTP 429, so no local Cargo test is claimed.
- Fishery remains standalone until sourced species-specific stock and effort data are available; no carrying capacities or CPUE values were invented.

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
The session of 2026-10-14 (three sessions in parallel; the open PRs, their merge order and the
owner's decisions) is recorded in `docs/handoff/SESSION_2026-10-14.md`.

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
  (`29ed53bd7769eb3025e70135bab8b7ea07718e26e7fca4cbf77ab379d62b7784`) pins one simulated week. Any change to what the founders or their world do
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


## Latest engineering handoff

- 2026-11-06: [Fishery subnormal biomass-ratio hardening](docs/handoff/FISHERY_SUBNORMAL_BIOMASS_RATIO_20261106.md) records the low-biomass underflow correction and exact-head verification gate.
- 2026-11-05: [Replay session default execution budget](docs/handoff/REPLAY_SESSION_DEFAULT_BUDGET_20261105.md) records the bounded default and exact-head verification gate.


- [Fishery subnormal MSY quarter-scaling correction](https://github.com/dylmarriner/Maer-ken-island/commit/93c4b040c050d708dafbb350cd2ca3c00e46cc9b) — PR #109 follow-up. For the exact 1/4 peak fraction, scale the rate before multiplying the smallest subnormal carrying capacity so the final operation performs the rounding. Regression distinguishes 22,149 from the prior 22,150 subnormal units. Source/test commit `93c4b040`; a fresh exact-head workflow is required, no pass claimed yet.
- [Fishery MSY subnormal preservation](docs/handoff/FISHERY_SNAPSHOT_BINARY_COMPATIBILITY_20261107.md) — PR #109 follow-up. `msy_t` now avoids losing representable subnormal results when applying the piecewise peak fraction to carrying capacity. Regression covers 1, 3 and 7 subnormal units at `r = 4`. Exact-head CI is required for this follow-up; the preceding legacy-snapshot workflow #42237465516 passed all required jobs.

## 2026-11-07 fishery residual-growth regression follow-up

- Exact-head workflow [#42285280868](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42285280868) passed formatting, Clippy, desktop, installed backend, dashboard, computer service, dependency audit and reference acceptance. Fast Rust tests reported 855 passed and one failure: the depensation-underflow baseline requested a partial catch whose remaining growth cannot be represented at the existing biomass scale.
- The production guard correctly rejected that partial harvest before mutation. The baseline now requests the full representable surplus; the dedicated partial-harvest regression continues to assert fail-closed atomicity.
- Test commit: `6704c1180ffbbe9ef3220a23c706dd18fe1c6e23`.
- Fresh exact-head workflow [#42292221961](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42292221961) was still running at the latest inspection.

Remaining: complete all required jobs, review the exact-head test result, then merge PR #111 only if green. Reconcile this fishery file with PR #112 after one of the two is merged; do not merge stale overlapping branches.
