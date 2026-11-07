# Unharvested fishery growth below the stock ULP

Date: 2026-11-07
Repository: `dylmarriner/Maer-ken-island`
Branch: `fix/fishery-unharvested-growth-remainder-20261107`
Base: `main` after PR #118 merge `134082a1c1bbcea837acc58344572476e8b14f74`

## Defect

`growth_capacity_t` could return a positive, representable interval-growth amount that was smaller than one ULP at the current biomass magnitude. In `fish()`, adding that increment to the stock could round back to the unchanged biomass. The existing guard rejected this only when a positive catch was taken. With a zero catch, the method could silently accept the step and discard all unharvested growth from the material ledger.

## Change

- Reject every positive unharvested growth remainder that rounds away when added to current biomass, regardless of whether the requested catch is zero, partial or full.
- Preserve full-harvest behaviour: when the entire interval growth is landed, `remaining_growth == 0`, so no biomass remainder is expected.
- Keep the failure before mutation; the existing `BiomassRemainderUnrepresentable` error remains the typed failure.

## Regression

Added `unharvested_growth_below_stock_ulp_is_rejected_without_mutation` in `crates/mk_engine/src/regional/industry/fishery.rs`.

The test uses a 1e308 carrying capacity, 1e-100 biomass, a finite 1e308 annual growth rate, depensation threshold 0.25 and a one-year step. The interval growth is positive and independently representable (about 4e-200), but adding it to 1e-100 biomass rounds to the same stored biomass. The read-only diagnostic returns the positive limit; a zero-catch `fish()` call returns `BiomassRemainderUnrepresentable` and leaves the entire stock unchanged.

## Commits

- `7f38911654fbf233de32e456c4e586abe33ba0f4` — broaden the production guard.
- `12ccd22872a3c9d87e1a0ab88d0e01a2cb0fae83` — add the zero-catch regression.
- Handoff and checklist commits to follow.

## Verification

- No local Cargo result is claimed; custom OpenCommander is returning HTTP 429.
- Exact-head CI is required for formatting, Clippy, fast Rust tests, desktop/backend acceptance and week/year reference-pack acceptance.
- Do not merge until every required job passes on the final PR head.

## Invariants

No reference values, species parameters, snapshot fields/layout, append-only logs, replay hashes, RNG, material ledger schema, IslandLife state or workforce assignments changed. Fishery remains standalone pending sourced species-specific stock/effort data and owner-approved workforce integration.
