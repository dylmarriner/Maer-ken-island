//! Phase 6 long-horizon verification integration tests.
//!
//! Full 100 kyr horizon runs step the world on the order of tens of millions of
//! daily ticks and routinely exceed CI and short heartbeat budgets.
//!
//! **Default** `cargo test -p mk_engine --test phase6_verification` runs only the fast
//! observables unit check. To run the full integration suite locally or in nightly jobs:
//!
//! ```text
//! cargo test -p mk_engine --test phase6_verification -- --ignored
//! ```
use mk_engine::biosphere::genetics;
use mk_engine::long_horizon_verification::{
    generate_final_report, LongHorizonVerifier, VerificationConfig, VerificationHorizon,
    VerificationObservables,
};

#[test]
#[ignore = "full 100 kyr world-step integration; run with: cargo test -p mk_engine --test phase6_verification -- --ignored"]
fn test_verification_horizon_100kyr() {
    let config = VerificationConfig {
        horizon: VerificationHorizon::Kyr100,
        initial_seed: [123u8; 32],
        sample_interval_years: 1000.0,
        checkpoint_interval_years: 10000.0,
        enable_determinism_tests: true,
        enable_conservation_tracking: true,
        ..Default::default()
    };

    let verifier = LongHorizonVerifier::new(config);
    let results = verifier.run_verification();

    // Verify basic requirements
    assert!(
        !results.time_series.is_empty(),
        "Time series should not be empty"
    );
    assert!(
        results.time_series.len() > 10,
        "Should have multiple time points"
    );

    // Verify time progression
    let final_time = results.time_series.last().unwrap().time_years;
    assert!(final_time >= 100000.0, "Should reach 100kyr");

    // Verify observables
    assert!(
        results.observables.biomass_variance > 0.0,
        "Biomass variance should be > 0"
    );
    assert!(
        results.observables.intelligence_variance > 0.0,
        "Intelligence variance should be > 0"
    );
    assert!(
        results.observables.intelligence_max <= genetics::PRE_SAPIENT_CEILING,
        "Intelligence should not exceed ceiling"
    );

    println!("✅ 100kyr Verification Test Passed");
    println!("  Final time: {:.0} years", final_time);
    println!("  Time points: {}", results.time_series.len());
    println!(
        "  Biomass variance: {:.6}",
        results.observables.biomass_variance
    );
    println!(
        "  Intelligence variance: {:.6}",
        results.observables.intelligence_variance
    );
    println!(
        "  Max intelligence: {:.3}",
        results.observables.intelligence_max
    );
    println!("  Extinctions: {}", results.observables.extinction_count);
    println!("  Apex crashes: {}", results.observables.apex_crash_count);
}

#[test]
fn test_verification_observables_validation() {
    let mut observables = VerificationObservables::new();

    // Test invalid observables (should fail validation)
    observables.biomass_variance = 0.0; // Invalid: must be > 0
    observables.intelligence_variance = 0.0; // Invalid: must be > 0
    observables.extinction_count = 0; // Invalid: must be > 0
    observables.apex_crash_count = 0; // Invalid: must be > 0

    let issues = observables.validate_mandatory_observables();
    assert!(!issues.is_empty(), "Should detect validation issues");
    assert!(issues.len() >= 4, "Should detect multiple issues");

    // Fix observables
    observables.biomass_variance = 0.1;
    observables.intelligence_variance = 0.05;
    observables.extinction_count = 1;
    observables.apex_crash_count = 1;
    observables.intelligence_max = 0.3; // Below ceiling

    let issues = observables.validate_mandatory_observables();
    assert!(
        issues.is_empty(),
        "Should pass validation when all requirements met"
    );

    println!("✅ Observables Validation Test Passed");
}

