# Reference range midpoint overflow — 6 November 2026

**Repository:** `dylmarriner/Maer-ken-island`  
**Branch:** `fix/reference-range-midpoint-overflow-20261106`  
**PR:** [#109](https://github.com/dylmarriner/Maer-ken-island/pull/109)  
**Code/test commit:** `0c1538a968a291674678033645e7b221aa1bb177`  
**Exact rustfmt correction:** `cf77459dd355d3564bce6228889374398e47699b`

## Finding

`ReferenceItem::central()` calculated an implicit range midpoint as `0.5 * (min + max)`. A reference pack may contain finite, ordered endpoints whose sum overflows even though the mathematical midpoint is finite. The pack validator accepted those endpoints, so a downstream model could receive infinity from a valid range.

## Change

- Same-sign endpoints use `min + (max - min) * 0.5`; the difference is bounded by the finite same-sign interval.
- Opposite-sign endpoints are halved before addition, so a full-domain range from negative to positive `f64::MAX` cannot overflow.
- Explicit `typical` values remain unchanged.
- Equal subnormal endpoints remain exactly representable.

## Regression coverage

`crates/mk_engine/tests/reference_packs.rs::range_midpoint_stays_finite_for_valid_extreme_bounds` covers:
- large positive same-sign endpoints;
- large negative same-sign endpoints;
- the full finite range spanning zero;
- equal smallest-subnormal endpoints;
- an explicit typical value.

## Verification

Initial exact-head workflow [#42198602010](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42198602010) failed only at `cargo fmt --all -- --check` on the explicit-typical test's line wrap; the exact formatter output is committed as `cf77459d`. Desktop, installed backend and week/year acceptance were still running when the format gate failed, so their results are not claimed. A fresh exact-head run is required. Custom OpenCommander returned HTTP 429; no local Cargo result is claimed. The PR remains draft pending exact-head CI and review.

## Safety

No real-world reference value or source pack was changed. No physical parameter, fishery calibration, deviation, simulation state, snapshot/replay format, hash baseline, budget or offline limit changed.
