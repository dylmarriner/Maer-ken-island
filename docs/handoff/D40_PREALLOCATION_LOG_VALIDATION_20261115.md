# D40 + material-ingestion integration: pre-allocation log identity (15 November 2026)

## Problem

After merging PR #135 into `main`, the D40 two-founder saw-to-Ute acceptance failed at `MaterialLedger::receive_felled_log`: PR #136 strengthened `Log::has_valid_physical_state` to reject zero log IDs, while the production felling path calls `felled_log(0, ...)` and asks the material ledger to assign the real ID only after receipt preflight.

## Resolution

- `Log::has_valid_unallocated_physical_state` validates finite geometry, endpoints, dry mass, carbon and water for newly felled logs before allocation.
- `Log::has_valid_physical_state` *additionally* requires a nonzero ID for stored/persisted logs, existing cross-cut sources and haul-planning inputs. Do not loosen that persisted-state guard.
- `receive_felled_log` uses pre-allocation validation, then allocates its one-based ID only after stock, water-source and audit preflight. No snapshot, RNG, tick duration, transport constants or replay digest intentionally changed.

## Exact local verification on AMD

- Before the fix, `two_person_ute_crosscut_restores_and_completes_the_full_material_haul` failed at the new physical-state assertion.
- Added `receiving_a_valid_unallocated_log_assigns_a_real_id_and_books_material`, observed it fail on the same assertion before the fix.
- With the fix, `cargo test -p mk_engine --test island_material_flows --locked`: 70 passed, zero failed.
- With the fix, `cargo test -p mk_engine --lib --locked two_person_ute_crosscut_restores_and_completes_the_full_material_haul`: one passed, zero failed.
- Full workspace CI and week/year reference-pack verification are still required on the updated PR head before merge.
