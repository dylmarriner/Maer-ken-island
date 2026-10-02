use blake3;
/// Phase 5 World Integration & Audit Chain
///
/// Implements the immutable 19-step subsystem execution order
/// with comprehensive auditing and hash chain verification
use mk_core::canon::CanonLocked;
use mk_core::hash::HashChain;
use mk_core::rng::RngRegistry;
use mk_core::time::Tick;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::warn;

use crate::agents::{AgentStepAudit, AgentSystem, AgentWorldObservation};

/// Ideal annual mean surface temperature for the founders' estate, in kelvin
/// (~22 °C) — a mild tropical/subtropical highland climate, neither frozen
/// nor scorching.
const ESTATE_IDEAL_TEMP_K: f64 = 295.15;

/// Deterministically choose where the founders' estate is built.
///
/// The design intent is "somewhere with good weather all year round", so
/// this scores every land cell of the freshly-computed world on real physical
/// state rather than picking a spot by hand:
///
/// * `temperature` — closeness to [`ESTATE_IDEAL_TEMP_K`] (mild),
/// * seasonal swing — from [`crate::climate::seasonal_temperature_amplitude`],
///   the same solstice formula the climate model itself uses,
/// * `precipitation` — more rain is better, normalised against the wettest
///   cell on *this* planet so no unit assumptions are baked in,
/// * `elevation` — a mild penalty for high ground (thin air, cold nights),
/// * volcanic heat flux — cells the biome pass already treats as actively
///   volcanic are excluded outright.
///
/// Pure function of published state: the same seed always yields the same
/// site. Ties break on the lowest `(row, col)` in row-major order, so the
/// result is stable. Returns `None` when no cell qualifies (e.g. a tiny
/// all-ocean test grid).
fn choose_estate_location(
    elevation: &mk_core::grid::Grid2<f64>,
    temperature: &mk_core::grid::Grid2<f64>,
    precipitation: &mk_core::grid::Grid2<f64>,
    volcanism: &crate::volcanism::VolcanismState,
    obliquity_rad: f64,
    grid_spec: &mk_core::grid::GridSpec,
) -> Option<(usize, usize)> {
    let max_precip = precipitation.data().iter().copied().fold(0.0_f64, f64::max);

    let mut best: Option<(f64, usize, usize)> = None;
    for row in 0..elevation.nlat() {
        for col in 0..elevation.nlon() {
            let elev = *elevation.get(row, col);
            if elev.is_nan() || elev <= 0.0 {
                continue; // ocean / below sea level
            }

            let volcanic = volcanism.volcanism.get_safe(row, col);
            if volcanic.is_some_and(|v| v.dominates_cell()) {
                continue; // actively volcanic, per the biome pass
            }

            let lat = grid_spec.lat_rad(row);
            let temp_k = *temperature.get(row, col);
            let temp_comfort = 1.0 - ((temp_k - ESTATE_IDEAL_TEMP_K).abs() / 25.0).clamp(0.0, 1.0);

            let swing = crate::climate::seasonal_temperature_amplitude(obliquity_rad, lat);
            let season_comfort = 1.0 - (swing / 15.0).clamp(0.0, 1.0);

            let wet = if max_precip > 0.0 {
                (*precipitation.get(row, col) / max_precip).clamp(0.0, 1.0)
            } else {
                0.0
            };

            let elevation_penalty = (elev / 2000.0).clamp(0.0, 1.0);

            let score =
                0.45 * temp_comfort + 0.35 * season_comfort + 0.20 * wet - 0.25 * elevation_penalty;

            match best {
                Some((best_score, _, _)) if score <= best_score => {}
                _ => best = Some((score, row, col)),
            }
        }
    }

    best.map(|(_, row, col)| (row, col))
}

/// Longest stretch of time a human acts on one decision (years): a day.
const HUMAN_DECISION_PERIOD_YEARS: f64 = 1.0 / 365.25;

/// Most decision periods one world step is split into. Steps up to 64 days
/// (the live and weekly cadences) get one decision per day. Longer steps
/// are coarse runs (the long-horizon verifier steps millennia at a time),
/// where each decision spans `dt / MAX_HUMAN_DECISION_PERIODS`; finer
/// splitting there would multiply cost without making the cadence daily.
const MAX_HUMAN_DECISION_PERIODS: usize = 64;

/// Number of decision periods a world step of `dt_years` holds: one per day,
/// at least 1 and at most [`MAX_HUMAN_DECISION_PERIODS`].
fn human_decision_periods(dt_years: f64) -> usize {
    if !dt_years.is_finite() || dt_years <= 0.0 {
        return 1;
    }
    ((dt_years / HUMAN_DECISION_PERIOD_YEARS).ceil() as usize).clamp(1, MAX_HUMAN_DECISION_PERIODS)
}

/// The RNG registry for decision period `period` (≥ 1) of world tick `tick`:
/// the world seed hashed with the tick and period, so every period draws
/// fresh, reproducible numbers.
fn decision_period_rng(world: &RngRegistry, tick: u64, period: usize) -> RngRegistry {
    let mut hasher = blake3::Hasher::new();
    hasher.update(&world.seed());
    hasher.update(b"human-decision-period");
    hasher.update(&tick.to_le_bytes());
    hasher.update(&(period as u64).to_le_bytes());
    RngRegistry::new(*hasher.finalize().as_bytes())
}

/// Climate steps per orbit in [`spin_up_climate`]: ten simulated days each
/// on the canon year, short against the land's thermal response.
const SPIN_UP_STEPS_PER_ORBIT: usize = 54;

/// Run the climate, weather and hydrology through two orbits from `initial`.
/// Returns the climate state after the second orbit (back at the starting
/// orbital phase), the climatology of that second orbit, and the spun-up
/// soil and surface water.
fn spin_up_climate(
    canon: &CanonLocked,
    initial: crate::climate::ClimateState,
    geothermal_flux_w_m2: &mk_core::grid::Grid2<f64>,
    volcanic_co2_mol_yr: f64,
    elevation: &mk_core::grid::Grid2<f64>,
    coriolis: &mk_core::grid::Grid2<f64>,
    grid_spec: &mk_core::grid::GridSpec,
) -> (
    crate::climate::ClimateState,
    crate::climate::Climatology,
    crate::hydrology::HydrologyState,
) {
    let dt = canon.orbital_period_s / SPIN_UP_STEPS_PER_ORBIT as f64;
    let mut state = initial;
    let mut climatology = crate::climate::Climatology::default();
    // Soils start half full and are filled and drained by the spun-up
    // rainfall, so the world begins with its streams and rivers running.
    let mut hydrology = crate::hydrology::HydrologyState::new(grid_spec, 0.5);
    for orbit in 0..2 {
        for step in 1..=SPIN_UP_STEPS_PER_ORBIT {
            let t = (orbit * SPIN_UP_STEPS_PER_ORBIT + step) as f64 * dt;
            let orbit_state = crate::orbit::step_orbit(canon, t);
            let rotation = crate::rotation::step_rotation(t, orbit_state.mean_anomaly, canon);
            state = crate::climate::step_climate(
                &state,
                &crate::climate::ClimateForcing {
                    toa_solar_flux_w_m2: canon.solar_constant_w_m2
                        / (orbit_state.r_over_a * orbit_state.r_over_a),
                    solar_declination_rad: rotation.subsolar_latitude,
                    albedo: canon.albedo_baseline,
                    surface_pressure_pa: canon.sea_level_pressure_pa,
                    surface_gravity_m_s2: canon.surface_gravity_m_s2,
                    geothermal_flux_w_m2,
                    volcanic_co2_mol_yr,
                    elevation_m: elevation,
                    dt_seconds: dt,
                },
                grid_spec,
            );
            // The first orbit only settles thermal inertia; the second
            // overwrites every phase bin.
            let weather =
                crate::weather::step_weather(canon, 0, &state, coriolis, elevation, grid_spec);
            hydrology = crate::hydrology::step_hydrology(
                canon, &hydrology, &weather, &state, elevation, dt, grid_spec,
            );
            climatology.update(
                &state.surface_temperature,
                &weather.precipitation,
                dt,
                // Midpoint of the interval this step covers.
                (t - 0.5 * dt) / canon.orbital_period_s,
            );
        }
    }
    (state, climatology, hydrology)
}

/// Complete world state for Phase 5
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldState {
    /// Current tick
    pub tick: Tick,

    /// Canon locked parameters
    pub canon: Arc<CanonLocked>,

    /// Derived canon constants
    pub canon_derived: Arc<mk_core::canon::CanonDerived>,

    /// Hash chain for deterministic verification
    pub hash_chain: HashChain,

    /// Random number generator
    pub rng: RngRegistry,

    /// Global flux ledger (cleared every tick)
    pub ledger: mk_core::flux::Ledger,

    /// Global grid specification
    pub grid_spec: mk_core::grid::GridSpec,

    /// Audit trail
    pub audit_trail: AuditTrail,

    /// Audited stocks at the end of the last stock audit, the "before" for
    /// the next one. Not persisted: after a snapshot load the first tick
    /// only takes a fresh measurement.
    #[serde(skip)]
    pub stocks_at_last_audit: Option<mk_core::flux::ReservoirStocks>,

    /// Subsystem states
    pub orbit_state: crate::orbit::OrbitState,
    pub insolation_state: crate::insolation::InsolationField,
    pub planet_state: crate::planet::PlanetFields,
    pub tides_state: crate::tides::TidalState,
    pub climate_state: crate::climate::ClimateState,
    pub weather_state: crate::weather::WeatherState,
    pub hydrology_state: crate::hydrology::HydrologyState,
    pub ocean_state: crate::ocean::OceanState,
    pub tectonics_state: crate::tectonics::TectonicsState,
    pub volcanism_state: crate::volcanism::VolcanismState,
    pub biosphere_state: crate::biosphere::BiosphereSystem,
    pub organisms_state: crate::organisms::OrganismSystem,
    pub vegetation_state: crate::organisms::VegetationSystem,
    pub settlements_state: crate::organisms::SettlementSystem,
    /// Deterministic unowned household property and its named physical storage.
    #[serde(default)]
    pub property_state: crate::organisms::PropertySystem,
    /// Bounded male/female individual pairs for the biodiversity catalogue's
    /// animal-major-group entries (see
    /// `crate::biosphere::catalogue_individuals`). Empty/no-op until
    /// [`WorldState::seed_catalogue_individuals`] is called with a loaded
    /// [`crate::biosphere::BiodiversityCatalogue`] — catalogue loading stays
    /// the caller's concern (mirrors how `mk_ui`/`mk_observatory` already
    /// load it independently for rendering), not something `WorldState`
    /// reaches out to the filesystem for on its own.
    pub catalogue_individuals: crate::biosphere::CatalogueIndividualRegistry,
    pub humans_state: crate::humans::HumanSystem,
    /// Active exogenous disturbances (see [`crate::disturbance`]).
    ///
    /// `#[serde(default)]`: this field was a contentless unit struct in
    /// snapshots written before the disturbance model existed.
    #[serde(default)]
    pub disturbance_state: DisturbanceState,
    /// The single evolution model for live species: logistic growth,
    /// mutation, selection, drift, speciation (pipeline step 13) and
    /// stochastic extinction events (step 14).
    #[serde(default)]
    pub deep_time: crate::biosphere::deep_time_evolution::DeepTimeEvolution,

    /// Biome classification per grid cell
    pub biome_grid: mk_core::grid::Grid2<mk_core::biomes::BiomeType>,
    /// Annual-mean temperature and precipitation the biomes are classified
    /// from.
    #[serde(default)]
    pub climatology: crate::climate::Climatology,

    /// Real web-search/email bridge for `ActionKind::WebSearch`/`SendEmail`.
    /// Deliberately excluded from serialization — see `humans::computer_bridge`
    /// module docs for why this is safe for replay/audit/verify (they never
    /// call [`WorldState::with_computer_bridge`], so this stays `None` there
    /// regardless of what a live `mk serve` run attached it to).
    #[serde(skip)]
    pub computer_bridge: Option<crate::humans::computer_bridge::ComputerBridgeHandle>,

    /// Elevation grid (m) cached from tectonics
    pub elevation_grid: mk_core::grid::Grid2<f64>,

    /// Agent subsystem state (Phase 10)
    pub agents_state: AgentSystem,

    /// Deterministic extraction, processing, crafting, and construction state.
    #[serde(default)]
    pub resource_economy_state: crate::resource_economy::ResourceEconomyState,

    /// Simulated seconds elapsed since the world began: the sum of every
    /// step's actual `dt_seconds`. Orbit, rotation and tides are evaluated
    /// at this time, so the planet's clock follows the real step lengths
    /// rather than `tick × canon.dt_seconds`.
    #[serde(default)]
    pub sim_time_seconds: f64,
    /// Persistent history of notable events and eras (see
    /// [`crate::chronicle`]). Backs `mk_view::TimelineView`.
    #[serde(default)]
    pub chronicle: crate::chronicle::WorldChronicle,

    /// Metrics collection
    pub metrics: WorldMetrics,
}

/// Hydrology spin-up when a world is built: one year of ten-day steps under
/// the initial climate (see [`WorldState::new`]).
const HYDROLOGY_SPIN_UP_STEPS: usize = 36;
const HYDROLOGY_SPIN_UP_STEP_SECONDS: f64 = 10.0 * 86_400.0;

