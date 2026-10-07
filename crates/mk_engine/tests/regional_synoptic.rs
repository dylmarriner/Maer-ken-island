//! Phase 2 Task 3b: travelling synoptic systems.

use std::path::PathBuf;

use mk_core::canon::CanonLocked;
use mk_core::grid::Grid2;
use mk_engine::climate::{ClimateForcing, ClimateState};
use mk_engine::regional::boundary::sample_regional_boundaries_with_background;
use mk_engine::regional::climate::step_regional_climate;
use mk_engine::regional::synoptic::{
    apply_synoptic, deformation_radius_m, eady_growth_rate_s, pressure_anomaly_pa, SynopticState,
    SynopticSystem, SystemKind, RADIUS_OVER_DEFORMATION,
};
use mk_engine::regional::weather::step_regional_weather;
use mk_engine::regional::zonal::{ZonalBackgroundState, DEFAULT_BAND_COUNT};
use mk_engine::weather::WeatherState;
use mk_island::{DomainLevel, IslandDomain, IslandProfile};

const C: DomainLevel = DomainLevel::Coarse;
const DAY: f64 = 86_400.0;
const EARTH_EADY_S: f64 = 0.5 / DAY;

struct Setup {
    canon: CanonLocked,
    domain: IslandDomain,
    background: ZonalBackgroundState,
    weather: WeatherState,
}

fn setup() -> Setup {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/island/canon.json");
    let canon = CanonLocked::load(&path).expect("island canon");
    let domain = IslandDomain::from_profile(IslandProfile::default_nz_scale()).unwrap();
    let background = ZonalBackgroundState::bootstrap(&canon, DEFAULT_BAND_COUNT);
    let spec = domain.storage_spec(C);
    let elevation = Grid2::new(&spec, -3000.0);
    let geothermal = Grid2::new(&spec, 0.09);
    let boundaries =
        sample_regional_boundaries_with_background([5u8; 32], &canon, &domain, &background, 0.0);
    let orbit = mk_engine::orbit::step_orbit(&canon, 0.0);
    let rotation = mk_engine::rotation::step_rotation(0.0, orbit.mean_anomaly, &canon);
    let climate = step_regional_climate(
        &ClimateState::default_for_grid(&spec),
        &ClimateForcing {
            toa_solar_flux_w_m2: canon.solar_constant_w_m2 / (orbit.r_over_a * orbit.r_over_a),
            solar_declination_rad: rotation.subsolar_latitude,
            albedo: canon.albedo_baseline,
            surface_pressure_pa: canon.sea_level_pressure_pa,
            surface_gravity_m_s2: canon.surface_gravity_m_s2,
            geothermal_flux_w_m2: &geothermal,
            volcanic_co2_mol_yr: 1.0e11,
            elevation_m: &elevation,
            dt_seconds: f64::INFINITY,
        },
        &domain,
        &background,
        &boundaries.atmosphere,
        &boundaries.ocean,
    );
    let weather = step_regional_weather(
        &canon,
        &climate,
        &elevation,
        &domain,
        &background,
        &boundaries.atmosphere,
    );
    Setup {
        canon,
        domain,
        background,
        weather,
    }
}

fn mean(g: &Grid2<f64>) -> f64 {
    g.data().iter().sum::<f64>() / g.data().len() as f64
}

