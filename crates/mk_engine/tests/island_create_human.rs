//! Phase 4 Task 6: creating a person in the running island.

use std::path::PathBuf;
use std::sync::Arc;

use mk_core::canon::CanonLocked;
use mk_core::human::BiologicalSex;
use mk_engine::regional::create_human::{CreateHumanError, CreateLocation, IslandCreateHuman};
use mk_engine::regional::estate_layout::SpaceId;
use mk_engine::regional::life::IslandLife;
use mk_island::IslandScenario;

fn repo(path: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path)
}

fn island() -> IslandLife {
    let mut scenario =
        IslandScenario::load(&repo("fixtures/island/default_scenario.json")).unwrap();
    scenario.estate_patch.tree_cap = 50;
    let canon = Arc::new(CanonLocked::load(&repo("fixtures/island/canon.json")).unwrap());
    IslandLife::bootstrap(scenario, canon).expect("the island bootstraps")
}

fn somebody(name: &str, location: CreateLocation) -> IslandCreateHuman {
    IslandCreateHuman {
        name: name.to_string(),
        biological_sex: BiologicalSex::Female,
        birth_timestamp: "2001-03-04T05:06:07Z".to_string(),
        age_years: 25.0,
        height_cm: 168.0,
        build: "average".to_string(),
        hair_color: "black".to_string(),
        eye_color: "brown".to_string(),
        skin_tone: "olive".to_string(),
        location,
        birthplace_here: true,
        birth_latitude: 0.0,
        birth_longitude: 0.0,
    }
}

/// The first space in the estate's layout, whatever it is called.
fn a_space(life: &IslandLife) -> (SpaceId, String) {
    let space = &life.placed.layout.spaces[0];
    (space.id, space.label.clone())
}

#[test]
fn somebody_created_in_a_room_is_in_that_room_and_in_the_world() {
    let mut life = island();
    let (space, label) = a_space(&life);
    let before = life.humans.registry.iter().count();

    let made = life
        .create_human(somebody("Ihaka", CreateLocation::EstateSpace(space)))
        .expect("the request is good");

    assert_eq!(made.space.as_deref(), Some(label.as_str()));
    assert_eq!(life.humans.registry.iter().count(), before + 1);
    assert!(
        life.humans.registry.get_human(&made.agent_id).is_some(),
        "created but not in the registry"
    );
    // They stand where the layout puts that space, inside the estate patch.
    let at = life
        .positions
        .0
        .get(&made.agent_id)
        .expect("no position on the estate");
    assert_eq!(Some(at.position_m), made.position_m);
    assert_eq!(made.cell, life.placed.location);

    // `birthplace_here` put their birthplace at the estate rather than at
    // the zeroes the request carried.
    assert!(made.birth_latitude.abs() > 0.1 || made.birth_longitude.abs() > 0.1);

    // And they are really in the world: a step does not throw them out.
    life.advance(life.scenario.cadences.human_seconds).unwrap();
    assert!(life.positions.0.contains_key(&made.agent_id));

    // They breathe. Being visible on a page is not being alive: without a
    // body in the material ledger a created person would respire nothing,
    // eat nothing and carry no carbon, while looking exactly like somebody
    // who does.
    let carbon = life
        .materials
        .body_carbon_kg(&made.agent_id)
        .expect("a created person has no body in the material ledger");
    assert!(carbon > 1.0, "their body carries {carbon} kg of carbon");

    // An hour of breathing moves it, as it does for the founders.
    life.advance(3_600).unwrap();
    let after = life.materials.body_carbon_kg(&made.agent_id).unwrap();
    assert!(
        after < carbon,
        "an hour passed and they respired nothing: {carbon} then {after}"
    );
}

#[test]
fn the_same_request_at_the_same_tick_makes_the_same_person() {
    // Creation draws on the world's keyed streams at the current tick, so
    // two islands that have run the same distance must build byte-identical
    // people. Without this, a replay would not reproduce a creation.
    let (mut a, mut b) = (island(), island());
    let step = a.scenario.cadences.human_seconds;
    a.advance(3 * step).unwrap();
    b.advance(3 * step).unwrap();

    let (space, _) = a_space(&a);
    let made_a = a
        .create_human(somebody("Awhina", CreateLocation::EstateSpace(space)))
        .unwrap();
    let made_b = b
        .create_human(somebody("Awhina", CreateLocation::EstateSpace(space)))
        .unwrap();

    assert_eq!(made_a.agent_id, made_b.agent_id);
    let one = a.humans.registry.get_human(&made_a.agent_id).unwrap();
    let two = b.humans.registry.get_human(&made_b.agent_id).unwrap();
    assert_eq!(
        serde_json::to_value(one).unwrap(),
        serde_json::to_value(two).unwrap(),
        "the same request at the same tick built two different people"
    );
    assert_eq!(
        a.state_digest(),
        b.state_digest(),
        "the islands diverged after an identical creation"
    );
}

#[test]
fn two_people_of_one_name_get_their_own_ids() {
    let mut life = island();
    let (space, _) = a_space(&life);
    let first = life
        .create_human(somebody("Mere", CreateLocation::EstateSpace(space)))
        .unwrap();
    let second = life
        .create_human(somebody("Mere", CreateLocation::EstateSpace(space)))
        .unwrap();
    assert_ne!(first.agent_id, second.agent_id);
    assert_eq!(life.humans.registry.iter().count(), 4);
}