/// Audit trail for deterministic verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditTrail {
    pub entries: Vec<AuditEntry>,
    /// Per-kind net exchange of the interior reservoirs with the boundary
    /// over the last tick (`Ledger::net_imbalance`), in the kind's SI unit.
    /// The names predate this meaning and are kept for snapshot
    /// compatibility; they are not conservation residuals.
    pub energy_closure: f64,
    pub water_closure: f64,
    pub carbon_closure: f64,
    pub oxygen_closure: f64,
    pub nitrogen_closure: f64,
    pub phosphorus_closure: f64,
    pub intelligence_ceiling_status: bool,
    pub hash_chain_continuity: bool,
    pub snapshot_digest_validity: bool,
    /// The last tick's conservation check of measured stocks against the
    /// ledger ([`crate::conservation::audit`]).
    #[serde(default)]
    pub stock_audit: crate::conservation::StockAuditReport,
}

/// Individual audit entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub tick: Tick,
    pub step: StepNumber,
    pub component: String,
    pub status: AuditStatus,
    pub metrics: ComponentMetrics,
    pub hash_before: [u8; 32],
    pub hash_after: [u8; 32],
}

/// Step numbers for immutable order
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StepNumber {
    Orbit = 1,
    Insolation = 2,
    Planet = 3,
    Tides = 4,
    Climate = 5,
    Weather = 6,
    Hydrology = 7,
    Ocean = 8,
    Tectonics = 9,
    Volcanism = 10,
    Biosphere = 11,
    Disturbance = 12,
    Evolution = 13,
    Extinction = 14,
    Agent = 15,
    Humans = 16,
    Audit = 17,
    HashCommit = 18,
    LedgerClear = 19,
    TimeAdvance = 20,
}

/// Audit status
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AuditStatus {
    Success,
    Warning(String),
    Error(String),
    Critical(String),
}

/// Component metrics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ComponentMetrics {
    pub energy_in: f64,
    pub energy_out: f64,
    pub water_in: f64,
    pub water_out: f64,
    pub carbon_in: f64,
    pub carbon_out: f64,
    pub oxygen_in: f64,
    pub oxygen_out: f64,
    pub nitrogen_in: f64,
    pub nitrogen_out: f64,
    pub phosphorus_in: f64,
    pub phosphorus_out: f64,
    pub intelligence_max: f64,
    /// Living individuals the component tracks (agents, humans); zero for
    /// components without a population.
    #[serde(default)]
    pub population: f64,
}

/// World metrics collection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldMetrics {
    /// Conserved-quantity throughput recorded in the ledger during the most
    /// recent tick, per kind (ledger amount units).
    pub total_energy: f64,
    pub total_water: f64,
    pub total_carbon: f64,
    pub total_oxygen: f64,
    pub total_nitrogen: f64,
    pub total_phosphorus: f64,
    pub max_intelligence: f64,
    pub species_count: usize,
    pub total_population: u64,
}

// Real disturbance state lives in `crate::disturbance`; re-exported here
// because `WorldState::disturbance_state` is its long-standing public path.
pub use crate::disturbance::DisturbanceState;

/// Fingerprint of the state a pipeline step produced, chained to the hash
/// the step ran under: `blake3(chain ‖ bincode(state))`. Recorded as the
/// audit entry's `hash_after`, so each entry identifies exactly what that
/// step left behind. A state that cannot be serialized fingerprints as the
/// hash of the chain alone, which `validate_hash_chain` rejects.
fn state_fingerprint<T: Serialize>(chain: &[u8; 32], state: &T) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    hasher.update(chain);
    match bincode::serialize(state) {
        Ok(bytes) => {
            hasher.update(b"state");
            hasher.update(&bytes);
            hasher.finalize().into()
        }
        Err(_) => *chain,
    }
}

/// Global environmental stress (0..1) on the biosphere from the live
/// climate: how far the planet's mean surface temperature sits from a
/// temperate 288 K, saturating at a 30 K departure.
fn climate_environmental_stress(climate: &crate::climate::ClimateState) -> f64 {
    const TEMPERATE_MEAN_K: f64 = 288.0;
    const SATURATING_DEPARTURE_K: f64 = 30.0;
    ((climate.global_temperature() - TEMPERATE_MEAN_K).abs() / SATURATING_DEPARTURE_K)
        .clamp(0.0, 1.0)
}

impl WorldState {
    /// Create new world state
    ///
    /// # Panics
    ///
    /// If `canon` fails `mk_core::canon::validator::validate_canon`: every
    /// subsystem derives its physics from the canon, so a world is never
    /// built on an invalid one.
    pub fn new(canon: Arc<CanonLocked>, initial_seed: [u8; 32]) -> Self {
        if let Err(err) = mk_core::canon::validator::validate_canon(&canon) {
            panic!("WorldState::new: invalid canon: {err}");
        }
        let canon_derived = Arc::new(mk_core::canon::CanonDerived::from(&canon));
        let rng = RngRegistry::new(initial_seed);
        let hash_chain = HashChain::new(initial_seed);

        let grid_spec = mk_core::grid::GridSpec::new(32, 64);
        let tick = 0u64;

        // Initial physical states
        let orbit_state = crate::orbit::step_orbit(&canon, 0.0);
        let rotation = crate::rotation::step_rotation(0.0, orbit_state.mean_anomaly, &canon);
        let planet_state = crate::planet::step_planet(&canon, &canon_derived, &grid_spec);

        let mut initial_ledger = mk_core::flux::Ledger::new();
        let tectonics_state = crate::tectonics::step_tectonics(&canon, tick, &grid_spec);

        let tides_state = crate::tides::step_tides(&canon, 0.0, 0, &grid_spec, &mut initial_ledger);
        let insolation_state = crate::insolation::step_insolation(
            &canon,
            &orbit_state,
            &rotation,
            &grid_spec,
            0.0,
            &mut initial_ledger,
        );

        let volcanism_state = crate::volcanism::step_volcanism(
            &canon,
            tick,
            &tectonics_state.plates,
            &tectonics_state.heat_flow,
            crate::volcanism::INITIAL_DEGASSED_FRACTION,
            0.0,
            &rng,
            &grid_spec,
        );

        // Settle the initial climate straight to equilibrium under the real
        // initial orbit, declination and geothermal flux.
        let initial_geothermal = volcanism_state.heat_flux_w_m2(&grid_spec);
        let initial_elevation = tectonics_state.get_elevation_grid();
        let climate_state = crate::climate::step_climate(
            &crate::climate::ClimateState::default_for_grid(&grid_spec),
            &crate::climate::ClimateForcing {
                toa_solar_flux_w_m2: canon.solar_constant_w_m2
                    / (orbit_state.r_over_a * orbit_state.r_over_a),
                solar_declination_rad: rotation.subsolar_latitude,
                albedo: canon.albedo_baseline,
                surface_pressure_pa: canon.sea_level_pressure_pa,
                surface_gravity_m_s2: canon.surface_gravity_m_s2,
                geothermal_flux_w_m2: &initial_geothermal,
                volcanic_co2_mol_yr: volcanism_state.total_co2_mol_yr(&grid_spec),
                elevation_m: &initial_elevation,
                dt_seconds: f64::INFINITY,
            },
            &grid_spec,
        );
        // Spin the climate through two full orbits: the first settles its
        // thermal inertia into the seasonal cycle, the second is averaged
        // into the annual-mean climatology biomes are classified from. The
        // world then starts from the settled state at the same orbital
        // phase (t = 0).
        let (climate_state, climatology, spun_hydrology) = spin_up_climate(
            &canon,
            climate_state,
            &initial_geothermal,
            volcanism_state.total_co2_mol_yr(&grid_spec),
            &initial_elevation,
            &planet_state.coriolis,
            &grid_spec,
        );
        let weather_state = crate::weather::step_weather(
            &canon,
            tick,
            &climate_state,
            &planet_state.coriolis,
            &initial_elevation,
            &grid_spec,
        );

        let topo = initial_elevation.clone();
        let hydrology_state = crate::hydrology::step_hydrology(
            &canon,
            &spun_hydrology,
            &weather_state,
            &climate_state,
            &topo,
            0.0,
            &grid_spec,
        );

        let ocean_prev = mk_core::grid::Grid2::new(
            &grid_spec,
            crate::ocean::OceanColumn::new(4000.0, 280.0, 35.0),
        );
        let ocean_state = crate::ocean::step_ocean(
            &canon,
            &ocean_prev,
            &crate::ocean::OceanForcing {
                wind: &weather_state.wind,
                climate: &climate_state,
                precipitation_mm_day: &weather_state.precipitation,
                coriolis: &planet_state.coriolis,
                elevation_m: &initial_elevation,
                dt_seconds: 0.0,
            },
            &grid_spec,
        );

        // Initialize biome classification from initial physical state
        let initial_elevation = tectonics_state.get_elevation_grid();
        let mut initial_biomes =
            mk_core::grid::Grid2::new(&grid_spec, mk_core::biomes::BiomeType::TemperateForest);
        let annual_temperature =
            mk_core::grid::Grid2::from_data(&grid_spec, climatology.temperature_k.clone());
        let annual_precipitation =
            mk_core::grid::Grid2::from_data(&grid_spec, climatology.precipitation_mm_day.clone());
        for (row, col, biome) in initial_biomes.indexed_iter_mut() {
            let elev = *initial_elevation.get(row, col);
            let temp = *annual_temperature.get(row, col);
            let precip = *annual_precipitation.get(row, col);
            let moist = hydrology_state.soil_water.get(row, col).moisture_fraction;
            let is_volcanic = {
                if let Some(v) = volcanism_state.volcanism.get_safe(row, col) {
                    v.dominates_cell()
                } else {
                    false
                }
            };
            *biome = mk_core::biomes::classify_biome(elev, temp, precip, moist, is_volcanic);
        }

        // Real, deterministic placement for the founders' estate: the land
        // cell with the best year-round climate — mild mean temperature,
        // small seasonal swing, adequate rainfall, low volcanic hazard —
        // scored from the physical state just computed. Falls back to no
        // placement if the generated planet has no qualifying land cell
        // (e.g. a tiny all-ocean test grid) rather than guessing a location.
        let founders_location = choose_estate_location(
            &initial_elevation,
            &annual_temperature,
            &annual_precipitation,
            &volcanism_state,
            canon.obliquity_deg.to_radians(),
            &grid_spec,
        );

        let resource_economy_state = crate::resource_economy::ResourceEconomyState::from_terrain(
            &initial_biomes,
            &mk_core::grid::Grid2::from_data(
                &grid_spec,
                volcanism_state
                    .volcanism
                    .data()
                    .iter()
                    .map(|cell| cell.volcanic_area_fraction)
                    .collect(),
            ),
        );
        let mut world = Self {
            tick,
            canon: Arc::clone(&canon),
            canon_derived: Arc::clone(&canon_derived),
            hash_chain,
            rng,
            ledger: mk_core::flux::Ledger::new(),
            grid_spec,
            audit_trail: AuditTrail {
                entries: Vec::new(),
                energy_closure: 0.0,
                water_closure: 0.0,
                carbon_closure: 0.0,
                oxygen_closure: 0.0,
                nitrogen_closure: 0.0,
                phosphorus_closure: 0.0,
                intelligence_ceiling_status: true,
                hash_chain_continuity: true,
                snapshot_digest_validity: true,
                stock_audit: crate::conservation::StockAuditReport::default(),
            },
            stocks_at_last_audit: None,
            orbit_state,
            insolation_state,
            planet_state,
            tides_state,
            climate_state,
            weather_state,
            hydrology_state,
            ocean_state,
            tectonics_state,
            volcanism_state,
            biosphere_state: {
                let mut biosphere = crate::biosphere::BiosphereSystem::new(
                    Arc::clone(&canon),
                    u64::from_le_bytes(initial_seed[..8].try_into().expect("8-byte seed prefix")),
                );
                // Initialize with founding species
                if let Err(e) = biosphere.initialize() {
                    warn!("Failed to initialize biosphere: {}", e);
                }
                biosphere
            },
            organisms_state: crate::organisms::OrganismSystem::new(),
            vegetation_state: crate::organisms::VegetationSystem::new(),
            settlements_state: crate::organisms::SettlementSystem::new(),
            property_state: crate::organisms::PropertySystem::new(founders_location),
            catalogue_individuals: crate::biosphere::CatalogueIndividualRegistry::new(),
            humans_state: crate::humans::HumanSystem::new(),
            disturbance_state: DisturbanceState::new(),
            deep_time: crate::biosphere::deep_time_evolution::DeepTimeEvolution::new(),
            biome_grid: initial_biomes,
            climatology,
            computer_bridge: None,
            elevation_grid: initial_elevation,
            agents_state: AgentSystem::new(),
            resource_economy_state,
            sim_time_seconds: 0.0,
            chronicle: crate::chronicle::WorldChronicle::new(),
            metrics: WorldMetrics {
                total_energy: 0.0,
                total_water: 0.0,
                total_carbon: 0.0,
                total_oxygen: 0.0,
                total_nitrogen: 0.0,
                total_phosphorus: 0.0,
                max_intelligence: 0.0,
                species_count: 0,
                total_population: 0,
            },
        };
        world.climate_state.atmospheric_o2_kg =
            Some(crate::conservation::canon_atmospheric_o2_kg(&world.canon));
        world.size_soil_nutrients();
        world.stocks_at_last_audit = Some(crate::conservation::measure(&world));
        world
    }

