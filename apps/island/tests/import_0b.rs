//! `--import-0b`: a Phase 0b population becomes `CreateHuman` commands.

use island_humans::{CreateHumanRequest, IslandHumanPopulation};
use mk_core::canon::CanonLocked;
use mk_engine::regional::world::IslandWorldState;
use mk_island::IslandScenario;
use std::path::PathBuf;
use std::sync::Arc;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn request(name: &str, sex: &str) -> CreateHumanRequest {
    CreateHumanRequest {
        name: name.into(),
        biological_sex: sex.into(),
        birth_timestamp: "1992-11-03T10:15:00+13:00".into(),
        birth_latitude: -41.3,
        birth_longitude: 174.8,
        age_years: 33.9,
        height_cm: 166.0,
        build: "Athletic".into(),
        hair_color: "Black".into(),
        eye_color: "Brown".into(),
        skin_tone: "Medium".into(),
    }
}

#[test]
fn a_phase_0b_population_is_imported_once_into_a_fresh_island() {
    let data = tempfile::tempdir().unwrap();
    {
        let (mut population, _) = IslandHumanPopulation::open(data.path(), [3u8; 32]).unwrap();
        population
            .create_human(request("Hine Moana", "female"), "test")
            .unwrap();
        population
            .create_human(request("Tama Rangi", "male"), "test")
            .unwrap();
    }
    let scenario =
        IslandScenario::load(&root().join("fixtures/island/default_scenario.json")).unwrap();
    let canon = Arc::new(CanonLocked::load(&root().join("fixtures/island/canon.json")).unwrap());
    let mut world = IslandWorldState::new(canon, scenario).unwrap();

    let (queued, refused) = island_runner::import_0b(&mut world, data.path()).unwrap();
    assert_eq!((queued, refused.len()), (2, 0), "{refused:?}");
    world.step(60).unwrap();
    let ids: Vec<String> = world
        .life
        .humans
        .registry
        .iter()
        .map(|h| h.agent_id().to_string())
        .collect();
    assert!(
        ids.contains(&"hine-moana".to_string()) && ids.contains(&"tama-rangi".to_string()),
        "{ids:?}"
    );
    assert_eq!(ids.len(), 4, "two founders and two imported people");

    // A second import, or an import into an island that has run, is refused.
    let again = island_runner::import_0b(&mut world, data.path());
    assert!(again.is_err(), "the island has already run");
}
