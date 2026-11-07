# Fishery near-capacity surplus precision

Date: 2026-11-07

## Defect

The high-biomass Schaefer branch computed the remaining fraction as `1 - (B / K)`. When biomass is within a few ULPs of carrying capacity, division rounds `B / K` before subtraction. The resulting cancellation can materially overstate the true headroom fraction even though `K - B` is exactly representable.

A concrete adjacent-binary64 pair (`K=0x38d2a19742372a92`, `B=0x38d2a19742372a91`) overstates the fraction by about 16.4%.

## Change

Compute the headroom first, then divide:

`(K - B) / K`

This keeps the near-capacity surplus calculation tied to the representable stock headroom. The low-biomass depensation path and all snapshot layouts remain unchanged.

## Regression

`near_capacity_surplus_uses_headroom_ratio_without_cancellation` pins the adjacent binary64 values, proves the old expression differs by more than 10%, and checks the read-only annual surplus against the stable headroom expression within the shared material tolerance.

## Commit

- `467dce77` — production correction and regression.

## PR and base

PR #121: https://github.com/dylmarriner/Maer-ken-island/pull/121

The PR has been retargeted to `main` after PR #120 merged as `66bf099f36c567ab344fcc9b268a9a33a656011d`. It is no longer stacked on the historical PR #120 branch.

## Verification

A fresh exact-head CI run is required for this commit. Earlier green results on `f0001a15` and `8475337b` do not certify the new near-capacity correction. No local Cargo test result is claimed.

## Safety and architecture

No species parameters, reference data, snapshot fields, serialization layouts, append-only history, replay hashes, RNG, material-ledger schema, IslandLife state or workforce assignments changed. The calculation is deterministic and single-writer; the regression is read-only.