    /// Enable persistent per-human profiles for this running world. This is
    /// deliberately separate from `new()` so deterministic tests and pure
    /// replay construction remain filesystem-free.
    pub fn enable_persistent_humans(
        &mut self,
        base_path: impl Into<std::path::PathBuf>,
    ) -> Result<(), crate::io::HumanStorageError> {
        let storage = crate::io::HumanStorage::new(base_path);
        self.humans_state = crate::humans::HumanSystem::with_persistent_founders(storage)?;
        // Put the founders where their estate actually was built, rather
        // than at their canonical birthplace coordinates (which remain
        // profile truth for astrology/trait derivation).
        self.humans_state
            .place_founders_at_home(self.property_state.founders_estate_location());
        Ok(())
    }

    /// Attach a real `apps/computer-service` bridge so humans can select
    /// `ActionKind::WebSearch`/`SendEmail`. Only `mk serve` calls this
    /// (gated on `COMPUTER_ACTIONS_ENABLED=1`, see
    /// `humans::computer_bridge::HttpComputerBridge::from_env`) — `run`,
    /// `replay`, `audit`, and `verify` never do, so those paths always see
    /// `computer_bridge: None` regardless of this method's existence.
    pub fn with_computer_bridge(
        mut self,
        bridge: crate::humans::computer_bridge::ComputerBridgeHandle,
    ) -> Self {
        self.computer_bridge = Some(bridge);
        self
    }

    /// Seeds one male + one female founding pair for every animal-major-group
    /// entry in a loaded [`crate::biosphere::BiodiversityCatalogue`].
    /// Idempotent (no-ops if already seeded). The caller is responsible for
    /// loading the catalogue file (see
    /// `crate::biosphere::BiodiversityCatalogue::load_from_jsonl`) — call
    /// this once after `WorldState::new()` to make catalogue-species
    /// reproduction part of the live simulation, e.g. from `mk_cli`'s `run`
    /// command.
    pub fn seed_catalogue_individuals(
        &mut self,
        catalogue: &crate::biosphere::BiodiversityCatalogue,
    ) {
        self.catalogue_individuals.seed_from_catalogue(catalogue);
        // Placement reads this world's climate/terrain, so it runs with the
        // registry temporarily detached from `self`.
        let mut registry = std::mem::take(&mut self.catalogue_individuals);
        registry.place_unplaced(catalogue, self);
        self.catalogue_individuals = registry;
    }

    /// Execute one complete world step following the immutable 18-step order
    pub fn step_world(&mut self, dt_seconds: u64) -> Result<(), WorldStepError> {
        self.audit_trail.entries.clear();
        // A snapshot saved before free O₂ was tracked starts from the canon
        // composition; no stock snapshot survives a load, so the first audit
        // after it only measures.
        if self.climate_state.atmospheric_o2_kg.is_none() {
            self.climate_state.atmospheric_o2_kg =
                Some(crate::conservation::canon_atmospheric_o2_kg(&self.canon));
        }
        self.size_soil_nutrients();

        // Step 1: Orbit
        self.step_orbit(dt_seconds)?;

        // Step 2: Insolation
        self.step_insolation(dt_seconds)?;

        // Step 3: Planet
        self.step_planet(dt_seconds)?;

        // Step 4: Tides
        self.step_tides(dt_seconds)?;

        // Step 5: Climate
        self.step_climate(dt_seconds)?;

        // Step 6: Weather
        self.step_weather(dt_seconds)?;

        // Step 7: Hydrology
        self.step_hydrology(dt_seconds)?;

        // Step 8: Ocean
        self.step_ocean(dt_seconds)?;

        // Step 9: Tectonics
        self.step_tectonics(dt_seconds)?;

        // Step 10: Volcanism
        self.step_volcanism(dt_seconds)?;

        // Step 11: Biosphere
        self.exchanging_biomass_carbon(|world| world.step_biosphere(dt_seconds))?;
        let soil_moisture = self.mean_land_soil_moisture();
        crate::conservation::decompose_detritus(
            &mut self.ledger,
            &mut self.climate_state,
            &mut self.biosphere_state,
            crate::conservation::atmospheric_carbon_kg_per_ppm(&self.canon),
            soil_moisture,
            dt_seconds as f64,
        );
        let npp_kgc_m2_yr = self.biosphere_state.npp_field_kgc_m2_yr.clone();
        crate::conservation::exchange_external_nutrients(
            &mut self.ledger,
            &mut self.biosphere_state,
            &crate::conservation::LandNutrientDrivers {
                spec: &self.grid_spec,
                radius_m: self.canon.planet_radius_m,
                elevation_m: &self.elevation_grid,
                surface_temperature_k: &self.climate_state.surface_temperature,
                runoff_mm_day: &self.hydrology_state.runoff,
                npp_kgc_m2_yr: &npp_kgc_m2_yr,
            },
            dt_seconds as f64,
        );

        // Step 12: Disturbance
        self.exchanging_biomass_carbon(|world| world.step_disturbance(dt_seconds))?;

        // Step 13: Evolution
        self.exchanging_biomass_carbon(|world| world.step_evolution(dt_seconds))?;

        // Step 14: Extinction
        self.exchanging_biomass_carbon(|world| world.step_extinction(dt_seconds))?;

        // Step 15: Agent
        self.step_agents(dt_seconds)?;

        // Step 16: Humans
        self.step_humans(dt_seconds)?;

        // Record this tick's notable events into the world's history.
        self.observe_chronicle();

        // Step 17: Audit
        self.step_audit(dt_seconds)?;

        // Step 18: Hash Commit
        self.step_hash_commit(dt_seconds)?;

        // Step 19: Ledger Clear
        self.step_ledger_clear(dt_seconds)?;

        // Step 20: Time Advance
        self.step_time_advance(dt_seconds)?;

        Ok(())
    }

    /// Area-weighted mean soil moisture fraction over land cells (the
    /// canon optimum when the world has no land).
    fn mean_land_soil_moisture(&self) -> f64 {
        let (mut weighted, mut land) = (0.0, 0.0);
        for row in 0..self.grid_spec.nlat {
            let area = self
                .grid_spec
                .cell_area_at_row_m2(row, self.canon.planet_radius_m)
                .unwrap_or(0.0);
            for col in 0..self.grid_spec.nlon {
                if self
                    .elevation_grid
                    .get_safe(row, col)
                    .copied()
                    .unwrap_or(0.0)
                    > 0.0
                {
                    let moisture = self
                        .hydrology_state
                        .soil_water
                        .get(row, col)
                        .moisture_fraction;
                    weighted += moisture * area;
                    land += area;
                }
            }
        }
        if land > 0.0 {
            weighted / land
        } else {
            crate::conservation::DECOMPOSITION_MOISTURE_OPTIMUM
        }
    }

    /// Size the bioavailable soil nitrogen and phosphorus pools from the
    /// canon, if this world (or the snapshot it was loaded from) has none.
    fn size_soil_nutrients(&mut self) {
        let biosphere = &mut self.biosphere_state;
        if biosphere.soil_nitrogen_kg.is_some() && biosphere.soil_phosphorus_kg.is_some() {
            return;
        }
        let (nitrogen, phosphorus) = crate::conservation::canon_soil_nutrients_kg(
            &self.grid_spec,
            self.canon.planet_radius_m,
            &self.elevation_grid,
        );
        biosphere.soil_nitrogen_kg.get_or_insert(nitrogen);
        biosphere.soil_phosphorus_kg.get_or_insert(phosphorus);
    }

    /// Run a step that can change living biomass, then exchange the carbon
    /// it gained or lost with the atmosphere
    /// ([`crate::conservation::exchange_biomass_carbon`]).
    fn exchanging_biomass_carbon(
        &mut self,
        step: impl FnOnce(&mut Self) -> Result<(), WorldStepError>,
    ) -> Result<(), WorldStepError> {
        let before = crate::conservation::biomass_carbon_by_class_kg(&self.biosphere_state.species);
        let nutrients_before =
            crate::conservation::biomass_nutrients_kg(&self.biosphere_state.species, &self.canon);
        step(self)?;
        let dead = crate::conservation::exchange_biomass_carbon(
            &mut self.ledger,
            &mut self.climate_state,
            &mut self.biosphere_state,
            crate::conservation::atmospheric_carbon_kg_per_ppm(&self.canon),
            before,
        );
        crate::conservation::exchange_biomass_nutrients(
            &mut self.ledger,
            &mut self.biosphere_state,
            &self.canon,
            nutrients_before,
            dead,
        );
        Ok(())
    }

    /// Real per-step conservation flux: sums every ledger entry pushed
    /// strictly after `ledger_start` (the entry count when this step began)
    /// into in/out totals per `FluxKind`. The ledger only clears once per
    /// tick (`step_ledger_clear`), so entries appended between capturing
    /// `ledger_start` and calling this isolate exactly what this one step
    /// contributed — real, ledger-backed data, not a placeholder.
    fn step_ledger_metrics(&self, ledger_start: usize) -> ComponentMetrics {
        use mk_core::flux::FluxKind;
        let mut m = ComponentMetrics::default();
        for entry in &self.ledger.entries()[ledger_start..] {
            let (in_field, out_field) = match entry.kind() {
                FluxKind::Energy => (&mut m.energy_in, &mut m.energy_out),
                FluxKind::Water => (&mut m.water_in, &mut m.water_out),
                FluxKind::Carbon => (&mut m.carbon_in, &mut m.carbon_out),
                FluxKind::Oxygen => (&mut m.oxygen_in, &mut m.oxygen_out),
                FluxKind::Nitrogen => (&mut m.nitrogen_in, &mut m.nitrogen_out),
                FluxKind::Phosphorus => (&mut m.phosphorus_in, &mut m.phosphorus_out),
            };
            // This entry moves `amount` out of `source` and into `sink`.
            // Inflow is what lands in an interior (non-boundary) reservoir,
            // outflow what leaves one: a transfer between interior
            // reservoirs counts on both sides, while energy arriving from
            // insolation or leaving to space counts on one side only, so
            // `in − out` is the change in interior stock.
            if !entry.sink().is_boundary() {
                *in_field += entry.amount();
            }
            if !entry.source().is_boundary() {
                *out_field += entry.amount();
            }
        }
        m
    }

    // Individual step implementations
    fn step_orbit(&mut self, _dt_seconds: u64) -> Result<(), WorldStepError> {
        let hash_before = self.hash_chain.current;

        // Execute orbit step (Phase 2 § 3.1)
        self.orbit_state = crate::orbit::step_orbit(&self.canon, self.sim_time_seconds);

        let entry = AuditEntry {
            tick: self.tick,
            step: StepNumber::Orbit,
            component: "Orbit".to_string(),
            status: AuditStatus::Success,
            // OrbitState (mean anomaly, r/a) carries no reservoir-flux-shaped
            // quantity at all — these are genuinely zero, not unfilled.
            metrics: ComponentMetrics {
                energy_in: 0.0,
                energy_out: 0.0,
                water_in: 0.0,
                water_out: 0.0,
                carbon_in: 0.0,
                carbon_out: 0.0,
                oxygen_in: 0.0,
                oxygen_out: 0.0,
                nitrogen_in: 0.0,
                nitrogen_out: 0.0,
                phosphorus_in: 0.0,
                phosphorus_out: 0.0,
                intelligence_max: 0.0,
                population: 0.0,
            },
            hash_before,
            hash_after: state_fingerprint(&self.hash_chain.current, &self.orbit_state),
        };

        self.audit_trail.entries.push(entry);
        Ok(())
    }

    /// Planetary rotation (subsolar point and declination) for this step,
    /// shared by insolation and climate so both see the same sun.
    fn rotation_for_step(&self, _dt_seconds: u64) -> crate::rotation::RotationState {
        crate::rotation::step_rotation(
            self.sim_time_seconds,
            self.orbit_state.mean_anomaly,
            &self.canon,
        )
    }

    fn step_insolation(&mut self, dt_seconds: u64) -> Result<(), WorldStepError> {
        let hash_before = self.hash_chain.current;
        let ledger_start = self.ledger.entries().len();

        let rotation = self.rotation_for_step(dt_seconds);

        // Execute insolation step (Phase 2 § 3.2)
        self.insolation_state = crate::insolation::step_insolation(
            &self.canon,
            &self.orbit_state,
            &rotation,
            &self.grid_spec,
            dt_seconds as f64,
            &mut self.ledger,
        );

        let metrics = self.step_ledger_metrics(ledger_start);

        let entry = AuditEntry {
            tick: self.tick,
            step: StepNumber::Insolation,
            component: "Insolation".to_string(),
            status: AuditStatus::Success,
            metrics,
            hash_before,
            hash_after: state_fingerprint(&self.hash_chain.current, &self.insolation_state),
        };

        self.audit_trail.entries.push(entry);
        Ok(())
    }

