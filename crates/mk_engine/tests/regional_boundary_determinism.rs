//! Phase 1 Task 2: regional boundary forcing is deterministic, finite and
//! uses the canon's own sun and moons.

use std::path::PathBuf;

use mk_core::canon::CanonLocked;
use mk_engine::regional::boundary::sample_regional_boundaries;
use mk_island::{DomainLevel, Edge, IslandDomain, IslandProfile};

fn canon() -> CanonLocked {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/island/canon.json");
    CanonLocked::load(&path).expect("island canon")
}

fn domain() -> IslandDomain {
    IslandDomain::from_profile(IslandProfile::test_small()).unwrap()
}

#[test]
fn identical_inputs_give_identical_samples() {
    let (c, d) = (canon(), domain());
    let a = sample_regional_boundaries([1; 32], &c, &d, 1.0e6);
    let b = sample_regional_boundaries([1; 32], &c, &d, 1.0e6);
    assert_eq!(a, b);
    assert_eq!(
        serde_json::to_vec(&a).unwrap(),
        serde_json::to_vec(&b).unwrap()
    );
}

#[test]
fn seed_and_time_change_the_samples() {
    let (c, d) = (canon(), domain());
    let base = sample_regional_boundaries([1; 32], &c, &d, 1.0e6);
    let other_seed = sample_regional_boundaries([2; 32], &c, &d, 1.0e6);
    assert_ne!(base.ocean, other_seed.ocean);
    assert_ne!(base.tectonic, other_seed.tectonic);
    assert_eq!(
        base.astronomy, other_seed.astronomy,
        "the sky does not depend on the seed"
    );

    let later = sample_regional_boundaries([1; 32], &c, &d, 1.0e6 + 5.0 * c.rotation_period_s);
    assert_ne!(base.atmosphere, later.atmosphere);
    assert_ne!(base.astronomy, later.astronomy);
    assert_eq!(
        base.tectonic, later.tectonic,
        "far-field plates are fixed for a seed"
    );
}

#[test]
fn every_edge_is_forced_with_finite_values() {
    let (c, d) = (canon(), domain());
    let s = sample_regional_boundaries([9; 32], &c, &d, 3.3e7);
    assert!(s.ocean.provisional && s.atmosphere.provisional);
    assert_eq!(s.ocean.edges.len(), 4);
    assert_eq!(s.atmosphere.edges.len(), 4);
    for o in &s.ocean.edges {
        let n = match o.edge {
            Edge::South | Edge::North => d.cols(DomainLevel::Coarse),
            Edge::West | Edge::East => d.rows(DomainLevel::Coarse),
        };
        for v in [
            &o.sea_surface_temperature_k,
            &o.salinity_psu,
            &o.inflow_m_s,
            &o.sea_level_anomaly_m,
        ] {
            assert_eq!(v.len(), n);
            assert!(v.iter().all(|x| x.is_finite()));
        }
        assert!(o
            .sea_surface_temperature_k
            .iter()
            .all(|t| (271.0..305.0).contains(t)));
    }
    for a in &s.atmosphere.edges {
        for v in [
            &a.air_temperature_k,
            &a.wind_u_m_s,
            &a.wind_v_m_s,
            &a.specific_humidity_kg_kg,
            &a.surface_pressure_pa,
        ] {
            assert!(!v.is_empty() && v.iter().all(|x| x.is_finite()));
        }
        assert!(a
            .specific_humidity_kg_kg
            .iter()
            .all(|q| *q > 0.0 && *q < 0.04));
    }
    assert_eq!(s.tectonic.far_field.len(), 4);
    for p in &s.tectonic.far_field {
        let speed = p.velocity_east_m_yr.hypot(p.velocity_north_m_yr);
        assert!((0.02..=0.09).contains(&speed), "{speed}");
    }
}

#[test]
fn astronomy_matches_the_orbit_rotation_and_tide_modules() {
    let c = canon();
    let t = 12_345_678.0;
    let s = sample_regional_boundaries([0; 32], &c, &domain(), t);
    let orbit = mk_engine::orbit::step_orbit(&c, t);
    let rotation = mk_engine::rotation::step_rotation(t, orbit.mean_anomaly, &c);
    assert_eq!(
        s.astronomy.solar_declination_rad,
        rotation.subsolar_latitude
    );
    assert_eq!(
        s.astronomy.sub_solar_longitude_rad,
        rotation.subsolar_longitude
    );
    assert_eq!(
        s.astronomy.moon_sub_longitudes_rad[0],
        mk_engine::tides::sub_lunar_longitude(t, c.moon_mckenz_period_s, c.rotation_period_s)
    );
    assert!((s.astronomy.season_phase * std::f64::consts::TAU - orbit.mean_anomaly).abs() < 1e-12);
    assert!(s.astronomy.solar_declination_rad.abs() <= c.obliquity_deg.to_radians() + 1e-12);
}

#[test]
fn the_state_has_no_wall_clock_fields() {
    let s = sample_regional_boundaries([0; 32], &canon(), &domain(), 0.0);
    let json = serde_json::to_string(&s).unwrap().to_lowercase();
    for word in ["wall", "now", "timestamp", "date", "utc"] {
        assert!(!json.contains(&format!("\"{word}")), "{word}");
    }
    assert_eq!(s.sim_time_seconds, 0.0);
}
