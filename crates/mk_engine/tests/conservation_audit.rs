//! The per-tick conservation audit: every audited interior stock (surface
//! and ocean heat, soil water, atmospheric and biomass carbon) must change
//! by exactly what the ledger recorded flowing in and out of it.

use mk_core::canon::CanonLocked;
use mk_engine::interventions::InterventionExecutor;
use mk_engine::world_integration::WorldState;
use mk_interventions::{
    BiomassType, ClimateParameter, DisturbanceType, EnergyType, InterventionAction,
    InterventionPermissions, Location, Region, ResourceType, ScenarioValue,
};
use std::sync::Arc;

fn assert_conserved(world: &WorldState, context: &str) {
    let report = &world.audit_trail.stock_audit;
    assert_eq!(
        report.tick + 1,
        world.tick,
        "{context}: audit must cover the last tick"
    );
    assert!(
        report.conserved,
        "{context}: {:?} (energy {:e}, water {:e}, carbon {:e}, oxygen {:e}, nitrogen {:e}, \
         phosphorus {:e})",
        report.failure,
        report.energy_relative_residual,
        report.water_relative_residual,
        report.carbon_relative_residual,
        report.oxygen_relative_residual,
        report.nitrogen_relative_residual,
        report.phosphorus_relative_residual
    );
}

#[test]
fn a_running_world_conserves_heat_water_and_carbon_every_tick() {
    let mut world = WorldState::new(Arc::new(CanonLocked::default()), [5u8; 32]);
    for (i, dt) in [1u64, 60, 3600, 86_400, 86_400, 7 * 86_400, 30 * 86_400]
        .into_iter()
        .enumerate()
    {
        world.step_world(dt).unwrap();
        assert_conserved(&world, &format!("tick {i} (dt {dt} s)"));
    }
}

#[test]
fn interventions_between_ticks_are_conserved() {
    let mut world = WorldState::new(Arc::new(CanonLocked::default()), [6u8; 32]);
    world.step_world(3600).unwrap();
    let executor = InterventionExecutor::new(InterventionPermissions::default());
    let here = Location::new(10.0, 20.0);
    for action in [
        InterventionAction::InjectEnergy {
            energy_type: EnergyType::Thermal,
            amount: 5.0e20,
            location: here.clone(),
        },
        InterventionAction::InjectResource {
            resource_type: ResourceType::Water,
            amount: 900.0,
            location: here.clone(),
        },
        InterventionAction::ModifyClimate {
            parameter: ClimateParameter::Temperature,
            value: 305.0,
            region: Some(Region::new(here.clone(), 2000.0)),
        },
        InterventionAction::TriggerDisturbance {
            disturbance_type: DisturbanceType::Flood,
            intensity: 0.8,
            location: here.clone(),
        },
        InterventionAction::TriggerDisturbance {
            disturbance_type: DisturbanceType::VolcanicEruption,
            intensity: 0.9,
            location: here.clone(),
        },
        InterventionAction::TriggerDisturbance {
            disturbance_type: DisturbanceType::Drought,
            intensity: 0.7,
            location: here.clone(),
        },
        InterventionAction::TriggerDisturbance {
            disturbance_type: DisturbanceType::Fire,
            intensity: 0.9,
            location: here.clone(),
        },
        InterventionAction::TriggerDisturbance {
            disturbance_type: DisturbanceType::Disease,
            intensity: 0.9,
            location: here.clone(),
        },
        InterventionAction::InjectBiomass {
            biomass_type: BiomassType::Producers,
            amount: 5.0e6,
            region: Region::new(here.clone(), 20_000.0),
        },
        InterventionAction::ModifyScenario {
            parameter: "climate.co2_concentration_ppm".to_string(),
            value: ScenarioValue::Float(720.0),
        },
    ] {
        let label = format!("{action:?}");
        executor.execute(&mut world, &action).expect(&label);
        world.step_world(3600).unwrap();
        assert_conserved(&world, &label);
        world.step_world(86_400).unwrap();
        assert_conserved(&world, &format!("{label}, one day later"));
    }
}

#[test]
fn an_unbooked_stock_change_is_caught() {
    let mut world = WorldState::new(Arc::new(CanonLocked::default()), [8u8; 32]);
    world.step_world(3600).unwrap();
    // Change a stock behind the ledger's back.
    if let Some(soil) = world.hydrology_state.soil_water.get_mut_safe(3, 3) {
        soil.storage_mm += 50.0;
    }
    world.step_world(3600).unwrap();
    let report = &world.audit_trail.stock_audit;
    assert!(!report.conserved, "an unbooked change must fail the audit");
    assert!(report
        .failure
        .as_deref()
        .unwrap_or("")
        .contains("SoilWater"));
}

