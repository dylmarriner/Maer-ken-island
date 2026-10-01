use std::fs;
use std::path::PathBuf;

use mk_core::human::{HumanId, HumanSchema};
use mk_engine::runtime::{
    agent::{AgentId, AgentSystem, AgentWorldObservation},
    human::{BiologicalSex, HumanBeing, HumanFactory, HumanSystem},
};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/")
        .parent()
        .expect("repo root")
        .to_path_buf()
}

#[test]
fn runtime_surface_exposes_agent_layer() {
    let mut system = AgentSystem::new();
    system.register(mk_engine::runtime::Agent::spawn_at(
        AgentId::new("agent-1"),
        0,
        0,
    ));

    let observation = AgentWorldObservation::default();

    assert_eq!(system.population(), 1);
    assert_eq!(observation.tick, 0);
}

#[test]
fn runtime_surface_exposes_human_layer() {
    let mut system = HumanSystem::new();
    let human = HumanBeing::new("human-1".to_string(), BiologicalSex::Female);
    system.registry.add_human_no_storage(human);

    assert_eq!(system.registry.population_count(), 1);
}

#[test]
fn runtime_human_layer_accepts_canon_minimal_fixture_schema() {
    let path = repo_root().join("docs/canon/examples/humans/golden-human.min.json");
    let raw = fs::read_to_string(&path).expect("canon minimal human fixture must exist");
    let schema: HumanSchema =
        serde_json::from_str(&raw).expect("fixture must deserialize as HumanSchema");

    let human = HumanFactory::from_schema(HumanId::new(9001), schema.clone());
    assert_eq!(human.agent_id(), schema.agent_id);

    let mut system = HumanSystem::new();
    system.registry.add_human_no_storage(human);
    assert_eq!(system.registry.population_count(), 1);
}

#[test]
fn runtime_human_layer_accepts_canonical_json_constructor() {
    let path = repo_root().join("docs/canon/examples/humans/golden-human.min.json");
    let raw = fs::read_to_string(&path).expect("canon minimal human fixture must exist");

    let human = HumanBeing::from_canonical_json(HumanId::new(9002), &raw)
        .expect("from_canonical_json must accept golden fixture");

    assert_eq!(human.agent_id(), "golden.min.human");
}