#[test]
#[ignore = "full 100 kyr world-step integration; run with: cargo test -p mk_engine --test phase6_verification -- --ignored"]
fn test_determinism_requirements() {
    let config = VerificationConfig {
        horizon: VerificationHorizon::Kyr100,
        initial_seed: [45u8; 32],
        sample_interval_years: 500.0,
        checkpoint_interval_years: 5000.0,
        enable_determinism_tests: true,
        enable_conservation_tracking: true,
        ..Default::default()
    };

    let verifier = LongHorizonVerifier::new(config);
    let results = verifier.run_verification();

    // Check hash continuity
    assert!(
        results.determinism_proof.hash_continuity_verified,
        "Hash continuity should be verified"
    );

    // Check initial and final hashes are different
    assert_ne!(
        results.determinism_proof.initial_hash, results.determinism_proof.final_hash,
        "Initial and final hashes should be different"
    );

    // Check checkpoint hashes exist
    assert!(
        !results.determinism_proof.checkpoint_hashes.is_empty(),
        "Should have checkpoint hashes"
    );

    println!("✅ Determinism Requirements Test Passed");
    println!(
        "  Initial hash: {:?}",
        results.determinism_proof.initial_hash
    );
    println!("  Final hash: {:?}", results.determinism_proof.final_hash);
    println!(
        "  Checkpoints: {}",
        results.determinism_proof.checkpoint_hashes.len()
    );
}

#[test]
#[ignore = "full 100 kyr world-step integration; run with: cargo test -p mk_engine --test phase6_verification -- --ignored"]
fn test_conservation_drift_bounds() {
    let config = VerificationConfig {
        horizon: VerificationHorizon::Kyr100,
        initial_seed: [78u8; 32],
        sample_interval_years: 2000.0,
        checkpoint_interval_years: 20000.0,
        enable_determinism_tests: true,
        enable_conservation_tracking: true,
        ..Default::default()
    };

    let verifier = LongHorizonVerifier::new(config);
    let results = verifier.run_verification();

    // Check conservation drift is bounded
    let drift_threshold = 0.01; // 1%
    assert!(
        results.observables.conservation_drift.energy_drift.abs() <= drift_threshold,
        "Energy drift should be bounded: {:.6}",
        results.observables.conservation_drift.energy_drift
    );
    assert!(
        results.observables.conservation_drift.water_drift.abs() <= drift_threshold,
        "Water drift should be bounded: {:.6}",
        results.observables.conservation_drift.water_drift
    );
    assert!(
        results.observables.conservation_drift.carbon_drift.abs() <= drift_threshold,
        "Carbon drift should be bounded: {:.6}",
        results.observables.conservation_drift.carbon_drift
    );

    println!("✅ Conservation Drift Bounds Test Passed");
    println!(
        "  Energy drift: {:.6}",
        results.observables.conservation_drift.energy_drift
    );
    println!(
        "  Water drift: {:.6}",
        results.observables.conservation_drift.water_drift
    );
    println!(
        "  Carbon drift: {:.6}",
        results.observables.conservation_drift.carbon_drift
    );
}

#[test]
#[ignore = "full 100 kyr world-step integration; run with: cargo test -p mk_engine --test phase6_verification -- --ignored"]
fn test_equilibrium_lock_detection() {
    let config = VerificationConfig {
        horizon: VerificationHorizon::Kyr100,
        initial_seed: [101u8; 32],
        sample_interval_years: 100.0,
        checkpoint_interval_years: 10000.0,
        enable_determinism_tests: false, // Skip for faster test
        enable_conservation_tracking: true,
        ..Default::default()
    };

    let verifier = LongHorizonVerifier::new(config);
    let results = verifier.run_verification();

    // System should show sufficient variability (no equilibrium lock)
    assert!(
        !results.observables.equilibrium_lock_detected,
        "Should not detect equilibrium lock in dynamic system"
    );

    // Should have sufficient variance
    assert!(
        results.observables.biomass_variance > 0.001,
        "Should have measurable biomass variance"
    );
    assert!(
        results.observables.intelligence_variance > 0.0001,
        "Should have measurable intelligence variance"
    );

    println!("✅ Equilibrium Lock Detection Test Passed");
    println!(
        "  Equilibrium lock detected: {}",
        results.observables.equilibrium_lock_detected
    );
    println!(
        "  Biomass variance: {:.6}",
        results.observables.biomass_variance
    );
    println!(
        "  Intelligence variance: {:.6}",
        results.observables.intelligence_variance
    );
}

