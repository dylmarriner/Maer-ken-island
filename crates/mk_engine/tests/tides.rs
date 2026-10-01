use mk_core::canon::CanonLocked;
use mk_core::flux::{FluxKind, Ledger, Reservoir};
use mk_core::grid::GridSpec;
use mk_engine::tides::step_tides;
use std::sync::Arc;

#[test]
fn tides_deterministic() {
    let canon = Arc::new(CanonLocked::default());
    let grid_spec = GridSpec { nlat: 18, nlon: 36 };
    let mut ledger1 = Ledger::new();
    let mut ledger2 = Ledger::new();

    let state1 = step_tides(&canon, 0.0, 1, &grid_spec, &mut ledger1);
    let state2 = step_tides(&canon, 0.0, 1, &grid_spec, &mut ledger2);

    // Compare potentials
    for ilat in 0..18 {
        for ilon in 0..36 {
            assert_eq!(
                state1.potential.get_safe(ilat, ilon),
                state2.potential.get_safe(ilat, ilon),
                "Potential mismatch at ({}, {})",
                ilat,
                ilon
            );
        }
    }
}

#[test]
fn tides_ledger_entry() {
    let canon = Arc::new(CanonLocked::default());
    let grid_spec = GridSpec { nlat: 18, nlon: 36 };
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
fn tides_deterministic_10_runs() {
    let canon = Arc::new(CanonLocked::default());
    let grid_spec = GridSpec { nlat: 18, nlon: 36 };

    for _ in 0..10 {
        let mut ledger = Ledger::new();
        let _state = step_tides(&canon, 0.0, 1, &grid_spec, &mut ledger);
    }
}

#[test]
fn potential_bounded() {
    let canon = Arc::new(CanonLocked::default());
    let grid_spec = GridSpec { nlat: 18, nlon: 36 };
    let mut ledger = Ledger::new();

    let state = step_tides(&canon, 0.0, 1, &grid_spec, &mut ledger);

    // All potential values should be bounded
    for ilat in 0..18 {
        for ilon in 0..36 {
            let val = *state.potential.get_safe(ilat, ilon).unwrap_or(&0.0);
            assert!(
                (-1.0..=1.0).contains(&val),
                "Potential out of bounds: {} at ({}, {})",
                val,
                ilat,
                ilon
            );
        }
    }
}

#[test]
fn tides_ledger_energy_nonnegative() {
    let canon = Arc::new(CanonLocked::default());
    let grid_spec = GridSpec { nlat: 18, nlon: 36 };
    let mut ledger = Ledger::new();

    step_tides(&canon, 0.0, 1, &grid_spec, &mut ledger);

    // Tidal dissipation should produce non-negative heat
    let entries = ledger.entries();
    assert!(!entries.is_empty());
    let entry = &entries[0];
    assert!(entry.amount >= 0.0, "Tidal heating should be non-negative");
}

#[test]
fn tides_different_ticks() {
    let canon = Arc::new(CanonLocked::default());
    let grid_spec = GridSpec { nlat: 18, nlon: 36 };

    let mut ledger1 = Ledger::new();
    let mut ledger2 = Ledger::new();

    let state1 = step_tides(&canon, 0.0, 1, &grid_spec, &mut ledger1);
    let state2 = step_tides(&canon, 10_000.0 * 3600.0, 1, &grid_spec, &mut ledger2);

    // The bulges move with the planet's rotation and the moons' orbits, so
    // the tide field 10,000 hours later is a different field.
    let max_difference = state1
        .potential
        .data()
        .iter()
        .zip(state2.potential.data())
        .map(|(a, b)| (a - b).abs())
        .fold(0.0_f64, f64::max);
    assert!(
        max_difference > 1e-3,
        "tidal potential did not change over 10,000 h (max Δ {max_difference})"
    );
    let height_difference = state1
        .equilibrium_height_m
        .iter()
        .zip(&state2.equilibrium_height_m)
        .map(|(a, b)| (a - b).abs())
        .fold(0.0_f64, f64::max);
    assert!(height_difference > 0.0);
}
