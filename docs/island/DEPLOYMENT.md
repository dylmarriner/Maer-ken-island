# Running the island on more than one machine

The island used to be one program. It is now a **backend** that simulates
and serves, and **frontends** that read it: a web dashboard that can be
hosted anywhere, and a desktop application. This is how to put them on
different computers.

Everything below has been run. Where a figure appears it was measured, and
where something has not been verified it says so.

## The shape of it

```
            ┌──────────────────────────────┐
            │  island serve --scenario …   │   the backend: one island,
            │  --bind 0.0.0.0:8080         │   one simulation thread,
            │  --allow-origin https://…    │   one HTTP server
            └──────────────┬───────────────┘
                           │  HTTP, one schema (crates/mk_island_api)
         ┌─────────────────┼──────────────────┐
         │                 │                  │
┌────────▼────────┐ ┌──────▼───────┐ ┌────────▼─────────┐
│ island serve    │ │  island-ui   │ │ any static host  │
│ --frontend-only │ │  --server …  │ │ (island export)  │
│ a web server    │ │  a desktop   │ │ the same pages   │
│ for the pages   │ │  application │ │ as plain files   │
└─────────────────┘ └──────────────┘ └──────────────────┘
```

There is **one** way to read an island and every frontend uses it. Even
`island-ui --scenario`, which runs an island in its own process, starts the
same backend on a loopback port and reads it through the same client — so
the local case is proven by the remote one and there is no second reader to
drift.

## The backend

```bash
export ISLAND_CONTROL_TOKEN="$(openssl rand -hex 32)"   # who may change it
export ISLAND_READ_TOKEN="$(openssl rand -hex 32)"      # who may look at it
island serve \
  --scenario fixtures/island/default_scenario.json \
  --bind 0.0.0.0:8080 \
  --allow-origin https://island.example \
  --snapshot-dir /var/lib/island/snapshots \
  --log /var/lib/island/commands.jsonl \
  --data-dir /var/lib/island/people
```

### The two tokens answer two questions

| Variable | Question | Without it |
|---|---|---|
| `ISLAND_CONTROL_TOKEN` | Who may **change** the island? | Writes are accepted on a loopback bind and refused on any other. |
| `ISLAND_READ_TOKEN` | Who may **look at** it? | Reads are open to anyone who can reach the port. |

Reads being open is right on a loopback bind — exactly as open as the
machine — and is a decision anywhere else. **A backend bound to `0.0.0.0`
with no read token can be read in full by anyone who can reach the port,
every person's complete record included**, and it says so on startup rather
than leaving that to be discovered.

The control token counts as a read token, so an operator carries one secret
rather than two.

Two endpoints answer without any token, because they are what a caller asks
*before* it can have one: `/api/health` (and `/healthz`) so a monitor can
find out the process is alive, and `/api/version` so a client can find out
whether this server still speaks its schema. Neither says anything about
the island.

### `--allow-origin`

Only a browser cares. `island-ui` and `curl` send no `Origin` and are not
bound by one.

Give it exactly as a browser sends it — scheme, host and port, no path, no
trailing slash — and repeat it for more than one:

```bash
--allow-origin https://island.example --allow-origin http://192.168.1.20:3000
```

Named none, the backend sends no cross-origin header at all, which is what
a server that never heard of CORS does and leaves the browser's own
same-origin rule in charge. That is correct when the backend serves its own
dashboard.

### Behind TLS

`island serve` speaks HTTP. Put it behind a reverse proxy that terminates
TLS, and give `--allow-origin` the **public** origin a browser will see,
not the proxy's internal one.

```nginx
location / {
    proxy_pass http://127.0.0.1:8080;
    proxy_set_header Host $host;
    proxy_set_header X-Forwarded-Proto $scheme;
}
```

`/api/elevation.bin` is 5.8 MB and already gzipped by the backend when a
caller offers it. Do not re-compress it — `gzip off` for that location, or
accept paying twice for nothing.

### As a service

`deploy/island.service` is a systemd unit; `deploy/Dockerfile` builds the
backend into a container. Both read the same variables as above.

## The web dashboard, somewhere else

Two ways, and they produce the same pages.

**A frontend process**, for a machine that can run the binary:

```bash
island serve --frontend-only \
  --backend https://island.example \
  --bind 0.0.0.0:3000
```

It opens no data directory and bootstraps no world. Every request the pages
make goes to `--backend`.

**Plain files**, for anything that serves static content:

```bash
island export --to /var/www/island --backend https://island.example
```

Fifteen files. Each page is written twice — `people.html` and
`people/index.html` — because hosts disagree about which one `/people`
means, and that is the difference between a frontend that works anywhere
and one that works on half the hosts.

