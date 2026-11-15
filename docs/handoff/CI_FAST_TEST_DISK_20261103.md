# Maer-Ken Island engineering handoff: hosted Rust test disk budget (2026-11-03)

## Canonical scope
Repository remote: `dylmarriner/Maer-ken-island`, draft PR #68 (`fix/island-client-human-id-20261101`). This is distinct from Maer-Ken. `WORK_REMAINING.md`, `docs/IMPLEMENTATION_READY_ROADMAP.md`, `HANDOFF.md` and `docs/island/DEVIATIONS.md` govern remaining scope.

## Failure reproduced from hosted CI
Exact-head run https://github.com/dylmarriner/Maer-ken-island/actions/runs/41495488320 failed in `fmt, clippy, fast tests` during `cargo test --workspace --exclude island_ui`: `rust-lld` SIGBUS and `No space left on device` while linking test binaries. Other seven non-nightly lanes succeeded or were skipped by design. This was a runner storage failure, not evidence of failed simulation assertions.

## Implemented
- CI Rust check job sets `CARGO_PROFILE_TEST_DEBUG=0` and `CARGO_PROFILE_DEV_DEBUG=0` to avoid excessive debug-symbol artifacts; `CARGO_BUILD_JOBS=2` bounds concurrent linker disk spikes. Test selection, debug assertions, optimizations, deterministic engine and acceptance gates remain unchanged.
- New `scripts/tests/test_ci_disk_budget.py` regression is run by existing repository-scripts CI unittest discovery.

## Evidence
- RED: new Python resource-budget regression failed before workflow fix.
- GREEN: 9/9 repository script tests passed; 8/8 `mk_island_client` offline Rust tests passed with the resource env; `cargo fmt --all -- --check`, Python compilation and `git diff --check` passed.
- Full hosted Rust workspace CI on this exact commit remains unverified until it completes.

## Exact remaining checklist
- [ ] Confirm the hosted Rust lane completes without ENOSPC on the new SHA; if still failing, measure disk use and split compilation/test groups without dropping tests.
- [ ] Run capacity-safe week/year deterministic replay with matching hashes and water/carbon/material ledger acceptance; do not re-pin digests without the full audit.
- [ ] Advance sourced ecology, industry and workforce tasks from `WORK_REMAINING.md` with reference-pack validation and deviation rows where required.
- [ ] Reconcile open PR #70 independently and merge only safe green work.

No simulation logic, snapshots, append-only logs, canon, live deployment or governance changed.
