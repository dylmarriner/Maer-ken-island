//! A death on the island: the body's carbon leaves the human stock through the
//! audit, the household stops counting the person, nobody re-registers them as
//! a newcomer, and the dead survive a snapshot.

use std::path::PathBuf;
use std::sync::Arc;

use mk_core::canon::CanonLocked;
use mk_core::flux::Reservoir;
use mk_core::human::{BiologicalSex, HumanStatus};
use mk_engine::io::island_snapshot::{load_island_snapshot, save_island_snapshot};
use mk_engine::regional::create_human::{CreateLocation, IslandCreateHuman};
use mk_engine::regional::life::IslandLife;
use mk_island::IslandScenario;

fn repo(p: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(p)
}

/// The engine's hard age limit (`humans::lifecycle::MAX_HUMAN_AGE_YEARS`, private).
const OLDEST_POSSIBLE_AGE_YEARS: f64 = 122.0;

fn canon() -> Arc<CanonLocked> {
    Arc::new(CanonLocked::load(&repo("fixtures/island/canon.json")).unwrap())
}

fn island() -> IslandLife {
    let scenario = IslandScenario::load(&repo("fixtures/island/default_scenario.json")).unwrap();
    IslandLife::bootstrap(scenario, canon()).unwrap()
}

fn hex(life: &IslandLife) -> String {
    life.state_digest()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

#[test]
fn a_death_returns_the_bodys_carbon_and_is_not_undone_by_adoption() {
    let root = tempfile::tempdir().unwrap();
    let mut w = island();
    w.enable_human_store(root.path()).unwrap();
    // The founders are protected from natural death (governance veto), so
    // the person who dies is a settler.
    // Outside the founders' house, which is theirs and their children's alone.
    let yard = w.placed.layout.spaces_outside_house().next().unwrap().id;
    let elder = w
        .create_human(IslandCreateHuman {
            name: "Elder".into(),
            biological_sex: BiologicalSex::Male,
            birth_timestamp: "1931-03-04T05:06:07Z".into(),
            age_years: 70.0,
            height_cm: 172.0,
            build: "average".into(),
            hair_color: "grey".into(),
            eye_color: "brown".into(),
            skin_tone: "olive".into(),
            location: CreateLocation::EstateSpace(yard),
            birthplace_here: true,
            birth_latitude: 0.0,
            birth_longitude: 0.0,
        })
        .expect("the elder is created")
        .agent_id;
    let elder_id = elder.as_str();
    w.advance(60).unwrap();
    assert!(w.materials.is_registered(elder_id));
    let carbon_before = w.materials.stock_of(Reservoir::HumanCarbon);

    // Past the oldest age anyone reaches: the real natural-age death path.
    w.humans
        .registry
        .get_human_mut(elder_id)
        .unwrap()
        .development
        .age_years = OLDEST_POSSIBLE_AGE_YEARS + 1.0;
    for _ in 0..3 {
        w.advance(60).unwrap();
    }
    let elder = w.humans.registry.get_human(elder_id).unwrap();
    assert!(
        matches!(elder.profile.status, HumanStatus::Dead),
        "Elder should have died"
    );

    // The body left the human stock (through the audit, which would have
    // failed the step otherwise), the person no longer stands in the estate,
    // and the founders are untouched.
    assert!(
        !w.materials.is_registered(elder_id),
        "the body is still in the ledger"
    );
    assert!(!w.positions.0.contains_key(elder_id));
    assert!(w.materials.is_registered("Gem-K") && w.materials.is_registered("Gem-D"));
    let carbon_after = w.materials.stock_of(Reservoir::HumanCarbon);
    assert!(
        carbon_after < carbon_before - 1.0,
        "{carbon_before} -> {carbon_after}"
    );

    // A day and a firing later nobody has re-registered the dead as a newcomer
    // (which would also have drawn carbon from someone).
    for _ in 0..400 {
        w.advance(60).unwrap();
    }
    assert!(
        !w.materials.is_registered(elder_id),
        "the dead were adopted again"
    );
    assert_eq!(w.shortfalls.food, 0, "the living still eat");
    assert_eq!(w.human_store().unwrap().failures().0, 0);

    // The death was written to the person's folder at once.
    let events = std::fs::read_to_string(
        w.human_store()
            .unwrap()
            .humans_dir()
            .join(elder_id)
            .join("events/event_log.jsonl"),
    )
    .unwrap();
    assert!(events.contains("\"died\""), "{events}");

    // And the dead survive a snapshot.
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("dead.snap");
    save_island_snapshot(&mut w, &path).unwrap();
    let loaded = load_island_snapshot(canon(), &path).unwrap();
    assert_eq!(hex(&loaded), hex(&w));
    assert!(matches!(
        loaded
            .humans
            .registry
            .get_human(elder_id)
            .unwrap()
            .profile
            .status,
        HumanStatus::Dead
    ));
}
