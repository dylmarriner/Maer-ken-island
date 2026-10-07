//! Travelling synoptic systems (Phase 2 Task 3b): the cyclones, fronts and
//! anticyclones that make mid-latitude weather a sequence of spells.
//!
//! Upstream weather is diagnostic from the climate, so it has no storms.
//! A dynamical atmosphere is beyond this phase; instead systems are
//! generated from the physics that creates them and steered by the
//! background flow:
//!
//! - **Genesis rate** from the Eady baroclinic growth rate
//!   `σ = 0.31 f |∂u/∂z| / N`. Thermal wind gives `∂u/∂z = g |∂T/∂y| /
//!   (f T)`, so `σ = 0.31 g |∂T/∂y| / (N T)`: the rotation rate cancels,
//!   and the 36 h day changes system *size*, not how often they form. The
//!   passage interval is the reference storm-track interval (4 Earth
//!   days, the middle of 2.5-6 d, Blackmon 1976) scaled by `σ_Earth / σ`.
//! - **Size** from the Rossby deformation radius `L_d = N H / f`: a system
//!   is `0.6 L_d` across its closed isobars.
//! - **Motion**: the flow at the steering level (~700 hPa): the
//!   background surface wind plus the thermal-wind shear over
//!   [`STEERING_HEIGHT_M`], `g H |∂T/∂y| / (f T)`, plus a slow poleward
//!   drift; systems enter at the upwind edge and leave at the downwind one.
//! - **Life**: a pressure anomaly that deepens and decays over a lifetime
//!   of ~3.5 days (2-7, Simmonds & Keay 2000), scaled like the passage
//!   interval.
//!
//! [`apply_synoptic`] modulates the diagnostic weather: lows blow
//! gradient-balanced wind around them and rain, fronts bring a rain band
//! and a wind shift, highs bring clear calm. A slow running normaliser
//! keeps the long-run mean rain equal to the climatology the systems
//! modulate.
//!
//! Every random value is a named blake3 stream keyed by the seed and the
//! system's index, never wall-clock time.

use mk_core::canon::CanonLocked;
use mk_core::grid::Grid2;
use mk_island::{DomainLevel, IslandDomain};
use serde::{Deserialize, Serialize};

use super::climate::regional_coriolis;
use super::zonal::ZonalBackgroundState;
use crate::weather::WeatherState;

