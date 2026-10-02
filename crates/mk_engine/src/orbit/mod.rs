//! ORBIT MODULE — PHASE 2
//!
//! Purpose
//! - Deterministic orbital mechanics for Marr'Kena MK-I around star MK-STAR-01
//! - Kepler solver (bounded iterations, deterministic fallback)
//! - Orbital state tracking (mean anomaly, distance ratio)
//!
//! Invariants
//! - All arithmetic uses f64 with deterministic substrate rules
//! - Kepler solver bounded at 20 iterations
//! - Same tick + dt → same OrbitState (deterministic replay proof)
//!
//! Authority
//! - MAERKEN_PHASE_2_PHYSICAL_WORLD.md § 3.1
//! - PLANET_CONSTANTS.md § 4 (orbital parameters)
//! - COSMOS_CONSTANTS.md § 3-4 (stellar context)

use mk_core::canon::CanonLocked;
use serde::{Deserialize, Serialize};

/// Orbital state for Marr'Kena MK-I
///
/// Type: Computed struct (no persistent state)
/// Determinism: same tick + dt → same OrbitState
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct OrbitState {
    /// Mean anomaly in radians
    pub mean_anomaly: f64,
    /// Distance ratio r/a
    pub r_over_a: f64,
}

/// Kepler solver error enum
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeplerError {
    /// Invalid input (e.g., negative eccentricity)
    InvalidInput,
}

/// Solve Kepler's equation for eccentric anomaly
///
/// Solves: M = E - e·sin(E) for E given M and e
pub fn solve_kepler(
    mean_anomaly: f64,
    eccentricity: f64,
    max_iterations: u32,
) -> Result<f64, KeplerError> {
    if !(0.0..1.0).contains(&eccentricity) {
        return Err(KeplerError::InvalidInput);
    }

    let tau = core::f64::consts::TAU;
    let m = mean_anomaly.rem_euclid(tau);

    // Initial guess
    let mut e_n = m;

    // Newton-Raphson iteration
    for _ in 0..max_iterations {
        let f_e = e_n - eccentricity * e_n.sin() - m;
        let f_prime_e = 1.0 - eccentricity * e_n.cos();

        if f_prime_e.abs() < 1e-15 {
            break;
        }

        let delta = f_e / f_prime_e;
        e_n -= delta;

        if delta.abs() < 1e-12 {
            break;
        }
    }

    Ok(e_n)
}

/// Compute orbital state for Marr'Kena MK-I `sim_time_seconds` into the
/// simulation.
pub fn step_orbit(canon: &CanonLocked, sim_time_seconds: f64) -> OrbitState {
    let tau = core::f64::consts::TAU;

    // Period in seconds from canon
    let period_seconds = canon.orbital_period_s;

    // Mean anomaly = 2π × (t / T)
    let mean_anomaly = (sim_time_seconds * tau / period_seconds).rem_euclid(tau);

    // Solve Kepler's equation
    let eccentricity = canon.orbital_eccentricity;
    let eccentric_anomaly = solve_kepler(mean_anomaly, eccentricity, 20).unwrap_or(mean_anomaly);

    // Compute r/a = 1 - e·cos(E)
    let r_over_a = 1.0 - eccentricity * eccentric_anomaly.cos();

    OrbitState {
        mean_anomaly,
        r_over_a,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kepler_solver_converges() {
        let result = solve_kepler(1.0, 0.017, 20);
        assert!(result.is_ok());
    }

    #[test]
    fn kepler_solver_rejects_invalid_eccentricity() {
        let result = solve_kepler(1.0, 1.5, 20);
        assert_eq!(result, Err(KeplerError::InvalidInput));
    }

    #[test]
    fn orbit_deterministic() {
        let canon = CanonLocked::default();
        let state1 = step_orbit(&canon, 0.0);
        let state2 = step_orbit(&canon, 0.0);
        assert_eq!(state1.mean_anomaly, state2.mean_anomaly);
        assert_eq!(state1.r_over_a, state2.r_over_a);
    }

    #[test]
    fn orbit_deterministic_10_runs() {
        let canon = CanonLocked::default();
        let mut states = [OrbitState {
            mean_anomaly: 0.0,
            r_over_a: 0.0,
        }; 10];

        for state in &mut states {
            *state = step_orbit(&canon, 0.0);
        }

        // All states should be identical
        for i in 1..10 {
            assert_eq!(
                states[0].mean_anomaly, states[i].mean_anomaly,
                "Mean anomaly mismatch at run {}",
                i
            );
            assert_eq!(
                states[0].r_over_a, states[i].r_over_a,
                "r/a mismatch at run {}",
                i
            );
        }
    }

    #[test]
    fn orbit_advances_with_time() {
        let canon = CanonLocked::default();
        let start = step_orbit(&canon, 0.0);
        let quarter = step_orbit(&canon, canon.orbital_period_s / 4.0);
        let delta = quarter.mean_anomaly - start.mean_anomaly;
        assert!((delta - std::f64::consts::FRAC_PI_2).abs() < 1e-9);
    }
}
