use mk_core::canon::CanonLocked;
use mk_engine::world_integration::WorldState;
use std::sync::Arc;

fn main() {
    let canon = Arc::new(CanonLocked::default());
    let mut world = WorldState::new(canon, [42u8; 32]);

    println!("=== INITIAL ===");
    println!("energy_closure: {:?}", world.audit_trail.energy_closure);
    println!("ledger entries: {}", world.ledger.len());

    // Step 1 tick
    world.step_world(86400).unwrap();
    println!("=== AFTER 1 TICK (86400s) ===");
    println!("energy_closure: {:.6e}", world.audit_trail.energy_closure);
    println!("ledger entries: {}", world.ledger.len());
    for e in world.ledger.entries() {
        println!(
            "  {:?} -> {:?}  amount={:.6e}  kind={:?}",
            e.source(),
            e.sink(),
            e.amount(),
            e.kind()
        );
    }

    // Check what closure_residuals would give
    let r = world.ledger.closure_residuals();
    println!(
        "closure_residuals: energy={:.6e}, water={:.6e}, carbon={:.6e}",
        r.energy, r.water, r.carbon
    );
    println!("metrics.total_energy: {:.6e}", world.metrics.total_energy);
    println!("metrics.total_water: {:.6e}", world.metrics.total_water);

    // Step more
    for _i in 0..100 {
        world.step_world(86400).unwrap();
    }
    println!("=== AFTER {} MORE TICKS ===", 100);
    println!("energy_closure: {:.6e}", world.audit_trail.energy_closure);
    println!("metrics.total_energy: {:.6e}", world.metrics.total_energy);
    println!("metrics.total_water: {:.6e}", world.metrics.total_water);
    println!("ledger entries: {}", world.ledger.len());

    // Check other metrics
    println!("tick: {}", world.tick);
    println!("track drift: {:?}", world.audit_trail.energy_closure);
}
