//! The coupled regional physical tick (Phase 2 Task 6): the island's
//! climate, weather, ocean, tides and hydrology stepped together,
//! deterministically, from the geophysics and the zonal background.
//!
//! Step order: zonal background → boundary forcing (stored in the state;
//! callers never pass one) → insolation → tides → climate (+ diurnal
//! cycle) → synoptic systems → weather (+ orographic rain + synoptic
//! modulation) → hydrology on the land cells → river water to the coarse
//! ocean → ocean. Geophysics (terrain) is static until Phase 4's slow
//! cadence; its hook is a no-op here.
//!
//! Grids: climate, weather, ocean, tides and insolation are **coarse**
//! (160 × 200); hydrology is **medium** (960 × 1,200) over the land only.
//! Nothing here allocates a planetary grid.

use serde::{Deserialize, Serialize};
use std::sync::Arc;

use mk_core::canon::CanonLocked;
use mk_core::flux::Ledger;
use mk_core::grid::Grid2;
use mk_core::time::Tick;
use mk_island::{DomainLevel, IslandDomain, RegionalBoundaryState};

use super::boundary::{sample_regional_boundaries, sample_regional_boundaries_with_background};
use super::climate::{
    coarse_latitudes, diurnal_offset_k, regional_coriolis, step_regional_climate, DiurnalSurface,
};
use super::geophysics::{
    bootstrap_regional_geophysics, RegionalGeophysics, RegionalGeophysicsError,
};
use super::hydrology::{
    bootstrap_regional_hydrology, freshwater_to_coarse_ocean_kg, step_regional_hydrology_on,
    FlowNetwork,
};
use super::levels::aggregate_medium_to_coarse;
use super::ocean::step_regional_ocean;
use super::synoptic::{apply_synoptic, SynopticState};
use super::tides::step_regional_tides;
use super::weather::{apply_orographic_precipitation, step_regional_weather};
use super::zonal::{ZonalBackgroundState, DEFAULT_BAND_COUNT};
use crate::climate::{ClimateForcing, ClimateState, Climatology};
use crate::hydrology::HydrologyState;
use crate::insolation::{step_insolation_on, InsolationField};
use crate::ocean::{OceanColumn, OceanForcing, OceanState};
use crate::tides::TidalState;
use crate::weather::WeatherState;

/// Climate steps per orbit during bootstrap spin-up (ten simulated days
/// each on the canon year), as the planetary world's spin-up.
const SPIN_UP_STEPS_PER_ORBIT: usize = 54;
/// Orbits of bootstrap spin-up: the first settles thermal inertia, the
/// second fills the climatology.
const SPIN_UP_ORBITS: usize = 2;
const SECONDS_PER_DAY: f64 = 86_400.0;

#[derive(Debug, Clone, PartialEq)]
pub enum RegionalPhysicalError {
    Geophysics(RegionalGeophysicsError),
    /// A field went non-finite: the named quantity.
    NonFinite(&'static str),
}

impl std::fmt::Display for RegionalPhysicalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Geophysics(e) => write!(f, "geophysics: {e:?}"),
            Self::NonFinite(what) => write!(f, "non-finite {what}"),
        }
    }
}

impl std::error::Error for RegionalPhysicalError {}

impl From<RegionalGeophysicsError> for RegionalPhysicalError {
    fn from(e: RegionalGeophysicsError) -> Self {
        Self::Geophysics(e)
    }
}

/// Everything the island's physical systems carry between steps.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegionalPhysicalState {
    pub geophysics: RegionalGeophysics,
    pub zonal_background: ZonalBackgroundState,
    /// This step's edge forcing: the state owns it.
    pub boundaries: RegionalBoundaryState,
    /// Instantaneous top-of-atmosphere insolation (W/m², coarse); a cell
    /// is in daylight where it is positive.
    pub insolation: InsolationField,
    /// Daily-mean climate (coarse).
    pub climate: ClimateState,
    /// Instantaneous departure from the daily mean (K, coarse): add to
    /// `climate.surface_temperature` for the temperature now.
    pub diurnal_offset_k: Grid2<f64>,
    pub synoptic: SynopticState,
    pub weather: WeatherState,
    /// Medium grid.
    pub hydrology: HydrologyState,
    pub ocean: OceanState,
    pub tides: TidalState,
    /// Annual-mean temperature and rain per coarse cell.
    pub climatology: Climatology,
    /// The last step's flux entries (tidal dissipation).
    pub ledger: Ledger,
    pub sim_time_seconds: f64,
    // Derived from the terrain, rebuilt if absent.
    elevation_coarse_m: Grid2<f64>,
    geothermal_coarse_w_m2: Grid2<f64>,
    diurnal_surface: DiurnalSurface,
    network: FlowNetwork,
    /// Fresh water the last step carried into the coarse ocean (kg).
    pub river_water_to_ocean_kg: f64,
}

