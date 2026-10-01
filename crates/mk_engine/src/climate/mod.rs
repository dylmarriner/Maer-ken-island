//! CLIMATE MODULE — PHASE 2
//!
//! Purpose
//! - Temperature gradients and seasonal cycles for Marr'Kena MK-I
//! - Energy balance: insolation + geothermal heat + CO2 forcing →
//!   temperature field
//! - Deterministic climate state based on orbital/rotational phase
//!
//! Model (a zonal energy-balance model, per grid cell):
//! 1. **Insolation**: daily-mean top-of-atmosphere flux at each latitude
//!    for the current solar declination, `Q = (S/π)(h₀ sinφ sinδ +
//!    cosφ cosδ sin h₀)` with `cos h₀ = −tanφ tanδ`. This gives polar day,
//!    polar night and real seasons from the orbit and axial tilt.
//! 2. **Greenhouse**: a single grey atmospheric layer of longwave
//!    absorptivity [`GREY_ATMOSPHERE_ABSORPTIVITY`] (calibrated so an
//!    Earth-like planet averages 288 K over a year in this model), plus logarithmic CO2 radiative
//!    forcing `5.35 ln(C/280)` W/m².
//! 3. **Meridional transport**: Budyko-style relaxation of each cell's
//!    radiative-equilibrium temperature toward the area-weighted global
//!    mean by [`MERIDIONAL_TRANSPORT_FRACTION`].
//! 4. **Thermal inertia**: each cell relaxes toward its equilibrium over
//!    `τ = C/λ`, with ocean mixed-layer or land heat capacity `C` and the
//!    linearised radiative feedback `λ`, so oceans damp seasons and land
//!    responds within weeks.
//!
//! Diurnal (day/night) variation is not resolved in temperature — the
//! daily-mean flux is used — while instantaneous daylight is available to
//! perception from the insolation field itself.
//!
//! Invariants
//! - All arithmetic uses f64 for continuous quantities
//! - Temperature field computed per latitude band
//! - Seasonal variation driven by orbital phase and axial tilt
//! - Same tick + inputs → same climate field (deterministic)
//! - Ledger-tracked: InsolationEnergy → SurfaceHeat
//!
//! Authority
//! - MAERKEN_PHASE_2_PHYSICAL_WORLD.md § 3.5
//! - PLANET_CONSTANTS.md § 7 (climate parameters)
//! - NATURE_CONSTANTS.md (thermal properties)

use mk_core::grid::Grid2;
use serde::{Deserialize, Serialize};

/// Climate state for a given tick
///
/// Type: Stepped state (each tick relaxes from the previous field)
/// Determinism: same previous state + forcing + dt → same climate field
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClimateState {
    /// Surface temperature (K) per grid cell
    pub surface_temperature: Grid2<f64>,
    /// Atmospheric temperature (K) per grid cell
    pub atmos_temperature: Grid2<f64>,
    /// Subsurface temperature (K) per grid cell
    pub subsurface_temperature: Grid2<f64>,
    /// Atmospheric CO2 concentration (ppm), stepped by the carbon cycle
    /// (volcanic outgassing against silicate weathering). It is carried from
    /// step to step, so an operator's scenario value persists and relaxes
    /// only on the carbon cycle's own timescale.
    pub co2_concentration: f64,
    /// Real absorbed radiative power this tick (W/m², summed over the
    /// grid): solar flux (latitude/albedo-adjusted) plus tidal heating,
    /// the same quantity `equilibrium_temperature` balances against.
    ///
    /// `#[serde(default)]` on this and `emitted_flux_w_m2`: added after
    /// some `WorldState` snapshots may already exist on disk without these
    /// fields — see the same note on
    /// `humans::technology::TechnologySnapshot::accumulated_knowledge`.
    #[serde(default)]
    pub absorbed_flux_w_m2: f64,
    /// Real emitted radiative power this tick (W/m², summed over the
    /// grid): Stefan-Boltzmann emission at the *final*, seasonally-
    /// adjusted temperature. At exact equilibrium this equals
    /// `absorbed_flux_w_m2`; the seasonal adjustment perturbs surface
    /// temperature away from that balance, so the gap between the two is
    /// a genuine (small) net radiative imbalance for the tick, not an
    /// invented number.
    #[serde(default)]
    pub emitted_flux_w_m2: f64,
    /// Area-weighted planetary mean absorbed flux this tick (W/m²). Unlike
    /// the per-cell sums above, polar cells count only for the small area
    /// they cover. It sets the energy-limited global evaporation rate.
    #[serde(default)]
    pub mean_absorbed_flux_w_m2: f64,
    /// Volcanic outgassing when this world's carbon cycle began (mol/yr):
    /// the level that corresponds to Earth's present outgassing rate.
    #[serde(default)]
    pub reference_volcanic_co2_mol_yr: f64,
    /// Free atmospheric O₂, kg. `None` until the world sets it from the
    /// canon composition (`mk_engine::conservation::canon_atmospheric_o2_kg`);
    /// then photosynthesis and decomposition move it with the carbon they
    /// exchange.
    #[serde(default)]
    pub atmospheric_o2_kg: Option<f64>,
}

