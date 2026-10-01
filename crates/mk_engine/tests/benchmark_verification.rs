use mk_engine::long_horizon_verification::{
    LongHorizonVerifier, VerificationConfig, VerificationHorizon,
};

#[test]
fn test_verification_horizon_1kyr_benchmark() {
    let config = VerificationConfig {
        horizon: VerificationHorizon::Kyr1,
        initial_seed: [123u8; 32],
        sample_interval_years: 100.0,
        checkpoint_interval_years: 1000.0,
        enable_determinism_tests: true,
        enable_conservation_tracking: true,
        world_step_mode: Default::default(),
    };

    let verifier = LongHorizonVerifier::new(config);
    let results = verifier.run_verification();

    assert!(results.time_series.len() >= 10);
}
