//! HYDROLOGY MODULE — PHASE 2
//!
//! Purpose
//! - Water cycle: precipitation, runoff, infiltration, evaporation
//! - Continental water storage and surface flow
//! - Deterministic hydrology based on climate and topography
//!
//! Invariants
//! - All arithmetic uses f64 for continuous quantities
//! - Water conservation: precipitation = evaporation + runoff + infiltration
//! - Per-cell water balance with storage depletion/recharge
//! - Same tick + weather + topography → same hydrology field (deterministic)
//! - Ledger-tracked: Precipitation → SurfaceRunoff + Infiltration + Evaporation
//!
//! Authority
//! - MAERKEN_PHASE_2_PHYSICAL_WORLD.md § 3.7
//! - PLANET_CONSTANTS.md § 8 (water distribution)
//! - NATURE_CONSTANTS.md (hydrologic properties)

use mk_core::canon::CanonLocked;
use mk_core::grid::Grid2;
use serde::{Deserialize, Serialize};

const HORTON_F0_MM_DAY: f64 = 1555.2;
const HORTON_FC_MM_DAY: f64 = 155.52;
const HORTON_K_DAY: f64 = 2.5;

/// Soil water content at a grid cell
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SoilWater {
    /// Soil moisture (fraction, 0 to 1)
    pub moisture_fraction: f64,
    /// Soil water storage (mm)
    pub storage_mm: f64,
}

impl SoilWater {
    /// Create new soil water state
    pub fn new(moisture: f64) -> Self {
        SoilWater {
            moisture_fraction: moisture.clamp(0.0, 1.0),
            storage_mm: moisture * 300.0, // 300 mm = field capacity
        }
    }
}

/// Hydrology state for a given tick
///
/// Type: Computed struct with persistent soil water
/// Determinism: same tick + weather + topography → same hydrology field
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HydrologyState {
    /// Soil water content (fraction 0-1) per grid cell
    pub soil_water: Grid2<SoilWater>,
    /// Surface outflow (mm/day) per grid cell: everything the cell passes
    /// downhill this step, including water that arrived from upstream.
    pub runoff: Grid2<f64>,
    /// Surface runoff (mm/day) the cell itself generated this step: its
    /// outflow minus the inflow that arrived from upstream. Flow
    /// accumulation (`mk_core::biomes::rivers::river_flow`) takes this, so
    /// upstream water is counted once. `None` only in a snapshot written
    /// before this field existed, until the next hydrology step.
    #[serde(default)]
    pub generated_runoff: Option<Grid2<f64>>,
    /// Infiltration rate (mm/day) per grid cell
    pub infiltration: Grid2<f64>,
    /// Evaporation rate (mm/day) per grid cell
    pub evaporation: Grid2<f64>,
    /// Surface water depth (mm) per grid cell
    pub surface_water: Grid2<f64>,
    /// Water that drained off each cell this step and reaches its downhill
    /// neighbour next step, mm over the *source* cell, row-major. Empty when
    /// nothing is in transit.
    #[serde(default)]
    pub routed_outflow_mm: Vec<f64>,
    /// This step's water budget, kg (see [`HydrologyBudget`]).
    #[serde(default)]
    pub budget: HydrologyBudget,
}

/// One hydrology step's water flows, kg, summed over the planet.
///
/// Surface water here means standing land water plus the routed outflow in
/// transit between cells; soil water is every cell's soil storage.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct HydrologyBudget {
    /// Rain onto land, atmosphere → surface water.
    pub precipitation_kg: f64,
    /// Surface water → soil water.
    pub infiltration_kg: f64,
    /// Soil water above field capacity returned to the surface.
    pub soil_excess_kg: f64,
    /// Surface water → atmosphere.
    pub surface_evaporation_kg: f64,
    /// Soil water → atmosphere.
    pub soil_evaporation_kg: f64,
    /// Soil water → groundwater recharge (leaves the modelled column).
    pub deep_drainage_kg: f64,
    /// Surface water (arriving runoff, or standing water on a cell that is
    /// now sea) → ocean.
    pub surface_to_ocean_kg: f64,
    /// Ocean → soil water of sea cells, whose soil is held saturated
    /// (negative when a sea cell's soil held more than saturation).
    pub ocean_to_sea_soil_kg: f64,
}

