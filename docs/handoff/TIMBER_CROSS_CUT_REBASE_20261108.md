# Timber cross-cut conservation mainline rebase

Date: 2026-11-08
Repository: `dylmarriner/Maer-ken-island`
Branch: `rebase/timber-cross-cut-main-20261108`
Base: `main` at `f62afa24fbc6e22bcdf36042a65cbddb4fc8df1d`

## Engineering slice

Ported the timber cross-cut implementation from PR #124 onto current main after PR #123's replay sequence work merged. All five source/test patches applied cleanly against the current main files, including the replay-modified `life.rs`. The port retains the current replay index implementation and does not replace mainline replay logic.

Included:
- binary64 remainder allocation for the final piece, preserving exact dry-mass, carbon, water and length totals;
- geometry derived from accumulated piece length;
- deterministic MaterialLedger Debug compatibility for empty reservations;
- fail-fast physical-state validation at log ingress and cross-cut boundaries;
- one-based, checked, bounded and previously-reserved piece-ID ranges;
- immutable haul preview and reservation only after founder acceptance;
- regression coverage for conservation, reservation, malformed state, zero/oversized cuts and declined haul.

## Commits

- `0a1c7b12c1b7552ed00c57fd02f7472ee26d599d` — labour error bound.
- `6a90b12231bce876f431760b221cab47a6820e04` — immutable haul preview and acceptance-only reservation.
- `3f8a3480dfd0651170ad01d5ace31ee44320a5f2` — material ledger invariants.
- `63c134a85cf1f927448a3f7342711d7a5f55bde0` — conserved cross-cut implementation.
- `02ec05d89d5dcb68ef55fbbac7d3cc9a56e87f37` — regression tests.
- `3737b845539b4af87177f5534ec24827920feb65` — canonical checklist update.

## Verification

- Patch application against current main: passed for all five source/test files.
- Exact-head Rust/desktop/backend/reference-pack CI: pending; branch has not yet been opened as a PR.
- Local Cargo checks: not run because custom OpenCommander currently returns HTTP 429.
- No snapshot schema, append-only log format, replay hash, RNG, simulation tick, LLM behaviour, live state or production infrastructure was changed intentionally. The week reference-pack digest compatibility regression is included and must pass before merge.

## Checklist

- [x] Port timber source and regression changes onto current main.
- [x] Preserve replay code already merged on main.
- [x] Update canonical remaining-work checklist.
- [ ] Open draft PR and capture exact head.
- [ ] Run formatting, Clippy, fast Rust tests, desktop/backend acceptance and week/year reference packs.
- [ ] Resolve review findings.
- [ ] Merge only after all required exact-head checks pass.