Either way the island's address lands in a generated `static/config.js`
rather than in the pages, so the pages are identical bytes whoever serves
them. A reader can also point the browser elsewhere through the server
control at the foot of every page; the islands they may choose from are
`--backend` plus any `--allow-backend`, because the page's own
content-security policy has to name them.

**The backend must name the frontend's origin in `--allow-origin`** or a
browser will refuse every request the pages make. This is the single most
common way to get a split deployment wrong, and the page says so when it
happens.

### Verified

Driven in Chromium against two processes, and again against
`npx http-server` serving an export: the overview, roster, island and
creator pages all load with no console errors and no failed requests;
arriving with no token shows the island's own words and opens the token
field; entering it connects; and the map draws the real island — 4,604
distinct colours over 2,401 km — from both.

## The desktop application

```bash
island-ui --server https://island.example --token "$ISLAND_READ_TOKEN"
```

Or with an island in its own process:

```bash
island-ui --scenario fixtures/island/default_scenario.json --speed 60
```

It opens on the island. `--view estate` opens on the founders' estate
instead — the same application, the other frame, switchable from the panel
at any time:

```bash
island-ui --server https://island.example --view estate
```

The estate is drawn from the island's own geometry, which is why it draws
at all on a machine with no `assets/` directory: every building, room and
thing is a box at the simulation's metres, and the Maer-Ken models hang on
top of that rather than standing in for it. `--assets DIR` says where they
are when they are not beside the binary.

It draws both, and CI checks both on every push by rendering each frame on
a virtual screen and asserting the picture is of something. The estate's
check reads the application's log as well, because a photograph cannot
tell a loaded model from the box underneath it: measured, an empty
`assets/` takes that frame from 3,457 colours to 2,172, which the picture
alone would not have failed, while the log names thirteen missing models.

A mouse and a keyboard are answered: five gestures are driven against the
running application and measured, from a held drag that moves 39.8% of the
pixels to a panel click that moves 59.5%.

What has **not** been established is what it costs on a GPU: every frame so
far was rasterised by lavapipe on the CPU, and no frame-rate figure from
real hardware is claimed. `docs/island/RENDER_STACK.md` is the full
account.

### The two claims this repository cannot check itself

Everything above was verified inside one container, which has no GPU and
one network interface. Two things therefore stand unverified, and neither
can be settled by any amount of further work in that container. They are
written out here as procedures rather than caveats, so that whoever has
the hardware can close them in an afternoon and write the numbers down.

**Does it use the GPU, and what does a frame cost?** The application asks
for the high-performance adapter in its own code (`wants_the_best_gpu`),
and says at startup which one it actually got. On a machine with a
graphics driver:

```sh
island-ui --scenario fixtures/island/default_scenario.json --measure 30
island-ui --scenario fixtures/island/default_scenario.json --view estate --measure 30
```

The startup line names the adapter. If it says `drawing on the CPU` the
machine has no driver and the figures are meaningless; otherwise
`--measure` prints frames per second after a sixty-frame warm-up. The
figures to beat are the software ones: **0.6 fps** on the island view and
**10.8** on the estate, at 1600x1000 under lavapipe. `--detail 4` is the
knob if the island view is still slow.

**Does it run across separate computers?** The parts that make this
possible are tested — the backend binds any address, enforces a read
token and an origin allow-list over the wire, and `--frontend-only
--backend <url>` generates a `config.js` naming the backend rather than
baking it in. What one container cannot show is the topology. On two or
more machines, with the backend on A and the frontends on B and C:

```sh
# on A
ISLAND_READ_TOKEN=... ISLAND_CONTROL_TOKEN=... island serve \
  --scenario ... --bind 0.0.0.0:8120 --allow-origin http://B:3010
# on B
island serve --frontend-only --backend http://A:8120 --bind 0.0.0.0:3010
# on C
island-ui --server http://A:8120 --token ... --view estate
```

Expect, and check: `/api/version` answering without a token, `/api/world`
refusing without one and answering with it, an origin that is not B
getting a 403, `window.ISLAND_BACKEND` on B naming A, and C logging
`Reading the island at http://A:8120` before it draws. Those exact checks
pass here between processes over this container's own interface; what a
second machine adds is the one thing a single host cannot fake.

On a machine with no screen it says so in a sentence and exits 1, after
telling you whether the backend was reachable — so a headless operator
still gets an answer about the connection.

## The computer service (opt-in, and off)

`services/computer-service` is a Node service that lets a founder at their
own machine in a powered computer room search the web and send email **for
real**. It is the only thing on this island that can see out.

```bash
cd services/computer-service && npm ci && cp .env.example .env   # then fill it in
node src/index.js

COMPUTER_ACTIONS_ENABLED=1 COMPUTER_SERVICE_URL=http://127.0.0.1:8002 \
  island serve --scenario …
```

