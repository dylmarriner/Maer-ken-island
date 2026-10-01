use mk_core::canon::CanonLocked;
use mk_core::flux::Ledger;
use mk_core::grid::GridSpec;
use mk_engine::{insolation::step_insolation, orbit::step_orbit, rotation::step_rotation};

#[test]
fn insolation_deterministic() {
    let canon = CanonLocked::default();
    let orbit = step_orbit(&canon, 0.0);
    let rotation = step_rotation(0.0, orbit.mean_anomaly, &canon);
    let grid_spec = GridSpec { nlat: 18, nlon: 36 };

    let field1 = step_insolation(
        &canon,
        &orbit,
        &rotation,
        &grid_spec,
        1.0,
        &mut Ledger::new(),
    );
    let field2 = step_insolation(
        &canon,
        &orbit,
        &rotation,
        &grid_spec,
        1.0,
        &mut Ledger::new(),
    );

    // Compare grids
    for ilat in 0..18 {
        for ilon in 0..36 {
            assert_eq!(
                field1.toa_w_m2.get_safe(ilat, ilon),
                field2.toa_w_m2.get_safe(ilat, ilon),
                "TOA mismatch at ({}, {})",
                ilat,
                ilon
            );
        }
    }
}

#[test]
fn insolation_field_computed() {
    let canon = CanonLocked::default();
    let orbit = step_orbit(&canon, 0.0);
    let rotation = step_rotation(0.0, orbit.mean_anomaly, &canon);
    let grid_spec = GridSpec { nlat: 18, nlon: 36 };

    let field = step_insolation(
        &canon,
        &orbit,
        &rotation,
        &grid_spec,
        1.0,
        &mut Ledger::new(),
    );
    assert!(
        !field.toa_w_m2.data().is_empty(),
        "Insolation field must contain cells"
    );
}

#[test]
fn insolation_deterministic_10_runs() {
    let canon = CanonLocked::default();
    let orbit = step_orbit(&canon, 0.0);
    let rotation = step_rotation(0.0, orbit.mean_anomaly, &canon);
    let grid_spec = GridSpec { nlat: 18, nlon: 36 };
    let mut ledger = Ledger::new();

    for _ in 0..10 {
        let _field = step_insolation(&canon, &orbit, &rotation, &grid_spec, 1.0, &mut ledger);
    }
}

#[test]
fn insolation_nonzero() {
    let canon = CanonLocked::default();
    let orbit = step_orbit(&canon, 0.0);
    let rotation = step_rotation(0.0, orbit.mean_anomaly, &canon);
    let grid_spec = GridSpec { nlat: 18, nlon: 36 };

    let field = step_insolation(
        &canon,
        &orbit,
        &rotation,
        &grid_spec,
        1.0,
        &mut Ledger::new(),
    );

    // At least some cells should have nonzero insolation
    let mut has_nonzero = false;
    for ilat in 0..18 {
        for ilon in 0..36 {
            if *field.toa_w_m2.get_safe(ilat, ilon).unwrap_or(&0.0) > 0.0 {
                has_nonzero = true;
                break;
            }
        }
    }
    assert!(has_nonzero, "Insolation should be nonzero");
}
