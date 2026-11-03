# Person inspector section-identity validation

Date: 2026-11-04
Canonical repository: `dylmarriner/Maer-ken-island`
Existing PR: [#68](https://github.com/dylmarriner/Maer-ken-island/pull/68), branch `fix/island-client-human-id-20261101`

## Change

- Refuse person-inspector payloads containing empty/whitespace-only section IDs or duplicate section IDs. Ambiguous section identity must not be rendered as if it were valid data.
- Add regression coverage for duplicate and empty section IDs.
- Add the explicit `Option<Whereabouts>` type at the deserialization boundary. The first targeted compile exposed an inference error in this existing live-whereabouts validation path; the annotation fixes it without changing runtime semantics.
- This is a client/UI projection boundary change only. No simulation tick, seed, snapshot, append-only log, material ledger, source pack or replay digest was changed.

## Verified locally on OpenCommander `amd`

- `CARGO_BUILD_JOBS=2 cargo test -p island_ui --lib --offline --locked human_inspector::tests`: **10 passed, 0 failed**.
- `cargo fmt --all -- --check`: passed.
- `git diff --check`: passed.

## Remaining gates

- Verify exact-new-head hosted CI after push.
- PR #68 was still draft and reported `mergeable=false` at the last metadata check; reconcile its base/conflicts before any merge.
- Run slow deterministic week/year replay acceptance, reference-pack checks and carbon/water/material ledger closure. Continue sourced ecology/industry items from `WORK_REMAINING.md`.
- Keep the PR draft until those gates pass; no live simulation or governance changes are authorized by this change.
