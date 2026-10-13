//! Water at home (realism programme R1): the house has a rainwater tank
//! sized from the estate's own climate, the founders drink from it at home,
//! the household's washing and flushing draw on it, and every kilogram is
//! in the books.

use std::path::PathBuf;
use std::sync::Arc;

use mk_core::canon::CanonLocked;
use mk_engine::regional::life::IslandLife;
use mk_island::IslandScenario;

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
fn the_house_draws_its_water_from_a_tank_on_its_own_roof() {
    let mut life = island();
    let tank = life.household_water.clone().expect("the house has a tank");
    assert!(tank.roof_area_m2 > 20.0, "roof {} m²", tank.roof_area_m2);
    assert!(
        tank.capacity_kg >= tank.spec.tank_unit_l
            && (tank.capacity_kg / tank.spec.tank_unit_l).fract() == 0.0,
        "{} kg is not whole tanks of {} L",
        tank.capacity_kg,
        tank.spec.tank_unit_l
    );
    assert!(
        tank.stored_kg > 0.99 * tank.capacity_kg,
        "a lived-in house starts with its tank full"
    );
    println!(
        "roof {:.0} m², {:.0} L installed, {:.0} L stored",
        tank.roof_area_m2, tank.capacity_kg, tank.stored_kg
    );

    let drunk_before: f64 = life.water_drunk_kg.values().sum();
    for _ in 0..(86_400 / 60) {
        // `advance` audits every household step, the tank's included: a
        // kilogram that moved without a ledger entry fails here.
        life.advance(60).unwrap();
    }
    let after = life.household_water.clone().unwrap();
    let drunk: f64 = life.water_drunk_kg.values().sum::<f64>() - drunk_before;
    assert!(drunk > 0.0, "nobody drank in a day");
    // The household used water for more than drinking: the founders were
    // home for at least part of the day, at ESR's 278 L or the 20 L ration.
    let from_tank = after.used_kg - tank.used_kg;
    assert!(
        from_tank > drunk,
        "the tank gave {from_tank} L, no more than the {drunk} L drunk: nobody washed, \
         flushed or cooked"
    );
    assert!(
        after.used_kg - tank.used_kg <= 2.0 * 280.0 + drunk + 1e-6,
        "two people used {} L in a day",
        after.used_kg - tank.used_kg
    );
    // What the tank holds is never more than it can, and what went in or
    // out is accounted for.
    assert!(after.stored_kg <= after.capacity_kg + 1e-6);
    let balance =
        tank.stored_kg + (after.caught_kg - tank.caught_kg) + (after.fetched_kg - tank.fetched_kg)
            - (after.used_kg - tank.used_kg);
    assert!(
        (balance - after.stored_kg).abs() < 1e-6,
        "tank: started {} + caught {} - used {} = {} but holds {}",
        tank.stored_kg,
        after.caught_kg - tank.caught_kg,
        after.used_kg - tank.used_kg,
        balance,
        after.stored_kg
    );
    println!(
        "a day: drank {drunk:.2} L, used {:.1} L in all, caught {:.1} L, overflowed {:.1} L, \
         rationed {:.0} s",
        after.used_kg - tank.used_kg,
        after.caught_kg - tank.caught_kg,
        after.overflowed_kg - tank.overflowed_kg,
        after.rationed_s
    );

    // The tank is part of the island: saved and restored with it, and in
    // its digest.
    let snapshot = life.snapshot();
    assert_eq!(snapshot.household_water.as_ref(), Some(&after));
    let digest = life.state_digest();
    life.household_water.as_mut().unwrap().stored_kg += 1.0;
    assert_ne!(
        life.state_digest(),
        digest,
        "the digest does not see the tank"
    );
}

/// With the tank dry and no rain reaching it, a founder who wants a drink
/// at home has to walk to water (realism R1b): the trip takes minutes, not
/// a tick, and comes back with the jerrycans full for the tank.
#[test]
fn with_the_tank_dry_a_founder_walks_to_water_and_carries_it_home() {
    let mut life = island();
    {
        let tank = life.household_water.as_mut().unwrap();
        tank.stored_kg = 0.0;
        tank.roof_area_m2 = 0.0;
    }
    let mut longest_trip_s: f64 = 0.0;
    for _ in 0..(2 * 86_400 / 60) {
        life.advance(60).unwrap();
        for trip in life.water_trips.values() {
            longest_trip_s = longest_trip_s.max(trip.worked_s);
            assert!(trip.carry_kg <= 40.0 + 1e-9, "two 20 L cans hold 40 L");
        }
        if life.household_water.as_ref().unwrap().fetched_kg > 0.0 {
            break;
        }
    }
    let tank = life.household_water.as_ref().unwrap();
    println!(
        "longest trip seen {:.0} s; fetched {:.1} L; tank {:.1} L",
        longest_trip_s, tank.fetched_kg, tank.stored_kg
    );
    assert!(
        longest_trip_s >= 120.0,
        "a trip to water took no time: {longest_trip_s} s"
    );
    assert!(tank.fetched_kg > 0.0, "nobody carried water home");
    assert!(
        tank.fetched_kg <= 40.0 * 10.0,
        "{} L fetched",
        tank.fetched_kg
    );
}
