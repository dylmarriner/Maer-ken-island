# Island Phase 0b: Early Human Creator Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let the owner create complete humans from a browser dashboard, each stored in their own folder, before the island world exists. This is the first slice of Phase 4 Task 6. The spawn path, validation, folder layout and `CreateHumanRequest` fields carry forward; Phase 4 extends the request with a location and changes `POST /api/humans` from immediate `201` to queued `202`, and Phase 5 Task 7 moves this code out of `apps/island_humans` before that crate is retired.

**Architecture:** Reuse upstream's creator path rather than writing a new one. Upstream creates authored humans through `InterventionAction::SpawnHuman` → `spawn_human` (`crates/mk_engine/src/interventions/mod.rs`), which validates with `mk_interventions::validate_intervention`, builds the person with `HumanBeing::sampled` from an RNG stream keyed by agent id and tick, applies the authored appearance, and adds them with `HumanRegistry::add_human` (which creates their folder through `HumanStorage`). That function takes a planetary `WorldState`; Task 1 extracts its human-building core so the island population can call it without a world. The dashboard ports upstream's `mk serve` server shape (warp + tokio, `ControlAuth`) and adds the Creator page upstream's browser dashboard never had. Upstream's only creator that spawns people is the `mk_studio` Bevy foundry panel (`apps/mk_studio/src/ui/foundry_panel.rs`), which submits `SpawnHuman`; `mk_ui`'s human foundry only saves templates.

**Tech Stack:** Rust 2021, `mk_engine`, `mk_interventions`, `island_humans`, warp 0.4 + tokio 1 (already in `[workspace.dependencies]`, unused until now), plain HTML/JS embedded with `include_str!`.

**Spec:** `docs/superpowers/specs/2026-10-01-maer-ken-island-regional-world-design.md` §4.6.

**Depends on:** Phase 0 Task 4b (a storage failure must never lose a human). Runs before Phase 1.

## Scope

In:
- Create humans from the dashboard with the upstream authored-profile fields: name, sex, birth date/time (RFC 3339), birthplace latitude/longitude, current age, height, build, hair colour, eye colour, skin tone.
- Every human — Gem-D, Gem-K and everyone created — has their own folder in the upstream `HumanStorage` layout; created humans get theirs at creation, with a `created` event.
- A roster and a per-human detail view in the dashboard.
- Restarting the dashboard on the same data directory reloads everyone from their folders.

Out (arrives later — say so in the UI):
- Placing a human somewhere on the island (Phase 4 Task 6, after the island exists). Here, location is the birthplace only.
- Time passing. Humans are created and stored but are not stepped; they do not age, act or have children until `IslandWorldState` exists (Phase 4). The dashboard states this plainly.

## Global Constraints

- One creator path. The dashboard, the existing `island_humans create` CLI and (later) Phase 4/5 creators all call the same `create_human` function.
- Validation is upstream's: `mk_interventions::validate_intervention` on the equivalent `SpawnHuman` action (name has a letter/digit, age 0–130, height 40–272 cm, non-empty appearance fields, RFC 3339 birth time, valid birthplace). Island-only checks are added on top, never instead.
- Agent ids follow upstream `agent_id_for_name`: lower-case alphanumeric slug of the name; a taken slug gets `-2`, `-3`, … Duplicate names are allowed, not rejected.
- Same seed + same ordered creation requests ⇒ byte-identical humans. The dashboard records each accepted request, in order, in `<data-dir>/creations.jsonl`.
- Disk failure never changes who exists: a human is added to the population even if their folder write fails; the failure is returned to the caller and shown in the UI.
- Write endpoints follow upstream `ControlAuth`: a configured `ISLAND_CONTROL_TOKEN` requires `Authorization: Bearer <token>`; with no token, writes are allowed only when bound to a loopback address; a non-loopback bind without a token refuses all writes.

## Review Focus

- Extracting the spawn core must not change any existing `interventions::` test result.
- Path traversal: names become directory names only through the slug, and `HumanStorage::validate_id` still applies.
- The page must escape every human-supplied string it renders.
- No endpoint may mutate state on GET.

---

### Task 1: Extract upstream's spawn core

**Files:**
- Create: `crates/mk_engine/src/humans/spawn.rs`
- Modify: `crates/mk_engine/src/humans/mod.rs` (declare module)
- Modify: `crates/mk_engine/src/interventions/mod.rs` (`spawn_human`, `agent_id_for_name`)
- Modify: `UPSTREAM.md` (divergence entry)
- Test: unit tests in `spawn.rs`