#[test]
#[ignore = "full 100 kyr world-step integration; run with: cargo test -p mk_engine --test phase6_verification -- --ignored"]
fn test_sapience_ceiling_enforcement() {
    let config = VerificationConfig {
        horizon: VerificationHorizon::Kyr100,
        initial_seed: [131u8; 32],
        sample_interval_years: 500.0,
        checkpoint_interval_years: 10000.0,
        enable_determinism_tests: true,
        enable_conservation_tracking: true,
        ..Default::default()
    };

    let verifier = LongHorizonVerifier::new(config);
    let results = verifier.run_verification();

    // Should never breach sapience ceiling
    assert!(
        !results.observables.sapience_breach_detected,
        "Should never breach sapience ceiling"
    );
    assert!(
        results.observables.intelligence_max <= genetics::PRE_SAPIENT_CEILING,
        "Max intelligence should not exceed ceiling"
    );

    // Check all time points respect ceiling
    for point in &results.time_series {
        assert!(
            point.intelligence_max <= genetics::PRE_SAPIENT_CEILING,
            "Intelligence at time {:.0} exceeds ceiling: {:.3}",
            point.time_years,
            point.intelligence_max
        );
    }

    println!("✅ Sapience Ceiling Enforcement Test Passed");
    println!(
        "  Sapience breach detected: {}",
        results.observables.sapience_breach_detected
    );
    println!(
        "  Max intelligence: {:.3}",
        results.observables.intelligence_max
    );
    println!("  Ceiling: {:.3}", genetics::PRE_SAPIENT_CEILING);
}

#[test]
#[ignore = "full 100 kyr world-step integration; run with: cargo test -p mk_engine --test phase6_verification -- --ignored"]
fn test_all_horizons_integration() {
    // This test runs all horizons but with reduced duration for testing
    println!("Running integrated Phase 6 verification test...");

    let horizons = vec![
        VerificationHorizon::Kyr100,
        VerificationHorizon::Myr1,
        // Note: Myr10 still excluded but potentially runnable in nightly
    ];

    for horizon in horizons {
        let config = VerificationConfig {
            horizon,
            initial_seed: [161u8; 32],
            sample_interval_years: match horizon {
                VerificationHorizon::Kyr1 => 10.0,
                VerificationHorizon::Kyr100 => 1000.0,
                VerificationHorizon::Myr1 => 5000.0,
                VerificationHorizon::Myr10 => 50000.0,
            },
            checkpoint_interval_years: match horizon {
                VerificationHorizon::Kyr1 => 100.0,
                VerificationHorizon::Kyr100 => 10000.0,
                VerificationHorizon::Myr1 => 50000.0,
                VerificationHorizon::Myr10 => 500000.0,
            },
            enable_determinism_tests: true,
            enable_conservation_tracking: true,
            ..Default::default()
        };

        let verifier = LongHorizonVerifier::new(config);
        let results = verifier.run_verification();

        // Verify Phase 6 exit gates for this horizon
        let validation_issues = results.observables.validate_mandatory_observables();

        if validation_issues.is_empty() && results.errors.is_empty() {
            println!(
                "✅ {} horizon passed all Phase 6 requirements",
                horizon.name()
            );
        } else {
            println!("❌ {} horizon had issues:", horizon.name());
            for issue in validation_issues {
                println!("  - {}", issue);
            }
            for error in &results.errors {
                println!("  - {}", error);
            }
        }

        // Basic sanity checks
        assert!(
            !results.time_series.is_empty(),
            "Time series should not be empty"
        );
        assert!(
            results.observables.biomass_variance > 0.0,
            "Should have biomass variance"
        );
        assert!(
            results.observables.intelligence_variance > 0.0,
            "Should have intelligence variance"
        );
    }

    println!("✅ All Horizons Integration Test Passed");
}

