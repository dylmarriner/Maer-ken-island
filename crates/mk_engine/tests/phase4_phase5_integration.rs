use mk_core::canon::CanonLocked;
use mk_engine::biosphere::deep_time_evolution::DeepTimeEvolution;
/// Phase 4 and Phase 5 Integration Test
///
/// Tests deep-time evolution mechanics and world integration with audit chain
use mk_engine::biosphere::BiosphereSystem;
use mk_engine::world_integration::WorldState;
use std::sync::Arc;

#[test]
#[ignore = "slow: ~35 s in debug; run with: cargo test --workspace --release -- --ignored slow_"]
fn slow_test_phase4_long_horizon_evolution() {
    let canon = Arc::new(CanonLocked::default());
    let mut biosphere = BiosphereSystem::new(canon.clone(), 12345);

    // Initialize biosphere
    biosphere.initialize().unwrap();

    // Create deep-time evolution system
    let mut deep_time = DeepTimeEvolution::new();

    // Step evolution forward 100,000 years
    let dt_years = 1000.0;
    let steps = 100;

    for step in 0..steps {
        let events = deep_time
            .step_deep_time_evolution(
                &mut biosphere.species,
                dt_years,
                0.3, // moderate environmental stress
                &mut biosphere.rng,
            )
            .unwrap();

        // Validate constraints (species were changed outside the
        // biosphere's own step, so refresh its statistics first).
        biosphere.refresh_statistics();
        deep_time.validate(&biosphere.species).unwrap();
        biosphere.validate().unwrap();

        // Check for extinction events
        if step % 20 == 0 {
            println!(
                "Step {}: {} species, {} events",
                step,
                biosphere.species.len(),
                events.len()
            );
        }

        // Verify intelligence ceiling
        for species in &biosphere.species {
            assert!(
                species.representative_genome.intelligence_index()
                    <= mk_engine::biosphere::genetics::PRE_SAPIENT_CEILING,
                "Species {} exceeded intelligence ceiling",
                species.species_id
            );
        }
    }

    // Verify Phase 4 requirements
    assert!(deep_time.current_time_years > 90000.0);
    assert!(
        !deep_time.extinction_events.is_empty()
            || deep_time.recovery_state.time_since_extinction_years > 0.0
    );

    println!("✅ Phase 4 Deep-Time Evolution Test Passed");
}

#[test]
fn test_phase5_world_integration() {
    let canon = Arc::new(CanonLocked::default());
    let initial_seed = [42u8; 32];

    // Create world state
    let mut world = WorldState::new(canon.clone(), initial_seed);

    // Initialize biosphere
    world.biosphere_state.initialize().unwrap();

    // Step world forward several ticks
    for tick in 0..10 {
        world.step_world(86400).unwrap(); // 1 day in seconds

        // Validate after each step
        world.validate().unwrap();

        // Check audit trail
        assert!(!world.audit_trail.entries.is_empty());

        // Verify intelligence ceiling status
        assert!(world.audit_trail.intelligence_ceiling_status);

        // Check hash chain continuity
        assert!(world.audit_trail.hash_chain_continuity);

        println!(
            "Tick {}: {} species, {} audit entries",
            tick,
            world.biosphere_state.species.len(),
            world.audit_trail.entries.len()
        );
    }

    // Test snapshot functionality
    let snapshot = world.create_snapshot().unwrap();

    // Modify world
    world.tick += 100;

    // Load snapshot and verify
    world.load_snapshot(&snapshot).unwrap();
    assert_eq!(world.tick, snapshot.tick);

    println!("✅ Phase 5 World Integration Test Passed");
}

#[test]
fn test_phase4_phase5_integration() {
    let canon = Arc::new(CanonLocked::default());
    let initial_seed = [123u8; 32];

    // Create integrated world
    let mut world = WorldState::new(canon.clone(), initial_seed);

    // Initialize biosphere with deep-time evolution
    world.biosphere_state.initialize().unwrap();

    // Add deep-time evolution to the world
    let mut deep_time = DeepTimeEvolution::new();

    // Run integrated simulation
    for cycle in 0..5 {
        println!("=== Cycle {} ===", cycle);

        // Step world (includes biosphere)
        world.step_world(86400 * 365).unwrap(); // 1 year

        // Step deep-time evolution
        let events = deep_time
            .step_deep_time_evolution(
                &mut world.biosphere_state.species,
                1000.0, // 1000 years
                0.2,    // low stress
                &mut world.rng,
            )
            .unwrap();

        // Validate integrated state
        world.validate().unwrap();
        deep_time.validate(&world.biosphere_state.species).unwrap();

        // Report status
        println!("  Species: {}", world.biosphere_state.species.len());
        println!(
            "  Intelligence max: {:.3}",
            world
                .biosphere_state
                .species
                .iter()
                .map(|s| s.representative_genome.intelligence_index())
                .fold(0.0, f64::max)
        );
        println!("  Events: {}", events.len());
        println!("  Extinction events: {}", deep_time.extinction_events.len());

        // Verify Phase 4 constraints
        assert!(deep_time.current_time_years >= cycle as f64 * 1000.0);

        // Verify Phase 5 audit requirements
        assert!(world.audit_trail.intelligence_ceiling_status);
        assert!(world.audit_trail.hash_chain_continuity);
    }

    println!("✅ Phase 4 + Phase 5 Integration Test Passed");
}

#[test]
fn test_intelligence_ceiling_enforcement() {
    let canon = Arc::new(CanonLocked::default());
    let mut biosphere = BiosphereSystem::new(canon.clone(), 99999);

    // Initialize with species near intelligence ceiling
    biosphere.initialize().unwrap();

    // Create deep-time evolution with high stress
    let mut deep_time = DeepTimeEvolution::new();

    // Run under high stress to trigger intelligence regression
    for _ in 0..50 {
        let _events = deep_time
            .step_deep_time_evolution(
                &mut biosphere.species,
                500.0,
                0.9, // very high stress
                &mut biosphere.rng,
            )
            .unwrap();

        // Validate intelligence ceiling
        for species in &biosphere.species {
            let intelligence = species.representative_genome.intelligence_index();
            assert!(
                intelligence <= mk_engine::biosphere::genetics::PRE_SAPIENT_CEILING,
                "Intelligence ceiling violated: {:.3} > {:.3}",
                intelligence,
                mk_engine::biosphere::genetics::PRE_SAPIENT_CEILING
            );
        }
    }

    println!("✅ Intelligence Ceiling Enforcement Test Passed");
}

#[test]
fn test_deterministic_replay() {
    let canon = Arc::new(CanonLocked::default());
    let initial_seed = [77u8; 32];

    // Create first world
    let mut world1 = WorldState::new(canon.clone(), initial_seed);
    world1.biosphere_state.initialize().unwrap();

    // Step forward
    for _ in 0..5 {
        world1.step_world(86400).unwrap();
    }

    // Create snapshot
    let snapshot = world1.create_snapshot().unwrap();

    // Create second world and load snapshot
    let mut world2 = WorldState::new(canon.clone(), initial_seed);
    world2.load_snapshot(&snapshot).unwrap();

    // Step both worlds forward same amount
    for _ in 0..3 {
        world1.step_world(86400).unwrap();
        world2.step_world(86400).unwrap();
    }

    // Verify identical states
    assert_eq!(world1.tick, world2.tick);
    assert_eq!(
        world1.biosphere_state.species.len(),
        world2.biosphere_state.species.len()
    );

    // Verify identical hash chains
    assert_eq!(world1.hash_chain.current, world2.hash_chain.current);

    println!("✅ Deterministic Replay Test Passed");
}
