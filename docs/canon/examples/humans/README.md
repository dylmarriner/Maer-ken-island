# Human Fixture Bundles

This directory contains the canonical human schema fixtures used by Rust contract tests mirrored from `docs/canon/HumanReplicationSchema.js`.

## Whole-schema fixtures

- `golden-human.min.json` — minimal end-to-end human fixture.
- `golden-human.full.json` — full end-to-end human fixture used as the authoritative source for family extraction.
- `invalid-missing-agent-id.json` — whole-schema rejection case.

## Family fixture bundles

- `schema-family-fixtures.valid.json` — authoritative valid fixtures for the schema families still marked `Partial` or `Deferred` in `docs/canon/SCHEMA_TO_ENGINE_OWNERSHIP_MAP.md`.
- `schema-family-fixtures.invalid.json` — matching rejection cases with one intentional contract violation per family.

The family bundles currently cover:

- `body`
- `runtime`
- `core_systems`
- `cognition`
- `emotion`
- `biology`
- `sensory`
- `reproductive`
- `brain`

The valid bundle is extracted from `golden-human.full.json` so the family fixtures stay aligned with the current canonical aggregate fixture instead of drifting into hand-maintained copies.
