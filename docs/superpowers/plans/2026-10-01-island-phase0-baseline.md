# Island Phase 0: Green Baseline Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the imported workspace green, fast to test and CI-gated before any island work starts, so every later phase builds on a verified baseline.

**Architecture:** No new runtime features. Fix inherited defects with the smallest correct change, separate slow tests from the fast tier, pin the toolchain, and add CI. Fixes to imported upstream code are recorded as island divergences in `UPSTREAM.md` and proposed upstream.

**Tech Stack:** Rust 1.97.0 (pinned), GitHub Actions.

**Spec:** `docs/superpowers/specs/2026-10-01-maer-ken-island-regional-world-design.md` §17 step 1.

## Baseline as audited (2026-10-02)

- **5 failing tests.**
  - `mk_core` lib: `flux::tests::reservoir_all` asserts 31 reservoirs; the enum and `Reservoir::all()` both have 32 (`SurfaceWater` was added without updating the test).
  - `mk_engine` lib: `audit_entries_fingerprint_the_state_each_step_produced` and `reflected_sunlight_never_enters_the_planet`; `mk_engine` integration: `phase4_phase5_basic::test_phase5_basic_functionality` (`tests/phase4_phase5_basic.rs:59`) and `phase4_phase5_integration::test_phase5_world_integration` (`:91`). One cause: `step_ledger_clear` clears `audit_trail.entries` after `step_audit`, so per-step entries are never observable after `step_world` returns. `step_world` already clears them at the start of each tick, so the second clear prevents no leak. Removing it makes all 508 lib tests pass (verified).
- **Slow tests (debug).** `benchmark_verification::test_verification_horizon_1kyr_benchmark` > 12 min; `phase7_artifacts_emit` (emits a Kyr100 artifact) > 5 min unfinished; `phase4_phase5_integration` 37 s; `humans_world_integration` 31 s; `conservation_audit` 13 s. Under 10 s: `audit`, `biodiversity_integration`, `closure_debug_test`, `insolation`, `orbit`, `phase6_verification` (its one non-ignored test). Not yet timed: `phase9_organisms_schemas`, `runtime_boundary`, `tides`, and the `mk_core`, `mk_interventions`, `island_humans` binaries. `debug_closure.rs` is a `fn main` with no tests. `phase6_verification.rs` already has 8 `#[ignore]` tests that run 100-kyr horizons ("tens of millions of daily ticks") and cannot be part of any routine tier.
- **No CI, no pinned toolchain.** Clippy reports 4 warnings (2 dead constants, 1 dead fn, 1 unused variable) plus a manual range check; `rustfmt` is clean.
- **Storage key.** `EncryptionManager::decode_hex` panics on odd-length or non-ASCII `MK_STORAGE_KEY` (byte-index slicing); with `panic = "abort"` this aborts. The generated key file is written with default permissions, then chmodded to `0600`. `HumanStorage::new` swallows any key error (`EncryptionManager::init(..).ok()`, `human_storage.rs:78`) and then writes sensitive files in plaintext (`:122`) — a silent security downgrade.
- **Bridge retries.** `HttpComputerBridge::with_retry` wraps `SendEmail` (`computer_bridge.rs:263`), so a timeout can send duplicates, and a final timeout is reported as `RateLimitError` (`:212`). Each retry sleeps 200/400/800 ms.
- **Disk changes who exists.** `HumanRegistry::add_human`, `create_human` and `create_named_human` write the folder *before* adding the human and return early on any storage error (including `AlreadyExists`). `deliver_due_births` consumes the pregnancy, calls `add_human` and ignores the error, so the child vanishes (`lifecycle.rs:1030`). Newborn ids are deterministic (`born_t{tick}_{hash}`), so re-running a seed into a data directory with an earlier run's folders drops those children. Upstream `spawn_human` maps the same error to a refusal (`interventions/mod.rs:1423-1427`). `sync_to_storage`, `sync_human_to_storage` and `record_event` discard errors (`let _ =`).

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

- [ ] **Step 1:** Run `cargo test -p mk_engine --lib founders_estate_placement_tests` and `cargo test -p mk_engine --test phase4_phase5_basic --test phase4_phase5_integration`; expect 4 FAIL.
- [ ] **Step 2:** Remove the `audit_trail.entries.clear()` from `step_ledger_clear`; the clear at the start of `step_world` keeps the trail bounded to one tick.
- [ ] **Step 3:** Add a test asserting the trail after `step_world` holds exactly one tick's entries (bounded across 10 steps).
- [ ] **Step 4:** Re-run Step 1's commands and `cargo test -p mk_engine --lib`; expect 0 failures.
- [ ] **Step 5:** Snapshots now carry one tick of audit entries, so snapshot digests change. Search tests and fixtures for hard-coded digests (`grep -rnE '[0-9a-f]{64}' crates fixtures`) and confirm none depend on the old payload.
- [ ] **Step 6:** Commit `fix(engine): keep the last tick's audit entries observable`.

