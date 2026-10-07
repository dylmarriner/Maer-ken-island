//! Regional climate (Phase 2 Task 3) on the coarse grid.
//!
//! Upstream's zonal energy balance (`climate::step_climate_on`) with the
//! domain's latitudes. What a window cannot compute from its own cells
//! (CO₂, the planetary mean radiative temperature the meridional
//! transport relaxes toward, the mean absorbed flux) comes from the zonal
//! background, so the island stays on Maer-Ken's climate. Two things
//! upstream lacks are added here:
//!
//! - a **lapse rate**: land cools 6.5 K per km of elevation (US Standard
//!   Atmosphere; `fixtures/reference/climate`), entering as an offset on
//!   each cell's equilibrium temperature so it relaxes in with the
//!   surface's heat capacity;
//! - a **diurnal cycle** ([`diurnal_offset_k`]): upstream is a daily mean;
//!   the island resolves day and night.
//!
//! The outermost cells relax toward the ocean-edge SST and air
//! temperature through [`relax_to_edges`](super::edge::relax_to_edges).

use mk_core::canon::CanonLocked;
use mk_core::grid::Grid2;
use mk_island::{AtmosphereBoundaryForcing, DomainLevel, Edge, IslandDomain, OceanBoundaryForcing};
use serde::{Deserialize, Serialize};

use super::edge::relax_to_edges;
use super::zonal::ZonalBackgroundState;
use crate::climate::{step_climate_on, ClimateForcing, ClimateState, RegionalClimateTerms};

/// Environmental lapse rate (K/m): 6.5 K/km, the US Standard Atmosphere.
pub const LAPSE_RATE_K_PER_M: f64 = 6.5e-3;

/// Coriolis parameter `f = 2Ω sin(latitude)` per cell of a level, from the
/// domain's latitudes and the canon's rotation.
pub fn regional_coriolis(
    domain: &IslandDomain,
    level: DomainLevel,
    canon: &CanonLocked,
) -> Grid2<f64> {
    let omega = if canon.rotation_period_s > 0.0 {
        std::f64::consts::TAU / canon.rotation_period_s
    } else {
        0.0
    };
    let (rows, cols) = (domain.rows(level), domain.cols(level));
    let data = (0..rows)
        .flat_map(|r| {
            let f = 2.0 * omega * domain.latitude_rad_for_row(level, r).sin();
            std::iter::repeat_n(f, cols)
        })
        .collect();
    Grid2::from_data(&domain.storage_spec(level), data)
}

/// Latitude (rad) of every coarse row.
pub fn coarse_latitudes(domain: &IslandDomain) -> Vec<f64> {
    (0..domain.rows(DomainLevel::Coarse))
        .map(|r| domain.latitude_rad_for_row(DomainLevel::Coarse, r))
        .collect()
}

/// Lapse-rate offsets (K) of every cell: land only, from its elevation.
pub fn lapse_offsets_k(elevation_m: &Grid2<f64>) -> Vec<f64> {
    elevation_m
        .data()
        .iter()
        .map(|&h| -LAPSE_RATE_K_PER_M * h.max(0.0))
        .collect()
}

fn edge_values<'a, T>(
    edges: &'a [T],
    edge: Edge,
    which: impl Fn(&T) -> Edge,
    values: impl Fn(&'a T) -> &'a [f64],
    index: usize,
) -> Option<f64> {
    edges
        .iter()
        .find(|e| which(e) == edge)
        .and_then(|e| values(e).get(index).copied())
        .filter(|v| v.is_finite())
        // Extreme but finite forcing stays inside what the model's
        // physics is valid for: 150-400 K.
        .map(|v| v.clamp(EDGE_TEMPERATURE_RANGE_K.0, EDGE_TEMPERATURE_RANGE_K.1))
}

/// Temperatures (K) an edge may force the surface or air toward.
pub const EDGE_TEMPERATURE_RANGE_K: (f64, f64) = (150.0, 400.0);