    fn step_planet(&mut self, _dt_seconds: u64) -> Result<(), WorldStepError> {
        let hash_before = self.hash_chain.current;

        // Execute planet fields step (Phase 2 § 3.3)
        self.planet_state =
            crate::planet::step_planet(&self.canon, &self.canon_derived, &self.grid_spec);

        let entry = AuditEntry {
            tick: self.tick,
            step: StepNumber::Planet,
            component: "Planet".to_string(),
            status: AuditStatus::Success,
            // PlanetFields (rotation rate, gravitational parameter, Coriolis)
            // carries no reservoir-flux-shaped quantity — genuinely zero.
            metrics: ComponentMetrics {
                energy_in: 0.0,
                energy_out: 0.0,
                water_in: 0.0,
                water_out: 0.0,
                carbon_in: 0.0,
                carbon_out: 0.0,
                oxygen_in: 0.0,
                oxygen_out: 0.0,
                nitrogen_in: 0.0,
                nitrogen_out: 0.0,
                phosphorus_in: 0.0,
                phosphorus_out: 0.0,
                intelligence_max: 0.0,
                population: 0.0,
            },
            hash_before,
            hash_after: state_fingerprint(&self.hash_chain.current, &self.planet_state),
        };

        self.audit_trail.entries.push(entry);
        Ok(())
    }

    fn step_tides(&mut self, dt_seconds: u64) -> Result<(), WorldStepError> {
        let hash_before = self.hash_chain.current;

        // Tides is the only physical subsystem that already records real
        // flux entries into the shared ledger — capture where it starts so
        // the metrics below reflect exactly what this step pushed.
        let ledger_start = self.ledger.entries().len();

        // Execute tidal forcing step (Phase 2 § 3.4)
        self.tides_state = crate::tides::step_tides(
            &self.canon,
            self.sim_time_seconds,
            dt_seconds,
            &self.grid_spec,
            &mut self.ledger,
        );

        let metrics = self.step_ledger_metrics(ledger_start);

        let entry = AuditEntry {
            tick: self.tick,
            step: StepNumber::Tides,
            component: "Tides".to_string(),
            status: AuditStatus::Success,
            metrics,
            hash_before,
            hash_after: state_fingerprint(&self.hash_chain.current, &self.tides_state),
        };

        self.audit_trail.entries.push(entry);
        Ok(())
    }

    fn step_climate(&mut self, dt_seconds: u64) -> Result<(), WorldStepError> {
        let hash_before = self.hash_chain.current;
        let ledger_start = self.ledger.entries().len();

        // Execute climate step (Phase 2 § 3.5): relax the previous field
        // toward the energy balance set by the current orbit, declination,
        // geothermal flux and the carbon cycle's volcanic CO2 source.
        let rotation = self.rotation_for_step(dt_seconds);
        let geothermal = self.volcanism_state.heat_flux_w_m2(&self.grid_spec);
        // Tidal dissipation heats the ocean: spread the tides step's power
        // over the ocean's area as an extra areal flux, so the heat the
        // ledger records as entering the ocean actually warms it.
        let tidal_power_w = self.tides_state.heating_power_w.max(0.0);
        let radius = self.canon.planet_radius_m;
        let ocean_area_m2: f64 = (0..self.grid_spec.nlat)
            .map(|row| {
                let area = self
                    .grid_spec
                    .cell_area_at_row_m2(row, radius)
                    .unwrap_or(0.0);
                (0..self.grid_spec.nlon)
                    .filter(|&col| {
                        self.elevation_grid
                            .get_safe(row, col)
                            .copied()
                            .unwrap_or(0.0)
                            <= 0.0
                    })
                    .count() as f64
                    * area
            })
            .sum();
        let tidal_w_m2 = if ocean_area_m2 > 0.0 {
            tidal_power_w / ocean_area_m2
        } else {
            0.0
        };
        let mut surface_heat_flux = geothermal.clone();
        for row in 0..self.grid_spec.nlat {
            for col in 0..self.grid_spec.nlon {
                if self
                    .elevation_grid
                    .get_safe(row, col)
                    .copied()
                    .unwrap_or(0.0)
                    <= 0.0
                {
                    if let Some(flux) = surface_heat_flux.get_mut_safe(row, col) {
                        *flux += tidal_w_m2;
                    }
                }
            }
        }
        let temperature_before = self.climate_state.surface_temperature.clone();
        let co2_before_ppm = self.climate_state.co2_concentration;
        self.climate_state = crate::climate::step_climate(
            &self.climate_state,
            &crate::climate::ClimateForcing {
                toa_solar_flux_w_m2: self.canon.solar_constant_w_m2
                    / (self.orbit_state.r_over_a * self.orbit_state.r_over_a),
                solar_declination_rad: rotation.subsolar_latitude,
                albedo: self.canon.albedo_baseline,
                surface_pressure_pa: self.canon.sea_level_pressure_pa,
                surface_gravity_m_s2: self.canon.surface_gravity_m_s2,
                geothermal_flux_w_m2: &surface_heat_flux,
                volcanic_co2_mol_yr: self.volcanism_state.total_co2_mol_yr(&self.grid_spec),
                elevation_m: &self.elevation_grid,
                dt_seconds: dt_seconds as f64,
            },
            &self.grid_spec,
        );
        crate::conservation::book_climate_step(
            &mut self.ledger,
            &self.grid_spec,
            radius,
            &self.elevation_grid,
            &temperature_before,
            &self.climate_state.surface_temperature,
            &geothermal,
            tidal_power_w * dt_seconds as f64,
            dt_seconds as f64,
        );
        // The climate step's carbon cycle moves CO2 between the air and the
        // crust: volcanic outgassing in, silicate weathering out.
        crate::conservation::book_atmospheric_co2_change(
            &mut self.ledger,
            mk_core::flux::Reservoir::CrustCarbon,
            crate::conservation::atmospheric_carbon_kg_per_ppm(&self.canon),
            co2_before_ppm,
            self.climate_state.co2_concentration,
        );

        // The carbon the carbon cycle exchanged with the crust this step, as
        // booked in the ledger above.
        let booked = self.step_ledger_metrics(ledger_start);
        let entry = AuditEntry {
            tick: self.tick,
            step: StepNumber::Climate,
            component: "Climate".to_string(),
            status: AuditStatus::Success,
            // Real: absorbed solar+tidal power vs. Stefan-Boltzmann emission
            // at the final (seasonally adjusted) temperature, summed over
            // the grid — the exact radiative balance `step_climate` solves,
            // not an invented number.
            metrics: ComponentMetrics {
                energy_in: self.climate_state.absorbed_flux_w_m2,
                energy_out: self.climate_state.emitted_flux_w_m2,
                water_in: 0.0,
                water_out: 0.0,
                carbon_in: booked.carbon_in,
                carbon_out: booked.carbon_out,
                oxygen_in: 0.0,
                oxygen_out: 0.0,
                nitrogen_in: 0.0,
                nitrogen_out: 0.0,
                phosphorus_in: 0.0,
                phosphorus_out: 0.0,
                intelligence_max: 0.0,
                population: 0.0,
            },
            hash_before,
            hash_after: state_fingerprint(&self.hash_chain.current, &self.climate_state),
        };

        self.audit_trail.entries.push(entry);
        Ok(())
    }

    fn step_weather(&mut self, _dt_seconds: u64) -> Result<(), WorldStepError> {
        let hash_before = self.hash_chain.current;

        // Execute weather step (Phase 2 § 3.6)
        self.weather_state = crate::weather::step_weather(
            &self.canon,
            self.tick,
            &self.climate_state,
            &self.planet_state.coriolis,
            &self.elevation_grid,
            &self.grid_spec,
        );

        let entry = AuditEntry {
            tick: self.tick,
            step: StepNumber::Weather,
            component: "Weather".to_string(),
            status: AuditStatus::Success,
            // Real: total precipitation actually computed this tick
            // (mm/day, summed over the grid) is genuine water entering the
            // surface/hydrology system. Weather has no evaporation field of
            // its own (that's computed downstream in hydrology), so
            // water_out stays zero here rather than double-counting.
            metrics: ComponentMetrics {
                energy_in: 0.0,
                energy_out: 0.0,
                water_in: self.weather_state.precipitation.sum(),
                water_out: 0.0,
                carbon_in: 0.0,
                carbon_out: 0.0,
                oxygen_in: 0.0,
                oxygen_out: 0.0,
                nitrogen_in: 0.0,
                nitrogen_out: 0.0,
                phosphorus_in: 0.0,
                phosphorus_out: 0.0,
                intelligence_max: 0.0,
                population: 0.0,
            },
            hash_before,
            hash_after: state_fingerprint(&self.hash_chain.current, &self.weather_state),
        };

        self.audit_trail.entries.push(entry);
        Ok(())
    }

    fn step_hydrology(&mut self, dt_seconds: u64) -> Result<(), WorldStepError> {
        let hash_before = self.hash_chain.current;

        // Execute hydrology step (Phase 2 § 3.7)
        let topo = self.tectonics_state.get_elevation_grid();
        self.hydrology_state = crate::hydrology::step_hydrology(
            &self.canon,
            &self.hydrology_state,
            &self.weather_state,
            &self.climate_state,
            &topo,
            dt_seconds as f64,
            &self.grid_spec,
        );
        crate::conservation::book_hydrology_step(&mut self.ledger, &self.hydrology_state.budget);

        let entry = AuditEntry {
            tick: self.tick,
            step: StepNumber::Hydrology,
            component: "Hydrology".to_string(),
            status: AuditStatus::Success,
            // Real: infiltration (mm/day, summed) is water genuinely
            // entering the soil/groundwater system this tick; evaporation
            // plus runoff (both real computed fields) is water genuinely
            // leaving the surface system.
            metrics: ComponentMetrics {
                energy_in: 0.0,
                energy_out: 0.0,
                water_in: self.hydrology_state.infiltration.sum(),
                water_out: self.hydrology_state.evaporation.sum()
                    + self.hydrology_state.runoff.sum(),
                carbon_in: 0.0,
                carbon_out: 0.0,
                oxygen_in: 0.0,
                oxygen_out: 0.0,
                nitrogen_in: 0.0,
                nitrogen_out: 0.0,
                phosphorus_in: 0.0,
                phosphorus_out: 0.0,
                intelligence_max: 0.0,
                population: 0.0,
            },
            hash_before,
            hash_after: state_fingerprint(&self.hash_chain.current, &self.hydrology_state),
        };

        self.audit_trail.entries.push(entry);
        Ok(())
    }

    fn step_ocean(&mut self, dt_seconds: u64) -> Result<(), WorldStepError> {
        let hash_before = self.hash_chain.current;

        // Execute ocean step (Phase 2 § 3.8)
        self.ocean_state = crate::ocean::step_ocean(
            &self.canon,
            &self.ocean_state.columns,
            &crate::ocean::OceanForcing {
                wind: &self.weather_state.wind,
                climate: &self.climate_state,
                precipitation_mm_day: &self.weather_state.precipitation,
                coriolis: &self.planet_state.coriolis,
                elevation_m: &self.elevation_grid,
                dt_seconds: dt_seconds as f64,
            },
            &self.grid_spec,
        );

        let entry = AuditEntry {
            tick: self.tick,
            step: StepNumber::Ocean,
            component: "Ocean".to_string(),
            status: AuditStatus::Success,
            // Real: mixed-layer heat uptake/release (W/m²) and ocean
            // precipitation in / bulk-formula evaporation out (mm/day),
            // summed over ocean cells — the quantities `step_ocean` uses to
            // update surface temperature and salinity.
            metrics: ComponentMetrics {
                energy_in: self.ocean_state.heat_absorbed_w_m2,
                energy_out: self.ocean_state.heat_released_w_m2,
                water_in: self.ocean_state.water_gained_mm_day,
                water_out: self.ocean_state.water_lost_mm_day,
                carbon_in: 0.0,
                carbon_out: 0.0,
                oxygen_in: 0.0,
                oxygen_out: 0.0,
                nitrogen_in: 0.0,
                nitrogen_out: 0.0,
                phosphorus_in: 0.0,
                phosphorus_out: 0.0,
                intelligence_max: 0.0,
                population: 0.0,
            },
            hash_before,
            hash_after: state_fingerprint(&self.hash_chain.current, &self.ocean_state),
        };

        self.audit_trail.entries.push(entry);
        Ok(())
    }

    fn step_tectonics(&mut self, _dt_seconds: u64) -> Result<(), WorldStepError> {
        let hash_before = self.hash_chain.current;

        // Execute tectonics step (Phase 2 § 3.9)
        // The plate field is regenerated each step; authored surface relief
        // is persistent world state and carries over.
        let surface_relief = self.tectonics_state.surface_relief_m.take();
        self.tectonics_state =
            crate::tectonics::step_tectonics(&self.canon, self.tick, &self.grid_spec);
        self.tectonics_state.surface_relief_m = surface_relief;

        let entry = AuditEntry {
            tick: self.tick,
            step: StepNumber::Tectonics,
            component: "Tectonics".to_string(),
            status: AuditStatus::Success,
            // Real: geothermal heat flow (W/m², summed over the grid) is
            // genuine energy entering the system from below the crust.
            metrics: ComponentMetrics {
                energy_in: self.tectonics_state.heat_flow.sum(),
                energy_out: 0.0,
                water_in: 0.0,
                water_out: 0.0,
                carbon_in: 0.0,
                carbon_out: 0.0,
                oxygen_in: 0.0,
                oxygen_out: 0.0,
                nitrogen_in: 0.0,
                nitrogen_out: 0.0,
                phosphorus_in: 0.0,
                phosphorus_out: 0.0,
                intelligence_max: 0.0,
                population: 0.0,
            },
            hash_before,
            hash_after: state_fingerprint(&self.hash_chain.current, &self.tectonics_state),
        };

        self.audit_trail.entries.push(entry);
        Ok(())
    }

