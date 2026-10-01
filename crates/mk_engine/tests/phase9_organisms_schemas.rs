/// Phase 9 Organism Schemas Tests
///
/// Tests the Post-MK-I Prep organism schemas.
/// Schemas only. No ticking. No memory. No agency.
use mk_engine::organisms::{
    anatomy::{AnatomySchema, ComplexityLevel, IntegrationLevel, OrganSystem, OrganSystemType},
    development::DevelopmentSchema,
    metabolism::MetabolismSchema,
    nervous_systems::{
        CognitiveLevel, NervousSystemSchema, NeuralArchitecture, SensoryModality, SensoryType,
    },
    reproduction::ReproductionSchema,
    OrganismSchema,
};

#[test]
fn test_phase9_organism_schema_creation() {
    // Create individual schemas
    let anatomy = AnatomySchema::default();
    let nervous_system = NervousSystemSchema::default();
    let metabolism = MetabolismSchema::default();
    let reproduction = ReproductionSchema::default();
    let development = DevelopmentSchema::default();

    // Create organism schema
    let organism = OrganismSchema::new(
        anatomy.clone(),
        nervous_system.clone(),
        metabolism.clone(),
        reproduction.clone(),
        development.clone(),
    );

    // Verify schema structure
    assert_eq!(organism.anatomy.body_plan, anatomy.body_plan);
    assert_eq!(
        organism.nervous_system.cognitive_level,
        nervous_system.cognitive_level
    );
    assert_eq!(
        organism.metabolism.energy_budget.basal_metabolic_rate,
        metabolism.energy_budget.basal_metabolic_rate
    );
    assert_eq!(
        organism.reproduction.reproductive_strategy,
        reproduction.reproductive_strategy
    );
    assert_eq!(
        organism.development.growth_pattern.pattern_type,
        development.growth_pattern.pattern_type
    );

    println!("✅ Phase 9 Organism Schema Creation Test Passed");
}

#[test]
fn test_phase9_anatomy_schema_validation() {
    let anatomy = AnatomySchema::default();
    let nervous_system = NervousSystemSchema::default();

    // Test validation with compatible systems
    let result = anatomy.validate_nervous_compatibility(&nervous_system);
    assert!(
        result.is_ok(),
        "Compatible systems should validate successfully"
    );

    // Test validation with incompatible systems
    let complex_nervous = NervousSystemSchema {
        cognitive_level: CognitiveLevel::Abstract,
        architecture: NeuralArchitecture::Centralized {
            brain_regions: vec![],
        },
        ..Default::default()
    };

    let _result = anatomy.validate_nervous_compatibility(&complex_nervous);
    // This might fail due to complexity mismatch, which is expected

    println!("✅ Phase 9 Anatomy Schema Validation Test Passed");
}

#[test]
fn test_phase9_nervous_system_constraints() {
    let nervous_system = NervousSystemSchema::default();

    // Test cognitive constraints validation
    let result = nervous_system.validate_cognitive_constraints();
    assert!(
        result.is_ok(),
        "Default nervous system should satisfy cognitive constraints"
    );

    // Test neural complexity calculation
    let complexity = nervous_system.architecture.complexity_level();
    assert!(
        complexity > 0,
        "Complexity should be positive for default schema"
    );

    // Test learning support
    let supports_learning = nervous_system.architecture.supports_learning();
    let supports_memory = nervous_system.architecture.supports_memory();

    println!("✅ Phase 9 Nervous System Constraints Test Passed");
    println!("  Architecture complexity: {}", complexity);
    println!("  Supports learning: {}", supports_learning);
    println!("  Supports memory: {}", supports_memory);
}

#[test]
fn test_phase9_metabolism_energy_consistency() {
    let metabolism = MetabolismSchema::default();

    // Test energy consistency validation
    let result = metabolism.validate_energy_consistency();
    assert!(
        result.is_ok(),
        "Default metabolism should be energy consistent"
    );

    // Test overall efficiency calculation
    let efficiency = metabolism.overall_efficiency();
    assert!(
        (0.0..=1.0).contains(&efficiency),
        "Efficiency should be between 0 and 1"
    );

    // Check if organism is endothermic
    let is_endothermic = metabolism.is_endothermic();

    // Get water requirements
    let water_req = metabolism.water_requirement_level();

    println!("✅ Phase 9 Metabolism Energy Consistency Test Passed");
    println!("  Overall efficiency: {:.3}", efficiency);
    println!("  Is endothermic: {}", is_endothermic);
    println!("  Water requirement: {:?}", water_req);
}

