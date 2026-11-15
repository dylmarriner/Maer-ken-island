# D40 exact-head fast-test corrections (15 November 2026)

The first complete CI on the reconciled PR #136 head `13b3092` (run #43678835386) passed formatting, dependency audit, installed backend, desktop application, scripts, dashboard, service and week/year reference-pack acceptance. The fast Rust job had 905 passing and two failing tests:

1. `accepting_a_conserved_ute_recut_reserves_ids_before_another_log_arrives` invoked `Log::cross_cut` on a not-yet-received log with ID zero. Production deliberately requires nonzero IDs for cross-cut, while `receive_felled_log` assigns an ID only after receipt preflight. The test now cross-cuts a temporary preview with the pending ID without modifying the unallocated log that is subsequently received.
2. `cross_cut_rejects_finite_source_with_overflowed_endpoint` expected a legacy panic message rather than the current stricter validator's message. The expected assertion message has been updated, and the physical rejection condition is unchanged.

On AMD the two previously failing tests both passed using `cargo test -p mk_engine --lib --locked <test-name>` after the corrections. The earlier combined PR #136 head also passed all 70 material-flow integration tests, the full two-person saw-to-Ute snapshot/restore acceptance and strict Clippy. The full CI on the newly corrected exact head is still required before merge. No production code was changed by these two test-fixture corrections.
