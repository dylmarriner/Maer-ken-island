//! How much water the household's drinking cell offers through a year,
//! measured on the physics alone (no one drawing), against three people's
//! 0.65 kg drinks every six hours. Slow tier; prints the supply statistics.

use std::path::PathBuf;
use std::sync::Arc;

use mk_core::canon::CanonLocked;
use mk_engine::regional::life::IslandLife;
use mk_engine::regional::materials::{water_available_kg, water_search_cells};
use mk_island::{DomainLevel, IslandScenario};

fn repo(p: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(p)
}

#[test]
#[ignore = "slow: one simulated year of physics"]
fn the_drinking_cell_supplies_a_household_all_year() {
    let scenario = IslandScenario::load(&repo("fixtures/island/default_scenario.json")).unwrap();
    let canon = Arc::new(CanonLocked::load(&repo("fixtures/island/canon.json")).unwrap());
    let mut life = IslandLife::bootstrap(scenario, canon).unwrap();
    let (r, c) = life.water_cell.expect("a drinking cell exists");
    let area = life.domain.cell_area_m2(DomainLevel::Medium);
    let dt = life.scenario.cadences.weather_ocean_seconds;
    let need_kg = 3.0 * 0.65;
    let (rows, cols) = (
        life.domain.rows(DomainLevel::Medium),
        life.domain.cols(DomainLevel::Medium),
    );
    let (mut min_kg, mut short, mut dry, mut n, mut usual_dry) = (f64::INFINITY, 0, 0, 0, 0);
    for hour in 1..=(365 * 24u64) {
        let t = (hour * dt) as f64;
        life.physical
            .step(&life.canon, &life.domain, life.scenario.seed, t, hour, dt)
            .unwrap();
        if hour % 6 == 0 {
            let usual = water_available_kg(&life.physical.hydrology, (r, c), area);
            // What the household really has: the usual cell, else a neighbour.
            let kg = water_search_cells((r, c), rows, cols)
                .into_iter()
                .map(|cell| water_available_kg(&life.physical.hydrology, cell, area))
                .find(|kg| *kg > 0.0)
                .unwrap_or(0.0);
            n += 1;
            usual_dry += usize::from(usual <= 0.0);
            min_kg = min_kg.min(kg);
            short += usize::from(kg < need_kg);
            dry += usize::from(kg <= 0.0);
            if kg < need_kg {
                println!(
                    "WATER still short at hour {hour} (day {:.2}): usual cell {usual:.1} kg",
                    hour as f64 / 24.0
                );
            }
        }
    }
    println!("WATER cell ({r},{c}): {n} draws; usual cell dry {usual_dry}; with neighbours: below 3 drinks {short}, dry {dry}, min {min_kg:.3} kg");
    assert_eq!(dry, 0, "even the neighbouring cells were dry");
}