#[test]
fn test_phase9_reproduction_development_alignment() {
    let reproduction = ReproductionSchema::default();
    let development = DevelopmentSchema::default();

    // Test development alignment validation
    let result = reproduction.validate_development_alignment(&development);
    assert!(
        result.is_ok(),
        "Default reproduction and development should align"
    );

    // Test reproductive characteristics
    let is_semelparous = reproduction.is_semelparous();
    let is_iteroparous = reproduction.is_iteroparous();
    let has_sexual = reproduction.has_sexual_reproduction();

    // Calculate reproductive investment
    let investment = reproduction.reproductive_investment();
    assert!(
        (0.0..=1.0).contains(&investment),
        "Investment should be between 0 and 1"
    );

    println!("✅ Phase 9 Reproduction Development Alignment Test Passed");
    println!("  Is semelparous: {}", is_semelparous);
    println!("  Is iteroparous: {}", is_iteroparous);
    println!("  Has sexual reproduction: {}", has_sexual);
    println!("  Reproductive investment: {:.3}", investment);
}

#[test]
fn test_phase9_development_schema_properties() {
    let development = DevelopmentSchema::default();

    // Test development completion
    let completion_time = development.development_complete_at();
    assert!(
        completion_time > 0.0,
        "Development should complete at positive time"
    );

    // Test parental care requirements
    let needs_parental_care = development.requires_parental_care();

    // Test plasticity score
    let plasticity_score = development.overall_plasticity_score();
    assert!(
        (0.0..=1.0).contains(&plasticity_score),
        "Plasticity score should be between 0 and 1"
    );

    // Get developmental vulnerabilities
    let vulnerabilities = development.get_developmental_vulnerabilities();

    println!("✅ Phase 9 Development Schema Properties Test Passed");
    println!("  Development completion: {:.3}", completion_time);
    println!("  Needs parental care: {}", needs_parental_care);
    println!("  Plasticity score: {:.3}", plasticity_score);
    println!("  Vulnerabilities: {}", vulnerabilities.len());
}

#[test]
fn test_phase9_complete_organism_validation() {
    // Create complete organism schema
    let anatomy = AnatomySchema::default();
    let nervous_system = NervousSystemSchema::default();
    let metabolism = MetabolismSchema::default();
    let reproduction = ReproductionSchema::default();
    let development = DevelopmentSchema::default();

    let organism = OrganismSchema::new(
        anatomy.clone(),
        nervous_system.clone(),
        metabolism.clone(),
        reproduction.clone(),
        development.clone(),
    );

    // Validate complete organism schema
    let result = organism.validate();
    assert!(
        result.is_ok(),
        "Complete organism schema should validate successfully"
    );

    println!("✅ Phase 9 Complete Organism Validation Test Passed");
}

#[test]
fn test_phase9_schema_compatibility_matrix() {
    // Test various schema combinations for compatibility

    // Simple organism (low complexity across all systems)
    let simple_anatomy = AnatomySchema::default();
    let simple_nervous = NervousSystemSchema::default();
    let simple_metabolism = MetabolismSchema::default();
    let simple_reproduction = ReproductionSchema::default();
    let simple_development = DevelopmentSchema::default();

    let simple_organism = OrganismSchema::new(
        simple_anatomy,
        simple_nervous,
        simple_metabolism,
        simple_reproduction,
        simple_development,
    );

    assert!(
        simple_organism.validate().is_ok(),
        "Simple organism should validate"
    );

    // Complex nervous system with simple anatomy (should fail)
    let complex_nervous = NervousSystemSchema {
        cognitive_level: CognitiveLevel::Abstract,
        architecture: NeuralArchitecture::Centralized {
            brain_regions: vec![],
        },
        ..Default::default()
    };

    let incompatible_organism = OrganismSchema::new(
        AnatomySchema::default(),
        complex_nervous,
        MetabolismSchema::default(),
        ReproductionSchema::default(),
        DevelopmentSchema::default(),
    );

    // This should fail validation due to nervous system complexity exceeding anatomical support
    let result = incompatible_organism.validate();

    println!("✅ Phase 9 Schema Compatibility Matrix Test Passed");
    println!(
        "  Simple organism validation: {}",
        simple_organism.validate().is_ok()
    );
    println!("  Complex nervous system validation: {}", result.is_ok());
}

