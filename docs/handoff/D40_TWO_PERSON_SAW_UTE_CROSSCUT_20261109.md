# D40 two-person saw cross-cut for Ute transport

Date: 2026-11-09
Repository: dylmarriner/Maer-ken-island
Branch: feat/d40-two-person-saw-ute-crosscut-20261109
Base: 6b57ad96273c2320910f93902963bc8800974175

## Slice

Implemented the still-open D40 step that cross-cuts an already-felled log with the shed's two-person crosscut saw when a Ute haul is the quickest plan and another founder is available.

- The lower-sorted founder owns one persisted HaulJob and the single append-only material mutation.
- The other gathering founder is selected deterministically by distance to the log butt, then agent ID.
- The crew task duration includes both founders' walks, the faster founder's wait, and the reference-pack two-person saw time for every cut.
- Each founder receives a separately calculated blended MET and energy charge using their own body and walk distance.
- The partner is not given a duplicate material-owning haul job; this prevents duplicate cross-cut execution and keeps replay state single-writer.
- The partner is moved to the log butt when the cut completes.
- If the partner leaves gathering/outdoors work, the persisted task deterministically falls back to the solo steel-axe duration and clears crew metadata. Time already spent remains credited.
- Legacy axe CrossCut snapshots remain byte-shape compatible. The crew uses a distinct CrossCutCrew enum variant so an older binary fails closed on a new crew snapshot rather than silently ignoring the partner.

## Regression

regional::life::tests::two_founders_cross_cut_a_ute_log_with_the_saw_and_both_pay_energy creates a long remote log, disables the excavator, ensures the Ute plan is selected, verifies the partner identity and MET, reserves the exact piece-ID range only after plan acceptance, advances the task, and asserts both founders pay energy while only the primary owns the haul job.

## Verification

Exact-head Rust formatting, Clippy, fast tests, desktop and installed-backend acceptance are required. No test pass is claimed before the new GitHub Actions run completes. This change does not alter RNG, snapshot version, replay hash format, material ledger semantics or vehicle fuel accounting.

## Remaining

- [ ] Exact-head Rust/desktop/backend CI.
- [ ] Full cross-cut completion and conservation test on the integrated work loop.
- [ ] Terrain-aware Ute routes and persistent vehicle positions.
- [ ] D31 regional soil-nutrient model.

## Safety

No production deployment, live data migration, destructive action, external communication or paid service was used.