/// Brunt–Väisälä frequency of the mid-latitude troposphere (s⁻¹).
const BRUNT_VAISALA_S: f64 = 0.01;
/// Depth of the troposphere (m).
const TROPOPAUSE_M: f64 = 10_000.0;
/// Earth's mid-latitude Eady growth rate (s⁻¹): ~0.5 per day.
const EARTH_EADY_RATE_S: f64 = 0.5 / 86_400.0;
/// Reference storm-track passage interval (s) at Earth's growth rate.
const REFERENCE_INTERVAL_S: f64 = 4.0 * 86_400.0;
/// Reference mean system lifetime (s): 3.5 Earth days.
const REFERENCE_LIFETIME_S: f64 = 3.5 * 86_400.0;
/// Height (m) of the level that steers a system, ~700 hPa.
const STEERING_HEIGHT_M: f64 = 3_000.0;
/// Largest steering wind (m/s) accepted near the equator, where f -> 0.
const MAX_STEERING_M_S: f64 = 40.0;
/// Poleward drift of a system (m/s).
const POLEWARD_DRIFT_M_S: f64 = 1.0;
/// Radius of a system's closed isobars in units of the deformation radius.
pub const RADIUS_OVER_DEFORMATION: f64 = 0.6;
/// Peak pressure anomaly (Pa): lows are deeper than highs are high.
const CYCLONE_DEPTH_PA: f64 = -1_600.0;
const ANTICYCLONE_HEIGHT_PA: f64 = 900.0;
/// Fraction of geostrophic wind that reaches the surface (friction).
const SURFACE_WIND_FRACTION: f64 = 0.6;
const AIR_DENSITY_KG_M3: f64 = 1.2;
/// Rain from a front's band at full strength, mm/day, and its width
/// across the front as a fraction of system radius.
const FRONT_RAIN_MM_DAY: f64 = 20.0;
const FRONT_WIDTH_OVER_RADIUS: f64 = 0.12;
/// A low's rain multiplier at its core and a high's suppression.
const CYCLONE_RAIN_GAIN: f64 = 3.0;
const ANTICYCLONE_RAIN_CUT: f64 = 0.9;
/// Time constant (s) of the running normaliser of the rain modulation.
const NORMALISER_TIME_CONSTANT_S: f64 = 60.0 * 86_400.0;
/// Spin-up (s) a bootstrap runs so the domain starts populated.
const SPIN_UP_S: f64 = 10.0 * 86_400.0;
const SPIN_UP_STEP_S: f64 = 6.0 * 3_600.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SystemKind {
    Cyclone,
    Anticyclone,
    Front,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SynopticSystem {
    pub id: u64,
    pub kind: SystemKind,
    /// Metres east and north of the domain's south-west corner.
    pub centre_m: (f64, f64),
    pub velocity_m_s: (f64, f64),
    /// Pressure anomaly at the centre (Pa; negative for a low). Zero for
    /// a front, which is a rain band.
    pub central_pressure_pa: f64,
    /// Closed-isobar radius (cyclone, anticyclone) or half-length of the
    /// band (front), m.
    pub radius_m: f64,
    pub age_s: f64,
    pub lifetime_s: f64,
    /// Peak anomaly the system reaches (Pa).
    pub peak_pa: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SynopticState {
    pub systems: Vec<SynopticSystem>,
    pub time_s: f64,
    /// Fractional systems owed by the genesis rate.
    pub genesis_credit: f64,
    pub next_id: u64,
    /// Running mean of the ratio of modulated to unmodulated domain rain.
    pub mean_rain_ratio: f64,
}

/// The Eady growth rate (s⁻¹) at `latitude_rad` from the background's
/// meridional surface-temperature gradient.
pub fn eady_growth_rate_s(
    background: &ZonalBackgroundState,
    latitude_rad: f64,
    canon: &CanonLocked,
) -> f64 {
    let dlat = std::f64::consts::PI / background.grid.nlat as f64;
    let t_north = background.surface_temperature_at(latitude_rad + dlat);
    let t_south = background.surface_temperature_at(latitude_rad - dlat);
    let t = background.surface_temperature_at(latitude_rad).max(1.0);
    let dt_dy = (t_north - t_south) / (2.0 * dlat * canon.planet_radius_m.max(1.0));
    0.31 * canon.surface_gravity_m_s2 * dt_dy.abs() / (BRUNT_VAISALA_S * t)
}

/// Eastward thermal-wind shear (m/s) between the surface and
/// [`STEERING_HEIGHT_M`] at `latitude_rad`, from the background's
/// meridional temperature gradient: `g H |∂T/∂y| / (|f| T)`.
pub fn thermal_wind_shear_m_s(
    background: &ZonalBackgroundState,
    latitude_rad: f64,
    canon: &CanonLocked,
) -> f64 {
    let dlat = std::f64::consts::PI / background.grid.nlat as f64;
    let dt_dy = (background.surface_temperature_at(latitude_rad + dlat)
        - background.surface_temperature_at(latitude_rad - dlat))
        / (2.0 * dlat * canon.planet_radius_m.max(1.0));
    let omega = std::f64::consts::TAU / canon.rotation_period_s.max(1.0);
    let f = (2.0 * omega * latitude_rad.sin()).abs().max(1e-6);
    let t = background.surface_temperature_at(latitude_rad).max(1.0);
    (canon.surface_gravity_m_s2 * STEERING_HEIGHT_M * dt_dy.abs() / (f * t)).min(MAX_STEERING_M_S)
}

/// The Rossby deformation radius `N H / |f|` (m).
pub fn deformation_radius_m(latitude_rad: f64, canon: &CanonLocked) -> f64 {
    let omega = std::f64::consts::TAU / canon.rotation_period_s.max(1.0);
    BRUNT_VAISALA_S * TROPOPAUSE_M / (2.0 * omega * latitude_rad.sin().abs()).max(1e-6)
}

fn uniform(seed: &[u8; 32], label: &str, id: u64) -> f64 {
    let mut h = blake3::Hasher::new();
    h.update(b"mk-island-synoptic");
    h.update(seed);
    h.update(label.as_bytes());
    h.update(&id.to_le_bytes());
    let b = h.finalize();
    let w = u64::from_le_bytes(b.as_bytes()[..8].try_into().expect("8 bytes"));
    (w >> 11) as f64 / (1u64 << 53) as f64
}

fn centre_latitude(domain: &IslandDomain, y_m: f64) -> f64 {
    domain.lat_lon_at_m(domain.profile().width_m / 2.0, y_m).0
}

impl SynopticState {
    /// A populated domain at `time_s`: ten days of systems already formed
    /// and moving.
    pub fn bootstrap(
        seed: [u8; 32],
        background: &ZonalBackgroundState,
        domain: &IslandDomain,
        canon: &CanonLocked,
    ) -> Self {
        let mut s = Self {
            systems: Vec::new(),
            time_s: 0.0,
            genesis_credit: 0.0,
            next_id: 0,
            mean_rain_ratio: 1.0,
        };
        let mut t = 0.0;
        while t < SPIN_UP_S {
            s.step(background, domain, canon, SPIN_UP_STEP_S, seed);
            t += SPIN_UP_STEP_S;
        }
        s.time_s = 0.0;
        s
    }

    /// Advance by `dt_seconds`: move systems, age them, drop the dead and
    /// the departed, and form new ones at the genesis rate. A zero or
    /// negative step changes nothing.
    pub fn step(
        &mut self,
        background: &ZonalBackgroundState,
        domain: &IslandDomain,
        canon: &CanonLocked,
        dt_seconds: f64,
        seed: [u8; 32],
    ) {
        if !(dt_seconds.is_finite() && dt_seconds > 0.0) {
            return;
        }
        let p = domain.profile();
        let (width, height) = (p.width_m, p.height_m);
        let mid_lat = centre_latitude(domain, height / 2.0);
        let sigma = eady_growth_rate_s(background, mid_lat, canon).max(1e-9);
        let scale = EARTH_EADY_RATE_S / sigma;
        let hemisphere = if mid_lat < 0.0 { -1.0 } else { 1.0 };

        for sys in &mut self.systems {
            let lat = centre_latitude(domain, sys.centre_m.1.clamp(0.0, height));
            let (u, _) = background.wind_at(lat);
            let steering = u + thermal_wind_shear_m_s(background, lat, canon);
            let drift = hemisphere * POLEWARD_DRIFT_M_S;
            let speed = if sys.kind == SystemKind::Front {
                1.1
            } else {
                1.0
            };
            sys.velocity_m_s = (steering * speed, drift);
            sys.centre_m.0 += sys.velocity_m_s.0 * dt_seconds;
            sys.centre_m.1 += sys.velocity_m_s.1 * dt_seconds;
            sys.age_s += dt_seconds;
            let phase = (sys.age_s / sys.lifetime_s).clamp(0.0, 1.0);
            // Deepens, peaks near 40% of life, decays.
            let shape = (std::f64::consts::PI * phase.powf(0.75)).sin().max(0.0);
            sys.central_pressure_pa = sys.peak_pa * shape;
        }
        self.systems.retain(|s| {
            s.age_s < s.lifetime_s
                && s.centre_m.0 > -2.0 * s.radius_m
                && s.centre_m.0 < width + 2.0 * s.radius_m
                && s.centre_m.1 > -2.0 * s.radius_m
                && s.centre_m.1 < height + 2.0 * s.radius_m
        });

        // Genesis: one system per passage interval, at the upwind edge.
        self.genesis_credit += dt_seconds / (REFERENCE_INTERVAL_S * scale);
        while self.genesis_credit >= 1.0 {
            self.genesis_credit -= 1.0;
            let id = self.next_id;
            self.next_id += 3;
            let y = height * (0.2 + 0.6 * uniform(&seed, "y", id));
            let lat = centre_latitude(domain, y);
            let (u, _) = background.wind_at(lat);
            let u = u + thermal_wind_shear_m_s(background, lat, canon);
            let radius = RADIUS_OVER_DEFORMATION * deformation_radius_m(lat, canon);
            let from_west = u >= 0.0;
            let x = if from_west {
                -0.5 * radius
            } else {
                width + 0.5 * radius
            };
            let lifetime = (REFERENCE_LIFETIME_S
                * scale.clamp(0.5, 2.0)
                * (0.4 * (2.0 * uniform(&seed, "life", id) - 1.0)).exp())
            .clamp(2.0 * 86_400.0, 10.0 * 86_400.0);
            let is_low = uniform(&seed, "kind", id) < 0.55;
            let peak = if is_low {
                CYCLONE_DEPTH_PA * (0.5 + uniform(&seed, "depth", id))
            } else {
                ANTICYCLONE_HEIGHT_PA * (0.5 + uniform(&seed, "depth", id))
            };
            // Age the newborn into this step so a long step doesn't make
            // every system appear at the edge at once.
            let make = |kind, id, centre, radius, peak| SynopticSystem {
                id,
                kind,
                centre_m: centre,
                velocity_m_s: (0.0, 0.0),
                central_pressure_pa: 0.0,
                radius_m: radius,
                age_s: 0.0,
                lifetime_s: lifetime,
                peak_pa: peak,
            };
            let kind = if is_low {
                SystemKind::Cyclone
            } else {
                SystemKind::Anticyclone
            };
            self.systems.push(make(kind, id, (x, y), radius, peak));
            if is_low {
                // The cold front trails equatorward and west of its low.
                let toward_equator = -hemisphere;
                let along = if from_west { -0.5 } else { 0.5 };
                self.systems.push(make(
                    SystemKind::Front,
                    id + 1,
                    (x + along * radius, y + toward_equator * 0.9 * radius),
                    1.2 * radius,
                    0.0,
                ));
            }
        }
        self.time_s += dt_seconds;
    }
}

/// Pressure anomaly (Pa) of every coarse cell from the systems.
pub fn pressure_anomaly_pa(state: &SynopticState, domain: &IslandDomain) -> Grid2<f64> {
    let c = DomainLevel::Coarse;
    let (rows, cols) = (domain.rows(c), domain.cols(c));
    let mut data = vec![0.0; rows * cols];
    for sys in state.systems.iter().filter(|s| s.kind != SystemKind::Front) {
        for r in 0..rows {
            for col in 0..cols {
                let (x, y) = domain.cell_center_m(c, r, col);
                let d2 = ((x - sys.centre_m.0).powi(2) + (y - sys.centre_m.1).powi(2))
                    / sys.radius_m.powi(2);
                if d2 < 16.0 {
                    data[r * cols + col] += sys.central_pressure_pa * (-d2).exp();
                }
            }
        }
    }
    Grid2::from_data(&domain.storage_spec(c), data)
}

/// Distance across a front's line (m, signed) and along it (m) from a
/// point. The line runs mostly north-south, tilted 20° to the west of the
/// system's heading, as cold fronts trail their lows.
fn front_frame(sys: &SynopticSystem, x: f64, y: f64) -> (f64, f64) {
    let tilt = 20.0_f64.to_radians();
    let heading = if sys.velocity_m_s.0 >= 0.0 { 1.0 } else { -1.0 };
    // Unit vector along the front (equatorward-poleward axis).
    let along = (-heading * tilt.sin(), tilt.cos());
    let normal = (along.1, -along.0);
    let (dx, dy) = (x - sys.centre_m.0, y - sys.centre_m.1);
    (dx * normal.0 + dy * normal.1, dx * along.0 + dy * along.1)
}

/// Modulate the diagnostic coarse weather by the systems: wind around
/// lows and highs and a wind shift at fronts, rain in lows and fronts and
/// none in highs. The ratio of modulated to unmodulated domain rain is
/// tracked in `state` and divided out, so the long-run mean rain stays the
/// climatology.
pub fn apply_synoptic(
    weather: &mut WeatherState,
    state: &mut SynopticState,
    domain: &IslandDomain,
    canon: &CanonLocked,
    dt_seconds: f64,
) {
    let c = DomainLevel::Coarse;
    let (rows, cols) = (domain.rows(c), domain.cols(c));
    let size = domain.cell_size_m(c);
    let anomaly = pressure_anomaly_pa(state, domain);
    let coriolis = regional_coriolis(domain, c, canon);

    let base_total: f64 = weather.precipitation.data().iter().sum();
    let mut out = weather.precipitation.data().to_vec();
    for r in 0..rows {
        let (rs, rn) = (r.saturating_sub(1), (r + 1).min(rows - 1));
        for col in 0..cols {
            let (cw, ce) = (col.saturating_sub(1), (col + 1).min(cols - 1));
            let idx = r * cols + col;
            let dp_dx =
                (anomaly.get(r, ce) - anomaly.get(r, cw)) / ((ce - cw).max(1) as f64 * size);
            let dp_dy =
                (anomaly.get(rn, col) - anomaly.get(rs, col)) / ((rn - rs).max(1) as f64 * size);
            let f = *coriolis.get(r, col);
            if f.abs() > 1e-6 {
                let k = SURFACE_WIND_FRACTION / (AIR_DENSITY_KG_M3 * f);
                let w = weather.wind.get_mut(r, col);
                w.u_east += (-dp_dy * k).clamp(-30.0, 30.0);
                w.v_north += (dp_dx * k).clamp(-30.0, 30.0);
            }
            let p = *anomaly.get(r, col);
            let mut mult = 1.0;
            if p < 0.0 {
                mult += CYCLONE_RAIN_GAIN * (-p / -CYCLONE_DEPTH_PA).clamp(0.0, 1.0);
            } else {
                mult -= ANTICYCLONE_RAIN_CUT * (p / ANTICYCLONE_HEIGHT_PA).clamp(0.0, 1.0);
            }
            out[idx] *= mult;
            let (x, y) = domain.cell_center_m(c, r, col);
            for sys in state.systems.iter().filter(|s| s.kind == SystemKind::Front) {
                let strength = (sys.age_s / sys.lifetime_s * std::f64::consts::PI)
                    .sin()
                    .max(0.0);
                let (across, along) = front_frame(sys, x, y);
                let width = FRONT_WIDTH_OVER_RADIUS * sys.radius_m;
                if along.abs() < sys.radius_m {
                    let band = (-(across / width).powi(2)).exp();
                    let moisture = (*weather.moisture.get(r, col) / 20.0).clamp(0.0, 2.0);
                    out[idx] += FRONT_RAIN_MM_DAY * strength * band * moisture;
                    // The wind turns across a front: a swing of up to
                    // 8 m/s along the front's normal.
                    let shift = 8.0
                        * strength
                        * (across / width).tanh()
                        * (1.0 - (along / sys.radius_m).powi(2));
                    weather.wind.get_mut(r, col).u_east += shift * 0.94;
                    weather.wind.get_mut(r, col).v_north += shift * 0.34;
                }
            }
        }
    }

    let modulated_total: f64 = out.iter().sum();
    if base_total > 0.0 && dt_seconds > 0.0 {
        let ratio = modulated_total / base_total;
        let gain = 1.0 - (-dt_seconds / NORMALISER_TIME_CONSTANT_S).exp();
        state.mean_rain_ratio += (ratio - state.mean_rain_ratio) * gain;
    }
    let normaliser = state.mean_rain_ratio.max(1e-3);
    for (p, o) in weather.precipitation.data_mut().iter_mut().zip(out) {
        *p = o / normaliser;
    }
}
