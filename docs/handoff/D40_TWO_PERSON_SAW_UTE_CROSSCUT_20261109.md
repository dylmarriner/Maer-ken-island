# D40 two-person saw cross-cut for Ute transport

Date: 2026-11-09
Repository: dylmarriner/Maer-ken-island
Branch: feat/d40-two-person-saw-ute-crosscut-20261109
Base: 6b57ad96273c2320910f93902963bc8800974175
Latest implementation commit: 5f1c9649c79db1711c60b6ce0b56ae2c3623052e
Latest PR head: 18a6ef92352bc57be71450db253f490053c436d2
Latest exact-head CI: [#42550881615](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42550881615) in progress; formatting has passed and Clippy/fast tests, desktop, installed backend and week/year acceptance are running.

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

regional::life::tests::two_founders_cross_cut_a_ute_log_with_the_saw_and_both_pay_energy creates a long remote log, disables the excavator, ensures the Ute plan is selected, verifies the partner identity and MET, reserves the exact piece-ID range only after plan acceptance, advances the task, and asserts both founders pay energy while only the primary owns the haul job, then removes the partner from the work site and verifies the primary task falls back to a solo axe CrossCut.

## Verification

Exact-head Rust formatting, Clippy, fast tests, desktop and installed-backend acceptance are required. No test pass is claimed before the new GitHub Actions run completes. This change does not alter RNG, snapshot version, replay hash format, material ledger semantics or vehicle fuel accounting.

## Remaining

- [ ] Exact-head Rust/desktop/backend CI.
- [ ] Full cross-cut completion and conservation test on the integrated work loop.
- [ ] Terrain-aware Ute routes and persistent vehicle positions.
- [ ] D31 regional soil-nutrient model.

## Safety

No production deployment, live data migration, destructive action, external communication or paid service was used.


## Exact-head CI correction — 9 November 2026

- Workflow [#42549729268](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42549729268) exposed a compile error: the assistant map stores a `String` primary founder ID, so reading its tuple with `.copied()` incorrectly required `String: Copy`. The lookup now clones the tuple.
- The same workflow's rustfmt gate required the completed-task `CrossCutCrew` pattern to use the formatter's single-line form.
- Corrected source commit: `ddda1903e58202e6ed6c6b2c4e7c43b1cede3993`.
- The prior run's desktop Clippy, installed-backend build and reference-pack acceptance failed or stopped because of this compile error; they are not treated as independent feature failures. Fresh exact-head CI is required.
- Full-duration cross-cut completion/conservation and interruption/restart acceptance remain open; the existing test only covers selection, energy, single-writer ownership, reservation and partner departure fallback.


## Same-tick saw-assistant exclusivity — 9 November 2026

- Review of the work loop found that a founder selected as a saw partner by an earlier primary could remain eligible for a later primary's haul plan in the same tick. The later plan could overwrite the partner-to-primary map entry, leaving one crew without its partner energy charge and assigning the same person to two persisted crew tasks.
- `plan_haul` now receives the current tick's assigned-assistant map and excludes any already-reserved partner from later candidate selection. The single-writer primary task remains the only owner of the material split.
- Regression coverage injects an already-assigned partner and proves the planner selects the solo axe CrossCut instead of creating a second crew assignment.
- Source commit: `dcde66d472ed5bfbb8c77b7572ed84ab81585e98`; work-register update: `46e3a22063e4ce5d14f5ca1844161c24c140926e`.
- Exact-head Rust formatting, Clippy, fast tests, desktop/backend and reference-pack CI remain required.


## Exact-head rustfmt correction — 9 November 2026

- Workflow [#42550746339](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42550746339) reached the formatter and identified three layout-only differences in the new assigned-assistant regression: compact `BTreeMap::from` tuple layout and wrapped planner assertions.
- Applied the exact rustfmt forms in `life.rs`; no production logic changed in this formatting correction.
- Commit: `5f1c9649c79db1711c60b6ce0b56ae2c3623052e`.
- The run's other acceptance jobs were still in progress when the format gate failed; their results are not a substitute for fresh exact-head CI.