impl ClimateState {
    /// Create new climate state with initial temperatures and CO2
    pub fn new(grid_spec: &mk_core::grid::GridSpec, equilibrium_temp: f64) -> Self {
        ClimateState {
            surface_temperature: Grid2::new(grid_spec, equilibrium_temp),
            atmos_temperature: Grid2::new(grid_spec, equilibrium_temp * 0.9),
            subsurface_temperature: Grid2::new(grid_spec, equilibrium_temp * 1.1),
            co2_concentration: 280.0,
            // No forcing inputs at construction time (this is an initial/
            // default state, not a derived tick), so there is no real
            // absorbed/emitted flux to report yet.
            absorbed_flux_w_m2: 0.0,
            mean_absorbed_flux_w_m2: 0.0,
            emitted_flux_w_m2: 0.0,
            reference_volcanic_co2_mol_yr: 0.0,
            atmospheric_o2_kg: None,
        }
    }

    /// Minimal constructor using default equilibrium temp and grid spec
    pub fn default_for_grid(grid_spec: &mk_core::grid::GridSpec) -> Self {
        ClimateState::new(grid_spec, 280.0)
    }

    /// Get global average surface temperature
    /// Area-weighted global mean surface temperature (K). Every row of a
    /// lat-lon grid has the same number of cells but polar rows cover far
    /// less of the sphere, so cells are weighted by cos(latitude).
    pub fn global_temperature(&self) -> f64 {
        let grid = &self.surface_temperature;
        let spec = mk_core::grid::GridSpec::new(grid.nlat(), grid.nlon());
        let (mut sum, mut weight) = (0.0, 0.0);
        for row in 0..grid.nlat() {
            let w = spec.lat_rad(row).cos().max(0.0);
            for col in 0..grid.nlon() {
                sum += grid.get(row, col) * w;
                weight += w;
            }
        }
        if weight > 0.0 {
            sum / weight
        } else {
            grid.average()
        }
    }

    /// Get global CO2 concentration
    pub fn co2_concentration(&self) -> f64 {
        self.co2_concentration
    }
}

const SIGMA: f64 = 5.670373e-8;
/// Longwave absorptivity of the single grey atmospheric layer, calibrated so
/// an Earth-like planet (1361 W/m², albedo 0.3, 23.44° tilt, 70% ocean)
/// averages 288 K over a year in *this* model. The uniform-temperature
/// estimate `T⁴ = (1−α)S/4 / (σ(1 − ε/2))` gives 0.772, but the model has an
/// equator-to-pole gradient and seasons, and because emission goes as T⁴ that
/// structure lowers the mean by ~5.5 K. The calibration is locked by
/// `an_earth_like_planet_averages_288_k_over_a_year`.
pub const GREY_ATMOSPHERE_ABSORPTIVITY: f64 = 0.869;
/// Pre-industrial reference CO2 concentration for radiative forcing (ppm).
pub const REFERENCE_CO2_PPM: f64 = 280.0;
/// Budyko-style share of each cell's departure from the global-mean
/// radiative temperature removed by atmospheric/oceanic heat transport.
pub const MERIDIONAL_TRANSPORT_FRACTION: f64 = 0.4;
/// Areal heat capacity of a ~50 m ocean mixed layer (J m⁻² K⁻¹).
const OCEAN_HEAT_CAPACITY: f64 = 2.1e8;
/// Areal heat capacity of the thermally active land surface layer
/// (J m⁻² K⁻¹).
const LAND_HEAT_CAPACITY: f64 = 3.0e6;
/// Areal heat capacity (J m⁻² K⁻¹) of the surface layer at a cell of the
/// given elevation: the ocean mixed layer at or below sea level, the
/// thermally active land layer above it. Energy added to a cell warms it by
/// `joules / (area × this)`.
pub fn surface_heat_capacity_j_m2_k(elevation_m: f64) -> f64 {
    if elevation_m <= 0.0 {
        OCEAN_HEAT_CAPACITY
    } else {
        LAND_HEAT_CAPACITY
    }
}

/// Relaxation time of the subsurface toward the surface temperature.
const SUBSURFACE_RELAXATION_SECONDS: f64 = 5.0 * 365.25 * 86_400.0;
/// Depth and conductivity of the reported subsurface layer, for the
/// geothermal gradient added on top of its relaxed temperature.
const SUBSURFACE_DEPTH_M: f64 = 10.0;
const ROCK_CONDUCTIVITY_W_M_K: f64 = 2.5;

/// Number of orbital-phase bins in a [`Climatology`].
pub const CLIMATOLOGY_PHASE_BINS: usize = 12;

