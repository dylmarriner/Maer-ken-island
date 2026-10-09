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

It draws the island, and CI checks that on every push by rendering a
frame on a virtual screen and asserting the picture is of something. What
has **not** been established is how it feels with a mouse, or what it
costs on a GPU: every frame so far was rasterised by lavapipe on the CPU,
and no frame-rate figure is claimed. `docs/island/RENDER_STACK.md` is the
full account.

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
