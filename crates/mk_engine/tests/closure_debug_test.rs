use mk_core::canon::CanonLocked;
use mk_engine::world_integration::WorldState;
use std::sync::Arc;

fn coarse_dt() -> u64 {
    let seconds: f64 = 1000.0 * 365.25 * 24.0 * 3600.0;
    seconds as u64
}

#[test]
fn debug_closure_daily() {
    let canon = Arc::new(CanonLocked::default());
    let mut world = WorldState::new(canon, [42u8; 32]);

    println!("=== DAILY SUBSTEPS (86400s) ===");
    for i in 0..5 {
        world.step_world(86400).unwrap();
        println!(
            "Step {}: energy_closure={:.6e}, water={:.6e}, ledger_len={}, total_energy={:.6e}",
            i,
            world.audit_trail.energy_closure,
            world.audit_trail.water_closure,
            world.ledger.len(),
            world.metrics.total_energy
        );
    }
}

#[test]
fn debug_closure_coarse() {
    let canon = Arc::new(CanonLocked::default());
    let mut world = WorldState::new(canon, [42u8; 32]);

    let coarse_dt = coarse_dt();
    println!("=== COARSE SINGLE STEP ({}s) ===", coarse_dt);
    for i in 0..5 {
        world.step_world(coarse_dt).unwrap();
        println!(
            "Step {}: energy_closure={:.6e}, water={:.6e}, ledger_len={}, total_energy={:.6e}",
            i,
            world.audit_trail.energy_closure,
            world.audit_trail.water_closure,
            world.ledger.len(),
            world.metrics.total_energy
        );
    }
}

#[test]
fn debug_closure_tides_trace() {
    use mk_core::canon::CanonLocked;
    use mk_core::flux::Ledger;
    use mk_core::grid::GridSpec;
    use std::sync::Arc;

    let canon = Arc::new(CanonLocked::default());
    let grid_spec = GridSpec::new(32, 64);
    let coarse_dt = coarse_dt();

    println!("=== TIDES TRACE with dt={}s ===", coarse_dt);
    let mut ledger = Ledger::new();
    let _state = mk_engine::tides::step_tides(&canon, 0.0, coarse_dt, &grid_spec, &mut ledger);

    println!("Ledger has {} entries:", ledger.len());
    for e in ledger.entries() {
        println!(
            "  {:?} -> {:?} amount={:.6e} kind={:?}  is_nan={} is_infinite={}",
            e.source(),
            e.sink(),
            e.amount(),
            e.kind(),
            e.amount().is_nan(),
            e.amount().is_infinite()
        );
    }

    let r = ledger.closure_residuals();
    println!("closure_residuals: energy={:.6e}", r.energy);
}
