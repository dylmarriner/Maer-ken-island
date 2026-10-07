# Testing

The toolchain is pinned in `rust-toolchain.toml` (Rust 1.97.0 with `rustfmt` and `clippy`).

## Tiers

| Tier | Marker | Command | When |
|------|--------|---------|------|
| Fast | none | `cargo test --workspace` | every push; every test is under ~30 s in debug |
| Slow | `#[ignore = "slow: <reason>"]` and a `slow_` name prefix | `cargo test --workspace --release -- --ignored slow_` | nightly |
| Deep | `#[ignore = "deep: <reason>"]` and a `deep_` name prefix | `cargo test --release -p mk_engine --test <binary> -- --ignored deep_` | manual only, never scheduled |

`--ignored deep_` is a substring filter, so a slow test's name must not contain `deep_`. libtest filters by test name, not by ignore reason, which is why the name prefix is required in addition to the `#[ignore]` reason. Tests are never deleted to make a tier faster; they move tier.

Current members:

- **Slow:** nine tests, found with `grep -rn 'fn slow_' --include=*.rs`:
  `island_life_acceptance::slow_a_week_on_the_island_matches_the_reference_packs` (one simulated
  week, and the canonical week digest),
  `island_snapshot::slow_a_full_island_survives_a_trip_through_a_file` (the full island — 200,000
  stems and a dozen 1,152,000-cell grids — written to a file and run on from it),
  `island_scheduler::slow_a_day_in_one_step_equals_1440_minute_steps`,
  `regional_physical_coupling::slow_a_year_of_island_weather_matches_the_reference_packs`,
  `regional_seismicity::slow_ten_thousand_years_of_seismicity_on_the_full_island`,
  `human_population_memory::slow_the_human_runtime_stays_small_and_roughly_linear_to_five_thousand_people`,
  `human_population_memory::slow_formal_predictive_processing_costs_this_share_of_a_human_step`,
  `human_scaling::slow_a_crowded_cell_costs_at_most_three_times_a_spread_population` and
  `phase4_phase5_integration::slow_test_phase4_long_horizon_evolution` (~35 s in debug).
- **Deep:** `benchmark_verification::deep_test_verification_horizon_1kyr_benchmark` (a 1,000-year whole-planet run: >12 min in debug and still unfinished after 1 h 45 min in release, so it cannot fit the nightly 120-minute budget; moved here from the slow tier, not removed), the eight 100-kyr tests in `phase6_verification.rs` (`deep_test_*`, tens of millions of daily ticks) and `phase7_artifacts_emit::deep_emit_phase7_artifacts_writes_files_and_canon_digest_matches_core` (emits a Kyr100 artifact, >5 min in debug).

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
- and, with a world, that the page's own claims are still true of it. The
  overview says "Time is not running" when there is no world and "Time is
  running" when there is; a page that contradicts the panel below it is a
  worse bug than a layout one, and only reading it catches that.

## Build profiles

`Cargo.toml` compiles `mk_core` and `mk_engine` at `opt-level = 2` in the dev profile, and sets the `bench` profile (used by `cargo test --release`) to `lto = false, codegen-units = 16` so test binaries link quickly.

## Measured times (debug, this container, after the profile change)

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

Fast tier (`cargo test --workspace`, warm build): 81 s wall, 0 failures.
