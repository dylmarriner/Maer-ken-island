//! Contract tests for Four Core Systems schema validation.
//!
//! Tests cover:
//! - Fixture loading and validation for gem-d and gem-k templates
//! - Schema constraint enforcement (range validation)
//! - Serialization round-trip
//! - Deterministic initialization verification

use mk_core::human::schema::CoreSystemsSchema;
use std::fs;

const FIXTURE_GEM_D: &str = "../../fixtures/human/core_systems_gem_d.json";
const FIXTURE_GEM_K: &str = "../../fixtures/human/core_systems_gem_k.json";

#[test]
fn fixture_gem_d_loads_and_validates() {
    let content = fs::read_to_string(FIXTURE_GEM_D).expect("fixture file should exist");
    let systems: CoreSystemsSchema =
        serde_json::from_str(&content).expect("fixture should parse as CoreSystemsSchema");

    systems
        .validate()
        .expect("gem-d core systems should validate against canon constraints");
}

#[test]
fn fixture_gem_k_loads_and_validates() {
    let content = fs::read_to_string(FIXTURE_GEM_K).expect("fixture file should exist");
    let systems: CoreSystemsSchema =
        serde_json::from_str(&content).expect("fixture should parse as CoreSystemsSchema");

    systems
        .validate()
        .expect("gem-k core systems should validate against canon constraints");
}

#[test]
fn biosys_atp_out_of_range_fails() {
    let mut systems = CoreSystemsSchema::default();
    systems.biosys.metabolic_baselines.atp = 1.5; // Invalid: > 1.0

    let result = systems.validate();
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("ATP"));
}

#[test]
fn biosys_negative_atp_fails() {
    let mut systems = CoreSystemsSchema::default();
    systems.biosys.metabolic_baselines.atp = -0.1; // Invalid: < 0.0

    let result = systems.validate();
    assert!(result.is_err());
}

#[test]
fn willsys_threshold_range_bounds_enforced() {
    let mut systems = CoreSystemsSchema::default();
    systems.willsys.willpower_profile.threshold_range.max = 1.2; // Invalid: > 1.0

    let result = systems.validate();
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("threshold_range.max"));
}

#[test]
fn willsys_working_memory_bounds_enforced() {
    let mut systems = CoreSystemsSchema::default();
    systems.willsys.cognitive_access.working_memory_capacity = 15; // Invalid: > 10

    let result = systems.validate();
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("working_memory"));
}

#[test]
fn psychesys_emotional_amplification_bounds() {
    let mut systems = CoreSystemsSchema::default();
    systems.psychesys.emotional_processing.affect_intensity = 2.5; // Invalid: > 2.0

    let result = systems.validate();
    assert!(result.is_err());
}

#[test]
fn chaossys_entropy_range_enforced() {
    let mut systems = CoreSystemsSchema::default();
    systems.chaossys.randomness_profile.entropy_level = -0.5; // Invalid: < 0.0

    let result = systems.validate();
    assert!(result.is_err());
}

#[test]
fn chaossys_signed_bounds_are_allowed() {
    let mut systems = CoreSystemsSchema::default();
    systems.chaossys.chaos_bounds.survival_chaos.min = 0.8;
    systems.chaossys.chaos_bounds.survival_chaos.max = -0.2;

    systems
        .validate()
        .expect("canon docs allow signed chaos bounds without min/max ordering rule");
}

#[test]
fn serialization_roundtrip_preserves_values() {
    let content = fs::read_to_string(FIXTURE_GEM_D).unwrap();
    let original: CoreSystemsSchema = serde_json::from_str(&content).unwrap();

    let serialized = serde_json::to_string(&original).unwrap();
    let deserialized: CoreSystemsSchema = serde_json::from_str(&serialized).unwrap();

    assert_eq!(
        original.biosys.metabolic_baselines.atp,
        deserialized.biosys.metabolic_baselines.atp
    );
    assert_eq!(
        original.psychesys.emotional_processing.affect_intensity,
        deserialized.psychesys.emotional_processing.affect_intensity
    );
    assert_eq!(
        original.willsys.cognitive_access.working_memory_capacity,
        deserialized
            .willsys
            .cognitive_access
            .working_memory_capacity
    );
    assert_eq!(
        original.chaossys.randomness_profile.entropy_level,
        deserialized.chaossys.randomness_profile.entropy_level
    );
}

#[test]
fn biosys_the_law_biological_authority_valid() {
    // BioSys validation enforces THE LAW: biological authority
    // All baselines must be in [0, 1] - tested via fixture validation
    let content = fs::read_to_string(FIXTURE_GEM_D).unwrap();
    let systems: CoreSystemsSchema = serde_json::from_str(&content).unwrap();

    // Verify specific gem-d values from HumanReplicationSchema canon
    assert!(
        systems.biosys.metabolic_baselines.atp >= 0.0
            && systems.biosys.metabolic_baselines.atp <= 1.0
    );
    assert!(
        systems.biosys.endocrine_baselines.dopamine >= 0.0
            && systems.biosys.endocrine_baselines.dopamine <= 1.0
    );

    // All drives in [0, 1]
    assert!(
        systems.biosys.drive_sensitivities.hunger >= 0.0
            && systems.biosys.drive_sensitivities.hunger <= 1.0
    );
    assert!(
        systems.biosys.drive_sensitivities.fatigue >= 0.0
            && systems.biosys.drive_sensitivities.fatigue <= 1.0
    );
}

#[test]
fn default_schema_validates() {
    let systems = CoreSystemsSchema::default();
    systems
        .validate()
        .expect("default CoreSystemsSchema should validate");
}

#[test]
fn gem_d_distinct_from_gem_k() {
    let content_d = fs::read_to_string(FIXTURE_GEM_D).unwrap();
    let content_k = fs::read_to_string(FIXTURE_GEM_K).unwrap();

    let gem_d: CoreSystemsSchema = serde_json::from_str(&content_d).unwrap();
    let gem_k: CoreSystemsSchema = serde_json::from_str(&content_k).unwrap();

    // gem-d: higher fatigue sensitivity (0.82 vs 0.68)
    assert!(gem_d.biosys.drive_sensitivities.fatigue > gem_k.biosys.drive_sensitivities.fatigue);

    // gem-k: higher autonomy drive (0.92 vs 0.78)
    assert!(
        gem_k.willsys.agency_patterns.autonomy_drive > gem_d.willsys.agency_patterns.autonomy_drive
    );

    // gem-d: lower mood stability (0.35 vs 0.42)
    assert!(
        gem_d.psychesys.emotional_processing.mood_stability
            < gem_k.psychesys.emotional_processing.mood_stability
    );
}
