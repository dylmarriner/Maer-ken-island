# Testing

The toolchain is pinned in `rust-toolchain.toml` (Rust 1.97.0 with `rustfmt` and `clippy`).

## Tiers

| Tier | Marker | Command | When |
|------|--------|---------|------|
| Fast | none | `cargo test --workspace --exclude island_ui` | every push (CI's `fmt, clippy, fast tests` job, about 28 minutes); every test is under ~30 s in debug |
| Desktop | none | `cargo test -p island_ui` | every push, in CI's `desktop application` job (needs ALSA and graphics headers) |
| Slow | `#[ignore = "slow: <reason>"]` and a `slow_` name prefix | `cargo test --workspace --exclude island_ui --release -- --ignored slow_` | nightly (CI's `slow` job, 03:17 UTC) and before merging behaviour changes |
| Deep | `#[ignore = "deep: <reason>"]` and a `deep_` name prefix | `cargo test --release -p mk_engine --test <binary> -- --ignored deep_` | manual only, never scheduled |

`--ignored deep_` is a substring filter, so a slow test's name must not contain `deep_`. libtest filters by test name, not by ignore reason, which is why the name prefix is required in addition to the `#[ignore]` reason. Tests are never deleted to make a tier faster; they move tier.

`island_ui` is excluded from the fast and slow commands because it pulls in Bevy, which needs
system audio and graphics headers a plain container lacks; it has its own CI job. The nightly
`slow` job in `.github/workflows/ci.yml` currently runs `--workspace` **without**
`--exclude island_ui`; if it fails while building, that is why.

Current members (find them with `grep -rn 'fn slow_\|fn deep_' --include=*.rs crates apps`):

- **Slow:** fifteen tests.
  - `island_life_acceptance::slow_a_week_on_the_island_matches_the_reference_packs`: one
    simulated week, audited every step, against the reference packs, and the canonical week
    digest (`WEEK_DIGEST`). **Run it on every behaviour change.**
  - `island_life_acceptance::slow_humans_choose_intimacy_about_as_often_as_couples_do`.
  - `regional_physical_coupling::slow_a_year_of_island_weather_matches_the_reference_packs`:
    a simulated year of weather against the climate packs. **Run it on every climate change.**
  - `island_snapshot::slow_a_full_island_survives_a_trip_through_a_file`: the full island,
    200,000 stems and a dozen 1,152,000-cell grids, through a file and run on from it.
  - `island_scheduler::slow_a_day_in_one_step_equals_1440_minute_steps`.
  - `island_phase4_cost::slow_what_phase_4_costs`.
  - `regional_seismicity::slow_ten_thousand_years_of_seismicity_on_the_full_island`.
  - `human_population_memory::slow_the_human_runtime_stays_small_and_roughly_linear_to_five_thousand_people`
    and `::slow_formal_predictive_processing_costs_this_share_of_a_human_step`.
  - `human_scaling::slow_a_crowded_cell_costs_at_most_three_times_a_spread_population`.
  - `conversation_rate::slow_how_long_a_lifetime_of_remembered_conversation_actually_lasts`.
  - `phase4_phase5_integration::slow_test_phase4_long_horizon_evolution` (~35 s in debug).
  - In `apps/island`: `serve::sim::timing::slow_what_a_step_and_a_digest_cost`,
    `conversation_cost::slow_publishing_conversations_costs_this_share_of_a_step` and
    `tree_cost::slow_republishing_the_stems_costs_this_share_of_a_step`.
- **Deep:** ten tests. `benchmark_verification::deep_test_verification_horizon_1kyr_benchmark` (a 1,000-year whole-planet run: >12 min in debug and still unfinished after 1 h 45 min in release, so it cannot fit the nightly 120-minute budget; moved here from the slow tier, not removed), the eight 100-kyr tests in `phase6_verification.rs` (`deep_test_*`, tens of millions of daily ticks) and `phase7_artifacts_emit::deep_emit_phase7_artifacts_writes_files_and_canon_digest_matches_core` (emits a Kyr100 artifact, >5 min in debug).

## Pinned digests

| Pin | Test | Moves when |
|---|---|---|
| `WEEK_DIGEST` (`3d268e8b…4338`) | `island_life_acceptance` (slow) | anything the founders or their world do changes. Re-pin only after the week's audit and reference-pack checks pass (`docs/handoff/WORKING_PRACTICES.md`). |
| Planetary topology (`766d5cd8…d497`) | `topology_hash_baseline` (fast) | never, from island work. |

## Checking the dashboard by hand

`cargo test -p island` covers the pages, the API, the hardening headers and the
per-person sections, and `apps/island/tests/serve.rs` drives a real listener on
a real port. What no test can judge is whether the pages are worth looking at,
so before changing them, run the thing and read it:

    cargo run -p island -- serve --data-dir /tmp/island-check

To check it with a world behind it — the clock, the founders, the estate's
power — give it a scenario and a speed worth watching:

    cargo run -p island -- serve --data-dir /tmp/island-check \
      --scenario fixtures/island/default_scenario.json --speed 20000

The island takes about half a minute to bootstrap before the first step, and
hashes itself once more at startup, so give it a minute before judging
whether the clock is moving.

Then open the overview, the roster and the creator; create someone; and check
the three things a browser decides rather than a test:

- the browser console stays empty (the content security policy allows nothing
  from outside the server, so a stray inline style or external script shows up
  here),
- nothing scrolls sideways at a phone width of 390 px,
- the dark theme is legible (switch the OS or browser to dark and reload),
- that a person created from the creator page lands in the room you picked
  and turns up in the world panel *with body carbon* — without that they are
  drawn on the page but outside the material flows, which looks identical
  until you read the line,
- and, with a world, that the page's own claims are still true of it. The
  overview says "Time is not running" when there is no world and "Time is
  running" when there is; a page that contradicts the panel below it is a
  worse bug than a layout one, and only reading it catches that.

## Build profiles

`Cargo.toml` compiles `mk_core`, `mk_engine` and `mk_island` at `opt-level = 2` in the dev profile, together with what the island tests spend their time in besides the engine (`serde`, `serde_json`, `ryu`, `itoa`, `bincode`, `blake3`, `flate2`, `miniz_oxide`, `crc32fast`, `rand_chacha`); only those, so the desktop job's Bevy build is unchanged. It sets the `bench` profile (used by `cargo test --release`) to `lto = false, codegen-units = 16` so test binaries link quickly.

## Measured times (debug, this container, after the first profile change, 2026-10-02; kept for comparison)

| Binary | Wall time |
|--------|-----------|
| `mk_engine` lib (509 tests) | 25 s |
| `phase4_phase5_integration` (fast tests only) | 16 s |
| `humans_world_integration` | 20 s |
| `conservation_audit` | 7 s |
| `closure_debug_test` | 3 s |
| `phase4_phase5_basic` | 3 s |
| `audit` | 1 s |
| `insolation`, `tides` | 1 s |
| `biodiversity_integration`, `orbit`, `phase6_verification`, `phase9_organisms_schemas`, `runtime_boundary` | under 1 s |
| `mk_core` (all binaries) | under 1 s of test time |
| `mk_interventions`, `island_humans` | under 1 s of test time |

Times exclude compilation; a cold `cargo test --workspace --no-run` took about 5.5 minutes. `debug_closure.rs` is a `fn main` with no tests.

Before the profile change `phase4_phase5_integration` took 37 s, `humans_world_integration` 31 s and `conservation_audit` 13 s.

Fast tier (`cargo test --workspace`, warm build, 2026-10-02): 81 s wall, 0 failures. The island has grown a great deal since; CI's fast job now takes about 28 minutes from a cache.
