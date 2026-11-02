# Island engineering continuation (2026-11-01)

Canonical repo: `dylmarriner/Maer-ken-island` (not the separate Maer-Ken repository). Branch `fix/island-client-human-id-20261101`, PR #68. Source-of-truth: `WORK_REMAINING.md`, `HANDOFF.md`, `docs/IMPLEMENTATION_READY_ROADMAP.md` and cited reference packs.

## This iteration
- Blocking HTTP client now limits successful JSON bodies to 16 MiB, binary map/terrain/cover responses to 64 MiB and error responses to 16 KiB. The stream reader detects overflow even without a trustworthy Content-Length. No simulation or canonical tick logic changed.
- New boundary and UTF-8 regression tests. On OpenCommander `amd`: `cargo test -q -p mk_island_client --lib --offline --locked` **8/8 passed**; `cargo clippy -q -p mk_island_client --all-targets --offline --locked -- -D warnings`, `cargo fmt --all`, and `git diff --check` passed.
- This is client resource-exhaustion hardening, not evidence that long-running world simulation is complete.

## Remaining acceptance checklist
- [ ] Exact-SHA PR #68 full CI green.
- [ ] Nightly week/year simulation acceptance, material/water/carbon ledgers and deterministic replay hashes verified.
- [ ] Sourced realism and industry tasks in `WORK_REMAINING.md` implemented with reference packs, deviation records and replay evidence.
- [ ] No owner-governed changes or upstream divergence without required approvals.

Next: run the slow acceptance suite in an isolated environment, then select the next unblocked sourced simulation slice. Keep Rust single-writer and append-only replay invariants.

