# Island snapshot: size, time and memory

Measured 2026-10-07, development machine, release build.

    cargo test --release -p mk_engine --test island_snapshot a_restored -- --nocapture

The whole island after 30 simulated minutes, saved and restored (JSON payload, digest-prefixed).

| Quantity | Result |
|---|---|
| File size | 336 MB |
| Save | 0.9 s |
| Load, validate, re-derive caches | 1.2 s |
| Peak resident memory of the test (two worlds resident + serialisation buffers) | 1.29 GB |

Size is dominated by the medium hydrology and ecology grids written as JSON text. Compression
(or a binary grid encoding for the grids only) is a Phase 5 option; a snapshot this size is fine
for an occasional save but heavy for frequent autosave.
