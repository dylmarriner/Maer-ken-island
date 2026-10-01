//! OCEAN MODULE — PHASE 2
//!
//! Purpose
//! - Full water model: depth, temperature, currents, mixing
//! - Ocean circulation driven by wind stress and temperature gradients
//! - Water mass balance: evaporation, precipitation, runoff
//!
//! Invariants
//! - All arithmetic uses f64 for continuous quantities
//! - Ocean temperature determined by energy input (insolation, tidal)
//! - Wind-driven surface currents + thermohaline circulation
//! - Same tick + forcing → same ocean state (deterministic)
//! - Ledger-tracked: OceanHeat from all sources
//!
//! Authority
//! - MAERKEN_PHASE_2_PHYSICAL_WORLD.md § 3.8
//! - PLANET_CONSTANTS.md § 8 (water distribution, ocean parameters)
//! - NATURE_CONSTANTS.md (ocean properties)

use mk_core::canon::CanonLocked;
use mk_core::grid::Grid2;
use serde::{Deserialize, Serialize};

const RHO0_SW: f64 = 1034.0;
const ALPHA_T_SW: f64 = 2.1e-4;
const BETA_S_SW: f64 = 7.6e-4;
const T_REF_SW: f64 = 286.0;
const S_REF_SW: f64 = 30.6;

/// Ocean velocity vector (zonal and meridional currents)
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct OceanVelocity {
    /// Zonal current (cm/s, positive = eastward)
    pub u_east: f64,
    /// Meridional current (cm/s, positive = northward)
    pub v_north: f64,
}

impl OceanVelocity {
    /// Get current speed (magnitude)
    pub fn speed(&self) -> f64 {
        (self.u_east * self.u_east + self.v_north * self.v_north).sqrt()
    }
}

/// Ocean water column state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OceanColumn {
    /// Depth (m)
    pub depth: f64,
    /// Surface temperature (K)
    pub surface_temp: f64,
    /// Mean temperature (K)
    pub mean_temp: f64,
    /// Salinity (PSU, practical salinity units)
    pub salinity: f64,
    /// Density (kg/m³)
    pub density: f64,
}

/// Ocean state for Phase 2 integration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OceanState {
    pub columns: Grid2<OceanColumn>,
    pub currents: Grid2<OceanVelocity>,
    /// Real air-sea heat exchange this tick (W/m², summed over ocean
    /// cells), split by direction: `heat_absorbed` where the mixed layer
    /// warmed, `heat_released` (as a positive magnitude) where it cooled —
    /// its heat capacity times the temperature change over the interval.
    /// `water_gained`/`water_lost` are precipitation onto and bulk-formula
    /// evaporation from the sea surface (mm/day, summed over ocean cells).
    ///
    /// `#[serde(default)]` on all four fields below: added after some
    /// `WorldState` snapshots may already exist on disk without them — see
    /// the same note on
    /// `humans::technology::TechnologySnapshot::accumulated_knowledge`.
    #[serde(default)]
    pub heat_absorbed_w_m2: f64,
    #[serde(default)]
    pub heat_released_w_m2: f64,
    /// Real net precipitation-minus-evaporation this tick (the same
    /// `precip_minus_evap` quantity already computed to update salinity),
    /// summed over the grid and split by direction: `water_gained` where
    /// precipitation exceeds evaporation, `water_lost` (as a positive
    /// magnitude) where evaporation dominates.
    #[serde(default)]
    pub water_gained_mm_day: f64,
    #[serde(default)]
    pub water_lost_mm_day: f64,
}

impl OceanColumn {
    /// Create new ocean column
    pub fn new(depth: f64, temp: f64, salinity: f64) -> Self {
        let density = ocean_density(temp, salinity);
        OceanColumn {
            depth,
            surface_temp: temp,
            mean_temp: temp * 0.95, // Mean slightly cooler than surface
            salinity,
            density,
        }
    }
}

/// Compute ocean density from the first-order seawater density envelope.
fn ocean_density(temp_k: f64, salinity: f64) -> f64 {
    let density =
        RHO0_SW * (1.0 - ALPHA_T_SW * (temp_k - T_REF_SW) + BETA_S_SW * (salinity - S_REF_SW));
    density.max(1000.0)
}

/// Compute surface current from wind stress
///
/// Ekman current: wind drives surface layer ~45° to wind direction
fn surface_current_from_wind(
    wind_u: f64,    // m/s
    wind_v: f64,    // m/s
    _coriolis: f64, // rad/s
) -> OceanVelocity {
    let tau_x = wind_u.abs() * wind_u;
    let tau_y = wind_v.abs() * wind_v;
    let factor = 0.008;
    let cos45 = std::f64::consts::FRAC_1_SQRT_2;
    let u_current = (tau_x * factor * cos45) / 100.0;
    let v_current = (tau_y * factor * cos45) / 100.0;

    OceanVelocity {
        u_east: u_current,
        v_north: v_current,
    }
}

