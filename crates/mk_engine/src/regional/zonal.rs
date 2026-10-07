//! The zonal background (Phase 2 Task 2): a 1-D latitude-band climate of
//! the whole planet that supplies every planet-wide quantity the island
//! cannot compute from its own cells (global mean temperature, absorbed
//! flux, CO₂, the energy-limited rainfall rate) and the zonal means its
//! ocean and atmosphere edges are forced from.
//!
//! It is upstream `climate::step_climate` and `weather::step_weather` run
//! unchanged on a `GridSpec::new(bands, BAND_COLUMNS)` grid, so the
//! upstream area weights (`cos(lat)`), row spacing and global means are
//! exact for it, and no equation is copied. Columns carry no longitude:
//! each band has as many land columns as the planet has land at that
//! latitude (counted once, at bootstrap, from upstream tectonics), and
//! the rest are ocean. Land matters: its small heat capacity follows the
//! seasons, and because radiative temperature goes as flux^¼, polar land
//! through the polar night averages several kelvin colder than ocean at
//! the same latitude, which an aquaplanet misses (measured: 4.6 K at
//! 73° S). `tests/regional_zonal_background.rs` checks the annual-mean
//! zonal temperatures against the upstream 32 × 64 world.

use mk_core::canon::{CanonDerived, CanonLocked};
use mk_core::grid::{Grid2, GridSpec};
use serde::{Deserialize, Serialize};

use crate::climate::{step_climate, ClimateForcing, ClimateState};
use crate::weather::{step_weather, WeatherState};

/// Latitude bands of the persisted background.
pub const DEFAULT_BAND_COUNT: usize = 64;
/// Planet-wide volcanic CO₂ outgassing (mol/yr): Earth's present-day
/// subaerial plus submarine ~0.26 Gt CO₂/yr (Gerlach 2011, Eos 92:201).
/// Upstream CO₂ responds only to this over its reference value, so a
/// constant planetary rate holds CO₂ at the canon's carbon-cycle balance.
pub const PLANETARY_VOLCANIC_CO2_MOL_YR: f64 = 6.0e12;
/// Mean surface heat flow (W/m²): 47 TW over Earth's 5.1×10¹⁴ m² (Davies
/// & Davies 2010, Solid Earth 1:5).
const MEAN_GEOTHERMAL_FLUX_W_M2: f64 = 0.092;
/// Columns per band: the land fraction is exact at upstream's default
/// longitude count (64 × 64 bands is 4,096 cells).
pub const BAND_COLUMNS: usize = 64;
/// Elevations given to background ocean and land columns (m): only the
/// sign matters upstream (ocean mixed layer or land heat capacity).
const OCEAN_COLUMN_ELEVATION_M: f64 = -4_000.0;
const LAND_COLUMN_ELEVATION_M: f64 = 500.0;
/// Longitudes the land fraction is counted over at bootstrap (upstream's
/// default world resolution).
const LAND_COUNT_LONGITUDES: usize = 64;

/// The planet as latitude bands: persisted, deterministic state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZonalBackgroundState {
    /// Upstream climate on the band grid (one column).
    pub climate: ClimateState,
    pub grid: GridSpec,
    /// Land columns per band (the first `n` of each row), south to north.
    pub band_land_columns: Vec<usize>,
    /// Area-weighted planetary means.
    pub global_mean_surface_temperature_k: f64,
    pub mean_absorbed_flux_w_m2: f64,
    pub global_mean_precipitation_mm_day: f64,
    /// Zonal means per band, south to north.
    pub band_precipitation_mm_day: Vec<f64>,
    pub band_wind_u_m_s: Vec<f64>,
    pub band_wind_v_m_s: Vec<f64>,
    pub atmospheric_co2_ppm: f64,
    pub planetary_volcanic_co2_mol_yr: f64,
    pub sim_time_seconds: f64,
}

/// Elevation and geothermal flux of the band grid: the first
/// `land[row]` columns of each band are land.
fn surface(grid: &GridSpec, land: &[usize]) -> (Grid2<f64>, Grid2<f64>) {
    let elevation = (0..grid.nlat)
        .flat_map(|r| {
            let n = land.get(r).copied().unwrap_or(0);
            (0..grid.nlon).map(move |c| {
                if c < n {
                    LAND_COLUMN_ELEVATION_M
                } else {
                    OCEAN_COLUMN_ELEVATION_M
                }
            })
        })
        .collect();
    (
        Grid2::from_data(grid, elevation),
        Grid2::new(grid, MEAN_GEOTHERMAL_FLUX_W_M2),
    )
}

