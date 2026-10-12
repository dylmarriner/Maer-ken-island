# Handoff: Maer-Ken Island

Written 2026-10-12, at the merge of PR #6 into `main`. This folder is for whoever picks the
island up next, human or agent. Read it in this order:

1. **[STATE_OF_THE_ISLAND.md](STATE_OF_THE_ISLAND.md)**: what exists, how the simulation is put
   together, and where each system lives in the code.
2. **[OPEN_WORK.md](OPEN_WORK.md)**: what is not done, in priority order, with what each item
   needs (sources, decisions, files).
3. **[WORKING_PRACTICES.md](WORKING_PRACTICES.md)**: the rules the project holds itself to
   (realism, reference data, digests, CI), the environment's traps, and the owner's standing
   decisions.
4. **[SESSION_2026-10-12.md](SESSION_2026-10-12.md)**: what the last working session changed, why,
   and what it measured.

## The authoritative documents (do not duplicate them, read them)

| Document | What it is |
|---|---|
| `docs/ISLAND_SYSTEM_STATUS.md` | The status of every system with its evidence (tests, benchmarks) and the "Not done" list. Kept current. |
| `docs/island/DEVIATIONS.md` | Every known departure from reality (D1-D40), with expected error, reason, owning phase and status. A test enforces its format. |
| `docs/island/REALISM.md` | The realism standard: what counts as a sourced value and how deviations are recorded. |
| `docs/IMPLEMENTATION_READY_ROADMAP.md` | The phase order (0, 0b, 0c, 1-4, 4b, 5) and acceptance gates. |
| `docs/superpowers/plans/*.md` | The detailed implementation plans per phase. The Phase 4b plan carries 2026-10-12 status notes. |
| `docs/superpowers/specs/2026-10-01-maer-ken-island-regional-world-design.md` | The design spec. |
| `docs/island/DEPLOYMENT.md`, `RENDER_STACK.md`, `PLANETARY_DEPENDENCIES.md` | Running it apart, the renderer, and what the island borrows from the planetary engine. |
| `UPSTREAM.md` | What is ported from upstream Maer-Ken, and how the island adapts it. |
| `docs/TESTING.md` | The test tiers. |

## Thirty-second orientation

- A New-Zealand-sized island (2,400 x 1,920 km domain; 12 km coarse and 2 km medium cells, a
  4 km estate patch at 5 m) on Marr'Kena, a planet with Earth's gravity and air but a 36 h day
  and 27 degrees of obliquity (`fixtures/island/canon.json`).
- Physics: geophysics, climate with a diurnal cycle, weather with travelling systems, ocean,
  tides and hydrology, coupled every step (`crates/mk_engine/src/regional/physical.rs`).
- Life: ecology and biomes, 200,000 individual trees on the estate patch, the founders Gem-D and
  Gem-K as full simulated humans who eat, drink, sleep, work and love by their own choices, all
  of it booked through an audited carbon/water/oxygen ledger (`regional/life.rs`).
- Industry: the estate runs on a micro-hydro plant on its own river, with solar, battery and a
  diesel backup (`regional/industry/`, `regional/energy.rs`). Felled trees come down as green
  logs that founders move by excavator, ute or hand (`regional/timber.rs`).
- Backend `island serve`, a web dashboard, and a Bevy desktop app (`apps/`).

## Build and test in one screen

```bash
cargo fmt --all -- --check
cargo clippy --workspace --exclude island_ui --all-targets -- -D warnings
cargo test --release --workspace --exclude island_ui --no-fail-fast          # fast tier
cargo test --release -p mk_engine --test island_life_acceptance -- --ignored   # the week audit
cargo test --release -p mk_engine --test regional_physical_coupling -- --ignored  # the year
cargo test --workspace --release --no-fail-fast -- --ignored slow_            # whole slow tier
```

`island_ui` (Bevy) needs ALSA and graphics headers. CI builds it in its own job; a plain cloud
container cannot.
