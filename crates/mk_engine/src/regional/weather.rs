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
use super::maritime::{adjustment_length_m, carry_from_sea};
use super::zonal::ZonalBackgroundState;
use crate::climate::ClimateState;
use crate::weather::{
    meridional_wind_from_gradient, rainout_weight, relative_humidity, saturation_specific_humidity,
    zonal_wind, WeatherState, WindVector, VAPOUR_COLUMN_FRACTION,
};

/// Fraction of the lifted vapour that falls on the slope (a lifted parcel
/// does not rain out completely), calibrated so the owner's island, a
/// range over 2 km high in mid-latitude westerlies, reaches the reference
/// windward/leeward ratio's typical value of 5 (Southern Alps; the pack's
/// band is 3-8, `fixtures/reference/climate`): measured over a year, 0.15
/// gives 4.4, 0.2 gives 5.2 and 0.25 gives 5.9, and at 0.2 the wettest
/// ridge receives 12.8 m a year against the pack's 10-12 m on the Alps'
/// main divide. It is the only calibrated number in the orographic rain:
/// the shadow follows from the vapour the windward slopes take out of the
/// air.
pub const OROGRAPHIC_EFFICIENCY: f64 = 0.2;
/// Scale height (m) of water vapour: it is concentrated in the lowest
/// ~2 km, so lifting a column's vapour through that depth condenses it.
const VAPOUR_SCALE_HEIGHT_M: f64 = 2_000.0;
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
    for (row, &lat) in latitudes.iter().enumerate().take(rows) {
        let south = row.saturating_sub(1);
        let north = (row + 1).min(rows - 1);
        let span = (north - south).max(1) as f64 * row_spacing_rad;
        for col in 0..cols {
            let idx = row * cols + col;
            u[idx] = zonal_wind(lat, *coriolis.get(row, col));
            let d_t_dlat = (climate.surface_temperature.get(north, col)
                - climate.surface_temperature.get(south, col))
                / span;
            v[idx] = meridional_wind_from_gradient(d_t_dlat);
        }
    }

    relax_to_edges(&mut u, rows, cols, |edge, i| {
        edge_value(atmosphere, edge, i, |e| &e.wind_u_m_s)
    });
    relax_to_edges(&mut v, rows, cols, |edge, i| {
        edge_value(atmosphere, edge, i, |e| &e.wind_v_m_s)
    });
    let wind = Grid2::from_data(
        &spec,
        u.into_iter()
            .zip(v)
            .map(|(u_east, v_north)| WindVector { u_east, v_north })
            .collect(),
    );

    // Humidity: upstream gives land a continental 0.62 against the sea's
    // 0.80, which over an island is the wrong air -- the air over its land
    // came off the sea. The sea's humidity is carried inland on the wind
    // and approaches the land's own over the same adjustment length as the
    // sea's heat (deviation D38, as D37 for temperature).
    let land: Vec<bool> = elevation_m.data().iter().map(|&h| h > 0.0).collect();
    let equilibrium_rh: Vec<f64> = (0..rows * cols)
        .map(|idx| relative_humidity(latitudes[idx / cols], !land[idx]))
        .collect();
    let carried_rh = carry_from_sea(
        &equilibrium_rh,
        &land,
        &wind,
        rows,
        cols,
        domain.cell_size_m(coarse),
    );

    let mut moisture = vec![0.0; rows * cols];
    let mut weight = vec![0.0; rows * cols];
    for row in 0..rows {
        for col in 0..cols {
            let idx = row * cols + col;
            let temp = *climate.surface_temperature.get(row, col);
            let rh = carried_rh[idx];
            let q = saturation_specific_humidity(temp, pressure_hpa) * rh;
            let precipitable_water = q * air_column_kg_m2 * VAPOUR_COLUMN_FRACTION;
            moisture[idx] = precipitable_water;
            weight[idx] = rainout_weight(precipitable_water, rh);
        }
    }

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

    WeatherState {
        wind,
        moisture: Grid2::from_data(&spec, moisture),
        precipitation: Grid2::from_data(&spec, precipitation),
    }
}