fn finite(grid: &Grid2<f64>, what: &'static str) -> Result<(), RegionalPhysicalError> {
    if grid.data().iter().all(|v| v.is_finite()) {
        Ok(())
    } else {
        Err(RegionalPhysicalError::NonFinite(what))
    }
}

impl RegionalPhysicalState {
    /// Generate the island's geophysics for `seed`, then spin the physical
    /// systems up through two orbits so the world starts on its seasonal
    /// cycle with a climatology, running rivers and full lakes. The
    /// returned state is at `sim_time_seconds = 0` (the same orbital phase
    /// the spin-up ended at).
    pub fn bootstrap(
        canon: &Arc<CanonLocked>,
        domain: &IslandDomain,
        seed: [u8; 32],
    ) -> Result<Self, RegionalPhysicalError> {
        let provisional = sample_regional_boundaries(seed, canon, domain, 0.0);
        let geophysics = bootstrap_regional_geophysics(canon, domain, &provisional, seed)?;
        Self::from_geophysics(canon, domain, seed, geophysics)
    }

    /// As [`bootstrap`](Self::bootstrap) for geophysics already generated.
    pub fn from_geophysics(
        canon: &Arc<CanonLocked>,
        domain: &IslandDomain,
        seed: [u8; 32],
        geophysics: RegionalGeophysics,
    ) -> Result<Self, RegionalPhysicalError> {
        let coarse = DomainLevel::Coarse;
        let spec = domain.storage_spec(coarse);
        let elevation_coarse_m = aggregate_medium_to_coarse(&geophysics.elevation_m, domain);
        let geothermal_coarse_w_m2 = geophysics.volcanism.heat_flux_w_m2(&spec);
        let zonal_background = ZonalBackgroundState::bootstrap(canon, DEFAULT_BAND_COUNT);
        let boundaries =
            sample_regional_boundaries_with_background(seed, canon, domain, &zonal_background, 0.0);
        let network = FlowNetwork::build(&geophysics.elevation_m);
        let hydrology = bootstrap_regional_hydrology(&network, domain);
        let diurnal_surface =
            DiurnalSurface::from_elevation(&elevation_coarse_m, domain.cell_size_m(coarse));
        let synoptic = SynopticState::bootstrap(seed, &zonal_background, domain, canon);

        let mut ledger = Ledger::new();
        let tides = step_regional_tides(canon, 0.0, 0, domain, &elevation_coarse_m, &mut ledger);
        let mut state = Self {
            geophysics,
            zonal_background,
            boundaries,
            insolation: InsolationField {
                toa_w_m2: Grid2::new(&spec, 0.0),
                band1: Grid2::new(&spec, 0.0),
                band2: Grid2::new(&spec, 0.0),
            },
            climate: ClimateState::default_for_grid(&spec),
            diurnal_offset_k: Grid2::new(&spec, 0.0),
            synoptic,
            weather: WeatherState::new(&spec),
            hydrology,
            ocean: OceanState {
                columns: Grid2::new(&spec, OceanColumn::new(0.0, 0.0, 0.0)),
                currents: Grid2::new(
                    &spec,
                    crate::ocean::OceanVelocity {
                        u_east: 0.0,
                        v_north: 0.0,
                    },
                ),
                heat_absorbed_w_m2: 0.0,
                heat_released_w_m2: 0.0,
                water_gained_mm_day: 0.0,
                water_lost_mm_day: 0.0,
            },
            tides,
            climatology: Climatology::default(),
            ledger,
            sim_time_seconds: 0.0,
            elevation_coarse_m,
            geothermal_coarse_w_m2,
            diurnal_surface,
            network,
            river_water_to_ocean_kg: 0.0,
        };

        // Settle straight to equilibrium at t = 0, as the planetary world
        // does, then spin through the orbits.
        state.settle(canon, domain, seed);
        let dt = canon.orbital_period_s / SPIN_UP_STEPS_PER_ORBIT as f64;
        let steps = SPIN_UP_ORBITS * SPIN_UP_STEPS_PER_ORBIT;
        for step in 1..=steps {
            state.step(canon, domain, seed, step as f64 * dt, 0, dt as u64)?;
        }
        // The planetary world starts at t = 0 on the spun-up state; the
        // orbital phase is the same.
        state.sim_time_seconds = 0.0;
        state.zonal_background.sim_time_seconds = 0.0;
        state.boundaries = sample_regional_boundaries_with_background(
            seed,
            canon,
            domain,
            &state.zonal_background,
            0.0,
        );
        Ok(state)
    }

