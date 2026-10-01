use std::path::PathBuf;

use mk_core::human::HumanSchema;

/// Canon human example bundles under `docs/canon/examples/humans/`.
/// Kept in one place so filesystem and serde gates stay aligned.
const CANON_HUMAN_EXAMPLE_PATHS: &[&str] = &[
    "docs/canon/examples/humans/golden-human.min.json",
    "docs/canon/examples/humans/golden-human.full.json",
    "docs/canon/examples/humans/invalid-missing-agent-id.json",
    "docs/canon/examples/humans/schema-family-fixtures.valid.json",
    "docs/canon/examples/humans/schema-family-fixtures.invalid.json",
];

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/")
        .parent()
        .expect("repo root")
        .to_path_buf()
}

#[test]
fn canon_human_example_fixtures_exist_on_disk() {
    for rel in CANON_HUMAN_EXAMPLE_PATHS {
        let path = repo_root().join(rel);
        assert!(
            path.is_file(),
            "expected canon human example fixture at {}",
            path.display()
        );
    }
}

#[test]
fn canonical_minimal_human_has_aligned_identity_fields() {
    let schema = HumanSchema::canonical_minimal("contract.agent");
    assert_eq!(schema.agent_id, "contract.agent");
    assert_eq!(schema.core_identity.agent_id, "contract.agent");
    assert_eq!(schema.identity.name, "contract.agent");
}

#[test]
fn canonical_minimal_human_serializes_with_schema_metadata() {
    let schema = HumanSchema::canonical_minimal("contract.meta");
    let value = serde_json::to_value(&schema).expect("schema serializes");

    assert_eq!(
        value.get("schema_version").and_then(|v| v.as_str()),
        Some(schema.schema_version.as_str())
    );
    assert!(value.get("metadata").is_some());
    assert!(value.get("core_systems").is_some());
    assert!(value.get("extreme_brain_detail").is_some());
}

#[test]
fn canonical_minimal_round_trips_through_json() {
    let schema = HumanSchema::canonical_minimal("roundtrip.agent");
    let value = serde_json::to_value(&schema).expect("serialize");
    let back: HumanSchema = serde_json::from_value(value).expect("deserialize");
    assert_eq!(back.agent_id, "roundtrip.agent");
    assert_eq!(back.core_identity.agent_id, "roundtrip.agent");
}
