//! Sampling the regional boundary forcing (Phase 1 Task 2).
//!
//! Astronomy is the canon's analytic orbit, rotation and moon geometry
//! (`crate::orbit`, `crate::rotation`, `crate::tides`), so the region sees
//! the same sun and moons as the planetary code. Ocean and atmosphere
//! edge values are a provisional zonal climatology with deterministic
//! noise (`provisional: true`); Phase 2 Task 2 replaces them with values
//! derived from the zonal background through a separate function. Every
//! random value comes from a named blake3 stream keyed by the seed, the
//! canon digest, the field, the edge, the cell and (for weather-like
//! fields) the local day — never from wall-clock time or iteration order.

use mk_core::canon::{CanonDerived, CanonLocked};
use mk_island::{
    AstronomyForcing, AtmosphereBoundaryForcing, CrustKind, DomainLevel, Edge,
    EdgeAtmosphereForcing, EdgeOceanForcing, FarFieldPlate, IslandDomain, OceanBoundaryForcing,
    RegionalBoundaryState, TectonicBoundaryForcing,
};

/// Peak seasonal swing of provisional mid-latitude SST (K): open-ocean
/// seasonal ranges at 40-45° are ~3-6 K (fixtures/reference/climate,
/// `sst_seasonal_range_40_45`), so ±2.5 K.
const SST_SEASONAL_AMPLITUDE_K: f64 = 2.5;
/// Peak seasonal swing of provisional maritime air temperature (K).
const AIR_SEASONAL_AMPLITUDE_K: f64 = 4.0;
/// Relative humidity of marine boundary-layer air (~75-80%).
const MARINE_RELATIVE_HUMIDITY: f64 = 0.78;
/// Present-day plate speeds span ~10-100 mm/yr (DeMets et al. 2010,
/// Geophys. J. Int. 181:1, MORVEL).
const PLATE_SPEED_RANGE_M_YR: (f64, f64) = (0.02, 0.09);

/// A uniform value in [0, 1) from the named stream.
fn uniform(seed: &[u8; 32], digest: &[u8; 32], label: &str, keys: &[u64]) -> f64 {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"mk-island-boundary");
    hasher.update(seed);
    hasher.update(digest);
    hasher.update(label.as_bytes());
    for key in keys {
        hasher.update(&key.to_le_bytes());
    }
    let bytes = hasher.finalize();
    let word = u64::from_le_bytes(bytes.as_bytes()[..8].try_into().expect("8 bytes"));
    (word >> 11) as f64 / (1u64 << 53) as f64
}

/// A value in [-1, 1) from the named stream.
fn signed(seed: &[u8; 32], digest: &[u8; 32], label: &str, keys: &[u64]) -> f64 {
    2.0 * uniform(seed, digest, label, keys) - 1.0
}

fn edge_key(edge: Edge) -> u64 {
    match edge {
        Edge::South => 0,
        Edge::North => 1,
        Edge::West => 2,
        Edge::East => 3,
    }
}

/// Latitudes (rad) of the coarse cells along an edge, in edge order.
fn edge_latitudes(domain: &IslandDomain, edge: Edge) -> Vec<f64> {
    let level = DomainLevel::Coarse;
    let (rows, cols) = (domain.rows(level), domain.cols(level));
    match edge {
        Edge::South => vec![domain.latitude_rad_for_row(level, 0); cols],
        Edge::North => vec![domain.latitude_rad_for_row(level, rows - 1); cols],
        Edge::West | Edge::East => (0..rows)
            .map(|r| domain.latitude_rad_for_row(level, r))
            .collect(),
    }
}

/// Saturation specific humidity (kg/kg) at temperature `t_k` and pressure
/// `p_pa`: Bolton (1980), Mon. Weather Rev. 108:1046.
fn saturation_specific_humidity(t_k: f64, p_pa: f64) -> f64 {
    let t_c = t_k - 273.15;
    let e_s = 611.2 * (17.67 * t_c / (t_c + 243.5)).exp();
    0.622 * e_s / (p_pa - 0.378 * e_s)
}