/// Compute thermohaline circulation (overturning)
///
/// Density-driven: cold salty water sinks, warm fresh water rises
fn thermohaline_velocity(density_diff: f64, // kg/m³
) -> f64 {
    density_diff * 0.01 // cm/s equivalent
}

/// Depth of the wind-mixed surface layer whose salinity responds to
/// precipitation and evaporation (m).
const MIXED_LAYER_DEPTH_M: f64 = 50.0;
/// Areal heat capacity of that mixed layer (J m⁻² K⁻¹), the same value the
/// climate model uses for ocean thermal inertia.
const MIXED_LAYER_HEAT_CAPACITY: f64 = 2.1e8;
/// Relaxation time of the full-depth column temperature toward the
/// surface (years): the deep ocean turns over on centennial timescales.
const DEEP_OCEAN_RELAXATION_YEARS: f64 = 500.0;
/// Bulk transfer coefficient for evaporation (Dalton number).
const EVAPORATION_TRANSFER_COEFFICIENT: f64 = 1.2e-3;
/// Minimum effective wind for evaporation: free convection keeps a still
/// sea evaporating.
const MIN_EVAPORATION_WIND_M_S: f64 = 1.0;
const R_DRY_AIR: f64 = 287.05;
const SECONDS_PER_DAY: f64 = 86_400.0;

/// Everything an ocean step is forced by.
#[derive(Debug, Clone, Copy)]
pub struct OceanForcing<'a> {
    pub wind: &'a Grid2<super::weather::WindVector>,
    pub climate: &'a super::climate::ClimateState,
    /// Precipitation reaching the surface (mm/day).
    pub precipitation_mm_day: &'a Grid2<f64>,
    pub coriolis: &'a Grid2<f64>,
    /// Elevation (m); cells at or below sea level hold a water column of
    /// that depth, land cells hold none.
    pub elevation_m: &'a Grid2<f64>,
    /// Simulated seconds since the previous ocean state.
    pub dt_seconds: f64,
}

/// Bulk aerodynamic evaporation (mm/day) from a sea surface at `sst_k`
/// under wind `wind_m_s`, with overlying air at relative humidity `rh`.
fn evaporation_mm_day(sst_k: f64, wind_m_s: f64, rh: f64, pressure_pa: f64) -> f64 {
    let air_density = pressure_pa / (R_DRY_AIR * sst_k.max(150.0));
    let q_sat = super::weather::saturation_specific_humidity(sst_k, pressure_pa / 100.0);
    let humidity_deficit = q_sat * (1.0 - rh).max(0.0);
    air_density
        * EVAPORATION_TRANSFER_COEFFICIENT
        * wind_m_s.max(MIN_EVAPORATION_WIND_M_S)
        * humidity_deficit
        * SECONDS_PER_DAY
}

