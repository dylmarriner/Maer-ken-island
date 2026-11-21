# Fishery biomass remainder precision

Date: 2026-11-07  
Repository: `dylmarriner/Maer-ken-island`  
Branch: `fix/fishery-biomass-remainder-precision-20261107`  
Base: `main` at `148ac98c95e58aac7b2065dd0cc68884790f4168` (PR #110 merge)

## Defect

`FishStock::fish` subtracts the requested catch from interval growth and adds the
remainder to stored biomass. At large biomass magnitudes, a positive remainder can
be smaller than half an ULP of `biomass_t`, so the addition rounds back to the
unchanged biomass. The method could then record a positive partial catch while
silently discarding the unharvested part of that interval's growth.

That is different from a fully harvested growth interval: when `catch == grown`,
the remainder is zero and unchanged biomass is correct.

## Change

- Add the typed `FisheryError::BiomassRemainderUnrepresentable`.
- Reject only when a positive catch leaves positive residual growth that cannot
  change the stored biomass value.
- Perform the check before either biomass or landed total is committed.
- Preserve no-catch steps and fully harvested growth intervals.
- Keep the existing `FishStock` serialization fields and order unchanged.

## Regression

`partial_harvest_rejects_unrepresentable_remaining_growth_without_mutation`
uses `K = 2^53`, `B = 2^52`, and a short interval producing approximately
0.125 tonnes of growth. It requests half the growth, proving:

- the positive residual is below half the one-tonne ULP at the stored biomass;
- adding that residual leaves `biomass_t` unchanged;
- the requested catch is independently representable in `landed_t`;
- `fish()` returns the typed error; and
- the entire stock equals its pre-call clone.

## Verification

- Source and regression are committed together with the handoff and checklist.
- Exact-head GitHub Actions is required: fast Rust tests, Clippy, desktop/backend
  checks and week/year reference-pack acceptance must pass before this invariant
  is marked complete.
- Custom OpenCommander device discovery returned HTTP 429 in this run; no local
  Cargo test result is claimed.

## Safety

No species calibration, reference data, RNG, append-only history, replay hash,
material ledger, snapshot schema, IslandLife integration or production state was
changed. Fishery-to-IslandLife integration still requires sourced species and
effort data plus owner-approved workforce decisions.
