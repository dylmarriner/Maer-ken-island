# Planetary dependencies in physical and living systems

Phase 2 Task 1 (`docs/superpowers/plans/2026-10-01-island-water-atmosphere.md`), 2026-10-07.

Every place where an imported system assumes it runs on the whole planet: spherical row area, `GridSpec::lat_rad`/`lon_rad`, row spacing from planet radius, longitude wrap, planet-wide means and totals, hard-coded cell distances and pole handling. Line numbers are against commit `6d624b5`; paths are relative to `crates/mk_engine/src/`.

Classes:

- **G — domain geometry.** Replace with `IslandDomain` (`latitude_rad_for_row`, `longitude_rad_for_col`, flat `cell_area_m2(level)`, `cell_size_m(level)`, non-wrapping neighbours).
- **Z — zonal background.** A planet-wide quantity the region cannot compute from its own cells; take it from `ZonalBackgroundState` (Task 2).
- **E — edge forcing.** Replace with `RegionalBoundaryState` ocean/atmosphere edge values.
- **U — unaffected.** Correct on any grid (analytic time phase, per-cell physics), or a wrap of time, not space.

## Phase 2 systems

### climate (`climate/mod.rs`)

| Line | Use | Class | Regional treatment |
|---|---|---|---|
| 126-139 | `ClimateState::global_temperature` weights rows by `cos(lat_rad)` | Z | CO₂ steps from the background's global temperature, never from regional cells. |
| 446-452 | `step_co2(previous.global_temperature(), forcing.volcanic_co2_mol_yr, …)` | Z | CO₂ is a background prognostic; the region reads `co2_ppm` from it. |
| 466 | `grid_spec.lat_rad(row)` for daily-mean insolation | G | `domain.latitude_rad_for_row(Coarse, row)`. |
| 472, 479-493 | `cos(lat)` area weight → `global_mean` radiative T and `mean_absorbed_flux_w_m2` | Z | Both come from the background. |
| 503-504 | Meridional transport relaxes to `0.6·T_rad + 0.4·global_mean` | Z | `global_mean` from the background (it is a planet-wide mean by construction). |
| 230, 327, 411 | Orbit phase `rem_euclid(1.0)`, declination, daily-mean insolation formula | U | Analytic; reused. |
| — | **Missing:** no lapse rate; elevation only changes heat capacity (`surface_heat_capacity_j_m2_k`). | new | Regional climate adds a lapse rate (~6.5 K/km), or 3,000 m ranges would be as warm as the coast. |
| — | **Missing:** daily-mean insolation only; no diurnal cycle. | new | Regional climate adds an hour-angle term (roadmap Phase 2 deliverable). |

### weather (`weather/mod.rs`)

| Line | Use | Class | Regional treatment |
|---|---|---|---|
| 195, 210-216 | Row spacing `π / nrows` (radians) for the meridional T gradient | G | Gradient over `cell_size_m(Coarse)` converted to K/rad through the planet radius, so `meridional_wind_from_gradient` keeps its units. |
| 201, 228 | `lat_rad(row)` for zonal wind and relative humidity | G | Domain latitude. |
| 234-236, 250-262 | `cos(lat)` area weights; precipitation rescaled so the **grid mean** equals global evaporation from `mean_absorbed_flux_w_m2` | Z | Must not be rescaled to the regional mean. The zonal background gives the band precipitation rate at the island's latitude; regional rainfall is distributed around that by moisture and orography (Task 3). |
| 203 | Coriolis read from a planetary field | G | `f = 2Ω sin(latitude_rad_for_row)` computed once per level. |
| 206 | `zonal_wind(lat, coriolis)` | U/E | Interior wind from the formula, blended toward edge `wind_u/v` near the edges. |
| — | **Missing:** no advection, no orographic rain, no travelling systems. | new | Tasks 3 (orographic) and 3b (synoptic). |

### ocean (`ocean/mod.rs`)

| Line | Use | Class | Regional treatment |
|---|---|---|---|
| 223 | SST set from climate surface temperature every step | U/E | Keep inside; edge cells relax to edge `sea_surface_temperature_k`. |
| 268 | `relative_humidity(grid_spec.lat_rad(row), true)` | G | Domain latitude. |
| 284-289 | Density gradient against `row - 1`, the southern neighbour (the comment calls it equatorward, which holds only in the northern hemisphere; the island is southern); row 0 has no neighbour | G/E | Gradient uses the domain's equatorward neighbour; the edge row reads boundary salinity/temperature density instead of 0. |
| — | Currents are local (wind + thermohaline), no advection, no wrap | U | Edge `inflow_m_s` adds the boundary current along edge cells (Task 4). |

### hydrology (`hydrology/mod.rs`)

| Line | Use | Class | Regional treatment |
|---|---|---|---|
| 168-189 | `downhill_neighbour` wraps longitude (`rem_euclid(nlon)`) | G | Non-wrapping 8-neighbourhood; flow leaving the domain is an edge outflow budget term. |
| 224-228, 393 | `cell_area_at_row_m2(row, planet_radius)` | G | Flat `domain.cell_area_m2(Medium)`. |
| 284 | `relative_humidity(grid_spec.lat_rad(row), false)` | G | Domain latitude. |
| 287 | Slope over a hard-coded 50 km cell distance | G | `cell_size_m(Medium)` (×√2 on diagonals); at 2 km slopes are 25× steeper than upstream assumes, so Horton infiltration on slopes must be checked against the catchments reference pack. |
| 233-251 | Routing moves water one cell per step | new | On a 2 km grid a 150 km river takes ~75 steps to reach the sea. "Real rivers" need flow accumulation (D8 over the medium grid, computed once per terrain) for discharge; per-step storage routing can stay. |
| 432-434 | Test water budget via `planet_radius_m` | G | Regional tests use the flat area. |