/// The edge forcing and astronomy at `sim_time_seconds`.
pub fn sample_regional_boundaries(
    seed: [u8; 32],
    canon: &CanonLocked,
    domain: &IslandDomain,
    sim_time_seconds: f64,
) -> RegionalBoundaryState {
    let digest = CanonDerived::from(canon).canon_digest;
    let t = sim_time_seconds.max(0.0);

    let orbit = crate::orbit::step_orbit(canon, t);
    let rotation = crate::rotation::step_rotation(t, orbit.mean_anomaly, canon);
    let astronomy = AstronomyForcing {
        solar_declination_rad: rotation.subsolar_latitude,
        sub_solar_longitude_rad: rotation.subsolar_longitude,
        moon_sub_longitudes_rad: vec![
            crate::tides::sub_lunar_longitude(
                t,
                canon.moon_mckenz_period_s,
                canon.rotation_period_s,
            ),
            crate::tides::sub_lunar_longitude(t, canon.moon_hahn_period_s, canon.rotation_period_s),
        ],
        season_phase: orbit.mean_anomaly / std::f64::consts::TAU,
        star_distance_ratio: orbit.r_over_a,
        rotation_angle_rad: rotation.rotation_angle,
    };

    // Weather-like noise changes once per local day.
    let day = (t / canon.rotation_period_s).floor() as u64;
    let tilt = canon.obliquity_deg.to_radians();
    // +1 at the hemisphere's summer solstice, -1 at its winter solstice.
    let season = |lat: f64| {
        if tilt > 0.0 {
            lat.signum() * rotation.subsolar_latitude / tilt
        } else {
            0.0
        }
    };

    let mut ocean_edges = Vec::new();
    let mut atmosphere_edges = Vec::new();
    for edge in Edge::ALL {
        let e = edge_key(edge);
        let lats = edge_latitudes(domain, edge);
        let mut ocean = EdgeOceanForcing {
            edge,
            sea_surface_temperature_k: Vec::with_capacity(lats.len()),
            salinity_psu: Vec::with_capacity(lats.len()),
            inflow_m_s: Vec::with_capacity(lats.len()),
            sea_level_anomaly_m: Vec::with_capacity(lats.len()),
        };
        let mut air = EdgeAtmosphereForcing {
            edge,
            air_temperature_k: Vec::with_capacity(lats.len()),
            wind_u_m_s: Vec::with_capacity(lats.len()),
            wind_v_m_s: Vec::with_capacity(lats.len()),
            specific_humidity_kg_kg: Vec::with_capacity(lats.len()),
            surface_pressure_pa: Vec::with_capacity(lats.len()),
        };
        for (i, &lat) in lats.iter().enumerate() {
            let i = i as u64;
            let noise = |label: &str| signed(&seed, &digest, label, &[e, i, day]);
            // Zonal-mean SST: ~28 °C at the equator, ~16 °C at 41°.
            let sst = 273.15
                + 28.0 * lat.cos().powi(2)
                + SST_SEASONAL_AMPLITUDE_K * season(lat)
                + 0.3 * noise("sst");
            ocean.sea_surface_temperature_k.push(sst);
            ocean
                .salinity_psu
                .push(canon.mean_salinity_psu + 0.3 * noise("salinity"));
            ocean.inflow_m_s.push(0.05 + 0.05 * noise("inflow"));
            ocean.sea_level_anomaly_m.push(0.1 * noise("sla"));

            let air_t = sst - 1.0
                + (AIR_SEASONAL_AMPLITUDE_K - SST_SEASONAL_AMPLITUDE_K) * season(lat)
                + 1.5 * noise("air_t");
            let pressure = canon.sea_level_pressure_pa + 800.0 * noise("pressure");
            air.air_temperature_k.push(air_t);
            // Trades below ~30°, westerlies peaking near 60° (zonal-mean
            // surface wind climatology).
            air.wind_u_m_s
                .push(-7.0 * (3.0 * lat.abs()).cos() + 2.0 * noise("u"));
            air.wind_v_m_s.push(1.5 * noise("v"));
            air.specific_humidity_kg_kg
                .push(MARINE_RELATIVE_HUMIDITY * saturation_specific_humidity(air_t, pressure));
            air.surface_pressure_pa.push(pressure);
        }
        ocean_edges.push(ocean);
        atmosphere_edges.push(air);
    }

    // Far-field plates are fixed for a seed: they do not change with time.
    let far_field = Edge::ALL
        .iter()
        .map(|&edge| {
            let e = edge_key(edge);
            let (lo, hi) = PLATE_SPEED_RANGE_M_YR;
            let speed = lo + (hi - lo) * uniform(&seed, &digest, "plate_speed", &[e]);
            let heading = std::f64::consts::TAU * uniform(&seed, &digest, "plate_heading", &[e]);
            // About 60% of Earth's surface is oceanic lithosphere.
            let crust = if uniform(&seed, &digest, "plate_crust", &[e]) < 0.6 {
                CrustKind::Oceanic
            } else {
                CrustKind::Continental
            };
            FarFieldPlate {
                edge,
                crust,
                velocity_east_m_yr: speed * heading.cos(),
                velocity_north_m_yr: speed * heading.sin(),
            }
        })
        .collect();

    RegionalBoundaryState {
        ocean: OceanBoundaryForcing {
            provisional: true,
            edges: ocean_edges,
        },
        atmosphere: AtmosphereBoundaryForcing {
            provisional: true,
            edges: atmosphere_edges,
        },
        tectonic: TectonicBoundaryForcing { far_field },
        astronomy,
        sim_time_seconds: t,
    }
}
