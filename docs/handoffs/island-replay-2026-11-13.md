# Maer-Ken Island replay verification, 2026-11-13
- Branch: test/replay-special-file-refusal-20261104; code commit 20c4a5a.
- Added runner contract cases for opt-in reduced-debug profile and 86400-second maximum timeout.
- Evidence: bash -n and bash scripts/test-replay-runner-contract.sh passed; git diff --check passed.
- REPLAY_TEST_TIMEOUT=25 bash scripts/test-replay-low-memory.sh returned 124 while compiling mk_engine; replay/hash equality NOT verified.
- Branch 38 commits ahead of origin before this handoff; review all before pushing.
- Next: run full replay suite on sufficiently resourced Rust builder, confirm hash equality and append-only invariants, then review PR/CI and publish validated changes.
- No production mutations.
