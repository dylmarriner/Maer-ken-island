//! WEATHER MODULE — PHASE 2
//!
//! Purpose
//! - Precipitation and wind patterns for Marr'Kena MK-I
//! - Moisture convergence zones, trade winds, jet streams
//! - Deterministic weather state based on temperature and rotation
//!
//! Invariants
//! - All arithmetic uses f64 for continuous quantities
//! - Wind field computed from Coriolis + local temperature gradients
//! - Moisture is precipitable water (kg/m²): saturation specific humidity at
//!   the canon surface pressure × relative humidity × the column mass of the
//!   water-vapour layer
//! - Relative humidity follows surface type (ocean vs land) and the Hadley
//!   circulation (moist rising ITCZ, dry descending subtropics)
//! - Precipitation is energy-limited: its global mean equals evaporation,
//!   set by the latent-heat share of absorbed radiation; it is distributed
//!   across cells in proportion to precipitable water × a relative-humidity
//!   rain-out efficiency
//! - Same tick + climate state → same weather field (deterministic)
//! - Ledger-tracked: AtmosEnergy → Precipitation
//!
//! Authority
//! - MAERKEN_PHASE_2_PHYSICAL_WORLD.md § 3.6
//! - PLANET_CONSTANTS.md § 7 (weather parameters)
//! - NATURE_CONSTANTS.md (thermodynamic properties)

use mk_core::canon::CanonLocked;
use mk_core::grid::Grid2;
use mk_core::time::Tick;
use serde::{Deserialize, Serialize};

const R_AIR: f64 = 287.05;
const R_V: f64 = 461.5;
const EPSILON: f64 = R_AIR / R_V;
const L_V: f64 = 2.45e6;
const T0_K: f64 = 273.15;
const E0_HPA: f64 = 6.112;

/// Wind vector (zonal and meridional components)
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct WindVector {
    /// Zonal (east-west) component (m/s, positive = eastward)
    pub u_east: f64,
    /// Meridional (north-south) component (m/s, positive = northward)
    pub v_north: f64,
}

impl WindVector {
    /// Get wind speed (magnitude)
    pub fn speed(&self) -> f64 {
        (self.u_east * self.u_east + self.v_north * self.v_north).sqrt()
    }

    /// Get wind direction (radians, 0 = north, π/2 = east)
    pub fn direction(&self) -> f64 {
        self.v_north.atan2(self.u_east)
    }
}

/// Weather state for a given tick
///
/// Type: Computed struct (no persistent state)
/// Determinism: same tick + climate → same weather field
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherState {
    /// Wind field (m/s) per grid cell
    pub wind: Grid2<WindVector>,
    /// Atmospheric moisture (kg/m²) per grid cell
    pub moisture: Grid2<f64>,
    /// Precipitation rate (mm/day) per grid cell
    pub precipitation: Grid2<f64>,
}

impl WeatherState {
    /// Minimal constructor to create an empty WeatherState for a given grid spec
    pub fn new(grid_spec: &mk_core::grid::GridSpec) -> Self {
        WeatherState {
            wind: Grid2::new(
                grid_spec,
                WindVector {
                    u_east: 0.0,
                    v_north: 0.0,
                },
            ),
            moisture: Grid2::new(grid_spec, 0.0),
            precipitation: Grid2::new(grid_spec, 0.0),
        }
    }
}

/// Compute zonal (trade) wind at latitude
///
/// Simple hadley cell model: equatorward flow in tropics, poleward in subtropics
/// Returns wind speed (positive = eastward)
fn zonal_wind(latitude: f64, coriolis_param: f64) -> f64 {
    let lat_deg = latitude * 180.0 / std::f64::consts::PI;

    // Trade winds equatorward of ~30°
    // Westerlies poleward of ~30°
    let tropical = if lat_deg.abs() < 30.0 {
        -5.0 * (lat_deg.abs() / 30.0) // 0 m/s at equator, -5 m/s at 30°
    } else {
        10.0 * ((lat_deg.abs() - 30.0) / 30.0) // +10 m/s at poles
    };

    // Coriolis enhancement
    let coriolis_factor = 1.0 + coriolis_param.abs();
    tropical * coriolis_factor
}

