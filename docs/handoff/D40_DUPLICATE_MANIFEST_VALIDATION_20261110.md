# D40 vehicle manifest integrity, 10 November 2026

PR #136. Production + regression commit `92d69c58f6364b1269cd5013b965f1513b65dde1`.

Persisted haul-job validation now rejects zero/duplicate log IDs in Drive, Load and Unload manifests using a per-manifest HashSet. The check is deterministic and read-only; no simulation timing, RNG, replay schema or snapshot format changes. Rust regression covers each duplicate action and valid/zero manifests.

Verification: GitHub accepted commit. Exact-head rustfmt, Clippy, Rust tests, replay/reference packs and security audit **not yet executed**. Prior dependency audit reported libsqlite3-sys advisory; verify current lockfile and remediate compatibly.

Remaining: run full exact-head CI, resolve security advisory, reconcile #134/#135, terrain-aware Ute routes and persistent vehicle positions, D31 soil nutrients. No production infrastructure changes.