### Task 3: Toolchain pin and test tiers

**Files:**
- Create: `rust-toolchain.toml` (`channel = "1.97.0"`, `components = ["rustfmt", "clippy"]`)
- Modify: slow tests under `crates/mk_engine/tests/`
- Modify: `crates/mk_engine/tests/phase6_verification.rs` (re-label existing ignores)
- Modify: `Cargo.toml` (profiles)
- Create: `docs/TESTING.md`

**Tiers:**
- **Fast:** `cargo test --workspace` — every test under ~30 s debug. Every push.
- **Slow:** `#[ignore = "slow: <reason>"]` plus a `slow_` name prefix, run by `cargo test --workspace --release -- --ignored slow_` (libtest filters by test name, not by ignore reason). Nightly.
- **Deep:** `#[ignore = "deep: <reason>"]` plus a `deep_` name prefix — the 8 existing 100-kyr `phase6_verification` tests and `phase7_artifacts_emit`'s Kyr100 emission. Never scheduled; run manually with `--ignored deep_`.

- [ ] **Step 1:** Add `rust-toolchain.toml`.
- [ ] **Step 2:** Time each remaining untimed binary by running `cargo test -p <crate> --test <name>` (and each crate's `--lib`) under `time`; record all results in `docs/TESTING.md`.
- [ ] **Step 3:** Move every test over ~30 s debug to the slow tier; re-label `phase6_verification`'s ignores and `phase7_artifacts_emit`'s Kyr100 test as deep. Delete nothing.
- [ ] **Step 4:** Add to `Cargo.toml`:
  ```toml
  [profile.dev.package.mk_engine]
  opt-level = 2

  [profile.dev.package.mk_core]
  opt-level = 2

  [profile.bench]
  lto = false
  codegen-units = 16
  ```
  (`--release` tests use the `bench` profile, which otherwise inherits the release `lto = true, codegen-units = 1` and makes every test binary slow to link.) Re-time the fast tier; record before/after.
- [ ] **Step 5:** Run `cargo test --workspace` (expect 0 failures and the recorded wall time) and `cargo test --workspace --release -- --ignored slow_` (expect 0 failures).
- [ ] **Step 6:** Commit `test: pin toolchain and split slow and deep test tiers`.

### Task 4: Storage key and bridge retries

**Files:**
- Modify: `crates/mk_engine/src/io/encryption.rs`
- Modify: `crates/mk_engine/src/io/human_storage.rs` (`HumanStorage::try_new`)
- Modify: `crates/mk_engine/src/humans/computer_bridge.rs`

- [ ] **Step 1:** Extract `parse_storage_key(hex: &str) -> Result<[u8; 32], EncryptionError>` and test it directly (no environment variables, which race between parallel tests): odd length, non-ASCII, non-hex and wrong length all return `InvalidKey`; a valid 64-char key parses.
- [ ] **Step 2:** Validate length and ASCII-hex before slicing. Create the key file with `OpenOptions::new().write(true).create_new(true).mode(0o600)` (import `std::os::unix::fs::OpenOptionsExt` under `#[cfg(unix)]`); on `AlreadyExists` (another process won the race) re-read the existing file. Add a regression test that a generated key file's mode is `0600` (it already ends up `0600` today; the test guards the new path).
- [ ] **Step 3:** Add `HumanStorage::try_new(base_path) -> Result<Self, HumanStorageError>` that propagates key errors. Keep `new` for upstream parity, but make it log the error. Island code (Phase 0b onward) uses `try_new` only. Test with an explicit-key constructor (no env var) that an invalid key fails rather than writing plaintext.
- [ ] **Step 4:** Extract `retry_with(policy: RetryPolicy, backoff: impl Fn(u32) -> Duration, attempt: impl FnMut() -> Result<T, BridgeError>)` with `RetryPolicy::{Idempotent, NonIdempotent}`. `WebSearch` is `Idempotent` (retries on timeout and 429); `SendEmail` is `NonIdempotent` (retries only on 429, which means the request was not accepted; never on timeout). The final attempt's real error is returned. `HttpComputerBridge` passes the real backoff; tests pass a zero backoff and a counting closure (`MockComputerBridge` cannot be used: it implements `ComputerBridge` directly and never calls the retry path).
- [ ] **Step 5:** Run `cargo test -p mk_engine --lib -- io:: humans::computer_bridge`; expect PASS.
- [ ] **Step 6:** Commit `fix(engine): harden storage key handling and bridge retries`.

### Task 4b: Who exists must never depend on disk

**Files:**
- Modify: `crates/mk_engine/src/humans/registry.rs` (`add_human`, `create_human`, `create_named_human`, `seed_founders`, `sync_to_storage`, `sync_human_to_storage`, `record_event`)
- Modify: `crates/mk_engine/src/humans/lifecycle.rs` (`deliver_due_births`)
- Modify: `crates/mk_engine/src/humans/mod.rs` (`with_persistent_founders` and other callers)
- Modify: `crates/mk_engine/src/interventions/mod.rs` (`spawn_human`)
- Test: beside each

**Interfaces:**
- `add_human`, `create_human`, `create_named_human`: reject a duplicate agent id with `HumanStorageError::DuplicateAgent` *before* inserting; otherwise insert the human first, then write the folder, returning `Err(storage error)` meaning "added; storage failed".
- `seed_founders` / `with_persistent_founders`: founders always exist; storage errors are returned after insertion.
- `deliver_due_births`: every child is added; birth/`reproduced` events are written when storage works; storage errors are returned in a `Vec<HumanStorageError>` for the caller to log and count.
- `spawn_human`: succeeds when the human was added, and includes any storage error in its `AppliedOutcome` message as a warning instead of refusing.
- `sync_to_storage`, `sync_human_to_storage`, `record_event`: return their errors; callers that deliberately ignore them do so explicitly. Moving `sync_to_storage` off the per-step path is Phase 4 Task 3b, not this task.

- [ ] **Step 1:** Add failing tests with storage enabled in a **fresh temp directory** per test (`with_storage` reloads humans already on disk, so tests must not share one; Phase 4 Task 3b's per-run directories handle this in production): (a) a pre-existing folder with the newborn's id, (b) an unwritable storage root. In both, the child joins the registry and the error is returned. Same for `spawn_human` (success with a warning) and `seed_founders`.
- [ ] **Step 2:** Implement the interface changes and update every caller.
- [ ] **Step 3:** Add a test that the same seed stepped with storage enabled (fresh dir), disabled, and failing yields the same human registry hash.
- [ ] **Step 4:** Run `cargo test -p mk_engine --lib -- humans:: interventions::` and `cargo test -p mk_engine --test humans_world_integration`; expect PASS.
- [ ] **Step 5:** Commit `fix(humans): never lose a human because a folder write failed`.

### Task 5: CI gate

**Files:**
- Create: `.github/workflows/ci.yml`
- Modify: inherited clippy warnings

- [ ] **Step 1:** Resolve the existing clippy warnings without `allow` attributes unless the item is genuinely kept for upstream parity (document each such `allow`).
- [ ] **Step 2:** Add a push/PR workflow using the pinned toolchain (`rustup show` installs it from `rust-toolchain.toml`), cargo caching, and: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`.
- [ ] **Step 3:** Add a nightly scheduled job: `cargo test --workspace --release -- --ignored slow_`.
- [ ] **Step 4:** Push to a branch and confirm both jobs run green.
- [ ] **Step 5:** Commit `ci: gate fmt, clippy and tests`.

### Task 6: Upstream drift check

**Files:**
- Modify: `UPSTREAM.md`

- [ ] **Step 1:** Compare the pinned commit against current `dylmarriner/Maer-Ken` default branch for the imported paths (`crates/mk_core`, `crates/mk_engine`, `crates/mk_interventions`, `fixtures/human`, `docs/canon`, the Blender tool and GLBs). As of 2026-10-02 upstream HEAD equals the pin and the imported human modules are identical apart from one rustfmt-only hunk in `mk_core/src/human/profile.rs`; re-check immediately before Phase 1.
- [ ] **Step 2:** List any upstream changes to human, property, vegetation and resource modules, and whether upstream already fixed Tasks 1, 2, 4 or 4b.
- [ ] **Step 3:** Record the findings and the Phase-0 divergences in `UPSTREAM.md`; ask the owner to confirm keep-pin (program plan decision 2) or resync.
- [ ] **Step 4:** Commit `docs(upstream): record phase-0 divergences and drift`.

### Task 7: Gate 0

- [ ] **Step 1:** Run fmt, clippy `-D warnings`, the fast tier and the slow tier; expect all green.
- [ ] **Step 2:** Tick Gate 0 in the program plan with the recorded fast-tier wall time.
