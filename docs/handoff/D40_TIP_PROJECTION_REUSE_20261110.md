# D40 tip projection | 2026-11-10

Branch: `feat/d40-two-person-saw-ute-crosscut-20261109`

- Production change: `0c4ada8afc5273482d78197cc77c9cdd188909d9`
- Regression: `ddeda96d3599c7c22c9e1e50ab49aa4d2662a11e`
- Verification: GitHub accepted source/test commits; exact-head CI is not yet proven green. No merge or production deployment.
- Remaining: Rust fmt, Clippy, focused/full tests, replay hash equality and dependency audit; then terrain-aware Ute routing and D31.
- Next run: inspect exact-head workflow, resolve failing checks, verify tests and only then consider safe merge.
