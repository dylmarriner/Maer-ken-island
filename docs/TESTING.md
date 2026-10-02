# Testing

The toolchain is pinned in `rust-toolchain.toml` (Rust 1.97.0 with `rustfmt` and `clippy`).

## Tiers

| Tier | Marker | Command | When |
|------|--------|---------|------|
| Fast | none | `cargo test --workspace` | every push; every test is under ~30 s in debug |
| Slow | `#[ignore = "slow: <reason>"]` and a `slow_` name prefix | `cargo test --workspace --release -- --ignored slow_` | nightly |
| Deep | `#[ignore = "deep: <reason>"]` and a `deep_` name prefix | `cargo test --release -p mk_engine --test <binary> -- --ignored deep_` | manual only, never scheduled |

libtest filters by test name, not by ignore reason, which is why the name prefix is required in addition to the `#[ignore]` reason. Tests are never deleted to make a tier faster; they move tier.

Current members:

- **Slow:** `benchmark_verification::slow_test_verification_horizon_1kyr_benchmark` (>12 min in debug), `phase4_phase5_integration::slow_test_phase4_deep_time_evolution` (~35 s in debug).
- **Deep:** the eight 100-kyr tests in `phase6_verification.rs` (`deep_test_*`, tens of millions of daily ticks) and `phase7_artifacts_emit::deep_emit_phase7_artifacts_writes_files_and_canon_digest_matches_core` (emits a Kyr100 artifact, >5 min in debug).

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
