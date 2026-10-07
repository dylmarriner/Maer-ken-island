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
- `apps/island/` — the dashboard: the overview, the roster, the Human Creator and the JSON behind them.
- `fixtures/human/` — canonical Gem-D / Gem-K human fixtures.
- `assets/humans/` — the founder GLB models.
- `tools/blender/generate_human_model.py` — the Maer-Ken human model generator.
- `docs/canon/` — the copied HumanReplicationSchema and human schema-family authority.

## Current bootstrap

A stored population lives in a data directory (default `./island-data`). Every human, Gem-D (`HUM-000001`) and Gem-K (`HUM-000002`) included, has their own folder under `island-data/humans/<agent-id>/` with `profile/`, `traits/`, `cognition/`, `social/`, `development/`, `reproduction/`, `memories/`, `state/`, `relationships/` and `events/`.

New people are built through upstream's `SpawnHuman` path from the population's seed and a creation counter. Every creation is appended to `island-data/creations.jsonl`, so replaying that log with the same seed reproduces the same people.

## Run it

```bash
cargo test --workspace                          # fast tier
cargo run -p island -- serve                    # dashboard at http://127.0.0.1:8080/
cargo run -p island_humans -- founders          # list everyone (CLI)
cargo run -p island_humans -- create "Hine Moana" female 1992-11-03T10:15:00+13:00 -41.3 174.8 --height 166 --hair black
```

### The dashboard (`island serve`)

Three pages, served from the binary with nothing to install beside it:

- **Overview** (`/`): how many people there are, how old they are, where their
  records live, who was added lately, and what does and does not work yet.
- **People** (`/people`): the roster, searchable and sortable, and everything
  stored about whoever you pick — who they are, what their body is doing, how
  they feel, how they think — with the complete record underneath to copy or
  download. `/people?human=<agent-id>` opens straight to one person.
- **Create a human** (`/creator`): name, sex, birth date and place, age and
  appearance. Each new person gets a folder immediately.

The JSON behind the pages is public too: `/api/status`, `/api/health` (also
`/healthz`), `/api/humans`, `/api/humans/<agent-id>`, `/api/activity` and
`/api/creator/options`. `POST /api/humans` is the only write.

Every response carries a strict content security policy and the usual
hardening headers; the pages load no third-party script, style or font. One
line per request is printed to stdout — set `ISLAND_ACCESS_LOG=off` to stop
that.

`island serve [--data-dir DIR] [--bind ADDR:PORT] [--seed HEX64]`. Reading never needs a token. Creating people is allowed:

- with `ISLAND_CONTROL_TOKEN` set: only for requests carrying `Authorization: Bearer <token>`. The Creator page has a field for it, kept in the browser tab only.
- without a token on a loopback bind (the default `127.0.0.1:8080`): from this machine.
- without a token on any other bind: never. Set a token before using `--bind 0.0.0.0:…`.

**Time is not running yet.** People are created and stored, but nobody ages, eats or acts until the island world exists (Phase 4).

**Back up your storage key.** Human files are encrypted with `island-data/humans/.secret_storage_key`, generated on first run, or with `MK_STORAGE_KEY` if set. Lose the key and every human file becomes unreadable. Back it up with the data, or set `MK_STORAGE_KEY` yourself.

## Source authority

The extraction is pinned by provenance to Maer-Ken commit `7c05f0dcf254387ffd7322dbb525fe4807228602`. See `UPSTREAM.md` before changing imported human behaviour.
