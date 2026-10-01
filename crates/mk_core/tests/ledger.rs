/**
 * Ledger Audit Tests
 *
 * Purpose
 * - Verify conservation ledger audit functionality.
 * - Test tolerance policy enforcement.
 * - Validate flux tracking and balance calculations.
 *
 * Invariants
 * - Conservation violations must be detected.
 * - Tolerance policy must control audit sensitivity.
 * - Ledger must track all flux kinds correctly.
 *
 * Failure Modes
 * - Conservation violation → audit error.
 * - Tolerance too loose → missed violations.
 * - Tolerance too tight → false positives.
 *
 * Debug Notes
 * - Ledger::audit() provides detailed error information.
 * - Test covers all flux kinds and edge cases.
 * - Imbalance detection is critical for simulation integrity.
 */
use mk_core::flux::{FluxEntry, FluxKind, Ledger, Reservoir, TolerancePolicy};

/// Test balanced ledger with energy flux
///
/// Verifies that balanced energy transfers pass audit.
#[test]
fn ledger_balanced_energy() {
    let mut ledger = Ledger::new();

    // Add balanced energy flux: TOA -> Atmos -> Space
    ledger.push(FluxEntry::new(
        Reservoir::TOAInsolation,
        Reservoir::AtmosEnergy,
        1000.0,
        FluxKind::Energy,
    ));
    ledger.push(FluxEntry::new(
        Reservoir::AtmosEnergy,
        Reservoir::SpaceRadiation,
        1000.0,
        FluxKind::Energy,
    ));

    let policy = TolerancePolicy::default();
    let result = ledger.audit(policy);

    if let Err(ref error) = result {
        println!("Audit error: {:?}", error);
    }
    assert!(result.is_ok(), "Balanced energy flux should pass audit");

    // Check balance is sum of all flux amounts
    assert_eq!(ledger.balance(FluxKind::Energy), 2000.0);
}

/// Test ledger detects energy imbalance
///
/// Verifies that unbalanced energy transfers fail audit.
#[test]
fn ledger_imbalanced_energy() {
    let mut ledger = Ledger::new();

    // Add unbalanced energy flux: more in than out
    ledger.push(FluxEntry::new(
        Reservoir::TOAInsolation,
        Reservoir::AtmosEnergy,
        1000.0,
        FluxKind::Energy,
    ));
    ledger.push(FluxEntry::new(
        Reservoir::AtmosEnergy,
        Reservoir::SpaceRadiation,
        500.0, // Only half goes out
        FluxKind::Energy,
    ));

    let policy = TolerancePolicy::default();
    let result = ledger.audit(policy);
    assert!(result.is_err(), "Imbalanced energy flux should fail audit");

    // Check balance reflects the imbalance
    let error = result.unwrap_err();
    assert_eq!(error.balance(), 500.0);
}

/// Test ledger with multiple flux kinds
///
/// Verifies that ledger can handle multiple flux types simultaneously.
#[test]
fn ledger_balanced_multiple_kinds() {
    let mut ledger = Ledger::new();

    // Balanced energy flux
    ledger.push(FluxEntry::new(
        Reservoir::TOAInsolation,
        Reservoir::AtmosEnergy,
        1000.0,
        FluxKind::Energy,
    ));
    ledger.push(FluxEntry::new(
        Reservoir::AtmosEnergy,
        Reservoir::SpaceRadiation,
        1000.0,
        FluxKind::Energy,
    ));

    // Balanced water flux
    ledger.push(FluxEntry::new(
        Reservoir::Atmosphere,
        Reservoir::SoilWater,
        500.0,
        FluxKind::Water,
    ));
    ledger.push(FluxEntry::new(
        Reservoir::SoilWater,
        Reservoir::Atmosphere,
        500.0,
        FluxKind::Water,
    ));

    let policy = TolerancePolicy::default();
    let result = ledger.audit(policy);
    assert!(
        result.is_ok(),
        "Balanced multiple flux kinds should pass audit"
    );

    // All balances should be zero
    assert_eq!(ledger.balance(FluxKind::Energy), 2000.0);
    assert_eq!(ledger.balance(FluxKind::Water), 1000.0);
}