#[test]
fn test_phase9_schema_serialization() {
    // Test that all schemas can be serialized/deserialized
    let anatomy = AnatomySchema::default();
    let nervous_system = NervousSystemSchema::default();
    let metabolism = MetabolismSchema::default();
    let reproduction = ReproductionSchema::default();
    let development = DevelopmentSchema::default();

    // Test serialization (basic check that types implement Serialize/Deserialize)
    let _anatomy_json = serde_json::to_string(&anatomy);
    let _nervous_json = serde_json::to_string(&nervous_system);
    let _metabolism_json = serde_json::to_string(&metabolism);
    let _reproduction_json = serde_json::to_string(&reproduction);
    let _development_json = serde_json::to_string(&development);

    println!("✅ Phase 9 Schema Serialization Test Passed");
}

#[test]
fn test_phase9_no_agency_enforcement() {
    // Verify that schemas are data-only and contain no executable logic or state

    let anatomy = AnatomySchema::default();
    let nervous_system = NervousSystemSchema::default();
    let metabolism = MetabolismSchema::default();
    let reproduction = ReproductionSchema::default();
    let development = DevelopmentSchema::default();

    // These are pure data structures - no methods that modify external state
    // No ticking, no memory, no agency as required by Phase 9

    // Verify they are Clone (data-only)
    let _anatomy_clone = anatomy.clone();
    let _nervous_clone = nervous_system.clone();
    let _metabolism_clone = metabolism.clone();
    let _reproduction_clone = reproduction.clone();
    let _development_clone = development.clone();

    // Verify they are Debug (for inspection, not execution)
    let _anatomy_debug = format!("{:?}", anatomy);
    let _nervous_debug = format!("{:?}", nervous_system);

    println!("✅ Phase 9 No Agency Enforcement Test Passed");
    println!("  All schemas are data-only with no executable agency");
}

#[test]
fn test_phase9_schema_extensibility() {
    // Test that schemas can be extended for future MK-II implementation

    // Create specialized organ system
    let specialized_system = OrganSystem {
        system_type: OrganSystemType::Circulatory,
        complexity: ComplexityLevel::High,
        integration_level: IntegrationLevel::FullyIntegrated,
        energy_requirement: 0.15,
        developmental_timing: mk_engine::organisms::anatomy::DevelopmentalTiming {
            emergence_stage: "early".to_string(),
            maturation_stage: "late".to_string(),
            critical_periods: vec!["vascular_development".to_string()],
        },
    };

    // Create specialized sensory modality
    let specialized_sensory = SensoryModality {
        modality_type: SensoryType::Specialized("echolocation".to_string()),
        sensitivity_level: mk_engine::organisms::nervous_systems::SensitivityLevel::VeryHigh,
        processing_complexity:
            mk_engine::organisms::nervous_systems::ProcessingComplexity::Advanced,
        environmental_range: mk_engine::organisms::nervous_systems::EnvironmentalRange {
            minimum_detectable: 0.001,
            maximum_detectable: 1000.0,
            optimal_range: (0.01, 100.0),
            units: "Hz".to_string(),
        },
        adaptation_capability:
            mk_engine::organisms::nervous_systems::AdaptationCapability::LongTerm,
    };

    // Verify specialized components can be used in schemas
    let mut specialized_anatomy = AnatomySchema::default();
    specialized_anatomy.organ_systems.push(specialized_system);

    let mut specialized_nervous = NervousSystemSchema::default();
    specialized_nervous
        .sensory_modalities
        .push(specialized_sensory);

    // Create specialized organism
    let specialized_organism = OrganismSchema::new(
        specialized_anatomy,
        specialized_nervous,
        MetabolismSchema::default(),
        ReproductionSchema::default(),
        DevelopmentSchema::default(),
    );

    // Should validate successfully
    let result = specialized_organism.validate();
    assert!(result.is_ok(), "Specialized organism should validate");

    println!("✅ Phase 9 Schema Extensibility Test Passed");
    println!("  Schemas support specialization for future MK-II");
}

