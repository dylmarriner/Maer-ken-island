# Maer-Ken Island engineering handoff: conversation text budget (2026-11-03)

## Scope
- Canonical remote `dylmarriner/Maer-ken-island`, PR #68, branch `fix/island-client-human-id-20261101`. This is **not** the separate Maer-Ken repository.
- Acceptance sources: `WORK_REMAINING.md`, `docs/IMPLEMENTATION_READY_ROADMAP.md`, `docs/handoff/AUTOMATION_ENGINEERING_HANDOFF.md`, canonical reference packs.

## Completed code slice
- The browser already limited the feed to 40 conversations and 12 lines each, matching Rust projection constants, but accepted unlimited text sizes. A malformed authenticated response could create large DOM strings and exhaust browser memory.
- `validateConversationFeed` now rejects oversized feed note (>2048 UTF-16 code units), relationship (>128), speaker name (>256) and dialogue line (>4096), before rendering. These are defensive client limits; they do not alter Rust simulation data.
- RED before fix: new budget regression failed (19/20 tests passed).
- GREEN on AMD/OpenCommander: **56/56** Node dashboard tests; JavaScript syntax checks; `cargo fmt --all -- --check`; Git diff check.
- Prior exact-head PR CI #41486528482 passed non-nightly jobs at `94f88f4`; new commit requires its own hosted checks.
- No changes to deterministic Rust single-writer, append-only logs, snapshots, replay digests, optional LLM, governance, budget or live simulation.

## Remaining acceptance checklist
- [ ] Verify exact-new-head full CI, including Rust, installed backend and desktop.
- [ ] Execute capacity-safe ignored week/year deterministic replay and water/carbon/material ledger tests; do not re-pin hashes without full audit.
- [ ] Implement sourced ecology, industry and workforce milestones, with deviations and reference evidence.
- [ ] Review whether engine-side response byte budgets should mirror browser limits for all authenticated API clients.
- [ ] Obtain owner acceptance only with evidence.

## Next engineering slice
Verify exact SHA CI, then execute slow deterministic acceptance in an isolated environment before changing canonical simulation invariants. Continue sourced roadmap milestones.