    fn step_volcanism(&mut self, dt_seconds: u64) -> Result<(), WorldStepError> {
        let hash_before = self.hash_chain.current;

        // Execute volcanism step (Phase 2 § 3.10)
        self.volcanism_state = crate::volcanism::step_volcanism(
            &self.canon,
            self.tick,
            &self.tectonics_state.plates,
            &self.tectonics_state.heat_flow,
            self.volcanism_state.degassed_fraction,
            dt_seconds as f64,
            &self.rng,
            &self.grid_spec,
        );

        let entry = AuditEntry {
            tick: self.tick,
            step: StepNumber::Volcanism,
            component: "Volcanism".to_string(),
            status: AuditStatus::Success,
            // Real: CO2 outgassed this tick (summed over the grid) is
            // genuine carbon entering the atmosphere reservoir.
            metrics: ComponentMetrics {
                energy_in: 0.0,
                energy_out: 0.0,
                water_in: 0.0,
                water_out: 0.0,
                carbon_in: self.volcanism_state.outgassed_co2.sum(),
                carbon_out: 0.0,
                oxygen_in: 0.0,
                oxygen_out: 0.0,
                nitrogen_in: 0.0,
                nitrogen_out: 0.0,
                phosphorus_in: 0.0,
                phosphorus_out: 0.0,
                intelligence_max: 0.0,
                population: 0.0,
            },
            hash_before,
            hash_after: state_fingerprint(&self.hash_chain.current, &self.volcanism_state),
        };

        self.audit_trail.entries.push(entry);
        Ok(())
    }

    /// Classify biomes from the annual-mean climate (temperature and
    /// precipitation), with the current elevation, soil moisture and
    /// volcanic ground.
    fn step_biome_classification(&mut self, dt_seconds: u64) {
        self.elevation_grid = self.tectonics_state.get_elevation_grid();
        self.climatology.update(
            &self.climate_state.surface_temperature,
            &self.weather_state.precipitation,
            dt_seconds as f64,
            self.sim_time_seconds / self.canon.orbital_period_s,
        );
        let nlon = self.grid_spec.nlon;
        for (row, col, biome) in self.biome_grid.indexed_iter_mut() {
            let elev = *self.elevation_grid.get(row, col);
            let index = row * nlon + col;
            let temp = self.climatology.temperature_k[index];
            let precip = self.climatology.precipitation_mm_day[index];
            let moist = self
                .hydrology_state
                .soil_water
                .get(row, col)
                .moisture_fraction;
            let is_volcanic = self
                .volcanism_state
                .volcanism
                .get_safe(row, col)
                .is_some_and(|v| v.dominates_cell());

            *biome = mk_core::biomes::classify_biome(elev, temp, precip, moist, is_volcanic);
        }

        // Major rivers: land cells whose upstream catchment's runoff
        // reaches the major-river discharge.
        let flow = mk_core::biomes::rivers::river_flow(
            &self.elevation_grid,
            self.hydrology_state
                .generated_runoff
                .as_ref()
                .unwrap_or(&self.hydrology_state.runoff),
            self.canon.planet_radius_m,
        );
        mk_core::biomes::rivers::mark_river_biomes(&mut self.biome_grid, &flow);
    }

    fn step_biosphere(&mut self, dt_seconds: u64) -> Result<(), WorldStepError> {
        let hash_before = self.hash_chain.current;

        // Classify biomes from current physical state
        self.step_biome_classification(dt_seconds);

        // Execute biosphere step (Phase 3 § 3.1)
        let dt_myr = dt_seconds as f64 / (365.25 * 24.0 * 3600.0 * 1_000_000.0);
        self.biosphere_state
            .step_biosphere(
                dt_myr,
                &self.climate_state,
                &self.weather_state,
                &self.ocean_state,
                &self.elevation_grid,
            )
            .map_err(WorldStepError::BiosphereError)?;

        let terrain = crate::organisms::OrganismTerrain {
            elevation: &self.elevation_grid,
            npp_kgc_m2_yr: &self.biosphere_state.npp_field_kgc_m2_yr,
        };
        self.organisms_state
            .seed_from_species(&self.biosphere_state.species, &terrain);
        self.organisms_state.step(
            self.tick,
            dt_seconds as f64 / (365.25 * 24.0 * 3600.0),
            &terrain,
            &self.biosphere_state.species,
            &self.rng,
        );
        self.vegetation_state.seed_from_biomes(&self.biome_grid);
        self.vegetation_state
            .step((dt_seconds as f64 / (365.25 * 24.0 * 3600.0)) as f32);

        let dt_years = dt_seconds as f32 / (365.25 * 24.0 * 3600.0);
        self.catalogue_individuals
            .step(dt_years, &self.rng, self.tick);

        let stats = self.biosphere_state.get_statistics();
        let max_intelligence = stats.maximum_intelligence;
        const PHOTOSYNTHETIC_ENERGY_J_PER_KG_C: f64 = 3.89e7;
        const PLANT_CARBON_TO_NITROGEN: f64 = 40.0;
        const PLANT_CARBON_TO_PHOSPHORUS: f64 = 400.0;
        let fixed_carbon_kg = stats.current_npp_kgc_yr * (dt_seconds as f64 / (365.25 * 86_400.0));

        let metrics = ComponentMetrics {
            // Per-step photosynthetic fluxes from the carbon actually
            // fixed this step (NPP × step length), by the stoichiometry of
            // CO2 + H2O → CH2O + O2: 18/12 kg water taken up and 32/12 kg
            // O2 released per kg C, ~38.9 MJ of chemical energy stored per
            // kg C, and nutrient uptake at typical terrestrial plant tissue
            // ratios (C:N ≈ 40, C:P ≈ 400). The model has no respiration
            // or decomposition flux, so nothing is reported leaving.
            energy_in: fixed_carbon_kg * PHOTOSYNTHETIC_ENERGY_J_PER_KG_C,
            energy_out: 0.0,
            water_in: fixed_carbon_kg * 18.0 / 12.0,
            water_out: 0.0,
            carbon_in: fixed_carbon_kg,
            carbon_out: 0.0,
            oxygen_in: 0.0,
            oxygen_out: fixed_carbon_kg * 32.0 / 12.0,
            nitrogen_in: fixed_carbon_kg / PLANT_CARBON_TO_NITROGEN,
            nitrogen_out: 0.0,
            phosphorus_in: fixed_carbon_kg / PLANT_CARBON_TO_PHOSPHORUS,
            phosphorus_out: 0.0,
            intelligence_max: max_intelligence,
            population: 0.0,
        };

        // Check intelligence ceiling
        let status = if max_intelligence > crate::biosphere::genetics::PRE_SAPIENT_CEILING {
            AuditStatus::Critical(format!(
                "Intelligence ceiling exceeded: {}",
                max_intelligence
            ))
        } else if let Err(violation) = self.biosphere_state.validate() {
            AuditStatus::Warning(format!("Biosphere invariant violated: {violation}"))
        } else if max_intelligence > crate::biosphere::genetics::PRE_SAPIENT_CEILING * 0.8 {
            AuditStatus::Warning(format!("High intelligence detected: {}", max_intelligence))
        } else {
            AuditStatus::Success
        };

        let entry = AuditEntry {
            tick: self.tick,
            step: StepNumber::Biosphere,
            component: "Biosphere".to_string(),
            status,
            metrics,
            hash_before,
            hash_after: state_fingerprint(&self.hash_chain.current, &self.biosphere_state),
        };

        self.metrics.max_intelligence = self.metrics.max_intelligence.max(max_intelligence);
        self.audit_trail.entries.push(entry);
        Ok(())
    }

    fn step_disturbance(&mut self, _dt_seconds: u64) -> Result<(), WorldStepError> {
        let hash_before = self.hash_chain.current;
        let ledger_start = self.ledger.entries().len();

        // Apply every active exogenous disturbance to the subsystem it
        // physically belongs to. Runs after the physical subsystems have
        // stepped so a disturbance perturbs this tick's computed state
        // rather than being overwritten by it.
        let disturbance_effects = crate::disturbance::step_disturbances(
            &mut self.disturbance_state,
            self.tick,
            &self.grid_spec,
            &self.elevation_grid,
            &mut self.climate_state,
            &mut self.weather_state,
            &mut self.hydrology_state,
            &mut self.biosphere_state,
            &mut self.ledger,
        );
        for effect in &disturbance_effects {
            tracing::debug!(
                tick = self.tick,
                kind = effect.kind.as_str(),
                row = effect.row,
                col = effect.col,
                intensity = effect.intensity,
                summary = %effect.summary,
                "disturbance applied"
            );
        }

        // The audit reports exactly the fluxes the active disturbances
        // pushed to the ledger this step (wildfire carbon, flood water,
        // eruption heat, …), like every other ledger-backed component.
        let metrics = self.step_ledger_metrics(ledger_start);

        let entry = AuditEntry {
            tick: self.tick,
            step: StepNumber::Disturbance,
            component: "Disturbance".to_string(),
            status: AuditStatus::Success,
            metrics,
            hash_before,
            hash_after: state_fingerprint(&self.hash_chain.current, &self.disturbance_state),
        };

        self.audit_trail.entries.push(entry);
        Ok(())
    }

    fn step_evolution(&mut self, dt_seconds: u64) -> Result<(), WorldStepError> {
        let hash_before = self.hash_chain.current;

        // Track species count before evolution for event detection
        let _species_count_before = self.biosphere_state.species.len();
        let species_ids_before: std::collections::HashSet<u64> = self
            .biosphere_state
            .species
            .iter()
            .map(|s| s.species_id)
            .collect();

        // Step evolution under the stress the live climate actually imposes.
        let dt_years = dt_seconds as f64 / (365.25 * 24.0 * 3600.0);
        let stress = climate_environmental_stress(&self.climate_state);
        self.deep_time
            .step_evolution_processes(
                &mut self.biosphere_state.species,
                dt_years,
                stress,
                &mut self.rng,
            )
            .map_err(WorldStepError::EvolutionError)?;
        self.biosphere_state.refresh_statistics();

        // Emit events for new species created during evolution
        for species in &self.biosphere_state.species {
            if !species_ids_before.contains(&species.species_id) {
                // New species was created
                crate::io::global_events::log_species_created(
                    self.tick,
                    species.species_id,
                    &species.species_name,
                );
                self.chronicle.record(
                    self.tick,
                    crate::chronicle::ChronicleKind::SpeciesEmerged,
                    format!("{} emerged", species.species_name),
                    0.5,
                );
            }
        }

        // Check for extinctions (species with 0 population)
        for species in &self.biosphere_state.species {
            if species.population_size == 0 {
                crate::io::global_events::log_species_extinct(
                    self.tick,
                    species.species_id,
                    &species.species_name,
                    0,
                );
            }
        }

        // Evolution operates on species population/genome fields, not a
        // reservoir-flux quantity — genuinely zero, not unfilled.
        let metrics = ComponentMetrics {
            energy_in: 0.0,
            energy_out: 0.0,
            water_in: 0.0,
            water_out: 0.0,
            carbon_in: 0.0,
            carbon_out: 0.0,
            oxygen_in: 0.0,
            oxygen_out: 0.0,
            nitrogen_in: 0.0,
            nitrogen_out: 0.0,
            phosphorus_in: 0.0,
            phosphorus_out: 0.0,
            intelligence_max: 0.0,
            population: 0.0,
        };

        let entry = AuditEntry {
            tick: self.tick,
            step: StepNumber::Evolution,
            component: "Evolution".to_string(),
            status: AuditStatus::Success,
            metrics,
            hash_before,
            hash_after: state_fingerprint(&self.hash_chain.current, &self.deep_time),
        };

        self.audit_trail.entries.push(entry);
        Ok(())
    }

