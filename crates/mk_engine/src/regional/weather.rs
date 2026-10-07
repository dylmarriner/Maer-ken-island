//! Regional weather (Phase 2 Task 3) on the coarse grid.
//!
//! Upstream's wind, humidity and rain-out equations with the domain's
//! latitudes, flat row spacing and Coriolis. Upstream rescales rain so the
//! *planet's* area mean equals the energy-limited global evaporation; a
//! window cannot compute that, so each row's mean rain is normalised to
//! the zonal background's rate at the row's latitude, and the spatial
//! pattern (land against sea, humidity) comes from the local equations.
//! The outermost cells relax toward the edge wind. Orographic rain is
//! added by [`apply_orographic_precipitation`].

use mk_core::canon::CanonLocked;
use mk_core::grid::Grid2;
use mk_island::{AtmosphereBoundaryForcing, DomainLevel, Edge, IslandDomain};

use super::climate::{coarse_latitudes, regional_coriolis};
use super::edge::relax_to_edges;
use super::zonal::ZonalBackgroundState;
use crate::climate::ClimateState;
use crate::weather::{
    meridional_wind_from_gradient, rainout_weight, relative_humidity, saturation_specific_humidity,
    zonal_wind, WeatherState, WindVector, VAPOUR_COLUMN_FRACTION,
};

/// Fraction of the lifted vapour that falls on the slope (a lifted parcel
/// does not rain out completely): tuned so a 2 km barrier in mid-latitude
/// westerlies reaches the reference windward/leeward ratio of 3-8
/// (`fixtures/reference/climate`, `windward_to_leeward_precipitation_ratio`).
const OROGRAPHIC_EFFICIENCY: f64 = 0.15;
/// Scale height (m) of water vapour: it is concentrated in the lowest
/// ~2 km, so lifting a column's vapour through that depth condenses it.
const VAPOUR_SCALE_HEIGHT_M: f64 = 2_000.0;
/// Rain-shadow strength: descending air (negative lift, m/s) cuts rain by
/// `1 + gain × lift`, floored at [`SHADOW_FLOOR`].
const SHADOW_GAIN_S_PER_M: f64 = 3.5;
const SHADOW_FLOOR: f64 = 0.2;
const SECONDS_PER_DAY: f64 = 86_400.0;

fn edge_value(
    atmosphere: &AtmosphereBoundaryForcing,
    edge: Edge,
    index: usize,
    values: impl Fn(&mk_island::EdgeAtmosphereForcing) -> &[f64],
) -> Option<f64> {
    atmosphere
        .edges
        .iter()
        .find(|e| e.edge == edge)
        .and_then(|e| values(e).get(index).copied())
        .filter(|v| v.is_finite())
        // Surface winds beyond a hurricane's are outside the model.
        .map(|v| v.clamp(-MAX_EDGE_WIND_M_S, MAX_EDGE_WIND_M_S))
}

/// Largest surface wind component (m/s) an edge may force.
const MAX_EDGE_WIND_M_S: f64 = 60.0;