/// Step the regional climate on the coarse grid over `forcing.dt_seconds`.
/// `forcing`'s grids are coarse-level; `forcing.volcanic_co2_mol_yr` is the
/// island's own output and does not move CO₂ (the background owns it). A
/// zero or negative step returns `previous` unchanged.
pub fn step_regional_climate(
    previous: &ClimateState,
    forcing: &ClimateForcing<'_>,
    domain: &IslandDomain,
    background: &ZonalBackgroundState,
    atmosphere: &AtmosphereBoundaryForcing,
    ocean_edge: &OceanBoundaryForcing,
) -> ClimateState {
    if forcing.dt_seconds.is_nan() || forcing.dt_seconds <= 0.0 {
        return previous.clone();
    }
    let spec = domain.storage_spec(DomainLevel::Coarse);
    let (rows, cols) = (spec.nlat, spec.nlon);
    let offsets = lapse_offsets_k(forcing.elevation_m);
    let terms = RegionalClimateTerms {
        co2_ppm: background.atmospheric_co2_ppm,
        global_mean_radiative_k: background.global_mean_radiative_temperature_k,
        mean_absorbed_flux_w_m2: background.mean_absorbed_flux_w_m2,
        equilibrium_offset_k: &offsets,
    };
    let stepped = step_climate_on(
        previous,
        forcing,
        &coarse_latitudes(domain),
        &spec,
        Some(&terms),
    );

    let mut surface = stepped.surface_temperature.data().to_vec();
    relax_to_edges(&mut surface, rows, cols, |edge, i| {
        edge_values(
            &ocean_edge.edges,
            edge,
            |e| e.edge,
            |e| &e.sea_surface_temperature_k,
            i,
        )
    });
    let mut air = stepped.atmos_temperature.data().to_vec();
    relax_to_edges(&mut air, rows, cols, |edge, i| {
        edge_values(
            &atmosphere.edges,
            edge,
            |e| e.edge,
            |e| &e.air_temperature_k,
            i,
        )
    });
    ClimateState {
        surface_temperature: Grid2::from_data(&spec, surface),
        atmos_temperature: Grid2::from_data(&spec, air),
        ..stepped
    }
}

/// Diurnal-cycle parameters (Dai et al. 1999; Geiger et al. 2009; Kawai &
/// Wada 2007, via `fixtures/reference/climate`), for a 24 h day.
const OCEAN_DTR_K: f64 = 0.3;
const COASTAL_LAND_DTR_K: f64 = 7.0;
const INLAND_LAND_DTR_K: f64 = 12.0;
/// Distance from the sea (m) over which land goes from coastal to inland.
const CONTINENTALITY_M: f64 = 100_000.0;
/// Rain rate (mm/day) at which skies count as overcast.
const OVERCAST_RAIN_MM_DAY: f64 = 5.0;
/// Overcast reduction of the land range: the midpoint of 25-50%.
pub const OVERCAST_DTR_REDUCTION: f64 = 0.375;
/// Extra nocturnal cooling in a valley floor: a cold-air pool fills a
/// basin 500 m below its surroundings (Geiger et al. 2009).
const VALLEY_DEPTH_M: f64 = 500.0;
const VALLEY_DTR_GAIN: f64 = 0.4;
/// Local solar time of the daily temperature maximum, as a fraction of
/// the local day (14:00 of 24 h).
const PEAK_FRACTION_OF_DAY: f64 = 14.0 / 24.0;

/// Distance (m) from each coarse cell to the nearest ocean cell, 8-way
/// chamfer, zero in the ocean.
pub fn distance_to_sea_m(elevation_m: &Grid2<f64>, cell_m: f64) -> Vec<f64> {
    let (rows, cols) = (elevation_m.nlat(), elevation_m.nlon());
    let mut d: Vec<f64> = elevation_m
        .data()
        .iter()
        .map(|&h| if h <= 0.0 { 0.0 } else { f64::INFINITY })
        .collect();
    let (a, b) = (cell_m, cell_m * std::f64::consts::SQRT_2);
    let idx = |r: usize, c: usize| r * cols + c;
    for r in 0..rows {
        for c in 0..cols {
            let mut best = d[idx(r, c)];
            if r > 0 {
                best = best.min(d[idx(r - 1, c)] + a);
                if c > 0 {
                    best = best.min(d[idx(r - 1, c - 1)] + b);
                }
                if c + 1 < cols {
                    best = best.min(d[idx(r - 1, c + 1)] + b);
                }
            }
            if c > 0 {
                best = best.min(d[idx(r, c - 1)] + a);
            }
            d[idx(r, c)] = best;
        }
    }
    for r in (0..rows).rev() {
        for c in (0..cols).rev() {
            let mut best = d[idx(r, c)];
            if r + 1 < rows {
                best = best.min(d[idx(r + 1, c)] + a);
                if c > 0 {
                    best = best.min(d[idx(r + 1, c - 1)] + b);
                }
                if c + 1 < cols {
                    best = best.min(d[idx(r + 1, c + 1)] + b);
                }
            }
            if c + 1 < cols {
                best = best.min(d[idx(r, c + 1)] + a);
            }
            d[idx(r, c)] = best;
        }
    }
    d
}