/// Annual-mean climate per cell: surface temperature (K) and precipitation
/// (mm/day) over the most recent full orbit.
///
/// The orbit is split into [`CLIMATOLOGY_PHASE_BINS`] phase bins. Each bin
/// holds the time-mean over its latest visit, and the annual mean is the
/// average of the bins. Every season therefore counts once, the annual cycle
/// cancels exactly, and biomes classified from it follow the climate rather
/// than the season.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Climatology {
    /// Annual-mean surface temperature per cell (K), row-major.
    pub temperature_k: Vec<f64>,
    /// Annual-mean precipitation per cell (mm/day), row-major.
    pub precipitation_mm_day: Vec<f64>,
    #[serde(default)]
    bin_temperature_k: Vec<Vec<f64>>,
    #[serde(default)]
    bin_precipitation_mm_day: Vec<Vec<f64>>,
    #[serde(default)]
    current_bin: usize,
    /// Seconds already averaged into the current bin on this visit.
    #[serde(default)]
    current_bin_seconds: f64,
}

impl Climatology {
    /// Fold `dt_seconds` of the current state, taken at `orbital_phase`
    /// (`0..1` of the orbit), into its phase bin. An empty climatology (a new
    /// world, or a snapshot saved before bins existed) starts every bin from
    /// the current state.
    pub fn update(
        &mut self,
        temperature_k: &Grid2<f64>,
        precipitation_mm_day: &Grid2<f64>,
        dt_seconds: f64,
        orbital_phase: f64,
    ) {
        let cells = temperature_k.data().len();
        let phase = if orbital_phase.is_finite() {
            orbital_phase.rem_euclid(1.0)
        } else {
            0.0
        };
        let bin =
            ((phase * CLIMATOLOGY_PHASE_BINS as f64) as usize).min(CLIMATOLOGY_PHASE_BINS - 1);
        let ready = self.bin_temperature_k.len() == CLIMATOLOGY_PHASE_BINS
            && self.bin_temperature_k.iter().all(|b| b.len() == cells)
            && self.bin_precipitation_mm_day.len() == CLIMATOLOGY_PHASE_BINS
            && self
                .bin_precipitation_mm_day
                .iter()
                .all(|b| b.len() == cells);
        if !ready {
            self.bin_temperature_k = vec![temperature_k.data().to_vec(); CLIMATOLOGY_PHASE_BINS];
            self.bin_precipitation_mm_day =
                vec![precipitation_mm_day.data().to_vec(); CLIMATOLOGY_PHASE_BINS];
            self.current_bin = bin;
            self.current_bin_seconds = 0.0;
            self.recompute_means();
            return;
        }
        if bin != self.current_bin {
            // A new visit to this phase replaces what the bin held a year ago.
            self.current_bin = bin;
            self.current_bin_seconds = 0.0;
        }
        let dt = if dt_seconds.is_finite() {
            dt_seconds.max(0.0)
        } else {
            0.0
        };
        let total = self.current_bin_seconds + dt;
        let keep = if total > 0.0 {
            self.current_bin_seconds / total
        } else {
            0.0
        };
        for (mean, now) in self.bin_temperature_k[bin]
            .iter_mut()
            .zip(temperature_k.data())
        {
            *mean = *mean * keep + now * (1.0 - keep);
        }
        for (mean, now) in self.bin_precipitation_mm_day[bin]
            .iter_mut()
            .zip(precipitation_mm_day.data())
        {
            *mean = *mean * keep + now * (1.0 - keep);
        }
        self.current_bin_seconds = total;
        self.recompute_means();
    }

    fn recompute_means(&mut self) {
        let mean_of = |bins: &[Vec<f64>]| -> Vec<f64> {
            let cells = bins.first().map_or(0, Vec::len);
            (0..cells)
                .map(|i| bins.iter().map(|b| b[i]).sum::<f64>() / bins.len() as f64)
                .collect()
        };
        self.temperature_k = mean_of(&self.bin_temperature_k);
        self.precipitation_mm_day = mean_of(&self.bin_precipitation_mm_day);
    }
}

/// Everything a climate step is forced by.
#[derive(Debug, Clone, Copy)]
pub struct ClimateForcing<'a> {
    /// Solar flux at the planet's current orbital distance (W/m²).
    pub toa_solar_flux_w_m2: f64,
    /// Current subsolar latitude (radians).
    pub solar_declination_rad: f64,
    /// Planetary (Bond) albedo.
    pub albedo: f64,
    /// Mean surface pressure (Pa) and gravity (m/s²), which set the grey
    /// layer's longwave optical depth ([`grey_absorptivity`]).
    pub surface_pressure_pa: f64,
    pub surface_gravity_m_s2: f64,
    /// Geothermal heat flux per cell (W/m²).
    pub geothermal_flux_w_m2: &'a Grid2<f64>,
    /// Planet-wide volcanic CO2 outgassing this step (mol/yr).
    pub volcanic_co2_mol_yr: f64,
    /// Elevation per cell (m); cells at or below 0 are ocean for heat
    /// capacity.
    pub elevation_m: &'a Grid2<f64>,
    /// Simulated seconds since the previous climate state. `f64::INFINITY`
    /// settles straight to equilibrium (used to initialise a world).
    pub dt_seconds: f64,
}

