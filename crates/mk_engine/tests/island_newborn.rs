//! A child born during a run joins the household books: its body is
//! registered with carbon taken from its mother, it starts where she stands,
//! and from then on it eats, drinks and respires like everyone else.

use std::path::PathBuf;
use std::sync::Arc;

use mk_core::canon::CanonLocked;
use mk_core::flux::Reservoir;
use mk_core::human::schema::BirthRecordSchema;
use mk_core::human::BiologicalSex;
use mk_engine::humans::HumanBeing;
use mk_engine::regional::world::IslandWorldState;
use mk_island::IslandScenario;

fn repo(p: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(p)
}

fn world() -> IslandWorldState {
    let scenario = IslandScenario::load(&repo("fixtures/island/default_scenario.json")).unwrap();
    let canon = Arc::new(CanonLocked::load(&repo("fixtures/island/canon.json")).unwrap());
    IslandWorldState::new(canon, scenario).unwrap()
}

#[test]
fn a_child_born_mid_run_is_adopted_with_its_mothers_carbon_and_position() {
    let mut w = world();
    let mut child = HumanBeing::new("Baby-1".to_string(), BiologicalSex::Female);
    child.body.weight_kg = 3.3;
    child
        .profile
        .canonical_schema
        .as_mut()
        .unwrap()
        .reproductive_systems
        .genetics_system
        .birth_records
        .push(BirthRecordSchema {
            birth_id: "birth_test".into(),
            genotype_id: "genotype_test".into(),
            father_id: "Gem-D".into(),
            mother_id: "Gem-K".into(),
            birth_timestamp: "2017-01-01T00:00:00+00:00".into(),
            agent_id: "Baby-1".into(),
            mutations: vec![],
        });
    let before = w.life.materials.stock_of(Reservoir::HumanCarbon);
    assert!(!w.life.materials.is_registered("Baby-1"));
    w.life.humans.registry.add_human_no_storage(child);

    w.step(60).unwrap();

    assert!(
        w.life.materials.is_registered("Baby-1"),
        "the newborn is in the books"
    );
    let after = w.life.materials.stock_of(Reservoir::HumanCarbon);
    assert!(
        (after - before).abs() < 1e-9,
        "body carbon must be conserved by a birth: {before} -> {after}"
    );
    assert_eq!(
        w.life.positions.0.get("Baby-1"),
        w.life.positions.0.get("Gem-K"),
        "the newborn starts where its mother stands"
    );

    // A day later the child has eaten and breathed like everyone else.
    for _ in 0..(24 * 60) {
        w.step(60).unwrap();
    }
    assert!(w.life.humans.registry.get_human("Baby-1").is_some());
    assert_eq!(w.life.shortfalls.food, 0, "the household found food");
}