/// Orographic rain over `elevation_m` (coarse), with the vapour budget it
/// draws on.
///
/// Air forced up a slope (`lift = wind · ∇h`, m/s) condenses vapour at
/// `efficiency × lift / H` of its column per second, and that rain is
/// taken out of the air: a column crossing a windward slope leaves it
/// drier, by `exp(-efficiency × lift × t / H)` over its crossing time `t`.
/// The vapour the air still carries is a fraction `f` of what it would
/// hold in equilibrium; every cell's ordinary rain scales with it, so the
/// rain shadow persists downwind of a range for as long as the air stays
/// depleted, recovering by evapotranspiration over the same adjustment
/// length as the air's heat and humidity ([`super::maritime`]). Over the
/// sea the air is replenished at once.
///
/// Nothing is rescaled afterwards. The rain a range wrings out is vapour
/// that would otherwise have passed on, so the island's total rises above
/// the background's by exactly what the windward slopes condensed; the
/// background normalisation of [`step_regional_weather`] still holds over
/// the open sea (deviations D26 and D38).
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
    let h = |r: usize, c: usize| elevation_m.get(r, c).max(0.0);
    let land: Vec<bool> = elevation_m.data().iter().map(|&e| e > 0.0).collect();
    // Rate (1/s) at which each land cell's lift condenses its column.
    let mut condensing = vec![0.0; rows * cols];
    for r in 0..rows {
        let (rs, rn) = (r.saturating_sub(1), (r + 1).min(rows - 1));
        for c in 0..cols {
            let idx = r * cols + c;
            if !land[idx] {
                continue;
            }
            let (cw, ce) = (c.saturating_sub(1), (c + 1).min(cols - 1));
            let dh_dx = (h(r, ce) - h(r, cw)) / ((ce - cw).max(1) as f64 * size);
            let dh_dy = (h(rn, c) - h(rs, c)) / ((rn - rs).max(1) as f64 * size);
            let wind = weather.wind.get(r, c);
            let lift = wind.u_east * dh_dx + wind.v_north * dh_dy;
            condensing[idx] = OROGRAPHIC_EFFICIENCY * lift.max(0.0) / VAPOUR_SCALE_HEIGHT_M;
        }
    }
    let budget = carry_vapour(&land, &condensing, &weather.wind, rows, cols, size);
    for idx in 0..rows * cols {
        if !land[idx] {
            continue;
        }
        let column = weather.moisture.data()[idx].max(0.0);
        let base = weather.precipitation.data()[idx] * budget.remaining[idx];
        let wrung = column * budget.condensed[idx] * SECONDS_PER_DAY;
        weather.precipitation.data_mut()[idx] = base + wrung;
    }
}

/// The vapour budget along the wind: per cell, the fraction of its
/// equilibrium vapour the air still carries, and the fraction of it
/// condensed per second on the cell's slope.
struct VapourBudget {
    remaining: Vec<f64>,
    condensed: Vec<f64>,
}

/// Solve the vapour fraction `f` along the wind: 1 over the sea; over land
/// the arriving air (upwind cells weighted by the wind's components) loses
/// `1 - exp(-k t)` of its vapour to the slope's condensation `k` over its
/// crossing time `t`, then recovers toward 1 over the crossing distance by
/// `exp(-d / L)`. Fast sweeping to convergence, as for the air's heat.
fn carry_vapour(
    land: &[bool],
    condensing: &[f64],
    wind: &Grid2<WindVector>,
    rows: usize,
    cols: usize,
    cell_m: f64,
) -> VapourBudget {
    let length = adjustment_length_m();
    let mut f = vec![1.0; rows * cols];
    let mut condensed = vec![0.0; rows * cols];
    let orders: [(bool, bool); 4] = [(false, false), (true, false), (false, true), (true, true)];
    for _ in 0..(rows + cols).max(1) {
        let mut largest_change: f64 = 0.0;
        for (rows_back, cols_back) in orders {
            for ri in 0..rows {
                let row = if rows_back { rows - 1 - ri } else { ri };
                for ci in 0..cols {
                    let col = if cols_back { cols - 1 - ci } else { ci };
                    let i = row * cols + col;
                    if !land[i] {
                        continue;
                    }
                    let w = *wind.get(row, col);
                    let (u, v) = (w.u_east, w.v_north);
                    let speed = u.hypot(v);
                    let weight = u.abs() + v.abs();
                    let (updated, rate) = if speed <= f64::EPSILON || weight <= f64::EPSILON {
                        // Still air carries nothing in or out.
                        (1.0, 0.0)
                    } else {
                        let up_col = if u > 0.0 {
                            col.saturating_sub(1)
                        } else {
                            (col + 1).min(cols - 1)
                        };
                        let up_row = if v > 0.0 {
                            row.saturating_sub(1)
                        } else {
                            (row + 1).min(rows - 1)
                        };
                        let arriving = (u.abs() * f[row * cols + up_col]
                            + v.abs() * f[up_row * cols + col])
                            / weight;
                        let crossing = cell_m * weight / speed;
                        let seconds = crossing / speed;
                        let left = arriving * (-condensing[i] * seconds).exp();
                        // Condensed per second, as a fraction of the
                        // equilibrium column: what the crossing took out,
                        // spread over the time it took.
                        let rate = if seconds > 0.0 {
                            (arriving - left) / seconds
                        } else {
                            0.0
                        };
                        let kept = (-crossing / length).exp();
                        (1.0 + (left - 1.0) * kept, rate)
                    };
                    largest_change = largest_change.max((updated - f[i]).abs());
                    f[i] = updated;
                    condensed[i] = rate;
                }
            }
        }
        if largest_change <= 1e-12 {
            break;
        }
    }
    VapourBudget {
        remaining: f,
        condensed,
    }
}
