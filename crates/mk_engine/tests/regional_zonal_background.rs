//! Phase 2 Task 2: the zonal background supplies every planet-wide term
//! the island cannot compute from its own cells, and it reproduces the
//! upstream global climate it stands in for.

use mk_core::canon::CanonLocked;
use mk_core::grid::GridSpec;
use mk_core::rng::RngRegistry;
use mk_engine::climate::{step_climate, ClimateForcing, ClimateState};
use mk_engine::regional::boundary::{
    sample_regional_boundaries, sample_regional_boundaries_with_background,
};
use mk_engine::regional::zonal::{ZonalBackgroundState, DEFAULT_BAND_COUNT};
use mk_island::{DomainLevel, Edge, IslandDomain, IslandProfile};
use std::path::PathBuf;

fn canon() -> CanonLocked {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/island/canon.json");
    CanonLocked::load(&path).expect("island canon")
}

/// Annual-mean surface temperature per band of the background over one
/// orbit in `steps` steps.
fn background_annual_means(canon: &CanonLocked, bands: usize, steps: usize) -> (Vec<f64>, f64) {
    let mut bg = ZonalBackgroundState::bootstrap(canon, bands);
    let dt = canon.orbital_period_s / steps as f64;
    let mut sums = vec![0.0; bands];
    let mut global = 0.0;
    for step in 1..=steps {
        bg.step(canon, step as f64 * dt, dt);
        for (s, t) in sums.iter_mut().zip(bg.band_surface_temperature_k()) {
            *s += t;
        }
        global += bg.global_mean_surface_temperature_k;
    }
    (
        sums.into_iter().map(|s| s / steps as f64).collect(),
        global / steps as f64,
    )
}

#[test]
fn the_background_matches_the_upstream_global_climate() {
    // Upstream: the default `WorldState` grid (32 × 64) with its
    // continents and geothermal field, built as `WorldState::new` builds
    // them (the island canon's declared density exception keeps
    // `WorldState::new` itself from accepting it), settled, spun through
    // two orbits like `spin_up_climate`, then stepped for one more orbit.
    let canon = canon();
    let grid = GridSpec::new(32, 64);
    let tectonics = mk_engine::tectonics::step_tectonics(&canon, 0, &grid);
    let volcanism = mk_engine::volcanism::step_volcanism(
        &canon,
        0,
        &tectonics.plates,
        &tectonics.heat_flow,
        mk_engine::volcanism::INITIAL_DEGASSED_FRACTION,
        0.0,
        &RngRegistry::new([3u8; 32]),
        &grid,
    );
    let geothermal = volcanism.heat_flux_w_m2(&grid);
    let volcanic = volcanism.total_co2_mol_yr(&grid);
    let elevation = tectonics.get_elevation_grid();
    let steps = 54;
    let dt = canon.orbital_period_s / steps as f64;
    let forcing = |t: f64, dt: f64| {
        let orbit = mk_engine::orbit::step_orbit(&canon, t);
        let rotation = mk_engine::rotation::step_rotation(t, orbit.mean_anomaly, &canon);
        ClimateForcing {
            toa_solar_flux_w_m2: canon.solar_constant_w_m2 / (orbit.r_over_a * orbit.r_over_a),
            solar_declination_rad: rotation.subsolar_latitude,
            albedo: canon.albedo_baseline,
            surface_pressure_pa: canon.sea_level_pressure_pa,
            surface_gravity_m_s2: canon.surface_gravity_m_s2,
            geothermal_flux_w_m2: &geothermal,
            volcanic_co2_mol_yr: volcanic,
            elevation_m: &elevation,
            dt_seconds: dt,
        }
    };
    let mut climate = step_climate(
        &ClimateState::default_for_grid(&grid),
        &forcing(0.0, f64::INFINITY),
        &grid,
    );
    for step in 1..=2 * steps {
        climate = step_climate(&climate, &forcing(step as f64 * dt, dt), &grid);
    }
    let mut row_sums = vec![0.0; grid.nlat];
    let mut global = 0.0;
    for step in 1..=steps {
        climate = step_climate(
            &climate,
            &forcing((2 * steps + step) as f64 * dt, dt),
            &grid,
        );
        for (row, sum) in row_sums.iter_mut().enumerate() {
            *sum += (0..grid.nlon)
                .map(|c| *climate.surface_temperature.get(row, c))
                .sum::<f64>()
                / grid.nlon as f64;
        }
        global += climate.global_temperature();
    }
    let upstream_rows: Vec<f64> = row_sums.iter().map(|s| s / steps as f64).collect();
    let upstream_global = global / steps as f64;

    let (bands, bg_global) = background_annual_means(&canon, grid.nlat, steps);
    // Tolerances: the background has upstream's land fraction per band,
    // and no longitude; what remains is
    // the uniform mean geothermal flux and the missing longitude.
    for (row, (u, b)) in upstream_rows.iter().zip(&bands).enumerate() {
        assert!(
            (u - b).abs() <= 1.5,
            "band {row}: upstream {u:.2} K, background {b:.2} K"
        );
    }
    assert!(
        (upstream_global - bg_global).abs() <= 0.5,
        "global: upstream {upstream_global:.2} K, background {bg_global:.2} K"
    );

    // The 64-band default agrees with 32 bands on the global mean.
    let (_, bg64) = background_annual_means(&canon, DEFAULT_BAND_COUNT, steps);
    assert!(
        (bg64 - bg_global).abs() <= 0.5,
        "64 bands {bg64:.2} K vs 32 bands {bg_global:.2} K"
    );
}