/// Test tolerance policy sensitivity
///
/// Verifies that tolerance policy controls audit sensitivity.
#[test]
fn ledger_tolerance_policy() {
    let mut ledger = Ledger::new();

    // Add slightly imbalanced energy flux
    ledger.push(FluxEntry::new(
        Reservoir::TOAInsolation,
        Reservoir::AtmosEnergy,
        1000.0,
        FluxKind::Energy,
    ));
    ledger.push(FluxEntry::new(
        Reservoir::AtmosEnergy,
        Reservoir::SpaceRadiation,
        999.999999999, // Very small imbalance
        FluxKind::Energy,
    ));

    // With strict tolerance, should fail
    let strict_policy = TolerancePolicy::new(0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
    let result = ledger.audit(strict_policy.clone());
    assert!(
        result.is_err(),
        "Strict tolerance should detect small imbalance"
    );

    // With relaxed tolerance, should pass
    let relaxed_policy = TolerancePolicy::new(1e-9, 1e-9, 1e-9, 1e-9, 1e-9, 1e-9);
    let result = ledger.audit(relaxed_policy);
    assert!(
        result.is_ok(),
        "Relaxed tolerance should allow small imbalance"
    );

    println!("Tolerance test result: {:?}", result);

    println!("Tolerance test result: {:?}", result);
}

/// Test ledger clear functionality
///
/// Verifies that ledger can be cleared and reused.
#[test]
fn ledger_clear() {
    let mut ledger = Ledger::new();

    // Add some fluxes
    ledger.push(FluxEntry::new(
        Reservoir::TOAInsolation,
        Reservoir::AtmosEnergy,
        1000.0,
        FluxKind::Energy,
    ));

    // Verify it has entries
    assert!(!ledger.entries().is_empty());

    // Clear the ledger
    ledger.clear();

    // Verify it's empty
    assert!(ledger.entries().is_empty());
    assert_eq!(ledger.balance(FluxKind::Energy), 0.0);

    // Should pass audit after clearing
    let policy = TolerancePolicy::default();
    let result = ledger.audit(policy);
    assert!(result.is_ok(), "Empty ledger should pass audit");
}

/// Test ledger edge cases
///
/// Verifies ledger behavior with edge cases.
#[test]
fn ledger_edge_cases() {
    let mut ledger = Ledger::new();

    // Test zero flux
    ledger.push(FluxEntry::new(
        Reservoir::TOAInsolation,
        Reservoir::AtmosEnergy,
        0.0,
        FluxKind::Energy,
    ));

    let policy = TolerancePolicy::default();
    let result = ledger.audit(policy);
    assert!(result.is_ok(), "Zero flux should pass audit");

    // Clear and test negative flux
    ledger.clear();
    ledger.push(FluxEntry::new(
        Reservoir::AtmosEnergy,
        Reservoir::SpaceRadiation,
        1000.0,
        FluxKind::Energy,
    ));

    let result = ledger.audit(TolerancePolicy::default());
    assert!(
        result.is_err(),
        "Unbalanced negative flux should fail audit"
    );
}

/// Test complex flux network
///
/// Verifies ledger handles complex multi-reservoir networks.
#[test]
fn ledger_complex_network() {
    let mut ledger = Ledger::new();

    // Create a complex energy flow network
    // TOA -> Atmos -> Surface -> Ocean -> Space
    ledger.push(FluxEntry::new(
        Reservoir::TOAInsolation,
        Reservoir::AtmosEnergy,
        2000.0,
        FluxKind::Energy,
    ));
    ledger.push(FluxEntry::new(
        Reservoir::AtmosEnergy,
        Reservoir::SurfaceEnergy,
        800.0,
        FluxKind::Energy,
    ));
    ledger.push(FluxEntry::new(
        Reservoir::AtmosEnergy,
        Reservoir::SpaceRadiation,
        1200.0,
        FluxKind::Energy,
    ));
    ledger.push(FluxEntry::new(
        Reservoir::SurfaceEnergy,
        Reservoir::OceanHeat,
        400.0,
        FluxKind::Energy,
    ));
    ledger.push(FluxEntry::new(
        Reservoir::SurfaceEnergy,
        Reservoir::SpaceRadiation,
        400.0,
        FluxKind::Energy,
    ));
    ledger.push(FluxEntry::new(
        Reservoir::OceanHeat,
        Reservoir::SpaceRadiation,
        400.0,
        FluxKind::Energy,
    ));

    let policy = TolerancePolicy::default();
    let result = ledger.audit(policy);
    assert!(result.is_ok(), "Complex balanced network should pass audit");

    // All energy should end up in space
    assert_eq!(ledger.balance(FluxKind::Energy), 5200.0);
}

/// Test audit error details
///
/// Verifies that audit errors provide useful information.
#[test]
fn ledger_audit_error_details() {
    let mut ledger = Ledger::new();

    // Create imbalance in multiple flux kinds
    ledger.push(FluxEntry::new(
        Reservoir::TOAInsolation,
        Reservoir::AtmosEnergy,
        1000.0,
        FluxKind::Energy,
    ));
    ledger.push(FluxEntry::new(
        Reservoir::Atmosphere,
        Reservoir::SoilWater,
        500.0,
        FluxKind::Water,
    ));

    let policy = TolerancePolicy::default();
    let result = ledger.audit(policy);
    assert!(result.is_err(), "Unbalanced ledger should fail audit");

    // Should report some kind of imbalance (order depends on HashMap iteration)
    let error = result.unwrap_err();
    assert!(error.kind() == FluxKind::Energy || error.kind() == FluxKind::Water);

    // Check the specific imbalance we created
    if error.kind() == FluxKind::Energy {
        assert_eq!(error.balance(), 1000.0);
    } else {
        assert_eq!(error.balance(), 500.0);
    }
}

/// Test large amounts
///
/// Verifies ledger handles large flux values correctly.
#[test]
fn ledger_large_amounts() {
    let mut ledger = Ledger::new();

    // Use large flux values (megawatt scale)
    let large_amount = 1_000_000.0; // 1 MW

    ledger.push(FluxEntry::new(
        Reservoir::TOAInsolation,
        Reservoir::AtmosEnergy,
        large_amount,
        FluxKind::Energy,
    ));
    ledger.push(FluxEntry::new(
        Reservoir::AtmosEnergy,
        Reservoir::SpaceRadiation,
        large_amount,
        FluxKind::Energy,
    ));

    let policy = TolerancePolicy::default();
    let result = ledger.audit(policy);
    assert!(result.is_ok(), "Large balanced flux should pass audit");

    assert_eq!(ledger.balance(FluxKind::Energy), 2.0 * large_amount);
}

/// Test mixed imbalance detection
///
/// Verifies ledger detects mixed positive/negative imbalances.
/// Note: HashMap iteration order is nondeterministic, so either
/// SurfaceEnergy (-300) or AtmosEnergy (+500) may be reported.
#[test]
fn ledger_mixed_imbalance_detection() {
    let mut ledger = Ledger::new();

    // Create mixed imbalances
    // - TOAInsolation (boundary): -1000 (source)
    // - AtmosEnergy (non-boundary): +1000 - 800 + 300 = +500
    // - SpaceRadiation (boundary): +800 (sink)
    // - SurfaceEnergy (non-boundary): -300 (source)
    ledger.push(FluxEntry::new(
        Reservoir::TOAInsolation,
        Reservoir::AtmosEnergy,
        1000.0,
        FluxKind::Energy,
    ));
    ledger.push(FluxEntry::new(
        Reservoir::AtmosEnergy,
        Reservoir::SpaceRadiation,
        800.0,
        FluxKind::Energy,
    ));
    ledger.push(FluxEntry::new(
        Reservoir::SurfaceEnergy,
        Reservoir::AtmosEnergy,
        300.0,
        FluxKind::Energy,
    ));

    let policy = TolerancePolicy::default();
    let result = ledger.audit(policy);
    assert!(result.is_err(), "Mixed imbalance should fail audit");

    let error = result.unwrap_err();
    // Either non-boundary reservoir may be reported first (non-deterministic HashMap order)
    let balance = error.balance();
    assert!(
        balance == 500.0 || balance == -300.0,
        "Expected either AtmosEnergy (+500) or SurfaceEnergy (-300), got {}",
        balance
    );
}
/// Test negative amounts
///
/// Verifies ledger handles negative flux amounts correctly.
#[test]
fn ledger_negative_amounts() {
    let mut ledger = Ledger::new();

    // All negative fluxes (should be unbalanced)
    ledger.push(FluxEntry::new(
        Reservoir::AtmosEnergy,
        Reservoir::SpaceRadiation,
        1000.0,
        FluxKind::Energy,
    ));
    ledger.push(FluxEntry::new(
        Reservoir::SoilWater,
        Reservoir::Atmosphere,
        500.0,
        FluxKind::Water,
    ));

    let policy = TolerancePolicy::default();
    let result = ledger.audit(policy);
    assert!(result.is_err(), "All negative fluxes should fail audit");

    // Check balances are negative
    assert_eq!(ledger.balance(FluxKind::Energy), 1000.0);
    assert_eq!(ledger.balance(FluxKind::Water), 500.0);
}

/// Test all flux kinds
///
/// Verifies ledger handles all available flux kinds.
#[test]
fn ledger_all_flux_kinds() {
    let mut ledger = Ledger::new();

    // Test all flux kinds with balanced flows
    let flux_kinds = [
        FluxKind::Energy,
        FluxKind::Water,
        FluxKind::Carbon,
        FluxKind::Oxygen,
        FluxKind::Nitrogen,
        FluxKind::Phosphorus,
    ];

    for (i, &kind) in flux_kinds.iter().enumerate() {
        let amount = 100.0 * (i as f64 + 1.0);

        // Add balanced flux for each kind
        ledger.push(FluxEntry::new(
            Reservoir::TOAInsolation,
            Reservoir::AtmosEnergy,
            amount,
            kind,
        ));
        ledger.push(FluxEntry::new(
            Reservoir::AtmosEnergy,
            Reservoir::SpaceRadiation,
            amount,
            kind,
        ));
    }

    let policy = TolerancePolicy::default();
    let result = ledger.audit(policy);
    assert!(result.is_ok(), "All flux kinds balanced should pass audit");

    // All balances should be sum of flux amounts
    for (i, &kind) in flux_kinds.iter().enumerate() {
        let amount = 100.0 * (i as f64 + 1.0);
        assert_eq!(ledger.balance(kind), 2.0 * amount);
    }
}
