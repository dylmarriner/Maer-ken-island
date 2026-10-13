# Working practices

## The realism standard

`docs/island/REALISM.md` is the rule; in short:

- **Every real-world number is sourced** and lives in a reference pack under
  `fixtures/reference/<domain>/`, with its citation and licence. Code reads it from the pack
  (`ReferenceLibrary`, `LabourTable`) or holds a constant that a test compares against the pack
  (for example `tests/hydro_grid.rs::the_plant_and_line_use_the_reference_pack_values`).
- **No invented values, no mocks, stubs or placeholders.** A value nobody could fetch is not
  guessed: the gap is recorded as a deviation and the work waits.
- **Departures from reality are registered** in `docs/island/DEVIATIONS.md`: reality, model,
  expected error, why, owning phase, status (Open / Accepted / Resolved).
  `crates/mk_core/tests/deviation_register.rs` enforces the table's format.
- **Derivations are allowed and said out loud**: for example, a Turgo takes the Pelton's curve
  shape at its own best efficiency, and a cross-cut costs the felling pack's cut time for the same
  section. Each such step is written in the code's doc comment and in the deviation row.
- **When a source is ambiguous, test against a worked example.** The RETScreen Pelton speed
  formula came through PDF extraction in two readings; ESHA's worked example (400 m, 0.42 m³/s,
  about 1,000 rpm) settled it, and a test pins it.

## The audited ledger

Every material movement in `IslandLife` happens inside `IslandLife::audited`, which fails the
step if the ledger's net flows do not equal the change in material carbon, human carbon and
material water. New matter (logs, say) must be counted in `MaterialLedger::stock_of` and booked
with `FluxEntry`s from where it came (logs: `BiomassCarbon → MaterialCarbon`,
`SoilWater → MaterialWater`).

## Digests: when and how to re-pin

The week test (`island_life_acceptance.rs::slow_a_week_on_the_island_matches_the_reference_packs`)
checks the founders' week against the reference packs, **then** compares the final state digest
with `WEEK_DIGEST`. Any change to physics, life or serialized state moves the digest.

1. Run it: `cargo test --release -p mk_engine --test island_life_acceptance -- --ignored --nocapture`.
2. Read every printed audit: 433 audits closed, no food or water shortfall, body carbon at 1.00x,
   eating about 43-47 min/day, drinking 1.4-1.8 L/day, forest NPP inside the reference range,
   intimacy about 65/yr in the four-week test.
3. Only if every deferred assertion passed and the movement is explained by your change, copy the
   new digest into `WEEK_DIGEST`.

`topology_hash_baseline.rs` pins the planetary engine. Island work should never move it.

## CI (`.github/workflows/ci.yml`)

- **fmt, clippy, fast tests**: `cargo clippy --workspace --exclude island_ui --all-targets -- -D warnings`,
  then `cargo test --workspace --exclude island_ui`. About 28 minutes; limit 45.
- **island_ui**: Bevy clippy, build, headless render and interaction smoke tests.
- **dependency audit**: `cargo-audit` on `Cargo.lock`, `npm audit` on the computer service.
- **nightly slow tier**: `cargo test --workspace --release -- --ignored slow_`.

The fast tier stays inside its limit because island test files bootstrap once
(`OnceLock`) and restore a copy per test, and the serialization crates build at opt-level 2 in
dev (`Cargo.toml`). Keep both when adding tests.

## Environment traps (cloud container)

- **ALSA headers are missing**, so anything that builds `island_ui` (or `--all-features`) fails.
  Use `--exclude island_ui` locally.
- **Release artifacts can go stale.** On 2026-10-12, `mk_island_api` kept an old `Person` without
  `height_m` in `target/release` and `island` failed to compile in release only. The fix was
  `cargo clean --release -p mk_island_api`.
- **Do not run two cargo builds at once and then trust the result**, and do not edit sources
  under a running build.
- **Filter test output for failures, not for a summary**: `grep -E "FAILED|panicked"`. A
  `sort | uniq | head` summary hid the run's tail once.
- **The network**: the container's proxy blocks most hosts (403 on CONNECT). WebSearch works, and
  the TinyFish fetcher reads PDFs and pages. Firecrawl worked until its credits ran out.
- **JSON packs**: insert rows textually; a Python `json.dump` round trip reformats a whole pack.

## Git and PRs

- Work on a feature branch; PRs go to `main`. A PR is merged only when CI is green.
- Commit messages end with the co-author and session lines the session's instructions give.
- Every GitHub comment ends with the Claude Code footer.

## Owner decisions on record

| Date | Decision |
|---|---|
| 2026-09-11 | Realignment: wall-clock ticks, real RNG and floating point allowed; no intelligence ceiling for humans (`docs/decisions.md` in the parent project). |
| 2026-10-02 | Phase 4b: hydro by the estate, oil refinery and gas plant, a town of about 200, Gem-D and Gem-K own the enterprises (plan defaults table). |
| 2026-10-12 | D33 accepted as is. The estate gets a felling axe, a crosscut saw and a gold pan. The listed hosts are allowed. Phase 4b Tasks 1-3 to start. |
| 2026-10-12 | Felled wood must fall as a damp log and be moved by machine or piece by piece by hand to storage (built: D40). Merge PR #6 to `main`. |
| 2026-10-13 | Who lives where. The founders' house is the two founders' and the children they have, nobody else's. Estate staff -- maids, kitchen staff, farm hands, the staff manager or supervisor -- each have a separate 2-3 bedroom cottage on the estate. Everyone else lives in the town. Enforced so far: `create_human` refuses every room of the house (`EstateLayout::is_house_space`), the creator is offered only spaces outside it, and `--import-0b` lands outside it. Not built yet: the cottages and the town (`WORK_REMAINING.md` 4d, 4g). |