    fn step_extinction(&mut self, dt_seconds: u64) -> Result<(), WorldStepError> {
        let hash_before = self.hash_chain.current;

        let living_before: std::collections::BTreeSet<_> = self
            .biosphere_state
            .species
            .iter()
            .filter(|s| s.population_size > 0)
            .map(|s| s.species_id)
            .collect();

        // Stochastic extinction events over the interval step 13 just
        // advanced, recorded where projections read extinction history.
        let dt_years = dt_seconds as f64 / (365.25 * 24.0 * 3600.0);
        let events_before = self.deep_time.extinction_events.len();
        self.deep_time.step_extinction_processes(
            &mut self.biosphere_state.species,
            dt_years,
            &mut self.rng,
        );
        self.biosphere_state.refresh_statistics();
        for event in &self.deep_time.extinction_events[events_before..] {
            self.biosphere_state
                .extinction_system
                .record(event.event_id, &format!("{:?}", event.severity));
        }

        for species in &self.biosphere_state.species {
            if species.population_size == 0 && living_before.contains(&species.species_id) {
                self.chronicle.record(
                    self.tick,
                    crate::chronicle::ChronicleKind::SpeciesExtinct,
                    format!("{} went extinct", species.species_name),
                    0.8,
                );
            }
        }

        // Extinction operates on species population fields, not a
        // reservoir-flux quantity — genuinely zero, not unfilled.
        let metrics = ComponentMetrics {
            energy_in: 0.0,
            energy_out: 0.0,
            water_in: 0.0,
            water_out: 0.0,
            carbon_in: 0.0,
            carbon_out: 0.0,
            oxygen_in: 0.0,
            oxygen_out: 0.0,
            nitrogen_in: 0.0,
            nitrogen_out: 0.0,
            phosphorus_in: 0.0,
            phosphorus_out: 0.0,
            intelligence_max: 0.0,
            population: 0.0,
        };

        let entry = AuditEntry {
            tick: self.tick,
            step: StepNumber::Extinction,
            component: "Extinction".to_string(),
            status: AuditStatus::Success,
            metrics,
            hash_before,
            hash_after: state_fingerprint(
                &self.hash_chain.current,
                &self.biosphere_state.extinction_system,
            ),
        };

        self.audit_trail.entries.push(entry);
        Ok(())
    }

    fn step_agents(&mut self, dt_seconds: u64) -> Result<(), WorldStepError> {
        let dt_years = dt_seconds as f64 / (365.25 * 24.0 * 3600.0);
        let hash_before = self.hash_chain.current;

        self.resource_economy_state.regenerate();

        let occupancy = self.living_occupancy();
        let built_shelter = self.built_shelter_field();
        let nlon = self.grid_spec.nlon;
        let mut audits: Vec<AgentStepAudit> = Vec::with_capacity(self.agents_state.agents.len());
        for i in 0..self.agents_state.agents.len() {
            if !self.agents_state.agents[i].is_alive() {
                continue;
            }

            let pos = self.agents_state.agents[i].position;
            let r = pos.row.max(0).min((self.grid_spec.nlat - 1) as i32) as usize;
            let c = pos.col.max(0).min((self.grid_spec.nlon - 1) as i32) as usize;

            let temp = if r < self.climate_state.surface_temperature.nlat()
                && c < self.climate_state.surface_temperature.nlon()
            {
                *self.climate_state.surface_temperature.get(r, c)
                    - crate::perception::KELVIN_TO_CELSIUS_OFFSET
            } else {
                18.0
            };

            let biome = self.biome_grid.get_safe(r, c).copied();
            let resource_abundance =
                crate::perception::caloric_access(&self.biosphere_state, self.grid_spec.nlon, r, c);

            let observation = AgentWorldObservation {
                tick: self.tick,
                ambient_temperature_c: temp,
                hydration_access: crate::perception::hydration_access(
                    &self.hydrology_state,
                    &self.weather_state.precipitation,
                    biome,
                    r,
                    c,
                ),
                caloric_access: resource_abundance.clamp(0.0, 1.0),
                shelter_quality: biome
                    .map(|b| {
                        let props = mk_core::biomes::biome_properties(b);
                        props.canopy_cover * 0.6 + 0.2
                    })
                    .unwrap_or(0.5)
                    .max(built_shelter[r * nlon + c]),
                social_density: occupancy.social_density(pos, true),
                hazard_index: biome
                    .map(|b| {
                        let props = mk_core::biomes::biome_properties(b);
                        props.roughness * 0.5 + (1.0 - props.traversability) * 0.5
                    })
                    .unwrap_or(0.1),
                daylight_fraction: crate::perception::daylight_fraction(
                    &self.insolation_state,
                    r,
                    c,
                ),
                biome_type: biome,
                resource_abundance,
                computer_access: 0.0,
                computer_bridge_available: 0.0,
            };

            let audit = self.agents_state.agents[i].step(&observation, &self.rng);
            self.resource_economy_state.apply_agent_action(
                &mut self.agents_state.agents[i],
                self.tick,
                &self.elevation_grid,
            );
            if let Some(next) = self.agent_movement(i, dt_years, &built_shelter) {
                self.agents_state.agents[i].position = next;
            }
            audits.push(audit);
        }
        self.agents_state.last_audits = audits;

        let entry = AuditEntry {
            tick: self.tick,
            step: StepNumber::Agent,
            component: "Agents".to_string(),
            status: AuditStatus::Success,
            metrics: ComponentMetrics {
                energy_in: 0.0,
                energy_out: 0.0,
                water_in: 0.0,
                water_out: 0.0,
                carbon_in: 0.0,
                carbon_out: 0.0,
                oxygen_in: 0.0,
                oxygen_out: 0.0,
                nitrogen_in: 0.0,
                nitrogen_out: 0.0,
                phosphorus_in: 0.0,
                phosphorus_out: 0.0,
                // World agents have no intelligence index of their own (that
                // lives on biosphere species); their living count is reported
                // as population.
                intelligence_max: 0.0,
                population: self.agents_state.living_population() as f64,
            },
            hash_before,
            hash_after: state_fingerprint(&self.hash_chain.current, &self.agents_state),
        };

        self.audit_trail.entries.push(entry);
        Ok(())
    }

    /// Where world agent `index` walks this step, if anywhere. Only a
    /// movement action moves it, and only with the walking-speed chance of
    /// reaching a neighbouring cell over `dt_years`
    /// (`humans::cell_crossing_probability`, the same pace as humans).
    /// `Move`/`Explore` take a keyed random neighbour. `SeekFood`,
    /// `SeekWater` and `SeekShelter` take the neighbour offering the most of
    /// what is sought. Longitude wraps, and an agent on land never steps onto
    /// open ocean.
    fn agent_movement(
        &self,
        index: usize,
        dt_years: f64,
        built_shelter: &[f64],
    ) -> Option<crate::agents::GridPosition> {
        use crate::agents::ActionKind;
        let agent = &self.agents_state.agents[index];
        let action = agent.last_action.kind;
        if !matches!(
            action,
            ActionKind::Move
                | ActionKind::Explore
                | ActionKind::SeekFood
                | ActionKind::SeekWater
                | ActionKind::SeekShelter
        ) {
            return None;
        }
        let (nlat, nlon) = (self.grid_spec.nlat as i32, self.grid_spec.nlon as i32);
        let here = agent.position;
        let key = |salt: u32| {
            mk_core::rng::RngKey::new(
                mk_core::rng::SubsystemId::Agents,
                index as u32,
                salt,
                self.tick,
            )
        };
        let crossing = crate::humans::cell_crossing_probability(here, dt_years, &self.grid_spec);
        if self.rng.gen_f64_01(key(0)) >= crossing {
            return None;
        }
        let is_ocean = |p: crate::agents::GridPosition| {
            self.elevation_grid
                .get_safe(p.row.max(0) as usize, p.col.max(0) as usize)
                .is_some_and(|h| *h < 0.0)
        };
        let neighbours: Vec<crate::agents::GridPosition> = [
            (-1, -1),
            (-1, 0),
            (-1, 1),
            (0, -1),
            (0, 1),
            (1, -1),
            (1, 0),
            (1, 1),
        ]
        .into_iter()
        .map(|(dr, dc)| {
            crate::agents::GridPosition::new(
                (here.row + dr).clamp(0, nlat - 1),
                (here.col + dc).rem_euclid(nlon.max(1)),
            )
        })
        .filter(|p| *p != here && (is_ocean(here) || !is_ocean(*p)))
        .collect();
        if neighbours.is_empty() {
            return None;
        }
        let access = |p: crate::agents::GridPosition| -> Option<f64> {
            let (r, c) = (p.row as usize, p.col as usize);
            match action {
                ActionKind::SeekFood => Some(crate::perception::caloric_access(
                    &self.biosphere_state,
                    self.grid_spec.nlon,
                    r,
                    c,
                )),
                ActionKind::SeekWater => Some(crate::perception::hydration_access(
                    &self.hydrology_state,
                    &self.weather_state.precipitation,
                    self.biome_grid.get_safe(r, c).copied(),
                    r,
                    c,
                )),
                ActionKind::SeekShelter => Some(
                    self.biome_grid
                        .get_safe(r, c)
                        .map(|b| mk_core::biomes::biome_properties(*b).canopy_cover * 0.6 + 0.2)
                        .unwrap_or(0.5)
                        .max(built_shelter[r * self.grid_spec.nlon + c]),
                ),
                _ => None,
            }
        };
        if access(here).is_some() {
            // Seeking: the best neighbour, if it beats staying.
            let (best, value) = neighbours
                .iter()
                .map(|p| (*p, access(*p).unwrap_or(0.0)))
                .max_by(|a, b| a.1.total_cmp(&b.1))?;
            return (value > access(here).unwrap_or(0.0)).then_some(best);
        }
        let pick = (self.rng.gen_f64_01(key(1)) * neighbours.len() as f64) as usize;
        neighbours.get(pick.min(neighbours.len() - 1)).copied()
    }

    /// Snapshot of where every living human and world agent stands this
    /// step, for perceived social density (see [`crate::perception`]).
    fn living_occupancy(&self) -> crate::perception::Occupancy {
        let humans = self
            .humans_state
            .registry
            .iter()
            .filter(|human| matches!(human.profile.status, mk_core::human::HumanStatus::Alive))
            .map(|human| human.position);
        let agents = self
            .agents_state
            .agents
            .iter()
            .filter(|agent| agent.is_alive())
            .map(|agent| agent.position);
        crate::perception::Occupancy::new(&self.grid_spec, humans.chain(agents))
    }

    /// One decision period of the human step: observe, decide, act, move.
    fn step_humans_period(&mut self, dt_years: f64, rng: &RngRegistry) {
        let built_shelter = self.built_shelter_field();
        let nlon = self.grid_spec.nlon;
        let grid_spec = &self.grid_spec;
        let climate_state = &self.climate_state;
        let biome_grid = &self.biome_grid;
        let elevation_grid = &self.elevation_grid;
        let properties = &self.property_state.properties;
        let computer_bridge_available = if self.computer_bridge.is_some() {
            1.0
        } else {
            0.0
        };
        let tick = self.tick;
        let occupancy = self.living_occupancy();
        let insolation_state = &self.insolation_state;
        let hydrology_state = &self.hydrology_state;
        let precipitation = &self.weather_state.precipitation;
        let biosphere_state = &self.biosphere_state;
        let observation_for = |position: &crate::humans::GridPosition, agent_id: &str| {
            let r = position.row.max(0).min((grid_spec.nlat - 1) as i32) as usize;
            let c = position.col.max(0).min((grid_spec.nlon - 1) as i32) as usize;

            let temp = if r < climate_state.surface_temperature.nlat()
                && c < climate_state.surface_temperature.nlon()
            {
                *climate_state.surface_temperature.get(r, c)
                    - crate::perception::KELVIN_TO_CELSIUS_OFFSET
            } else {
                18.0
            };

            let biome = biome_grid.get_safe(r, c).copied();
            let resource_abundance =
                crate::perception::caloric_access(biosphere_state, grid_spec.nlon, r, c);

            crate::humans::AgentWorldObservation {
                tick,
                ambient_temperature_c: temp,
                hydration_access: crate::perception::hydration_access(
                    hydrology_state,
                    precipitation,
                    biome,
                    r,
                    c,
                ),
                caloric_access: resource_abundance.clamp(0.0, 1.0),
                shelter_quality: biome
                    .map(|b| {
                        let props = mk_core::biomes::biome_properties(b);
                        props.canopy_cover * 0.6 + 0.2
                    })
                    .unwrap_or(0.5)
                    .max(built_shelter[r * nlon + c]),
                social_density: occupancy.social_density(*position, true),
                hazard_index: biome
                    .map(|b| {
                        let props = mk_core::biomes::biome_properties(b);
                        props.roughness * 0.5 + (1.0 - props.traversability) * 0.5
                    })
                    .unwrap_or(0.1),
                daylight_fraction: crate::perception::daylight_fraction(insolation_state, r, c),
                biome_type: biome,
                resource_abundance,
                computer_access: if properties.iter().any(|property| {
                    property.location == Some((r, c))
                        && property.buildings.iter().any(|building| {
                            matches!(
                                building.kind,
                                crate::organisms::property::PropertyBuildingKind::ComputerRoom
                            )
                        })
                        && property
                            .network_accounts
                            .iter()
                            .any(|account| account.agent_id == agent_id)
                }) {
                    1.0
                } else {
                    0.0
                },
                computer_bridge_available,
            }
        };

        self.humans_state.step(
            dt_years,
            self.tick,
            rng,
            grid_spec,
            &mut self.resource_economy_state,
            elevation_grid,
            observation_for,
            self.computer_bridge.as_ref().map(|h| h.as_bridge()),
        );
        self.settlements_state
            .sync_humans(&self.humans_state, &self.resource_economy_state.structures);
    }

