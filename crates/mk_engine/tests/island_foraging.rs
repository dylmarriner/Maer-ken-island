//! A person's meals and drinks are taken where they stand, not at the
//! estate: the carbon leaves the plants around them, and water comes from a
//! wet cell within their reach.

use std::path::PathBuf;
use std::sync::Arc;

use mk_core::canon::CanonLocked;
use mk_core::human::BiologicalSex;
use mk_engine::regional::create_human::{CreateLocation, IslandCreateHuman};
use mk_engine::regional::life::IslandLife;
use mk_engine::regional::materials::{cells_around, water_available_kg};
use mk_island::{DomainLevel, IslandScenario};

fn repo(p: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(p)
}

fn island() -> IslandLife {
    let scenario = IslandScenario::load(&repo("fixtures/island/default_scenario.json")).unwrap();
    let canon = Arc::new(CanonLocked::load(&repo("fixtures/island/canon.json")).unwrap());
    IslandLife::bootstrap(scenario, canon).unwrap()
}

#[test]
fn a_far_traveller_eats_and_drinks_where_they_stand() {
    let mut life = island();
    let medium = DomainLevel::Medium;
    let (rows, cols) = (life.domain.rows(medium), life.domain.cols(medium));
    let area = life.domain.cell_area_m2(medium);
    let home = life.placed.location;

    // A land cell with plants and, within two cells, standing water or a
    // stream: somewhere a person could really live, well away from the estate.
    let spot = (0..rows)
        .flat_map(|r| (0..cols).map(move |c| (r, c)))
        .find(|&(r, c)| {
            r.abs_diff(home.0).max(c.abs_diff(home.1)) > 40
                && *life.physical.geophysics.elevation_m.get(r, c) > 5.0
                && *life.ecology.biomass_kgc_m2.get(r, c) > 1.0
                && cells_around((r, c), 2, rows, cols)
                    .iter()
                    .any(|&cell| water_available_kg(&life.physical.hydrology, cell, area) > 10.0)
        })
        .expect("the island has such a place");

    let made = life
        .create_human(IslandCreateHuman {
            name: "Traveller".into(),
            biological_sex: BiologicalSex::Male,
            birth_timestamp: "2001-03-04T05:06:07Z".into(),
            age_years: 30.0,
            height_cm: 178.0,
            build: "average".into(),
            hair_color: "black".into(),
            eye_color: "brown".into(),
            skin_tone: "olive".into(),
            location: CreateLocation::IslandCell {
                row: spot.0,
                col: spot.1,
            },
            birthplace_here: true,
            birth_latitude: 0.0,
            birth_longitude: 0.0,
        })
        .expect("the traveller is created");
    let id = made.agent_id;

    // Meals are taken whenever the traveller chooses to eat, so watch two
    // days of substeps for a step in which their body gained carbon. Steps
    // in which the six-hourly ecology also ran are skipped: plant growth
    // there would hide where the meal came from.
    let six_hourly = life.scenario.cadences.hydrology_ecology_resource_seconds;
    let water_shortfalls_before = life.shortfalls.water;
    let mut meals_checked = 0;
    for _ in 0..(2 * 86_400 / 60) {
        let before: Vec<f64> = life.ecology.biomass_kgc_m2.data().to_vec();
        let carbon_before = life.materials.body_carbon_kg(&id).unwrap_or(0.0);
        life.advance(60).unwrap();
        let ate = life.materials.body_carbon_kg(&id).unwrap_or(0.0) > carbon_before + 1e-12;
        if !ate || life.sim_time_s.is_multiple_of(six_hourly) {
            continue;
        }
        let at = life.humans.registry.get_human(&id).unwrap().position;
        let stands = (at.row as usize, at.col as usize);
        assert!(
            stands.0.abs_diff(home.0).max(stands.1.abs_diff(home.1)) > 30,
            "the traveller should still be far from the estate, at {stands:?}"
        );
        let drained: Vec<(usize, usize)> = life
            .ecology
            .biomass_kgc_m2
            .data()
            .iter()
            .zip(&before)
            .enumerate()
            .filter(|(_, (now, was))| **now < **was - 1e-12)
            .map(|(i, _)| (i / cols, i % cols))
            .collect();
        assert!(
            drained
                .iter()
                .any(|&(r, c)| r.abs_diff(stands.0).max(c.abs_diff(stands.1)) <= 1),
            "the traveller's meal was not taken from the plants around {stands:?}; plants drained at {drained:?}"
        );
        meals_checked += 1;
    }
    assert!(meals_checked > 0, "the traveller never ate in two days");
    assert_eq!(
        life.shortfalls.water, water_shortfalls_before,
        "someone found no water within reach"
    );
}
