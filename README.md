# Maer-ken-island

Standalone island project bootstrapped from the human simulation work in `dylmarriner/Maer-Ken`.

This repository carries forward the canonical Maer-Ken human schema/runtime and exposes it as a public API for island-specific simulation work.

## Implementation roadmap

The top-level execution map is [`docs/IMPLEMENTATION_READY_ROADMAP.md`](docs/IMPLEMENTATION_READY_ROADMAP.md).

It defines the locked scope (a uniquely shaped, New-Zealand-sized island on an Earth-like Marr'Kena, simulated realistically), the phase order (0, 0b, 0c, 1–4, 4b, 5), the agent execution contract, acceptance gates, and links to the nine detailed implementation plans under `docs/superpowers/plans/`.

## What is here

- `crates/mk_core/src/human/` — canonical identity, genetics, temperament, neurocognition, personality, drives, hormones, attachment, schema and profile types.
- `crates/mk_engine/src/humans/` — live runtime systems for body, needs, cognition, emotion, memory, consciousness, reproduction, lifecycle, social behaviour and brain-state models.
- `apps/island_humans/` — the island-facing population bootstrap and CLI.
- `fixtures/human/` — canonical Gem-D / Gem-K human fixtures.
- `assets/humans/` — the founder GLB models.
- `tools/blender/generate_human_model.py` — the Maer-Ken human model generator.
- `docs/canon/` — the copied HumanReplicationSchema and human schema-family authority.

## Current bootstrap

`IslandHumanPopulation::with_founders()` loads the canonical founders Gem-D (`HUM-000001`) and Gem-K (`HUM-000002`).

New simulated people are created through the same deterministic `HumanBeing::new_born_at` path used by Maer-Ken. Identical agent ID, sex, birth data and birthplace produce the same profile.

## Run it

```bash
cargo test -p island_humans
cargo run -p island_humans -- founders
cargo run -p island_humans -- create islander-001 female 2000-01-02T03:04:05Z -36.85 174.76 "Maer-Ken Island"
```

## Source authority

The extraction is pinned by provenance to Maer-Ken commit `7c05f0dcf254387ffd7322dbb525fe4807228602`. See `UPSTREAM.md` before changing imported human behaviour.
