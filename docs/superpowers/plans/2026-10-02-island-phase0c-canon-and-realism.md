# Island Phase 0c: Earth-like Marr'Kena Canon and Realism Standard

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give the island a physically self-consistent planet that real humans can live on, and a realism standard every later phase must meet: every parameter sourced, every system validated against real-world data, every remaining simplification declared.

**Owner decision (2026-10-02):** "Earth-like Marr'Kena": keep the canonical planet's name, size (radius 19,113 km), 36-hour day, 27° axial tilt and two moons; give it Earth surface gravity, an Earth atmosphere and Earth-like sunlight; adjust everything else so the physics is consistent.

**Architecture:** Upstream `CanonLocked::default()` is unchanged. The island loads its own canon from `fixtures/island/canon.json` through `CanonLocked`, with every derived value recomputed and a new physical-consistency validator. All later phases use the island canon.

**Tech Stack:** Rust, `mk_core::canon`, serde_json.

**Depends on:** Phase 0. Runs before Phase 1 (Phase 0b does not depend on it).

## Upstream canon problems (audited)

- **Gravity vs humans.** Upstream: radius 1.9113e7 m with surface gravity 19.62 m/s² (2 g). The human runtime models Earth humans, who could not live normally at 2 g.
- **Year vs orbit.** Upstream star mass 2.2270864e30 kg (1.12 M☉) and semi-major axis 1.75e11 m give a Kepler year of 37.74 Ms, not the canon 46.656 Ms (24% off).
- **Outer moon unstable.** At Earth-like planet mass, Hahn's 135-day orbit lies at 0.62 of the planet's Hill radius; prograde satellites are long-term stable only inside about 0.49 (Domingos, Winter & Yokoyama 2006).
- **No intrinsic body clock.** The human circadian hormone follows `daylight_fraction` directly (`humans/neurochemistry.rs:73`), so humans would lock perfectly onto a 36-hour day. Real humans have an intrinsic period of about 24.2 h (Czeisler et al. 1999) and cannot entrain to days far outside roughly 23.5–24.7 h.
- **Missing systems the spec promises or reality requires:** no earthquakes; human actions take no real time and cost no real energy; vehicles, power tools and computers run with no fuel or electricity.

## Island canon (derived values)

| Quantity | Value | Derivation |
|---|---|---|
| Planet radius | 1.9113e7 m | kept |
| Surface gravity | 9.80665 m/s² | Earth standard gravity |
| Planet mass | 5.370e25 kg (8.99 M⊕) | g·R²/G |
| Bulk density | 1,836 kg/m³ | **declared exception** (see below) |
| Escape velocity | 19.36 km/s | √(2gR) |
| Rotation period | 129,600 s (36 h) | kept; equator speed 927 m/s, centrifugal 0.46% g |
| Obliquity | 27.0° | kept |
| Star mass | 2.2271e30 kg (1.12 M☉) | kept |
| Star luminosity | 6.017e26 W (1.57 L☉) | main-sequence L ∝ M⁴ |
| Insolation at orbit | 1,361 W/m² | Earth value |
| Semi-major axis | 1.8757e11 m (1.254 AU) | √(L / 4πS) |
| Orbital period | 41,865,187 s (323.0 local days, 484.6 Earth days) | Kepler's third law |
| Equilibrium temperature | 254.6 K (Bond albedo 0.30) | Stefan–Boltzmann; greenhouse brings the mean near 288 K |
| Surface pressure | 101,325 Pa | Earth |
| Atmosphere | N₂ 78.08%, O₂ 20.95%, Ar 0.93%, CO₂ 280 ppm (pre-industrial) | Earth dry air |
| McKenz | period 3,888,000 s (45 d), orbit 1.111e9 m (0.30 R_Hill) | kept; stable |
| Hahn | period ≈ 6,912,000 s (80 d), orbit ≈ 1.6e9 m (≈0.44 R_Hill) | shortened from 135 d for stability; final value set by Task 1's stability check |