/// Earth's mean pole-to-equator surface temperature gradient (K per radian
/// of latitude), the reference against which meridional flow is scaled.
const REFERENCE_MERIDIONAL_GRADIENT_K_PER_RAD: f64 = 40.0;
/// Near-surface meridional wind (m/s) driven by the reference gradient.
const REFERENCE_MERIDIONAL_WIND_M_S: f64 = 3.0;
/// Water vapour scale height divided by the air's (~2 km / ~8 km): the
/// share of the air column's mass over which vapour is distributed.
const VAPOUR_COLUMN_FRACTION: f64 = 0.25;
/// Share of absorbed radiation the surface spends evaporating water (Earth:
/// ~80 of ~240 W/m²), which sets global-mean evaporation = precipitation.
const LATENT_HEAT_SHARE: f64 = 0.33;
/// Seconds per day, for converting kg m⁻² s⁻¹ to mm/day.
const SECONDS_PER_DAY: f64 = 86_400.0;

/// Near-surface meridional wind (m/s, positive = northward) from the local
/// northward temperature gradient: thermally direct surface flow runs from
/// the cold (high-pressure) side toward the warm side — equatorward trade
/// winds in both hemispheres — scaled against Earth's reference gradient.
fn meridional_wind_from_gradient(temp_gradient_k_per_rad: f64) -> f64 {
    (REFERENCE_MERIDIONAL_WIND_M_S * temp_gradient_k_per_rad
        / REFERENCE_MERIDIONAL_GRADIENT_K_PER_RAD)
        .clamp(-10.0, 10.0)
}

/// Saturation specific humidity (kg/kg) at `temperature_k` and surface
/// pressure `pressure_hpa`, from the Clausius-Clapeyron envelope.
pub(crate) fn saturation_specific_humidity(temperature_k: f64, pressure_hpa: f64) -> f64 {
    let e_s_hpa = E0_HPA * ((L_V / R_V) * ((1.0 / T0_K) - (1.0 / temperature_k))).exp();
    let e_s_hpa = e_s_hpa.min(pressure_hpa * 0.99);
    ((EPSILON * e_s_hpa) / (pressure_hpa - (1.0 - EPSILON) * e_s_hpa)).max(0.0)
}

/// Near-surface relative humidity (0..1): moist over open water, drier
/// over land, raised in the rising ITCZ and lowered in the descending
/// subtropical branch of the Hadley cell.
pub(crate) fn relative_humidity(latitude_rad: f64, is_ocean: bool) -> f64 {
    let lat_deg = latitude_rad.to_degrees().abs();
    let surface = if is_ocean { 0.80 } else { 0.62 };
    let itcz = 0.10 * (-(lat_deg / 10.0).powi(2)).exp();
    let subtropical_descent = 0.22 * (-((lat_deg - 27.0) / 8.0).powi(2)).exp();
    (surface + itcz - subtropical_descent).clamp(0.2, 0.98)
}

/// Relative weight of rain falling from a column holding
/// `precipitable_water_kg_m2` at relative humidity `rh`: rain forms more
/// readily the closer the air is to saturation, and not at all in dry,
/// subsiding air.
fn rainout_weight(precipitable_water_kg_m2: f64, rh: f64) -> f64 {
    let efficiency = ((rh - 0.3) / 0.7).clamp(0.0, 1.0);
    precipitable_water_kg_m2.max(0.0) * efficiency
}

/// Global-mean precipitation (mm/day) = evaporation, from the mean
/// absorbed radiative flux (W/m²).
fn global_mean_precipitation(mean_absorbed_w_m2: f64) -> f64 {
    (LATENT_HEAT_SHARE * mean_absorbed_w_m2.max(0.0) / L_V) * SECONDS_PER_DAY
}

