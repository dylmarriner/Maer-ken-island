# Timber cross-cut mass conservation

Date: 2026-11-08  
Repository: `dylmarriner/Maer-ken-island`  
Branch: `fix/timber-cross-cut-exact-conservation-20261108`  
Base: `main`

## Finding

`Log::cross_cut` divided dry wood, carbon, water and length into `n` identical
binary64 shares. Repeated rounded shares do not always sum to the original value.
For example, three shares of `914.6308346979964` kg sum one ULP above the
source; a length of `7.123456789012345` m sums one ULP below it. The cross-cut
operation therefore could create or lose a tiny amount of material in the
ledger even though it is only meant to partition an existing log.

## Change

- Keep the first `n - 1` pieces at the equal share.
- Assign the final piece the remaining dry mass, carbon, water and length.
- Derive each piece's centre from the accumulated preceding length, so the
  final geometry uses the same residual rather than assuming every rounded
  piece has identical length.
- No source parameters, species data, snapshot fields, replay format, RNG,
  single-writer ownership or LLM behaviour changed.

## Regression

`cross_cutting_assigns_floating_point_remainders_without_losing_material`
uses non-exactly-divisible dry mass and length values. It compares the summed
piece totals to the original values by `to_bits()`, not a loose tolerance, for
dry mass, carbon, water and length.

## Commits

- Production: `fa81e5ba92a69b3ef414d8a75faab1f896798881`
- Regression: `3aa0c6bdf56842debc62671be0c501aad8ddf344`

## Verification

No local Cargo result is claimed because custom OpenCommander device discovery
is currently returning HTTP 429. Exact-head CI must pass formatting, Clippy,
fast Rust tests, desktop/backend acceptance and week/year reference-pack tests.
The week/year digest must not be changed merely to accommodate this precision
correction; any digest difference must be audited against the simulation path
before a deliberate re-pin.

No live island state, production infrastructure, deployment or irreversible
data operation was changed.


## Exact-head formatting correction

The first exact-head workflow `#42370311309` reached `cargo fmt --all -- --check` and
reported one line-wrap difference in the new carbon-total assertion. Commit
`3607dc9a27e9733c729b7f886508b8a536b992ef` applies the formatter's exact output. Clippy and fast tests
were skipped after formatting failed; the new head requires a fresh full exact-head run.


## Haul-limit review correction — 8 November 2026

Codex review identified that `haul_plans_over` selected cut counts and estimated
carrying/payload costs from average piece mass. The conserved final remainder
can be larger than the average after repeated binary64 additions, so a plan
could treat a piece as within the sustainable load or ute payload when the
actual piece was not.

The follow-up now:

- Builds the actual conserved split and checks its largest piece mass and length.
- Increases the cut count until every piece fits the sustainable load, and for
  the ute also fits the payload and bed-length limits. A four-ULP tolerance
  avoids pathological over-cutting for insignificant representation noise.
- Calculates ute trip capacity from the largest actual piece, not average mass.
- Prices each loading, unloading and on-foot carry using that piece's actual
  green mass.

Regression: `haul_plans_recut_a_conserved_remainder_above_the_sustainable_load`
uses 9,928 nominal pieces to reproduce an accumulated remainder above the
sustainable-load limit, asserts a 9,929-piece plan, checks every ute trip
against payload, and verifies the final on-foot carry duration uses the
actual final-piece mass.

Production commit: `45ef1bb27a8056d8008af97690baa338732dad19`.
Rustfmt correction: `aa1e3f991a9f53b0542c39abeb1ef9c0d64211a8`.

