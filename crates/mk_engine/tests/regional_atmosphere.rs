//! Phase 2 Task 3: regional climate, weather, orographic rain and the
//! diurnal cycle on the island's coarse grid.

use std::path::PathBuf;

use mk_core::canon::CanonLocked;
use mk_core::grid::Grid2;
use mk_engine::climate::{ClimateForcing, ClimateState};
use mk_engine::regional::boundary::sample_regional_boundaries_with_background;
use mk_engine::regional::climate::{diurnal_offset_k, step_regional_climate, DiurnalSurface};
use mk_engine::regional::weather::{apply_orographic_precipitation, step_regional_weather};
use mk_engine::regional::zonal::{ZonalBackgroundState, DEFAULT_BAND_COUNT};
use mk_island::{DomainLevel, IslandDomain, IslandProfile, RegionalBoundaryState};

const STEPS_PER_ORBIT: usize = 54;
const C: DomainLevel = DomainLevel::Coarse;

fn canon() -> CanonLocked {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/island/canon.json");
    CanonLocked::load(&path).expect("island canon")
}

struct Run {
    canon: CanonLocked,
    domain: IslandDomain,
    background: ZonalBackgroundState,
    boundaries: RegionalBoundaryState,
    elevation: Grid2<f64>,
    climate: ClimateState,
    /// (sub-solar latitude, interior mean temperature) per step of the
    /// last orbit.
    seasonal: Vec<(f64, f64)>,
}

fn elevation(domain: &IslandDomain, f: impl Fn(usize, usize) -> f64) -> Grid2<f64> {
    let (rows, cols) = (domain.rows(C), domain.cols(C));
    Grid2::from_data(
        &domain.storage_spec(C),
        (0..rows)
            .flat_map(|r| (0..cols).map(move |c| (r, c)))
            .map(|(r, c)| f(r, c))
            .collect(),
    )
}

/// Spin the regional climate through four orbits beside the background
/// (the ocean relaxes in ~1.4 orbits, so the last is on the periodic cycle).
fn spin(elevation_of: impl Fn(&IslandDomain) -> Grid2<f64>) -> Run {
    let canon = canon();
    let domain = IslandDomain::from_profile(IslandProfile::default_nz_scale()).unwrap();
    let elev = elevation_of(&domain);
    let spec = domain.storage_spec(C);
    let geothermal = Grid2::new(&spec, 0.09);
    let mut background = ZonalBackgroundState::bootstrap(&canon, DEFAULT_BAND_COUNT);
    let dt = canon.orbital_period_s / STEPS_PER_ORBIT as f64;
    let mut boundaries =
        sample_regional_boundaries_with_background([5u8; 32], &canon, &domain, &background, 0.0);
    // Settle straight to equilibrium at t = 0, as the world does, so two
    // orbits of spin-up are about the seasonal cycle, not the cold start.
    let orbit0 = mk_engine::orbit::step_orbit(&canon, 0.0);
    let rotation0 = mk_engine::rotation::step_rotation(0.0, orbit0.mean_anomaly, &canon);
    let mut climate = step_regional_climate(
        &ClimateState::default_for_grid(&spec),
        &ClimateForcing {
            toa_solar_flux_w_m2: canon.solar_constant_w_m2 / (orbit0.r_over_a * orbit0.r_over_a),
            solar_declination_rad: rotation0.subsolar_latitude,
            albedo: canon.albedo_baseline,
            surface_pressure_pa: canon.sea_level_pressure_pa,
            surface_gravity_m_s2: canon.surface_gravity_m_s2,
            geothermal_flux_w_m2: &geothermal,
            volcanic_co2_mol_yr: 1.0e11,
            elevation_m: &elev,
            dt_seconds: f64::INFINITY,
        },
        &domain,
        &background,
        &boundaries.atmosphere,
        &boundaries.ocean,
        // Calm: no air carried in, the energy balance on its own.
        &mk_core::grid::Grid2::new(
            &spec,
            mk_engine::weather::WindVector {
                u_east: 0.0,
                v_north: 0.0,
            },
        ),
    );
    let mut seasonal = Vec::new();
    for step in 1..=4 * STEPS_PER_ORBIT {
        let t = step as f64 * dt;
        background.step(&canon, t, dt);
        boundaries =
            sample_regional_boundaries_with_background([5u8; 32], &canon, &domain, &background, t);
        let orbit = mk_engine::orbit::step_orbit(&canon, t);
        let rotation = mk_engine::rotation::step_rotation(t, orbit.mean_anomaly, &canon);
        climate = step_regional_climate(
            &climate,
            &ClimateForcing {
                toa_solar_flux_w_m2: canon.solar_constant_w_m2 / (orbit.r_over_a * orbit.r_over_a),
                solar_declination_rad: rotation.subsolar_latitude,
                albedo: canon.albedo_baseline,
                surface_pressure_pa: canon.sea_level_pressure_pa,
                surface_gravity_m_s2: canon.surface_gravity_m_s2,
                geothermal_flux_w_m2: &geothermal,
                volcanic_co2_mol_yr: 1.0e11,
                elevation_m: &elev,
                dt_seconds: dt,
            },
            &domain,
            &background,
            &boundaries.atmosphere,
            &boundaries.ocean,
            &calm(&domain),
        );
        if step > 3 * STEPS_PER_ORBIT {
            seasonal.push((
                rotation.subsolar_latitude,
                interior_mean(&domain, &climate, None),
            ));
        }
    }
    Run {
        canon,
        domain,
        background,
        boundaries,
        elevation: elev,
        climate,
        seasonal,
    }
}