/// Weather for the current coarse climate: wind, precipitable water and
/// precipitation. `elevation_m` is the coarse elevation (sea level 0).
pub fn step_regional_weather(
    canon: &CanonLocked,
    climate: &ClimateState,
    elevation_m: &Grid2<f64>,
    domain: &IslandDomain,
    background: &ZonalBackgroundState,
    atmosphere: &AtmosphereBoundaryForcing,
) -> WeatherState {
    let coarse = DomainLevel::Coarse;
    let spec = domain.storage_spec(coarse);
    let (rows, cols) = (spec.nlat, spec.nlon);
    let latitudes = coarse_latitudes(domain);
    let coriolis = regional_coriolis(domain, coarse, canon);
    let pressure_hpa = canon.sea_level_pressure_pa / 100.0;
    let air_column_kg_m2 = canon.sea_level_pressure_pa / canon.surface_gravity_m_s2.max(1e-9);
    // Row spacing as an angle at the planet's surface, so the upstream
    // meridional-gradient scaling (K per radian) keeps its units.
    let row_spacing_rad = domain.cell_size_m(coarse) / canon.planet_radius_m.max(1.0);

    let (mut u, mut v) = (vec![0.0; rows * cols], vec![0.0; rows * cols]);
    let mut moisture = vec![0.0; rows * cols];
    let mut weight = vec![0.0; rows * cols];
    for (row, &lat) in latitudes.iter().enumerate().take(rows) {
        let south = row.saturating_sub(1);
        let north = (row + 1).min(rows - 1);
        let span = (north - south).max(1) as f64 * row_spacing_rad;
        for col in 0..cols {
            let idx = row * cols + col;
            let temp = *climate.surface_temperature.get(row, col);
            u[idx] = zonal_wind(lat, *coriolis.get(row, col));
            let d_t_dlat = (climate.surface_temperature.get(north, col)
                - climate.surface_temperature.get(south, col))
                / span;
            v[idx] = meridional_wind_from_gradient(d_t_dlat);

            let is_ocean = *elevation_m.get(row, col) <= 0.0;
            let rh = relative_humidity(lat, is_ocean);
            let q = saturation_specific_humidity(temp, pressure_hpa) * rh;
            let precipitable_water = q * air_column_kg_m2 * VAPOUR_COLUMN_FRACTION;
            moisture[idx] = precipitable_water;
            weight[idx] = rainout_weight(precipitable_water, rh);
        }
    }

    relax_to_edges(&mut u, rows, cols, |edge, i| {
        edge_value(atmosphere, edge, i, |e| &e.wind_u_m_s)
    });
    relax_to_edges(&mut v, rows, cols, |edge, i| {
        edge_value(atmosphere, edge, i, |e| &e.wind_v_m_s)
    });

    // Each row's mean rain is the background's rate at that latitude.
    let mut precipitation = weight;
    for row in 0..rows {
        let slice = &mut precipitation[row * cols..(row + 1) * cols];
        let mean_weight = slice.iter().sum::<f64>() / cols as f64;
        let target = background.precipitation_at(latitudes[row]);
        let scale = if mean_weight > 0.0 {
            target / mean_weight
        } else {
            0.0
        };
        for p in slice {
            *p *= scale;
        }
    }

    let wind = Grid2::from_data(
        &spec,
        u.into_iter()
            .zip(v)
            .map(|(u_east, v_north)| WindVector { u_east, v_north })
            .collect(),
    );
    WeatherState {
        wind,
        moisture: Grid2::from_data(&spec, moisture),
        precipitation: Grid2::from_data(&spec, precipitation),
    }
}

/// Orographic redistribution of rain over `elevation_m` (coarse). Air
/// forced up a slope (`lift = wind · ∇h`, m/s) condenses a fraction of its
/// vapour: windward slopes gain `efficiency × lift × moisture / H` and air
/// descending on the lee loses rain down to [`SHADOW_FLOOR`] of its
/// value. Domain-total rain is then rescaled to what it was, so the
/// background normalisation still holds (the ridge moves rain; it does
/// not make it).
pub fn apply_orographic_precipitation(
    weather: &mut WeatherState,
    elevation_m: &Grid2<f64>,
    domain: &IslandDomain,
) {
    let (rows, cols) = (elevation_m.nlat(), elevation_m.nlon());
    if rows < 3 || cols < 3 {
        return;
    }
    let size = domain.cell_size_m(DomainLevel::Coarse);
    let before: f64 = weather.precipitation.data().iter().sum();
    if before <= 0.0 {
        return;
    }
    let h = |r: usize, c: usize| elevation_m.get(r, c).max(0.0);
    let mut out = weather.precipitation.data().to_vec();
    for r in 0..rows {
        let (rs, rn) = (r.saturating_sub(1), (r + 1).min(rows - 1));
        for c in 0..cols {
            let (cw, ce) = (c.saturating_sub(1), (c + 1).min(cols - 1));
            let dh_dx = (h(r, ce) - h(r, cw)) / ((ce - cw).max(1) as f64 * size);
            let dh_dy = (h(rn, c) - h(rs, c)) / ((rn - rs).max(1) as f64 * size);
            let wind = weather.wind.get(r, c);
            let lift = wind.u_east * dh_dx + wind.v_north * dh_dy;
            let idx = r * cols + c;
            if lift > 0.0 {
                let moisture = *weather.moisture.get(r, c);
                out[idx] += OROGRAPHIC_EFFICIENCY * lift * moisture / VAPOUR_SCALE_HEIGHT_M
                    * SECONDS_PER_DAY;
            } else {
                out[idx] *= (1.0 + SHADOW_GAIN_S_PER_M * lift).max(SHADOW_FLOOR);
            }
        }
    }
    let after: f64 = out.iter().sum();
    let scale = if after > 0.0 { before / after } else { 1.0 };
    for (p, o) in weather.precipitation.data_mut().iter_mut().zip(out) {
        *p = o * scale;
    }
}