    /// Best built shelter per cell (row-major): constructed structures and
    /// placed property buildings.
    fn built_shelter_field(&self) -> Vec<f64> {
        let (nlat, nlon) = (self.grid_spec.nlat, self.grid_spec.nlon);
        let mut field = vec![0.0_f64; nlat * nlon];
        for structure in &self.resource_economy_state.structures {
            let (row, col) = (structure.position.row, structure.position.col);
            if row < 0 || col < 0 || row as usize >= nlat || col as usize >= nlon {
                continue;
            }
            if let Some(quality) =
                crate::resource_economy::structure_shelter_quality(structure.recipe)
            {
                let cell = &mut field[row as usize * nlon + col as usize];
                *cell = (*cell).max(quality);
            }
        }
        for row in 0..nlat {
            for col in 0..nlon {
                let cell = &mut field[row * nlon + col];
                *cell = (*cell).max(self.property_state.shelter_at(row, col));
            }
        }
        field
    }

    fn step_humans(&mut self, dt_seconds: u64) -> Result<(), WorldStepError> {
        let hash_before = self.hash_chain.current;

        let dt_years = dt_seconds as f64 / (365.25 * 24.0 * 3600.0);

        // Humans decide and act in periods of at most a day: a longer world
        // step is split into that many decisions, each seeing where everyone
        // stands after the last. Period 0 draws from the world's own RNG,
        // later periods from registries derived from it, so a step of a day
        // or less behaves exactly as a single decision.
        // With nobody alive there is nothing to decide: one pass suffices.
        let anyone_alive = self
            .humans_state
            .registry
            .iter()
            .any(|human| matches!(human.profile.status, mk_core::human::HumanStatus::Alive));
        let periods = if anyone_alive {
            human_decision_periods(dt_years)
        } else {
            1
        };
        let period_years = dt_years / periods as f64;
        for period in 0..periods {
            let rng = if period == 0 {
                self.rng.clone()
            } else {
                decision_period_rng(&self.rng, self.tick, period)
            };
            self.step_humans_period(period_years, &rng);
        }

        let entry = AuditEntry {
            tick: self.tick,
            step: StepNumber::Humans,
            component: "Humans".to_string(),
            status: AuditStatus::Success,
            metrics: ComponentMetrics {
                energy_in: 0.0,
                energy_out: 0.0,
                water_in: 0.0,
                water_out: 0.0,
                carbon_in: 0.0,
                carbon_out: 0.0,
                oxygen_in: 0.0,
                oxygen_out: 0.0,
                nitrogen_in: 0.0,
                nitrogen_out: 0.0,
                phosphorus_in: 0.0,
                phosphorus_out: 0.0,
                intelligence_max: 0.0,
                population: self.humans_state.registry.population_count() as f64,
            },
            hash_before,
            hash_after: state_fingerprint(&self.hash_chain.current, &self.humans_state),
        };

        self.audit_trail.entries.push(entry);
        Ok(())
    }

    fn step_audit(&mut self, _dt_seconds: u64) -> Result<(), WorldStepError> {
        // Calculate closure metrics from the reconciliation of the current ledger
        self.calculate_closure_metrics();
        self.update_world_metrics();

        // Check conservation: every audited interior stock's measured change
        // since the last audit must equal the net inflow the ledger recorded.
        let stocks_now = crate::conservation::measure(self);
        if let Some(before) = self.stocks_at_last_audit.take() {
            self.audit_trail.stock_audit =
                crate::conservation::audit(&self.ledger, self.tick, &before, &stocks_now);
        }
        self.stocks_at_last_audit = Some(stocks_now);

        // Validate intelligence ceiling from the biosphere state
        self.validate_intelligence_ceiling();

        // Check hash chain continuity
        self.validate_hash_chain();

        let entry = AuditEntry {
            tick: self.tick,
            step: StepNumber::Audit,
            component: "Audit".to_string(),
            status: AuditStatus::Success,
            metrics: ComponentMetrics {
                energy_in: 0.0,
                energy_out: 0.0,
                water_in: 0.0,
                water_out: 0.0,
                carbon_in: 0.0,
                carbon_out: 0.0,
                oxygen_in: 0.0,
                oxygen_out: 0.0,
                nitrogen_in: 0.0,
                nitrogen_out: 0.0,
                phosphorus_in: 0.0,
                phosphorus_out: 0.0,
                intelligence_max: 0.0,
                population: 0.0,
            },
            hash_before: self.hash_chain.current,
            hash_after: state_fingerprint(&self.hash_chain.current, &self.ledger),
        };
        self.audit_trail.entries.push(entry);
        Ok(())
    }

    fn step_hash_commit(&mut self, _dt_seconds: u64) -> Result<(), WorldStepError> {
        let hash_before = self.hash_chain.current;

        // Hash the actual subsystem state content, not just audit-entry
        // placeholders. Previously this only hashed `entry.hash_after` for
        // every subsystem AuditEntry — but every one of those is
        // unconditionally set equal to `hash_chain.current` unchanged, by
        // design (see `validate_hash_chain`'s "steps 1-14 must not mutate
        // hash_chain.current" continuity invariant). So the hasher's input
        // was structurally `(prior hash, tick, N copies of the same
        // unchanged prior hash)` and never actually depended on organism/
        // individual/species/biosphere/etc. state at all — two runs could
        // diverge in that state and this chain would never detect it.
        // Bincode-serializing each subsystem field in a fixed declaration
        // order (not a HashMap iteration order — deterministic either way)
        // makes the chain sensitive to the state it claims to audit, while
        // preserving the existing "only step_hash_commit mutates
        // hash_chain.current" invariant.
        let mut hasher = blake3::Hasher::new();
        hasher.update(&self.hash_chain.current);
        hasher.update(&self.tick.to_le_bytes());

        macro_rules! hash_subsystem_state {
            ($field:expr) => {
                if let Ok(bytes) = bincode::serialize($field) {
                    hasher.update(&bytes);
                }
            };
        }
        hash_subsystem_state!(&self.orbit_state);
        hash_subsystem_state!(&self.insolation_state);
        hash_subsystem_state!(&self.planet_state);
        hash_subsystem_state!(&self.tides_state);
        hash_subsystem_state!(&self.climate_state);
        hash_subsystem_state!(&self.weather_state);
        hash_subsystem_state!(&self.hydrology_state);
        hash_subsystem_state!(&self.ocean_state);
        hash_subsystem_state!(&self.tectonics_state);
        hash_subsystem_state!(&self.volcanism_state);
        hash_subsystem_state!(&self.biosphere_state);
        hash_subsystem_state!(&self.organisms_state);
        hash_subsystem_state!(&self.vegetation_state);
        hash_subsystem_state!(&self.settlements_state);
        hash_subsystem_state!(&self.property_state);
        hash_subsystem_state!(&self.catalogue_individuals);
        hash_subsystem_state!(&self.humans_state);
        hash_subsystem_state!(&self.disturbance_state);
        hash_subsystem_state!(&self.deep_time);
        hash_subsystem_state!(&self.resource_economy_state);
        hash_subsystem_state!(&self.agents_state);

        self.hash_chain.current = hasher.finalize().into();

        let entry = AuditEntry {
            tick: self.tick,
            step: StepNumber::HashCommit,
            component: "HashCommit".to_string(),
            status: AuditStatus::Success,
            metrics: ComponentMetrics::default(),
            hash_before,
            hash_after: self.hash_chain.current,
        };
        self.audit_trail.entries.push(entry);

        Ok(())
    }

    fn step_ledger_clear(&mut self, _dt_seconds: u64) -> Result<(), WorldStepError> {
        // Authoritative clear of the flux ledger for the next tick. The audit
        // trail is deliberately kept: it holds exactly this tick's entries
        // (`step_world` clears it on entry), so it stays bounded to one tick
        // and remains observable after `step_world` returns.
        self.ledger = mk_core::flux::Ledger::new();
        Ok(())
    }

    fn step_time_advance(&mut self, dt_seconds: u64) -> Result<(), WorldStepError> {
        self.tick += 1;
        self.sim_time_seconds += dt_seconds as f64;
        Ok(())
    }

    /// Refresh the world-level summary: conserved-quantity throughput the
    /// ledger recorded this tick (per kind, before it clears), the number of
    /// live species and their total population. `max_intelligence` is kept
    /// as the running maximum by the biosphere step.
    fn update_world_metrics(&mut self) {
        use mk_core::flux::FluxKind;
        let throughput = |kind: FluxKind| -> f64 {
            self.ledger
                .entries()
                .iter()
                .filter(|entry| entry.kind() == kind)
                .map(|entry| entry.amount().abs())
                .sum()
        };
        self.metrics.total_energy = throughput(FluxKind::Energy);
        self.metrics.total_water = throughput(FluxKind::Water);
        self.metrics.total_carbon = throughput(FluxKind::Carbon);
        self.metrics.total_oxygen = throughput(FluxKind::Oxygen);
        self.metrics.total_nitrogen = throughput(FluxKind::Nitrogen);
        self.metrics.total_phosphorus = throughput(FluxKind::Phosphorus);
        self.metrics.species_count = self.biosphere_state.species.len();
        self.metrics.total_population = self
            .biosphere_state
            .species
            .iter()
            .map(|species| species.population_size)
            .sum();
    }

    fn calculate_closure_metrics(&mut self) {
        // Net exchange of the interior reservoirs with the boundary this
        // tick (`Ledger::net_imbalance`): the ledger-implied change in stored
        // interior quantity. It is not a conservation residual, because the
        // double-entry ledger balances by construction; a real residual needs
        // measured reservoir stocks (`Ledger::audit_against_stocks`).
        self.audit_trail.energy_closure =
            self.ledger.net_imbalance(mk_core::flux::FluxKind::Energy);
        self.audit_trail.water_closure = self.ledger.net_imbalance(mk_core::flux::FluxKind::Water);
        self.audit_trail.carbon_closure =
            self.ledger.net_imbalance(mk_core::flux::FluxKind::Carbon);
        self.audit_trail.oxygen_closure =
            self.ledger.net_imbalance(mk_core::flux::FluxKind::Oxygen);
        self.audit_trail.nitrogen_closure =
            self.ledger.net_imbalance(mk_core::flux::FluxKind::Nitrogen);
        self.audit_trail.phosphorus_closure = self
            .ledger
            .net_imbalance(mk_core::flux::FluxKind::Phosphorus);

        self.metrics.total_energy += self.audit_trail.energy_closure;
        self.metrics.total_water += self.audit_trail.water_closure;
        self.metrics.total_carbon += self.audit_trail.carbon_closure;
        self.metrics.total_oxygen += self.audit_trail.oxygen_closure;
        self.metrics.total_nitrogen += self.audit_trail.nitrogen_closure;
        self.metrics.total_phosphorus += self.audit_trail.phosphorus_closure;
    }

    fn validate_intelligence_ceiling(&mut self) {
        self.audit_trail.intelligence_ceiling_status =
            self.metrics.max_intelligence <= crate::biosphere::genetics::PRE_SAPIENT_CEILING;
    }

    fn validate_hash_chain(&mut self) {
        // Subsystem steps must not mutate hash_chain.current — only
        // step_hash_commit does — so every pre-commit entry ran under the
        // same chain position, and each carries a real fingerprint of the
        // state it produced (never just a copy of the chain hash).
        let expected = self.hash_chain.current;
        let continuity = self
            .audit_trail
            .entries
            .iter()
            .all(|e| e.hash_before == expected && e.hash_after != expected);
        self.audit_trail.hash_chain_continuity = continuity;
    }

    /// Create snapshot for replay
    ///
    /// The payload is JSON, the same encoding `io::snapshot` persists:
    /// `WorldState` carries `serde_json::Value` fields (human profiles),
    /// which bincode can serialize but not deserialize.
    pub fn create_snapshot(&self) -> Result<WorldSnapshot, SnapshotError> {
        let serialized = serde_json::to_vec(self)
            .map_err(|e| SnapshotError::SerializationError(e.to_string()))?;

        let digest = blake3::hash(&serialized);

        Ok(WorldSnapshot {
            tick: self.tick,
            data: serialized,
            digest: digest.into(),
        })
    }

    /// Restore a world from a snapshot made by [`Self::create_snapshot`]:
    /// the digest is verified, the canon validated, and the derived canon
    /// values recomputed from the restored canon.
    pub fn from_snapshot(snapshot: &WorldSnapshot) -> Result<WorldState, SnapshotError> {
        let digest: [u8; 32] = blake3::hash(&snapshot.data).into();
        if digest != snapshot.digest {
            return Err(SnapshotError::DigestMismatch);
        }
        let mut loaded: WorldState = serde_json::from_slice(&snapshot.data)
            .map_err(|e| SnapshotError::DeserializationError(e.to_string()))?;
        mk_core::canon::validator::validate_canon(&loaded.canon)
            .map_err(|e| SnapshotError::DeserializationError(format!("invalid canon: {e}")))?;
        loaded.canon_derived = Arc::new(mk_core::canon::CanonDerived::from(&loaded.canon));
        Ok(loaded)
    }

    /// Load snapshot and verify (see [`Self::from_snapshot`]).
    pub fn load_snapshot(&mut self, snapshot: &WorldSnapshot) -> Result<(), SnapshotError> {
        *self = Self::from_snapshot(snapshot)?;
        Ok(())
    }