/// Daily-mean top-of-atmosphere insolation (W/m²) at `latitude` for solar
/// declination `declination`, including polar day and polar night.
pub fn daily_mean_insolation(solar_flux: f64, latitude: f64, declination: f64) -> f64 {
    let cos_h0 = (-latitude.tan() * declination.tan()).clamp(-1.0, 1.0);
    let h0 = cos_h0.acos();
    let flux = solar_flux / std::f64::consts::PI
        * (h0 * latitude.sin() * declination.sin() + latitude.cos() * declination.cos() * h0.sin());
    flux.max(0.0)
}

/// Earth's present volcanic CO2 outgassing expressed as atmospheric
/// concentration change (ppm/yr; ~0.1 GtC/yr over ~2.1 GtC/ppm).
const EARTH_VOLCANIC_CO2_PPM_PER_YEAR: f64 = 0.047;
/// Silicate-weathering temperature sensitivity (per K): weathering
/// roughly doubles for every 10 K of warming.
const WEATHERING_TEMPERATURE_SENSITIVITY: f64 = 0.07;
/// Surface temperature at which weathering balances Earth's outgassing at
/// the reference CO2 concentration (K).
const WEATHERING_REFERENCE_TEMPERATURE_K: f64 = 288.0;
/// How many times larger the ocean-buffered exchangeable carbon pool is
/// than the atmosphere's alone; it stretches the carbon cycle's response
/// to its real ~10⁵-year timescale.
const OCEAN_CARBON_BUFFER_FACTOR: f64 = 50.0;
const SECONDS_PER_YEAR: f64 = 365.25 * 86_400.0;

/// Step atmospheric CO2 (ppm) over `dt_seconds`: volcanic outgassing
/// (Earth's present rate scaled by this world's outgassing relative to its
/// reference) minus silicate weathering, which rises with CO2 and with
/// temperature — the long-term carbon-cycle thermostat.
fn step_co2(
    co2_ppm: f64,
    mean_temperature_k: f64,
    volcanic_co2_mol_yr: f64,
    reference_volcanic_co2_mol_yr: f64,
    dt_seconds: f64,
) -> f64 {
    if !dt_seconds.is_finite() || dt_seconds <= 0.0 || reference_volcanic_co2_mol_yr <= 0.0 {
        return co2_ppm;
    }
    let source = EARTH_VOLCANIC_CO2_PPM_PER_YEAR
        * (volcanic_co2_mol_yr / reference_volcanic_co2_mol_yr).max(0.0);
    let weathering_rate = EARTH_VOLCANIC_CO2_PPM_PER_YEAR / REFERENCE_CO2_PPM
        * (WEATHERING_TEMPERATURE_SENSITIVITY
            * (mean_temperature_k - WEATHERING_REFERENCE_TEMPERATURE_K))
            .exp();
    let equilibrium = source / weathering_rate;
    let dt_years = dt_seconds / SECONDS_PER_YEAR;
    let decay = (-weathering_rate * dt_years / OCEAN_CARBON_BUFFER_FACTOR).exp();
    (equilibrium + (co2_ppm - equilibrium) * decay).max(1.0)
}

/// CO2 radiative forcing relative to [`REFERENCE_CO2_PPM`] (W/m²).
pub fn co2_forcing_w_m2(co2_ppm: f64) -> f64 {
    5.35 * (co2_ppm.max(1.0) / REFERENCE_CO2_PPM).ln()
}

/// Earth's mean surface pressure (Pa) and gravity (m/s²): the reference
/// atmosphere [`GREY_ATMOSPHERE_ABSORPTIVITY`] is calibrated for.
pub const EARTH_SURFACE_PRESSURE_PA: f64 = 101_325.0;
pub const EARTH_SURFACE_GRAVITY_M_S2: f64 = 9.806_65;

/// Longwave absorptivity of the grey layer for an atmosphere of surface
/// pressure `p` and gravity `g`. The layer's optical depth `τ = −ln(1 − ε)`
/// scales with the absorbing column mass `p/g` times pressure broadening of
/// the absorption lines (∝ `p`), so `τ = τ_⊕ · (p/g)/(p_⊕/g_⊕) · p/p_⊕`,
/// with `τ_⊕` from [`GREY_ATMOSPHERE_ABSORPTIVITY`].
pub fn grey_absorptivity(surface_pressure_pa: f64, surface_gravity_m_s2: f64) -> f64 {
    let positive = |x: f64| x.is_finite() && x > 0.0;
    if !positive(surface_pressure_pa) || !positive(surface_gravity_m_s2) {
        return GREY_ATMOSPHERE_ABSORPTIVITY;
    }
    let tau_earth = -(1.0 - GREY_ATMOSPHERE_ABSORPTIVITY).ln();
    let pressure_ratio = surface_pressure_pa / EARTH_SURFACE_PRESSURE_PA;
    let column_ratio = pressure_ratio * EARTH_SURFACE_GRAVITY_M_S2 / surface_gravity_m_s2;
    let tau = tau_earth * column_ratio * pressure_ratio;
    1.0 - (-tau).exp()
}