#[test]
#[ignore = "full 100 kyr world-step integration; run with: cargo test -p mk_engine --test phase6_verification -- --ignored"]
fn test_phase6_exit_gates() {
    let config = VerificationConfig {
        horizon: VerificationHorizon::Kyr100,
        initial_seed: [192u8; 32],
        sample_interval_years: 800.0,
        checkpoint_interval_years: 8000.0,
        enable_determinism_tests: true,
        enable_conservation_tracking: true,
        ..Default::default()
    };

    let verifier = LongHorizonVerifier::new(config);
    let results = verifier.run_verification();

    // Phase 6 Exit Gate Verification

    // 1. All verification horizons pass (for this test horizon)
    let validation_issues = results.observables.validate_mandatory_observables();
    assert!(
        validation_issues.is_empty(),
        "All mandatory observables should be satisfied"
    );
    assert!(results.errors.is_empty(), "Should have no errors");

    // 2. Deterministic final hashes reproduce
    assert!(
        results.determinism_proof.hash_continuity_verified,
        "Hash continuity should be verified"
    );
    assert_ne!(
        results.determinism_proof.initial_hash, results.determinism_proof.final_hash,
        "Should have different initial and final hashes"
    );

    // 3. Conservation drift remains bounded
    let drift_threshold = 0.01;
    assert!(
        results.observables.conservation_drift.energy_drift.abs() <= drift_threshold,
        "Energy drift bounded"
    );
    assert!(
        results.observables.conservation_drift.water_drift.abs() <= drift_threshold,
        "Water drift bounded"
    );
    assert!(
        results.observables.conservation_drift.carbon_drift.abs() <= drift_threshold,
        "Carbon drift bounded"
    );

    // 4. World remains unstable enough to be alive, but never sapient by accident
    assert!(
        results.observables.biomass_variance > 0.0,
        "World should be unstable (biomass variance)"
    );
    assert!(
        results.observables.intelligence_variance > 0.0,
        "World should be unstable (intelligence variance)"
    );
    assert!(
        !results.observables.equilibrium_lock_detected,
        "Should not be in equilibrium lock"
    );
    assert!(
        !results.observables.sapience_breach_detected,
        "Should never become sapient"
    );
    assert!(
        results.observables.intelligence_max <= genetics::PRE_SAPIENT_CEILING,
        "Intelligence ceiling enforced"
    );

    println!("✅ Phase 6 Exit Gates Test Passed");
    println!("  All mandatory observables: ✅");
    println!("  Deterministic hashes: ✅");
    println!("  Bounded conservation drift: ✅");
    println!("  Unstable but non-sapient: ✅");
}

#[test]
#[ignore = "full 100 kyr world-step integration; run with: cargo test -p mk_engine --test phase6_verification -- --ignored"]
fn test_verification_report_generation() {
    // Run a quick verification
    let config = VerificationConfig {
        horizon: VerificationHorizon::Kyr100,
        initial_seed: [222u8; 32],
        sample_interval_years: 2000.0,
        checkpoint_interval_years: 20000.0,
        enable_determinism_tests: true,
        enable_conservation_tracking: true,
        ..Default::default()
    };

    let verifier = LongHorizonVerifier::new(config);
    let results = verifier.run_verification();

    // Generate report
    let report = generate_final_report(&[results]);

    // Verify report structure
    assert!(report.contains("MARR'KENA PHASE 6 LONG-HORIZON VERIFICATION REPORT"));
    assert!(report.contains("100kyr Verification"));
    assert!(report.contains("Observables"));
    assert!(report.contains("Conservation Drift"));
    assert!(report.contains("Determinism"));
    assert!(report.contains("Overall Assessment"));

    println!("✅ Verification Report Generation Test Passed");
    println!("  Report length: {} characters", report.len());
    println!("  Report preview:\n{}", &report[..report.len().min(500)]);
}