**Interfaces:**
- Produces: `pub fn agent_id_for_name(registry: &HumanRegistry, name: &str) -> String` (moved from `interventions`, same behaviour).
- Produces: `pub fn build_authored_human(rng: &RngRegistry, tick: Tick, agent_id: String, sex: BiologicalSex, profile: &HumanSpawnProfile) -> Result<HumanBeing, SpawnHumanError>` containing exactly the part of `spawn_human` from the RNG stream (`SubsystemId::Humans`, `SPAWN_HUMAN_EPOCH`) through `HumanBeing::sampled`, the authored body/appearance fields and `refresh_phase11_layers()`.
- `interventions::spawn_human` keeps its signature and calls these, then still places the human at the requested cell and calls `add_human`.

- [ ] **Step 1:** Run `cargo test -p mk_engine --lib interventions::` and record the passing set.
- [ ] **Step 2:** Write tests: `build_authored_human` is byte-identical for identical inputs and differs for a different tick or agent id; `agent_id_for_name` returns `sam`, then `sam-2`, then `sam-3` as humans are added.
- [ ] **Step 3:** Move the code; re-run Step 1 and the new tests; expect the same passing set plus the new tests.
- [ ] **Step 4:** Record the extraction in `UPSTREAM.md` as an island divergence suitable for upstreaming.
- [ ] **Step 5:** Commit `refactor(humans): extract the authored-spawn core from interventions`.

### Task 2: Population with folders and a creator

**Files:**
- Modify: `apps/island_humans/src/lib.rs`
- Modify: `apps/island_humans/Cargo.toml` (add `mk_interventions`)
- Modify: `apps/island_humans/src/main.rs` (CLI `create` uses the new path)
- Test: `apps/island_humans/tests/creator.rs`

**Interfaces:**
- Produces: `IslandHumanPopulation::open(data_dir: &Path, seed: [u8; 32]) -> Result<Self, PopulationError>` — opens `<data-dir>/humans` with `HumanStorage::try_new` (Phase 0 Task 4; an invalid key is an error, never a silent plaintext fallback), reloads every stored human (`HumanRegistry::with_storage`), and adds Gem-D/Gem-K with folders only if absent (`HumanRegistry::seed_founders`). `seed` is persisted in `<data-dir>/population.json` on first open and must match on later opens.
- Produces: `CreateHumanRequest { name, biological_sex, birth_timestamp, birth_latitude, birth_longitude, age_years, height_cm, build, hair_color, eye_color, skin_tone }` (serde; `biological_sex` is `"male" | "female"`, the two templates upstream `spawn_human` supports).
- Produces: `IslandHumanPopulation::create_human(&mut self, request: CreateHumanRequest) -> Result<CreatedHuman, CreateHumanError>` where `CreatedHuman { summary: HumanSummary, storage_error: Option<String> }` and `CreateHumanError::Invalid(Vec<String>)` carries upstream's validation messages.
- The creation tick is the population's creation counter (0, 1, 2, …), persisted with the seed, so ordering alone determines the RNG stream.
- After creating, write a `created` event (`{"kind":"created","by":"dashboard"|"cli","counter":n}`) and append the request to `creations.jsonl`. Write the event through `HumanStorage::record_event` directly (not `HumanRegistry::record_event`, which discards errors) so a failure lands in `storage_error`.
- Sensitive files are encrypted by `HumanStorage::new`, which creates `<data-dir>/humans/.secret_storage_key` unless `MK_STORAGE_KEY` is set. Losing that key makes those files unreadable.
- The existing in-memory `IslandHumanPopulation::with_founders()` / `empty()` stay for tests.

- [ ] **Step 1:** Write tests (temp dirs): first open creates Gem-D/Gem-K folders; `create_human` creates a folder with `profile/`, `traits/`, `cognition/`, `social/`, `development/`, `reproduction/`, `memories/{episodic,semantic,procedural}/`, `state/`, `relationships/`, `events/` and a `created` event; reopening reloads the same humans; each upstream validation rule rejects with its message; two "Sam"s become `sam` and `sam-2`; replaying `creations.jsonl` into a fresh directory with the same seed yields identical profiles; a read-only `humans/` directory still adds the human and returns `storage_error`.
- [ ] **Step 2:** Run `cargo test -p island_humans --test creator`; expect FAIL.
- [ ] **Step 3:** Implement on `HumanRegistry::with_storage` and Task 1's functions; no new folder format.
- [ ] **Step 4:** Point the CLI `create` command at `create_human` with `--data-dir` (default `./island-data`), keeping its argument order and adding the appearance fields as optional flags with upstream defaults.
- [ ] **Step 5:** Re-run; expect PASS. Commit `feat(island): create humans with their own folders`.

