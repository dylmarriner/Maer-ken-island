//! TIDES MODULE — PHASE 2
//!
//! Purpose
//! - Deterministic tidal forcing for Marr'Kena MK-I
//! - Multi-moon tidal system (Mckenz + Hahn)
//! - Degree-2 tidal potential and equilibrium tide height about each
//!   moon's planet-fixed sub-lunar point, and entrainment signals
//! - Ledger-tracked tidal dissipation from the tidal torque, over the real step length
//!
//! Invariants
//! - All arithmetic uses f64 with deterministic substrate rules
//! - Tidal potential bounded (no unbounded growth)
//! - Same simulated time + dt → same TidalState (deterministic)
//! - All tidal heating must be ledger-tracked
//!
//! Authority
//! - MAERKEN_PHASE_2_PHYSICAL_WORLD.md § 3.4
//! - PLANET_CONSTANTS.md § 6 (moon system constants)

use mk_core::canon::CanonLocked;
use mk_core::flux::{FluxEntry, FluxKind, Ledger, Reservoir};
use mk_core::grid::Grid2;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Newtonian gravitational constant, m³ kg⁻¹ s⁻².
const GRAVITATIONAL_CONSTANT: f64 = 6.674_30e-11;
/// Degree-2 tidal Love number of the planet. Earth-analogue value; the canon
/// carries no interior model.
pub const LOVE_NUMBER_K2: f64 = 0.30;
/// Effective tidal quality factor. With `LOVE_NUMBER_K2` this reproduces the
/// ~3 TW of present-day Earth-Moon tidal dissipation.
pub const TIDAL_QUALITY_FACTOR: f64 = 12.0;

/// Tidal state for Marr'Kena MK-I
///
/// Type: Computed struct (no persistent state)
/// Determinism: same simulated time + dt → same TidalState
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TidalState {
    /// Degree-2 tide-generating potential per cell, normalised so the two
    /// moons' weights sum to 1: `Σ wᵢ P₂(cos ψᵢ)`, in `[-0.5, 1]`, where `ψᵢ`
    /// is the angular distance from moon *i*'s sub-lunar point.
    pub potential: Grid2<f64>,
    /// Day-cycle tidal entrainment signal
    pub entrainment_day: Grid2<f64>,
    /// Hahn-cycle tidal entrainment signal
    pub entrainment_hahn: Grid2<f64>,
    /// Mckenz-cycle tidal entrainment signal
    pub entrainment_mckenz: Grid2<f64>,
    /// Equilibrium tide height per cell, m: `Σ (Mᵢ/M)(R⁴/aᵢ³) P₂(cos ψᵢ)`.
    #[serde(default)]
    pub equilibrium_height_m: Vec<f64>,
    /// Tidal dissipation power this step, W (see [`tidal_dissipation_power_w`]).
    #[serde(default)]
    pub heating_power_w: f64,
}

impl TidalState {
    /// Tidal dissipation power, W.
    pub fn get_tidal_heating_rate(&self) -> f64 {
        self.heating_power_w
    }
}

/// Planet-fixed longitude (rad) of a moon's sub-lunar point. The moon's
/// inertial orbital angle is `2π t / P_moon`; the planet has rotated by
/// `ω t`, the same `rotation_angle` convention `crate::rotation` uses (the
/// subsolar point sits at `−ω t`).
pub fn sub_lunar_longitude(time_s: f64, moon_period_s: f64, rotation_period_s: f64) -> f64 {
    let orbit = std::f64::consts::TAU * (time_s / moon_period_s).rem_euclid(1.0);
    let rotation = std::f64::consts::TAU * (time_s / rotation_period_s).rem_euclid(1.0);
    (orbit - rotation).rem_euclid(std::f64::consts::TAU)
}

/// Degree-2 Legendre polynomial of the cosine of the angular distance from
/// the sub-lunar point, for a moon in the equatorial plane:
/// `cos ψ = cos φ cos(λ − λ_moon)`.
pub fn degree_two_potential(lat_rad: f64, lon_rad: f64, sub_lunar_lon_rad: f64) -> f64 {
    let cos_psi = lat_rad.cos() * (lon_rad - sub_lunar_lon_rad).cos();
    0.5 * (3.0 * cos_psi * cos_psi - 1.0)
}

/// Tidal dissipation raised in the planet by one moon, W: the tidal torque
/// `Γ = (3/2)(k₂/Q) G M_moon² R⁵ / a⁶` times the rate the bulge is dragged
/// relative to the moon, `|Ω − n|`.
pub fn tidal_dissipation_power_w(
    moon_mass_kg: f64,
    semi_major_axis_m: f64,
    planet_radius_m: f64,
    rotation_rate_rad_s: f64,
    mean_motion_rad_s: f64,
) -> f64 {
    let torque = 1.5
        * (LOVE_NUMBER_K2 / TIDAL_QUALITY_FACTOR)
        * GRAVITATIONAL_CONSTANT
        * moon_mass_kg
        * moon_mass_kg
        * planet_radius_m.powi(5)
        / semi_major_axis_m.powi(6);
    torque * (rotation_rate_rad_s - mean_motion_rad_s).abs()
}