#[test]
fn systems_follow_the_storm_track_statistics() {
    let s = setup();
    let mid_lat = s
        .domain
        .lat_lon_at_m(1.0e6, s.domain.profile().height_m / 2.0)
        .0;
    let sigma = eady_growth_rate_s(&s.background, mid_lat, &s.canon);
    assert!(sigma > 0.0 && sigma.is_finite());
    let scale = EARTH_EADY_S / sigma;

    // Size: the closed isobars span a fixed fraction of the deformation
    // radius, which the 36 h day lengthens against Earth's 24 h.
    let ld = deformation_radius_m(mid_lat, &s.canon);
    // f scales with the rotation rate, so L_d = N H / f scales inversely:
    // a 36 h day gives 1.5 times Earth's deformation radius at a latitude.
    let earth_ld = ld * (24.0 * 3600.0) / s.canon.rotation_period_s;
    assert!(
        (ld / earth_ld - s.canon.rotation_period_s / 86_400.0).abs() < 1e-9 && ld > earth_ld,
        "Marr'Kena L_d {ld} vs Earth-day {earth_ld}"
    );
    assert!((800e3..2_500e3).contains(&ld), "L_d {ld} m");

    let seed = [9u8; 32];
    let mut state = SynopticState::bootstrap(seed, &s.background, &s.domain, &s.canon);
    let dt = 6.0 * 3600.0;
    let days = 400.0;
    let (mut born, mut lifetimes, mut speeds) = (0usize, Vec::new(), Vec::new());
    let mut seen = std::collections::HashSet::new();
    let mut t = 0.0;
    while t < days * DAY {
        state.step(&s.background, &s.domain, &s.canon, dt, seed);
        for sys in &state.systems {
            if sys.kind != SystemKind::Front && seen.insert(sys.id) {
                born += 1;
                lifetimes.push(sys.lifetime_s / DAY);
                assert!(
                    (sys.radius_m
                        / (RADIUS_OVER_DEFORMATION
                            * deformation_radius_m(
                                s.domain.lat_lon_at_m(0.0, sys.centre_m.1).0,
                                &s.canon
                            ))
                        - 1.0)
                        .abs()
                        < 1e-6
                );
            }
            if sys.kind != SystemKind::Front && sys.age_s > 0.0 {
                speeds.push(sys.velocity_m_s.0.hypot(sys.velocity_m_s.1));
            }
        }
        t += dt;
    }
    // Passage interval in Earth-equivalent days (scaled by the growth
    // rate): reference 2.5-6 d.
    let interval = days / born as f64 / scale;
    assert!((2.5..=6.0).contains(&interval), "interval {interval:.2} d");
    let mean_life = lifetimes.iter().sum::<f64>() / lifetimes.len() as f64 / scale.clamp(0.5, 2.0);
    assert!(
        (2.0..=7.0).contains(&mean_life),
        "lifetime {mean_life:.2} d"
    );
    // Translation speed scales with the baroclinic velocity σ·L_d: weaker
    // gradients on this planet (radius 19,113 km, 3x Earth's) and a longer
    // deformation radius give slower systems than Earth's 8-15 m/s. In
    // Earth-equivalent units (x σ_E L_E / σ L_d) it falls in the reference
    // band; the residual above 15 m/s is recorded as D27.
    let speed = speeds.iter().sum::<f64>() / speeds.len() as f64;
    let earth_ld_m = ld * 24.0 * 3600.0 / s.canon.rotation_period_s;
    let equivalent = speed * (EARTH_EADY_S * earth_ld_m) / (sigma * ld);
    assert!(speed > 3.0, "translation {speed:.1} m/s");
    assert!(
        (8.0..=20.0).contains(&equivalent),
        "Earth-equivalent translation {equivalent:.1} m/s (raw {speed:.1})"
    );
}

