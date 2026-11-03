# Inspector NFC identity integrity — 2026-11-03

Change: reject non-NFC person identifiers before constructing authenticated stored/live inspector URLs, avoiding byte-distinct identity ambiguity.
Evidence: node --test apps/island/tests/people-selection.test.mjs (26/26); git diff --check (pass).
Remaining: full hosted CI, deterministic week/year replay and ledger acceptance, ecology/industry/workforce roadmap.
Next: inspect exact-head CI and implement replay/ledger acceptance.