    /// The climate at equilibrium under t = 0 forcing, before spin-up.
    fn settle(&mut self, canon: &Arc<CanonLocked>, domain: &IslandDomain, _seed: [u8; 32]) {
        let orbit = crate::orbit::step_orbit(canon, 0.0);
        let rotation = crate::rotation::step_rotation(0.0, orbit.mean_anomaly, canon);
        self.climate = step_regional_climate(
            &self.climate,
            &ClimateForcing {
                toa_solar_flux_w_m2: canon.solar_constant_w_m2 / (orbit.r_over_a * orbit.r_over_a),
                solar_declination_rad: rotation.subsolar_latitude,
                albedo: canon.albedo_baseline,
                surface_pressure_pa: canon.sea_level_pressure_pa,
                surface_gravity_m_s2: canon.surface_gravity_m_s2,
                geothermal_flux_w_m2: &self.geothermal_coarse_w_m2,
                volcanic_co2_mol_yr: 0.0,
                elevation_m: &self.elevation_coarse_m,
                dt_seconds: f64::INFINITY,
            },
            domain,
            &self.zonal_background,
            &self.boundaries.atmosphere,
            &self.boundaries.ocean,
        );
    }

    /// Advance to `sim_time_seconds` over `dt_seconds`. A zero step
    /// changes nothing.
    pub fn step(
        &mut self,
        canon: &Arc<CanonLocked>,
        domain: &IslandDomain,
        seed: [u8; 32],
        sim_time_seconds: f64,
        _tick: Tick,
        dt_seconds: u64,
    ) -> Result<(), RegionalPhysicalError> {
        if dt_seconds == 0 {
            return Ok(());
        }
        let dt = dt_seconds as f64;
        let t = sim_time_seconds;
        let coarse = DomainLevel::Coarse;
        let spec = domain.storage_spec(coarse);

        // 1-2. Background and the boundary forcing it implies.
        self.zonal_background.step(canon, t, dt);
        self.boundaries = sample_regional_boundaries_with_background(
            seed,
            canon,
            domain,
            &self.zonal_background,
            t,
        );

        // 3. Insolation at the domain's latitudes and longitudes.
        let orbit = crate::orbit::step_orbit(canon, t);
        let rotation = crate::rotation::step_rotation(t, orbit.mean_anomaly, canon);
        let latitudes = coarse_latitudes(domain);
        let longitudes: Vec<f64> = (0..spec.nlon)
            .map(|c| domain.longitude_rad_for_col(coarse, c))
            .collect();
        let mut scratch = Ledger::new();
        self.insolation = step_insolation_on(
            canon,
            &orbit,
            &rotation,
            &spec,
            &latitudes,
            &longitudes,
            &|_| domain.cell_area_m2(coarse),
            dt,
            &mut scratch,
        );

        // 4. Tides.
        self.ledger = Ledger::new();
        self.tides = step_regional_tides(
            canon,
            t,
            dt_seconds,
            domain,
            &self.elevation_coarse_m,
            &mut self.ledger,
        );

        // 5. Climate: tidal heat warms the ocean as upstream does.
        let ocean_cells = self
            .elevation_coarse_m
            .data()
            .iter()
            .filter(|&&h| h <= 0.0)
            .count() as f64;
        let tidal_w_m2 = if ocean_cells > 0.0 {
            self.tides.heating_power_w.max(0.0) / (ocean_cells * domain.cell_area_m2(coarse))
        } else {
            0.0
        };
        let mut surface_heat_flux = self.geothermal_coarse_w_m2.clone();
        for (flux, &h) in surface_heat_flux
            .data_mut()
            .iter_mut()
            .zip(self.elevation_coarse_m.data())
        {
            if h <= 0.0 {
                *flux += tidal_w_m2;
            }
        }
        self.climate = step_regional_climate(
            &self.climate,
            &ClimateForcing {
                toa_solar_flux_w_m2: canon.solar_constant_w_m2 / (orbit.r_over_a * orbit.r_over_a),
                solar_declination_rad: rotation.subsolar_latitude,
                albedo: canon.albedo_baseline,
                surface_pressure_pa: canon.sea_level_pressure_pa,
                surface_gravity_m_s2: canon.surface_gravity_m_s2,
                geothermal_flux_w_m2: &surface_heat_flux,
                volcanic_co2_mol_yr: self.geophysics.volcanism.total_co2_mol_yr(&spec),
                elevation_m: &self.elevation_coarse_m,
                dt_seconds: dt,
            },
            domain,
            &self.zonal_background,
            &self.boundaries.atmosphere,
            &self.boundaries.ocean,
        );

        // 6-7. Synoptic systems, then weather: the diagnostic field,
        // orographic redistribution, synoptic modulation.
        self.synoptic
            .step(&self.zonal_background, domain, canon, dt, seed);
        self.weather = step_regional_weather(
            canon,
            &self.climate,
            &self.elevation_coarse_m,
            domain,
            &self.zonal_background,
            &self.boundaries.atmosphere,
        );
        apply_orographic_precipitation(&mut self.weather, &self.elevation_coarse_m, domain);
        apply_synoptic(&mut self.weather, &mut self.synoptic, domain, canon, dt);

        // Diurnal departure from the daily mean, from the weather's cloud.
        self.diurnal_offset_k = diurnal_offset_k(
            &self.diurnal_surface,
            &self.elevation_coarse_m,
            &self.weather.precipitation,
            domain,
            canon,
            rotation.subsolar_longitude,
        );
        self.climatology.update(
            &self.climate.surface_temperature,
            &self.weather.precipitation,
            dt,
            (t - 0.5 * dt) / canon.orbital_period_s,
        );

        // 8. Hydrology on the land cells; 9. its river water to the ocean.
        self.hydrology = step_regional_hydrology_on(
            canon,
            &self.hydrology,
            &self.weather,
            &self.climate,
            &self.geophysics.elevation_m,
            domain,
            &self.network,
            dt,
        );
        let river_kg = freshwater_to_coarse_ocean_kg(&self.hydrology, &self.network, domain, dt);
        self.river_water_to_ocean_kg = river_kg.data().iter().sum();
        let coarse_area = domain.cell_area_m2(coarse);
        let mut ocean_precipitation = self.weather.precipitation.clone();
        for (p, kg) in ocean_precipitation
            .data_mut()
            .iter_mut()
            .zip(river_kg.data())
        {
            // kg over a coarse cell -> mm/day over it.
            *p += kg / coarse_area / (dt / SECONDS_PER_DAY);
        }

        // 10. Ocean.
        let coriolis = regional_coriolis(domain, coarse, canon);
        self.ocean = step_regional_ocean(
            canon,
            &self.ocean.columns,
            &OceanForcing {
                wind: &self.weather.wind,
                climate: &self.climate,
                precipitation_mm_day: &ocean_precipitation,
                coriolis: &coriolis,
                elevation_m: &self.elevation_coarse_m,
                dt_seconds: dt,
            },
            domain,
            &self.boundaries.ocean,
        );

        // 11. Slow geophysics cadence: terrain is static until Phase 4's
        // scheduler gives it a cadence.
        self.sim_time_seconds = t;

        finite(&self.climate.surface_temperature, "surface temperature")?;
        finite(&self.climate.atmos_temperature, "air temperature")?;
        finite(&self.weather.precipitation, "precipitation")?;
        finite(&self.weather.moisture, "moisture")?;
        finite(&self.diurnal_offset_k, "diurnal offset")?;
        let b = self.hydrology.budget;
        if ![
            b.precipitation_kg,
            b.surface_to_ocean_kg,
            b.surface_evaporation_kg,
            b.soil_evaporation_kg,
            b.deep_drainage_kg,
        ]
        .iter()
        .all(|v| v.is_finite())
        {
            return Err(RegionalPhysicalError::NonFinite("water budget"));
        }
        Ok(())
    }