/// Effective surface emission coefficient under a grey layer of absorptivity
/// `ε`: outgoing longwave = `σ (1 − ε/2) T⁴`.
fn effective_emission_coefficient(absorptivity: f64) -> f64 {
    SIGMA * (1.0 - absorptivity.clamp(0.0, 1.0) / 2.0)
}

/// Seasonal temperature adjustment based on orbital phase
///
/// orbital_phase: [0, 1] where 0 = perihelion, 0.5 = aphelion
/// Returns temperature adjustment in Kelvin
fn seasonal_adjustment(orbital_phase: f64, axial_tilt_rad: f64, latitude: f64) -> f64 {
    // Declination depends on orbital phase
    let declination = axial_tilt_rad * (2.0 * std::f64::consts::PI * orbital_phase).cos();

    // Seasonal forcing from declination/latitude alignment
    let seasonal_forcing = latitude.tan() * declination.tan();

    // Limit and scale for stability, with polar exaggeration
    let adjustment = seasonal_forcing.clamp(-0.2, 0.2) * 50.0;
    adjustment * latitude.abs().sin()
}

/// Peak seasonal temperature swing at `latitude_rad` for a given
/// `axial_tilt_rad`, in the same units as the temperature field (a delta;
/// kelvin and Celsius steps are equal).
///
/// This is [`seasonal_adjustment`] evaluated at the solstice (`orbital_phase`
/// `0.0`, where the solar declination reaches its maximum `axial_tilt_rad`)
/// — the amplitude of the yearly cycle rather than its value at one instant.
/// Callers use it to judge how *seasonal* a place is, not how warm it is
/// right now.
pub fn seasonal_temperature_amplitude(axial_tilt_rad: f64, latitude_rad: f64) -> f64 {
    seasonal_adjustment(0.0, axial_tilt_rad, latitude_rad).abs()
}

/// Step climate forward from `previous` under `forcing`.
pub fn step_climate(
    previous: &ClimateState,
    forcing: &ClimateForcing<'_>,
    grid_spec: &mk_core::grid::GridSpec,
) -> ClimateState {
    let (nlat, nlon) = (grid_spec.nlat, grid_spec.nlon);
    let reference_volcanic_co2_mol_yr = if previous.reference_volcanic_co2_mol_yr > 0.0 {
        previous.reference_volcanic_co2_mol_yr
    } else {
        forcing.volcanic_co2_mol_yr.max(0.0)
    };
    let co2 = step_co2(
        previous.co2_concentration,
        previous.global_temperature(),
        forcing.volcanic_co2_mol_yr,
        reference_volcanic_co2_mol_yr,
        forcing.dt_seconds,
    );
    let co2_forcing = co2_forcing_w_m2(co2);
    let emission = effective_emission_coefficient(grey_absorptivity(
        forcing.surface_pressure_pa,
        forcing.surface_gravity_m_s2,
    ));
    let albedo = forcing.albedo.clamp(0.0, 1.0);

    // Radiative-equilibrium temperature and absorbed flux per cell.
    let mut absorbed = vec![0.0; nlat * nlon];
    let mut radiative = vec![0.0; nlat * nlon];
    let (mut weighted_sum, mut weight_total) = (0.0, 0.0);
    let mut weighted_absorbed = 0.0;
    for row in 0..nlat {
        let latitude = grid_spec.lat_rad(row);
        let insolation = daily_mean_insolation(
            forcing.toa_solar_flux_w_m2,
            latitude,
            forcing.solar_declination_rad,
        );
        let area_weight = latitude.cos().max(0.0);
        for col in 0..nlon {
            let idx = row * nlon + col;
            let geothermal = cell(forcing.geothermal_flux_w_m2, row, col).max(0.0);
            let flux = ((1.0 - albedo) * insolation + geothermal + co2_forcing).max(0.0);
            absorbed[idx] = flux;
            radiative[idx] = (flux / emission).powf(0.25);
            weighted_sum += radiative[idx] * area_weight;
            weighted_absorbed += flux * area_weight;
            weight_total += area_weight;
        }
    }
    let global_mean = if weight_total > 0.0 {
        weighted_sum / weight_total
    } else {
        0.0
    };
    let mean_absorbed_flux_w_m2 = if weight_total > 0.0 {
        weighted_absorbed / weight_total
    } else {
        0.0
    };

    let mut surface = vec![0.0; nlat * nlon];
    let mut atmos = vec![0.0; nlat * nlon];
    let mut subsurface = vec![0.0; nlat * nlon];
    let (mut absorbed_total, mut emitted_total) = (0.0, 0.0);
    let subsurface_keep = (-forcing.dt_seconds.max(0.0) / SUBSURFACE_RELAXATION_SECONDS).exp();
    for row in 0..nlat {
        for col in 0..nlon {
            let idx = row * nlon + col;
            let equilibrium = (1.0 - MERIDIONAL_TRANSPORT_FRACTION) * radiative[idx]
                + MERIDIONAL_TRANSPORT_FRACTION * global_mean;
            let heat_capacity = surface_heat_capacity_j_m2_k(cell(forcing.elevation_m, row, col));
            // Linearised radiative restoring strength at this temperature.
            let feedback = (4.0 * emission * equilibrium.max(1.0).powi(3)).max(1e-9);
            let keep = (-forcing.dt_seconds.max(0.0) * feedback / heat_capacity).exp();
            let prior = cell(&previous.surface_temperature, row, col);
            let temperature = equilibrium + (prior - equilibrium) * keep;
            surface[idx] = temperature;
            // A grey layer in radiative equilibrium sits at T_s / 2^(1/4).
            atmos[idx] = temperature * 0.5_f64.powf(0.25);
            let prior_deep = cell(&previous.subsurface_temperature, row, col);
            // The deep layer relaxes toward the surface temperature plus the
            // conductive geothermal gradient over its depth, so its target
            // and its approach to it are both independent of step length.
            let geothermal_gradient = cell(forcing.geothermal_flux_w_m2, row, col).max(0.0)
                * SUBSURFACE_DEPTH_M
                / ROCK_CONDUCTIVITY_W_M_K;
            let deep_target = temperature + geothermal_gradient;
            subsurface[idx] = deep_target + (prior_deep - deep_target) * subsurface_keep;
            absorbed_total += absorbed[idx];
            emitted_total += emission * temperature.powi(4);
        }
    }

    ClimateState {
        surface_temperature: Grid2::from_data(grid_spec, surface),
        atmos_temperature: Grid2::from_data(grid_spec, atmos),
        subsurface_temperature: Grid2::from_data(grid_spec, subsurface),
        co2_concentration: co2,
        absorbed_flux_w_m2: absorbed_total,
        mean_absorbed_flux_w_m2,
        emitted_flux_w_m2: emitted_total,
        reference_volcanic_co2_mol_yr,
        atmospheric_o2_kg: previous.atmospheric_o2_kg,
    }
}

