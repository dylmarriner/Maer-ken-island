# Reference range midpoint overflow — 6 November 2026

**Repository:** `dylmarriner/Maer-ken-island`  
**Branch:** `fix/reference-range-midpoint-overflow-20261106`  
**PR:** [#109](https://github.com/dylmarriner/Maer-ken-island/pull/109)  
**Code/test commit:** `0c1538a968a291674678033645e7b221aa1bb177`

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

Exact-head `cargo fmt --all -- --check`, workspace Clippy and `cargo test --workspace --exclude island_ui` are required. Custom OpenCommander access is not available in this run, so no local Cargo result is claimed. The PR is draft pending exact-head CI and review.

## Safety

No real-world reference value or source pack was changed. No physical parameter, fishery calibration, deviation, simulation state, snapshot/replay format, hash baseline, budget or offline limit changed.