#[test]
fn a_cell_on_land_works_and_the_sea_does_not() {
    let mut life = island();
    let (row, col) = life.placed.location;

    let ashore = life
        .create_human(somebody("Tane", CreateLocation::IslandCell { row, col }))
        .map(|_| ())
        .err();
    assert!(
        ashore.is_none(),
        "the estate's own cell is not land: {ashore:?}"
    );

    // Find a sea cell, and refuse it.
    let mut sea = None;
    'outer: for r in 0..life.domain.rows(mk_island::DomainLevel::Medium) {
        for c in 0..life.domain.cols(mk_island::DomainLevel::Medium) {
            if !*life.physical.geophysics.land_mask.get(r, c) {
                sea = Some((r, c));
                break 'outer;
            }
        }
    }
    let (r, c) = sea.expect("an island has a sea around it");
    match life.create_human(somebody(
        "Tangaroa",
        CreateLocation::IslandCell { row: r, col: c },
    )) {
        Err(CreateHumanError::Invalid(problems)) => {
            assert!(
                problems.iter().any(|p| p.contains("in the sea")),
                "the refusal does not say why: {problems:?}"
            );
        }
        other => panic!("somebody was created in the sea: {other:?}"),
    }
}

#[test]
fn every_bad_field_is_named_and_they_are_named_together() {
    let mut life = island();
    let (space, _) = a_space(&life);

    let mut request = somebody("", CreateLocation::EstateSpace(space));
    request.age_years = 900.0;
    request.height_cm = 10.0;

    match life.create_human(request) {
        Err(CreateHumanError::Invalid(problems)) => {
            let all = problems.join(" | ");
            assert!(all.contains("name"), "{all}");
            assert!(all.contains("age_years"), "{all}");
            assert!(all.contains("height_cm"), "{all}");
            assert!(
                problems.len() >= 3,
                "a form should learn every problem at once, got {problems:?}"
            );
        }
        other => panic!("a nonsense request was accepted: {other:?}"),
    }
    assert_eq!(
        life.humans.registry.iter().count(),
        2,
        "a refused creation still added somebody"
    );
}

#[test]
fn a_room_that_does_not_exist_is_refused_and_says_what_does() {
    let mut life = island();
    match life.create_human(somebody(
        "Nowhere",
        CreateLocation::EstateSpace(SpaceId(9_999)),
    )) {
        Err(CreateHumanError::Invalid(problems)) => {
            let all = problems.join(" | ");
            assert!(all.contains("no space 9999"), "{all}");
            assert!(
                all.contains("Bedroom") || all.contains("Hall") || all.contains("Workshop"),
                "the refusal does not list the spaces there are: {all}"
            );
        }
        other => panic!("a space that does not exist was accepted: {other:?}"),
    }
}

#[test]
fn an_age_nobody_has_reached_is_refused_but_an_old_age_is_not() {
    let mut life = island();
    let (space, _) = a_space(&life);

    let mut old = somebody("Kuia", CreateLocation::EstateSpace(space));
    old.age_years = 110.0;
    assert!(
        life.create_human(old).is_ok(),
        "110 is old, not impossible — people have lived longer"
    );

    let mut impossible = somebody("Methuselah", CreateLocation::EstateSpace(space));
    impossible.age_years = 130.0;
    match life.create_human(impossible) {
        Err(CreateHumanError::Invalid(problems)) => assert!(
            problems.iter().any(|p| p.contains("122.45")),
            "the refusal does not cite the verified maximum: {problems:?}"
        ),
        other => panic!("an impossible age was accepted: {other:?}"),
    }
}

#[test]
fn somebody_born_here_is_born_at_this_islands_coordinates() {
    // `IslandDomain::lat_lon_at_m` answers in radians, because its other
    // caller does trigonometry with the result. A birthplace is degrees.
    // The first version of `create_human` passed the radians straight
    // through, so somebody born on an island at 41° S was recorded as born
    // at 0.716° S — and nothing on the island reads a birthplace back, so
    // every test still passed. This is the one that would not have.
    let mut life = island();
    let (space, _) = a_space(&life);
    let mut request = somebody("Tane", CreateLocation::EstateSpace(space));
    request.birthplace_here = true;
    let made = life.create_human(request).expect("the request is good");

    let reference = life.domain.profile().reference_latitude_deg;
    assert!(
        (made.birth_latitude - reference).abs() < 1.0,
        "born at {:.4}° but the island is at {reference:.4}°",
        made.birth_latitude
    );
    assert!(
        (-90.0..=90.0).contains(&made.birth_latitude)
            && (-180.0..=180.0).contains(&made.birth_longitude),
        "a birthplace has to be a place on a planet: {:.4}, {:.4}",
        made.birth_latitude,
        made.birth_longitude
    );

    // And on a cell, not only on the estate.
    let (row, col) = life.placed.location;
    let mut request = somebody("Aroha", CreateLocation::IslandCell { row, col });
    request.birthplace_here = true;
    let made = life.create_human(request).expect("the request is good");
    assert!(
        (made.birth_latitude - reference).abs() < 1.0,
        "born at {:.4}° but the island is at {reference:.4}°",
        made.birth_latitude
    );
}