/// Mean surface temperature of the interior (clear of the edge relaxation),
/// of one row or of all rows.
fn interior_mean(domain: &IslandDomain, climate: &ClimateState, row: Option<usize>) -> f64 {
    let (rows, cols) = (domain.rows(C), domain.cols(C));
    let rr = row.map_or(40..rows - 40, |r| r..r + 1);
    let (mut sum, mut n) = (0.0, 0.0);
    for r in rr {
        for c in 40..cols - 40 {
            sum += climate.surface_temperature.get(r, c);
            n += 1.0;
        }
    }
    sum / n
}

fn ocean(domain: &IslandDomain) -> Grid2<f64> {
    elevation(domain, |_, _| -3000.0)
}

#[test]
fn regional_climate_follows_the_background_and_the_southern_seasons() {
    let run = spin(ocean);
    let (domain, bg) = (&run.domain, &run.background);

    // Interior rows match the background's sea-surface temperature at the
    // same latitude (the island is open ocean here).
    for row in (40..domain.rows(C) - 40).step_by(10) {
        let lat = domain.latitude_rad_for_row(C, row);
        let want = bg.sea_surface_temperature_at(lat);
        let got = interior_mean(domain, &run.climate, Some(row));
        assert!(
            (got - want).abs() <= 1.0,
            "row {row} ({:.1}°): regional {got:.2} K, background {want:.2} K",
            lat.to_degrees()
        );
    }

    // Warmer at the northern edge (toward the equator) than the southern.
    let south = interior_mean(domain, &run.climate, Some(40));
    let north = interior_mean(domain, &run.climate, Some(domain.rows(C) - 41));
    assert!(
        north > south + 1.0,
        "north {north:.2} K, south {south:.2} K"
    );

    // Southern-hemisphere phase: the warm peak follows the austral
    // (southern) solstice and the cool peak the northern one, each by less
    // than a third of an orbit. Upstream's ocean relaxes with radiative
    // feedback only (~3 W/m²/K, so ~1.4 orbits), which puts the lag near a
    // quarter orbit; observed mixed-layer SST lags the solstice by 1-2
    // months (recorded in docs/island/DEVIATIONS.md, D25).
    let n = run.seasonal.len();
    // Index of the step maximising `key`.
    let argmax = |key: &dyn Fn(&(f64, f64)) -> f64| {
        (0..n)
            .max_by(|&i, &j| key(&run.seasonal[i]).total_cmp(&key(&run.seasonal[j])))
            .unwrap()
    };
    let austral_solstice = argmax(&|s| -s.0); // most southern sun
    let boreal_solstice = argmax(&|s| s.0);
    let warmest = argmax(&|s| s.1);
    let coolest = argmax(&|s| -s.1);
    let lag = |from: usize, to: usize| ((to + n - from) % n) as f64 / n as f64;
    assert!(
        lag(austral_solstice, warmest) < 0.33,
        "warm peak {:.2} orbits after the austral solstice",
        lag(austral_solstice, warmest)
    );
    assert!(
        lag(boreal_solstice, coolest) < 0.33,
        "cool peak {:.2} orbits after the boreal solstice",
        lag(boreal_solstice, coolest)
    );
}