/// Step ocean state forward from `previous` under `forcing`.
///
/// Sea-surface temperature is the climate model's surface temperature for
/// ocean cells (which already carries mixed-layer thermal inertia); the
/// air-sea heat flux is the energy that change took. The full-depth column
/// relaxes toward the surface over centuries. Mixed-layer salinity is
/// diluted by precipitation and concentrated by bulk-formula evaporation.
/// Land cells carry no water column.
pub fn step_ocean(
    canon: &CanonLocked,
    previous: &Grid2<OceanColumn>,
    forcing: &OceanForcing<'_>,
    grid_spec: &mk_core::grid::GridSpec,
) -> OceanState {
    let grid_rows = grid_spec.nlat;
    let grid_cols = grid_spec.nlon;
    let dt = forcing.dt_seconds.max(0.0);
    let dt_days = dt / SECONDS_PER_DAY;
    let deep_keep = (-dt / (DEEP_OCEAN_RELAXATION_YEARS * 365.25 * SECONDS_PER_DAY)).exp();

    let mut ocean = Grid2::new(grid_spec, OceanColumn::new(0.0, 0.0, 0.0));
    let mut currents = Grid2::new(
        grid_spec,
        OceanVelocity {
            u_east: 0.0,
            v_north: 0.0,
        },
    );

    let mut heat_absorbed_total = 0.0;
    let mut heat_released_total = 0.0;
    let mut water_gained_total = 0.0;
    let mut water_lost_total = 0.0;

    for row in 0..grid_rows {
        for col in 0..grid_cols {
            let sst = *forcing.climate.surface_temperature.get(row, col);
            let elevation = forcing
                .elevation_m
                .get_safe(row, col)
                .copied()
                .unwrap_or(0.0);
            if elevation > 0.0 {
                // Land: no water column.
                ocean.set(
                    row,
                    col,
                    OceanColumn {
                        depth: 0.0,
                        surface_temp: sst,
                        mean_temp: sst,
                        salinity: 0.0,
                        density: 0.0,
                    },
                );
                continue;
            }
            let depth = -elevation;
            let prior = previous.get(row, col);
            let (prior_sst, prior_mean, prior_salinity) = if prior.depth > 0.0 {
                (prior.surface_temp, prior.mean_temp, prior.salinity)
            } else {
                // Newly submerged (or first) column starts at the surface
                // state with a standard seawater salinity.
                (sst, sst, 35.0)
            };

            // Air-sea heat flux: the energy the mixed layer took to change
            // temperature over this interval (W/m²).
            if dt > 0.0 {
                let heat_flux = MIXED_LAYER_HEAT_CAPACITY * (sst - prior_sst) / dt;
                if heat_flux >= 0.0 {
                    heat_absorbed_total += heat_flux;
                } else {
                    heat_released_total += -heat_flux;
                }
            }
            let mean_temp = sst + (prior_mean - sst) * deep_keep;

            // Freshwater exchange: precipitation in, evaporation out.
            let wind = *forcing.wind.get(row, col);
            let rh = super::weather::relative_humidity(grid_spec.lat_rad(row), true);
            let evaporation =
                evaporation_mm_day(sst, wind.speed(), rh, canon.sea_level_pressure_pa);
            let precipitation = forcing.precipitation_mm_day.get(row, col).max(0.0);
            water_gained_total += precipitation;
            water_lost_total += evaporation;
            let net_freshwater_m = (precipitation - evaporation) / 1000.0 * dt_days;
            let salinity =
                (prior_salinity * (1.0 - net_freshwater_m / MIXED_LAYER_DEPTH_M)).clamp(0.0, 45.0);

            let density = ocean_density(sst, salinity);

            // Wind-driven surface current plus the meridional overturning
            // driven by the density contrast with the equatorward neighbour.
            let coriolis = *forcing.coriolis.get(row, col);
            let surface_curr = surface_current_from_wind(wind.u_east, wind.v_north, coriolis);
            let density_gradient = if row > 0 && previous.get(row - 1, col).depth > 0.0 {
                density - previous.get(row - 1, col).density
            } else {
                0.0
            };
            let thermohaline_v = thermohaline_velocity(density_gradient);

            ocean.set(
                row,
                col,
                OceanColumn {
                    depth,
                    surface_temp: sst,
                    mean_temp,
                    salinity,
                    density,
                },
            );
            currents.set(
                row,
                col,
                OceanVelocity {
                    u_east: surface_curr.u_east,
                    v_north: surface_curr.v_north + thermohaline_v * 0.001,
                },
            );
        }
    }

    OceanState {
        columns: ocean,
        currents,
        heat_absorbed_w_m2: heat_absorbed_total,
        heat_released_w_m2: heat_released_total,
        water_gained_mm_day: water_gained_total,
        water_lost_mm_day: water_lost_total,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ocean_deterministic() {
        let canon = CanonLocked::default();
        let grid_spec = mk_core::grid::GridSpec::new(4, 8);
        let climate = {
            let geothermal = mk_core::grid::Grid2::new(&grid_spec, 0.09);
            let elevation = mk_core::grid::Grid2::new(&grid_spec, -1000.0);
            crate::climate::step_climate(
                &crate::climate::ClimateState::default_for_grid(&grid_spec),
                &crate::climate::ClimateForcing {
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
        let wind = mk_core::grid::Grid2::new(
            &grid_spec,
            crate::weather::WindVector {
                u_east: 10.0,
                v_north: 0.0,
            },
        );
        let coriolis = mk_core::grid::Grid2::new(&grid_spec, 0.0);
        let ocean_prev =
            mk_core::grid::Grid2::new(&grid_spec, OceanColumn::new(4000.0, 280.0, 35.0));

        let state1 = step_ocean(
            &canon,
            &ocean_prev,
            &OceanForcing {
                wind: &wind,
                climate: &climate,
                precipitation_mm_day: &mk_core::grid::Grid2::new(&grid_spec, 2.0),
                coriolis: &coriolis,
                elevation_m: &mk_core::grid::Grid2::new(&grid_spec, -3000.0),
                dt_seconds: 86_400.0,
            },
            &grid_spec,
        );
        let state2 = step_ocean(
            &canon,
            &ocean_prev,
            &OceanForcing {
                wind: &wind,
                climate: &climate,
                precipitation_mm_day: &mk_core::grid::Grid2::new(&grid_spec, 2.0),
                coriolis: &coriolis,
                elevation_m: &mk_core::grid::Grid2::new(&grid_spec, -3000.0),
                dt_seconds: 86_400.0,
            },
            &grid_spec,
        );

        // Check current fields are identical
        for row in 0..grid_spec.nlat {
            for col in 0..grid_spec.nlon {
                let w1 = state1.currents.get(row, col);
                let w2 = state2.currents.get(row, col);
                assert_eq!(w1.u_east, w2.u_east);
                assert_eq!(w1.v_north, w2.v_north);
            }
        }
    }

    #[test]
    fn ocean_density_increases_with_salinity() {
        let rho_fresh = ocean_density(288.15, 0.0);
        let rho_salty = ocean_density(288.15, 35.0);

        assert!(
            rho_salty > rho_fresh,
            "Density should increase with salinity"
        );
    }

    #[test]
    fn ocean_density_decreases_with_temperature() {
        let rho_cold = ocean_density(273.15, 35.0); // 0°C
        let rho_warm = ocean_density(303.15, 35.0); // 30°C

        assert!(
            rho_warm < rho_cold,
            "Density should decrease with temperature"
        );
    }

    #[test]
    fn ocean_velocity_speed() {
        let vel = OceanVelocity {
            u_east: 3.0,
            v_north: 4.0,
        };
        assert_eq!(vel.speed(), 5.0);
    }

    #[test]
    fn warmer_atmosphere_yields_real_heat_absorbed_and_no_release() {
        let canon = CanonLocked::default();
        let grid_spec = mk_core::grid::GridSpec::new(4, 8);
        let mut climate = crate::climate::ClimateState::new(&grid_spec, 280.0);
        // The surface warms from the previous 280 K everywhere, so the
        // ocean absorbs heat in every cell.
        climate.surface_temperature = mk_core::grid::Grid2::new(&grid_spec, 285.0);

        let wind = mk_core::grid::Grid2::new(
            &grid_spec,
            crate::weather::WindVector {
                u_east: 0.0,
                v_north: 0.0,
            },
        );
        let coriolis = mk_core::grid::Grid2::new(&grid_spec, 0.0);
        let ocean_prev =
            mk_core::grid::Grid2::new(&grid_spec, OceanColumn::new(4000.0, 280.0, 35.0));

        let state = step_ocean(
            &canon,
            &ocean_prev,
            &OceanForcing {
                wind: &wind,
                climate: &climate,
                precipitation_mm_day: &mk_core::grid::Grid2::new(&grid_spec, 2.0),
                coriolis: &coriolis,
                elevation_m: &mk_core::grid::Grid2::new(&grid_spec, -3000.0),
                dt_seconds: 86_400.0,
            },
            &grid_spec,
        );

        assert!(state.heat_absorbed_w_m2 > 0.0);
        assert_eq!(state.heat_released_w_m2, 0.0);
    }

    #[test]
    fn ocean_column_creation() {
        let col = OceanColumn::new(4000.0, 280.0, 35.0);
        assert_eq!(col.depth, 4000.0);
        assert_eq!(col.surface_temp, 280.0);
        assert!(col.density > 1000.0, "Ocean density should be > 1000 kg/m³");
    }

    #[test]
    fn tropical_evaporation_is_a_few_mm_per_day() {
        let e = evaporation_mm_day(300.0, 7.0, 0.8, 101_325.0);
        assert!((2.0..6.0).contains(&e), "evaporation {e} mm/day");
    }

    #[test]
    fn land_cells_hold_no_water_and_rain_freshens_the_sea() {
        let canon = CanonLocked::default();
        let grid_spec = mk_core::grid::GridSpec::new(2, 2);
        let climate = crate::climate::ClimateState::new(&grid_spec, 290.0);
        let wind = mk_core::grid::Grid2::new(
            &grid_spec,
            crate::weather::WindVector {
                u_east: 0.0,
                v_north: 0.0,
            },
        );
        let coriolis = mk_core::grid::Grid2::new(&grid_spec, 0.0);
        let elevation =
            mk_core::grid::Grid2::from_data(&grid_spec, vec![500.0, -2000.0, -2000.0, -2000.0]);
        let heavy_rain = mk_core::grid::Grid2::new(&grid_spec, 50.0);
        let ocean_prev =
            mk_core::grid::Grid2::new(&grid_spec, OceanColumn::new(2000.0, 290.0, 35.0));

        let state = step_ocean(
            &canon,
            &ocean_prev,
            &OceanForcing {
                wind: &wind,
                climate: &climate,
                precipitation_mm_day: &heavy_rain,
                coriolis: &coriolis,
                elevation_m: &elevation,
                dt_seconds: 30.0 * 86_400.0,
            },
            &grid_spec,
        );

        assert_eq!(state.columns.get(0, 0).depth, 0.0);
        assert_eq!(state.columns.get(0, 1).depth, 2000.0);
        assert!(state.columns.get(0, 1).salinity < 35.0);
    }
}
