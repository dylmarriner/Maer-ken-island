# Unique species catalogue identity guard — 6 November 2026

Repository: `dylmarriner/Maer-ken-island`  
Branch: `fix/unique-species-identities-20261106`  
Parent: PR #95, `perf/index-marine-aquatic-species-20261106`

## Finding

The cached marine projection uses `species_id` plus the catalogue index and surveyed affinity as its identity guard. The public catalogue was made private and replacement was routed through `replace_species_catalogue`, but that API still accepted duplicate `species_id` values.

Two catalogue entries with the same ID could therefore make the identity guard ambiguous: reordering equal-ID entries with the same affinity could leave the cached identity check unable to distinguish which species occupies the index. Downstream marine nodes could then attribute a population/body mass to the wrong catalogue entry.

## Change

- Add a typed `MarineProjectionError::DuplicateSpeciesId` carrying the first index, duplicate index and ID.
- Validate uniqueness across the entire catalogue before any replacement/cache mutation.
- Run the same validation on the upstream founding catalogue during `IslandBiota::survey`; duplicate upstream identities fail fast instead of producing an ambiguous derived view.
- Preserve the existing marine-abundance validation in the same catalogue validation pass.
- Add an integration regression that attempts a duplicate-ID replacement and proves both the original catalogue and its cached marine projection remain unchanged.
- Update the island status, plan, deviation note, remaining-work ledger and root handoff.

## Commits

- `89dd1036ad07a72aba690bba194c79425395802f` — typed duplicate-ID validation at survey and replacement boundaries.
- `e0b7f0c571206a6baf55d3866522dbd65ba7cdee` — atomic replacement regression.
- `9cd4679128417b98e266caebed6fc3ad791cc9df` — island status.
- `05eb0180a85bbffc856334f408b1b57a469db833` — remaining-work ledger.
- `5c4679296667ecfd1ff09e77ffda8cb3715d9a90` — deviation register note.
- `bde23006ea130c15c883688748f1bc4b8ad13282` — Phase 3 plan status.

## Invariants

The catalogue and index remain derived, read-only views of upstream species. No simulation state, RNG stream, snapshot layout, append-only log, material ledger, replay input/hash, ecological parameter or external service is changed. The new validation is deterministic and does not introduce a physical-model assumption.

## Verification

No local Cargo result is claimed because custom OpenCommander device discovery returns MCP HTTP 429. The parent PR #95 exact-head workflow is separately running; this stacked PR requires its own exact-head CI for formatting, Clippy, fast tests, desktop/backend builds and week/year reference acceptance. Keep this PR draft until all required checks pass.