/// Read a grid cell, treating a mis-sized grid as contributing nothing
/// rather than panicking.
fn cell(grid: &Grid2<f64>, row: usize, col: usize) -> f64 {
    if row < grid.nlat() && col < grid.nlon() {
        *grid.get(row, col)
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EARTH_SOLAR: f64 = 1361.0;

    fn grids(grid_spec: &mk_core::grid::GridSpec, elevation: f64) -> (Grid2<f64>, Grid2<f64>) {
        (
            Grid2::new(grid_spec, 0.09),
            Grid2::new(grid_spec, elevation),
        )
    }

    fn settle(
        grid_spec: &mk_core::grid::GridSpec,
        declination: f64,
        elevation: f64,
    ) -> ClimateState {
        let (geo, elev) = grids(grid_spec, elevation);
        let forcing = ClimateForcing {
            toa_solar_flux_w_m2: EARTH_SOLAR,
            solar_declination_rad: declination,
            albedo: 0.3,
            surface_pressure_pa: EARTH_SURFACE_PRESSURE_PA,
            surface_gravity_m_s2: EARTH_SURFACE_GRAVITY_M_S2,
            geothermal_flux_w_m2: &geo,
            volcanic_co2_mol_yr: 1.0e12,
            elevation_m: &elev,
            dt_seconds: f64::INFINITY,
        };
        step_climate(
            &ClimateState::default_for_grid(grid_spec),
            &forcing,
            grid_spec,
        )
    }

    fn area_mean(state: &ClimateState, grid_spec: &mk_core::grid::GridSpec) -> f64 {
        let (mut sum, mut weight) = (0.0, 0.0);
        for row in 0..grid_spec.nlat {
            let w = grid_spec.lat_rad(row).cos();
            for col in 0..grid_spec.nlon {
                sum += state.surface_temperature.get(row, col) * w;
                weight += w;
            }
        }
        sum / weight
    }

    #[test]
    fn equinox_daily_mean_insolation_matches_the_closed_form() {
        // At equinox the daily mean is S·cosφ/π.
        let q = daily_mean_insolation(EARTH_SOLAR, 0.0, 0.0);
        assert!((q - EARTH_SOLAR / std::f64::consts::PI).abs() < 1e-9);
        // Polar night: winter pole receives nothing.
        let tilt = 23.44_f64.to_radians();
        assert_eq!(daily_mean_insolation(EARTH_SOLAR, -1.5, tilt), 0.0);
        // Polar day: summer pole receives more than the equator at solstice.
        assert!(
            daily_mean_insolation(EARTH_SOLAR, 1.5, tilt)
                > daily_mean_insolation(EARTH_SOLAR, 0.0, tilt)
        );
    }

    #[test]
    fn an_earth_like_planet_is_temperate_not_frozen() {
        let grid_spec = mk_core::grid::GridSpec::new(18, 36);
        let state = settle(&grid_spec, 0.0, 100.0);
        let mean = area_mean(&state, &grid_spec);
        assert!((280.0..296.0).contains(&mean), "global mean {mean} K");
        let equator = *state.surface_temperature.get(9, 0);
        let pole = *state.surface_temperature.get(0, 0);
        assert!(
            equator > 295.0 && pole < 265.0,
            "equator {equator} pole {pole}"
        );
    }

    #[test]
    fn seasons_follow_the_declination() {
        let grid_spec = mk_core::grid::GridSpec::new(18, 36);
        let tilt = 23.44_f64.to_radians();
        let north_summer = settle(&grid_spec, tilt, 100.0);
        let north_winter = settle(&grid_spec, -tilt, 100.0);
        let north_row = 15;
        assert!(
            north_summer.surface_temperature.get(north_row, 0)
                > north_winter.surface_temperature.get(north_row, 0)
        );
    }

    #[test]
    fn oceans_respond_more_slowly_than_land() {
        let grid_spec = mk_core::grid::GridSpec::new(4, 8);
        let (geo, land) = grids(&grid_spec, 100.0);
        let ocean = Grid2::new(&grid_spec, -1000.0);
        let cold = ClimateState::new(&grid_spec, 250.0);
        let step = |elevation: &Grid2<f64>| {
            let forcing = ClimateForcing {
                toa_solar_flux_w_m2: EARTH_SOLAR,
                solar_declination_rad: 0.0,
                albedo: 0.3,
                surface_pressure_pa: EARTH_SURFACE_PRESSURE_PA,
                surface_gravity_m_s2: EARTH_SURFACE_GRAVITY_M_S2,
                geothermal_flux_w_m2: &geo,
                volcanic_co2_mol_yr: 1.0e12,
                elevation_m: elevation,
                dt_seconds: 10.0 * 86_400.0,
            };
            *step_climate(&cold, &forcing, &grid_spec)
                .surface_temperature
                .get(2, 0)
        };
        let land_warming = step(&land) - 250.0;
        let ocean_warming = step(&ocean) - 250.0;
        assert!(land_warming > ocean_warming && ocean_warming > 0.0);
    }

    #[test]
    fn more_co2_warms_the_surface() {
        assert!(co2_forcing_w_m2(560.0) > 3.5);
        assert_eq!(co2_forcing_w_m2(REFERENCE_CO2_PPM), 0.0);
    }

    #[test]
    fn climate_is_deterministic_and_reports_real_fluxes() {
        let grid_spec = mk_core::grid::GridSpec::new(4, 8);
        let a = settle(&grid_spec, 0.1, 100.0);
        let b = settle(&grid_spec, 0.1, 100.0);
        assert_eq!(a.surface_temperature.data(), b.surface_temperature.data());
        assert!(a.absorbed_flux_w_m2 > 0.0);
        // Settled straight to equilibrium: emission balances absorption
        // except for the transported share.
        assert!(a.emitted_flux_w_m2 > 0.0);
    }

    #[test]
    fn weathering_thermostat_balances_outgassing() {
        // At the reference temperature and outgassing, 280 ppm is steady.
        let steady = step_co2(280.0, 288.0, 1.0, 1.0, 1.0e6 * SECONDS_PER_YEAR);
        assert!((steady - 280.0).abs() < 1e-6);
        // Doubling outgassing raises CO2; a hotter world weathers it down.
        let more_volcanism = step_co2(280.0, 288.0, 2.0, 1.0, 1.0e7 * SECONDS_PER_YEAR);
        assert!(more_volcanism > 500.0);
        let hotter = step_co2(280.0, 298.0, 1.0, 1.0, 1.0e7 * SECONDS_PER_YEAR);
        assert!(hotter < 280.0);
        // Nothing happens within a human lifetime.
        let decades = step_co2(280.0, 288.0, 2.0, 1.0, 50.0 * SECONDS_PER_YEAR);
        assert!((decades - 280.0).abs() < 0.5);
    }

    #[test]
    fn climatology_is_an_exact_annual_mean() {
        let spec = mk_core::grid::GridSpec::new(1, 1);
        let precipitation = Grid2::new(&spec, 1.0);
        let mut climatology = Climatology::default();
        let season = |phase: f64| 280.0 + 20.0 * (std::f64::consts::TAU * phase).sin();
        let steps = 120;
        let dt = 3.0 * 86_400.0;
        for year in 0..3 {
            for step in 0..steps {
                let phase = step as f64 / steps as f64;
                climatology.update(&Grid2::new(&spec, season(phase)), &precipitation, dt, phase);
                if year >= 1 {
                    // Every point of the year sees the same annual mean.
                    assert!(
                        (climatology.temperature_k[0] - 280.0).abs() < 2.0,
                        "year {year} phase {phase}: {}",
                        climatology.temperature_k[0]
                    );
                }
            }
        }
        assert_eq!(climatology.precipitation_mm_day, vec![1.0]);
    }

    #[test]
    fn global_temperature_weights_cells_by_area() {
        let spec = mk_core::grid::GridSpec::new(4, 2);
        let mut state = ClimateState::new(&spec, 250.0);
        // Warm equatorial rows, frigid polar rows.
        for col in 0..2 {
            *state.surface_temperature.get_mut(0, col) = 200.0;
            *state.surface_temperature.get_mut(1, col) = 300.0;
            *state.surface_temperature.get_mut(2, col) = 300.0;
            *state.surface_temperature.get_mut(3, col) = 200.0;
        }
        // The unweighted mean is 250 K; the tiny polar rows count for less.
        assert!(state.global_temperature() > 270.0);
    }

    #[test]
    fn an_earth_like_planet_averages_288_k_over_a_year() {
        let spec = mk_core::grid::GridSpec::new(32, 64);
        let geothermal = Grid2::new(&spec, 0.09);
        let mut elevation = Grid2::new(&spec, -3000.0);
        for row in 0..32 {
            for col in 0..64 {
                if (col * 7 + row * 3) % 10 < 3 {
                    *elevation.get_mut(row, col) = 500.0;
                }
            }
        }
        let tilt = 23.44_f64.to_radians();
        let year = 365.25 * 86_400.0;
        let dt = 5.0 * 86_400.0;
        let steps = (year / dt) as usize;
        let mut state = ClimateState::default_for_grid(&spec);
        let (mut sum, mut count, mut t) = (0.0, 0.0, 0.0);
        for year_index in 0..6 {
            for _ in 0..steps {
                t += dt;
                let forcing = ClimateForcing {
                    toa_solar_flux_w_m2: 1361.0,
                    solar_declination_rad: tilt * (std::f64::consts::TAU * t / year).sin(),
                    albedo: 0.3,
                    surface_pressure_pa: EARTH_SURFACE_PRESSURE_PA,
                    surface_gravity_m_s2: EARTH_SURFACE_GRAVITY_M_S2,
                    geothermal_flux_w_m2: &geothermal,
                    volcanic_co2_mol_yr: 1.0e12,
                    elevation_m: &elevation,
                    dt_seconds: dt,
                };
                state = step_climate(&state, &forcing, &spec);
                if year_index == 5 {
                    sum += state.global_temperature();
                    count += 1.0;
                }
            }
        }
        let mean = sum / count;
        assert!((mean - 288.0).abs() < 0.5, "annual mean {mean} K");
    }

    #[test]
    fn subsurface_relaxes_to_the_geothermal_target_at_any_step_length() {
        let spec = mk_core::grid::GridSpec::new(4, 8);
        let geothermal = Grid2::new(&spec, 0.09);
        let elevation = Grid2::new(&spec, 100.0);
        let forcing = |dt: f64| ClimateForcing {
            toa_solar_flux_w_m2: 1361.0,
            solar_declination_rad: 0.0,
            albedo: 0.3,
            surface_pressure_pa: EARTH_SURFACE_PRESSURE_PA,
            surface_gravity_m_s2: EARTH_SURFACE_GRAVITY_M_S2,
            geothermal_flux_w_m2: &geothermal,
            volcanic_co2_mol_yr: 1.0e12,
            elevation_m: &elevation,
            dt_seconds: dt,
        };
        let start = step_climate(
            &ClimateState::default_for_grid(&spec),
            &forcing(f64::INFINITY),
            &spec,
        );
        let gradient = 0.09 * SUBSURFACE_DEPTH_M / ROCK_CONDUCTIVITY_W_M_K;

        // A zero-length step leaves the deep layer where it was.
        let still = step_climate(&start, &forcing(0.0), &spec);
        assert_eq!(
            still.subsurface_temperature.get(1, 0),
            start.subsurface_temperature.get(1, 0)
        );

        // Many short steps and one long one reach the same deep temperature,
        // which sits one gradient above the surface.
        let year = 365.25 * 86_400.0;
        let mut short = start.clone();
        for _ in 0..400 {
            short = step_climate(&short, &forcing(year / 10.0), &spec);
        }
        let long = step_climate(&start, &forcing(40.0 * year), &spec);
        let deep = |s: &ClimateState| *s.subsurface_temperature.get(1, 0);
        let surface = |s: &ClimateState| *s.surface_temperature.get(1, 0);
        assert!(
            (deep(&short) - deep(&long)).abs() < 1e-3,
            "{} vs {}",
            deep(&short),
            deep(&long)
        );
        assert!((deep(&long) - surface(&long) - gradient).abs() < 1e-3);
    }

    #[test]
    fn thicker_heavier_atmospheres_trap_more_longwave() {
        let earth = grey_absorptivity(EARTH_SURFACE_PRESSURE_PA, EARTH_SURFACE_GRAVITY_M_S2);
        assert!((earth - GREY_ATMOSPHERE_ABSORPTIVITY).abs() < 1e-12);
        // Marr'Ken: 1.82 bar under 19.62 m/s² → τ about 1.6× Earth's.
        let marrkena = grey_absorptivity(182_000.0, 19.62);
        assert!(marrkena > earth && marrkena < 1.0, "{marrkena}");
        let tau_ratio = (1.0 - marrkena).ln() / (1.0 - earth).ln();
        assert!((tau_ratio - 1.613).abs() < 0.01, "{tau_ratio}");
        // Half the pressure, a quarter of the optical depth.
        let thin = grey_absorptivity(EARTH_SURFACE_PRESSURE_PA / 2.0, EARTH_SURFACE_GRAVITY_M_S2);
        assert!(((1.0 - thin).ln() / (1.0 - earth).ln() - 0.25).abs() < 1e-9);
    }
}