### Task 3: Dashboard server

**Files:**
- Create: `apps/island/Cargo.toml` (binary `island`; deps `island_humans`, `mk_engine`, `mk_interventions`, `serde`, `serde_json`, `warp`, `tokio`; dev-dep `warp` with `test` feature as upstream `mk_cli`)
- Create: `apps/island/src/main.rs`, `apps/island/src/serve/{mod.rs,server.rs,handlers.rs,auth.rs}`
- Modify: `Cargo.toml` (workspace member)
- Test: `apps/island/src/serve/server.rs` (`#[tokio::test]`, as upstream)

**Interfaces:**
- CLI: `island serve [--data-dir ./island-data] [--seed <64 hex chars>] [--bind 127.0.0.1:8080]`.
- `auth.rs`: port of upstream `ControlAuth { BearerToken, LoopbackOnly, Disabled }` with `resolve(token, bind_ip)` and `permits(header)`.
- `GET /api/status` → `{ population, data_dir, seed_hex, time_running: false, notes: ["Humans are not stepped until Phase 4"] }`.
- `GET /api/humans` → roster `[HumanSummary]`; `GET /api/humans/<agent_id>` → detail sections serialized from the stored `HumanBeing` (identity, birth chart, body/appearance, personality/temperament, drives, attachment, development, reproduction) plus the human's folder path.
- `GET /api/creator/options` → the selectable values for sex, build, hair, eye and skin (copy the option lists from upstream `apps/mk_studio/src/ui/foundry_panel.rs`) and the numeric limits (`MAX_SPAWN_AGE_YEARS`, `SPAWN_HEIGHT_RANGE_CM`).
- `POST /api/humans` (auth) → `201 { summary, storage_error }`, `401` unauthorised, `422 { errors: [..] }` invalid.
- The population sits behind a `Mutex`; one request mutates at a time, in arrival order.

- [ ] **Step 1:** Write server tests: GETs work unauthenticated; `POST` without a token under `BearerToken` is 401; under `LoopbackOnly` it succeeds; under `Disabled` it is refused; a valid `POST` is listed by the next `GET /api/humans` and its folder exists; an invalid `POST` returns 422 with upstream's messages; `GET /api/humans/../x` cannot escape the data directory.
- [ ] **Step 2:** Run `cargo test -p island`; expect FAIL.
- [ ] **Step 3:** Implement, following upstream `apps/mk_cli/src/web/server.rs` structure.
- [ ] **Step 4:** Re-run; expect PASS. Commit `feat(island): dashboard api with human creation`.

### Task 4: Dashboard pages

**Files:**
- Create: `apps/island/static/{index.html,dashboard.js,creator.js,style.css}` (embedded with `include_str!`, served at `/`, `/creator`, `/static/*`)
- Test: `apps/island/tests/static_pages.rs`

**Interfaces:**
- **People** (`/`): roster (name, sex, age, status) with a detail panel showing the sections from `GET /api/humans/<id>`; a banner states that time is not running yet.
- **Create a human** (`/creator`): form with name, sex, birth date and time, birthplace latitude/longitude (default: Gem-D's recorded birthplace until the island exists), current age, height, build, hair, eye and skin dropdowns from `/api/creator/options`; optional token field stored in `sessionStorage`; server validation errors shown next to the form; on success, the new person opens in the People view with their folder path, and any `storage_error` is shown as a warning.
- All human-supplied text is inserted with `textContent`, never `innerHTML`.

- [ ] **Step 1:** Write tests that each page and asset is served with the right content type and that `dashboard.js`/`creator.js` contain no `innerHTML` assignments.
- [ ] **Step 2:** Implement the pages; reuse upstream `apps/mk_cli/static/dashboard` styling where it fits.
- [ ] **Step 3:** Run `cargo test -p island`; expect PASS.
- [ ] **Step 4:** Manual check: `cargo run -p island -- serve`, open `http://127.0.0.1:8080/creator`, create a person, confirm they appear in People and that `island-data/humans/<id>/` exists with a `created` event; restart and confirm they are still listed.
- [ ] **Step 5:** Commit `feat(island): people dashboard and human creator page`.

### Task 5: Gate 0b

- [ ] **Step 1:** Run fmt, clippy `-D warnings` and the fast test tier; expect green.
- [ ] **Step 2:** Update `README.md` with `island serve` usage, the "time is not running yet" limitation, and a warning to back up `island-data/humans/.secret_storage_key` (or set `MK_STORAGE_KEY`), because losing it makes encrypted human files unreadable.
- [ ] **Step 3:** Tick Gate 0b in the program plan.
