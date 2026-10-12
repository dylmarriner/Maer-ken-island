//! Phase 4b Task 3: the estate's hydro plant, its line and the river it
//! draws on.

use std::path::PathBuf;

use mk_core::canon::CanonLocked;
use mk_core::flux::Ledger;
use mk_core::grid::Grid2;
use mk_engine::climate::ClimateState;
use mk_engine::hydrology::HydrologyState;
use mk_engine::organisms::property::{PropertyItemKind, StarterProperty};
use mk_engine::regional::energy::{EstateEnergy, HydroSupply};
use mk_engine::regional::hydrology::{
    step_regional_hydrology_diverted, step_regional_hydrology_on, Diversion, FlowNetwork, Receiver,
};
use mk_engine::regional::industry::grid::Line;
use mk_engine::regional::industry::hydro::{design_hydro, WATER_DENSITY_KG_M3};
use mk_engine::regional::industry::hydro_sites::HydroSite;
use mk_engine::weather::WeatherState;
use mk_island::{DomainLevel, EstateEnergyConfig, IslandDomain, IslandProfile};

const M: DomainLevel = DomainLevel::Medium;
const DAY: f64 = 86_400.0;

/// A 25 kW plant on an 80 m fall, 600 m of penstock, on a river whose
/// year runs 0.05-0.43 m^3/s with 0.03 m^3/s always left in it.
fn site() -> HydroSite {
    let flows: Vec<f64> = (0..20).map(|k| 0.05 + 0.02 * f64::from(k)).collect();
    let environmental = 0.03;
    let design = design_hydro(25.0, 80.0, 600.0, &flows, environmental).unwrap();
    HydroSite {
        intake_cell: (0, 0),
        powerhouse_cell: (0, 1),
        head_m: 80.0,
        penstock_length_m: 600.0,
        design_flow_m3_s: design.turbine.design_flow_m3_s,
        firm_flow_m3_s: 0.06,
        mean_flow_m3_s: 0.24,
        environmental_flow_m3_s: environmental,
        design,
    }
}

#[test]
fn output_follows_the_river_and_stops_below_the_environmental_flow() {
    let s = site();
    let supply = |q: f64| HydroSupply {
        site: s.clone(),
        line: Line::design(0, 1, s.design.rated_kw, 1.0),
        river_m3_s: q,
    };
    // Plenty of water: rated output, less the line.
    let full = supply(1.0);
    assert!((full.generated_kw() - s.design.rated_kw).abs() < 1e-9);
    assert!(full.delivered_kw() < full.generated_kw());
    // A drought: less water, less power, never more than the river minus
    // what it must keep.
    let mut last = f64::INFINITY;
    for q in [0.5, 0.2, 0.1, 0.06, 0.045, 0.031] {
        let p = supply(q);
        assert!(p.diverted_m3_s() <= q - s.environmental_flow_m3_s + 1e-12);
        assert!(p.generated_kw() <= last + 1e-9);
        last = p.generated_kw();
    }
    for q in [0.03, 0.01, 0.0] {
        assert_eq!(supply(q).diverted_m3_s(), 0.0);
        assert_eq!(supply(q).generated_kw(), 0.0);
    }
    // P = rho g Q H_net e_t e_g at a flow inside the curve.
    let p = supply(0.1);
    let q = p.diverted_m3_s();
    let d = &s.design;
    let expected = WATER_DENSITY_KG_M3
        * 9.80665
        * q
        * (d.gross_head_m - d.penstock.head_loss_m(q))
        * d.turbine.efficiency(q)
        * d.generator_efficiency
        / 1_000.0;
    assert!((p.generated_kw() - expected).abs() < 1e-9);
}

#[test]
fn the_estate_runs_on_the_river_first_and_falls_back_to_diesel() {
    let property = StarterProperty::founders_estate((0, 0));
    let mut energy = EstateEnergy::from_config(&EstateEnergyConfig::default(), &property);
    energy.attach_hydro(site(), 1.0);
    let computer = property
        .items
        .iter()
        .find(|i| i.kind == PropertyItemKind::Computer)
        .unwrap()
        .id;
    energy.set_in_use(computer, true);
    let mut ledger = Ledger::new();

    // A running river: the computer runs on hydro, the full battery takes
    // nothing, the rest goes to the ballast load; no fuel burns.
    energy.hydro[0].river_m3_s = 1.0;
    let wet = energy.step(3_600.0, 0.0, &mut ledger);
    assert!(energy.has_power(computer));
    assert!(wet.hydro_used_kwh > 0.0 && wet.hydro_dumped_kwh > 0.0);
    assert!(wet.line_loss_kwh > 0.0);
    assert_eq!(wet.generator_kwh, 0.0);
    assert_eq!(wet.fuel_burned_litres, 0.0);
    // Energy balance: what arrived = what was used + dumped.
    let generated = energy.hydro[0].generated_kw();
    assert!(
        (wet.hydro_used_kwh + wet.hydro_dumped_kwh + wet.line_loss_kwh - generated).abs() < 1e-9
    );

    // The river runs dry: battery, then the diesel generator.
    energy.hydro[0].river_m3_s = 0.0;
    let mut burned = 0.0;
    for _ in 0..200 {
        let dry = energy.step(3_600.0, 0.0, &mut ledger);
        assert_eq!(dry.hydro_used_kwh, 0.0);
        burned += dry.fuel_burned_litres;
    }
    assert!(
        burned > 0.0,
        "no diesel burned once the river and battery ran out"
    );
    assert!(energy.has_power(computer));
}