#[test]
fn the_climate_is_deterministic_finite_and_has_no_cell_to_cell_jumps() {
    let a = spin(ocean);
    let b = spin(ocean);
    assert_eq!(
        serde_json::to_string(&a.climate).unwrap(),
        serde_json::to_string(&b.climate).unwrap()
    );
    let (rows, cols) = (a.domain.rows(C), a.domain.cols(C));
    let t = &a.climate.surface_temperature;
    for r in 0..rows {
        for c in 0..cols {
            assert!(t.get(r, c).is_finite());
            if r + 1 < rows {
                assert!(
                    (t.get(r + 1, c) - t.get(r, c)).abs() < 3.0,
                    "row jump at ({r},{c})"
                );
            }
            if c + 1 < cols {
                assert!(
                    (t.get(r, c + 1) - t.get(r, c)).abs() < 3.0,
                    "col jump at ({r},{c})"
                );
            }
        }
    }
}

#[test]
fn extreme_edge_forcing_stays_finite_and_physical() {
    let run = spin(ocean);
    let mut b = run.boundaries.clone();
    for e in &mut b.ocean.edges {
        e.sea_surface_temperature_k
            .iter_mut()
            .for_each(|v| *v = 1.0e9);
    }
    for e in &mut b.atmosphere.edges {
        e.air_temperature_k.iter_mut().for_each(|v| *v = -1.0e9);
        e.wind_u_m_s.iter_mut().for_each(|v| *v = 1.0e9);
    }
    let spec = run.domain.storage_spec(C);
    let geothermal = Grid2::new(&spec, 0.09);
    let next = step_regional_climate(
        &run.climate,
        &ClimateForcing {
            toa_solar_flux_w_m2: run.canon.solar_constant_w_m2,
            solar_declination_rad: 0.0,
            albedo: run.canon.albedo_baseline,
            surface_pressure_pa: run.canon.sea_level_pressure_pa,
            surface_gravity_m_s2: run.canon.surface_gravity_m_s2,
            geothermal_flux_w_m2: &geothermal,
            volcanic_co2_mol_yr: 1.0e11,
            elevation_m: &run.elevation,
            dt_seconds: 86_400.0,
        },
        &run.domain,
        &run.background,
        &b.atmosphere,
        &b.ocean,
        &calm(&run.domain),
    );
    assert!(next
        .surface_temperature
        .data()
        .iter()
        .chain(next.atmos_temperature.data())
        .all(|t| t.is_finite() && (100.0..=450.0).contains(t)));
    let weather = step_regional_weather(
        &run.canon,
        &next,
        &run.elevation,
        &run.domain,
        &run.background,
        &b.atmosphere,
    );
    assert!(weather
        .wind
        .data()
        .iter()
        .all(|w| w.u_east.abs() <= 61.0 && w.v_north.is_finite()));

    // A zero step changes nothing.
    let same = step_regional_climate(
        &run.climate,
        &ClimateForcing {
            toa_solar_flux_w_m2: run.canon.solar_constant_w_m2,
            solar_declination_rad: 0.0,
            albedo: run.canon.albedo_baseline,
            surface_pressure_pa: run.canon.sea_level_pressure_pa,
            surface_gravity_m_s2: run.canon.surface_gravity_m_s2,
            geothermal_flux_w_m2: &geothermal,
            volcanic_co2_mol_yr: 1.0e11,
            elevation_m: &run.elevation,
            dt_seconds: 0.0,
        },
        &run.domain,
        &run.background,
        &run.boundaries.atmosphere,
        &run.boundaries.ocean,
        &calm(&run.domain),
    );
    assert_eq!(
        serde_json::to_string(&same).unwrap(),
        serde_json::to_string(&run.climate).unwrap()
    );
}