Exact-head workflow [#42372078046](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42372078046)
reached the branch and failed only at `cargo fmt --all -- --check`; the exact
formatter output was applied in `aa1e3f9`. Fresh exact-head workflow
[#42372226293](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42372226293)
is queued. No Clippy, fast-test, desktop, backend or week/year pass is claimed
for the corrected head yet. OpenCommander remains unavailable (HTTP 429), so
no local Cargo result is claimed.


## Clippy follow-up — 8 November 2026

Exact-head workflow [#42372299956](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42372299956)
passed `cargo fmt --all -- --check`, then failed Clippy on
`clippy::unnecessary_map_or` for the optional bed-length predicate. The desktop
application job reached the same Clippy failure; the fast Rust tests did not
run because the lint gate failed. The predicate now uses Rust 1.97's
`Option::is_none_or`, as Clippy requested.

Correction commit: `61a8cd4c9cb4b70480d3bc07a8ea078db3ea64b8`.
A fresh exact-head workflow for this commit is pending. No Clippy, fast-test,
desktop, backend or week/year pass is claimed for the corrected head.


## Clippy correction commit and fresh verification — 8 November 2026

The optional bed-length predicate now uses `Option::is_none_or` as required
by Rust 1.97 Clippy. Correction commit:
`61a8cd4c9cb4b70480d3bc07a8ea078db3ea64b8`.

Fresh exact-head workflow
[#42372526154](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42372526154)
is in progress. The previous run's only reported source-level blocker was
`clippy::unnecessary_map_or`; no test result is inferred from that correction.


## Reserve the exact final haul-piece id range — 8 November 2026

PR #124 review found that `plan_haul` reserved ids from the average mass/length estimate before `haul_plans_over` applied its conserved-remainder re-cut. When a 13-piece estimate required 14 actual pieces, the next felled log could receive the id of the 14th planned piece and be overwritten when the CrossCut task materialised that piece.

- Added `MaterialLedger::next_log_id()` as a read-only single-writer preview of the next unreserved id.
- Build every candidate haul plan from that preview, inspect the actual `CrossCut.pieces` counts, then reserve the maximum candidate range before returning the selected plan.
- `reserve_log_ids` now uses checked addition and fails fast rather than wrapping the id space.
- Added an integration regression using the 13-to-14-piece Ute bed-length remainder. It inserts a second felled log before CrossCut executes, then verifies the second log and all fourteen piece ids survive without collision.

Commits:
- Materials id preview and checked reservation: `ff5d61840396684b44f1517703c9692077e18051`
- Planner correction and collision regression: `bee64c6f08f27595a86b6de99110cb47a05ee65a`
- Rustfmt correction: `8138ed5ba1c582a2d46bda6fe9a3ed70e5b2fb6e`

The first exact-head workflow [#42379017290](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42379017290) reached checkout and reported only two rustfmt differences in the new test; Clippy and fast tests were skipped behind that format gate. The formatter output was applied in `8138ed5`. A fresh exact-head workflow is required; no Rust test or Clippy pass is claimed for the corrected head yet.


## Refuse cross-cut destination ID collisions before mutation — 8 November 2026

The planner now reserves the exact candidate piece range, but the public material-ledger
boundary also needs to defend itself against a stale or malformed caller-supplied range.
Previously `cross_cut_log` removed the source before inserting pieces and unconditionally
inserted into the destination IDs; a direct caller could overwrite an existing log.

- `cross_cut_log` now reads the source without removing it, clamps the requested piece count
  to the existing minimum of one, and rejects zero, overflowing or already-occupied
  destination ranges before any mutation.
- The occupancy check uses the `BTreeMap::range` index, not a per-ID scan, so validation is
  logarithmic in the ledger size plus the first matching entry.
- Only after the full range is valid does it materialise pieces, remove the source and insert
  the split. Invalid requests preserve the source, every existing destination and audited
  carbon/water stocks.
- Regression:
  `cross_cut_refuses_colliding_or_overflowing_piece_ids_without_mutating_the_ledger`
  creates two felled logs, attempts a colliding split and an overflowing range, and verifies
  both logs, next-id state and material stocks remain unchanged.
- Commits:
  - Production: `a8c0abca755aefc59e09786aceb5f19aff15c4a3`
  - Regression: `0855c4dcb4c534e90a1a416c7da990083498c300`

The first exact-head attempt [#42383634990](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42383634990)
failed only the new test's rustfmt blank-line check; commit `57ba202957f77dc570e7b9f29620eccacadd83f7`
applies the formatter output. The planned CrossCut caller now asserts that
`cross_cut_log` succeeds, so a malformed or colliding plan fails fast instead of silently
completing a haul with the source log still present. Caller commit:
`74c4c865c1f7c80a342e95604e4203a12dc1c970`.

Exact-head workflow [#42383983549](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42383983549)
on `74c4c86` has passed formatting, Clippy, installed backend, computer service, web
dashboard, repository scripts, dependency audit and week/year reference-pack acceptance.
Fast Rust tests and desktop render/input acceptance were still running at the latest check;
nightly slow tests are skipped by design. No full exact-head pass is claimed until those
two jobs finish. Codex reviewed this head without major findings, and both existing PR #124
review threads are resolved.

No snapshot schema, replay log, canonical digest, RNG, simulation tick, live state or
production infrastructure was changed.


## Reserved-range precondition — 8 November 2026

A follow-up API audit found that the collision/overflow guard still accepted a future
but unreserved destination range. Because cross_cut_log did not advance next_log,
a later receive_felled_log could eventually allocate one of those piece IDs and
overwrite it. The normal IslandLife planner already reserves its range first, but
the public ledger boundary must enforce that invariant itself.

Production change in MaterialLedger::cross_cut_log:
- Require the complete destination range to be below the exclusive next_log
  watermark, proving the single writer reserved it before materialisation.
- Continue rejecting zero, overflow and occupied IDs before removing the source.
- Preserve the existing no-mutation refusal path.

Regression added to
cross_cut_refuses_colliding_or_overflowing_piece_ids_without_mutating_the_ledger:
- An unoccupied future range at IDs 10–11 is refused while next_log_id() == 3.
- Source log, destination slots and carbon/water stocks remain unchanged.
- The same cut succeeds after reserving IDs 3–4, and the allocator advances to 5.

This does not change snapshot shape, replay events, hashes, RNG, material stocks,
or the valid planned single-writer path. Exact-head CI is required; no local Cargo
test result is claimed while custom OpenCommander remains unavailable.


## Direct Log::cross_cut ID-range preflight — 8 November 2026

The material-ledger entry point already rejects zero, overflowing, occupied
and unreserved destination ranges. A final public-API audit found that the
lower-level pure `Log::cross_cut` function still formed piece IDs with
unchecked addition. A direct caller near `u64::MAX` could therefore wrap IDs
even though the normal ledger path rejects the same range.

The pure function now fails before constructing any pieces unless
`first_id` is one-based and the exclusive end `first_id + max(n, 1)`
is representable. The ledger remains responsible for proving that the range
was actually reserved. Regression coverage checks zero, overflow, and the
last non-overflowing exclusive range (`u64::MAX - 1` for one piece).

Production and regression are in the same Rust module and are covered by the
PR #124 exact-head formatting, Clippy and fast-test jobs. No ledger, snapshot,
replay, RNG, simulation or live state is mutated by this preflight.


## Reject previously consumed piece-ID ranges — 8 November 2026

A new P2 review found that the exclusive `next_log` watermark plus an occupancy check was insufficient: after a cross-cut piece was seasoned into the wood stock and its log row removed, a different live source could reuse that now-empty old ID range. The watermark proves an ID was once allocated or reserved, not that it is still pending.

- `MaterialLedger` now tracks pending cross-cut IDs in a deterministic `BTreeSet`. The set is serialized only when non-empty and has `serde(default)`, so older snapshots remain readable and snapshots without pending cuts retain their previous representation.
- Immediate felled-log allocation uses a separate allocator path and is never marked as a pending cross-cut reservation.
- `cross_cut_log` requires every destination ID to remain pending, then consumes those reservations only after the full preflight succeeds. A refused cut leaves the reservation available for its original planned operation.
- `plan_haul` now reserves only the selected quickest plan's piece range. Alternative plans no longer leave unused candidate IDs pending.
- Snapshot regression covers pending reservation round-trip. The integration regression materialises a cut, seasons/removes its pieces, then proves the old empty range cannot be reused by another live log and that source, stocks and allocator remain unchanged.

Commits:
- Materials reservation tracking and stale-range rejection: `f54fdf6bcb375ca8293b7558cafd14f3906011ec`
- Selected-plan reservation and no-unused-range correction: `c660e6ca68c1b611950df23ada9246709aaa699f`
- Snapshot and consumed-range regressions: `19edbf731a1017781368306916892e74d89648e6`

A fresh exact-head CI run is required. The predecessor workflow #42388369118 was still running on the earlier head; its results do not verify these commits. No local Cargo result is claimed while custom OpenCommander access returns HTTP 429. No replay format, RNG, append-only event log or live island state was modified; the new pending-reservation state is optional in snapshots and must still pass week/year replay acceptance before merge.


### Legacy active-haul snapshot compatibility — 8 November 2026

The reservation set is an additive optional field, but an older IslandLife snapshot can contain an active `HaulAction::CrossCut` and the allocator watermark without the new set. To keep those saves resumable without weakening the public ledger boundary:

- Added a crate-private migration helper that reconstructs a range only from an active haul task whose source log still exists, whose full range is below the persisted allocator watermark, and whose destination IDs are unoccupied. Partial or conflicting ranges fail closed.
- `finish_haul_task` invokes this helper before the normal strict `cross_cut_log` boundary. Direct callers cannot use the migration path.
- The Ute reservation regression now serializes a live haul, removes the new field to simulate a legacy snapshot, restores the island, executes the persisted CrossCut task, and verifies the later log and all fourteen pieces remain intact.
- The test occupies the excavator deliberately, so the selected plan is explicitly the Ute plan and only its fourteen-piece range is reserved; unused alternative-plan IDs are not leaked.

This keeps the existing IslandLife snapshot version readable while preserving fail-closed validation at the ledger API. Exact-head Rust, desktop/backend and week/year acceptance are required; no snapshot digest or reference pin is changed by hand.

- The snapshot regression also asserts that an empty reservation set is omitted from serialized `MaterialLedger`, preserving the previous empty-state representation; a legacy active-haul snapshot with the field absent is separately restored through the task-scoped migration path.


### Exact-head formatting correction — 8 November 2026

Workflow [#42391475499](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42391475499) on predecessor head `083814b77bdf28bec22d52e49473b8166a66207d` reached the code and failed only the `cargo fmt --all -- --check` job on one line wrap in `restore_pending_cross_cut_ids`. The exact rustfmt output is applied in commit `5fe70a82ccd08348a3acd611760b9c554d702da4`. The same workflow's backend, desktop and week/year jobs were still running at inspection; their results do not verify the new head. Fresh exact-head CI is required.


## Bound cross-cut materialisation — 8 November 2026

A persisted haul task or direct caller could request an extreme u32 piece count.
The pure Log::cross_cut API would allocate one Log per piece, and the ledger
reservation path would expand the same range into a BTreeSet. A malformed
snapshot could therefore turn a small input into an unbounded allocation.

- Added MAX_CROSS_CUT_PIECES = 10,000 as a generous fail-fast bound.
- Log::cross_cut rejects larger direct requests before constructing the vector.
- MaterialLedger::reserve_log_ids refuses larger ranges before extending the set.
- Legacy reservation restoration and the public cross_cut_log boundary return
  false for oversized ranges before iterating or mutating.
- Haul planning returns the explicit LabourError::CrossCutLimitExceeded
  rather than attempting an oversized split.
- Added regressions for direct pure-API rejection, reservation rejection, and
  no-mutation ledger refusal.

Commits:
- 46b5c581fcacd1f587e9d63604e0f13a5fc97c82 — typed labour error.
- b6233988d31d11dad2ae6772000607dd89b54f69 — pure cut and planner bound.
- 1be3fae2b3e01067b2bd99522c2590b9c20d353d — ledger reservation/materialisation guards.
- ce87b0e0621ce668e5a5c30a44adad878af7ae07 — regression tests.

Exact-head CI is required for formatting, Clippy, fast Rust tests, desktop,
installed backend and week/year reference acceptance. No local Cargo pass is
claimed while custom OpenCommander is rate-limited. No snapshot schema,
replay log, RNG, live island state or production infrastructure was changed.


## Follow-up: preserve the canonical week digest for empty reservations — 8 November 2026

The exact-head week reference-pack gate failed on head 680f757 while the year reference-pack gate passed. The week run retained all recorded physical acceptance values (27,597 audits, zero food/water shortfalls, 112 economy events, 200,000 estate trees, both founders' expected body carbon and NPP 471 g C/m2/yr), but the canonical digest changed from 29ed53bd... to 5c975e00....

The change was traced to the new reserved_log_ids bookkeeping field: serde already omitted an empty set from snapshots, but IslandLife::state_digest hashes MaterialLedger through Debug, whose derived representation included the empty field. This changed the hash despite no pending cross-cut reservation and no physical state change.

Correction:
- MaterialLedger now implements Debug explicitly.
- The pre-reservation field order and representation are retained byte-for-byte while reserved_log_ids is empty.
- Non-empty pending reservations remain part of Debug/digest because they change which future cross-cuts are valid.
- Added regressions pinning the exact legacy empty-ledger representation and proving pending IDs remain represented.

Commits:
- cb9d89bb6d5b68e6f33dc3473607558985d60242 — compatibility-preserving Debug.
- 6962194d2873d733783d6e709d9ef4925e79f199 — regression tests.

The previous exact-head workflow 42394121853 failed its week reference-pack acceptance because of the digest drift; the year pack passed. Formatting and Clippy passed, the installed backend, dashboard, computer service and repository scripts passed, fast Rust tests were still running, and desktop acceptance was still in progress. The new current head needs a fresh exact-head CI run. Do not repin WEEK_DIGEST unless the canonical week test still fails after this representation correction and the remaining state difference is independently audited.

No simulation behaviour, RNG, replay hash, snapshot schema, physical parameter, production state or infrastructure was changed by this digest-compatibility correction.


## Exact-head fast-test repair — 8 November 2026

The exact-head fast-test workflow `#42395540972` reached the Rust suite and
reported **880 passed / 1 failed**. The failure was in
`haul_planning_reserves_ids_for_a_conserved_ute_recut_before_another_log_arrives`:
the fixture used a 10 kg log, so carrying fourteen very light pieces on foot
was correctly faster than using the Ute. The test's expected transport method
was wrong; the production planner was not shown to be wrong.

The fixture now derives the source mass from the founder's reference-pack
sustainable load (ten sustainable loads total). The length boundary still
forces fourteen pieces while each piece remains carryable and the complete
load fits the Ute payload, so the test exercises the intended selected-Ute
reservation path rather than relying on an unrealistic 10 kg log.

Codex also identified a separate P1 test expectation error: `MaterialLedger::new()`
sets `next_structure = 1`, but the exact Debug string expected zero. The
regression now pins the actual initialized value, preserving the intended
legacy empty-reservation representation.

Commits:
- The Ute fixture correction and Debug expectation correction are recorded
  in the current PR branch; exact SHAs are in the PR commit list.

The failed workflow is evidence for the predecessor head only. Fresh exact-head
formatting, Clippy, fast Rust tests, desktop/backend and week/year reference-pack
acceptance are required. No local Cargo pass is claimed. No reference digest,
snapshot schema, replay log, RNG or simulation parameter was changed.


## Follow-up: reject zero-piece cross-cut operations — 8 November 2026

A final boundary audit found that three paths silently normalised a persisted or
direct `pieces = 0` request to one piece: the pure `Log::cross_cut` API,
`MaterialLedger::cross_cut_log`, and legacy active-task reservation restore.
That made malformed input look like a valid one-piece operation. The allocator
also accepted an empty `reserve_log_ids(0)` request.

The boundary now fails closed:
- `Log::cross_cut(0, ...)` asserts before allocation.
- `reserve_log_ids(0)` asserts before advancing the allocator.
- `cross_cut_log(..., 0, ...)` returns false without removing the source or
  consuming a pending reservation.
- `restore_pending_cross_cut_ids(..., 0, ...)` returns false, so the
  `finish_haul_task` fail-fast assertion rejects a malformed persisted task.

Regression coverage includes direct zero-piece rejection, empty reservation
rejection, no-mutation ledger refusal followed by successful use of the same
reserved ID, and rejection of zero-piece legacy reservation restoration.

The current PR also corrects two predecessor test defects:
- The Ute reservation fixture now derives source mass from ten sustainable
  loads, leaving length as the reason fourteen pieces are required while the
  Ute is actually quicker than hand carrying.
- The empty-reservation Debug expectation now correctly pins
  `next_structure: 1` for `MaterialLedger::new()`.

Commits:
- `31df86c68c1f15567e1893e7a1ea69582188cdc3` — representative Ute fixture.
- `90d70b852aee56d1b22f4b08e82d4728f35d0afd` — initialized Debug counter.
- `6c6f3fc98b2115fc331feca3e6ecab5c6931e714` — pure cross-cut zero guard.
- `fed236478e53e444d3cdca99246026b37207e72d` — ledger and restore guards.
- `595e8be430d5c290d6924410ae933e1a4d628e7b` — ledger regressions.
- `3159dc0c5776de4b04ad2d7efccbe989d7a164e6` — legacy-task regression.
- `e51834818b426ecbe8200a1a74ec341a26def2cb` and
  `b06d511c729ecba4e247d876e8b9ce5b339d2b24` — prior handoff/checklist
  corrections.

The earlier workflow `#42398472485` was superseded while its fast-test job
was running and was cancelled after the new commits. It does not verify this
head. Fresh exact-head formatting, Clippy, fast Rust tests, desktop/backend and
week/year reference acceptance are required. No local Cargo pass is claimed.
No digest pin, simulation parameter, RNG, replay hash or snapshot schema was
changed by the zero-piece guard.


### Rust formatting correction — 8 November 2026

Exact-head workflow [#42398863262](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42398863262) reached `cargo fmt --all -- --check` and failed on one line wrap in the new zero-piece legacy-restore assertion. Applied the exact rustfmt output in `a7b0ddd04655222e75d39a32af85c78e108e817a`. This was a formatting-only failure; fast tests were not run in that job. Other application/reference jobs were still in progress at inspection. Fresh exact-head CI is required; no format or test pass is claimed for the corrected head.


## Follow-up: reserve haul IDs only after founder acceptance — 8 November 2026

Codex review found that `plan_haul` reserved the selected CrossCut range before `work` asked the founder whether to haul or fell. A founder who declined the haul therefore leaked a pending ID range, even though no haul job owned it.

Correction:
- `plan_haul` now previews the allocator's next ID and builds candidate plans without mutating the ledger.
- The work loop calls `reserve_haul_plan` only after `chooses_to_haul` returns true.
- `reserve_haul_plan` verifies the accepted job still owns the previewed next ID and reserves the largest actual CrossCut range, preserving the 14-piece conserved-remainder case.
- The accepted-plan regression now proves preview leaves the allocator unchanged and explicit acceptance reserves the exact range.
- New work-loop regression forces Gem-D to prefer felling, proves a valid haul candidate exists, runs the real work decision, and verifies no ID reservation leaks when the haul is declined.

Implementation commit: `5281c014610cb338923efc50e7da640b55897bf4`.

Fresh exact-head rustfmt, Clippy, fast tests, desktop/backend and week/year reference acceptance are required. No local Cargo result is claimed. The single-writer allocator and deterministic choice model are unchanged; only the point at which the reservation is committed has moved after acceptance.


## Exact-head CI repair — 8 November 2026

The run on the prior merge head failed Clippy while compiling the new haul-decline regression: the test module referred to the sibling module as `super::choice::Work`, but `super` inside `life::tests` is `life`, not `regional`. The two test references now use the compiler-confirmed `crate::regional::choice::Work` path.

- Production/test-path correction: `131332c3db69dbec56b6b7707bb4ebda76d5b84c`.
- The first exact-head workflow after that correction reached rustfmt and found one line-wrap difference in the second preference assertion.
- Applied the exact formatter layout in `52ee736225238076807f4c713926bc787d629cad`.
- New exact-head workflow: [#42400504685](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42400504685). It was pending at handoff time; no format, Clippy or Rust-test pass is claimed for this head.

The fix changes only test-module name resolution and formatting. It does not change timber logic, material IDs, snapshot layout, replay hashes, RNG, simulation state or production infrastructure.


## Immutable haul preview and declined-plan regression — 8 November 2026

IslandLife::plan_haul now takes &self rather than &mut self. Candidate
planning is therefore enforced by Rust's type system as a read-only preview;
the work loop reserves the selected CrossCut range only after the founder
accepts the haul.

The declined-haul regression now exercises the complete work-loop decision:
Gem-D is made to prefer felling, the candidate haul is declined, an attempted
cross-cut using the previewed range is refused without removing the source log,
and the next real felled log receives the still-unreserved preview ID. This
proves both the allocator watermark and pending-reservation set remain
untouched by a declined candidate.

No simulation parameters, physical stocks, snapshot schema, replay format,
RNG, append-only histories, digest pins or external infrastructure changed.
Fresh exact-head Rust/desktop/backend/week/year CI is required.