**Declared exception.** A rocky planet of this radius would have a surface gravity of roughly 6–7 g; Earth gravity at this size implies a bulk density one third of Earth's, like the ice-rich moons Ganymede and Titan. The island keeps the owner's size and Earth gravity, and models the crust, mantle, heat flow and tectonics with Earth-like rock properties. This is the only place the simulation knowingly departs from physical consistency, and it is recorded first in the deviation register.

## Global Constraints

- No simulated quantity uses an Earth-specific time constant where the planet differs: days, years and tidal periods come from the island canon; human and animal biology run in SI seconds (the human runtime already ages in Earth-second years, `rates.rs:58`, `lifecycle.rs:946` — keep it that way).
- Every parameter added from Phase 0c onward cites a source or a derivation in code.
- Changes to upstream human biology are made upstream first (`dylmarriner/Maer-Ken`, same owner) and synced; island-only divergences are allowed only when upstream cannot take them, and are recorded.

---

### Task 1: Island canon and physical-consistency validator

**Files:**
- Create: `fixtures/island/canon.json`
- Modify: `crates/mk_core/src/canon/mod.rs` (`CanonLocked::load(path)`, moon mass fields if absent), `crates/mk_core/src/canon/validator.rs` (`validate_physical_consistency`)
- Test: `crates/mk_core/tests/island_canon.rs`

**Interfaces:**
- Produces: `CanonLocked::load(path: &Path) -> Result<CanonLocked, CanonError>` recomputing every derived value; `validate_physical_consistency(&CanonLocked) -> Vec<ConsistencyIssue>` checking: g = GM/R² (±0.1%), Kepler's third law for the planet and each moon (±0.5%), insolation = L/(4πa²) (±0.5%), each moon inside 0.49 R_Hill and outside the Roche limit, moons not in a destabilising mean-motion resonance (period ratio not within 2% of 2:1 or 3:2), and escape velocity sufficient to retain N₂/O₂ (Jeans parameter > 50 at a 1,000 K exobase).
- Moon masses and radii: keep upstream values if defined; otherwise choose values giving tidal ranges of the same order as Earth's (documented derivation) and record them.

- [ ] **Step 1:** Write tests: upstream default canon reports the gravity/year/Hahn issues; the island canon reports none except the declared density exception (reported as `Declared`, not an error); derived values match the table above within tolerance; loading is deterministic and the canon digest is stable.
- [ ] **Step 2:** Run `cargo test -p mk_core --test island_canon`; expect FAIL.
- [ ] **Step 3:** Implement loading and the validator; set Hahn's period to the longest period that passes the stability and resonance checks, rounded to whole days.
- [ ] **Step 4:** Re-run, plus `cargo test -p mk_core --lib canon::`; expect PASS (upstream default canon behaviour unchanged).
- [ ] **Step 5:** Commit `feat(canon): earth-like marr'kena island canon with consistency checks`.

### Task 2: Realism standard and deviation register

**Files:**
- Create: `docs/island/REALISM.md`
- Create: `docs/island/DEVIATIONS.md`

**Content:**
- **Standard:** (1) every parameter cites a source or derivation; (2) every subsystem has a validation suite against real-world reference data (Task 3), translated for planetary differences (36-h day, 323-day year, larger radius) by physical scaling, not by copying Earth numbers; (3) no game shortcuts — no free regeneration, instant actions, teleporting, infinite fuel or unpowered machines; (4) conservation of energy, water, carbon, oxygen, nitrogen and phosphorus closes every step; (5) resolution limits are stated, not hidden.
- **Deviation register:** one row per known simplification — what reality does, what the model does, expected error, why, and the phase that could remove it. Seed rows: the density exception; daily-mean upstream climate (Phase 2 adds a diurnal cycle); parameterised convection at 12 km; species evolved for Marr'Kena rather than Earth species (validated by functional traits instead); industrial equipment (turbines, generators, rigs, refinery and gas-plant units) imported before the simulation began, with finite spares (Phase 4b); the computer service bridges to the real internet.