    /// Peak-to-peak temperature range (K) of the coming local day at every
    /// coarse cell, from the current surface and cloud, sampled at 24
    /// evenly spaced local times.
    pub fn diurnal_range_k(&self, domain: &IslandDomain, canon: &CanonLocked) -> Grid2<f64> {
        let spec = domain.storage_spec(DomainLevel::Coarse);
        let mut lo = vec![f64::MAX; spec.nlat * spec.nlon];
        let mut hi = vec![f64::MIN; spec.nlat * spec.nlon];
        for k in 0..24 {
            let lon = std::f64::consts::TAU * f64::from(k) / 24.0;
            let offsets = diurnal_offset_k(
                &self.diurnal_surface,
                &self.elevation_coarse_m,
                &self.weather.precipitation,
                domain,
                canon,
                lon,
            );
            for (i, &o) in offsets.data().iter().enumerate() {
                lo[i] = lo[i].min(o);
                hi[i] = hi[i].max(o);
            }
        }
        Grid2::from_data(&spec, hi.into_iter().zip(lo).map(|(h, l)| h - l).collect())
    }

    /// The coarse elevation the atmosphere and ocean see (m).
    pub fn coarse_elevation_m(&self) -> &Grid2<f64> {
        &self.elevation_coarse_m
    }