#[test]
fn weather_rows_carry_the_backgrounds_rain_without_spherical_weights() {
    let run = spin(ocean);
    let weather = step_regional_weather(
        &run.canon,
        &run.climate,
        &run.elevation,
        &run.domain,
        &run.background,
        &run.boundaries.atmosphere,
    );
    let cols = run.domain.cols(C);
    for row in (0..run.domain.rows(C)).step_by(7) {
        let mean = (0..cols)
            .map(|c| weather.precipitation.get(row, c))
            .sum::<f64>()
            / cols as f64;
        let want = run
            .background
            .precipitation_at(run.domain.latitude_rad_for_row(C, row));
        assert!(
            (mean - want).abs() < 1e-6 * want.max(1.0),
            "row {row}: {mean} vs {want}"
        );
    }
    // Mid-latitude surface westerlies; meridional wind bounded.
    let w = weather.wind.get(80, 100);
    assert!(w.u_east > 0.0, "u = {}", w.u_east);
    assert!(weather.wind.data().iter().all(|w| w.v_north.abs() <= 10.0));
    // Deterministic.
    let again = step_regional_weather(
        &run.canon,
        &run.climate,
        &run.elevation,
        &run.domain,
        &run.background,
        &run.boundaries.atmosphere,
    );
    assert_eq!(
        serde_json::to_string(&weather).unwrap(),
        serde_json::to_string(&again).unwrap()
    );
}

#[test]
fn a_ridge_in_westerlies_wets_its_west_flank_and_dries_the_air_beyond_it() {
    // A north-south ridge, 2.5 km high and ~60 km wide, in open ocean.
    let ridge = |d: &IslandDomain| {
        elevation(d, |_, c| {
            let x = (c as f64 - 100.0) / 3.0;
            if x.abs() < 1.0 {
                2500.0 * (1.0 - x * x)
            } else {
                -3000.0
            }
        })
    };
    let run = spin(ridge);
    let mut weather = step_regional_weather(
        &run.canon,
        &run.climate,
        &run.elevation,
        &run.domain,
        &run.background,
        &run.boundaries.atmosphere,
    );
    let plain = weather.clone();
    apply_orographic_precipitation(&mut weather, &run.elevation, &run.domain);

    // Nothing changes over the sea, and nothing is taken from it: the rain
    // the ridge wrings out is vapour that would have passed on, so the
    // total rises rather than being moved from elsewhere (deviation D38).
    for (i, (&h, (&before, &after))) in run
        .elevation
        .data()
        .iter()
        .zip(
            plain
                .precipitation
                .data()
                .iter()
                .zip(weather.precipitation.data()),
        )
        .enumerate()
    {
        if h <= 0.0 {
            assert_eq!(before, after, "sea cell {i} changed");
        }
    }
    let before: f64 = plain.precipitation.data().iter().sum();
    let after: f64 = weather.precipitation.data().iter().sum();
    assert!(after > before, "{before} -> {after}");

    let flank = |field: &mk_engine::weather::WeatherState, cols: std::ops::Range<usize>| {
        let mut sum = 0.0;
        let mut n = 0.0;
        for r in 60..100 {
            for c in cols.clone() {
                sum += field.precipitation.get(r, c);
                n += 1.0;
            }
        }
        sum / n
    };
    let (west, east) = (flank(&weather, 95..99), flank(&weather, 101..105));
    assert!(west > east, "west {west:.2} mm/day, east {east:.2} mm/day");
    // The reference pack's windward/leeward ratio for a barrier is 3-8; a
    // 60 km ridge on a 12 km grid should at least double the contrast.
    assert!(west / east >= 2.0, "ratio {:.2}", west / east);
    // And the lee is drier than it would have been with no ridge upwind:
    // its air lost vapour on the way over.
    assert!(
        east < flank(&plain, 101..105),
        "the lee is not in a shadow: {east:.3} against {:.3}",
        flank(&plain, 101..105)
    );
}

