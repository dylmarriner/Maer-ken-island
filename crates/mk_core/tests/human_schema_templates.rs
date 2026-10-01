use std::fs;
use std::path::PathBuf;

use mk_core::human::schema::{
    ArchitecturalLawsSchema, AttentionSystemSchema, BodySchema, ComprehensiveEmotionTaxonomySchema,
    ConsciousnessSchema, CoreSystemsSchema, CreativeSystemsSchema, CurrentCognitionSchema,
    CurrentEmotionSchema, DarkTriadSchema, DecisionMakingSchema, DeterministicArchitectureSchema,
    ExtremeBrainDetailSchema, GranularEmotionsSchema, ImmuneSystemSchema, ImmutableStateSchema,
    LearningAdaptationSchema, LegacyMemorySystemsSchema, ReproductiveSystemsSchema, RuntimeSchema,
    SensorySystemsSchema, SkinSystemSchema, SocialCognitionSchema, SystemDependenciesSchema,
};
use mk_core::human::{HumanId, HumanProfile, HumanSchema};
use serde::de::DeserializeOwned;
use serde::Deserialize;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/")
        .parent()
        .expect("repo root")
        .to_path_buf()
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct CoreSystemsFamilyFixture {
    core_systems: CoreSystemsSchema,
    architectural_laws: ArchitecturalLawsSchema,
    deterministic_architecture: DeterministicArchitectureSchema,
    immutable_state: ImmutableStateSchema,
    system_dependencies: SystemDependenciesSchema,
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct CognitionFamilyFixture {
    memory_systems: LegacyMemorySystemsSchema,
    attention_system: AttentionSystemSchema,
    consciousness: ConsciousnessSchema,
    learning_adaptation: LearningAdaptationSchema,
    social_cognition: SocialCognitionSchema,
    creative_systems: CreativeSystemsSchema,
    decision_making: DecisionMakingSchema,
    cognition: CurrentCognitionSchema,
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct EmotionFamilyFixture {
    comprehensive_emotion_taxonomy: ComprehensiveEmotionTaxonomySchema,
    granular_emotions: GranularEmotionsSchema,
    dark_triad: DarkTriadSchema,
    emotion: CurrentEmotionSchema,
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct BiologyFamilyFixture {
    immune_system: ImmuneSystemSchema,
    skin_system: SkinSystemSchema,
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct SensoryFamilyFixture {
    sensory_systems: SensorySystemsSchema,
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct ReproductiveFamilyFixture {
    reproductive_systems: ReproductiveSystemsSchema,
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct BrainFamilyFixture {
    extreme_brain_detail: ExtremeBrainDetailSchema,
}

fn read_json_value(path: &str) -> serde_json::Value {
    let raw = fs::read_to_string(repo_root().join(path)).expect("fixture should exist");
    serde_json::from_str(&raw).expect("fixture should parse as json")
}

fn assert_family_valid<T: DeserializeOwned>(bundle: &serde_json::Value, family: &str) {
    let fixture = bundle
        .get(family)
        .cloned()
        .unwrap_or_else(|| panic!("missing family fixture: {family}"));
    serde_json::from_value::<T>(fixture)
        .unwrap_or_else(|err| panic!("family fixture {family} should deserialize: {err}"));
}

fn assert_family_invalid<T: DeserializeOwned>(bundle: &serde_json::Value, family: &str) {
    let fixture = bundle
        .get(family)
        .cloned()
        .unwrap_or_else(|| panic!("missing family fixture: {family}"));
    let parsed = serde_json::from_value::<T>(fixture);
    assert!(
        parsed.is_err(),
        "family fixture {family} should fail deserialization"
    );
}

#[test]
fn golden_min_human_fixture_deserializes() {
    let path = repo_root().join("docs/canon/examples/humans/golden-human.min.json");
    let raw = fs::read_to_string(path).expect("fixture should exist");
    let schema: HumanSchema =
        serde_json::from_str(&raw).expect("minimal fixture should deserialize");
    assert_eq!(schema.agent_id, "golden.min.human");
    assert_eq!(schema.core_identity.agent_id, "golden.min.human");
}

#[test]
fn golden_full_human_fixture_deserializes_and_maps_to_profile() {
    let path = repo_root().join("docs/canon/examples/humans/golden-human.full.json");
    let raw = fs::read_to_string(path).expect("fixture should exist");
    let schema: HumanSchema = serde_json::from_str(&raw).expect("full fixture should deserialize");
    let profile = HumanProfile::from_canonical_schema(HumanId::new(101), schema.clone());

    assert_eq!(schema.agent_id, "golden.full.human");
    assert_eq!(profile.agent_id, "golden.full.human");
    assert!(profile.canonical_schema().is_some());
}

#[test]
fn invalid_human_fixture_is_rejected() {
    let path = repo_root().join("docs/canon/examples/humans/invalid-missing-agent-id.json");
    let raw = fs::read_to_string(path).expect("invalid fixture should exist");
    let parsed = serde_json::from_str::<HumanSchema>(&raw);
    assert!(
        parsed.is_err(),
        "invalid fixture should fail deserialization"
    );
}

#[test]
fn schema_family_valid_fixtures_deserialize() {
    let bundle = read_json_value("docs/canon/examples/humans/schema-family-fixtures.valid.json");

    assert_family_valid::<BodySchema>(&bundle, "body");
    assert_family_valid::<RuntimeSchema>(&bundle, "runtime");
    assert_family_valid::<CoreSystemsFamilyFixture>(&bundle, "core_systems");
    assert_family_valid::<CognitionFamilyFixture>(&bundle, "cognition");
    assert_family_valid::<EmotionFamilyFixture>(&bundle, "emotion");
    assert_family_valid::<BiologyFamilyFixture>(&bundle, "biology");
    assert_family_valid::<SensoryFamilyFixture>(&bundle, "sensory");
    assert_family_valid::<ReproductiveFamilyFixture>(&bundle, "reproductive");
    assert_family_valid::<BrainFamilyFixture>(&bundle, "brain");
}

#[test]
fn schema_family_invalid_fixtures_are_rejected() {
    let bundle = read_json_value("docs/canon/examples/humans/schema-family-fixtures.invalid.json");

    assert_family_invalid::<BodySchema>(&bundle, "body");
    assert_family_invalid::<RuntimeSchema>(&bundle, "runtime");
    assert_family_invalid::<CoreSystemsFamilyFixture>(&bundle, "core_systems");
}

/// Runtime mapping test: every schema category has declared consuming subsystem
#[test]
fn every_schema_family_has_consuming_subsystem() {
    // Define all schema families from human schema
    let schema_families = vec![
        "CoreSystems",
        "ArchitecturalLaws",
        "DeterministicArchitecture",
        "ImmutableState",
        "SystemDependencies",
        "Cognition",
        "MemorySystems",
        "AttentionSystem",
        "Consciousness",
        "LearningAdaptation",
        "SocialCognition",
        "CreativeSystems",
        "DecisionMaking",
        "Emotion",
        "ComprehensiveEmotionTaxonomy",
        "GranularEmotions",
        "DarkTriad",
        "Body",
        "ImmuneSystem",
        "SkinSystem",
        "Sensory",
        "Reproductive",
        "Brain",
        "Runtime",
    ];

    // Define engine subsystems that consume schema families
    // This mapping should be kept in sync with actual implementation
    let consuming_subsystems = vec![
        ("CoreSystems", "mk_engine::agents::AgentSystem"),
        (
            "ArchitecturalLaws",
            "mk_engine::agents::behavior::BehaviorSnapshot",
        ),
        (
            "DeterministicArchitecture",
            "mk_engine::agents::cognition::CognitiveSnapshot",
        ),
        (
            "ImmutableState",
            "mk_engine::agents::mortality::MortalitySnapshot",
        ),
        (
            "SystemDependencies",
            "mk_engine::agents::nervous_system::NervousSystemSnapshot",
        ),
        (
            "Cognition",
            "mk_engine::agents::cognition::CognitiveSnapshot",
        ),
        ("MemorySystems", "mk_engine::humans::HumanCognitionSnapshot"),
        (
            "AttentionSystem",
            "mk_engine::agents::cognition::CognitiveSnapshot",
        ),
        (
            "Consciousness",
            "mk_engine::agents::cognition::CognitiveSnapshot",
        ),
        (
            "LearningAdaptation",
            "mk_engine::agents::cognition::CognitiveSnapshot",
        ),
        (
            "SocialCognition",
            "mk_engine::humans::SocialSystemsSnapshot",
        ),
        (
            "CreativeSystems",
            "mk_engine::agents::cognition::CognitiveSnapshot",
        ),
        (
            "DecisionMaking",
            "mk_engine::agents::behavior::BehaviorSnapshot",
        ),
        ("Emotion", "mk_engine::agents::affect::AffectSnapshot"),
        (
            "ComprehensiveEmotionTaxonomy",
            "mk_engine::agents::affect::AffectSnapshot",
        ),
        (
            "GranularEmotions",
            "mk_engine::agents::affect::AffectSnapshot",
        ),
        ("DarkTriad", "mk_engine::agents::affect::AffectSnapshot"),
        ("Body", "mk_engine::agents::physiology::PhysiologySnapshot"),
        (
            "ImmuneSystem",
            "mk_engine::agents::physiology::PhysiologySnapshot",
        ),
        (
            "SkinSystem",
            "mk_engine::agents::physiology::PhysiologySnapshot",
        ),
        (
            "Sensory",
            "mk_engine::agents::nervous_system::NervousSystemSnapshot",
        ),
        (
            "Reproductive",
            "mk_engine::agents::endocrinology::EndocrineSnapshot",
        ),
        ("Brain", "mk_engine::agents::cognition::CognitiveSnapshot"),
        ("Runtime", "mk_engine::runtime::human::HumanSystem"),
    ];

    // Verify every schema family has at least one consuming subsystem
    for family in &schema_families {
        let has_consumer = consuming_subsystems.iter().any(|(f, _)| *f == *family);
        assert!(
            has_consumer,
            "Schema family '{}' has no declared consuming subsystem",
            family
        );
    }

    // Verify consuming subsystems reference valid schema families
    for (family, _subsystem) in &consuming_subsystems {
        assert!(
            schema_families.contains(family),
            "Consuming subsystem references unknown schema family: '{}'",
            family
        );
    }
}
