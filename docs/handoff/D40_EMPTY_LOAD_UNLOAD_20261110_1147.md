# D40 empty load/unload guard

Source and regression: `6456eb04`. Persisted Ute Load and Unload tasks reject empty manifests, while empty return Drive tasks remain valid. No RNG, tick, snapshot schema or material arithmetic changed. Exact-head rustfmt, Clippy, fast tests, replay/reference packs and dependency audit remain unverified; do not merge PR #136 until green. Next: run these checks, investigate libsqlite3-sys advisory, reconcile PRs #134/#135 and continue terrain-aware routing and vehicle persistence.
