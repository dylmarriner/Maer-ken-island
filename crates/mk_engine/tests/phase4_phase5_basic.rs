use mk_core::canon::CanonLocked;
use mk_engine::biosphere::deep_time_evolution::DeepTimeEvolution;
/// Basic Phase 4 and Phase 5 Integration Test
use mk_engine::biosphere::BiosphereSystem;
use std::sync::Arc;

#[test]
fn test_phase4_basic_functionality() {
    let canon = Arc::new(CanonLocked::default());
    let mut biosphere = BiosphereSystem::new(canon.clone(), 12345);

    // Initialize biosphere
    biosphere.initialize().unwrap();

    // Create deep-time evolution system
    let mut deep_time = DeepTimeEvolution::new();

    // Step evolution forward
    let events = deep_time
        .step_deep_time_evolution(&mut biosphere.species, 1000.0, 0.3, &mut biosphere.rng)
        .unwrap();

    // Validate
    deep_time.validate(&biosphere.species).unwrap();
    biosphere.validate().unwrap();

    // Check intelligence ceiling
    for species in &biosphere.species {
        assert!(
            species.representative_genome.intelligence_index()
                <= mk_engine::biosphere::genetics::PRE_SAPIENT_CEILING
        );
    }

    println!("✅ Phase 4 Basic Test Passed");
    println!("  Species: {}", biosphere.species.len());
    println!("  Events: {}", events.len());
    println!("  Time: {:.0} years", deep_time.current_time_years);
}

#[test]
fn test_phase5_basic_functionality() {
    let canon = CanonLocked::default();
    let initial_seed = [42u8; 32];

    // Create world state
    let mut world = mk_engine::world_integration::WorldState::new(Arc::new(canon), initial_seed);

    // Initialize biosphere
    world.biosphere_state.initialize().unwrap();

    // Step world forward
    world.step_world(86400).unwrap();

    // Validate
    world.validate().unwrap();

    // Check audit trail
    assert!(!world.audit_trail.entries.is_empty());
    assert!(world.audit_trail.intelligence_ceiling_status);
    assert!(world.audit_trail.hash_chain_continuity);

    println!("✅ Phase 5 Basic Test Passed");
    println!("  Tick: {}", world.tick);
    println!("  Species: {}", world.biosphere_state.species.len());
    println!("  Audit entries: {}", world.audit_trail.entries.len());
}
