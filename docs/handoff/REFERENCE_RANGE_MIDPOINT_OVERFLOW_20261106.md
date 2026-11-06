# Reference range midpoint overflow — 6 November 2026

**Repository:** `dylmarriner/Maer-ken-island`  
**Branch:** `fix/reference-range-midpoint-overflow-20261106`  
**PR:** [#109](https://github.com/dylmarriner/Maer-ken-island/pull/109)  
**Code/test commit:** `0c1538a968a291674678033645e7b221aa1bb177`  
**Exact rustfmt correction:** `cf77459dd355d3564bce6228889374398e47699b`  
**Eager fallback cleanup:** `828f748b14e19d9f7dcc01cb57506f12ef8b3c06`

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

Initial exact-head workflow [#42198602010](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42198602010) failed only at `cargo fmt --all -- --check` on the explicit-typical test's line wrap; the exact formatter output is committed as `cf77459d`. The eager fallback in `828f748b` avoids an unnecessary-lazy-evaluation Clippy warning. Desktop, installed backend and week/year acceptance were still running when the format gate failed, so their results are not claimed. A fresh exact-head run is required. Custom OpenCommander returned HTTP 429; no local Cargo result is claimed. The PR remains draft pending exact-head CI and review.

## Safety

No real-world reference value or source pack was changed. No physical parameter, fishery calibration, deviation, simulation state, snapshot/replay format, hash baseline, budget or offline limit changed.

## Subnormal midpoint rounding follow-up

A second numerical edge was found during review: halving each endpoint or halving the endpoint difference can underflow before the midpoint is rounded. The helper now sums opposite-sign endpoints directly (safe from overflow) and sums small same-sign endpoints before halving; only large same-sign ranges use the bounded-difference formula.

Regression `range_midpoint_preserves_rounding_for_adjacent_subnormals` covers positive and negative adjacent subnormal ties and a range spanning zero. This is a numerical utility correction only; no published reference value or simulation state changed. Exact-head Rust formatting, Clippy, fast tests, desktop/backend and week/year acceptance remain required.


## Wide-exponent-gap double-rounding follow-up

A further exact-arithmetic review found that `min + (max - min) * 0.5` can still be one ULP wrong when subtracting endpoints rounds before the final addition. The original extreme-range and adjacent-subnormal tests did not cover this exponent-gap case.

- The midpoint helper now uses `(min + max) * 0.5` only when both magnitudes are at most `f64::MAX / 2`; that sum cannot overflow and preserves subnormal tie rounding.
- Otherwise it uses `min * 0.5 + max * 0.5`. Halving the large finite endpoints is exact and the final addition performs the rounding once, without an overflowing intermediate sum.
- Added positive and negative regressions for `min = 3 × 2^969` and `max = 2^1023`. The positive expected result is bit pattern `0x7fd0000000000001`; the prior difference-based formula returned `0x7fd0000000000000`.
- Commits: `2471472c` (midpoint algorithm), `f07a3d29` (regressions).
- Exact-head workflow `#42205227479` on code/test head `f07a3d29` was cancelled after subsequent documentation-only pushes superseded the in-progress run. Dashboard, computer service, dependency audit and repository scripts passed; the fast Rust tests, week reference acceptance, desktop and installed backend were cancelled. No Rust test or week/year pass is claimed for the new algorithm. The current PR head is documentation-only after the code/test commit; a fresh exact-head run is still required.

No source-pack values, citations, physical parameters, deviations, simulation state, snapshot/replay format, hash baseline, budget or offline limit changed.
