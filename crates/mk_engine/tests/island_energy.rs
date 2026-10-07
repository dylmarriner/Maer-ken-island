//! Phase 3 Task 4b: fuel and electricity for the estate.

use std::path::PathBuf;

use mk_core::flux::{FluxKind, Ledger, Reservoir};
use mk_engine::conservation::O2_PER_CARBON;
use mk_engine::organisms::property::{PropertyItemKind, StarterProperty};
use mk_engine::regional::energy::{
    book_burn, reference_l_per_100km, solar_output_kw, Burn, EstateEnergy, FuelKind, Surface,
    DIESEL_KWH_PER_LITRE, GENERATOR_EFFICIENCY, WORK_L_PER_KWH,
};
use mk_engine::validation::ReferencePack;
use mk_island::EstateEnergyConfig;

fn estate() -> StarterProperty {
    StarterProperty::founders_estate((0, 0))
}

fn energy(config: EstateEnergyConfig) -> (EstateEnergy, u64) {
    let property = estate();
    let e = EstateEnergy::from_config(&config, &property);
    let computer = e
        .loads
        .iter()
        .find(|l| l.name.contains("Computer"))
        .unwrap()
        .item_id;
    (e, computer)
}

fn vehicle(e: &EstateEnergy, name: &str) -> u64 {
    e.vehicles.iter().find(|v| v.name == name).unwrap().item_id
}

#[test]
fn a_computer_runs_on_solar_by_day_then_the_battery_then_the_generator() {
    let (mut e, computer) = energy(EstateEnergyConfig::default());
    e.set_in_use(computer, true);
    let mut ledger = Ledger::new();

    // Midday: ~3 kW of solar covers a 0.2 kW computer; no battery, no fuel.
    let noon = solar_output_kw(5.0, 1_000.0, 0.0);
    assert!(noon > 2.0 && noon <= 5.0, "noon output {noon} kW");
    let day = e.step(3_600.0, noon, &mut ledger);
    assert!(e.has_power(computer));
    assert_eq!(
        (day.battery_discharged_kwh, day.generator_kwh, day.unmet_kwh),
        (0.0, 0.0, 0.0)
    );
    assert!((day.solar_used_kwh - 0.2).abs() < 1e-12);
    assert!(
        day.solar_curtailed_kwh > 0.0,
        "a full battery curtails the surplus"
    );
    // Cloud cuts the output; the night has none.
    assert!(solar_output_kw(5.0, 1_000.0, 1.0) < 0.3 * noon);
    assert_eq!(solar_output_kw(5.0, 0.0, 0.0), 0.0);

    // Night: the battery runs it (10 kWh x 0.95 / 0.2 kW ~ 47.5 h), then
    // the generator takes over without a break in service.
    let diesel_before = e.stored_litres(FuelKind::Diesel);
    let mut first_generator_hour = None;
    for hour in 0..60 {
        let step = e.step(3_600.0, 0.0, &mut ledger);
        assert!(e.has_power(computer), "power failed at hour {hour}");
        if step.generator_kwh > 0.0 && first_generator_hour.is_none() {
            first_generator_hour = Some(hour);
        }
        if first_generator_hour.is_none() {
            assert_eq!(step.fuel_burned_litres, 0.0);
        }
    }
    let at = first_generator_hour.expect("the battery never ran out");
    assert!((46..=49).contains(&at), "battery lasted {at} h");
    assert!(e.stored_litres(FuelKind::Diesel) < diesel_before);
}

#[test]
fn the_generator_burns_fuel_at_its_efficiency_and_stops_when_the_fuel_does() {
    let config = EstateEnergyConfig {
        battery_kwh: 0.0,
        solar_rated_kw: 0.0,
        ..EstateEnergyConfig::default()
    };
    let (mut e, computer) = energy(config);
    e.set_in_use(computer, true);
    let mut ledger = Ledger::new();
    let before = e.stored_litres(FuelKind::Diesel);
    let step = e.step(3_600.0, 0.0, &mut ledger);
    let per_kwh = 1.0 / (DIESEL_KWH_PER_LITRE * GENERATOR_EFFICIENCY);
    assert!((step.generator_kwh - 0.2).abs() < 1e-12);
    assert!((step.fuel_burned_litres - 0.2 * per_kwh).abs() < 1e-12);
    assert!((before - e.stored_litres(FuelKind::Diesel) - 0.2 * per_kwh).abs() < 1e-9);

    // Empty the diesel: nothing runs, and the computer has no power.
    for s in &mut e.fuel_stores {
        if s.fuel == FuelKind::Diesel {
            s.litres = 0.0;
        }
    }
    let dark = e.step(3_600.0, 0.0, &mut ledger);
    assert!(dark.unmet_kwh > 0.0 && dark.generator_kwh == 0.0);
    assert!(
        !e.has_power(computer),
        "a computer with no power must be denied"
    );
    // A load that is not in use draws nothing and is not "powered".
    e.set_in_use(computer, false);
    assert_eq!(e.step(3_600.0, 0.0, &mut ledger).demand_kwh, 0.0);
    assert!(!e.has_power(computer));
}