fn medium(d: &IslandDomain, f: impl Fn(usize, usize) -> f64) -> Grid2<f64> {
    let (rows, cols) = (d.rows(M), d.cols(M));
    Grid2::from_data(
        &d.storage_spec(M),
        (0..rows)
            .flat_map(|r| (0..cols).map(move |c| (r, c)))
            .map(|(r, c)| f(r, c))
            .collect(),
    )
}

#[test]
fn diverted_water_skips_the_reach_and_returns_at_the_tailrace() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/island/canon.json");
    let canon = CanonLocked::load(&path).unwrap();
    let d = IslandDomain::from_profile(IslandProfile::test_small()).unwrap();
    // A domed island in open sea.
    let t = medium(&d, |r, c| {
        let dist = (r as f64 - 96.0).hypot(c as f64 - 120.0);
        if dist < 25.0 {
            400.0 * (1.0 - dist / 25.0) + 2.0
        } else {
            -50.0
        }
    });
    let net = FlowNetwork::build(&t);
    let spec = d.storage_spec(DomainLevel::Coarse);
    let mut weather = WeatherState::new(&spec);
    weather.precipitation = Grid2::new(&spec, 15.0);
    let climate = ClimateState::new(&spec, 285.0);
    let mut s = HydrologyState::new(&d.storage_spec(M), 0.5);
    for _ in 0..20 {
        s = step_regional_hydrology_on(&canon, &s, &weather, &climate, &t, &d, &net, DAY);
    }

    // A river from a flank cell four cells down toward the sea.
    let cols = d.cols(M);
    let mut cells = vec![(96usize, 135usize)];
    while cells.len() < 5 {
        let (r, c) = *cells.last().unwrap();
        match net.receiver_of(r, c) {
            Receiver::Cell(j) => cells.push((j as usize / cols, j as usize % cols)),
            other => panic!("the river reached {other:?} too soon"),
        }
    }
    let (intake, tailrace) = (cells[0], cells[4]);
    let flat = |(r, c): (usize, usize)| (r * cols + c) as u32;
    let q = 0.25 * mk_engine::regional::hydrology::discharge_m3_s(&s, &d, intake.0, intake.1);
    let plain = step_regional_hydrology_on(&canon, &s, &weather, &climate, &t, &d, &net, DAY);
    let diverted = step_regional_hydrology_diverted(
        &canon,
        &s,
        &weather,
        &climate,
        &t,
        &d,
        &net,
        DAY,
        &[Diversion {
            intake: flat(intake),
            tailrace: flat(tailrace),
            flow_m3_s: q,
        }],
    );
    let flow = |h: &HydrologyState, (r, c): (usize, usize)| {
        mk_engine::regional::hydrology::discharge_m3_s(h, &d, r, c)
    };
    // The intake records the river arriving at the weir.
    assert_eq!(flow(&plain, intake), flow(&diverted, intake));
    // The bypassed reach carries the diverted flow less.
    for &cell in &cells[1..4] {
        let less = flow(&plain, cell) - flow(&diverted, cell);
        assert!(
            (less - q).abs() < 1e-9 * q.max(1.0),
            "{cell:?}: {less} vs {q}"
        );
    }
    // At and below the tailrace the river is whole again, and the books
    // close the same.
    assert!((flow(&plain, tailrace) - flow(&diverted, tailrace)).abs() < 1e-9);
    assert!(
        (plain.budget.surface_to_ocean_kg - diverted.budget.surface_to_ocean_kg).abs()
            <= 1e-9 * plain.budget.surface_to_ocean_kg
    );
}

#[test]
fn the_plant_and_line_use_the_reference_pack_values() {
    use mk_engine::regional::industry::{grid, hydro};
    use mk_engine::validation::{default_reference_dir, ReferenceDomain, ReferenceLibrary};
    let lib = ReferenceLibrary::load(&default_reference_dir()).unwrap();
    let item = |key: &str| lib.item(ReferenceDomain::Industry, key).unwrap();
    assert_eq!(
        item("environmental_flow_fraction_of_mean").central(),
        Some(hydro::ENVIRONMENTAL_FLOW_FRACTION)
    );
    assert_eq!(
        item("firm_flow_exceedance").point(),
        Some(hydro::FIRM_FLOW_EXCEEDANCE)
    );
    assert_eq!(
        item("design_hydraulic_loss_fraction").point(),
        Some(hydro::DESIGN_HYDRAULIC_LOSS_FRACTION)
    );
    assert_eq!(
        item("water_kinematic_viscosity_10c").point(),
        Some(hydro::WATER_KINEMATIC_VISCOSITY_M2_S)
    );
    let rough = item("pipe_absolute_roughness").table().unwrap();
    assert_eq!(
        rough.get("plastic_drawn_tubing", "roughness_m"),
        Some(hydro::PLASTIC_PIPE_ROUGHNESS_M)
    );
    assert_eq!(
        rough.get("commercial_steel", "roughness_m"),
        Some(hydro::STEEL_PIPE_ROUGHNESS_M)
    );
    assert_eq!(
        item("max_voltage_drop_fraction").point(),
        Some(grid::MAX_VOLTAGE_DROP_FRACTION)
    );
    let volts = item("standard_three_phase_voltages").table().unwrap();
    let volts: Vec<f64> = volts
        .column("nominal_v")
        .unwrap()
        .iter()
        .map(|(_, v)| *v)
        .collect();
    assert_eq!(volts, grid::STANDARD_LINE_VOLTAGES_V.to_vec());
    let acsr = item("acsr_conductor_resistance").table().unwrap();
    let acsr = acsr.column("ohm_per_km").unwrap();
    assert_eq!(acsr.len(), grid::ACSR_CONDUCTORS.len());
    for ((name, ohm), (code, r)) in acsr.iter().zip(grid::ACSR_CONDUCTORS) {
        assert_eq!((*name, *ohm), (code, r));
    }
}
