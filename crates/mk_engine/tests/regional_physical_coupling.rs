//! Phase 2 Task 6: the coupled regional physical tick.

use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use mk_core::canon::CanonLocked;
use mk_engine::regional::physical::RegionalPhysicalState;
use mk_island::{DomainLevel, IslandDomain, IslandProfile};

const C: DomainLevel = DomainLevel::Coarse;
const M: DomainLevel = DomainLevel::Medium;
const DAY: u64 = 86_400;

struct Base {
    canon: Arc<CanonLocked>,
    domain: IslandDomain,
    seed: [u8; 32],
    state: RegionalPhysicalState,
}

fn repo(path: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path)
}

/// The owner's island, bootstrapped once and cloned per test.
fn base() -> &'static Base {
    static BASE: OnceLock<Base> = OnceLock::new();
    BASE.get_or_init(|| {
        let canon = Arc::new(CanonLocked::load(&repo("fixtures/island/canon.json")).unwrap());
        let profile = IslandProfile::load(&repo("fixtures/island/default_profile.json")).unwrap();
        let seed = profile.seed_bytes().expect("pinned seed");
        let domain = IslandDomain::from_profile(profile).unwrap();
        let state = RegionalPhysicalState::bootstrap(&canon, &domain, seed).expect("bootstrap");
        Base {
            canon,
            domain,
            seed,
            state,
        }
    })
}

/// Soil and standing water over every medium cell (kg).
fn water_kg(b: &Base, s: &RegionalPhysicalState) -> f64 {
    let a = b.domain.cell_area_m2(M);
    (s.hydrology
        .soil_water
        .data()
        .iter()
        .map(|w| w.storage_mm)
        .sum::<f64>()
        + s.hydrology.surface_water.data().iter().sum::<f64>())
        * a
}

fn run(b: &Base, days: u64) -> (RegionalPhysicalState, Vec<f64>) {
    let mut s = b.state.clone();
    let mut closure_errors = Vec::new();
    for day in 1..=days {
        let before = water_kg(b, &s);
        s.step(&b.canon, &b.domain, b.seed, (day * DAY) as f64, day, DAY)
            .expect("step");
        let bud = s.hydrology.budget;
        let expected = bud.precipitation_kg
            - bud.surface_evaporation_kg
            - bud.soil_evaporation_kg
            - bud.deep_drainage_kg
            - bud.surface_to_ocean_kg
            + bud.ocean_to_sea_soil_kg;
        closure_errors
            .push((water_kg(b, &s) - before - expected).abs() / bud.precipitation_kg.max(1.0));
    }
    (s, closure_errors)
}

#[test]
fn thirty_days_change_the_weather_stay_finite_conserve_water_and_replay_exactly() {
    let b = base();
    let start = b.state.state_hash();
    let (a, closure) = run(b, 30);

    // Bootstrap left the world on its seasonal cycle at t = 0.
    assert_eq!(b.state.sim_time_seconds, 0.0);
    assert_ne!(a.state_hash(), start, "thirty days changed nothing");
    assert_eq!(a.sim_time_seconds, 30.0 * DAY as f64);

    // Weather, ocean and hydrology all moved.
    let moved = |x: &[f64], y: &[f64]| x.iter().zip(y).any(|(p, q)| p != q);
    assert!(moved(
        a.weather.precipitation.data(),
        b.state.weather.precipitation.data()
    ));
    assert!(moved(
        &a.ocean
            .columns
            .data()
            .iter()
            .map(|c| c.surface_temp)
            .collect::<Vec<_>>(),
        &b.state
            .ocean
            .columns
            .data()
            .iter()
            .map(|c| c.surface_temp)
            .collect::<Vec<_>>()
    ));
    assert!(moved(
        &a.hydrology
            .soil_water
            .data()
            .iter()
            .map(|w| w.storage_mm)
            .collect::<Vec<_>>(),
        &b.state
            .hydrology
            .soil_water
            .data()
            .iter()
            .map(|w| w.storage_mm)
            .collect::<Vec<_>>()
    ));

    // Everything finite; the land mask (and so the terrain) is preserved.
    for g in [
        &a.climate.surface_temperature,
        &a.climate.atmos_temperature,
        &a.weather.precipitation,
        &a.weather.moisture,
        &a.diurnal_offset_k,
    ] {
        assert!(g.data().iter().all(|v| v.is_finite()));
    }
    assert!(a
        .ocean
        .columns
        .data()
        .iter()
        .all(|c| c.salinity.is_finite() && c.density.is_finite()));
    assert_eq!(
        a.geophysics.land_mask.data(),
        b.state.geophysics.land_mask.data()
    );
    // Land cells hold no ocean column; ocean cells do.
    let (rows, cols) = (b.domain.rows(C), b.domain.cols(C));
    for r in 0..rows {
        for c in 0..cols {
            let depth = a.ocean.columns.get(r, c).depth;
            if *a.coarse_elevation_m().get(r, c) > 0.0 {
                assert_eq!(depth, 0.0);
            } else {
                assert!(depth > 0.0);
            }
        }
    }

    // The water budget closed on every step (soil + standing water =
    // rain - evaporation - drainage - to the sea).
    let worst = closure.iter().cloned().fold(0.0, f64::max);
    assert!(worst <= 1e-9, "worst budget closure error {worst:e}");
    assert!(a.river_water_to_ocean_kg >= 0.0);

    // Identical replay: the same final bits.
    let (again, _) = run(b, 30);
    assert_eq!(a.state_hash(), again.state_hash());
}

#[test]
fn a_zero_step_changes_nothing() {
    let b = base();
    let mut s = b.state.clone();
    s.step(&b.canon, &b.domain, b.seed, 5.0 * DAY as f64, 0, 0)
        .unwrap();
    assert_eq!(s.state_hash(), b.state.state_hash());
}

#[test]
fn the_physical_state_allocates_no_planetary_grid() {
    let b = base();
    let s = &b.state;
    let (rows_c, cols_c) = (b.domain.rows(C), b.domain.cols(C));
    for (name, nlat, nlon) in [
        (
            "climate",
            s.climate.surface_temperature.nlat(),
            s.climate.surface_temperature.nlon(),
        ),
        (
            "weather",
            s.weather.precipitation.nlat(),
            s.weather.precipitation.nlon(),
        ),
        ("ocean", s.ocean.columns.nlat(), s.ocean.columns.nlon()),
        ("tides", s.tides.potential.nlat(), s.tides.potential.nlon()),
        (
            "insolation",
            s.insolation.toa_w_m2.nlat(),
            s.insolation.toa_w_m2.nlon(),
        ),
    ] {
        assert_eq!(
            (nlat, nlon),
            (rows_c, cols_c),
            "{name} is not on the coarse grid"
        );
    }
    assert_eq!(s.hydrology.soil_water.nlat(), b.domain.rows(M));
    // Land is a small share of the medium grid, so hydrology iterates
    // ~6% of its cells.
    let land = s.flow_network().land_cells() as f64;
    assert!(
        land / (b.domain.rows(M) * b.domain.cols(M)) as f64 <= 0.10,
        "land cells {land}"
    );
}
