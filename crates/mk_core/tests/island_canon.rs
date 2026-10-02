//! The island's Earth-like Marr'Kena canon (`fixtures/island/canon.json`):
//! Marr'Kena's radius, day, tilt, star and two moons, with Earth gravity,
//! atmosphere and sunlight, and every orbit recomputed so the physics agrees.

use mk_core::canon::validator::{validate_canon, validate_physical_consistency, IssueSeverity};
use mk_core::canon::{CanonDerived, CanonLocked};
use std::path::PathBuf;

fn island_canon_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/island/canon.json")
}

fn island_canon() -> CanonLocked {
    CanonLocked::load(&island_canon_path()).expect("island canon loads")
}

fn rel(a: f64, b: f64) -> f64 {
    (a - b).abs() / b.abs()
}

#[test]
fn island_canon_is_consistent_except_the_declared_density() {
    let issues = validate_physical_consistency(&island_canon());
    let errors: Vec<_> = issues
        .iter()
        .filter(|i| i.severity == IssueSeverity::Error)
        .collect();
    assert!(errors.is_empty(), "{errors:#?}");
    let declared: Vec<_> = issues
        .iter()
        .filter(|i| i.severity == IssueSeverity::Declared)
        .collect();
    assert_eq!(declared.len(), 1, "{declared:#?}");
    assert_eq!(declared[0].check, "bulk density >= rocky minimum");
}

#[test]
fn island_canon_keeps_marrkena_and_sets_earth_conditions() {
    let island = island_canon();
    let upstream = CanonLocked::default();
    // Kept from Marr'Kena.
    assert_eq!(island.planet, upstream.planet);
    assert_eq!(island.planet_radius_m, upstream.planet_radius_m);
    assert_eq!(island.rotation_period_s, upstream.rotation_period_s);
    assert_eq!(island.obliquity_deg, upstream.obliquity_deg);
    assert_eq!(island.star_luminosity_w, upstream.star_luminosity_w);
    assert_eq!(island.star_mass_kg, upstream.star_mass_kg);
    assert_eq!(island.mckenz_name, upstream.mckenz_name);
    assert_eq!(island.hahn_name, upstream.hahn_name);
    assert_eq!(island.moon_mckenz_period_s, upstream.moon_mckenz_period_s);
    // Earth conditions.
    assert_eq!(island.surface_gravity_m_s2, 9.80665);
    assert_eq!(island.sea_level_pressure_pa, 101_325.0);
    assert!(rel(island.solar_constant_w_m2, 1361.0) < 1e-3);
    assert!(rel(island.planet_mass_kg / 5.9722e24, 8.987) < 1e-3);
}

#[test]
fn island_year_and_moons_follow_from_the_physics() {
    let island = island_canon();
    let local_days = island.orbital_period_s / island.rotation_period_s;
    assert!((local_days - 288.19).abs() < 0.01, "{local_days}");
    assert!(rel(island.semi_major_axis_m / 1.495_978_707e11, 1.162) < 1e-3);
    // Hahn was shortened from 90 to 48 local days so it stays inside half
    // the Hill radius; McKenz keeps its 30-day period.
    assert_eq!(island.hahn_period_local_days, 48.0);
    assert_eq!(island.mckenz_period_local_days, 30.0);
}

#[test]
fn upstream_canon_reports_its_real_inconsistencies() {
    let checks: Vec<_> = validate_physical_consistency(&CanonLocked::default())
        .into_iter()
        .filter(|i| i.severity == IssueSeverity::Error)
        .map(|i| i.check)
        .collect();
    assert!(
        checks.contains(&"orbital_period_s (Kepler, planet around star)"),
        "{checks:?}"
    );
    assert!(
        checks.contains(&"moon_mckenz_period_s (Kepler)"),
        "{checks:?}"
    );
    assert!(
        checks.contains(&"moon_hahn_period_s (Kepler)"),
        "{checks:?}"
    );
    // Upstream's gravity does agree with its mass and radius.
    assert!(
        !checks.contains(&"surface_gravity_m_s2 = G M / R^2"),
        "{checks:?}"
    );
}

#[test]
fn upstream_identity_validator_still_rejects_the_island_canon() {
    // `validate_canon` checks identity against the default canon; the island
    // uses `validate_physical_consistency` instead.
    assert!(validate_canon(&CanonLocked::default()).is_ok());
    assert!(validate_canon(&island_canon()).is_err());
}

#[test]
fn island_canon_loads_deterministically_with_a_stable_digest() {
    let a = CanonDerived::from(&island_canon());
    let b = CanonDerived::from(&island_canon());
    assert_eq!(a.canon_digest, b.canon_digest);
    assert_ne!(
        a.canon_digest,
        CanonDerived::from(&CanonLocked::default()).canon_digest
    );
    assert!(
        rel(
            a.mu_planet_m3_s2,
            6.674_30e-11 * island_canon().planet_mass_kg
        ) < 5e-3
    );
}

#[test]
fn a_missing_canon_file_is_an_error() {
    assert!(CanonLocked::load(&PathBuf::from("/nonexistent/canon.json")).is_err());
}