- [ ] **Step 1:** Write both documents.
- [ ] **Step 2:** Add a CI check that every row in `DEVIATIONS.md` names an owning phase.
- [ ] **Step 3:** Commit `docs(island): realism standard and deviation register`.

### Task 3: Real-world reference data packs

**Files:**
- Create: `fixtures/reference/{climate,hydrology,ecology,geology,humans,labour}/*.json`, each with `source`, `licence`, `retrieved`, and the values used
- Create: `crates/mk_engine/src/validation/reference.rs` (typed loaders)
- Test: `crates/mk_engine/tests/reference_packs.rs`

**Required packs** (values, not raw datasets; each with citation):
- **Climate:** mid-latitude maritime climate structure — environmental lapse rate (~6.5 K/km), diurnal temperature range by surface type, windward/leeward orographic precipitation ratios from mountain ranges in westerlies (e.g. New Zealand Southern Alps), sea-surface temperature seasonal range at 40–45°, synoptic storm frequency and lifetime in mid-latitude storm tracks.
- **Hydrology:** runoff ratio vs aridity (Budyko curve), discharge–catchment-area scaling, flood frequency.
- **Ecology:** NPP ranges by biome (MODIS MOD17), forest allometry (height/diameter/biomass, Chave et al. 2014), stem densities by forest type, tree growth and mortality rates, soil carbon by biome, metabolic scaling (Kleiber's law) for animals.
- **Geology:** Gutenberg–Richter b-values, Omori aftershock decay, uplift and erosion rates at active margins, volcanic eruption frequency by VEI, USGS grade–tonnage models for each Phase-1 deposit kind.
- **Humans:** basal metabolic rate (Mifflin–St Jeor), activity energy costs (Compendium of Physical Activities, MET values), water needs and dehydration/starvation survival times, sleep need, intrinsic circadian period and entrainment range, walking speed and load carriage, gestation length distribution, fertility by age, mortality by age (life tables / Gompertz), growth curves (WHO).
- **Labour:** realistic productivity — trees felled per day by hand axe and by chainsaw, hand-mining rates, gold panning throughput and yields, smelting yields and fuel ratios, construction labour hours per m², vehicle fuel consumption and speeds on terrain.

- [ ] **Step 1:** Write loader tests: every pack parses, carries a source and licence, and covers the items listed.
- [ ] **Step 2:** Assemble the packs from published sources; record licences (use only values whose licence permits inclusion).
- [ ] **Step 3:** Run `cargo test -p mk_engine --test reference_packs`; expect PASS.
- [ ] **Step 4:** Commit `test(island): real-world reference data packs`.

### Task 4: Human realism validation (upstream first)

**Files:**
- Create: `crates/mk_engine/tests/human_realism.rs`
- Upstream PRs to `dylmarriner/Maer-Ken` for fixes; sync per `UPSTREAM.md`

- [ ] **Step 1:** Write validation tests comparing the human runtime against the Task-3 human pack under Earth-like conditions: daily energy expenditure for sedentary and labouring adults; time to death without water and without food; sleep duration; growth and ageing; gestation; fertility and mortality by age. Each test states its tolerance and source.
- [ ] **Step 2:** Add a circadian test: under a 36-h light/dark cycle, the body clock must not lock to 36 h; it free-runs near 24.2 h with light-driven phase shifts limited to the published phase-response curve (Kronauer/Jewett–Forger model), producing realistic sleep disruption unless the human adopts an artificial schedule.
- [ ] **Step 3:** Run; record every failure in `docs/island/DEVIATIONS.md` with its measured error.
- [ ] **Step 4:** Fix failures upstream (circadian oscillator, energy expenditure, survival times, as needed), keeping cognition/behaviour code unchanged; sync; re-run until each test passes or its deviation is accepted by the owner.
- [ ] **Step 5:** Commit `test(humans): validate human runtime against real-world data`.

### Task 5: Gate 0c

- [ ] **Step 1:** Island canon passes `validate_physical_consistency` with only the declared exception; reference packs load; human realism results are recorded and either passing or accepted.
- [ ] **Step 2:** Tick Gate 0c in the program plan.
