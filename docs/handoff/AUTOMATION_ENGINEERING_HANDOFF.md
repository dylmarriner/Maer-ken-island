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
