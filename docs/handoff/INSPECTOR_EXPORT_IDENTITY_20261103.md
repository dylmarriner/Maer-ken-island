# Maer-Ken Island inspector export identity, 2026-11-03
- Change: Reject exported human record lacking valid identity rather than silently using a fallback filename.
- Regression: reject absent, whitespace and spoofed Unicode/control IDs.
- Verified: node --test apps/island/tests/*.test.mjs: 64 passed; git diff --check passed.
- Hosted CI: check exact head after push.
- Remaining: deterministic week/year replay, water/carbon/material ledger acceptance, ecology, industry and workforce roadmap.
- Next: verify full Rust/backend CI and deterministic replay acceptance.