impl HydrologyState {
    /// Minimal constructor to create a HydrologyState with default soil water
    pub fn new(grid_spec: &mk_core::grid::GridSpec, default_moisture: f64) -> Self {
        HydrologyState {
            soil_water: Grid2::new(grid_spec, SoilWater::new(default_moisture)),
            runoff: Grid2::new(grid_spec, 0.0),
            generated_runoff: Some(Grid2::new(grid_spec, 0.0)),
            infiltration: Grid2::new(grid_spec, 0.0),
            evaporation: Grid2::new(grid_spec, 0.0),
            surface_water: Grid2::new(grid_spec, 0.0),
            routed_outflow_mm: Vec::new(),
            budget: HydrologyBudget::default(),
        }
    }
}

/// Soil field capacity (mm of water the root zone holds).
pub(crate) const FIELD_CAPACITY_MM: f64 = 300.0;
/// Soil moisture fraction above which evaporation is not moisture-limited.
pub(crate) const UNLIMITED_EVAPORATION_MOISTURE: f64 = 0.7;
/// Fraction of soil storage lost to deep drainage (groundwater recharge)
/// per day.
pub(crate) const DEEP_DRAINAGE_PER_DAY: f64 = 0.002;
/// Rate (per day) at which standing water drains off a cell on flat
/// ground; steeper slopes drain faster.
pub(crate) const SURFACE_DRAINAGE_PER_DAY: f64 = 0.3;
const R_DRY_AIR: f64 = 287.05;
const EVAPORATION_TRANSFER_COEFFICIENT: f64 = 1.2e-3;
const MIN_EVAPORATION_WIND_M_S: f64 = 1.0;
const SECONDS_PER_DAY: f64 = 86_400.0;

/// Horton infiltration capacity (mm/day) of soil at `soil_moisture` on
/// ground of `slope`: high when dry, falling toward the saturated rate as
/// the soil wets, and reduced on slopes where water runs off first.
pub(crate) fn infiltration_capacity(soil_moisture: f64, slope: f64) -> f64 {
    let wetness = soil_moisture.clamp(0.0, 1.0);
    let horton =
        HORTON_FC_MM_DAY + (HORTON_F0_MM_DAY - HORTON_FC_MM_DAY) * (-HORTON_K_DAY * wetness).exp();
    let room = (1.0 - wetness).max(0.05);
    horton * room / (1.0 + 8.0 * slope.max(0.0))
}

/// Potential evaporation (mm/day) from a wet land surface: the bulk
/// aerodynamic formula with land-surface relative humidity.
pub(crate) fn potential_evaporation(
    temperature_k: f64,
    wind_m_s: f64,
    rh: f64,
    pressure_pa: f64,
) -> f64 {
    if temperature_k <= 0.0 {
        return 0.0;
    }
    let air_density = pressure_pa / (R_DRY_AIR * temperature_k);
    let q_sat = super::weather::saturation_specific_humidity(temperature_k, pressure_pa / 100.0);
    air_density
        * EVAPORATION_TRANSFER_COEFFICIENT
        * wind_m_s.max(MIN_EVAPORATION_WIND_M_S)
        * q_sat
        * (1.0 - rh).max(0.0)
        * SECONDS_PER_DAY
}

/// The lowest of a cell's 8 neighbours (longitude wraps), if it is lower
/// than the cell: where its surface water flows. `None` for a pit, where
/// water pools.
fn downhill_neighbour(topography: &Grid2<f64>, row: usize, col: usize) -> Option<(usize, usize)> {
    let (nlat, nlon) = (topography.nlat(), topography.nlon());
    let here = *topography.get(row, col);
    let mut best: Option<((usize, usize), f64)> = None;
    for d_row in [-1i64, 0, 1] {
        for d_col in [-1i64, 0, 1] {
            if d_row == 0 && d_col == 0 {
                continue;
            }
            let r = row as i64 + d_row;
            if r < 0 || r >= nlat as i64 {
                continue;
            }
            let c = (col as i64 + d_col).rem_euclid(nlon as i64);
            let height = *topography.get(r as usize, c as usize);
            if height < here && best.is_none_or(|(_, lowest)| height < lowest) {
                best = Some(((r as usize, c as usize), height));
            }
        }
    }
    best.map(|(cell, _)| cell)
}

