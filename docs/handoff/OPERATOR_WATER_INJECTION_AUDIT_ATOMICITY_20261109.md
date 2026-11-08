# Operator water-injection audit atomicity — 9 November 2026

## Finding

The public InjectResource(Water) path called IslandLife::audited before it knew whether MaterialLedger::inject would accept the request. audited closes a successful audit window even when its closure made no stock change. If the material ledger rejected an overflow or a positive increment rounded away, the operator action returned an error and was omitted from the replay log, but audits_closed had still advanced. A refused command was therefore not state-neutral.

## Production change

- Extracted the stock, amount, resulting-stock and carbon representability checks into the pure MaterialLedger::injection_carbon_kgc calculation.
- Added MaterialLedger::validate_injection, which runs the exact same preflight without changing stock or writing flux entries.
- inject_water now validates before entering IslandLife::audited.
- The subsequent mutation repeats the same check and is treated as a single-writer invariant: no other operation can change the material stock between preflight and the audited mutation.
- Invalid requests now return before the audit counter, water-window ledger, material stock, physical flux ledger or replay log can change.

## Regression

rejected_operator_water_injection_does_not_close_audit_or_enter_replay_log seeds Water to f64::MAX, attempts a positive 1 kg operator injection, and asserts:

- typed MaterialError::InvalidAmount rejection;
- unchanged Water stock;
- unchanged physical flux-entry count;
- unchanged audits_closed;
- unchanged replay-entry count;
- byte-identical state digest.

## Commits

- d8f29e7db9b3d6a82a98b4c2363ae7920a13c9cf — pure material-injection preflight.
- 6137aa5d804078d0b8d99b85a25e02cadc074250 — reject before audit mutation.
- 8d729b3a7c6154a9e3707f97d13aae790725a443 — end-to-end atomicity regression.

## Verification

Exact-head CI is required for formatting, Clippy, fast Rust tests, desktop/backend acceptance, dependency audit and week/year reference packs. No local Cargo result is claimed because the custom OpenCommander connection returned HTTP 429. The CI result for the preceding PR #131 does not verify this new branch.

## Safety and architecture

No snapshot format, append-only replay schema, replay hash, RNG, tick cadence, scientific constant, reference pack or live simulation state changed. The fix preserves deterministic single-writer ownership and the existing append-only command log.

## Next

Run exact-head CI on this branch, address any formatter/Clippy/test findings, then merge only after all required non-nightly acceptance jobs pass. Continue D40 two-person saw cross-cutting and founder labour/energy accounting as the next separate product slice.