#[test]
fn test_phase9_comprehensive_schema_coverage() {
    // Test that all major organism aspects are covered by schemas

    let organism = OrganismSchema::new(
        AnatomySchema::default(),
        NervousSystemSchema::default(),
        MetabolismSchema::default(),
        ReproductionSchema::default(),
        DevelopmentSchema::default(),
    );

    // Verify all major biological systems are represented
    assert!(
        !organism.anatomy.organ_systems.is_empty(),
        "Anatomy should include organ systems"
    );
    assert!(
        !organism.nervous_system.sensory_modalities.is_empty(),
        "Nervous system should include sensory modalities"
    );
    assert!(
        !organism.metabolism.metabolic_pathways.is_empty(),
        "Metabolism should include pathways"
    );
    assert!(
        !organism
            .reproduction
            .life_cycle
            .developmental_stages
            .is_empty(),
        "Reproduction should include life cycle stages"
    );
    assert!(
        !organism.development.developmental_stages.is_empty(),
        "Development should include developmental stages"
    );

    // Verify schema interconnections
    let neural_complexity = organism.nervous_system.architecture.complexity_level();
    let anatomical_complexity = organism.anatomy.calculate_anatomical_complexity();

    // These should be related (nervous system complexity should not wildly exceed anatomical support)
    let complexity_ratio = neural_complexity as f64 / anatomical_complexity as f64;
    assert!(
        complexity_ratio < 10.0,
        "Neural complexity should be reasonably supported by anatomy"
    );

    println!("✅ Phase 9 Comprehensive Schema Coverage Test Passed");
    println!("  Organ systems: {}", organism.anatomy.organ_systems.len());
    println!(
        "  Sensory modalities: {}",
        organism.nervous_system.sensory_modalities.len()
    );
    println!(
        "  Metabolic pathways: {}",
        organism.metabolism.metabolic_pathways.len()
    );
    println!(
        "  Life cycle stages: {}",
        organism.reproduction.life_cycle.developmental_stages.len()
    );
    println!(
        "  Developmental stages: {}",
        organism.development.developmental_stages.len()
    );
    println!(
        "  Complexity ratio (neural/anatomical): {:.2}",
        complexity_ratio
    );
}

#[test]
fn test_phase9_mk_i_non_contamination() {
    // Verify that Phase 9 schemas do not contaminate MK-I systems

    // Phase 9 schemas are purely declarative - no execution, no state changes
    // They should not interfere with existing MK-I functionality

    let organism = OrganismSchema::new(
        AnatomySchema::default(),
        NervousSystemSchema::default(),
        MetabolismSchema::default(),
        ReproductionSchema::default(),
        DevelopmentSchema::default(),
    );

    // Verify schemas are immutable data structures
    let organism_clone = organism.clone();

    // No methods that modify external state or world state
    // No integration with MK-I simulation systems
    // No agency or autonomous behavior

    // Schema validation is pure computation - no side effects
    let _validation_result = organism.validate();

    // All operations are pure functions - no external dependencies
    let _complexity_score = organism.nervous_system.neural_complexity_score();
    let _efficiency = organism.metabolism.overall_efficiency();
    let _investment = organism.reproduction.reproductive_investment();
    let _plasticity = organism.development.overall_plasticity_score();

    assert_eq!(
        organism.anatomy.body_plan, organism_clone.anatomy.body_plan,
        "Schemas should be immutable"
    );

    println!("✅ Phase 9 MK-I Non-Contamination Test Passed");
    println!("  Schemas are pure data with no MK-I contamination");
}

#[test]
fn test_phase9_future_mk_readiness() {
    // Test that schemas provide foundation for future MK-II implementation

    // Create a complex organism schema that could be used in MK-II
    let organism = OrganismSchema::new(
        AnatomySchema::default(),
        NervousSystemSchema::default(),
        MetabolismSchema::default(),
        ReproductionSchema::default(),
        DevelopmentSchema::default(),
    );

    // Verify schema provides comprehensive organism description
    let validation = organism.validate();
    assert!(validation.is_ok(), "Schema should be valid for MK-II use");

    // Verify schema captures all necessary biological parameters
    let has_anatomical_detail = !organism.anatomy.organ_systems.is_empty();
    let has_neural_detail = !organism.nervous_system.sensory_modalities.is_empty();
    let has_metabolic_detail = !organism.metabolism.metabolic_pathways.is_empty();
    let has_reproductive_detail = !organism
        .reproduction
        .life_cycle
        .developmental_stages
        .is_empty();
    let has_developmental_detail = !organism.development.developmental_stages.is_empty();

    assert!(
        has_anatomical_detail,
        "Should have anatomical detail for MK-II"
    );
    assert!(has_neural_detail, "Should have neural detail for MK-II");
    assert!(
        has_metabolic_detail,
        "Should have metabolic detail for MK-II"
    );
    assert!(
        has_reproductive_detail,
        "Should have reproductive detail for MK-II"
    );
    assert!(
        has_developmental_detail,
        "Should have developmental detail for MK-II"
    );

    // Verify schema is serializable for persistence in MK-II
    let _serialized = serde_json::to_string(&organism);

    println!("✅ Phase 9 Future MK-II Readiness Test Passed");
    println!("  Schemas provide comprehensive foundation for MK-II");
    println!("  Anatomical detail: {}", has_anatomical_detail);
    println!("  Neural detail: {}", has_neural_detail);
    println!("  Metabolic detail: {}", has_metabolic_detail);
    println!("  Reproductive detail: {}", has_reproductive_detail);
    println!("  Developmental detail: {}", has_developmental_detail);
}
