//! A death on the island: the body's carbon leaves the human stock through the
//! audit, the household stops counting the person, nobody re-registers them as
//! a newcomer, and the dead survive a snapshot.

use std::path::PathBuf;
use std::sync::Arc;

use mk_core::canon::CanonLocked;
use mk_core::flux::Reservoir;
use mk_core::human::{BiologicalSex, HumanStatus};
use mk_engine::io::island_snapshot::{load_island_snapshot, save_island_snapshot};
use mk_engine::regional::commands::{IslandCommand, IslandCreateHumanRequest};
use mk_engine::regional::world::IslandWorldState;
use mk_island::IslandScenario;

fn repo(p: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(p)
}

/// The engine's hard age limit (`humans::lifecycle::MAX_HUMAN_AGE_YEARS`, private).
const OLDEST_POSSIBLE_AGE_YEARS: f64 = 122.0;

fn world() -> IslandWorldState {
    let scenario = IslandScenario::load(&repo("fixtures/island/default_scenario.json")).unwrap();
    let canon = Arc::new(CanonLocked::load(&repo("fixtures/island/canon.json")).unwrap());
    IslandWorldState::new(canon, scenario).unwrap()
}

fn hex(w: &IslandWorldState) -> String {
    w.state_hash()
        .unwrap()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

#[test]
fn a_death_returns_the_bodys_carbon_and_is_not_undone_by_adoption() {
    let root = tempfile::tempdir().unwrap();
    let mut w = world();
    w.enable_human_store(root.path()).unwrap();
    // The founders are protected from natural death (governance veto), so
    // the person who dies is a settler.
    w.queue(IslandCommand::CreateHuman(IslandCreateHumanRequest {
        agent_id: "Elder".into(),
        sex: BiologicalSex::Male,
        age_years: 70.0,
        appearance: None,
        location: None,
    }))
    .unwrap();
    w.step(60).unwrap();
    assert!(w.life.materials.is_registered("Elder"));
    let carbon_before = w.life.materials.stock_of(Reservoir::HumanCarbon);

    // Past the oldest age anyone reaches: the real natural-age death path.
    w.life
        .humans
        .registry
        .get_human_mut("Elder")
        .unwrap()
        .development
        .age_years = OLDEST_POSSIBLE_AGE_YEARS + 1.0;
    for _ in 0..3 {
        w.step(60).unwrap();
    }
    let elder = w.life.humans.registry.get_human("Elder").unwrap();
    assert!(
        matches!(elder.profile.status, HumanStatus::Dead),
        "Elder should have died"
    );

    // The body left the human stock (through the audit, which would have
    // failed the step otherwise), the person no longer stands in the estate,
    // and the founders are untouched.
    assert!(
        !w.life.materials.is_registered("Elder"),
        "the body is still in the ledger"
    );
    assert!(!w.life.positions.0.contains_key("Elder"));
    assert!(w.life.materials.is_registered("Gem-K") && w.life.materials.is_registered("Gem-D"));
    let carbon_after = w.life.materials.stock_of(Reservoir::HumanCarbon);
    assert!(
        carbon_after < carbon_before - 1.0,
        "{carbon_before} -> {carbon_after}"
    );

    // A day and a firing later nobody has re-registered the dead as a newcomer
    // (which would also have drawn carbon from someone).
    for _ in 0..400 {
        w.step(60).unwrap();
    }
    assert!(
        !w.life.materials.is_registered("Elder"),
        "the dead were adopted again"
    );
    assert_eq!(w.life.shortfalls.food, 0, "the living still eat");
    assert_eq!(w.human_store_status().unwrap().errors, 0);

    // The death was written to the person's folder at once.
    let events = std::fs::read_to_string(
        w.human_store_status()
            .unwrap()
            .humans_dir
            .join("Elder/events/event_log.jsonl"),
    )
    .unwrap();
    assert!(events.contains("\"died\""), "{events}");

    // And the dead survive a snapshot.
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("dead.snap");
    save_island_snapshot(&w, &path).unwrap();
    let loaded = load_island_snapshot(&path).unwrap();
    assert_eq!(hex(&loaded), hex(&w));
    assert!(matches!(
        loaded
            .life
            .humans
            .registry
            .get_human("Elder")
            .unwrap()
            .profile
            .status,
        HumanStatus::Dead
    ));
}
