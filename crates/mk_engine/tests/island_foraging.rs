//! A person's meals and drinks are taken where they stand, not at the
//! estate: the carbon leaves the plants around them, and water comes from a
//! wet cell within their reach.

use std::path::PathBuf;
use std::sync::Arc;

use mk_core::canon::CanonLocked;
use mk_core::human::BiologicalSex;
use mk_engine::regional::commands::{CreateLocation, IslandCommand, IslandCreateHumanRequest};
use mk_engine::regional::materials::{cells_around, water_available_kg};
use mk_engine::regional::world::IslandWorldState;
use mk_island::{DomainLevel, IslandScenario};

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
fn a_far_traveller_eats_and_drinks_where_they_stand() {
    let mut w = world();
    let medium = DomainLevel::Medium;
    let (rows, cols) = (w.life.domain.rows(medium), w.life.domain.cols(medium));
    let area = w.life.domain.cell_area_m2(medium);
    let home = w.life.placed.location;

    // A land cell with plants and, within two cells, standing water or a
    // stream: somewhere a person could really live, well away from the estate.
    let spot = (0..rows)
        .flat_map(|r| (0..cols).map(move |c| (r, c)))
        .find(|&(r, c)| {
            r.abs_diff(home.0).max(c.abs_diff(home.1)) > 40
                && *w.life.physical.geophysics.elevation_m.get(r, c) > 5.0
                && *w.life.ecology.biomass_kgc_m2.get(r, c) > 1.0
                && cells_around((r, c), 2, rows, cols)
                    .iter()
                    .any(|&cell| water_available_kg(&w.life.physical.hydrology, cell, area) > 10.0)
        })
        .expect("the island has such a place");

    w.queue(IslandCommand::CreateHuman(IslandCreateHumanRequest {
        agent_id: "Traveller".into(),
        sex: BiologicalSex::Male,
        age_years: 30.0,
        appearance: None,
        location: Some(CreateLocation::IslandCell {
            row: spot.0,
            col: spot.1,
        }),
    }))
    .unwrap();

    // The first six-hourly firing is at tick 360; look at the step before it
    // and the step that includes it.
    for _ in 0..359 {
        w.step(60).unwrap();
    }
    let before: Vec<f64> = w.life.ecology.biomass_kgc_m2.data().to_vec();
    let water_shortfalls_before = w.life.shortfalls.water;
    w.step(60).unwrap();
    assert_eq!(w.tick(), 360);

    let at = w
        .life
        .humans
        .registry
        .get_human("Traveller")
        .unwrap()
        .position;
    let stands = (at.row as usize, at.col as usize);
    assert!(
        stands.0.abs_diff(home.0).max(stands.1.abs_diff(home.1)) > 30,
        "the traveller should still be far from the estate, at {stands:?}"
    );
    let cols_n = cols;
    let drained: Vec<(usize, usize)> = w
        .life
        .ecology
        .biomass_kgc_m2
        .data()
        .iter()
        .zip(&before)
        .enumerate()
        .filter(|(_, (now, was))| **now < **was - 1e-12)
        .map(|(i, _)| (i / cols_n, i % cols_n))
        .collect();
    assert!(
        drained
            .iter()
            .any(|&(r, c)| r.abs_diff(stands.0).max(c.abs_diff(stands.1)) <= 1),
        "the traveller's meal was not taken from the plants around {stands:?}; plants drained at {drained:?}"
    );
    assert_eq!(
        w.life.shortfalls.water, water_shortfalls_before,
        "the traveller found no water within reach"
    );
}