    /// The land drainage network.
    pub fn flow_network(&self) -> &FlowNetwork {
        &self.network
    }

    /// A blake3 digest of the whole prognostic state (every field's exact
    /// bits), for determinism checks and replay.
    pub fn state_hash(&self) -> [u8; 32] {
        let mut h = blake3::Hasher::new();
        let mut feed = |xs: &[f64]| {
            for x in xs {
                h.update(&x.to_le_bytes());
            }
        };
        feed(self.climate.surface_temperature.data());
        feed(self.climate.atmos_temperature.data());
        feed(self.climate.subsurface_temperature.data());
        feed(&[self.climate.co2_concentration]);
        feed(self.diurnal_offset_k.data());
        feed(self.insolation.toa_w_m2.data());
        feed(
            &self
                .weather
                .wind
                .data()
                .iter()
                .flat_map(|w| [w.u_east, w.v_north])
                .collect::<Vec<_>>(),
        );
        feed(self.weather.moisture.data());
        feed(self.weather.precipitation.data());
        feed(
            &self
                .ocean
                .columns
                .data()
                .iter()
                .flat_map(|c| [c.depth, c.surface_temp, c.mean_temp, c.salinity, c.density])
                .collect::<Vec<_>>(),
        );
        feed(
            &self
                .ocean
                .currents
                .data()
                .iter()
                .flat_map(|c| [c.u_east, c.v_north])
                .collect::<Vec<_>>(),
        );
        feed(self.tides.potential.data());
        feed(&self.tides.equilibrium_height_m);
        feed(
            &self
                .hydrology
                .soil_water
                .data()
                .iter()
                .map(|s| s.storage_mm)
                .collect::<Vec<_>>(),
        );
        feed(self.hydrology.surface_water.data());
        feed(self.hydrology.runoff.data());
        feed(&[
            self.hydrology.budget.precipitation_kg,
            self.hydrology.budget.surface_to_ocean_kg,
            self.sim_time_seconds,
            self.river_water_to_ocean_kg,
        ]);
        let json = |v: &dyn erased::Json| v.json();
        h.update(json(&self.synoptic).as_bytes());
        h.update(json(&self.zonal_background).as_bytes());
        h.update(json(&self.climatology).as_bytes());
        *h.finalize().as_bytes()
    }
}

/// Serialising to JSON through one trait object keeps `state_hash` free of
/// generic noise.
mod erased {
    pub trait Json {
        fn json(&self) -> String;
    }
    impl<T: serde::Serialize> Json for T {
        fn json(&self) -> String {
            serde_json::to_string(self).expect("state serialises")
        }
    }
}
