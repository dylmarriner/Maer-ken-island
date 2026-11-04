# Week digest reconciliation — 5 November 2026

Canonical repository: `dylmarriner/Maer-ken-island`
Branch: `fix/week-digest-audit-20261105`
Pull request: [#77](https://github.com/dylmarriner/Maer-ken-island/pull/77)

## Finding

The week reference-pack acceptance job on PR #76 run [#41861164760](https://github.com/dylmarriner/Maer-ken-island/actions/runs/41861164760) ran both slow reference tests independently.

- Week test: all ledger, founder, tree, ecology and water assertions passed; the final digest assertion failed because the source constant was `c32897c4…7054` while the observed state digest was `bc8706aca29525e6bd75d27d94678a56cbd97a717aa4553ac8a15a2a64c7a076`.
- Year-weather reference-pack test: passed.
- The week output reported 27,597 closed ledger audits, 167 hourly water audits, zero food/water shortfalls, 112 economy events, 200,000 patch trees, Gem-D body carbon 13.0/13.0 kg, Gem-K 10.4/10.4 kg, and temperate-forest NPP 471 g C/m2/yr against the reference 400-900.
- The current test's own eating and drinking assertions also passed: 44/39 minutes per day and 2.49/1.72 L per day respectively.

The previous pin was last changed on 22 October. The default branch later integrated the town into `IslandLife` (PR #66) and merged fitness/genome/free-will work on 29 October. The digest includes that complete state. Active documents were also inconsistent: the source held `c32897c4…7054`, `WORK_REMAINING.md` listed `577e4215…540a`, and `HANDOFF.md`, `docs/TESTING.md` and the state handoff listed `f6699166…9c81`.

## Change

- Re-pin the week digest to the exact observed value `bc8706aca29525e6bd75d27d94678a56cbd97a717aa4553ac8a15a2a64c7a076` after the pre-digest audit and both reference-pack checks passed.
- Update all active documents to the full digest; historical session notes remain historical.
- Refresh `WORKING_PRACTICES.md` so it describes the current test assertions and measured evidence rather than obsolete October thresholds.
- Add `week_digest_documentation.rs` to fail the fast Rust tier if any active digest reference drifts from the canonical test constant.

No simulation, physics, replay, snapshot, real-world reference pack, owner-governance or production state was changed.

## Commits

- `dfb8fe39b3eacad9acfe99f5d07350026814ae2e` — crates/mk_engine/tests/island_life_acceptance.rs
- `34542a713be857708ee042ada300f43a6c76404d` — WORK_REMAINING.md
- `2b7621dca9a5725c4b95828ed91b62f1c25aec68` — docs/TESTING.md
- `dbb9f8c1fe821b987bc5ede34655f00f653e0a2b` — docs/handoff/STATE_OF_THE_ISLAND.md
- `602944699ad9dc572170c3c30e279965e1b659b7` — HANDOFF.md
- `a3dcad898170a703d480efad5a39c1391fc941ae` — docs/handoff/WORKING_PRACTICES.md
- `63681d206db68809acd16bfce2ee1f9074639345` — `crates/mk_engine/tests/week_digest_documentation.rs`

## Verification

- Evidence from prior exact-head run: week test's non-digest assertions passed; year-weather reference test passed.
- This branch's new consistency test and the re-pinned week/year tests have **not yet run**. A fresh exact-head CI run is required before this change can be accepted.
- Do not merge if either week or year reference acceptance fails.

## Next

1. Verify the new digest consistency test in the fast Rust suite.
2. Verify both week and year reference-pack acceptance on this exact PR head.
3. Run the full slow tier before final simulation acceptance and preserve deterministic replay-hash equality.


## Current exact-head CI

- The PR now includes the `reference-acceptance` workflow gate, copied as a workflow-only change from the reviewed PR #76 patch. It runs both the week and year reference-pack tests independently for changes under `crates/*/(src|tests)`, reference fixtures, `UPSTREAM.md` or the deviation register.
- The digest source and the new documentation-consistency test are within that path scope, so both reference tests are required for this PR.
- Code-bearing head `a9cbb604b1b58e0b49de4093a24b3a8a677a5b19` has workflow run [#41877917983](https://github.com/dylmarriner/Maer-ken-island/actions/runs/41877917983); it was queued at last inspection. The earlier run #41877017851 predates the gate and does not satisfy this acceptance requirement.
- This handoff update is documentation-only. It does not alter simulation code or the acceptance test. No new-head pass is claimed; the next execution must inspect the workflow attached to the latest PR head.

## Exact-head CI follow-up (2026-11-05)

Run [#41878005820](https://github.com/dylmarriner/Maer-ken-island/actions/runs/41878005820) failed the initial `cargo fmt --all -- --check` step because the new `week_digest_documentation.rs` assertion expressions were not rustfmt-formatted. The failure occurred before Clippy, compilation or runtime tests; it provides no new digest/reference-pack result.

The test source was reformatted to match the exact rustfmt diff from the workflow log. This is a formatting-only correction; the digest value and simulation behavior are unchanged. A fresh exact-head run must pass formatting, fast tests, the documentation-consistency test and both week/year reference acceptance tests before this PR can be considered verified.