/// Step hydrology forward from `previous` over `dt_seconds`.
///
/// A daily-rate bucket model per land cell, integrated over the interval:
/// rain plus the previous step's runoff arriving from uphill neighbours
/// collects as standing water; it infiltrates up to the Horton capacity
/// and the soil's remaining room; potential evaporation draws first on
/// standing water and then on soil water (moisture-limited below
/// `UNLIMITED_EVAPORATION_MOISTURE`); soil loses a little to deep
/// drainage and any excess over field capacity returns to the surface;
/// standing water drains downhill (faster on slopes), and pools as a lake
/// where no neighbour is lower. Ocean cells (elevation ≤ 0) are saturated
/// and hold no standing land water.
pub fn step_hydrology(
    canon: &CanonLocked,
    previous: &HydrologyState,
    weather_state: &super::weather::WeatherState,
    climate_state: &super::climate::ClimateState,
    topography: &Grid2<f64>, // elevation (m)
    dt_seconds: f64,
    grid_spec: &mk_core::grid::GridSpec,
) -> HydrologyState {
    let grid_rows = grid_spec.nlat;
    let grid_cols = grid_spec.nlon;
    let dt_days = dt_seconds.max(0.0) / SECONDS_PER_DAY;

    let mut soil_water = Grid2::new(grid_spec, SoilWater::new(1.0));
    let mut runoff = Grid2::new(grid_spec, 0.0);
    let mut generated_runoff = Grid2::new(grid_spec, 0.0);
    let mut infiltration = Grid2::new(grid_spec, 0.0);
    let mut evaporation = Grid2::new(grid_spec, 0.0);
    let mut surface_water = Grid2::new(grid_spec, 0.0);
    let mut routed_outflow_mm = vec![0.0; grid_rows * grid_cols];
    let mut budget = HydrologyBudget::default();
    let area = |row: usize| {
        grid_spec
            .cell_area_at_row_m2(row, canon.planet_radius_m)
            .unwrap_or(0.0)
    };

    // The water that drained off each cell during the previous step arrives
    // at its downhill neighbour now, rescaled from the source cell's area to
    // the destination's.
    let mut inflow_mm = Grid2::new(grid_spec, 0.0);
    if previous.routed_outflow_mm.len() == grid_rows * grid_cols {
        for row in 0..grid_rows {
            for col in 0..grid_cols {
                let sent_mm = previous.routed_outflow_mm[row * grid_cols + col];
                if sent_mm <= 0.0 {
                    continue;
                }
                let sent_kg = sent_mm * area(row);
                match downhill_neighbour(topography, row, col) {
                    Some((r, c)) if *topography.get(r, c) > 0.0 && area(r) > 0.0 => {
                        *inflow_mm.get_mut(r, c) += sent_kg / area(r);
                    }
                    // Reaches the sea, or the terrain changed under it.
                    _ => budget.surface_to_ocean_kg += sent_kg,
                }
            }
        }
    }

    for row in 0..grid_rows {
        for col in 0..grid_cols {
            let elevation = *topography.get(row, col);
            let cell_m2 = area(row);
            let prior_soil = previous
                .soil_water
                .get_safe(row, col)
                .copied()
                .unwrap_or(SoilWater::new(0.5));
            if elevation <= 0.0 {
                // Sea: its soil is held saturated and it holds no standing
                // land water; any that was here (the coast moved) joins it.
                budget.ocean_to_sea_soil_kg +=
                    (FIELD_CAPACITY_MM - prior_soil.storage_mm) * cell_m2;
                budget.surface_to_ocean_kg += previous
                    .surface_water
                    .get_safe(row, col)
                    .copied()
                    .unwrap_or(0.0)
                    .max(0.0)
                    * cell_m2;
                continue;
            }
            let precip = weather_state
                .precipitation
                .get_safe(row, col)
                .copied()
                .unwrap_or(0.0)
                .max(0.0);
            let temp = *climate_state.surface_temperature.get(row, col);
            let wind_speed = weather_state.wind.get(row, col).speed();
            let rh = super::weather::relative_humidity(grid_spec.lat_rad(row), false);
            let downhill = downhill_neighbour(topography, row, col);
            let slope = downhill
                .map(|(r, c)| ((elevation - *topography.get(r, c)) / 50_000.0).max(0.0))
                .unwrap_or(0.0);

            // Soil above field capacity (a flood, an injection) returns its
            // excess to the surface.
            let mut storage = prior_soil.storage_mm.max(0.0);
            let excess = (storage - FIELD_CAPACITY_MM).max(0.0);
            storage -= excess;
            let mut standing = previous
                .surface_water
                .get_safe(row, col)
                .copied()
                .unwrap_or(0.0)
                .max(0.0)
                + excess
                + precip * dt_days
                + *inflow_mm.get(row, col);

            // Infiltration.
            let capacity = infiltration_capacity(storage / FIELD_CAPACITY_MM, slope) * dt_days;
            let infiltrated = standing
                .min(capacity)
                .min(FIELD_CAPACITY_MM - storage)
                .max(0.0);
            standing -= infiltrated;
            storage += infiltrated;

            // Evaporation: standing water first, then moisture-limited soil.
            let potential =
                potential_evaporation(temp, wind_speed, rh, canon.sea_level_pressure_pa) * dt_days;
            let from_surface = standing.min(potential);
            standing -= from_surface;
            let moisture_limit =
                (storage / (UNLIMITED_EVAPORATION_MOISTURE * FIELD_CAPACITY_MM)).min(1.0);
            let from_soil = ((potential - from_surface) * moisture_limit)
                .min(storage)
                .max(0.0);
            storage -= from_soil;

            // Deep drainage.
            let drained = storage * (1.0 - (-DEEP_DRAINAGE_PER_DAY * dt_days).exp());
            storage -= drained;

            // Surface drainage downhill; a pit holds its water as a lake.
            let outflow = if downhill.is_some() {
                let rate = SURFACE_DRAINAGE_PER_DAY * (1.0 + 50.0 * slope);
                standing * (1.0 - (-rate * dt_days).exp())
            } else {
                0.0
            };
            standing -= outflow;

            soil_water.set(
                row,
                col,
                SoilWater {
                    moisture_fraction: (storage / FIELD_CAPACITY_MM).clamp(0.0, 1.0),
                    storage_mm: storage,
                },
            );
            surface_water.set(row, col, standing);
            routed_outflow_mm[row * grid_cols + col] = outflow;
            budget.precipitation_kg += precip * dt_days * cell_m2;
            budget.infiltration_kg += infiltrated * cell_m2;
            budget.soil_excess_kg += excess * cell_m2;
            budget.surface_evaporation_kg += from_surface * cell_m2;
            budget.soil_evaporation_kg += from_soil * cell_m2;
            budget.deep_drainage_kg += drained * cell_m2;
            if dt_days > 0.0 {
                infiltration.set(row, col, infiltrated / dt_days);
                evaporation.set(row, col, (from_surface + from_soil) / dt_days);
                // Surface runoff only: deep drainage recharges groundwater
                // rather than flowing across the land.
                runoff.set(row, col, outflow / dt_days);
                let generated = (outflow - *inflow_mm.get(row, col)).max(0.0);
                generated_runoff.set(row, col, generated / dt_days);
            }
        }
    }

    HydrologyState {
        soil_water,
        runoff,
        generated_runoff: Some(generated_runoff),
        infiltration,
        evaporation,
        surface_water,
        routed_outflow_mm,
        budget,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn weather(grid: &mk_core::grid::GridSpec, precip: f64) -> crate::weather::WeatherState {
        let mut state = crate::weather::WeatherState::new(grid);
        state.precipitation = Grid2::new(grid, precip);
        state
    }

    /// Total land water (kg): soil, standing water and routed outflow.
    fn water_kg(grid: &mk_core::grid::GridSpec, radius: f64, state: &HydrologyState) -> f64 {
        let mut total = 0.0;
        for row in 0..grid.nlat {
            let area = grid.cell_area_at_row_m2(row, radius).unwrap();
            for col in 0..grid.nlon {
                let routed = state
                    .routed_outflow_mm
                    .get(row * grid.nlon + col)
                    .copied()
                    .unwrap_or(0.0);
                total += (state.soil_water.get(row, col).storage_mm
                    + state.surface_water.get(row, col)
                    + routed)
                    * area;
            }
        }
        total
    }

    #[test]
    fn routed_runoff_is_conserved_when_the_step_length_changes() {
        // Runoff leaving a cell in a one-day step must arrive as the same
        // mass in the next step, however long that step is: it used to be
        // re-multiplied by the new step length, creating water.
        let canon = CanonLocked::default();
        let grid = mk_core::grid::GridSpec::new(4, 8);
        let climate = crate::climate::ClimateState::new(&grid, 280.0);
        // A slope falling eastward, all land.
        let slope = Grid2::from_data(
            &grid,
            (0..grid.nlat * grid.nlon)
                .map(|i| 2000.0 - (i % grid.nlon) as f64 * 200.0)
                .collect(),
        );
        let mut state = HydrologyState::new(&grid, 0.95);
        state.surface_water = Grid2::new(&grid, 80.0);
        let dry = weather(&grid, 0.0);
        for dt in [
            SECONDS_PER_DAY,
            30.0 * SECONDS_PER_DAY,
            0.25 * SECONDS_PER_DAY,
        ] {
            let before = water_kg(&grid, canon.planet_radius_m, &state);
            state = step_hydrology(&canon, &state, &dry, &climate, &slope, dt, &grid);
            let after = water_kg(&grid, canon.planet_radius_m, &state);
            let b = state.budget;
            let net_out = b.surface_evaporation_kg
                + b.soil_evaporation_kg
                + b.deep_drainage_kg
                + b.surface_to_ocean_kg
                - b.precipitation_kg
                - b.ocean_to_sea_soil_kg;
            assert!(
                ((before - after - net_out) / before).abs() < 1e-12,
                "dt {dt}: {before:e} -> {after:e}, budget out {net_out:e}"
            );
        }
    }

    #[test]
    fn rain_soaks_in_and_dry_weather_dries_the_soil() {
        let canon = CanonLocked::default();
        let grid = mk_core::grid::GridSpec::new(4, 4);
        let climate = crate::climate::ClimateState::new(&grid, 295.0);
        let land = Grid2::new(&grid, 100.0);
        let start = HydrologyState::new(&grid, 0.3);
        let week = 7.0 * SECONDS_PER_DAY;

        let wet = step_hydrology(
            &canon,
            &start,
            &weather(&grid, 10.0),
            &climate,
            &land,
            week,
            &grid,
        );
        let dry = step_hydrology(
            &canon,
            &start,
            &weather(&grid, 0.0),
            &climate,
            &land,
            week,
            &grid,
        );

        assert!(wet.soil_water.get(1, 1).moisture_fraction > 0.3);
        assert!(dry.soil_water.get(1, 1).moisture_fraction < 0.3);
    }

    #[test]
    fn water_runs_downhill_and_pools_in_a_pit() {
        let canon = CanonLocked::default();
        let grid = mk_core::grid::GridSpec::new(3, 3);
        let climate = crate::climate::ClimateState::new(&grid, 280.0);
        // A bowl: the centre cell is lowest.
        let bowl = Grid2::from_data(
            &grid,
            vec![
                300.0, 300.0, 300.0, 300.0, 100.0, 300.0, 300.0, 300.0, 300.0,
            ],
        );
        let mut start = HydrologyState::new(&grid, 1.0);
        start.surface_water = Grid2::new(&grid, 40.0);
        let day = SECONDS_PER_DAY;

        let after = step_hydrology(
            &canon,
            &start,
            &weather(&grid, 0.0),
            &climate,
            &bowl,
            day,
            &grid,
        );
        // Rims drain; the pit keeps its standing water.
        assert!(*after.runoff.get(0, 0) > 0.0);
        assert_eq!(*after.runoff.get(1, 1), 0.0);
        let next = step_hydrology(
            &canon,
            &after,
            &weather(&grid, 0.0),
            &climate,
            &bowl,
            day,
            &grid,
        );
        assert!(*next.surface_water.get(1, 1) > *next.surface_water.get(0, 0));
    }

    #[test]
    fn no_time_means_no_change() {
        let canon = CanonLocked::default();
        let grid = mk_core::grid::GridSpec::new(2, 2);
        let climate = crate::climate::ClimateState::new(&grid, 290.0);
        let land = Grid2::new(&grid, 100.0);
        let start = HydrologyState::new(&grid, 0.4);
        let same = step_hydrology(
            &canon,
            &start,
            &weather(&grid, 50.0),
            &climate,
            &land,
            0.0,
            &grid,
        );
        assert!((same.soil_water.get(0, 0).moisture_fraction - 0.4).abs() < 1e-12);
    }
}