### tides (`tides/mod.rs`)

| Line | Use | Class | Regional treatment |
|---|---|---|---|
| 71-73, 115-116, 163, 277 | Orbit and rotation phases `rem_euclid` | U | Analytic time; reused. |
| 79-81, 150-158 | Degree-2 potential from `lat_rad`/`lon_rad` per cell | G | Domain latitude/longitude on Medium ocean/coastal cells; amplitude mapped onto regional cells (Task 4). |
| 87-100, 167-188 | Tidal dissipation power for the whole planet, ledgered into ocean heat | Z | Regional ledger takes the domain's area fraction of the planet total, or none (decision in Task 4; record it). |

### insolation (`insolation/mod.rs`)

| Line | Use | Class | Regional treatment |
|---|---|---|---|
| 63-78 | `lat_rad`/`lon_rad`, hour angle from sub-solar longitude | G | Domain latitude/longitude; `AstronomyForcing::sub_solar_longitude_rad` already exists. This gives the diurnal cycle. |
| 66 | `cell_area_at_row_m2` for total intercepted power | G | Flat cell area; total is the domain's, not the planet's. |
| 126 | `get_average_insolation` across the globe | Z | Not used regionally. |

### world tick (`world_integration.rs`)

| Line | Use | Class | Regional treatment |
|---|---|---|---|
| 130 | `spin_up_climate` is private | — | Make `pub(crate)` for the zonal background spin-up (recorded divergence, Task 2). |
| 520-554, 1240-1350 | Coriolis from `planet_state.coriolis` | G | Domain Coriolis per level. |

## Phase 3 consumers (sized now, changed in Phase 3)

| File:line | Use | Class |
|---|---|---|
| `biosphere/mod.rs:655-656` | Latitude as `(row+0.5)/nlat·π − π/2` (own formula, south-first) | G |
| `biosphere/mod.rs:662-664, 721-723` | Spherical band area `R²·Δλ·(sin N − sin S)` for NPP and land area | G |
| `biosphere/marine.rs:157-220` | Ekman transport and Coriolis with an equatorial minimum latitude; longitude wraps `(col ± 1) % nlon`; row spacing `π/nlat`; `cos(lat)` metric terms | G |
| `biosphere/habitat.rs` | No geometry (hash only) | U |
| `organisms/runtime.rs:252, 298-302` | Positions and movement wrap longitude | G |
| `organisms/runtime.rs:448` | Cell width from `cell_area_at_row_m2` | G |
| `perception.rs:152` | Social neighbourhood wraps longitude | G |
| `humans/autonomy.rs:263, 299` | Movement wraps longitude | G |
| `humans/mod.rs:1078` | Cell width from `cell_area_at_row_m2` | G |
| `humans/mod.rs:1087-1160` | Step-toward and target search wrap longitude | G |
| `humans/mod.rs:1850-1880` | Grid position ↔ latitude/longitude with **row 0 = 90° N** | G — **defect**: `GridSpec::lat_rad` and `IslandDomain` put row 0 at the south, so birthplaces are mirrored north–south on the planetary grid. Fix in Phase 3 Task 6 through the domain; record as an upstream defect. |
| `humans/reproduction.rs:499` | Menstrual cycle day `rem_euclid` | U (time) |

## Re-estimate (Task 1 Step 3)

Work counts below include tests, and a "unit" is about one focused working session.

| Task | Original scope | Found | Estimate |
|---|---|---|---|
| 2 Zonal background | 1-D bands reusing `step_climate` on `GridSpec::new(bands, 1)` | `step_climate` is already zonal on a 1-column grid; only CO₂/global terms need to stay inside it. Spin-up needs a visibility change. | 1 unit |
| 3 Regional climate/weather + resampling | Latitude, flat weights, background terms, orographic rain | Also lapse rate and diurnal cycle (both missing upstream), weather precipitation rescale must be replaced, not reused. | 2 units |
| 3b Synoptic systems | Travelling lows/highs | New code, no upstream equivalent. | 1 unit |
| 4 Ocean and tides | Edge relaxation, tide mapping | Density-gradient neighbour and tidal-heat ledger decisions. | 1 unit |
| 5 Hydrology | Port routing non-wrapping with flat area | Slope distance 25× different; flow accumulation needed for real rivers; active-cell iteration over ~67 k land cells. | 2 units |
| 6 Coupled tick + preview + year realism | Order, determinism, preview | Year-long run must fit the 8-16 GB / smooth-running budget: coarse 32 k cells × 3 systems + ~70 k active medium cells. | 1-2 units |

Total ≈ 8-9 units, against the plan's implied 6. The extra comes from adding the lapse rate, the diurnal cycle and flow accumulation; without them the Gate 2 realism checks (cooler mountains, wetter windward slopes, real rivers) cannot pass.