/// Land columns per band: upstream tectonics' initial land fraction at
/// each band's latitude, on a transient `bands × 64` planetary grid that
/// is dropped once counted.
fn land_columns(canon: &CanonLocked, bands: usize) -> Vec<usize> {
    let count_grid = GridSpec::new(bands, LAND_COUNT_LONGITUDES);
    let elevation = crate::tectonics::step_tectonics(canon, 0, &count_grid).get_elevation_grid();
    (0..bands)
        .map(|r| {
            let land = (0..LAND_COUNT_LONGITUDES)
                .filter(|&c| *elevation.get(r, c) > 0.0)
                .count();
            (land as f64 / LAND_COUNT_LONGITUDES as f64 * BAND_COLUMNS as f64).round() as usize
        })
        .collect()
}

fn forcing<'a>(
    canon: &CanonLocked,
    t: f64,
    dt_seconds: f64,
    elevation: &'a Grid2<f64>,
    geothermal: &'a Grid2<f64>,
) -> ClimateForcing<'a> {
    let orbit = crate::orbit::step_orbit(canon, t);
    let rotation = crate::rotation::step_rotation(t, orbit.mean_anomaly, canon);
    ClimateForcing {
        toa_solar_flux_w_m2: canon.solar_constant_w_m2 / (orbit.r_over_a * orbit.r_over_a),
        solar_declination_rad: rotation.subsolar_latitude,
        albedo: canon.albedo_baseline,
        surface_pressure_pa: canon.sea_level_pressure_pa,
        surface_gravity_m_s2: canon.surface_gravity_m_s2,
        geothermal_flux_w_m2: geothermal,
        volcanic_co2_mol_yr: PLANETARY_VOLCANIC_CO2_MOL_YR,
        elevation_m: elevation,
        dt_seconds,
    }
}

fn coriolis(canon: &CanonLocked, grid: &GridSpec) -> Grid2<f64> {
    crate::planet::step_planet(canon, &CanonDerived::from(canon), grid).coriolis
}

/// Linear interpolation of per-band `values` at `latitude_rad`, held
/// constant beyond the outermost band centres.
fn band_value_at(grid: &GridSpec, values: &[f64], latitude_rad: f64) -> f64 {
    let n = values.len().min(grid.nlat);
    if n == 0 {
        return 0.0;
    }
    // Band centres sit at ((i + 0.5) / n − 0.5)·π.
    let x = (latitude_rad / std::f64::consts::PI + 0.5) * n as f64 - 0.5;
    if !x.is_finite() || x <= 0.0 {
        return values[0];
    }
    if x >= (n - 1) as f64 {
        return values[n - 1];
    }
    let i = x.floor() as usize;
    let f = x - i as f64;
    values[i] * (1.0 - f) + values[i + 1] * f
}

impl ZonalBackgroundState {
    /// The background settled to equilibrium at `t = 0` and spun through
    /// two orbits (upstream `spin_up_climate`), so it starts on its
    /// seasonal cycle at the same orbital phase as the planetary world.
    pub fn bootstrap(canon: &CanonLocked, band_count: usize) -> Self {
        let bands = band_count.max(2);
        let grid = GridSpec::new(bands, BAND_COLUMNS);
        let band_land_columns = land_columns(canon, bands);
        let (elevation, geothermal) = surface(&grid, &band_land_columns);
        let f = coriolis(canon, &grid);
        let settled = step_climate(
            &ClimateState::default_for_grid(&grid),
            &forcing(canon, 0.0, f64::INFINITY, &elevation, &geothermal),
            &grid,
        );
        let (climate, _, _) = crate::world_integration::spin_up_climate(
            canon,
            settled,
            &geothermal,
            PLANETARY_VOLCANIC_CO2_MOL_YR,
            &elevation,
            &f,
            &grid,
        );
        let weather = step_weather(canon, 0, &climate, &f, &elevation, &grid);
        let mut state = Self {
            climate,
            grid,
            band_land_columns,
            global_mean_surface_temperature_k: 0.0,
            mean_absorbed_flux_w_m2: 0.0,
            global_mean_precipitation_mm_day: 0.0,
            band_precipitation_mm_day: Vec::new(),
            band_wind_u_m_s: Vec::new(),
            band_wind_v_m_s: Vec::new(),
            atmospheric_co2_ppm: 0.0,
            planetary_volcanic_co2_mol_yr: PLANETARY_VOLCANIC_CO2_MOL_YR,
            sim_time_seconds: 0.0,
        };
        state.derive(&weather);
        state
    }

    /// Advance to `sim_time_seconds` over `dt_seconds`. A zero, negative
    /// or non-finite step changes nothing.
    pub fn step(&mut self, canon: &CanonLocked, sim_time_seconds: f64, dt_seconds: f64) {
        if !(dt_seconds.is_finite() && dt_seconds > 0.0 && sim_time_seconds.is_finite()) {
            return;
        }
        let (elevation, geothermal) = surface(&self.grid, &self.band_land_columns);
        self.climate = step_climate(
            &self.climate,
            &forcing(canon, sim_time_seconds, dt_seconds, &elevation, &geothermal),
            &self.grid,
        );
        let weather = step_weather(
            canon,
            0,
            &self.climate,
            &coriolis(canon, &self.grid),
            &elevation,
            &self.grid,
        );
        self.derive(&weather);
        self.sim_time_seconds = sim_time_seconds;
    }

