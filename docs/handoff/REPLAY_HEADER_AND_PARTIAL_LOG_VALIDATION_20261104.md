# Replay header and complete-log validation

**Date:** 2026-11-04  
**Repository:** `dylmarriner/Maer-ken-island`  
**PR:** #75, stacked on #74 → #73  
**Branch:** `fix/replay-inmemory-digest-validation-20261104`

## Change

The replay file loader already rejected unsupported `REPLAY_LOG_VERSION` values, but the writer and direct in-memory replay path did not enforce the same format-version contract. Direct replay also validated only entries at or before `until_tick`, allowing a partial replay to accept a malformed later portion of the same log.

The change centralises header validation (format version plus canonical 64-character lowercase-hex scenario digest) and applies it at load, save and in-memory replay boundaries. In-memory replay now validates the complete command sequence before bootstrapping the island, then filters the validated entries for the requested replay horizon.

The existing save size-preflight ordering is preserved: oversized serializations still report `TooLarge` before header-field validation.

## Regression coverage added

- In-memory replay rejects an unsupported log version.
- Loading rejects an unsupported log version.
- Saving rejects an unsupported log version and leaves no output file.
- A partial replay rejects an invalid sequence in entries beyond the requested tick.

## Commits

- Source: `31505098546e7d025852460821e0a66f0bd31ff0`
- Tests: `f92b9fbf40bf5916d473a3e821e84fb8adc37f5c`

## Verification

- Changes are pushed to the PR branch.
- Exact-head GitHub Actions must run after the test commit.
- No local Cargo test pass is claimed.
- The preceding exact-head run #41806794152 passed formatting, Clippy, fast tests, desktop build/tests/render smoke, installed backend, web dashboard, computer service, dependency audit and repository scripts. Its nightly slow job was skipped and it predates this change.

## Remaining

1. Verify the new exact-head fast test and all required CI jobs.
2. Run the mandatory week/year reference-pack slow tests on the supported pinned toolchain.
3. Re-run fixed-seed replay and compare canonical state digests.
4. Keep PR #75 stacked until PRs #73 and #74 are accepted; do not retarget or merge early.

No simulation state, scenario digest algorithm, snapshot format, owner governance, live infrastructure or SCOS state was changed.


## CI correction (2026-11-04)

The first run on the new head, #41816490579, failed at `cargo fmt --all -- --check` on one multiline assertion in the new unsupported-version save test. The formatter's exact diff was applied in commit `28488c39e7a8f57d9f0452fd0b87a6ca3cc6792f`. The follow-up exact-head run #41816993183 is pending. No Rust test pass is claimed for the new code until that run completes.

## Whole-project remaining checklist (source: `WORK_REMAINING.md` and `docs/ISLAND_SYSTEM_STATUS.md`)

- [ ] Timber: finish the remaining routes/sawing and tool/recipe work; keep sourced action durations and material ledgers.
- [ ] Living world: finish remaining gem/tin placer models, tools/recipes, species-to-resource wiring and the D31 soil-nutrient limitation on NPP.
- [ ] Human/runtime: complete action timing for craft/explore/social actions, job attendance/trades/maintenance, free will to quit/leave/settle, and upstream-first human mind changes. Do not put staff or villagers in the founders' house; estate staff have separate cottages.
- [ ] Industry: remaining oil-field water-cut/spill/workover/harm models, refinery reformer, staffing and maintenance, petroleum payroll/payslips, household bills and home/business market decisions.
- [ ] Food/timber chain: wire forestry, sawmill, farms, livestock, food processing and fishery into the map and `IslandLife`; fishery must consume the island's actual fish populations; finish sourced roof-truss/second-stage food/supermarket details.
- [ ] Complete genetics/reproduction and remaining human agency/psyche/qualia requirements without invented biological or real-world constants.
- [ ] Every real-world number must come from a cited reference pack; every material movement must close the audited carbon/water ledgers; every known departure must be in `docs/island/DEVIATIONS.md`.
- [ ] Run `island_life_acceptance -- --ignored`, both week and year reference-pack slow tests, and fixed-seed replay-hash equality. Re-pin `WEEK_DIGEST` only after the audit passes.
- [ ] Complete Phase 5 acceptance. PR #75 stays stacked on #74 → #73 until parent acceptance; PR #68 and the paused merge-completion job remain separate.

## Architecture and ownership constraints

Preserve deterministic Rust single-writer state, fixed-tick ordering, append-only replay logs/snapshots, canonical replay hashes, fail-fast production invariants, offline operation and budget limits. LLM integration remains optional. Do not alter simulation state or owner governance as part of replay hardening. `WORK_REMAINING.md` explicitly requires upstream-first changes to `dylmarriner/Maer-Ken`; that is a separate repository and must not be confused with this target `Maer-ken-island` repository.

## Latest verification and next task

- Exact-head run #41816490579 failed only `cargo fmt --all -- --check` on one new assertion; commit `28488c39e7a8f57d9f0452fd0b87a6ca3cc6792f` applied the formatter output.
- Current follow-up run #41817102111: Format and Clippy passed; fast tests were in progress at last poll. Desktop build and tests passed, island render smoke was in progress, installed backend passed; dashboard, computer service, dependency audit and repository scripts passed. Nightly week/year slow tests were skipped.
- **Next:** finish exact-head fast/desktop checks, then obtain week/year reference-pack and fixed-seed replay evidence before choosing the next `WORK_REMAINING.md` engineering slice.