#[test]
fn the_background_is_deterministic_and_serializable() {
    let canon = canon();
    let run = || {
        let mut bg = ZonalBackgroundState::bootstrap(&canon, DEFAULT_BAND_COUNT);
        for step in 1..=10 {
            bg.step(&canon, step as f64 * 86_400.0, 86_400.0);
        }
        serde_json::to_string(&bg).unwrap()
    };
    let a = run();
    assert_eq!(a, run());
    let back: ZonalBackgroundState = serde_json::from_str(&a).unwrap();
    assert_eq!(serde_json::to_string(&back).unwrap(), a);
}

#[test]
fn a_zero_length_step_changes_nothing() {
    let canon = canon();
    let mut bg = ZonalBackgroundState::bootstrap(&canon, DEFAULT_BAND_COUNT);
    bg.step(&canon, 86_400.0, 86_400.0);
    let before = serde_json::to_string(&bg).unwrap();
    bg.step(&canon, 86_400.0, 0.0);
    assert_eq!(serde_json::to_string(&bg).unwrap(), before);
}

#[test]
fn extreme_finite_inputs_stay_finite() {
    let canon = canon();
    let mut bg = ZonalBackgroundState::bootstrap(&canon, DEFAULT_BAND_COUNT);
    bg.step(&canon, 1.0e15, 1.0e15);
    bg.step(&canon, 1.0e15 + 1.0e-3, 1.0e-3);
    bg.step(&canon, f64::MAX / 4.0, f64::MAX / 4.0);
    let scalars = [
        bg.global_mean_surface_temperature_k,
        bg.mean_absorbed_flux_w_m2,
        bg.global_mean_precipitation_mm_day,
        bg.atmospheric_co2_ppm,
    ];
    assert!(scalars.iter().all(|v| v.is_finite()), "{scalars:?}");
    assert!(bg
        .climate
        .surface_temperature
        .data()
        .iter()
        .all(|t| t.is_finite() && *t > 0.0));
    assert!(bg
        .band_precipitation_mm_day
        .iter()
        .all(|p| p.is_finite() && *p >= 0.0));
}

#[test]
fn the_background_has_a_realistic_zonal_climate() {
    let canon = canon();
    let bg = ZonalBackgroundState::bootstrap(&canon, DEFAULT_BAND_COUNT);
    let n = DEFAULT_BAND_COUNT;
    let t = bg.band_surface_temperature_k();
    // Warm tropics, cold poles; symmetric enough at the equinox-free start.
    assert!(
        t[n / 2] > t[0] + 20.0 && t[n / 2] > t[n - 1] + 20.0,
        "{t:?}"
    );
    // Earth-like global mean and rainfall (canon is Earth-like).
    assert!((280.0..300.0).contains(&bg.global_mean_surface_temperature_k));
    assert!((2.0..3.5).contains(&bg.global_mean_precipitation_mm_day));
    // Rainier at the ITCZ than in the subtropics (~27°).
    let sub = ((27.0_f64.to_radians() / std::f64::consts::PI + 0.5) * n as f64) as usize;
    assert!(bg.band_precipitation_mm_day[n / 2] > bg.band_precipitation_mm_day[sub]);
}

#[test]
fn boundaries_from_the_background_follow_its_bands() {
    let canon = canon();
    let domain = IslandDomain::from_profile(IslandProfile::default_nz_scale()).unwrap();
    let mut bg = ZonalBackgroundState::bootstrap(&canon, DEFAULT_BAND_COUNT);
    let t = 30.0 * 86_400.0;
    bg.step(&canon, t, t);
    let seed = [5u8; 32];
    let provisional = sample_regional_boundaries(seed, &canon, &domain, t);
    let b = sample_regional_boundaries_with_background(seed, &canon, &domain, &bg, t);
    assert!(!b.ocean.provisional && !b.atmosphere.provisional);
    assert!(provisional.ocean.provisional);
    // Astronomy and plates are the same; only the fluid edges change.
    assert_eq!(b.astronomy, provisional.astronomy);
    assert_eq!(b.tectonic, provisional.tectonic);
    assert_ne!(b.ocean.edges, provisional.ocean.edges);

    // Edge SST follows the background band at that latitude (to within the
    // deterministic sub-daily noise, ±0.3 K).
    let south = b
        .ocean
        .edges
        .iter()
        .find(|e| e.edge == Edge::South)
        .unwrap();
    let lat = domain.latitude_rad_for_row(DomainLevel::Coarse, 0);
    let band_t = bg.sea_surface_temperature_at(lat);
    for sst in &south.sea_surface_temperature_k {
        assert!((sst - band_t).abs() <= 0.31, "{sst} vs band {band_t}");
    }
    // The north edge is warmer than the south edge in the southern
    // hemisphere (the island sits at ~41° S).
    let north = b
        .ocean
        .edges
        .iter()
        .find(|e| e.edge == Edge::North)
        .unwrap();
    let mean = |v: &[f64]| v.iter().sum::<f64>() / v.len() as f64;
    assert!(mean(&north.sea_surface_temperature_k) > mean(&south.sea_surface_temperature_k));
    // Every value is finite.
    for e in &b.atmosphere.edges {
        for v in e
            .air_temperature_k
            .iter()
            .chain(&e.wind_u_m_s)
            .chain(&e.wind_v_m_s)
            .chain(&e.specific_humidity_kg_kg)
            .chain(&e.surface_pressure_pa)
        {
            assert!(v.is_finite());
        }
    }
}