    /// Validate world state
    pub fn validate(&self) -> Result<(), ValidationError> {
        // Validate audit trail
        if !self.audit_trail.intelligence_ceiling_status {
            return Err(ValidationError::IntelligenceCeilingViolation);
        }

        if !self.audit_trail.hash_chain_continuity {
            return Err(ValidationError::HashChainBreak);
        }

        // Validate biosphere
        self.biosphere_state
            .validate()
            .map_err(ValidationError::BiosphereError)?;

        Ok(())
    }
}

/// World snapshot for replay
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldSnapshot {
    pub tick: Tick,
    pub data: Vec<u8>,
    pub digest: [u8; 32],
}

/// World step errors
#[derive(Debug, Clone, thiserror::Error)]
pub enum WorldStepError {
    #[error("orbit error: {0}")]
    OrbitError(String),
    #[error("insolation error: {0}")]
    InsolationError(String),
    #[error("planet error: {0}")]
    PlanetError(String),
    #[error("tides error: {0}")]
    TidesError(String),
    #[error("climate error: {0}")]
    ClimateError(String),
    #[error("weather error: {0}")]
    WeatherError(String),
    #[error("hydrology error: {0}")]
    HydrologyError(String),
    #[error("ocean error: {0}")]
    OceanError(String),
    #[error("tectonics error: {0}")]
    TectonicsError(String),
    #[error("volcanism error: {0}")]
    VolcanismError(String),
    #[error("biosphere error: {0}")]
    BiosphereError(String),
    #[error("disturbance error: {0}")]
    DisturbanceError(String),
    #[error("evolution error: {0}")]
    EvolutionError(String),
    #[error("extinction error: {0}")]
    ExtinctionError(String),
    #[error("audit error: {0}")]
    AuditError(String),
    #[error("hash error: {0}")]
    HashError(String),
    #[error("ledger error: {0}")]
    LedgerError(String),
    #[error("time error: {0}")]
    TimeError(String),
}

/// Snapshot errors
#[derive(Debug, Clone, thiserror::Error)]
pub enum SnapshotError {
    #[error("serialization error: {0}")]
    SerializationError(String),
    #[error("deserialization error: {0}")]
    DeserializationError(String),
    #[error("snapshot digest mismatch")]
    DigestMismatch,
    #[error("io error: {0}")]
    IoError(String),
}

/// Validation errors
#[derive(Debug, Clone, thiserror::Error)]
pub enum ValidationError {
    #[error("intelligence ceiling violation")]
    IntelligenceCeilingViolation,
    #[error("hash chain break")]
    HashChainBreak,
    #[error("biosphere error: {0}")]
    BiosphereError(String),
    #[error("NaN value in field: {0}")]
    NaNValue(String),
    #[error("infinite value in field: {0}")]
    InfiniteValue(String),
}

#[cfg(test)]
mod founders_estate_placement_tests {
    use super::*;

    #[test]
    fn new_world_places_founders_estate_on_a_real_land_cell() {
        let world = WorldState::new(Arc::new(CanonLocked::default()), [9u8; 32]);

        let estate = world
            .property_state
            .properties
            .iter()
            .find(|p| p.owner_agent_ids.contains(&"Gem-D".to_string()))
            .expect("founders' estate should exist whenever the generated planet has living land");

        let (row, col) = estate
            .location
            .expect("founders' estate should have a real placed location");

        assert!(
            *world.elevation_grid.get(row, col) > 0.0,
            "founders' estate must sit on land, not ocean"
        );
        assert!(estate
            .buildings
            .iter()
            .any(|b| b.kind == crate::organisms::PropertyBuildingKind::House));
    }

    #[test]
    fn estate_site_is_the_best_year_round_climate_cell() {
        let world = WorldState::new(Arc::new(CanonLocked::default()), [9u8; 32]);

        // Re-running the scan over the world's own published state must
        // reproduce the site that was actually chosen — the placement is a
        // pure function of the generated planet, not a hand-picked cell.
        // The site is scored on the annual-mean climate, not one day's.
        let annual_temperature = mk_core::grid::Grid2::from_data(
            &world.grid_spec,
            world.climatology.temperature_k.clone(),
        );
        let annual_precipitation = mk_core::grid::Grid2::from_data(
            &world.grid_spec,
            world.climatology.precipitation_mm_day.clone(),
        );
        let expected = choose_estate_location(
            &world.elevation_grid,
            &annual_temperature,
            &annual_precipitation,
            &world.volcanism_state,
            world.canon.obliquity_deg.to_radians(),
            &world.grid_spec,
        );

        assert!(
            expected.is_some(),
            "the default planet should have a land cell"
        );
        assert_eq!(world.property_state.founders_estate_location(), expected);
    }

    #[test]
    fn estate_site_is_less_seasonal_than_the_poles() {
        let world = WorldState::new(Arc::new(CanonLocked::default()), [9u8; 32]);
        let Ok((row, _col)) = world.property_state.founders_estate_location().ok_or(()) else {
            return;
        };

        // "Good weather all year round" means a small axial-tilt seasonal
        // swing: the chosen site must beat the most extreme latitude row.
        let obliquity = world.canon.obliquity_deg.to_radians();
        let site =
            crate::climate::seasonal_temperature_amplitude(obliquity, world.grid_spec.lat_rad(row));
        let pole = crate::climate::seasonal_temperature_amplitude(
            obliquity,
            std::f64::consts::FRAC_PI_2 * 0.98,
        );
        assert!(
            site < pole,
            "estate seasonal swing {site} should be milder than the polar swing {pole}"
        );
    }

    #[test]
    fn audit_entries_fingerprint_the_state_each_step_produced() {
        let mut world = WorldState::new(Arc::new(CanonLocked::default()), [5u8; 32]);
        world.step_world(3600).unwrap();

        assert!(world.audit_trail.hash_chain_continuity);
        // The ocean is not touched after its own step (later steps exchange
        // carbon and oxygen with the climate state), so the final ocean
        // state is exactly the one the Ocean step produced.
        let ocean = world
            .audit_trail
            .entries
            .iter()
            .find(|e| e.step == StepNumber::Ocean)
            .expect("ocean entry");
        assert_ne!(ocean.hash_after, ocean.hash_before);
        assert_eq!(
            ocean.hash_after,
            state_fingerprint(&ocean.hash_before, &world.ocean_state)
        );
    }

    #[test]
    fn audit_trail_holds_exactly_one_ticks_entries() {
        let mut world = WorldState::new(Arc::new(CanonLocked::default()), [5u8; 32]);
        world.step_world(3600).unwrap();
        let per_tick = world.audit_trail.entries.len();
        assert!(per_tick > 0, "entries must be observable after step_world");

        for _ in 0..10 {
            world.step_world(3600).unwrap();
            assert_eq!(
                world.audit_trail.entries.len(),
                per_tick,
                "the trail must stay bounded to one tick's entries"
            );
        }
    }

    #[test]
    fn reflected_sunlight_never_enters_the_planet() {
        let mut world = WorldState::new(Arc::new(CanonLocked::default()), [5u8; 32]);
        world.step_world(3600).unwrap();

        let insolation = world
            .audit_trail
            .entries
            .iter()
            .find(|e| e.step == StepNumber::Insolation)
            .expect("insolation entry");
        // The insolation step books only the albedo share, which goes from
        // the top-of-atmosphere boundary straight back to space: no interior
        // reservoir gains or loses it. The absorbed rest is booked by the
        // climate step against the heat it adds.
        assert_eq!(insolation.metrics.energy_in, 0.0);
        assert_eq!(insolation.metrics.energy_out, 0.0);
        assert!(world.ledger.entries().is_empty() || world.tick > 0);
    }
}

#[cfg(test)]
mod snapshot_tests {
    use super::*;

    #[test]
    fn snapshot_round_trips_a_world_with_humans() {
        let mut world = WorldState::new(Arc::new(CanonLocked::default()), [5u8; 32]);
        world
            .humans_state
            .registry
            .add_human_no_storage(crate::humans::HumanBeing::gem_d_founder());
        world.step_world(1).expect("world steps");

        let snapshot = world.create_snapshot().expect("snapshot is created");
        let restored = WorldState::from_snapshot(&snapshot).expect("snapshot restores");

        assert_eq!(restored.tick, world.tick);
        assert_eq!(restored.hash_chain.current, world.hash_chain.current);
        assert_eq!(
            restored.humans_state.registry.count(),
            world.humans_state.registry.count()
        );
        assert!(restored.humans_state.registry.get_human("Gem-D").is_some());

        // The restored world continues exactly as the original does: every
        // float survives the JSON round trip bit for bit.
        let mut original = world;
        let mut restored = restored;
        original.step_world(1).expect("original steps");
        restored.step_world(1).expect("restored steps");
        assert_eq!(restored.hash_chain.current, original.hash_chain.current);
    }

    #[test]
    fn tampered_snapshot_is_rejected() {
        let world = WorldState::new(Arc::new(CanonLocked::default()), [5u8; 32]);
        let mut snapshot = world.create_snapshot().expect("snapshot is created");
        let last = snapshot.data.len() - 1;
        snapshot.data[last] ^= 0x01;
        assert!(matches!(
            WorldState::from_snapshot(&snapshot),
            Err(SnapshotError::DigestMismatch)
        ));
    }
}

#[cfg(test)]
mod agent_movement_tests {
    use super::*;
    use crate::agents::{ActionKind, Agent, AgentId};

    #[test]
    fn agents_walk_on_land_at_walking_pace_and_seek_what_they_want() {
        let mut world = WorldState::new(Arc::new(CanonLocked::default()), [6u8; 32]);
        let spec = world.grid_spec.clone();
        let land = (0..spec.nlat)
            .flat_map(|r| (0..spec.nlon).map(move |c| (r, c)))
            .find(|&(r, c)| {
                *world.elevation_grid.get(r, c) >= 0.0 && (1..spec.nlat - 1).contains(&r)
            })
            .expect("land");
        let mut agent = Agent::spawn_at(AgentId::new("walker"), land.0 as i32, land.1 as i32);
        agent.last_action.kind = ActionKind::Explore;
        world.agents_state.agents.push(agent);
        let shelter = vec![0.0; spec.nlat * spec.nlon];

        // A second's walk almost never leaves a ~1,900 km cell.
        let second = 1.0 / (365.25 * 86_400.0);
        assert_eq!(world.agent_movement(0, second, &shelter), None);

        // A year's walk crosses it, onto a neighbouring land cell.
        let next = world.agent_movement(0, 1.0, &shelter).expect("moves");
        let here = world.agents_state.agents[0].position;
        let d_col = (next.col - here.col).rem_euclid(spec.nlon as i32);
        assert!((next.row - here.row).abs() <= 1 && (d_col <= 1 || d_col == spec.nlon as i32 - 1));
        assert!(
            *world
                .elevation_grid
                .get(next.row as usize, next.col as usize)
                >= 0.0
        );

        // Resting does not move.
        world.agents_state.agents[0].last_action.kind = ActionKind::Rest;
        assert_eq!(world.agent_movement(0, 1.0, &shelter), None);
    }
}

#[cfg(test)]
mod human_decision_period_tests {
    use super::*;

    #[test]
    fn long_steps_split_into_daily_decisions_with_fresh_draws() {
        let day = 1.0 / 365.25;
        assert_eq!(human_decision_periods(0.0), 1);
        assert_eq!(human_decision_periods(day / 10.0), 1);
        assert_eq!(human_decision_periods(day), 1);
        assert_eq!(human_decision_periods(7.0 * day), 7);
        assert_eq!(human_decision_periods(7.5 * day), 8);
        // Coarse steps are bounded.
        assert_eq!(human_decision_periods(64.0 * day), 64);
        assert_eq!(human_decision_periods(1.0), MAX_HUMAN_DECISION_PERIODS);
        assert_eq!(human_decision_periods(1000.0), MAX_HUMAN_DECISION_PERIODS);

        let world = RngRegistry::new([4u8; 32]);
        let key = mk_core::rng::RngKey::new(mk_core::rng::SubsystemId::Humans, 1, 2, 3);
        let draws: Vec<f64> = (1..4)
            .map(|period| decision_period_rng(&world, 3, period).gen_f64_01(key))
            .collect();
        assert!(draws[0] != draws[1] && draws[1] != draws[2]);
        // Reproducible from the world seed alone.
        assert_eq!(decision_period_rng(&world, 3, 2).gen_f64_01(key), draws[1]);
    }
}

#[cfg(test)]
mod river_biome_tests {
    use super::*;

    #[test]
    fn a_stepped_world_carries_a_few_major_rivers() {
        let mut world = WorldState::new(Arc::new(CanonLocked::default()), [2u8; 32]);
        for _ in 0..3 {
            world.step_world(86_400).expect("world steps");
        }
        let count = |kind: fn(&mk_core::biomes::BiomeType) -> bool| {
            world.biome_grid.data().iter().filter(|b| kind(b)).count()
        };
        let rivers = count(|b| *b == mk_core::biomes::BiomeType::River);
        let land = world
            .elevation_grid
            .data()
            .iter()
            .filter(|h| **h >= 0.0)
            .count();
        println!("river cells: {rivers} of {land} land cells");
        assert!(rivers > 0, "the default world has major rivers");
        assert!(
            rivers * 10 < land,
            "rivers are trunks, not most of the land"
        );
    }
}