## 2026-11-01 continuation: reject stale cell-inspector responses
- Prior exact-SHA [CI #41286409716](https://github.com/dylmarriner/Maer-ken-island/actions/runs/41286409716) passed all non-nightly jobs on `dfda99a`; slow week/year tests were skipped by design.
- Web map inspector now gives each click a monotonically increasing request generation. A slow earlier `/api/cell` response, an older `/api/world` selection refresh, or a late request error cannot overwrite the latest selected cell/intervention location. This changes read-only dashboard state only; no Rust simulation tick, ledger, snapshot or replay path changed.
- Evidence on OpenCommander `amd`: `node --test apps/island/tests/cell-details.test.mjs` **5/5 passed** including two new out-of-order selection tests; `node --check` on both changed modules, `cargo fmt --all -- --check`, and `git diff --check` passed. Full Rust/server acceptance not rerun locally.
- Remaining: [ ] exact-new-SHA CI; [ ] nightly week/year deterministic replay and material/water/carbon ledgers; [ ] sourced ecology/industry tasks from `WORK_REMAINING.md` (including serving species without inventing densities); [ ] final owner acceptance.
- Next: verify CI and execute the slow simulation acceptance suite before claiming completion. Do not confuse map correctness with simulation realism.

## 2026-11-01 continuation: prevent cross-island token forwarding

- The browser previously stored a single session control token independent of backend origin. An untrusted backend query link could silently switch the API target and cause all three fetch helpers to forward the existing owner's bearer token to that host. This is a credential-disclosure risk.
- Backend URLs now require a bare HTTP(S) origin without credentials, paths, queries or fragments. Tokens are scoped to the origin on which they were entered. An existing token is never sent to a different island, including when selected via query string; legacy unscoped tokens require re-entry. All three fetch helpers reject redirects. Invalid manual backend addresses display form validation.
- Added regression tests covering malicious query override, origin switching, legacy tokens, malformed addresses and redirect refusal; CI web dashboard runs the expanded suite.
- Verification: OpenCommander AMD node --test apps/island/tests/*.test.mjs **10/10 passed**, node --check apps/island/static/app.js, git diff --check passed before commit. Rust simulation tick and ledger code unchanged.
- Remaining: [ ] exact-SHA full CI; [ ] slow week/year ledger and replay acceptance; [ ] sourced ecology/industry roadmap; [ ] owner acceptance. Next: check CI, execute slow simulation suite, then continue sourced tasks.

## 2026-11-01 continuation: constrain dashboard API paths

- Shared browser apiUrl() now rejects scheme-relative/absolute, non-API, fragment, backslash and traversal paths before any JSON, command or image fetch can carry an owner token. Previously, with a same-host dashboard, a scheme-relative path could be resolved by the browser as a different origin if constructed from untrusted input. Cached backend origins are revalidated instead of trusted from localStorage.
- OpenCommander amd evidence: Node dashboard tests **13/13 passed**, JavaScript syntax, cargo fmt --all -- --check, git diff --check passed. Added three regressions for poisoned storage, malicious paths across all three fetch helpers and valid API queries. No Rust simulation logic, replay hashes or owner state changed.
- Remaining: [ ] exact-new-SHA CI; [ ] slow week/year replay, water/carbon/material ledgers and reference-pack acceptance; [ ] sourced ecology/industry roadmap, including serving species and real workforce data; [ ] owner review. Next: inspect the manual slow workflow and exact-commit CI, then advance an unblocked sourced simulation slice. Do not claim slow acceptance until it actually passes.

## 2026-11-02 continuation: avoid re-binding prefilled owner tokens when switching islands

- The server-connection form prefilled the current island's token. On a backend URL change, submitting without touching the password field called `rememberToken(oldToken)` after `setBackend(newOrigin)`, silently binding island A's credential to island B despite the earlier origin guard.
- The form now distinguishes an explicitly edited destination token from an untouched prefill, clears the old credential when switching origins, and preserves a token on same-origin reconnect. The help text explains re-entry. No Rust single-writer, ledger, snapshot, replay or simulation configuration changed.
- OpenCommander `amd` evidence: `node --test apps/island/tests/*.test.mjs` **15/15 passed**, `node --check apps/island/static/app.js`, `cargo fmt --all -- --check` and `git diff --check` passed.
- [ ] Exact-new-SHA CI green.
- [ ] Slow week/year deterministic replay and water/carbon/material ledger acceptance; manual slow run #41295467331 was still in progress at inspection.
- [ ] Remaining sourced ecology, industry and human runtime tasks in `WORK_REMAINING.md`; owner acceptance.

Next: inspect slow workflow completion and the exact-SHA CI, then take the next unblocked sourced simulation task. Never re-pin week digest without the full ledger and replay audit.

## 2026-11-02 continuation: reject stale People inspector responses

- Found that the People page guarded successful requests only by selected agent ID and did not guard errors. A late failure could replace the newer person's details with an obsolete error/retry button. A -> B -> A also defeated the ID-only guard, allowing the first A response to overwrite the newest A.
- Added a monotonic per-selection generation helper and wired both success and failure rendering through it. An empty roster invalidates pending requests. No simulation state, Rust single-writer, ledger, snapshots, append-only log, replay or owner governance changed.
- OpenCommander `amd` verification: `node --test apps/island/tests/*.test.mjs` **18/18 passed** (3 new async race regressions); `node --check` on both changed modules, `cargo fmt --all -- --check`, and `git diff --check` passed.
- [ ] Verify exact-commit PR CI, slow week/year simulation, ledger and deterministic replay evidence.
- [ ] Continue sourced ecology/industry work from `WORK_REMAINING.md` and owner acceptance; no week digest re-pin without slow audit.
- Next: inspect exact-SHA CI and execute slow acceptance in an isolated environment, then take the next unblocked sourced realism slice.

## 2026-11-02 continuation: live People record downloads

- People inspector selected live island records from `/api/world/humans/<id>`, but its Download link always fetched `/api/humans/<id>` (stored, potentially stale, or absent). The exported JSON could therefore disagree with the inspected live record.
- Added shared `personRecordPath` for both inspection and download, selecting the live or stored endpoint consistently and percent-encoding agent IDs.
- OpenCommander AMD: 4/4 People-selection Node tests, JS syntax, Cargo formatting and git diff checks passed. No simulation state or canon modified.
- Remaining: [ ] exact-commit full CI; [ ] week/year replay, water/carbon/material ledgers; [ ] sourced realism and industry roadmap. Next: validate CI and run slow deterministic acceptance before merge.

## 2026-11-02: authenticated inspected-person export (code df7ac0a)

- Canonical repo/branch verified: dylmarriner/Maer-ken-island, PR #68, not separate Maer-Ken. Sources: WORK_REMAINING.md, HANDOFF.md, docs/IMPLEMENTATION_READY_ROADMAP.md and reference packs.
- People Download previously navigated directly to a live/stored API path without the JavaScript bearer token; protected exports could return 401 and a second GET could export a different tick. The UI now downloads the inspected JSON snapshot via a temporary Blob URL with safe filename and URL revocation. No extra network request or simulation mutation.
- OpenCommander AMD: 21/21 browser Node tests, both JS syntax checks, cargo fmt --check and git diff --check passed. Prior exact-SHA full CI #41313267055 succeeded on 08852f4; new SHA CI required.
- [ ] New exact-SHA full CI green; [ ] slow week/year replay hashes and water/carbon/material ledgers; [ ] sourced realism/industry/workforce milestones and deviation records; [ ] owner acceptance. No simulation, canon or live deployment changes.
- Next: verify new CI, run slow acceptance on isolated runner, then implement a sourced roadmap item; never re-pin week digests without full audit.

## 2026-11-02 continuation: prevent stale conversation polling results

- Confirmed canonical `dylmarriner/Maer-ken-island`, PR #68; separate from the similarly named Maer-Ken repo. Prior exact-SHA full CI [#41318269395](https://github.com/dylmarriner/Maer-ken-island/actions/runs/41318269395) **passed** on `3e165c7`, with the slow nightly job **skipped**.
- People dashboard polls `/api/conversations` every 5 seconds. Slow older responses (or failures) could overwrite a newer conversation feed or hide its panel. Generalised the tested generation-based latest-request guard and applied it to both success and failure callbacks of conversation refresh. Rust single-writer, ledgers, logs, snapshots, replay and governance untouched.
- New tests cover out-of-order success and late failure; `node --test apps/island/tests/*.test.mjs` **23/23 passed**, JS syntax, `cargo fmt --all -- --check`, and `git diff --check` passed. Code commit `1445eee` pushed to PR #68. Full Rust slow acceptance was not executed; this is a read-only UI correctness fix.
- [ ] Exact-new-SHA full CI green; [ ] ignored week/year simulation with deterministic replay and water/carbon/material ledger audit; [ ] sourced ecology, industry and workforce milestones in `WORK_REMAINING.md`; [ ] owner acceptance.
- Next: inspect exact-SHA CI, then execute slow deterministic simulation acceptance on an isolated runner before claiming ledger/replay completion or re-pinning digests.

## 2026-11-02 continuation: abort superseded People and conversation HTTP requests

- Confirmed canonical `dylmarriner/Maer-ken-island`, PR #68 (not `Maer-Ken`). Previous exact-head CI [#41323156295](https://github.com/dylmarriner/Maer-ken-island/actions/runs/41323156295) passed at `f3f6ff2`, **excluding skipped slow nightly tests**. Latest manual slow workflow [#41295467331](https://github.com/dylmarriner/Maer-ken-island/actions/runs/41295467331) was cancelled.
- The generation guard prevented stale DOM updates but did not stop obsolete network fetches, so repeated 5-second conversation polls or rapid person selections could leave multiple requests in flight. Added `AbortController` cancellation on replacement/invalidation, forwarded signals through the authenticated JSON fetch, and preserved latest-wins rendering/error behavior.
- On OpenCommander `amd`: 3 new tests **failed before** implementation, **26/26 dashboard Node tests passed** afterward; JS syntax checks for all changed modules, `cargo fmt --all -- --check`, and `git diff --check` passed. Code commit `da08253` pushed to PR #68. No Rust single-writer, append-only log, snapshots, ledger, replay, owner governance or live simulation changed.
- [ ] Exact-new-SHA full CI green.
- [ ] Run ignored week/year slow simulation with replay hashes and material/water/carbon ledger audits before re-pinning digests.
- [ ] Implement sourced ecology, industry and workforce tasks from `WORK_REMAINING.md`, document deviations and complete owner acceptance.
- Next: verify new CI, obtain slow deterministic acceptance evidence, then select a sourced realism/industry slice. Avoid claiming full simulation acceptance from browser tests.

## 2026-11-02 continuation: serve People dashboard module from embedded Rust binary

- Canonical repo `dylmarriner/Maer-ken-island`, PR #68, not the separate Maer-Ken repo. The previous exact-head CI [#41338956880](https://github.com/dylmarriner/Maer-ken-island/actions/runs/41338956880) failed `serve::pages::tests::pages_are_served_with_their_content_types`: stale test expected `/api/humans` in `people.js` after the roster source was extracted to `people-selection.mjs`.
- More importantly, the embedded Rust `ASSETS` table did **not serve** `/static/people-selection.mjs`, despite `people.js` importing it, so the actual People page could fail to load in the installed single-binary server. Registered the module with the correct JavaScript content type, updated the route test, added a regression verifying imported static modules are served, and extended markup-insertion checks to `.mjs` files.
- OpenCommander `amd`: **32/32** dashboard Node tests, `cargo fmt --all -- --check` and `git diff --check` passed. Local targeted Rust test was attempted but the fresh dependency compilation exceeded the 240-second command timeout; **Rust test success is not claimed**. New exact-head CI must prove it. No simulation tick, single-writer, ledgers, snapshots, replay or owner governance changed.
- [ ] Exact-new-SHA full CI including Rust server tests green; [ ] slow week/year deterministic replay with material/water/carbon ledger audit; [ ] sourced ecology, industry and workforce tasks from `WORK_REMAINING.md`; [ ] owner acceptance.
- Next: inspect CI for the embedded asset regression, then run slow acceptance on a capacity-safe isolated runner. Do not re-pin digests without the full audit.

## 2026-11-02 continuation: refresh live People roster (code 041d819)

- Canonical repo `dylmarriner/Maer-ken-island`, PR #68, distinct from `Maer-Ken`. Prior exact-head CI #41344245993 passed all non-nightly jobs (fmt/clippy/fast tests, desktop, installed backend, web, scripts and dependency audit); slow week/year acceptance was skipped.
- The People page refreshed an inspected individual's live record but never refreshed its roster, leaving arrivals/deaths and roster age/status frozen until a full page reload. Added bounded non-overlapping five-second live roster polling (hidden tabs and stored populations excluded). The current selection and inspector stay intact while present; if it disappears the UI selects a surviving person, or shows a genuine empty roster. Failed/malformed responses preserve the last valid roster. Rust simulation tick, ledgers, logs, snapshots and owner governance unchanged.
- Regression first failed before implementation (missing exports); **35/35** Node dashboard tests passed afterward, including selection reconciliation, non-overlap, failure preservation and malformed response validation. Both JS syntax checks, Cargo formatting and git diff --check passed on OpenCommander `amd`.
- [ ] Exact-new-SHA CI including embedded Rust asset serving; [ ] ignored week/year deterministic replay and material/water/carbon ledger audit on capacity-safe runner; [ ] sourced ecology/industry/workforce work from `WORK_REMAINING.md` and deviation/reference-pack evidence; [ ] owner acceptance.
- Next: verify exact-commit CI, execute slow acceptance without re-pinning digests prematurely, then implement a sourced simulation milestone. No live simulation or canon change.

## 2026-11-02 continuation: cancel obsolete live People roster requests

- Confirmed canonical `dylmarriner/Maer-ken-island`, PR #68, `WORK_REMAINING.md` and the implementation-ready roadmap. Baseline PR #68 CI #41349084330 passed all non-nightly jobs at `07e04dd`; slow week/year acceptance was skipped.
- Found the five-second live roster poller lacked cancellation/invalidation. A request started in a visible tab could complete after the tab was hidden, applying stale roster data, and an obsolete request could occupy the only pending slot.
- Added abortable, revision-guarded roster polling and wired visibility/pagehide invalidation. A newly visible tab can refresh without waiting for an obsolete fetch. No Rust single-writer, canon, append-only logs, snapshots, replay, ledgers or owner governance changed.
- Two new Node regressions failed before the fix; after, **37/37 dashboard Node tests passed**, both JS syntax checks, Cargo fmt and `git diff --check` passed on OpenCommander `amd`.
- [ ] Exact-new-SHA full CI green; [ ] ignored week/year replay-hash and material/water/carbon ledger acceptance on a capacity-safe runner; [ ] sourced ecology/industry/workforce milestones and deviations; [ ] owner acceptance.
- Next: verify exact-commit CI, run slow deterministic acceptance without prematurely re-pinning digests, then implement the next sourced roadmap item. No live simulation or deployment changes.

## 2026-11-02 continuation: fail-closed live People roster integrity

- Verified canonical `dylmarriner/Maer-ken-island`, PR #68, branch `fix/island-client-human-id-20261101` against `WORK_REMAINING.md`, `HANDOFF.md` and implementation-ready roadmap. The similarly named `Maer-Ken` repository was not used.
- Earlier exact-head full CI [#41354119665](https://github.com/dylmarriner/Maer-ken-island/actions/runs/41354119665) passed its non-nightly jobs; slow week/year deterministic acceptance was skipped.
- Code `36d5ced`: the live roster previously accepted duplicate/blank identities, truthy non-boolean `alive` fields and negative/non-finite/non-numeric ages. It could therefore display `alive: "false"` as Alive, and duplicate IDs made inspector selection ambiguous. Now validates every entry and rejects the entire malformed roster before updating the UI; existing last-good-roster preservation applies.
- Regression failed before the fix, then OpenCommander AMD **38/38** dashboard Node tests passed, both JS syntax checks, `cargo fmt --all -- --check` and `git diff --check` passed. No simulation tick, Rust single-writer, append-only logs, snapshots, replay hashes, material/water/carbon ledgers, canon or owner governance changed.
- [ ] Exact-new-SHA full CI green including installed Rust server and desktop.
- [ ] Capacity-safe ignored week/year deterministic replay-hash and material/water/carbon ledger acceptance before any digest re-pin.
- [ ] Sourced ecology/industry/workforce milestones from `WORK_REMAINING.md`, reference packs/deviation records and owner acceptance.
- Next: inspect CI, execute slow deterministic acceptance without changing pinned digests, then implement the highest-impact sourced roadmap item. No live deployment or governance changes.

## 2026-11-02: fail closed on ambiguous live/stored population source

- Canonical dylmarriner/Maer-ken-island PR #68, not the separate Maer-Ken project. Reviewed WORK_REMAINING.md, its source-pack/ledger rules and the prior island handoff. Previous exact-head non-nightly CI passed at 42dbfa0; slow week/year tests were skipped.
- loadPeopleRoster previously treated a missing/null/string/number /api/world.running flag as a stopped world, silently substituting stored people for a potentially live population. It now requires a real boolean before requesting either roster; malformed state is surfaced as an error without requesting unrelated stored people.
- New regression failed before the fix (10 passed/1 failed), then **40/40 dashboard Node tests passed**. Both changed JS syntax checks, Cargo formatting and Git whitespace checks passed on OpenCommander AMD. No Rust single-writer, event log, snapshots, replay, ledger, reference packs, pinned digest or live simulation modified.
- [ ] Exact-new-SHA CI including installed backend green; [ ] ignored week/year replay-hash and material/water/carbon ledger acceptance; [ ] sourced ecology, industry and workforce milestones/deviation records from WORK_REMAINING.md; [ ] owner acceptance.
- Next: inspect exact-head CI and run slow deterministic acceptance on a capacity-safe isolated runner. Never re-pin digests without the full reference-pack/ledger audit.

## 2026-11-02: prevent caching of dynamic island JSON and human records

- Verified canonical `dylmarriner/Maer-ken-island`, PR #68, `WORK_REMAINING.md` and `docs/IMPLEMENTATION_READY_ROADMAP.md`; this is not the separate Maer-Ken repo. Prior exact-head non-nightly CI jobs all passed; slow week/year acceptance remains skipped.
- Dynamic API JSON previously had no explicit cache policy, and the authenticated dashboard GET helper did not bypass browser caches. A browser/proxy could retain sensitive human snapshots or replay stale live population/clock state. The server's shared JSON reply now sets `Cache-Control: no-store` (including errors and commands), and the browser's authenticated JSON GET sets `cache: 'no-store'`.
- Regressions: Node browser test failed before fix; **41/41** dashboard Node tests passed after. Real-loopback Rust HTTP server regression asserts `cache-control: no-store` on `/api/humans`; targeted `cargo test -p island --test serve the_server_serves_its_pages_and_its_api_on_a_real_port --locked --offline` **1/1 passed** on OpenCommander AMD after resolving a concrete JSON reply type mismatch. `cargo fmt --all -- --check` and diff check passed. No simulation tick, single-writer, canon, ledgers, logs, snapshots, replay or owner governance changed.
- [ ] Exact-new-SHA full CI (Rust, desktop, installed backend and web); [ ] ignored week/year replay-hash and water/carbon/material ledger acceptance on a capacity-safe runner; [ ] sourced ecology, industry and workforce tasks with reference-pack/deviation evidence; [ ] owner acceptance.
- Next: verify full CI, run slow deterministic acceptance, then implement sourced realism/industry work. Do not re-pin digests prematurely or change live simulation.

## 2026-11-02 continuation: canonicalize authenticated dashboard API paths

- Verified exact repository `dylmarriner/Maer-ken-island` (not `Maer-Ken`), PR #68, `WORK_REMAINING.md` and `docs/IMPLEMENTATION_READY_ROADMAP.md`. Baseline non-nightly CI [#41374670978](https://github.com/dylmarriner/Maer-ken-island/actions/runs/41374670978) passed; slow simulation acceptance was skipped.
- Fixed an authenticated browser routing gap: `apiUrl` checked only the final origin and `/api/` prefix. URL normalization could silently rewrite `/api/world/humans/../status`, percent-encoded dot segments or ASCII control characters into a different endpoint while still forwarding the owner's token. The client now rejects those non-canonical paths before URL resolution. No Rust simulation, single-writer, append-only log, snapshot, ledger, replay digest or governance changed.
- Added regressions across all four authenticated request helpers. On OpenCommander `pi5-deflab`, new regression **failed before** the fix (11/12 auth tests passed); after the fix, **41/41 dashboard Node tests passed**, JS syntax and Git whitespace checks passed. Verified the scratch changes exactly matched pushed branch `054834f`, then reran **41/41** tests on that exact commit. Code `00833d6`, regression `054834f`; PR #68.
- [ ] Exact-head CI #41379511481 green (pending at handoff).
- [ ] Capacity-safe week/year ignored simulation tests with replay-hash equality and material/water/carbon ledger closure; do not re-pin digests before audit.
- [ ] Sourced ecology/industry/workforce milestones and reference-pack/deviation records in `WORK_REMAINING.md`; owner acceptance.
- Next: verify exact-head CI, then execute slow deterministic simulation acceptance on a capacity-safe isolated runner. No live simulation or canon modifications.


## 2026-11-02: merge audited dependency fix and reconcile island feature branch

- Canonical repository `dylmarriner/Maer-ken-island`, not `Maer-Ken`; checked `WORK_REMAINING.md`, `docs/IMPLEMENTATION_READY_ROADMAP.md`, PR #68 and the separate dependency PR #69.
- Security PR #69 refreshed only `services/computer-service/package-lock.json` to remove six moderate npm audit advisories without changing direct dependencies. Exact-head CI [#41379462541](https://github.com/dylmarriner/Maer-ken-island/actions/runs/41379462541) passed all seven non-nightly jobs, including dependency audit. PR #69 was safely squash-merged to main as `6467b1f876c29ce6670b67daaa646fb78e9d65c2`.
- The existing PR #68 was based on the old main. Deepened a shallow OpenCommander checkout to recover its true merge base, then reconciled new main into the feature branch without content conflicts; the same lockfile pruning was already present on the feature branch.
- On OpenCommander AMD after reconciliation: `node --test apps/island/tests/*.test.mjs` **41/41 passed**; `cargo test -q -p mk_island_client --lib --offline --locked` **8/8 passed**; `cargo fmt --all -- --check` and `git diff --check` passed. The ignored week/year slow simulation, replay-hash and material/water/carbon acceptance remains **unverified**. A separate `cargo test -p mk_engine --test island_replay --offline --locked -- --test-threads=1` attempt was terminated (exit -9) during dependency compilation before any test assertions; it is **not** a passing replay result.
- [ ] Verify exact-merge-head PR #68 CI and confirm no regression in npm audit.
- [ ] Run the ignored week/year deterministic replay and ledger audits on a capacity-safe isolated runner, and record pinned digests without changing them prematurely.
- [ ] Continue sourced ecology/industry/workforce milestones and deviation/reference-pack evidence from `WORK_REMAINING.md`.
- [ ] Complete owner acceptance. No live simulation, canon, single-writer, event log, snapshots or governance changed.

Next: verify PR #68 after the merge-main reconciliation, then run the slow deterministic ledger/replay gate before implementing the next sourced simulation slice.

## 2026-11-02 continuation: validate stored population before adoption

- Confirmed `dylmarriner/Maer-ken-island` PR #68 and `WORK_REMAINING.md`/implementation-ready roadmap, not the similarly named Maer-Ken repository. Previous non-nightly exact-head CI #41385796860 passed; slow week/year suite skipped.
- The People page previously adopted the new source mode and rendered roster before validating a stored population. Malformed/duplicate stored summaries could produce misleading identities, invalid age sorting, or a half-updated UI after failure. New shared `preparePeopleRoster` validates live tick/roster or stored `HumanSummary` shape, identity uniqueness, status, sex and finite age **before** the page changes source or population.
- Regression evidence on OpenCommander AMD: new tests **failed before** implementation (missing helper); after, dashboard Node suite **43/43** passed. Both JS syntax checks, Cargo fmt and Git whitespace checks passed. No Rust simulation code or digest changed.
- [ ] New exact-head CI green (Rust, installed backend, dashboard, audit); [ ] ignored week/year replay hashes and material/water/carbon ledger closure on capacity-safe runner; [ ] sourced ecology/industry/workforce roadmap and deviations; [ ] owner acceptance.

Next: inspect CI, execute slow deterministic acceptance without changing pinned digests, then implement a sourced roadmap slice. No live simulation or governance changes.


## 2026-11-02 continuation: conversation snapshot integrity and hidden-tab budget

- Verified canonical dylmarriner/Maer-ken-island PR #68 (not separate Maer-Ken), WORK_REMAINING.md, HANDOFF.md, implementation roadmap and Rust Conversation wire schema. Prior exact-head CI [#41390572782](https://github.com/dylmarriner/Maer-ken-island/actions/runs/41390572782) passed all seven non-nightly jobs including desktop, installed backend, Rust and dependency audit; slow tests skipped.
- The People dialogue panel previously adopted unvalidated conversation feeds. Malformed clock spans, invalid line counts or missing speaker/text could crash rendering or misrepresent simulated speech. Added fail-closed whole-snapshot validation before DOM changes, including legacy zero-valued counters.
- Hidden People tabs continued polling dialogue every five seconds. Now skip hidden-tab requests, abort an in-flight request on hide/pagehide, and refresh on visibility restoration. This reduces unnecessary authenticated reads and prevents obsolete hidden-tab responses.
- RED: newly added tests failed before implementation because the validator was absent. GREEN: **45/45** dashboard Node tests passed; both changed JS syntax checks, cargo fmt --all -- --check and git diff --check passed on OpenCommander AMD.
- [ ] Verify new exact-head CI for browser/Rust embedded-asset compatibility and dependency audit.
- [ ] Run capacity-safe ignored week/year replay-hash equality and water/carbon/material ledger acceptance; never re-pin without full audit.
- [ ] Continue sourced ecology, industry and workforce milestones with deviation/reference-pack evidence from WORK_REMAINING.md; owner acceptance remains outstanding.
- Next: complete CI, execute slow deterministic replay/ledger acceptance on a capable runner, then implement the next sourced simulation milestone. No Rust tick, single-writer, append-only logs, snapshots, canon, replay digests, live simulation or governance changed.
