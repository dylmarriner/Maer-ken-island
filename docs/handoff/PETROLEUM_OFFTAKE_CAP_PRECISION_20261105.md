# Petroleum offtake-cap precision

Date: 2026-11-05
Repository: `dylmarriner/Maer-ken-island`
Branch: `fix/petroleum-offtake-cap-precision-20261105`
Base: current `main` after PR #86 merge

## Defect

`PetroleumField::produce_up_to` scaled the decline share by
`max_oil_m3 / potential`, then multiplied that share by the remaining reserve.
Floating-point rounding could make the returned oil exceed a finite offtake cap
by one ULP. Negative finite caps were silently treated as zero by `max(0.0)`
instead of being rejected as invalid input.

## Change

- Reject negative finite and negative-infinite offtake caps before state or ledger mutation.
- Preserve positive infinity as the explicit unbounded cap used by `produce()`.
- Normalise negative zero to zero.
- When a finite cap binds, return the cap itself and derive the shared gas-production
  fraction from that exact capped oil amount. This prevents the returned oil from
  exceeding the requested maximum through a multiply-back rounding step.
- Add regression coverage for negative caps/no mutation, a deterministic
  floating-point overshoot boundary, and equivalence of positive-infinity and
  unbounded production.

## Commits

- `e143b27f7b1e737e15f2a76810ca2836f44fa403` — exact finite cap and negative-cap guard.
- `853c417388fcf963f4ddc0213abe6d99ae448f65` — regression tests.

## Invariants

Valid finite intervals retain the same exponential decline model. Gas remains
produced in the same share as oil when oil remains; a field with no remaining
oil remains unchoked. No reference data, decline parameters, carbon/energy
constants, snapshot schema or persistence format changed. The one-week state
digest did move because the corrected finite-cap calculation changes the exact
floating-point production state. This is an intentional behavior change, not a
reference-pack edit or planetary-topology change.

## Verification

The exact-head GitHub CI must pass formatting, Clippy, fast tests, reference-pack
week/year acceptance, desktop/render, installed backend, dashboard, computer
service, repository scripts and dependency audit. No local Cargo result is
claimed because the custom OpenCommander connection is unavailable.

## Next

1. Verify the exact PR head, including the deterministic cap-boundary test.
2. Merge only after all required checks pass.
3. Continue the source-backed petroleum water-cut/spill requirements only when
   the documented field history and spill-volume sources support them; do not
   invent inputs.

No live island state, production infrastructure, network service or owner data
was changed.


## Exact-head cleanup

- `e144f7e245ba09beff50319649017da1893b0898` clarifies that both NaN and
  negative caps are invalid.
- `f546139b3a0e9fa3bd8b51a9da4f420a404c1373` normalises the regression-test
  spacing after the source/test additions.
- Current branch head: `f546139b3a0e9fa3bd8b51a9da4f420a404c1373`.
- These follow-ups do not alter the production algorithm or test assertions.
  A fresh exact-head CI run remains mandatory.


## First exact-head CI correction

Workflow [#41990190567](https://github.com/dylmarriner/Maer-ken-island/actions/runs/41990190567)
failed at `cargo fmt --all -- --check` on one assertion-wrapping difference in
the new negative-cap regression. The exact formatter layout was applied in the
follow-up commit. Clippy and fast tests were skipped by the formatting failure;
the independent reference-pack, desktop and installed-backend jobs were still
running at the time of this correction. A fresh exact-head run is required.


## Test-fixture selection correction

The island can contain gas-only petroleum fields. The finite-cap and unbounded-cap
regressions now explicitly select an oil-bearing field rather than assuming the
first surveyed field has oil. This keeps the cap-boundary assertion tied to a
non-zero oil reserve and avoids making the test depend on deposit ordering.
Commit: `402085e412f1f88095ea71baaa9620b8a7b3cb7d`.


## Audited week-digest movement

Exact-head run [#41990556526](https://github.com/dylmarriner/Maer-ken-island/actions/runs/41990556526)
observed `29ed53bd7769eb3025e70135bab8b7ea07718e26e7fca4cbf77ab379d62b7784` instead of the prior pin `bc8706aca29525e6bd75d27d94678a56cbd97a717aa4553ac8a15a2a64c7a076`.

The week test passed its ledger, water, founder, ecology, tree, eating and drinking
assertions before failing only at the stale digest equality. The independent
year-weather reference-pack test passed. The printed week evidence was 27,597
closed audits (167 hourly water audits), zero food/water shortfalls, 112 economy
events, 200,000 patch trees, body carbon 13.0/13.0 kg and 10.4/10.4 kg, and
temperate-forest NPP 471 g C/m²/yr against the 400–900 reference range.

The pin is updated to the exact observed digest in this follow-up. A fresh
exact-head workflow must pass the week and year acceptance gates before merge.