#[test]
fn biomass_growth_draws_its_carbon_from_the_atmosphere() {
    let mut world = WorldState::new(Arc::new(CanonLocked::default()), [9u8; 32]);
    world.step_world(3600).unwrap();
    let before = mk_engine::conservation::measure(&world);
    let tick = world.tick;
    world.step_world(30 * 86_400).unwrap();
    assert_conserved(&world, "one month");
    let after = mk_engine::conservation::measure(&world);
    let get = |stocks: &mk_core::flux::ReservoirStocks, reservoir| {
        stocks
            .get(reservoir, mk_core::flux::FluxKind::Carbon)
            .unwrap()
    };
    use mk_core::flux::Reservoir::{BiomassCarbon, DetritusCarbon};
    let biomass_change = get(&after, BiomassCarbon) - get(&before, BiomassCarbon);
    assert!(biomass_change != 0.0, "the biosphere changed no biomass");
    let detritus_change = get(&after, DetritusCarbon) - get(&before, DetritusCarbon);
    // The air also trades CO2 with the crust (the carbon cycle's volcanic
    // outgassing and weathering), which `assert_conserved` checks against
    // the ledger every tick. Free O2 is exact regardless: photosynthesis
    // frees, and decomposition binds, the O2 of exactly the carbon living
    // and dead biomass took from or returned to the air, while crust
    // exchange moves CO2 with its own oxygen.
    use mk_core::flux::FluxKind::Oxygen;
    let o2 =
        |stocks: &mk_core::flux::ReservoirStocks, reservoir| stocks.get(reservoir, Oxygen).unwrap();
    let free_o2_change = o2(&after, mk_core::flux::Reservoir::AtmosO2)
        - o2(&before, mk_core::flux::Reservoir::AtmosO2);
    assert!(free_o2_change != 0.0, "the biosphere moved no oxygen");
    let expected = (biomass_change + detritus_change) * mk_engine::conservation::O2_PER_CARBON;
    assert!(
        ((free_o2_change - expected) / o2(&before, mk_core::flux::Reservoir::AtmosO2)).abs()
            < 1e-12,
        "free O2 {free_o2_change:e} vs {expected:e}"
    );
}

#[test]
fn nitrogen_and_phosphorus_enter_from_air_and_rock_and_stay_audited() {
    use mk_core::flux::FluxKind::{Nitrogen, Phosphorus};
    use mk_core::flux::Reservoir::*;
    let mut world = WorldState::new(Arc::new(CanonLocked::default()), [11u8; 32]);
    world.step_world(3600).unwrap();
    let before = mk_engine::conservation::measure(&world);
    for _ in 0..3 {
        world.step_world(30 * 86_400).unwrap();
        // Fixation, denitrification, weathering and burial are booked
        // against the AtmosN2 and CrustPhosphorus boundaries, so every
        // interior pool still closes each step.
        assert_conserved(&world, "monthly");
    }
    let after = mk_engine::conservation::measure(&world);
    // The canon starting pools sit below Marr'Ken's equilibrium (inputs
    // exceed first-order losses), so over three months the biosphere's
    // total N and P rise slightly: ≈1e-4 and ≈3e-4 of the pool per month.
    for (kind, pools, max_rise) in [
        (
            Nitrogen,
            [BiomassNitrogen, DetritusNitrogen, SoilNitrogen],
            3e-3,
        ),
        (
            Phosphorus,
            [BiomassPhosphorus, DetritusPhosphorus, SoilPhosphorus],
            1e-2,
        ),
    ] {
        let total = |stocks: &mk_core::flux::ReservoirStocks| -> f64 {
            pools.iter().map(|r| stocks.get(*r, kind).unwrap()).sum()
        };
        let biomass_moved =
            (after.get(pools[0], kind).unwrap() - before.get(pools[0], kind).unwrap()).abs();
        assert!(
            biomass_moved > 0.0,
            "{kind}: biomass took up or released nothing"
        );
        let rise = (total(&after) - total(&before)) / total(&before);
        assert!(
            rise > 0.0 && rise < max_rise,
            "{kind}: {:e} → {:e} ({rise:e})",
            total(&before),
            total(&after)
        );
    }
}

#[test]
fn unbooked_biomass_is_caught() {
    let mut world = WorldState::new(Arc::new(CanonLocked::default()), [10u8; 32]);
    world.step_world(3600).unwrap();
    world.biosphere_state.species[0].population_size += 1_000_000;
    world.step_world(3600).unwrap();
    let report = &world.audit_trail.stock_audit;
    assert!(!report.conserved, "an unbooked biomass change must fail");
    assert!(report
        .failure
        .as_deref()
        .unwrap_or("")
        .contains("BiomassCarbon"));
}