**This makes the island irreproducible, on purpose.** Two runs of the same
seed will not agree once a human has used it. That is the feature. It is
why nothing attaches it unless `COMPUTER_ACTIONS_ENABLED=1`, why `island
run`, `island replay` and `island inspect` never attach it at all — so a
log replays identically whether or not a live human sent real email in the
run that produced it — and why the backend says so loudly at startup when
it is on.

Three things are tested in `crates/mk_engine/tests/island_computer_service.rs`:
off, the island is exactly the island it was; on but failing, likewise,
because the costs are charged on success only; and the affordance is
offered where it should be. Measured end to end: with the service running
and the variable set, `/api/version` reports `computer_service: true`; with
neither, `false` and no mention of it at startup.

Credentials live in `.env`, which is gitignored and must stay that way.

## Checking a deployment

```bash
curl -s https://island.example/api/health                      # no token needed
curl -s https://island.example/api/version | jq .capabilities  # what it can do
curl -s -H "Authorization: Bearer $ISLAND_READ_TOKEN" \
     https://island.example/api/world | jq .clock              # is it ticking
```

`capabilities` is what a frontend reads to decide what to offer rather than
offering everything and letting half of it fail:

| Field | Means |
|---|---|
| `world` | An island is running. `false` on a human-only bootstrap. |
| `writes` | `token`, `loopback` or `disabled`. |
| `reads_need_token` | A read token is set. |
| `allowed_origins` | Browser origins this backend accepts. |
| `elevation` | Terrain is served as numbers for a 3D client. |
| `computer_service` | A real bridge to the outside is attached. |
| `snapshots` | A snapshot directory was configured, so the button will work. |

### The whole shape, measured

Run against one backend on `0.0.0.0:8120` with both tokens and
`--allow-origin http://127.0.0.1:3010`, a dashboard on `127.0.0.1:3010`
started with `--frontend-only --backend`, and the desktop application
reading the backend with the read token:

| Asked | Answer |
|---|---|
| `GET /api/world`, no token | `401` |
| `GET /api/world`, read token | `200` |
| `GET /api/world/estate/layout`, read token | `200`, the five buildings in metres |
| `POST /api/control`, read token | `401` |
| `POST /api/control`, control token | `200` |
| `GET /api/version`, no token | `200` — a frontend must be able to find out it needs one |
| `GET /island` on the dashboard | `200` |
| `GET /static/config.js` on the dashboard | `window.ISLAND_BACKEND = "http://127.0.0.1:8120"` |
| `GET /api/world` with `Origin: http://127.0.0.1:3010` | `200`, `access-control-allow-origin` set |
| `GET /api/world` with any other `Origin` | `403`, naming `--allow-origin` |
| `island-ui --server … --token … --view estate` | draws the estate, 52 models, clock at 60.0× of the 60× asked for |

Three processes, three machines' worth of separation, one island.

**The shape is re-proved whenever the wire changes**, because the wire is
the only thing holding those three processes together and a schema that
parses locally can still be wrong across a network. Re-run after the
conversation and dimension fields were added, against separately built
backend and desktop binaries:

| Asked of the backend over HTTP | Answer |
|---|---|
| `/api/world/estate/layout` | 84 items, **84 of 84 carrying a real size**; 5 buildings, **5 of 5 carrying a real height**, each with its source |
| `/api/conversations`, after the island had run to tick 444 | **one** exchange spanning **ticks 1-420** — not 420 conversations |
| the same exchange's lines | **840 said, 12 sent**, and `lines_said` never smaller than the lines it was sent with |
| `island-ui --server … --view estate` | "the estate: 5 buildings, 14 rooms, 84 things (52 with models of their own)", frame at 3,458 colours |

That `840 said, 12 sent` is the whole of D35's second half and its payload
bound, working between two processes rather than inside one. Before the
merge the same stretch of talking was 420 separate conversations of two
lines each.

Version skew is checked separately and does not need a running island:
`crates/mk_island_api/tests/version_skew.rs` reads hand-written payloads
from older backends and from a backend newer than the types, because
frontends and backends are not upgraded at the same moment.

## What the backend costs

Measured on the development machine, from `benchmarks/`:

| | |
|---|---|
| One step of the real island | **4.3 ms** |
| A simulated week (physics, ecology, 200,000 trees, founders, energy, materials) | 49 s, **307 MB** peak |
| A complete snapshot | 44 MB on disk, **27 s** during which the island does not step |
| `/api/elevation.bin` | 5.8 MB raw, built once, served immutable |

Forty concurrent readers on one backend were measured at 200 with no
errors. Each frontend polls four times a second, so the load is a function
of how many screens are open rather than of how fast the island runs.

**Back up `island-data/humans/.secret_storage_key`.** Human records are
encrypted with it. Lose it and every record becomes unreadable. Back it up
with the data, or set `MK_STORAGE_KEY` yourself.