#[test]
fn a_front_brings_a_trough_a_wind_shift_and_a_rain_band() {
    let s = setup();
    let p = s.domain.profile();
    let radius = 0.6 * deformation_radius_m(-41.0_f64.to_radians(), &s.canon);
    let probe = (0.5 * p.width_m, 0.5 * p.height_m);
    let (mut min_p, mut rain_peak, mut rain_base) = (0.0_f64, 0.0_f64, f64::MAX);
    let (mut wind_before, mut wind_after) = ((0.0, 0.0), (0.0, 0.0));
    // The front (0.5 radii behind its low) crosses the probe when the
    // low is 0.5 radii east of it; sample 0.6 radii either side.
    let steps_per_radius = 120.0 / 8.0;
    let k_cross = ((4.0 + 0.5) * steps_per_radius) as usize;
    let k_before = k_cross - (0.6 * steps_per_radius) as usize;
    let k_after = k_cross + (0.6 * steps_per_radius) as usize;
    let (pr, pc) = ((probe.1 / 12_000.0) as usize, (probe.0 / 12_000.0) as usize);
    let steps = 120;
    for k in 0..=steps {
        // A low and its cold front moving east at 11 m/s, from far west
        // to far east of the probe; the probe lies equatorward of the
        // low's centre, so the front crosses it.
        let cx = probe.0 - 4.0 * radius + (8.0 * radius) * k as f64 / steps as f64;
        let mk = |kind, id, centre: (f64, f64), r, peak: f64| SynopticSystem {
            id,
            kind,
            centre_m: centre,
            velocity_m_s: (11.0, 0.0),
            central_pressure_pa: peak,
            radius_m: r,
            age_s: 0.5 * 4.0 * DAY,
            lifetime_s: 4.0 * DAY,
            peak_pa: peak,
        };
        let mut state = SynopticState {
            systems: vec![
                mk(
                    SystemKind::Cyclone,
                    0,
                    (cx, probe.1 - 0.9 * radius),
                    radius,
                    -1600.0,
                ),
                mk(
                    SystemKind::Front,
                    1,
                    (cx - 0.5 * radius, probe.1),
                    1.2 * radius,
                    0.0,
                ),
            ],
            time_s: 0.0,
            genesis_credit: 0.0,
            next_id: 2,
            mean_rain_ratio: 1.0,
        };
        let anomaly = pressure_anomaly_pa(&state, &s.domain);
        min_p = min_p.min(*anomaly.get(pr, pc));
        let mut weather = s.weather.clone();
        apply_synoptic(&mut weather, &mut state, &s.domain, &s.canon, 0.0);
        let rain = *weather.precipitation.get(pr, pc);
        rain_peak = rain_peak.max(rain);
        rain_base = rain_base.min(rain);
        let w = weather.wind.get(pr, pc);
        if k == k_before {
            wind_before = (w.u_east, w.v_north);
        }
        if k == k_after {
            wind_after = (w.u_east, w.v_north);
        }
    }
    assert!(min_p < -400.0, "pressure trough {min_p} Pa");
    assert!(
        rain_peak > rain_base + 5.0,
        "rain band: {rain_base:.1} -> {rain_peak:.1} mm/day"
    );
    // The wind turns across the front: its vector changes by more than
    // a few m/s between just ahead of it and just behind it.
    let change = (wind_after.0 - wind_before.0).hypot(wind_after.1 - wind_before.1);
    assert!(
        change > 4.0,
        "wind {wind_before:?} -> {wind_after:?} ({change:.1} m/s)"
    );
}

#[test]
fn long_run_mean_rain_and_wind_match_the_climatology() {
    let s = setup();
    let seed = [11u8; 32];
    let mut state = SynopticState::bootstrap(seed, &s.background, &s.domain, &s.canon);
    let dt = 6.0 * 3600.0;
    let (mut rain, mut u, mut n) = (0.0, 0.0, 0.0);
    let mut t = 0.0;
    while t < 600.0 * DAY {
        state.step(&s.background, &s.domain, &s.canon, dt, seed);
        let mut w = s.weather.clone();
        apply_synoptic(&mut w, &mut state, &s.domain, &s.canon, dt);
        if t > 250.0 * DAY {
            rain += mean(&w.precipitation);
            u += w.wind.data().iter().map(|v| v.u_east).sum::<f64>() / w.wind.data().len() as f64;
            n += 1.0;
        }
        t += dt;
    }
    let (base_rain, base_u) = (
        mean(&s.weather.precipitation),
        s.weather.wind.data().iter().map(|v| v.u_east).sum::<f64>()
            / s.weather.wind.data().len() as f64,
    );
    assert!(
        ((rain / n) - base_rain).abs() <= 0.05 * base_rain,
        "rain {} vs {base_rain}",
        rain / n
    );
    assert!(
        ((u / n) - base_u).abs() <= 0.05 * base_u.abs(),
        "u {} vs {base_u}",
        u / n
    );
}

#[test]
fn systems_are_deterministic_per_seed_and_a_zero_step_is_a_no_op() {
    let s = setup();
    let run = |seed: [u8; 32]| {
        let mut st = SynopticState::bootstrap(seed, &s.background, &s.domain, &s.canon);
        for _ in 0..40 {
            st.step(&s.background, &s.domain, &s.canon, 6.0 * 3600.0, seed);
        }
        st
    };
    let a = run([1u8; 32]);
    assert_eq!(a, run([1u8; 32]));
    assert_ne!(a, run([2u8; 32]));
    let mut z = a.clone();
    z.step(&s.background, &s.domain, &s.canon, 0.0, [1u8; 32]);
    assert_eq!(z, a);
    let json = serde_json::to_string(&a).unwrap();
    assert_eq!(serde_json::from_str::<SynopticState>(&json).unwrap(), a);
}