/// Static surface description the diurnal cycle needs per coarse cell.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiurnalSurface {
    /// Peak-to-peak range of the 24 h cycle (K) before the day-length
    /// scaling, with clear skies.
    pub clear_sky_range_k: Vec<f64>,
}

impl DiurnalSurface {
    /// From the coarse elevation: ocean has the skin-SST range, land
    /// widens from the coastal to the inland range with distance from the
    /// sea, and a valley floor (below the mean of its 8 neighbours)
    /// widens further.
    pub fn from_elevation(elevation_m: &Grid2<f64>, cell_m: f64) -> Self {
        let (rows, cols) = (elevation_m.nlat(), elevation_m.nlon());
        let sea = distance_to_sea_m(elevation_m, cell_m);
        let mut range = Vec::with_capacity(rows * cols);
        for r in 0..rows {
            for c in 0..cols {
                let h = *elevation_m.get(r, c);
                if h <= 0.0 {
                    range.push(OCEAN_DTR_K);
                    continue;
                }
                let t = (sea[r * cols + c] / CONTINENTALITY_M).clamp(0.0, 1.0);
                let mut dtr = COASTAL_LAND_DTR_K + (INLAND_LAND_DTR_K - COASTAL_LAND_DTR_K) * t;
                let (mut sum, mut n) = (0.0, 0.0);
                for dr in -1i64..=1 {
                    for dc in -1i64..=1 {
                        let (rr, cc) = (r as i64 + dr, c as i64 + dc);
                        if (dr, dc) != (0, 0)
                            && rr >= 0
                            && cc >= 0
                            && (rr as usize) < rows
                            && (cc as usize) < cols
                        {
                            sum += elevation_m.get(rr as usize, cc as usize).max(0.0);
                            n += 1.0;
                        }
                    }
                }
                let valley = if n > 0.0 {
                    ((sum / n - h) / VALLEY_DEPTH_M).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                dtr *= 1.0 + VALLEY_DTR_GAIN * valley;
                range.push(dtr);
            }
        }
        Self {
            clear_sky_range_k: range,
        }
    }
}

/// The deviation (K) of every coarse cell's temperature from its daily
/// mean at `sim_time_seconds`: a sinusoid peaking at 14:00 local solar
/// time whose peak-to-peak amplitude is the surface's clear-sky range,
/// reduced under cloud (overcast: -37.5%, over land only) and widened on
/// a longer day (heating and cooling run longer; amplitude scales with
/// the square root of the day length against 24 h). It averages to zero
/// over a local day, so adding it never changes the daily mean the
/// zonal-background coupling works with.
///
/// `precipitation_mm_day` stands in for cloud cover: no rain is clear,
/// [`OVERCAST_RAIN_MM_DAY`] or more is overcast.
pub fn diurnal_offset_k(
    surface: &DiurnalSurface,
    elevation_m: &Grid2<f64>,
    precipitation_mm_day: &Grid2<f64>,
    domain: &IslandDomain,
    canon: &CanonLocked,
    sub_solar_longitude_rad: f64,
) -> Grid2<f64> {
    let coarse = DomainLevel::Coarse;
    let (rows, cols) = (domain.rows(coarse), domain.cols(coarse));
    let day_scale = (canon.rotation_period_s / 86_400.0).max(0.0).sqrt();
    let mut data = Vec::with_capacity(rows * cols);
    for r in 0..rows {
        for c in 0..cols {
            let idx = r * cols + c;
            let land = *elevation_m.get(r, c) > 0.0;
            let cloud = (precipitation_mm_day.get(r, c) / OVERCAST_RAIN_MM_DAY).clamp(0.0, 1.0);
            let cloud_factor = if land {
                1.0 - OVERCAST_DTR_REDUCTION * cloud
            } else {
                1.0
            };
            let amplitude = 0.5 * surface.clear_sky_range_k[idx] * cloud_factor * day_scale;
            // Local solar time as a fraction of the local day: noon = 0.5.
            let hour_angle = domain.longitude_rad_for_col(coarse, c) - sub_solar_longitude_rad;
            let fraction = (0.5 + hour_angle / std::f64::consts::TAU).rem_euclid(1.0);
            data.push(
                amplitude * (std::f64::consts::TAU * (fraction - PEAK_FRACTION_OF_DAY)).cos(),
            );
        }
    }
    Grid2::from_data(&domain.storage_spec(coarse), data)
}
