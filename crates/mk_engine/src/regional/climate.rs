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
use super::maritime::{
    boundary_layer_air_k, sensible_conductance_w_m2_k, AIR_SPECIFIC_HEAT_J_KG_K,
    DRY_AIR_GAS_CONSTANT_J_KG_K,
};
use super::zonal::ZonalBackgroundState;
use crate::climate::{step_climate_on, ClimateForcing, ClimateState, RegionalClimateTerms};
use crate::hydrology::UNLIMITED_EVAPORATION_MOISTURE;
use crate::weather::{
    saturation_specific_humidity, WindVector, L_V as LATENT_HEAT_VAPORISATION_J_KG,
};

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
    wind: &Grid2<WindVector>,
) -> ClimateState {
    if forcing.dt_seconds.is_nan() || forcing.dt_seconds <= 0.0 {
        return previous.clone();
    }
    let spec = domain.storage_spec(DomainLevel::Coarse);
    let (rows, cols) = (spec.nlat, spec.nlon);
    let offsets = lapse_offsets_k(forcing.elevation_m);
    // The sea's air, carried over the land by the wind the island had last
    // step (deviation D37).
    let air = boundary_layer_air_k(
        previous.surface_temperature.data(),
        forcing.elevation_m.data(),
        wind,
        rows,
        cols,
        domain.cell_size_m(DomainLevel::Coarse),
    );
    let conductance: Vec<f64> = (0..rows * cols)
        .map(|i| {
            if forcing.elevation_m.data()[i] > 0.0 {
                let w = wind.data()[i];
                sensible_conductance_w_m2_k(
                    forcing.surface_pressure_pa,
                    air[i],
                    w.u_east.hypot(w.v_north),
                )
            } else {
                0.0
            }
        })
        .collect();
    let terms = RegionalClimateTerms {
        co2_ppm: background.atmospheric_co2_ppm,
        global_mean_radiative_k: background.global_mean_radiative_temperature_k,
        mean_absorbed_flux_w_m2: background.mean_absorbed_flux_w_m2,
        equilibrium_offset_k: &offsets,
        boundary_air_k: &air,
        sensible_conductance_w_m2_k: &conductance,
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

/// Diurnal range of the ocean's skin temperature for a 24 h day (Kawai &
/// Wada 2007, via `fixtures/reference/climate`): the mixed layer's heat
/// capacity holds it to tenths of a kelvin (D25 has no diurnal mixed
/// layer to compute it from).
const OCEAN_DTR_K: f64 = 0.3;
/// Typical ranges of near-surface air on maritime coasts and inland
/// grass (Dai et al. 1999; Geiger et al. 2009). Only their ratio is used:
/// how much the sea's air damps the land's own range at the shore.
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
/// Lapse rate of potential temperature above the boundary layer (K/m),
/// into which the daytime convective layer grows: the ERA5 median at
/// FLUXNET sites, alike for short vegetation, savanna and forest (Panwar &
/// Kleidon 2022, J. Climate 35, 6301).
pub const BACKGROUND_THETA_LAPSE_K_PER_M: f64 = 7.0e-3;
/// Shortwave albedo of the land: grass, 0.16-0.26 (Oke 1987, *Boundary
/// Layer Climates*, Table 1.1), the midpoint.
pub const LAND_ALBEDO: f64 = 0.21;
/// Clear-sky solar radiation at the surface over that at the top of the
/// atmosphere, at sea level and its gain per metre of elevation (Allen et
/// al. 1998, FAO-56, Eq. 37: `0.75 + 2e-5 z`).
pub const CLEAR_SKY_TRANSMITTANCE: f64 = 0.75;
pub const CLEAR_SKY_TRANSMITTANCE_PER_M: f64 = 2.0e-5;
/// Evaporation from a wet surface over the equilibrium rate
/// `s / (s + gamma)` of the available energy (Priestley & Taylor 1972).
pub const PRIESTLEY_TAYLOR_ALPHA: f64 = 1.26;

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
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DiurnalSurface {
    /// The share of the land's own diurnal range its air keeps: zero over
    /// the sea; on land the coastal-to-inland ratio at the shore rising to
    /// one [`CONTINENTALITY_M`] inland, widened on a valley floor. Empty in
    /// a state saved before the range came from the energy balance; the
    /// next step rebuilds it from the terrain.
    #[serde(default)]
    pub land_range_share: Vec<f64>,
}

impl DiurnalSurface {
    /// From the coarse elevation: land air keeps less of its range near
    /// the sea, whose air it partly is, and more on a valley floor (below
    /// the mean of its 8 neighbours), where cold air pools at night.
    pub fn from_elevation(elevation_m: &Grid2<f64>, cell_m: f64) -> Self {
        let (rows, cols) = (elevation_m.nlat(), elevation_m.nlon());
        let sea = distance_to_sea_m(elevation_m, cell_m);
        let coastal = COASTAL_LAND_DTR_K / INLAND_LAND_DTR_K;
        let mut share = Vec::with_capacity(rows * cols);
        for r in 0..rows {
            for c in 0..cols {
                let h = *elevation_m.get(r, c);
                if h <= 0.0 {
                    share.push(0.0);
                    continue;
                }
                let t = (sea[r * cols + c] / CONTINENTALITY_M).clamp(0.0, 1.0);
                let mut s = coastal + (1.0 - coastal) * t;
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
                s *= 1.0 + VALLEY_DTR_GAIN * valley;
                share.push(s);
            }
        }
        Self {
            land_range_share: share,
        }
    }
}

/// What sets the coming local day's temperature range, per coarse cell.
#[derive(Debug, Clone, Copy)]
pub struct DiurnalForcing<'a> {
    /// Solar flux at the top of the atmosphere at the planet's present
    /// distance from its star (W/m^2).
    pub toa_flux_w_m2: f64,
    /// Latitude of the sub-solar point (rad).
    pub declination_rad: f64,
    /// Sea-level pressure (Pa).
    pub sea_level_pressure_pa: f64,
    /// Daily-mean surface temperature (K).
    pub surface_temperature_k: &'a Grid2<f64>,
    /// Soil water as a fraction of field capacity, over each cell's land.
    pub soil_moisture: &'a Grid2<f64>,
    /// Stands in for cloud: no rain is clear, [`OVERCAST_RAIN_MM_DAY`] or
    /// more is overcast.
    pub precipitation_mm_day: &'a Grid2<f64>,
}

/// Solar energy reaching the top of the atmosphere over one local day
/// (J/m^2) at `latitude_rad`: the integral of `S cos(zenith)` from sunrise
/// to sunset, `S (P / pi) (H sin(lat) sin(dec) + sin(H) cos(lat) cos(dec))`
/// with `H` the sunset hour angle and `P` the day length. Zero in polar
/// night; the whole day's sun in polar day.
pub fn daily_toa_insolation_j_m2(
    toa_flux_w_m2: f64,
    latitude_rad: f64,
    declination_rad: f64,
    day_length_s: f64,
) -> f64 {
    let a = latitude_rad.sin() * declination_rad.sin();
    let b = latitude_rad.cos() * declination_rad.cos();
    let sunset = if b > 1e-12 {
        (-a / b).clamp(-1.0, 1.0).acos()
    } else if a > 0.0 {
        std::f64::consts::PI
    } else {
        0.0
    };
    toa_flux_w_m2 * (day_length_s / std::f64::consts::PI) * (a * sunset + b * sunset.sin())
}

/// Evaporative fraction `LE / (LE + H)` of a land surface at `temperature_k`
/// whose soil holds `soil_moisture` of field capacity: a wet surface
/// evaporates at the Priestley-Taylor rate, `alpha s / (s + gamma)` of the
/// available energy (`s / gamma = (L / c_p) dq_s/dT`), and a drying soil
/// at the hydrology's own moisture-limited share of it.
pub fn evaporative_fraction(temperature_k: f64, pressure_pa: f64, soil_moisture: f64) -> f64 {
    let hpa = pressure_pa / 100.0;
    let dqs_dt = saturation_specific_humidity(temperature_k + 0.5, hpa)
        - saturation_specific_humidity(temperature_k - 0.5, hpa);
    let s_over_gamma = LATENT_HEAT_VAPORISATION_J_KG / AIR_SPECIFIC_HEAT_J_KG_K * dqs_dt;
    let wet = (PRIESTLEY_TAYLOR_ALPHA * s_over_gamma / (1.0 + s_over_gamma)).min(1.0);
    let limit = (soil_moisture / UNLIMITED_EVAPORATION_MOISTURE).clamp(0.0, 1.0);
    wet * limit
}

/// Diurnal range of near-surface air over land (K) from the heat the
/// day's sunshine stores in the boundary layer (Panwar & Kleidon 2022):
/// the convective layer grows into air whose potential temperature rises
/// `Gamma` per metre, so storing `dU` warms it by `dT` with
/// `dT^2 = 2 Gamma dU / (c_p rho)`; what the surface evaporates is not
/// stored, cutting the range by `((2 - f_e) / (2 + f_e))^(1/2)`.
/// `absorbed_j_m2` is the solar energy the surface absorbed over the day.
pub fn land_air_range_k(
    absorbed_j_m2: f64,
    temperature_k: f64,
    pressure_pa: f64,
    evaporative_fraction: f64,
) -> f64 {
    let density = pressure_pa / (DRY_AIR_GAS_CONSTANT_J_KG_K * temperature_k.max(1.0));
    let dry = (2.0 * BACKGROUND_THETA_LAPSE_K_PER_M * absorbed_j_m2.max(0.0)
        / (AIR_SPECIFIC_HEAT_J_KG_K * density))
        .sqrt();
    let fe = evaporative_fraction.clamp(0.0, 1.0);
    dry * ((2.0 - fe) / (2.0 + fe)).sqrt()
}

/// Peak-to-peak temperature range (K) of the coming local day at every
/// coarse cell. Over land, the boundary layer's heat storage
/// ([`land_air_range_k`]) of the day's clear-sky sunshine absorbed by grass
/// at the cell's height, damped toward the sea's range near the shore,
/// widened in a valley, and cut under cloud (overcast: -37.5%). Over the
/// sea, the skin range, widened on a longer day as the square root of the
/// day length against 24 h.
pub fn diurnal_range_k(
    surface: &DiurnalSurface,
    elevation_m: &Grid2<f64>,
    forcing: &DiurnalForcing,
    domain: &IslandDomain,
    canon: &CanonLocked,
) -> Grid2<f64> {
    let coarse = DomainLevel::Coarse;
    let (rows, cols) = (domain.rows(coarse), domain.cols(coarse));
    let day_s = canon.rotation_period_s.max(0.0);
    let ocean_dtr_k = OCEAN_DTR_K * (day_s / 86_400.0).sqrt();
    let mut data = Vec::with_capacity(rows * cols);
    for r in 0..rows {
        let toa_j_m2 = daily_toa_insolation_j_m2(
            forcing.toa_flux_w_m2,
            domain.latitude_rad_for_row(coarse, r),
            forcing.declination_rad,
            day_s,
        );
        for c in 0..cols {
            let h = *elevation_m.get(r, c);
            if h <= 0.0 {
                data.push(ocean_dtr_k);
                continue;
            }
            let temperature_k = *forcing.surface_temperature_k.get(r, c);
            let transmittance = CLEAR_SKY_TRANSMITTANCE + CLEAR_SKY_TRANSMITTANCE_PER_M * h;
            let absorbed_j_m2 = toa_j_m2 * transmittance * (1.0 - LAND_ALBEDO);
            // Hypsometric pressure at the cell's height.
            let pressure_pa = forcing.sea_level_pressure_pa
                * (-canon.surface_gravity_m_s2 * h
                    / (DRY_AIR_GAS_CONSTANT_J_KG_K * temperature_k.max(1.0)))
                .exp();
            let fe =
                evaporative_fraction(temperature_k, pressure_pa, *forcing.soil_moisture.get(r, c));
            let cloud =
                (forcing.precipitation_mm_day.get(r, c) / OVERCAST_RAIN_MM_DAY).clamp(0.0, 1.0);
            let share = surface
                .land_range_share
                .get(r * cols + c)
                .copied()
                .unwrap_or(1.0);
            data.push(
                land_air_range_k(absorbed_j_m2, temperature_k, pressure_pa, fe)
                    * share
                    * (1.0 - OVERCAST_DTR_REDUCTION * cloud),
            );
        }
    }
    Grid2::from_data(&domain.storage_spec(coarse), data)
}

/// The deviation (K) of every coarse cell's temperature from its daily
/// mean at `sub_solar_longitude_rad`: a sinusoid peaking at 14:00 local
/// solar time whose peak-to-peak amplitude is the cell's `range_k`
/// ([`diurnal_range_k`]). It averages to zero over a local day, so adding
/// it never changes the daily mean the zonal-background coupling works
/// with.
pub fn diurnal_offset_k(
    range_k: &Grid2<f64>,
    domain: &IslandDomain,
    sub_solar_longitude_rad: f64,
) -> Grid2<f64> {
    let coarse = DomainLevel::Coarse;
    let (rows, cols) = (domain.rows(coarse), domain.cols(coarse));
    let mut data = Vec::with_capacity(rows * cols);
    for r in 0..rows {
        for c in 0..cols {
            let amplitude = 0.5 * range_k.get(r, c);
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