/// Step weather for one tick
pub fn step_weather(
    canon: &CanonLocked,
    _tick: Tick,
    climate_state: &super::climate::ClimateState,
    coriolis_field: &Grid2<f64>,
    elevation_m: &Grid2<f64>,
    grid_spec: &mk_core::grid::GridSpec,
) -> WeatherState {
    let grid_rows = grid_spec.nlat;
    let grid_cols = grid_spec.nlon;

    let mut wind = Grid2::new(
        grid_spec,
        WindVector {
            u_east: 0.0,
            v_north: 0.0,
        },
    );
    let mut moisture = Grid2::new(grid_spec, 0.0);
    let mut precipitation = Grid2::new(grid_spec, 0.0);

    let pressure_hpa = canon.sea_level_pressure_pa / 100.0;
    // Mass of the air column above each square metre (kg/m²).
    let air_column_kg_m2 = canon.sea_level_pressure_pa / canon.surface_gravity_m_s2.max(1e-9);
    let row_spacing_rad = std::f64::consts::PI / grid_rows.max(1) as f64;
    let (mut weighted_rainout, mut total_area) = (0.0, 0.0);

    // Compute per-cell weather
    for row in 0..grid_rows {
        for col in 0..grid_cols {
            let lat = grid_spec.lat_rad(row);
            let temp = *climate_state.surface_temperature.get(row, col);
            let coriolis = *coriolis_field.get(row, col);

            // Wind components
            let u = zonal_wind(lat, coriolis);

            // Local meridional temperature gradient (K per radian, toward
            // the north), by central difference between neighbouring rows.
            let south = row.saturating_sub(1);
            let north = (row + 1).min(grid_rows - 1);
            let span = (north - south).max(1) as f64 * row_spacing_rad;
            let d_t_dlat = (climate_state.surface_temperature.get(north, col)
                - climate_state.surface_temperature.get(south, col))
                / span;
            let v = meridional_wind_from_gradient(d_t_dlat);

            wind.set(
                row,
                col,
                WindVector {
                    u_east: u,
                    v_north: v,
                },
            );

            let is_ocean = elevation_m.get_safe(row, col).is_none_or(|h| *h <= 0.0);
            let rh = relative_humidity(lat, is_ocean);
            let q = saturation_specific_humidity(temp, pressure_hpa) * rh;
            let precipitable_water = q * air_column_kg_m2 * VAPOUR_COLUMN_FRACTION;
            moisture.set(row, col, precipitable_water);
            let weight = rainout_weight(precipitable_water, rh);
            precipitation.set(row, col, weight);
            let area = lat.cos().max(0.0);
            weighted_rainout += weight * area;
            total_area += area;
        }
    }

    // Scale the rain-out weights so the area-mean precipitation equals the
    // energy-limited global evaporation rate.
    // Area-weighted planetary mean; a state saved before that field existed
    // falls back to the per-cell average of the summed flux.
    let mean_absorbed = if climate_state.mean_absorbed_flux_w_m2 > 0.0 {
        climate_state.mean_absorbed_flux_w_m2
    } else {
        climate_state.absorbed_flux_w_m2 / (grid_rows * grid_cols).max(1) as f64
    };
    let mean_precipitation = global_mean_precipitation(mean_absorbed);
    let mean_weight = if total_area > 0.0 {
        weighted_rainout / total_area
    } else {
        0.0
    };
    let scale = if mean_weight > 0.0 {
        mean_precipitation / mean_weight
    } else {
        0.0
    };
    for (_, _, value) in precipitation.indexed_iter_mut() {
        *value *= scale;
    }

    WeatherState {
        wind,
        moisture,
        precipitation,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weather_deterministic() {
        let canon = CanonLocked::default();
        let grid_spec = mk_core::grid::GridSpec::new(4, 8);
        let climate = {
            let geothermal = mk_core::grid::Grid2::new(&grid_spec, 0.09);
            let elevation = mk_core::grid::Grid2::new(&grid_spec, -1000.0);
            super::super::climate::step_climate(
                &super::super::climate::ClimateState::default_for_grid(&grid_spec),
                &super::super::climate::ClimateForcing {
                    toa_solar_flux_w_m2: canon.solar_constant_w_m2,
                    solar_declination_rad: 0.0,
                    albedo: canon.albedo_baseline,
                    surface_pressure_pa: canon.sea_level_pressure_pa,
                    surface_gravity_m_s2: canon.surface_gravity_m_s2,
                    geothermal_flux_w_m2: &geothermal,
                    volcanic_co2_mol_yr: 1.0e12,
                    elevation_m: &elevation,
                    dt_seconds: f64::INFINITY,
                },
                &grid_spec,
            )
        };
        let coriolis = mk_core::grid::Grid2::new(&grid_spec, 0.0);

        let ocean = mk_core::grid::Grid2::new(&grid_spec, -1000.0);
        let state1 = step_weather(&canon, 0, &climate, &coriolis, &ocean, &grid_spec);
        let state2 = step_weather(&canon, 0, &climate, &coriolis, &ocean, &grid_spec);

        // Check wind fields are identical
        for row in 0..grid_spec.nlat {
            for col in 0..grid_spec.nlon {
                let w1 = state1.wind.get(row, col);
                let w2 = state2.wind.get(row, col);
                assert_eq!(w1.u_east, w2.u_east);
                assert_eq!(w1.v_north, w2.v_north);
            }
        }
    }

    #[test]
    fn wind_vector_speed() {
        let w = WindVector {
            u_east: 3.0,
            v_north: 4.0,
        };
        assert_eq!(w.speed(), 5.0); // 3-4-5 triangle
    }

    #[test]
    fn saturation_increases_with_temp_and_falls_with_pressure() {
        let cold = saturation_specific_humidity(273.15, 1013.25);
        let warm = saturation_specific_humidity(293.15, 1013.25);
        assert!(warm > cold);
        assert!(saturation_specific_humidity(293.15, 1820.0) < warm);
        // ~14.7 g/kg at 20 °C and one atmosphere.
        assert!((warm - 0.0147).abs() < 0.001, "q_s(20 °C) = {warm}");
    }

    #[test]
    fn subtropics_are_drier_than_the_itcz_and_ocean_moister_than_land() {
        let equator = relative_humidity(0.0, true);
        let subtropics = relative_humidity(27.0_f64.to_radians(), true);
        assert!(equator > subtropics);
        assert!(relative_humidity(0.0, true) > relative_humidity(0.0, false));
    }

    #[test]
    fn earth_like_energy_budget_gives_earth_like_rainfall() {
        // ~240 W/m² absorbed → ~2.8 mm/day global mean, as on Earth.
        let mean = global_mean_precipitation(240.0);
        assert!((2.5..3.2).contains(&mean), "global mean {mean} mm/day");
        assert_eq!(rainout_weight(20.0, 0.25), 0.0);
        assert!(rainout_weight(50.0, 0.9) > rainout_weight(50.0, 0.6));
    }

    #[test]
    fn precipitation_field_matches_the_energy_budget() {
        let canon = CanonLocked::default();
        let grid_spec = mk_core::grid::GridSpec::new(18, 36);
        let geothermal = mk_core::grid::Grid2::new(&grid_spec, 0.09);
        let ocean = mk_core::grid::Grid2::new(&grid_spec, -1000.0);
        let climate = super::super::climate::step_climate(
            &super::super::climate::ClimateState::default_for_grid(&grid_spec),
            &super::super::climate::ClimateForcing {
                toa_solar_flux_w_m2: canon.solar_constant_w_m2,
                solar_declination_rad: 0.0,
                albedo: canon.albedo_baseline,
                surface_pressure_pa: canon.sea_level_pressure_pa,
                surface_gravity_m_s2: canon.surface_gravity_m_s2,
                geothermal_flux_w_m2: &geothermal,
                volcanic_co2_mol_yr: 1.0e12,
                elevation_m: &ocean,
                dt_seconds: f64::INFINITY,
            },
            &grid_spec,
        );
        let coriolis = mk_core::grid::Grid2::new(&grid_spec, 0.0);
        let weather = step_weather(&canon, 0, &climate, &coriolis, &ocean, &grid_spec);

        let equator = *weather.precipitation.get(9, 0);
        let subtropics = *weather.precipitation.get(12, 0);
        assert!(
            equator > subtropics,
            "ITCZ {equator} vs subtropics {subtropics}"
        );
        assert!(weather.precipitation.data().iter().all(|p| *p < 30.0));

        // The area-weighted global mean is the energy-limited evaporation
        // of canon's ~240 W/m² absorbed flux: Earth-like, ~2.7 mm/day.
        let (mut sum, mut area) = (0.0, 0.0);
        for row in 0..grid_spec.nlat {
            let w = grid_spec.lat_rad(row).cos();
            for col in 0..grid_spec.nlon {
                sum += weather.precipitation.get(row, col) * w;
                area += w;
            }
        }
        let mean = sum / area;
        assert!((2.4..3.1).contains(&mean), "global mean {mean} mm/day");
    }

    #[test]
    fn meridional_wind_blows_from_cold_toward_warm() {
        // Warmer to the north: surface air flows north.
        let v = meridional_wind_from_gradient(REFERENCE_MERIDIONAL_GRADIENT_K_PER_RAD);
        assert!((v - REFERENCE_MERIDIONAL_WIND_M_S).abs() < 1e-12);
        // Warmer to the south: it flows south.
        assert!(meridional_wind_from_gradient(-10.0) < 0.0);
        assert!(meridional_wind_from_gradient(1.0e6).abs() <= 10.0);
    }
}
