# Marine projection metadata guard

**Date:** 2026-11-06  
**Branch:** `perf/index-marine-aquatic-species-20261106`  
**PR:** #95 — `perf(species): index and validate marine aquatic projections`

## Finding

The marine projection caches species vector indices at survey time. The catalogue is publicly mutable. The existing stale-index guard checked only `species_id`, so changing an indexed species' category or terrain affinity while keeping its ID could leave the cached projection semantically stale:

- an indexed marine species could be changed to a terrestrial category while retaining an OpenOcean/ReefSea affinity and still be emitted as marine stock;
- changing an indexed species from OpenOcean/ReefSea to River could silently skip it because the current cell mask no longer matched, rather than reporting that the cached index no longer described the surveyed catalogue.

## Change

- Store the surveyed affinity alongside each cached species ID and vector index.
- Return typed `MarineProjectionError::StaleSpeciesMetadata` when an indexed species is no longer aquatic or its affinity has changed.
- Perform this validation before the current cell-mask check, so a stale entry cannot be silently omitted.
- Preserve the read-only derived projection and all simulation/replay invariants.

## Regression coverage

Two integration tests clone the surveyed biota, mutate one indexed marine species while preserving its identity, and assert the typed stale-metadata error:

1. category changes from Aquatic to Tree;
2. affinity changes from OpenOcean/ReefSea to River.

## Commits

- `37398b39aa38df1f29d4ee249289b494040ad57f` — cached metadata validation.
- `5ae24fd55f2ab30a0a166e582606fa791bd8f303` — category/affinity regressions.
- `6b5b8c5e15c509356a01999f68689bd1ee4523c4` — rustfmt-style line wrapping.

## Verification

The commits are pushed to the existing PR #95 branch. Cargo fmt, Clippy, fast tests, desktop/backend builds and week/year reference acceptance have not yet been executed on this exact head. The prior exact-head run on the previous commit reported a rustfmt failure; this follow-up formats the new long predicates, but only a fresh exact-head CI run can verify formatting and the full suite.

No reference pack, real-world parameter, deviation register, snapshot, append-only log, material ledger, RNG stream, replay digest, or live simulation state changed.

## Next

1. Inspect the exact-head workflow after these commits.
2. Require formatting, Clippy, fast tests, desktop/backend builds and week/year acceptance to pass.
3. Merge PR #95 only after all required checks are green.
4. Continue fishery integration into IslandLife only with sourced resilience, carrying-capacity and CPUE parameters.

No production deployment, external service, LLM, budget spend or simulation-state mutation was performed.

## Exact-head formatting follow-up

The first workflow after the metadata-guard commits checked out the PR merge commit and reported two rustfmt differences:

- the earlier large-population test assignment in `regional/species.rs`;
- the new category-regression test's single-line habitat iterator.

Both were corrected in follow-up commits:

- `7485eb9877a1cb3c072878cb26ecefcbab3078fc` — source test formatting.
- `1657d7217770266feffa7f1ed75af5f5ced60864` — integration test formatting.

The previous run's formatting failure is now addressed in source, but a fresh exact-head workflow is still required. Clippy, fast tests, backend/desktop builds and week/year acceptance remain unverified until that run completes.

## Fixture update after adding the affinity snapshot

The new `MarineAquaticSpeciesIndex.affinity` field also required updating the existing private unit-test fixtures in `species.rs`. All 11 explicit index literals now provide the surveyed affinity; the index-order test records OpenOcean for species 1 and ReefSea for species 3.

Commit: `ddf35001a10cc6d79e3dab1ab03743c7f48c8343`.

A fresh exact-head CI run is required after this fixture update. The previous workflow was started before this commit and cannot verify it.


## Follow-up: preserve aquatic abundance semantics

A further mutable-catalogue gap remained after category/affinity fencing: an indexed aquatic species could retain its ID, category and marine affinity while its public `abundance` field changed from `UpstreamDensity` to a carbon-pool variant. The marine projection would then report biomass from upstream density while the general animal projection treated the same species as carbon.

The checked projection now rejects any indexed marine species whose abundance is not `Abundance::UpstreamDensity`, using the existing typed `StaleSpeciesMetadata` error.

Regression coverage mutates a surveyed marine species to both `ProducerCarbon` and `GrazerCarbon` and verifies that neither can enter the fishery projection.

This remains a read-only derived-data guard. It does not alter snapshots, logs, RNG, replay digests, ecological parameters, material ledgers or live simulation state.

A fresh exact-head workflow is required; no Rust test or Clippy pass is claimed until CI reports it.
