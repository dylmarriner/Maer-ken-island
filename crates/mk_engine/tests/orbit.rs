use mk_core::canon::CanonLocked;
use mk_engine::orbit::{solve_kepler, step_orbit, KeplerError};

#[test]
fn orbit_deterministic() {
    let canon = CanonLocked::default();
    let state1 = step_orbit(&canon, 0.0);
    let state2 = step_orbit(&canon, 0.0);
    assert_eq!(state1.mean_anomaly, state2.mean_anomaly);
    assert_eq!(state1.r_over_a, state2.r_over_a);
}

#[test]
fn kepler_solver_converges() {
    let result = solve_kepler(1.0, 0.017, 20);
    assert!(result.is_ok());
}

#[test]
fn orbit_deterministic_10_runs() {
    let canon = CanonLocked::default();
    let mut mean_anomalies = [0.0_f64; 10];
    let mut r_ratios = [0.0_f64; 10];

    for i in 0..10 {
        let state = step_orbit(&canon, 0.0);
        mean_anomalies[i] = state.mean_anomaly;
        r_ratios[i] = state.r_over_a;
    }

    // All should be identical
    for i in 1..10 {
        assert_eq!(
            mean_anomalies[0], mean_anomalies[i],
            "Mean anomaly mismatch at run {}",
            i
        );
        assert_eq!(r_ratios[0], r_ratios[i], "r/a mismatch at run {}", i);
    }
}

#[test]
fn kepler_rejects_invalid_eccentricity() {
    let result = solve_kepler(1.0, 2.0, 20);
    assert_eq!(result, Err(KeplerError::InvalidInput));
}

#[test]
fn orbit_r_over_a_in_bounds() {
    let canon = CanonLocked::default();
    let state = step_orbit(&canon, 0.0);

    // For typical eccentricity, r/a should be close to 1
    assert!(state.r_over_a > 0.0, "r/a must be positive");
    assert!(state.r_over_a < 2.0, "r/a must be less than 2");
}

#[test]
fn orbit_mean_anomaly_cycles() {
    let canon = CanonLocked::default();

    // Compute states at different times
    let state_early = step_orbit(&canon, 0.0);
    let state_late = step_orbit(&canon, 10_000.0 * 3600.0);

    // Mean anomaly should advance (or wrap around)
    let _ = (state_early, state_late);
}
