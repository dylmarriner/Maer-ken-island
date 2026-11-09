# D40 physical endpoint validation — 2026-11-10

PR #136. Code commit `67d6c22742d6ed3bc64935707c6b62772b1e0302`; regression commit `1a9e21a635d466bc0174613af85a4d46e41b4fdd`.

`Log::has_valid_physical_state` now rejects logs with individually finite centre/length but non-finite projected butt/tip endpoints. Regression constructs such a log. No RNG, snapshot format, replay hash, material constants or vehicle routing changes.

**Verification:** GitHub accepted commits; exact-head rustfmt, Clippy, Rust tests, replay and security audit NOT verified. Check previously reported `libsqlite3-sys` advisory against current lockfile and update compatible chain without suppressing the advisory. Reconcile #134/#135; finish D40 terrain-aware Ute routing and D31 regional soil nutrients. Do not merge before green.