    /// Refresh the planetary means and band fields from the climate and
    /// the weather on it.
    fn derive(&mut self, weather: &WeatherState) {
        let (n, cols) = (self.grid.nlat, self.grid.nlon);
        let weights: Vec<f64> = (0..n)
            .map(|r| self.grid.lat_rad(r).cos().max(0.0))
            .collect();
        let total: f64 = weights.iter().sum();
        let zonal = |f: &dyn Fn(usize, usize) -> f64| -> Vec<f64> {
            (0..n)
                .map(|r| (0..cols).map(|c| f(r, c)).sum::<f64>() / cols.max(1) as f64)
                .collect()
        };
        let precipitation = zonal(&|r, c| weather.precipitation.get(r, c).max(0.0));
        self.global_mean_surface_temperature_k = self.climate.global_temperature();
        self.mean_absorbed_flux_w_m2 = self.climate.mean_absorbed_flux_w_m2;
        self.global_mean_precipitation_mm_day = if total > 0.0 {
            precipitation
                .iter()
                .zip(&weights)
                .map(|(p, w)| p * w)
                .sum::<f64>()
                / total
        } else {
            0.0
        };
        self.band_precipitation_mm_day = precipitation;
        self.band_wind_u_m_s = zonal(&|r, c| weather.wind.get(r, c).u_east);
        self.band_wind_v_m_s = zonal(&|r, c| weather.wind.get(r, c).v_north);
        self.atmospheric_co2_ppm = self.climate.co2_concentration;
    }

    /// Mean surface temperature of each band (land and sea), south to
    /// north (K).
    pub fn band_surface_temperature_k(&self) -> Vec<f64> {
        let cols = self.grid.nlon.max(1);
        (0..self.grid.nlat)
            .map(|r| {
                (0..cols)
                    .map(|c| *self.climate.surface_temperature.get(r, c))
                    .sum::<f64>()
                    / cols as f64
            })
            .collect()
    }

    /// Mean sea-surface temperature of each band's ocean columns (the
    /// whole band where it is all land), south to north (K).
    pub fn band_sea_surface_temperature_k(&self) -> Vec<f64> {
        let cols = self.grid.nlon;
        (0..self.grid.nlat)
            .map(|r| {
                let first = self
                    .band_land_columns
                    .get(r)
                    .copied()
                    .unwrap_or(0)
                    .min(cols);
                let first = if first >= cols { 0 } else { first };
                (first..cols)
                    .map(|c| *self.climate.surface_temperature.get(r, c))
                    .sum::<f64>()
                    / (cols - first).max(1) as f64
            })
            .collect()
    }

    /// Zonal-mean surface temperature (land and sea) at a latitude (K).
    pub fn surface_temperature_at(&self, latitude_rad: f64) -> f64 {
        band_value_at(&self.grid, &self.band_surface_temperature_k(), latitude_rad)
    }

    /// Zonal-mean sea-surface temperature at a latitude (K).
    pub fn sea_surface_temperature_at(&self, latitude_rad: f64) -> f64 {
        band_value_at(
            &self.grid,
            &self.band_sea_surface_temperature_k(),
            latitude_rad,
        )
    }

    /// Zonal-mean precipitation at a latitude (mm/day).
    pub fn precipitation_at(&self, latitude_rad: f64) -> f64 {
        band_value_at(&self.grid, &self.band_precipitation_mm_day, latitude_rad)
    }

    /// Zonal-mean surface wind (east, north) at a latitude (m/s).
    pub fn wind_at(&self, latitude_rad: f64) -> (f64, f64) {
        (
            band_value_at(&self.grid, &self.band_wind_u_m_s, latitude_rad),
            band_value_at(&self.grid, &self.band_wind_v_m_s, latitude_rad),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn band_interpolation_hits_centres_and_clamps() {
        let grid = GridSpec::new(4, 1);
        let v = [1.0, 2.0, 3.0, 4.0];
        for (r, want) in v.iter().enumerate() {
            assert!((band_value_at(&grid, &v, grid.lat_rad(r)) - want).abs() < 1e-12);
        }
        assert_eq!(band_value_at(&grid, &v, -1.6), 1.0);
        assert_eq!(band_value_at(&grid, &v, 1.6), 4.0);
        assert!((band_value_at(&grid, &v, 0.0) - 2.5).abs() < 1e-12);
    }
}
