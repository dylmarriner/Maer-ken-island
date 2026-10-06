//! Boundary forcing at the edges of the regional domain: what the rest of
//! the planet imposes on the island region. Data only; sampled by
//! `mk_engine::regional::boundary`. Nothing here depends on wall-clock time.

use serde::{Deserialize, Serialize};

/// A side of the domain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Edge {
    South,
    North,
    West,
    East,
}

impl Edge {
    pub const ALL: [Edge; 4] = [Edge::South, Edge::North, Edge::West, Edge::East];
}

/// Ocean state imposed along one edge, one value per coarse edge cell
/// (west-to-east for south/north edges, south-to-north for west/east).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EdgeOceanForcing {
    pub edge: Edge,
    pub sea_surface_temperature_k: Vec<f64>,
    pub salinity_psu: Vec<f64>,
    /// Inflow speed normal to the edge, positive into the domain (m/s).
    pub inflow_m_s: Vec<f64>,
    pub sea_level_anomaly_m: Vec<f64>,
}

/// Atmosphere state imposed along one edge.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EdgeAtmosphereForcing {
    pub edge: Edge,
    pub air_temperature_k: Vec<f64>,
    /// Eastward and northward wind (m/s).
    pub wind_u_m_s: Vec<f64>,
    pub wind_v_m_s: Vec<f64>,
    pub specific_humidity_kg_kg: Vec<f64>,
    pub surface_pressure_pa: Vec<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OceanBoundaryForcing {
    /// True while the values are seed-derived climatology placeholders
    /// (Phase 1); Phase 2 derives them from the zonal background.
    pub provisional: bool,
    pub edges: Vec<EdgeOceanForcing>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AtmosphereBoundaryForcing {
    pub provisional: bool,
    pub edges: Vec<EdgeAtmosphereForcing>,
}

/// Crust kind of a plate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CrustKind {
    Oceanic,
    Continental,
}

/// A plate beyond one edge of the domain, pushing on the regional plates.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FarFieldPlate {
    pub edge: Edge,
    pub crust: CrustKind,
    /// Velocity relative to the planet's mean (m/yr), east and north.
    pub velocity_east_m_yr: f64,
    pub velocity_north_m_yr: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TectonicBoundaryForcing {
    /// One far-field plate per edge, in [`Edge::ALL`] order.
    pub far_field: Vec<FarFieldPlate>,
}

/// Sun and moons at the sampled instant, from the canon's analytic orbit
/// and rotation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AstronomyForcing {
    pub solar_declination_rad: f64,
    pub sub_solar_longitude_rad: f64,
    /// Planet-fixed longitude under each moon (McKenz, Hahn).
    pub moon_sub_longitudes_rad: Vec<f64>,
    /// Fraction of the orbital year elapsed since periapsis (0..1).
    pub season_phase: f64,
    /// Star distance over the semi-major axis.
    pub star_distance_ratio: f64,
    pub rotation_angle_rad: f64,
}

/// Everything imposed on the region at one simulated instant.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RegionalBoundaryState {
    pub ocean: OceanBoundaryForcing,
    pub atmosphere: AtmosphereBoundaryForcing,
    pub tectonic: TectonicBoundaryForcing,
    pub astronomy: AstronomyForcing,
    pub sim_time_seconds: f64,
}