/// Compute tidal state for Marr'Kena MK-I at `sim_time_seconds` of simulated time
pub fn step_tides(
    canon: &Arc<CanonLocked>,
    sim_time_seconds: f64,
    dt_seconds: u64,
    grid_spec: &mk_core::grid::GridSpec,
    ledger: &mut Ledger,
) -> TidalState {
    let tau = std::f64::consts::TAU;
    let mckenz_period_seconds = canon.moon_mckenz_period_s;
    let hahn_period_seconds = canon.moon_hahn_period_s;
    let mckenz_phase = (sim_time_seconds / mckenz_period_seconds).rem_euclid(1.0);
    let hahn_phase = (sim_time_seconds / hahn_period_seconds).rem_euclid(1.0);
    let mckenz_lon = sub_lunar_longitude(
        sim_time_seconds,
        mckenz_period_seconds,
        canon.rotation_period_s,
    );
    let hahn_lon = sub_lunar_longitude(
        sim_time_seconds,
        hahn_period_seconds,
        canon.rotation_period_s,
    );

    let radius = canon.planet_radius_m;
    let mckenz_mass_kg = canon.planet_mass_kg * canon.moon_mckenz_mass_ratio;
    let hahn_mass_kg = canon.planet_mass_kg * canon.moon_hahn_mass_ratio;
    // Equilibrium tide amplitude (Mᵢ/M)(R⁴/aᵢ³): the tide-generating
    // potential G Mᵢ R² / aᵢ³ divided by surface gravity G M / R².
    let mckenz_amplitude_m =
        canon.moon_mckenz_mass_ratio * radius.powi(4) / canon.mckenz_semi_major_axis_m.powi(3);
    let hahn_amplitude_m =
        canon.moon_hahn_mass_ratio * radius.powi(4) / canon.hahn_semi_major_axis_m.powi(3);
    let combined_amplitude = mckenz_amplitude_m + hahn_amplitude_m;
    let (mckenz_weight, hahn_weight) = if combined_amplitude > 0.0 {
        (
            mckenz_amplitude_m / combined_amplitude,
            hahn_amplitude_m / combined_amplitude,
        )
    } else {
        (0.0, 0.0)
    };

    let cells = grid_spec.nlat * grid_spec.nlon;
    let mut potential_data = Vec::with_capacity(cells);
    let mut height_m = Vec::with_capacity(cells);
    for ilat in 0..grid_spec.nlat {
        let lat_rad = grid_spec.lat_rad(ilat);
        for ilon in 0..grid_spec.nlon {
            let lon_rad = grid_spec.lon_rad(ilon);
            let mckenz = degree_two_potential(lat_rad, lon_rad, mckenz_lon);
            let hahn = degree_two_potential(lat_rad, lon_rad, hahn_lon);
            potential_data.push(mckenz_weight * mckenz + hahn_weight * hahn);
            height_m.push(mckenz_amplitude_m * mckenz + hahn_amplitude_m * hahn);
        }
    }
    let potential_grid = Grid2::from_data(grid_spec, potential_data);

    // Entrainment signals: the phase of each cycle, uniform over the planet.
    let day_signal = ((sim_time_seconds / canon.rotation_period_s).rem_euclid(1.0) * tau).cos();
    let hahn_signal = (hahn_phase * tau).cos();
    let mckenz_signal = (mckenz_phase * tau).cos();
    let entrainment_day = Grid2::from_data(grid_spec, vec![day_signal; cells]);
    let entrainment_hahn = Grid2::from_data(grid_spec, vec![hahn_signal; cells]);
    let entrainment_mckenz = Grid2::from_data(grid_spec, vec![mckenz_signal; cells]);

    // Tidal dissipation from each moon's torque, integrated over the step
    // and ledger-tracked from the tidal reservoir into ocean heat.
    let rotation_rate = tau / canon.rotation_period_s;
    let heating_power_w = tidal_dissipation_power_w(
        mckenz_mass_kg,
        canon.mckenz_semi_major_axis_m,
        radius,
        rotation_rate,
        tau / mckenz_period_seconds,
    ) + tidal_dissipation_power_w(
        hahn_mass_kg,
        canon.hahn_semi_major_axis_m,
        radius,
        rotation_rate,
        tau / hahn_period_seconds,
    );
    ledger.push(FluxEntry {
        source: Reservoir::TidalHeat,
        sink: Reservoir::OceanHeat,
        amount: heating_power_w * dt_seconds as f64,
        kind: FluxKind::Energy,
    });

    TidalState {
        potential: potential_grid,
        entrainment_day,
        entrainment_hahn,
        entrainment_mckenz,
        equilibrium_height_m: height_m,
        heating_power_w,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tides_deterministic() {
        let canon = Arc::new(CanonLocked::default());
        let grid_spec = mk_core::grid::GridSpec { nlat: 18, nlon: 36 };
        let mut ledger1 = Ledger::new();
        let mut ledger2 = Ledger::new();

        let state1 = step_tides(&canon, 0.0, 1, &grid_spec, &mut ledger1);
        let state2 = step_tides(&canon, 0.0, 1, &grid_spec, &mut ledger2);

        // Compare potentials
        assert_eq!(state1.potential.data(), state2.potential.data());
    }

    #[test]
    fn tides_ledger_entry() {
        let canon = Arc::new(CanonLocked::default());
        let grid_spec = mk_core::grid::GridSpec { nlat: 18, nlon: 36 };
        let mut ledger = Ledger::new();

        step_tides(&canon, 0.0, 1, &grid_spec, &mut ledger);

        assert!(
            !ledger.is_empty(),
            "Ledger must contain tidal heating entry"
        );
        let entries = ledger.entries();
        assert!(!entries.is_empty());
        let entry = &entries[0];
        assert_eq!(entry.kind, FluxKind::Energy);
        assert_eq!(entry.source, Reservoir::TidalHeat);
        assert_eq!(entry.sink, Reservoir::OceanHeat);
    }

    #[test]
    fn potential_bounded() {
        let canon = Arc::new(CanonLocked::default());
        let grid_spec = mk_core::grid::GridSpec { nlat: 18, nlon: 36 };
        let mut ledger = Ledger::new();

        let state = step_tides(&canon, 0.0, 1, &grid_spec, &mut ledger);

        // All potential values should be bounded by the normalized dual-moon forcing envelope.
        for &val in state.potential.data() {
            assert!(
                (-1.0..=1.0).contains(&val),
                "Potential out of bounds: {}",
                val
            );
        }
    }

    #[test]
    fn potential_has_two_bulges_under_and_opposite_the_moon() {
        let sub = 1.0;
        let under = degree_two_potential(0.0, sub, sub);
        let opposite = degree_two_potential(0.0, sub + std::f64::consts::PI, sub);
        let quadrature = degree_two_potential(0.0, sub + std::f64::consts::FRAC_PI_2, sub);
        assert!((under - 1.0).abs() < 1e-12 && (opposite - 1.0).abs() < 1e-12);
        assert!((quadrature + 0.5).abs() < 1e-12);
    }

    #[test]
    fn sub_lunar_point_is_carried_by_planet_rotation() {
        // Half a rotation later, with the moon barely moved, the sub-lunar
        // point has swept half-way round the planet: a semi-diurnal tide.
        let rotation = 129_600.0;
        let moon = 3_888_000.0;
        let start = sub_lunar_longitude(0.0, moon, rotation);
        let later = sub_lunar_longitude(rotation / 2.0, moon, rotation);
        let swept = (start - later).rem_euclid(std::f64::consts::TAU);
        // The moon's own orbital motion over that time slightly shortens it.
        let moon_advance = std::f64::consts::TAU * (rotation / 2.0) / moon;
        let expected = std::f64::consts::PI - moon_advance;
        assert!(
            (swept - expected).abs() < 1e-9,
            "swept {swept}, expected {expected}"
        );
    }

    #[test]
    fn dissipation_matches_earth_moon_and_scales_with_step() {
        let earth_moon = tidal_dissipation_power_w(
            7.342e22,
            3.844e8,
            6.371e6,
            7.2921e-5,
            std::f64::consts::TAU / (27.3217 * 86_400.0),
        );
        // Observed lunar tidal dissipation is ~3.2 TW.
        assert!((2.5e12..4.0e12).contains(&earth_moon), "{earth_moon} W");

        let canon = Arc::new(CanonLocked::default());
        let grid_spec = mk_core::grid::GridSpec { nlat: 18, nlon: 36 };
        let (mut one, mut ten) = (Ledger::new(), Ledger::new());
        let state = step_tides(&canon, 0.0, 1, &grid_spec, &mut one);
        step_tides(&canon, 0.0, 10, &grid_spec, &mut ten);
        let joules = |ledger: &Ledger| ledger.entries()[0].amount;
        assert!((joules(&ten) - 10.0 * joules(&one)).abs() <= 1e-9 * joules(&ten));
        assert!((joules(&one) - state.heating_power_w).abs() <= 1e-9 * joules(&one));
        assert_eq!(state.equilibrium_height_m.len(), 18 * 36);
    }
}
