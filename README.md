# Maer-ken-island

Standalone island project bootstrapped from the human simulation work in `dylmarriner/Maer-Ken`.

This repository carries forward the canonical Maer-Ken human schema/runtime and exposes it as a public API for island-specific simulation work.

## Implementation roadmap

The top-level execution map is [`docs/IMPLEMENTATION_READY_ROADMAP.md`](docs/IMPLEMENTATION_READY_ROADMAP.md).

It defines the locked scope (a uniquely shaped, New-Zealand-sized island on an Earth-like Marr'Kena, simulated realistically), the phase order (0, 0b, 0c, 1–4, 4b, 5), the agent execution contract, acceptance gates, and links to the nine detailed implementation plans under `docs/superpowers/plans/`.

## What is here

The island is no longer one program. It is a **backend** that simulates
and serves, and **frontends** that read it — a web dashboard that can be
hosted anywhere, and a desktop application. They share one wire schema and
there is exactly one way to read an island.

### The island itself

- `crates/mk_core/src/human/` — canonical identity, genetics, temperament, neurocognition, personality, drives, hormones, attachment, schema and profile types.
- `crates/mk_engine/src/humans/` — live runtime systems for body, needs, cognition, emotion, memory, consciousness, reproduction, lifecycle, social behaviour and brain-state models.
- `crates/mk_engine/src/regional/` — the island: geophysics, climate, ecology, the estate, materials, energy, the humans on it.
- `crates/mk_island/` — the regional domain contract.
- `fixtures/human/` — canonical Gem-D / Gem-K human fixtures.
- `docs/canon/` — the copied HumanReplicationSchema and human schema-family authority.

### The backend, and what reads it

- `apps/island/` — `island serve`, `run`, `replay`, `inspect` and `export`. The simulation, the HTTP API and the dashboard's pages.
- `crates/mk_island_api/` — every shape that crosses the network, defined once. Serde and nothing else, so a client can link it without building the island.
- `crates/mk_island_client/` — a typed, blocking client for a backend. What the desktop application uses, and what anything else should.
- `apps/island_ui/` — `island-ui`, the desktop application, on Bevy 0.19.
- `apps/island_humans/` — the Phase-0b population bootstrap and CLI.
- `apps/island_preview/` — the headless renderer for the geophysics galleries.
- `services/computer-service/` — the opt-in Node bridge that lets a founder reach the real internet. Off unless asked for.

### Assets

- `assets/humans/` — the founder GLB models.
- `assets/property/`, `assets/computers/`, `assets/tools/`, `assets/vehicles/`, `assets/terrain/` — the estate's buildings, the four machines in the computer room, the workshop equipment, the named vehicles and the terrain materials, from upstream Maer-Ken. `assets/CREDITS.md` says exactly what they are, which is less than it would be convenient to claim.
- `tools/blender/`, `tools/assets/` — the generators they came from, so they can be rebuilt rather than only trusted.

## Current bootstrap

A stored population lives in a data directory (default `./island-data`). Every human, Gem-D (`HUM-000001`) and Gem-K (`HUM-000002`) included, has their own folder under `island-data/humans/<agent-id>/` with `profile/`, `traits/`, `cognition/`, `social/`, `development/`, `reproduction/`, `memories/`, `state/`, `relationships/` and `events/`.

New people are built through upstream's `SpawnHuman` path from the population's seed and a creation counter. Every creation is appended to `island-data/creations.jsonl`, so replaying that log with the same seed reproduces the same people.

## Run it

```bash
cargo test --workspace --exclude island_ui    # fast tier
cargo run -p island -- serve --scenario fixtures/island/default_scenario.json
cargo run -p island_humans -- founders        # list everyone (CLI)
```

### On one machine

`island serve --scenario …` is the whole thing: the island, its API and its
dashboard at <http://127.0.0.1:8080/>. This is unchanged and still the
simplest way to run it.

- **Overview** (`/`): how many people there are, how old they are, where their records live, what the island's clock is doing, and what does and does not work yet.
- **People** (`/people`): the roster and everything stored about whoever you pick, with the complete record underneath.
- **The island** (`/island`): the map — elevation or standing vegetation across the whole island, the estate's patch 400× finer, and the individual stems in the wood around it — what has been built, and everything that has reached the island from outside.
- **Create a human** (`/creator`): name, sex, birth date and place, age and appearance.

### On several machines

The backend on one, the frontends on others:

```bash
# the island
export ISLAND_CONTROL_TOKEN=…   # who may change it
export ISLAND_READ_TOKEN=…      # who may look at it
island serve --scenario … --bind 0.0.0.0:8080 --allow-origin https://island.example

# the dashboard, somewhere else
island serve --frontend-only --backend https://island.example --bind 0.0.0.0:3000
#   …or as plain files for any web server:
island export --to /var/www/island --backend https://island.example

# the desktop application, on somebody's desk
island-ui --server https://island.example --token "$ISLAND_READ_TOKEN"
#   …opening on the founders' estate rather than on the island
island-ui --server https://island.example --view estate
```

`docs/island/DEPLOYMENT.md` is the full account: the two tokens and what
each is for, CORS, TLS, the systemd unit and the container, and how to
check a deployment.

`island-ui` draws two things, and CI proves both: the island at ten
kilometres to the unit, and the founders' estate at one metre to it, with
the Maer-Ken models standing on it. `scripts/render-smoke.sh` runs the
application on a virtual screen against a real island, photographs each
frame and asserts the picture is of something; both frames are uploaded as
artifacts. For the estate it reads the log as well, because a photograph
cannot tell a loaded model from the box underneath it — measured, an empty
`assets/` directory costs that frame a third of its colours and the log
names thirteen models it could not find.

What none of that answers is how it feels with a mouse or what it costs on
a GPU, because every frame so far was rasterised on the CPU.
`docs/island/RENDER_STACK.md` is the full account.

### The JSON behind it all

`/api/status`, `/api/version`, `/api/health` (also `/healthz`),
`/api/world` and its parts, `/api/world/humans`, `/api/properties`,
`/api/economy`, `/api/timeline`, `/api/conversations`, `/api/map.png`,
`/api/vegetation.png`, `/api/patch.png`, `/api/elevation.bin`,
`/api/cell/<row>/<col>`, `/api/trees`, `/api/humans`, `/api/activity` and
`/api/creator/options`. The writes are `POST /api/humans`,
`/api/world/humans`, `/api/world/interventions` and `/api/control`.

Every response carries a strict content security policy and the usual
hardening headers; the pages load no third-party script, style or font.
One line per request is printed to stdout — set `ISLAND_ACCESS_LOG=off` to
stop that.

### Who may do what

Two tokens, answering two questions. `ISLAND_CONTROL_TOKEN` decides who may
**change** the island: set it and every write needs
`Authorization: Bearer <token>`; without it only a loopback bind accepts
writes at all. `ISLAND_READ_TOKEN` decides who may **look**. Reads are open
without it, which is right on a loopback bind and a decision anywhere else
— a backend bound to `0.0.0.0` with neither is readable in full by anyone
who can reach the port, and it says so on startup.

`/api/health` and `/api/version` answer without either, because they are
what a caller asks before it can have one.

**Back up your storage key.** Human files are encrypted with
`island-data/humans/.secret_storage_key`, generated on first run, or with
`MK_STORAGE_KEY` if set. Lose the key and every human file becomes
unreadable. Back it up with the data, or set `MK_STORAGE_KEY` yourself.

## Source authority

The extraction is pinned by provenance to Maer-Ken commit `7c05f0dcf254387ffd7322dbb525fe4807228602`. See `UPSTREAM.md` before changing imported human behaviour.
