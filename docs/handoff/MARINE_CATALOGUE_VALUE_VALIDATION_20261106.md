# Marine catalogue value validation — 6 November 2026

Repository: `dylmarriner/Maer-ken-island`
Branch: `fix/marine-catalogue-value-validation-20261106`
Base: main after PR #102 merge `2f11b5e9a3cd84a35f6cb69e1966cd3cd72391ed`

## Finding

The controlled catalogue replacement already rejected duplicate species identities and invalid marine abundance sources. It did not validate the marine species' upstream density or body mass before publishing the replacement. Those values were checked later only when a cell projection happened to include that species. A malformed marine catalogue could therefore be accepted when its affected species was not present in the queried cell.

## Change

- Validate every eligible OpenOcean/ReefSea aquatic species at the catalogue boundary.
- Require finite, non-negative upstream density.
- Require finite, strictly positive body mass.
- Reuse the existing typed `InvalidDensity` and `InvalidBodyMass` errors.
- Keep the per-cell projection checks as defence in depth.
- Add regressions proving rejected replacements leave both the original catalogue and cached projection unchanged.

## Invariants

No carrying capacity, resilience, CPUE, catch, workforce decision, simulation tick, RNG, snapshot, append-only log, replay hash, material ledger, reference pack or real-world parameter changed. No unsourced data was added.

## Commits

- `fc7de1be` — catalogue validation.
- `6c26c86e` — initial invalid-density/body-mass regressions.
- `23ab3d7e` — correct test attributes and apply rustfmt-style wrapping.
- `805ba8de` — initial handoff.

## Verification and PR status

This repository is public, so GitHub-hosted CI is available; the earlier assumption that another workflow would incur private-repository billing was incorrect. PR #102 is now merged into main. The follow-up branch has been rebased onto that merge and is being opened as PR #104. Exact-head formatting, Clippy, fast tests, desktop/backend and reference acceptance are required before merge. No local Cargo result is claimed while the authorised OpenCommander connector returns HTTP 429.

## Next

Verify PR #104's exact head. If all required jobs pass, merge only after confirming the catalogue and cached projection remain unchanged on rejected replacements. Do not add unsourced stock parameters.
