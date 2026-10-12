# Open work (2026-10-12)

In priority order. Each item names its deviation (`docs/island/DEVIATIONS.md`), the files it
touches and, where the blocker is data, exactly which cited values are missing. **Never fill a
missing value from memory with a citation attached**: `docs/island/REALISM.md` exists to prevent
that, and an earlier session caught itself doing it (the water viscosity, 1.307 remembered
against 1.3063 measured).

## 1. Timber the estate can actually use (D40)

Felled logs reach the building workshop green (0.82 kg water per kg dry wood) and stay that way.
Next:

- **Drying.** Logs and pieces should lose water toward the equilibrium moisture content of the
  air they sit in. Needs a cited model: the Wood Handbook's EMC equation (Hailwood-Horrobin with
  Simpson's coefficients, Wood Handbook ch. 4) and a cited air-drying rate. FPL GTR-117 *Air
  Drying of Lumber* gives times to dry stock of a given thickness; whole logs with bark dry far
  slower. Water leaves by `MaterialWater → Atmosphere` in the ledger.
- **Seasoned wood into the stock.** Once a piece is air-dry it should join `Material::Wood` (the
  stock's composition is air-dry, 15% water) so building and burning can use it. Note that the
  stock does not count its bound water in `MaterialWater`; the conversion has to be booked
  consistently with that (see the D40 row and `materials.rs`).
- **Splitting** firewood (the Compendium has `chopping_splitting_wood`), and the two-person
  **crosscut saw** (needs both founders together).
- **The excavator's rigging.** It lifts with the shed's rope, not a grapple; attaching a sling
  is not timed (no cited figure was found). A cited rigging time, or a grapple added by owner
  decision, would close it.
- Routes are straight lines; slope (the patch terrain is in `EstateLayout::terrain_m`) and
  obstacles are ignored. Vehicles have no position of their own.

Files: `regional/timber.rs`, `regional/life.rs` (`work`, `plan_haul`, `finish_haul_task`),
`regional/materials.rs`, `fixtures/reference/labour/timber.json`.

## 2. Phase 4b Tasks 4-9: oil and gas, refinery, trades, money, town

Plan: `docs/superpowers/plans/2026-10-02-island-phase4b-energy-industry-town.md` (the defaults
table there is the owner's). Hydro (Tasks 1-3) is done. Each later task needs its slice of
Task 1's reference tables, which were not fetched:

- petroleum: reservoir recovery factors, Arps decline parameters (Task 4);
- refinery and gas plant: crude assays and product yields, energy and emissions per barrel, gas
  composition and processing yields (Task 5);
- staffing per plant type and size, occupational wage ratios (Tasks 6-7);
- household electricity, cooking and transport-fuel demand (Task 8).

Also D14 (methane leaks), D16 (fuel production), D19 (rigs), and **D39**: the hydro plant's wear,
overhaul and failure (needs cited overhaul intervals; restored by Task 6's maintenance labour)
and flood closure of the intake (needs a cited threshold).

## 3. Climate fidelity

- **D3 (rest): sea and land breezes, katabatic winds.** Needs a diurnal wind solver. The
  coastal damping of the day-night range is still the observed coastal/inland ratio.
- **D25: the SST season lags 1-2 months too long.** Turbulent air-sea heat flux (about 15-40
  W/m²/K; Frankignoul's work) would shorten it, but the ocean physics is shared with the
  planetary run, whose topology digest is pinned. Change it regionally, or re-pin with the
  owner's agreement.
- **D4: weather systems are parametric** (no occlusion, no warm fronts).
- **D38: the driest tenth of the island gets ~320 mm/yr** against NZ's driest basin at 360.
- **D29: energy is audited per module, not end to end.**

## 4. Living world

- **Phase 3 Task 1b: species.** `TreeInstance` has `PlantKind` only; fauna is absent.
- **D31: NPP ignores soil nutrients.** Needs regional soils.
- **D9: most actions still take one tick.** Felling and hauling are the exceptions.
- **Phase 3 Task 2: placers and hand mining.** Need a cited downstream placer-gold grade decay
  and a cited artisanal shaft size.

## 5. Phase 5 and the app

- A GPU frame-rate figure for `island-ui` (every frame so far was CPU-rasterised).
- The Creator page does not yet use the map.

## Housekeeping

- **Remove the Snyk GitHub App** (owner's action). It is out of quota and posts an error on every
  push; the `dependency audit` CI job replaced it.
- **Firecrawl credits are exhausted** (2026-10-12). WebSearch and the TinyFish fetcher still work
  for sources; the container's own network is blocked for most hosts.