#[test]
fn vehicles_burn_the_reference_fuel_and_an_empty_tank_does_not_move() {
    let pack = ReferencePack::load(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/reference/labour/productivity.json"),
    )
    .unwrap();
    let table = pack
        .item("light_4wd_fuel_consumption")
        .unwrap()
        .table()
        .unwrap();
    for (row, surface) in [
        ("sealed_road", Surface::SealedRoad),
        ("gravel_road", Surface::GravelRoad),
        ("off_road_track", Surface::OffRoadTrack),
    ] {
        let (lo, hi) = (
            table.get(row, "min").unwrap(),
            table.get(row, "max").unwrap(),
        );
        let mid = reference_l_per_100km(surface);
        assert!((lo..=hi).contains(&mid), "{row}: {mid} outside {lo}-{hi}");
    }

    let (mut e, _) = energy(EstateEnergyConfig::default());
    let mut ledger = Ledger::new();
    let raptor = vehicle(&e, "Ford Raptor 4x4 Ute");
    e.refuel(raptor, 200.0);
    let tank = e
        .vehicles
        .iter()
        .find(|v| v.item_id == raptor)
        .unwrap()
        .litres;
    let driven = e.drive_km(raptor, Surface::SealedRoad, 100.0, &mut ledger);
    assert_eq!(driven, 100.0);
    let used = tank
        - e.vehicles
            .iter()
            .find(|v| v.item_id == raptor)
            .unwrap()
            .litres;
    let (lo, hi) = (
        table.get("sealed_road", "min").unwrap(),
        table.get("sealed_road", "max").unwrap(),
    );
    assert!(
        (lo..=hi).contains(&used),
        "100 km used {used} L; reference {lo}-{hi}"
    );

    // A tractor works at the reference specific consumption.
    let fendt = vehicle(&e, "Fendt 900 Vario");
    e.refuel(fendt, 600.0);
    let before = e
        .vehicles
        .iter()
        .find(|v| v.item_id == fendt)
        .unwrap()
        .litres;
    assert_eq!(e.run_engine_hours(fendt, 1.0, &mut ledger), 1.0);
    let burned = before
        - e.vehicles
            .iter()
            .find(|v| v.item_id == fendt)
            .unwrap()
            .litres;
    assert!((burned - 220.0 * 0.5 * WORK_L_PER_KWH).abs() < 1e-9);

    // An empty tank covers no distance and burns nothing.
    let rzr = vehicle(&e, "Polaris RZR 1000 Turbo");
    for v in &mut e.vehicles {
        if v.item_id == rzr {
            v.litres = 0.0;
        }
    }
    let entries = ledger.entries().len();
    assert_eq!(
        e.drive_km(rzr, Surface::OffRoadTrack, 10.0, &mut ledger),
        0.0
    );
    assert_eq!(ledger.entries().len(), entries);
    // A trailer has no engine.
    let trailer = estate()
        .items
        .iter()
        .find(|i| i.name == "Double Axle Trailer")
        .unwrap()
        .id;
    assert_eq!(
        e.drive_km(trailer, Surface::SealedRoad, 5.0, &mut ledger),
        0.0
    );
    assert!(estate()
        .items
        .iter()
        .any(|i| i.kind == PropertyItemKind::Vehicle));
}

#[test]
fn burning_fuel_books_the_stoichiometric_carbon_and_oxygen() {
    for fuel in [FuelKind::Diesel, FuelKind::Petrol] {
        // Mass balance per litre: fuel + O2 = CO2 + H2O.
        let density = match fuel {
            FuelKind::Diesel => 0.832,
            FuelKind::Petrol => 0.745,
        };
        let lhs = density + fuel.o2_kg_per_litre();
        let rhs = fuel.co2_kg_per_litre() + fuel.water_kg_per_litre();
        assert!((lhs - rhs).abs() < 1e-3 * lhs, "{fuel:?}: {lhs} vs {rhs}");
        // About 2.6 kg CO2 per litre of diesel, 2.3 of petrol (EPA 2.68, 2.35).
        let co2 = fuel.co2_kg_per_litre();
        let want = if fuel == FuelKind::Diesel { 2.68 } else { 2.35 };
        assert!((co2 - want).abs() < 0.12 * want, "{fuel:?}: {co2} kg CO2/L");

        let mut ledger = Ledger::new();
        book_burn(
            &mut ledger,
            Burn {
                fuel,
                litres: 100.0,
            },
        );
        let carbon = ledger
            .entries()
            .iter()
            .find(|e| e.kind == FluxKind::Carbon)
            .unwrap();
        assert_eq!(
            (carbon.source, carbon.sink),
            (Reservoir::CrustCarbon, Reservoir::AtmosCO2)
        );
        assert!((carbon.amount - 100.0 * fuel.carbon_kg_per_litre()).abs() < 1e-9);
        // CO2 mass from the carbon, and the oxygen it binds.
        assert!((carbon.amount * 44.009 / 12.011 - 100.0 * co2).abs() < 1e-6);
        let oxygen = ledger
            .entries()
            .iter()
            .find(|e| e.kind == FluxKind::Oxygen)
            .unwrap();
        assert_eq!(
            (oxygen.source, oxygen.sink),
            (Reservoir::AtmosO2, Reservoir::AtmosCO2)
        );
        assert!((oxygen.amount - carbon.amount * O2_PER_CARBON).abs() < 1e-9);
    }
    // Nothing burned books nothing.
    let mut ledger = Ledger::new();
    book_burn(
        &mut ledger,
        Burn {
            fuel: FuelKind::Diesel,
            litres: 0.0,
        },
    );
    assert!(ledger.entries().is_empty());
}
