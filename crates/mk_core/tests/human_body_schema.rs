//! Contract tests for Body Schema validation
//!
//! Tests cover:
//! - Fixture loading and validation for gem-d (XY) and gem-k (XX) templates
//! - DNA validation (19 neurochemical traits per strand in [0, 1])
//! - Sex chromosome validation (XX, XY)
//! - Genetic expression weight validation (sum = 1.0)
//! - Serialization round-trip
//! - Biological sex determination

use mk_core::human::schema::BodySchema;
use std::fs;

const FIXTURE_GEM_D: &str = "../../fixtures/human/body_gem_d.json";
const FIXTURE_GEM_K: &str = "../../fixtures/human/body_gem_k.json";

#[test]
fn fixture_gem_d_loads_and_validates() {
    let content = fs::read_to_string(FIXTURE_GEM_D).expect("fixture file should exist");
    let body: BodySchema =
        serde_json::from_str(&content).expect("fixture should parse as BodySchema");

    body.validate()
        .expect("gem-d body should validate against canon constraints");
}

#[test]
fn fixture_gem_k_loads_and_validates() {
    let content = fs::read_to_string(FIXTURE_GEM_K).expect("fixture file should exist");
    let body: BodySchema =
        serde_json::from_str(&content).expect("fixture should parse as BodySchema");

    body.validate()
        .expect("gem-k body should validate against canon constraints");
}

#[test]
fn gem_d_has_xy_chromosomes() {
    let content = fs::read_to_string(FIXTURE_GEM_D).unwrap();
    let body: BodySchema = serde_json::from_str(&content).unwrap();

    assert_eq!(body.biological_sex(), "male");

    // Explicit chromosome check
    match (
        &body.dna.chromosomes.pair23.A,
        &body.dna.chromosomes.pair23.B,
    ) {
        (
            mk_core::human::schema::SexChromosomeSchema::X,
            mk_core::human::schema::SexChromosomeSchema::Y,
        ) => {}
        _ => panic!("Expected XY chromosomes for gem-d (male)"),
    }
}

#[test]
fn gem_k_has_xx_chromosomes() {
    let content = fs::read_to_string(FIXTURE_GEM_K).unwrap();
    let body: BodySchema = serde_json::from_str(&content).unwrap();

    assert_eq!(body.biological_sex(), "female");

    // Explicit chromosome check
    match (
        &body.dna.chromosomes.pair23.A,
        &body.dna.chromosomes.pair23.B,
    ) {
        (
            mk_core::human::schema::SexChromosomeSchema::X,
            mk_core::human::schema::SexChromosomeSchema::X,
        ) => {}
        _ => panic!("Expected XX chromosomes for gem-k (female)"),
    }
}

#[test]
fn dna_traits_in_valid_range() {
    let content = fs::read_to_string(FIXTURE_GEM_D).unwrap();
    let body: BodySchema = serde_json::from_str(&content).unwrap();

    // All 19 traits in strand A should be in [0, 1]
    assert!(body.dna.strandA.openness >= 0.0 && body.dna.strandA.openness <= 1.0);
    assert!(body.dna.strandA.dopamine_base >= 0.0 && body.dna.strandA.dopamine_base <= 1.0);
    assert!(body.dna.strandA.serotonin_base >= 0.0 && body.dna.strandA.serotonin_base <= 1.0);
    assert!(
        body.dna.strandA.norepinephrine_base >= 0.0 && body.dna.strandA.norepinephrine_base <= 1.0
    );
    assert!(body.dna.strandA.cortisol_sens >= 0.0 && body.dna.strandA.cortisol_sens <= 1.0);
    assert!(body.dna.strandA.novelty_seek >= 0.0 && body.dna.strandA.novelty_seek <= 1.0);
    assert!(body.dna.strandA.rumination >= 0.0 && body.dna.strandA.rumination <= 1.0);
    assert!(body.dna.strandA.exec_control >= 0.0 && body.dna.strandA.exec_control <= 1.0);
    assert!(body.dna.strandA.threat_bias >= 0.0 && body.dna.strandA.threat_bias <= 1.0);
    assert!(body.dna.strandA.episodic_gain >= 0.0 && body.dna.strandA.episodic_gain <= 1.0);
    assert!(body.dna.strandA.mem_decay >= 0.0 && body.dna.strandA.mem_decay <= 1.0);
    assert!(body.dna.strandA.trauma_sticky >= 0.0 && body.dna.strandA.trauma_sticky <= 1.0);
    assert!(body.dna.strandA.attachment >= 0.0 && body.dna.strandA.attachment <= 1.0);
    assert!(body.dna.strandA.trust_gain >= 0.0 && body.dna.strandA.trust_gain <= 1.0);
    assert!(body.dna.strandA.trust_decay >= 0.0 && body.dna.strandA.trust_decay <= 1.0);
    assert!(body.dna.strandA.jealousy >= 0.0 && body.dna.strandA.jealousy <= 1.0);
    assert!(body.dna.strandA.fatigue_sens >= 0.0 && body.dna.strandA.fatigue_sens <= 1.0);
    assert!(body.dna.strandA.pain_sens >= 0.0 && body.dna.strandA.pain_sens <= 1.0);
}

