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

- **People** (`/`): the roster, and everything stored about whoever you pick.
- **Create a human** (`/creator`): name, sex, birth date and place, age and appearance. Each new person gets a folder immediately.

`island serve [--data-dir DIR] [--bind ADDR:PORT] [--seed HEX64]`. Reading never needs a token. Creating people is allowed:

- with `ISLAND_CONTROL_TOKEN` set: only for requests carrying `Authorization: Bearer <token>`. The Creator page has a field for it, kept in the browser tab only.
- without a token on a loopback bind (the default `127.0.0.1:8080`): from this machine.
- without a token on any other bind: never. Set a token before using `--bind 0.0.0.0:…`.

That is the older, human-only dashboard (`--data-dir`): people are created and stored, but time does not pass there. The live island below is where time runs.

**Back up your storage key.** Human files are encrypted with `island-data/humans/.secret_storage_key`, generated on first run, or with `MK_STORAGE_KEY` if set. Lose the key and every human file becomes unreadable. Back it up with the data, or set `MK_STORAGE_KEY` yourself.

## The live island

The whole island (climate, weather, ocean, rivers, ecology, the estate, 200,000 trees, energy, materials and the people on it) runs as one deterministic world.

```bash
# Run it headless: 1,000 one-minute steps, save a snapshot, write a replay log
cargo run --release -p island -- run --scenario fixtures/island/default_scenario.json \
    --steps 1000 --dt 60 --save run.snap --log run.log.json --save-root island-data
cargo run --release -p island -- inspect --snapshot run.snap            # summary and state hash
cargo run --release -p island -- replay --scenario fixtures/island/default_scenario.json --log run.log.json
cargo run --release -p island -- run --load run.snap --steps 1000       # continue a saved run

# Serve it live (real time by default; see the speed control on the page)
cargo run --release -p island -- serve --scenario fixtures/island/default_scenario.json \
    --save-root island-data --snapshot-out run.snap
```

Every command prints a `state hash`: the same scenario, seed and commands always give the same hash, in any process, at any speed, and a saved run continues to exactly the hash an uninterrupted run reaches. `--save-root` keeps a folder for every person under `<dir>/<run id>/humans/` (the run id is printed; each run gets a new one). `--import-0b <data-dir>` brings in people created with the older dashboard. `island_preview world --snapshot run.snap --out dir` renders a saved run as images.

The live page (`/`) shows the clock, lets you pause, step and change speed, lists the people, and adds a person (optionally choosing appearance and where they appear, e.g. the Kitchen). The API is `GET /api/status|humans|humans/{id}|estate|properties|vegetation|economy|timeline` and, behind the same write rules as above, `POST /api/humans` and `POST /api/control` (both answer `202`: the change applies at the next tick).

### What it needs

Measured on a 16 GB development machine, release build (`benchmarks/`):

| | Memory (peak) | Speed |
|---|---|---|
| Two founders, one simulated week | about 310 MB | 49 s |
| Two founders, one simulated year | about 400 MB | about 32 min |
| 1,000 people, first hours | about 600 MB | 115 ms per simulated minute |
| 5,000 people, first hours | about 1.0 GB | 0.5 s per simulated minute |
| People, settled (after about a month) | about 6.7 MB each | so roughly 1,000 people on 8 GB and 2,000 on 16 GB |
| Snapshot file, two people | 0.05 GB (compressed) | save about 2 s, load about 2 s |
| Snapshot file, 5,000 people | about 0.9 GB measured before compression, so expect roughly 0.13 GB | save 30 s, load 10 s before compression |

An 8 GB machine is comfortable for the founders and for hundreds of people; thousands fit only for short runs (see the settled figure above). Compiling is the heavy part: `rustc` can use 4 GB or more, so on a small or busy machine build with `cargo build -j 1`, and keep large files out of `/tmp` where it is RAM-backed.

Five simulated years of the two founders ran in a flat 250-320 MB and ended with five people alive (three children), (that run predates booking each person's meals and drinks at the cell where they stand).

Limits worth knowing (full list in [`docs/ISLAND_SYSTEM_STATUS.md`](docs/ISLAND_SYSTEM_STATUS.md)): people work and eat through a simplified household model, not yet through timed, energy-costed tasks; children grow along WHO curves, eat by body size and stay with their mother until five, and children do not do adult work or the mature actions (round-number age limits), but they still choose their own other actions; there is no refinery, town or deep-time spin-up yet; populated runs have been measured for hours, not for generations.

## Source authority

The extraction is pinned by provenance to Maer-Ken commit `7c05f0dcf254387ffd7322dbb525fe4807228602`. See `UPSTREAM.md` before changing imported human behaviour.
