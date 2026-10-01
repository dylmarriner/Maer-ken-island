# Upstream provenance

Source repository: `dylmarriner/Maer-Ken`
Source commit: `7c05f0dcf254387ffd7322dbb525fe4807228602`
Extraction date: 2026-10-01

Imported directly from that commit:

- `crates/mk_core`
- `crates/mk_engine`
- `crates/mk_interventions`
- `fixtures/human`
- canonical human schema documents under `docs/canon`
- `tools/blender/generate_human_model.py`
- Gem-D and Gem-K GLB assets

Intentional island divergence at extraction:

1. `mk_engine::humans` is public in this repository so the island application can use the human runtime directly.
2. `apps/island_humans` is new and owns island population bootstrapping.
3. A small amount of rustfmt-only normalization occurred in copied source files.

Human schema or runtime changes should be compared against upstream first. Island-only rules belong in `apps/island_humans` unless the underlying human model itself is intentionally being changed.