#[test]
fn expression_weights_sum_to_one() {
    let content = fs::read_to_string(FIXTURE_GEM_D).unwrap();
    let body: BodySchema = serde_json::from_str(&content).unwrap();

    let sum = body.dna.expression.weights.A + body.dna.expression.weights.B;
    assert!(
        (sum - 1.0).abs() < 0.001,
        "Expression weights should sum to 1.0, got {}",
        sum
    );
}

#[test]
fn invalid_dna_trait_fails_validation() {
    let mut body = BodySchema::default();
    body.dna.strandA.dopamine_base = 1.5; // Invalid: > 1.0

    let result = body.validate();
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("dopamine_base"));
}

#[test]
fn invalid_chromosome_pair_fails_validation() {
    use mk_core::human::schema::{Pair23Schema, SexChromosomeSchema, SexChromosomesSchema};

    let mut body = BodySchema::default();
    body.dna.chromosomes = SexChromosomesSchema {
        pair23: Pair23Schema {
            A: SexChromosomeSchema::Y,
            B: SexChromosomeSchema::Y, // Invalid: YY
        },
    };

    let result = body.validate();
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Invalid sex chromosome"));
}

#[test]
fn invalid_expression_weights_fails_validation() {
    let mut body = BodySchema::default();
    body.dna.expression.weights.A = 0.7;
    body.dna.expression.weights.B = 0.4; // Sum = 1.1, not 1.0

    let result = body.validate();
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("sum to 1.0"));
}

#[test]
fn serialization_roundtrip_preserves_values() {
    let content = fs::read_to_string(FIXTURE_GEM_D).unwrap();
    let original: BodySchema = serde_json::from_str(&content).unwrap();

    let serialized = serde_json::to_string(&original).unwrap();
    let deserialized: BodySchema = serde_json::from_str(&serialized).unwrap();

    assert_eq!(
        original.dna.strandA.dopamine_base,
        deserialized.dna.strandA.dopamine_base
    );
    assert_eq!(
        original.dna.strandB.serotonin_base,
        deserialized.dna.strandB.serotonin_base
    );
    assert_eq!(original.biological_sex(), deserialized.biological_sex());
}

#[test]
fn default_body_validates() {
    let body = BodySchema::default();
    body.validate().expect("default BodySchema should validate");
}

#[test]
fn gem_d_and_gem_k_have_different_dna() {
    let content_d = fs::read_to_string(FIXTURE_GEM_D).unwrap();
    let content_k = fs::read_to_string(FIXTURE_GEM_K).unwrap();

    let gem_d: BodySchema = serde_json::from_str(&content_d).unwrap();
    let gem_k: BodySchema = serde_json::from_str(&content_k).unwrap();

    // Different chromosomes
    assert_ne!(gem_d.biological_sex(), gem_k.biological_sex());

    // gem-d has higher dopamine (novelty seeking)
    assert!(gem_d.dna.strandA.novelty_seek > gem_k.dna.strandA.novelty_seek);

    // gem-k has higher serotonin (stability)
    assert!(gem_k.dna.strandA.serotonin_base > gem_d.dna.strandA.serotonin_base);
}
