# Island Phase 0: Green Baseline Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the imported workspace green, fast to test and CI-gated before any island work starts, so every later phase builds on a verified baseline.

**Architecture:** No new runtime features. Fix inherited defects with the smallest correct change, separate slow tests from the fast tier, and add CI. Fixes to imported upstream code are recorded as island divergences in `UPSTREAM.md` and proposed upstream.

**Tech Stack:** Rust 2021 workspace, GitHub Actions.

**Spec:** `docs/superpowers/specs/2026-10-01-maer-ken-island-regional-world-design.md` §17 step 1 ("preserve its passing human/core runtime baseline").

## Baseline as audited (2026-10-02)

- `mk_core` lib: 1 failure — `flux::tests::reservoir_all` asserts 31 reservoirs; the enum and `Reservoir::all()` both have 32 (`SurfaceWater` was added without updating the test).
- `mk_engine` lib: 2 failures — `audit_entries_fingerprint_the_state_each_step_produced` and `reflected_sunlight_never_enters_the_planet`. Cause: `step_ledger_clear` clears `audit_trail.entries` after `step_audit`, so per-step entries are never observable after `step_world` returns. `step_world` already clears them at the start of each tick, so the second clear prevents no leak. Removing it makes all 508 lib tests pass.
- `benchmark_verification::test_verification_horizon_1kyr_benchmark` runs for more than 12 minutes in a debug build; other long-horizon integration tests have not been timed.
- No CI. Clippy reports a handful of warnings (dead code, an unused variable, a manual range check).
- `EncryptionManager::decode_hex` panics on odd-length or non-ASCII `MK_STORAGE_KEY` (byte-index slicing); with `panic = "abort"` this aborts the process.
- The generated storage key file is written with default permissions, then chmodded to `0600`.
- `HttpComputerBridge::with_retry` retries `SendEmail` on timeout (can send duplicates) and reports `RateLimitError` for a final timeout.

## Review Focus

- Fixes must not change deterministic simulation results except where a test proves the old behaviour was wrong.
- No test may be deleted, weakened or skipped to get green; slow tests move tiers, they are not removed.
- Every imported-code fix is listed in `UPSTREAM.md` as a divergence.

---

### Task 1: Correct the stale reservoir count

**Files:**
- Modify: `crates/mk_core/src/flux/mod.rs` (test `reservoir_all`)

- [ ] **Step 1:** Run `cargo test -p mk_core --lib flux::tests::reservoir_all`; expect FAIL (`left: 32, right: 31`).
- [ ] **Step 2:** Replace the hard-coded count with an exhaustive `match` over `Reservoir` that fails to compile when a variant is added, and assert `all()` contains every variant exactly once. Fix the stale comment.
- [ ] **Step 3:** Re-run; expect PASS.
- [ ] **Step 4:** Commit `test(core): make reservoir coverage exhaustive`.

### Task 2: Keep per-step audit entries observable

**Files:**
- Modify: `crates/mk_engine/src/world_integration.rs` (`step_ledger_clear`)

- [ ] **Step 1:** Run `cargo test -p mk_engine --lib founders_estate_placement_tests`; expect 2 FAIL ("ocean entry", "insolation entry").
- [ ] **Step 2:** Remove the `audit_trail.entries.clear()` from `step_ledger_clear`; the clear at the start of `step_world` keeps the trail bounded to one tick.
- [ ] **Step 3:** Add a test asserting the trail after `step_world` holds exactly one tick's entries (bounded across 10 steps).
- [ ] **Step 4:** Run `cargo test -p mk_engine --lib`; expect 0 failures.
- [ ] **Step 5:** Commit `fix(engine): keep the last tick's audit entries observable`.

### Task 3: Split fast and slow test tiers

**Files:**
- Modify: slow tests under `crates/mk_engine/tests/` (at minimum `benchmark_verification.rs`)
- Modify: `Cargo.toml` (dev profile overrides)
- Create: `docs/TESTING.md`

- [ ] **Step 1:** Time each test binary by running `cargo test -p <crate> --test <name>` (and each crate's `--lib`) under `time`; record results in `docs/TESTING.md`.
- [ ] **Step 2:** Mark every test over ~30 s debug with `#[ignore = "slow: <reason>; run with --release -- --ignored"]`.
- [ ] **Step 3:** Add `[profile.dev.package.mk_engine] opt-level = 2` and `[profile.dev.package.mk_core] opt-level = 2`; re-time the fast tier and record before/after.
- [ ] **Step 4:** Run `cargo test --workspace` (expect 0 failures, documented wall time) and `cargo test --workspace --release -- --ignored` (expect 0 failures).
- [ ] **Step 5:** Commit `test: split slow long-horizon tests into an ignored tier`.

### Task 4: Fix inherited defects

**Files:**
- Modify: `crates/mk_engine/src/io/encryption.rs`
- Modify: `crates/mk_engine/src/humans/computer_bridge.rs`

- [ ] **Step 1:** Add failing tests: odd-length key, non-ASCII key and non-hex key return `InvalidKey` (no panic); a freshly generated key file is never readable by group/other.
- [ ] **Step 2:** Validate key length and ASCII-hex before slicing; create the key file with `OpenOptions::new().write(true).create_new(true).mode(0o600)` on unix.
- [ ] **Step 3:** Add tests with `MockComputerBridge`-style counting that `SendEmail` is attempted once on timeout while idempotent `WebSearch` still retries, and that a final timeout reports `TimeoutError`.
- [ ] **Step 4:** Restrict retries to idempotent requests and preserve the real last error.
- [ ] **Step 5:** Run `cargo test -p mk_engine --lib io:: humans::computer_bridge`; expect PASS.
- [ ] **Step 6:** Commit `fix(engine): harden storage key handling and bridge retries`.

### Task 5: CI gate

**Files:**
- Create: `.github/workflows/ci.yml`
- Modify: inherited clippy warnings (dead code, unused variable, manual range)

- [ ] **Step 1:** Resolve the existing clippy warnings without `allow` attributes unless the item is genuinely kept for upstream parity (document each such `allow`).
- [ ] **Step 2:** Add a push/PR workflow: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, with cargo caching.
- [ ] **Step 3:** Add a nightly scheduled job: `cargo test --workspace --release -- --ignored`.
- [ ] **Step 4:** Push to a branch and confirm both jobs run green.
- [ ] **Step 5:** Commit `ci: gate fmt, clippy and tests`.

### Task 6: Upstream drift check

**Files:**
- Modify: `UPSTREAM.md`

- [ ] **Step 1:** Compare the pinned commit against current `dylmarriner/Maer-Ken` default branch for the imported paths (`crates/mk_core`, `crates/mk_engine`, `crates/mk_interventions`, `fixtures/human`, `docs/canon`, the Blender tool and GLBs).
- [ ] **Step 2:** List upstream changes to human, property, vegetation and resource modules, and whether upstream already fixed Tasks 1, 2 and 4.
- [ ] **Step 3:** Record the findings and the Phase-0 divergences in `UPSTREAM.md`; recommend resync-now or keep-pin (program plan Owner decision 2).
- [ ] **Step 4:** Commit `docs(upstream): record phase-0 divergences and drift`.

### Task 7: Gate 0

- [ ] **Step 1:** Run fmt, clippy `-D warnings`, the fast tier and the slow tier; expect all green.
- [ ] **Step 2:** Tick Gate 0 in the program plan with the recorded fast-tier wall time.
