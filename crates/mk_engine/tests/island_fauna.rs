//! The first grazer layer on the real island: seeded at equilibrium, every
//! kilogram eaten accounted for, the plants close to a no-grazer run, and a
//! stable herbivore total. Parameters are assumptions (deviation D33).

use std::path::PathBuf;
use std::sync::Arc;

use mk_core::canon::CanonLocked;
use mk_engine::regional::ecology::GrazerFlows;
use mk_engine::regional::fauna::{equilibrium_kgc_m2, MAX_INTAKE_FRACTION_OF_NPP};
use mk_engine::regional::life::IslandLife;
use mk_island::{DomainLevel, IslandScenario};

fn repo(p: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(p)
}

#[test]
fn grazers_seed_at_equilibrium_balance_their_carbon_and_leave_the_plants_alone() {
    let scenario = IslandScenario::load(&repo("fixtures/island/default_scenario.json")).unwrap();
    let canon = Arc::new(CanonLocked::load(&repo("fixtures/island/canon.json")).unwrap());
    let life = IslandLife::bootstrap(scenario, canon).unwrap();
    let area = life.domain.cell_area_m2(DomainLevel::Medium);
    let start = life.ecology.clone();

    // Seeded at equilibrium on land with producers, zero elsewhere.
    let mut land_cells = 0;
    for ((h, npp), b) in start
        .herbivore_kgc_m2
        .data()
        .iter()
        .zip(start.npp_kgc_m2_yr.data())
        .zip(start.biomass_kgc_m2.data())
    {
        if *npp > 0.0 && *b > 0.0 {
            land_cells += 1;
            assert!((h - equilibrium_kgc_m2(*npp)).abs() < 1e-15);
        } else {
            assert_eq!(*h, 0.0);
        }
    }
    assert!(
        land_cells > 10_000,
        "the island has producers: {land_cells} cells"
    );
    let herb_start = start.total_herbivore_kgc(area);
    assert!(herb_start > 0.0);

    // One simulated year in six-hour steps, with and without grazers.
    let mut with = start.clone();
    let mut without = start.clone();
    let mut sum = GrazerFlows::default();
    for _ in 0..(4 * 365) {
        let f = with.step_with_grazers(21_600.0, area);
        sum.eaten += f.eaten;
        sum.growth += f.growth;
        sum.faeces += f.faeces;
        sum.respired += f.respired;
        sum.deaths += f.deaths;
        without.step(21_600.0, None, None);
    }

    // Every kilogram eaten went somewhere, and the pool balances.
    let rel = |a: f64, b: f64| (a - b).abs() / a.abs().max(1.0);
    assert!(sum.eaten > 0.0);
    assert!(
        rel(sum.eaten, sum.growth + sum.faeces + sum.respired) < 1e-9,
        "{sum:?}"
    );
    let herb_end = with.total_herbivore_kgc(area);
    assert!(
        rel(herb_end - herb_start, sum.growth - sum.deaths) < 1e-6,
        "{herb_start} -> {herb_end} {sum:?}"
    );

    // At equilibrium the total barely moves.
    assert!(
        rel(herb_start, herb_end) < 0.02,
        "{herb_start} -> {herb_end}"
    );

    // Grazing takes at most its cap of production: plants stay within it of
    // the no-grazer run (and never above it).
    let plants_with = with.total_biomass_kgc(&life.domain);
    let plants_without = without.total_biomass_kgc(&life.domain);
    assert!(
        plants_with <= plants_without * (1.0 + 1e-12),
        "{plants_with} vs {plants_without}"
    );
    assert!(
        plants_with >= plants_without * (1.0 - 1.5 * MAX_INTAKE_FRACTION_OF_NPP),
        "grazers took too much: {plants_with} vs {plants_without}"
    );
    println!(
        "FAUNA herbivores {:.0} t C at start, {:.0} t C after a year; ate {:.0} t C ({:.1}% of one year of plant production); plants {:.4}% below the no-grazer run",
        herb_start / 1000.0,
        herb_end / 1000.0,
        sum.eaten / 1000.0,
        100.0 * sum.eaten / (start.npp_kgc_m2_yr.data().iter().sum::<f64>() * area),
        100.0 * (1.0 - plants_with / plants_without),
    );
}
