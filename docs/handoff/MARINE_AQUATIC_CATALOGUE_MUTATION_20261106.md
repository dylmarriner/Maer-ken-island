# Marine species catalogue mutation guard — 6 November 2026

Repository: `dylmarriner/Maer-ken-island`
Branch: `perf/index-marine-aquatic-species-20261106`
PR: #95

## Review finding addressed

Codex identified that `IslandBiota::species` was a public `Vec`. The cached marine index detected reorder and metadata changes for already indexed species, but it could not discover a species changed from non-marine to OpenOcean/ReefSea aquatic or a species appended after survey. Such a mutation could silently omit that species from the marine projection.

## Change

- Make the species catalogue private.
- Expose `species()` as an immutable slice.
- Add `replace_species_catalogue(Vec<IslandSpecies>)` as the controlled replacement path.
- Validate that every OpenOcean/ReefSea aquatic species uses `UpstreamDensity`; reject invalid marine abundance before mutating the existing catalogue.
- Rebuild the marine membership index and producer/grazer carbon weights before returning from a successful replacement.
- Keep identity/metadata checks in `try_marine_aquatic_nodes` as defence in depth against internal drift.

## Regression coverage

- Category changes remove the species from the marine projection after controlled replacement.
- Affinity changes remove the species from the original marine habitat projection.
- Invalid producer/grazer abundance for a marine aquatic species is rejected and leaves the original catalogue intact.
- A previously non-marine species changed to OpenOcean aquatic is included in the rebuilt index.
- Existing per-cell/whole-island population and biomass conservation tests now use the read-only accessor.

## Commits

- `7c027f2ab6f98d17443df48d94945575773dd41a` — private catalogue, controlled replacement, index/weight rebuild.
- `6eb3d582c42c8c71d4f6e257e79d0aea6cc70c77` — borrow-safe validation and defensive comments.
- `8663ce13daa6377aa2cc1f6e8f9ba452e801c2fc` — membership and replacement regressions.
- `5c090dd6928371811d02bc1e33180660354ba785` — grazer-weight rebuild regression.
- `fb32fe414a0f7eef656459ed152d91869f813973` — checked test-fixture arithmetic.
- Documentation commits update `WORK_REMAINING.md`, `docs/ISLAND_SYSTEM_STATUS.md`, `docs/island/DEVIATIONS.md` and the Phase 3 program plan.

## Verification

No local Cargo result is claimed because custom OpenCommander discovery returned HTTP 429. The prior exact-head CI on the pre-review-fix head passed, but that does not verify this follow-up. A new exact-head run must pass formatting, Clippy, fast Rust tests, desktop/backend builds, week/year reference acceptance, web, computer service, repository scripts and dependency audit.

## Invariants

No simulation state, RNG, append-only log, snapshot, ledger, replay input/hash, upstream source data, fishery calibration or external service was changed. The catalogue remains derived from the scenario and is not persisted.
