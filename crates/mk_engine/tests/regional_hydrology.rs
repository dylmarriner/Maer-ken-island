//! Phase 2 Task 5: regional hydrology and coastline exchange.

use std::path::PathBuf;

use mk_core::canon::CanonLocked;
use mk_core::grid::Grid2;
use mk_engine::climate::ClimateState;
use mk_engine::hydrology::{HydrologyState, SoilWater};
use mk_engine::regional::hydrology::{
    bootstrap_regional_hydrology, discharge_m3_s, freshwater_to_coarse_ocean_kg,
    regional_downhill_neighbour, route_to_coarse_sea, step_regional_hydrology,
    step_regional_hydrology_on, FlowNetwork, Receiver, RIVER_MIN_DISCHARGE_M3_S,
};
use mk_engine::weather::WeatherState;
use mk_island::{DomainLevel, IslandDomain, IslandProfile};

const M: DomainLevel = DomainLevel::Medium;
const DAY: f64 = 86_400.0;

fn canon() -> CanonLocked {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/island/canon.json");
    CanonLocked::load(&path).expect("island canon")
}

fn domain() -> IslandDomain {
    IslandDomain::from_profile(IslandProfile::test_small()).unwrap()
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

/// Coarse weather and climate with uniform rain and a mild temperature.
fn forcing(d: &IslandDomain, rain_mm_day: f64) -> (WeatherState, ClimateState) {
    let spec = d.storage_spec(DomainLevel::Coarse);
    let mut weather = WeatherState::new(&spec);
    weather.precipitation = Grid2::new(&spec, rain_mm_day);
    (weather, ClimateState::new(&spec, 285.0))
}

/// Total water (kg): every cell's soil and standing water.
fn water_kg(d: &IslandDomain, s: &HydrologyState) -> f64 {
    let a = d.cell_area_m2(M);
    s.soil_water
        .data()
        .iter()
        .map(|w| w.storage_mm)
        .sum::<f64>()
        * a
        + s.surface_water.data().iter().sum::<f64>() * a
}

/// A 40 × 40 cell (80 km) island, domed, in open sea.
fn island(d: &IslandDomain) -> Grid2<f64> {
    medium(d, |r, c| {
        let (dr, dc) = (r as f64 - 96.0, c as f64 - 120.0);
        let dist = dr.hypot(dc);
        if dist < 25.0 {
            400.0 * (1.0 - dist / 25.0) + 2.0
        } else {
            -50.0
        }
    })
}

#[test]
fn nothing_wraps_across_the_domain_edge() {
    let d = domain();
    // A high plateau with a deep trench along the *east* edge only: with
    // longitude wrap the west-edge cells would drain into it.
    let t = medium(&d, |_, c| if c == d.cols(M) - 1 { -10.0 } else { 100.0 });
    assert_eq!(regional_downhill_neighbour(&t, 5, 0), None);
    assert_eq!(
        regional_downhill_neighbour(&t, 5, d.cols(M) - 2),
        Some((4, d.cols(M) - 1))
            .or(Some((5, d.cols(M) - 1)))
            .or(Some((6, d.cols(M) - 1)))
    );

    // The network never links cells that are not adjacent, and water on a
    // slope down toward the west edge leaves through it, not the east.
    let slope = medium(&d, |_, c| 10.0 + 0.05 * c as f64);
    let net = FlowNetwork::build(&slope);
    let cols = d.cols(M);
    for r in 0..d.rows(M) {
        for c in 0..cols {
            if let Receiver::Cell(j) = net.receiver_of(r, c) {
                let (jr, jc) = (j as usize / cols, j as usize % cols);
                assert!(
                    jr.abs_diff(r) <= 1 && jc.abs_diff(c) <= 1,
                    "({r},{c}) -> ({jr},{jc})"
                );
            }
        }
        assert_eq!(net.receiver_of(r, 0), Receiver::Edge, "row {r} west edge");
    }
    // Rain on this slope reaches the west edge as ocean inflow; the
    // east-edge column holds the divide and gets nothing from the west.
    let canon = canon();
    let (w, cl) = forcing(&d, 20.0);
    let mut s = bootstrap_regional_hydrology(&net, &d);
    for _ in 0..30 {
        s = step_regional_hydrology_on(&canon, &s, &w, &cl, &slope, &d, &net, DAY);
    }
    assert!(s.budget.surface_to_ocean_kg > 0.0);
    let west: f64 = (0..d.rows(M)).map(|r| *s.runoff.get(r, 0)).sum();
    let east: f64 = (0..d.rows(M)).map(|r| *s.runoff.get(r, cols - 1)).sum();
    assert!(west > 5.0 * east.max(1e-9), "west {west} east {east}");
}

#[test]
fn rain_runs_to_the_sea_and_every_kilogram_is_accounted_for() {
    let d = domain();
    let canon = canon();
    let t = island(&d);
    let net = FlowNetwork::build(&t);
    let (w, cl) = forcing(&d, 15.0);
    let mut s = HydrologyState::new(&d.storage_spec(M), 0.5);
    let mut delivered = 0.0;
    for step in 0..20 {
        let before = water_kg(&d, &s);
        let next = step_regional_hydrology_on(&canon, &s, &w, &cl, &t, &d, &net, DAY);
        let b = next.budget;
        let expected = b.precipitation_kg
            - b.surface_evaporation_kg
            - b.soil_evaporation_kg
            - b.deep_drainage_kg
            - b.surface_to_ocean_kg
            + b.ocean_to_sea_soil_kg;
        let change = water_kg(&d, &next) - before;
        assert!(
            (change - expected).abs() <= 1e-9 * b.precipitation_kg.max(1.0),
            "step {step}: storage changed {change}, budget says {expected}"
        );
        if step >= 1 {
            // The coarse ocean receives exactly what the land lost.
            let to_sea = freshwater_to_coarse_ocean_kg(&next, &net, &d, DAY);
            let total: f64 = to_sea.data().iter().sum();
            assert!(
                (total - b.surface_to_ocean_kg).abs() <= 1e-9 * total.max(1.0),
                "coarse ocean {total} vs budget {}",
                b.surface_to_ocean_kg
            );
            delivered += b.surface_to_ocean_kg;
        }
        s = next;
    }
    assert!(delivered > 0.0, "no rain reached the sea");
}

#[test]
fn a_river_reaches_the_sea_within_the_step() {
    let d = domain();
    let canon = canon();
    // A 5-cell-wide valley, 100 cells (200 km) long, falling to the sea.
    let t = medium(&d, |r, c| {
        if (40..45).contains(&r) && (20..120).contains(&c) {
            2.0 + 0.5 * (c as f64 - 20.0)
        } else {
            -20.0
        }
    });
    let net = FlowNetwork::build(&t);
    let (w, cl) = forcing(&d, 40.0);
    // Soils at field capacity, so rain runs off instead of wetting them.
    let mut s = HydrologyState::new(&d.storage_spec(M), 1.0);
    for _ in 0..3 {
        s = step_regional_hydrology_on(&canon, &s, &w, &cl, &t, &d, &net, DAY);
    }
    // Water that fell 200 km upstream is already at the mouth.
    assert!(s.budget.surface_to_ocean_kg > 0.0);
    let mouth = (0..5)
        .map(|k| discharge_m3_s(&s, &d, 40 + k, 20))
        .fold(0.0, f64::max);
    assert!(
        mouth > RIVER_MIN_DISCHARGE_M3_S,
        "mouth discharge {mouth} m³/s"
    );
    // Discharge grows downstream as tributary cells add runoff.
    let upstream = (0..5)
        .map(|k| discharge_m3_s(&s, &d, 40 + k, 100))
        .fold(0.0, f64::max);
    assert!(mouth > upstream, "mouth {mouth} vs upstream {upstream}");
}

#[test]
fn a_depression_is_a_lake_that_fills_then_spills() {
    let d = domain();
    let canon = canon();
    // A plateau draining west to the sea, with a 2 m deep basin.
    let t = medium(&d, |r, c| {
        if !(50..90).contains(&r) || !(30..110).contains(&c) {
            return -20.0;
        }
        let base = 30.0 + 0.02 * (c as f64 - 30.0);
        let basin = (68..73).contains(&r) && (66..71).contains(&c);
        if basin {
            base - 2.0
        } else {
            base
        }
    });
    let net = FlowNetwork::build(&t);
    let (centre_r, centre_c) = (70, 68);
    let depth = net.lake_depth_m(centre_r, centre_c);
    assert!(depth > 0.5, "lake depth {depth} m");
    let cap_mm = depth * 1_000.0;

    // Bootstrapped lakes are full.
    let full = bootstrap_regional_hydrology(&net, &d);
    assert!((full.surface_water.get(centre_r, centre_c) - cap_mm).abs() < 1e-6);

    // An empty lake fills from rain and river water, with no outflow
    // from the lake cell until it is full, then spills.
    let (w, cl) = forcing(&d, 60.0);
    let mut s = HydrologyState::new(&d.storage_spec(M), 0.9);
    let (mut filled_at, mut spilled_at) = (None, None);
    for step in 0..60 {
        s = step_regional_hydrology_on(&canon, &s, &w, &cl, &t, &d, &net, DAY);
        let stored = *s.surface_water.get(centre_r, centre_c);
        assert!(
            stored <= cap_mm + 1e-6,
            "lake over spill level: {stored} > {cap_mm}"
        );
        let out = *s.generated_runoff.as_ref().unwrap().get(centre_r, centre_c);
        if out > 0.0 && spilled_at.is_none() {
            spilled_at = Some(step);
            assert!(
                stored >= cap_mm - 1e-6,
                "spilled at {stored} of {cap_mm} mm"
            );
        }
        if stored >= cap_mm - 1e-6 && filled_at.is_none() {
            filled_at = Some(step);
        }
    }
    assert!(filled_at.is_some(), "the lake never filled");
    assert!(spilled_at.is_some(), "the lake never spilled");
    assert!(filled_at <= spilled_at);
}

#[test]
fn a_cell_that_floods_gives_its_water_to_the_ocean() {
    let d = domain();
    let canon = canon();
    let t = island(&d);
    let net = FlowNetwork::build(&t);
    let (w, cl) = forcing(&d, 15.0);
    let mut s = HydrologyState::new(&d.storage_spec(M), 0.5);
    for _ in 0..5 {
        s = step_regional_hydrology_on(&canon, &s, &w, &cl, &t, &d, &net, DAY);
    }
    // The sea rises over a coastal cell (elevation 2 m) and a hill cell.
    let (coast, hill) = ((96, 144), (96, 120));
    assert!(*t.get(coast.0, coast.1) > 0.0);
    let held = |s: &HydrologyState, p: (usize, usize)| {
        (s.surface_water.get(p.0, p.1) + s.soil_water.get(p.0, p.1).storage_mm) * d.cell_area_m2(M)
    };
    let (soil_before, standing_before) = (
        s.soil_water.get(coast.0, coast.1).storage_mm,
        *s.surface_water.get(coast.0, coast.1),
    );
    let flooded = {
        let (cr, cc) = coast;
        medium(&d, |r, c| {
            if (r, c) == (cr, cc) {
                -1.0
            } else {
                *t.get(r, c)
            }
        })
    };
    let net2 = FlowNetwork::build(&flooded);
    let after = step_regional_hydrology_on(&canon, &s, &w, &cl, &flooded, &d, &net2, DAY);
    // Its standing water joined the ocean, its soil is held saturated,
    // and the budget closes.
    assert!(after.budget.surface_to_ocean_kg >= standing_before * d.cell_area_m2(M) - 1e-6);
    assert_eq!(*after.soil_water.get(coast.0, coast.1), SoilWater::new(1.0));
    assert_eq!(*after.surface_water.get(coast.0, coast.1), 0.0);
    let b = after.budget;
    let expected = b.precipitation_kg
        - b.surface_evaporation_kg
        - b.soil_evaporation_kg
        - b.deep_drainage_kg
        - b.surface_to_ocean_kg
        + b.ocean_to_sea_soil_kg;
    let change = water_kg(&d, &after) - water_kg(&d, &s);
    assert!((change - expected).abs() <= 1e-9 * b.precipitation_kg.max(1.0));
    assert!(soil_before <= 300.0 && held(&s, hill) > 0.0);
}

#[test]
fn a_zero_step_changes_nothing_and_steps_are_deterministic() {
    let d = domain();
    let canon = canon();
    let t = island(&d);
    let (w, cl) = forcing(&d, 15.0);
    let s0 = HydrologyState::new(&d.storage_spec(M), 0.5);
    let run = || {
        let mut s = s0.clone();
        for _ in 0..5 {
            s = step_regional_hydrology(&canon, &s, &w, &cl, &t, &d, DAY);
        }
        s
    };
    let a = run();
    assert_eq!(
        serde_json::to_string(&a).unwrap(),
        serde_json::to_string(&run()).unwrap()
    );
    let zero = step_regional_hydrology(&canon, &a, &w, &cl, &t, &d, 0.0);
    assert_eq!(zero.budget, Default::default());
    assert_eq!(zero.soil_water, a.soil_water);
    assert_eq!(zero.surface_water, a.surface_water);
}

#[test]
fn river_water_on_a_coarse_land_cell_reaches_the_nearest_sea_cell() {
    // A 3 x 4 coarse window: sea down the west column, land elsewhere. An
    // outlet whose medium cell sits inside a coarse land cell would leave
    // its water where the ocean keeps no column.
    let spec = mk_core::grid::GridSpec::new(3, 4);
    let elevation = mk_core::grid::Grid2::from_data(
        &spec,
        vec![
            -50.0, 10.0, 20.0, 30.0, //
            -50.0, 15.0, 25.0, 35.0, //
            -50.0, 10.0, 20.0, 30.0,
        ],
    );
    let mut water = vec![0.0; 12];
    water[1] = 4.0e6; // beside the sea
    water[7] = 1.0e6; // three cells inland, nearest the middle sea cell
    water[8] = 2.0e5; // already at sea
    let routed = route_to_coarse_sea(&mk_core::grid::Grid2::from_data(&spec, water), &elevation);
    let out = routed.data();
    assert_eq!(out[0], 4.0e6);
    assert_eq!(out[4], 1.0e6);
    assert_eq!(out[8], 2.0e5);
    for (i, kg) in out.iter().enumerate() {
        if elevation.data()[i] > 0.0 {
            assert_eq!(*kg, 0.0, "water left on land cell {i}");
        }
    }
    assert_eq!(out.iter().sum::<f64>(), 5.2e6);
}