#[test]
fn the_diurnal_range_follows_the_surface_and_the_daily_mean_is_unchanged() {
    // A 600 km square of land with a valley, in open ocean.
    let island = |d: &IslandDomain| {
        elevation(d, |r, c| {
            let in_block = (60..120).contains(&r) && (70..130).contains(&c);
            let valley = (r, c) == (90, 100);
            match (in_block, valley) {
                (_, true) => 50.0,
                (true, _) => 700.0,
                _ => -3000.0,
            }
        })
    };
    let canon = canon();
    let domain = IslandDomain::from_profile(IslandProfile::default_nz_scale()).unwrap();
    let elev = island(&domain);
    let surface = DiurnalSurface::from_elevation(&elev, domain.cell_size_m(C));
    let dry = Grid2::new(&domain.storage_spec(C), 0.0);
    let rainy = Grid2::new(&domain.storage_spec(C), 6.0);

    let samples = 24;
    let day = |precip: &Grid2<f64>| -> Vec<Grid2<f64>> {
        (0..samples)
            .map(|k| {
                let lon = std::f64::consts::TAU * k as f64 / samples as f64;
                diurnal_offset_k(&surface, &elev, precip, &domain, &canon, lon)
            })
            .collect()
    };
    let clear = day(&dry);
    let range = |days: &[Grid2<f64>], r: usize, c: usize| {
        let v: Vec<f64> = days.iter().map(|g| *g.get(r, c)).collect();
        v.iter().cloned().fold(f64::MIN, f64::max) - v.iter().cloned().fold(f64::MAX, f64::min)
    };
    let (ocean_r, coastal_r, inland_r, valley_r) = (
        range(&clear, 10, 10),
        range(&clear, 90, 71),
        range(&clear, 75, 100),
        range(&clear, 90, 100),
    );
    // Reference pack (24 h day): ocean 0.1-0.5 K, coastal land 5-9 K,
    // inland grass 9-15 K. A 36 h day widens the land range.
    assert!((0.1..=0.6).contains(&ocean_r), "ocean {ocean_r}");
    assert!(
        coastal_r > 5.0 && coastal_r < inland_r,
        "coastal {coastal_r}, inland {inland_r}"
    );
    assert!(inland_r >= 9.0, "inland {inland_r}");
    assert!(
        inland_r / ocean_r >= 9.0 / 0.5,
        "ratio {}",
        inland_r / ocean_r
    );
    assert!(valley_r > coastal_r, "valley {valley_r}");

    // Cloud cuts the land range by 25-50% and leaves the ocean alone.
    let overcast = day(&rainy);
    let cut = 1.0 - range(&overcast, 75, 100) / inland_r;
    assert!((0.25..=0.5).contains(&cut), "cloud reduction {cut}");
    assert!((range(&overcast, 10, 10) - ocean_r).abs() < 1e-9);

    // Zero mean over a day for every cell: the daily mean is unchanged.
    for (r, c) in [(10, 10), (75, 100), (90, 100), (90, 71)] {
        let mean: f64 = clear.iter().map(|g| *g.get(r, c)).sum::<f64>() / samples as f64;
        assert!(mean.abs() < 1e-9, "daily mean offset {mean} at ({r},{c})");
    }
}

/// No wind: no air carried in, the energy balance on its own.
fn calm(domain: &IslandDomain) -> Grid2<mk_engine::weather::WindVector> {
    Grid2::new(
        &domain.storage_spec(C),
        mk_engine::weather::WindVector {
            u_east: 0.0,
            v_north: 0.0,
        },
    )
}
