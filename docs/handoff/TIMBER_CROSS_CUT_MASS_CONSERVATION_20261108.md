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
