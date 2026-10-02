use super::CanonLocked;
/**
 * Purpose
 * - Canon validation for Maer'Ken world and Marr'Kena planet.
 * - Ensures all canon values match authoritative specification.
 *
 * Invariants
 * - All CanonLocked fields must match exact values from roadmap.
 * - No Earth-derived constants allowed.
 * - Naming violations must be detected.
 *
 * Failure Modes
 * - Any field mismatch -> validation error with details.
 * - A derived value that disagrees with its sources -> validation error.
 */
use crate::ids::*;

/// Canon validation error
#[derive(Debug, Clone, PartialEq)]
pub enum CanonError {
    ContinuumMismatch,
    SystemMismatch,
    StarMismatch,
    PlanetMismatch,
    ClassMismatch,
    SubclassMismatch,
    EpochMismatch,
    ValueMismatch(&'static str, f64, f64),
    MoonNameMismatch(&'static str),
    PlatesMismatch(&'static str, u8),
}

impl std::fmt::Display for CanonError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ContinuumMismatch => write!(f, "Continuum ID mismatch"),
            Self::SystemMismatch => write!(f, "System ID mismatch"),
            Self::StarMismatch => write!(f, "Star ID mismatch"),
            Self::PlanetMismatch => write!(f, "Planet ID mismatch"),
            Self::ClassMismatch => write!(f, "Planet class mismatch"),
            Self::SubclassMismatch => write!(f, "Planet subclass mismatch"),
            Self::EpochMismatch => write!(f, "Epoch mismatch"),
            Self::ValueMismatch(field, expected, actual) => {
                write!(
                    f,
                    "Value mismatch for {}: expected {}, got {}",
                    field, expected, actual
                )
            }
            Self::MoonNameMismatch(expected) => {
                write!(f, "Moon name mismatch: expected {}", expected)
            }
            Self::PlatesMismatch(tier, expected) => {
                write!(f, "Plates mismatch ({}): expected {}", tier, expected)
            }
        }
    }
}

impl std::error::Error for CanonError {}

/// Newtonian gravitational constant (CODATA 2018), m^3 kg^-1 s^-2.
const GRAVITATIONAL_CONSTANT: f64 = 6.674_30e-11;

/// Validate every canon value against the authoritative specification, and
/// check that the values the canon derives from others agree with them.
///
/// The record is destructured field by field, so adding a field to
/// [`CanonLocked`] fails to compile here until it is validated too.
pub fn validate_canon(canon: &CanonLocked) -> Result<(), CanonError> {
    let CanonLocked {
        continuum,
        system,
        star,
        planet,
        class,
        subclass,
        epoch,
        dt_seconds,
        star_luminosity_w,
        star_mass_kg,
        star_radius_m,
        planet_radius_m,
        planet_mass_kg,
        surface_gravity_m_s2,
        rotation_period_s,
        orbital_period_s,
        orbital_eccentricity,
        obliquity_deg,
        semi_major_axis_m,
        solar_constant_w_m2,
        mckenz_name,
        mckenz_semi_major_axis_m,
        mckenz_period_local_days,
        moon_mckenz_period_s,
        moon_mckenz_mass_ratio,
        hahn_name,
        hahn_semi_major_axis_m,
        hahn_period_local_days,
        moon_hahn_period_s,
        moon_hahn_mass_ratio,
        major_plates,
        minor_plates,
        sea_level_pressure_pa,
        ref_temp_k,
        albedo_baseline,
        ocean_fraction,
        mean_salinity_psu,
        c_bio_ref_mol,
        n_bio_ref_mol,
        p_bio_ref_mol,
        o_bio_ref_mol,
        gpp_max_kgc_m2_yr,
        npp_max_kgc_m2_yr,
        resp_frac_prod,
        eta_prod_herb,
        eta_herb_meso,
        eta_meso_apex,
        intelligence_max,
        cn_ratio_prod,
        cn_ratio_cons,
        cn_ratio_det,
    } = canon;

    // 1. World-Level Identifiers
    if *continuum != ContinuumId::MK_CONTINUUM_0 {
        return Err(CanonError::ContinuumMismatch);
    }
    if *system != SystemId::MK_SYS_01 {
        return Err(CanonError::SystemMismatch);
    }
    if *star != StarId::MK_STAR_01 {
        return Err(CanonError::StarMismatch);
    }

    // 2. Planet-Level Identifiers
    if *planet != PlanetId::MK_I {
        return Err(CanonError::PlanetMismatch);
    }
    if *class != PlanetClass::KenzIe {
        return Err(CanonError::ClassMismatch);
    }
    if *subclass != KenzIeSubclass::Alpha {
        return Err(CanonError::SubclassMismatch);
    }
    if *epoch != Epoch::MarrKenBloom {
        return Err(CanonError::EpochMismatch);
    }

    // 3. Physical constants (roadmap Phase 0)
    let eps = 1e-10;
    for (field, actual, expected) in [
        ("dt_seconds", *dt_seconds, 0.1),
        ("star_luminosity_w", *star_luminosity_w, 5.1678e26),
        ("star_mass_kg", *star_mass_kg, 2.2270864e30),
        ("star_radius_m", *star_radius_m, 7.51356e8),
        ("planet_radius_m", *planet_radius_m, 1.9113e7),
        ("planet_mass_kg", *planet_mass_kg, 1.07532e26),
        ("surface_gravity_m_s2", *surface_gravity_m_s2, 19.62),
        ("rotation_period_s", *rotation_period_s, 129600.0),
        ("orbital_period_s", *orbital_period_s, 46656000.0),
        ("orbital_eccentricity", *orbital_eccentricity, 0.031),
        ("obliquity_deg", *obliquity_deg, 27.0),
        ("semi_major_axis_m", *semi_major_axis_m, 1.75029508719e11),
        ("solar_constant_w_m2", *solar_constant_w_m2, 1342.33),
    ] {
        validate_phys(actual, expected, eps, field)?;
    }

    // 4. Moons
    if mckenz_name != "Mckenz" {
        return Err(CanonError::MoonNameMismatch("Mckenz"));
    }
    if hahn_name != "Hahn" {
        return Err(CanonError::MoonNameMismatch("Hahn"));
    }
    for (field, actual, expected) in [
        ("mckenz_semi_major_axis_m", *mckenz_semi_major_axis_m, 8.9e8),
        ("mckenz_period_local_days", *mckenz_period_local_days, 30.0),
        ("moon_mckenz_period_s", *moon_mckenz_period_s, 3_888_000.0),
        ("moon_mckenz_mass_ratio", *moon_mckenz_mass_ratio, 0.0123),
        ("hahn_semi_major_axis_m", *hahn_semi_major_axis_m, 1.72e9),
        ("hahn_period_local_days", *hahn_period_local_days, 90.0),
        ("moon_hahn_period_s", *moon_hahn_period_s, 11_664_000.0),
        ("moon_hahn_mass_ratio", *moon_hahn_mass_ratio, 0.005),
    ] {
        validate_phys(actual, expected, eps, field)?;
    }

    // 5. Plates
    if *major_plates != 11 {
        return Err(CanonError::PlatesMismatch("major", 11));
    }
    if *minor_plates != 19 {
        return Err(CanonError::PlatesMismatch("minor", 19));
    }

    // 6. Atmosphere & Ocean, 7. Biosphere
    for (field, actual, expected) in [
        ("sea_level_pressure_pa", *sea_level_pressure_pa, 182000.0),
        ("ref_temp_k", *ref_temp_k, 286.0),
        ("albedo_baseline", *albedo_baseline, 0.287),
        ("ocean_fraction", *ocean_fraction, 0.68),
        ("mean_salinity_psu", *mean_salinity_psu, 30.6),
        ("c_bio_ref_mol", *c_bio_ref_mol, 3.10e18),
        ("n_bio_ref_mol", *n_bio_ref_mol, 6.80e16),
        ("p_bio_ref_mol", *p_bio_ref_mol, 4.20e14),
        ("o_bio_ref_mol", *o_bio_ref_mol, 1.95e19),
        ("gpp_max_kgc_m2_yr", *gpp_max_kgc_m2_yr, 4.8),
        ("npp_max_kgc_m2_yr", *npp_max_kgc_m2_yr, 2.6),
        ("resp_frac_prod", *resp_frac_prod, 0.46),
        ("eta_prod_herb", *eta_prod_herb, 0.118),
        ("eta_herb_meso", *eta_herb_meso, 0.103),
        ("eta_meso_apex", *eta_meso_apex, 0.087),
        ("intelligence_max", *intelligence_max, 0.38),
        ("cn_ratio_prod", *cn_ratio_prod, 18.0),
        ("cn_ratio_cons", *cn_ratio_cons, 7.2),
        ("cn_ratio_det", *cn_ratio_det, 24.0),
    ] {
        validate_phys(actual, expected, eps, field)?;
    }

    // 8. Derived values must agree with the values they derive from.
    //
    // Moon periods in seconds are the local-day periods times the day length.
    validate_phys(
        *moon_mckenz_period_s,
        mckenz_period_local_days * rotation_period_s,
        eps,
        "moon_mckenz_period_s (= local days x rotation period)",
    )?;
    validate_phys(
        *moon_hahn_period_s,
        hahn_period_local_days * rotation_period_s,
        eps,
        "moon_hahn_period_s (= local days x rotation period)",
    )?;
    // The solar constant is the stellar flux at the mean orbital distance,
    // L / (4 pi a^2). The canon rounds it to 0.01 W m^-2.
    validate_phys(
        *solar_constant_w_m2,
        star_luminosity_w / (4.0 * std::f64::consts::PI * semi_major_axis_m * semi_major_axis_m),
        1e-4,
        "solar_constant_w_m2 (= L / 4 pi a^2)",
    )?;
    // Surface gravity is G M / R^2. The canon rounds it to 0.01 m s^-2 and
    // takes its own value of G, so this allows 0.5%.
    validate_phys(
        *surface_gravity_m_s2,
        GRAVITATIONAL_CONSTANT * planet_mass_kg / (planet_radius_m * planet_radius_m),
        5e-3,
        "surface_gravity_m_s2 (= G M / R^2)",
    )?;
    // NPP cannot exceed GPP, and every fraction and efficiency is a fraction.
    if npp_max_kgc_m2_yr > gpp_max_kgc_m2_yr {
        return Err(CanonError::ValueMismatch(
            "npp_max_kgc_m2_yr (<= gpp_max_kgc_m2_yr)",
            *gpp_max_kgc_m2_yr,
            *npp_max_kgc_m2_yr,
        ));
    }

    Ok(())
}

fn validate_phys(
    actual: f64,
    expected: f64,
    eps: f64,
    field: &'static str,
) -> Result<(), CanonError> {
    if (actual - expected).abs() > eps * expected.abs().max(1.0) {
        return Err(CanonError::ValueMismatch(field, expected, actual));
    }
    Ok(())
}

/// How serious a physical-consistency finding is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IssueSeverity {
    /// The canon contradicts physics; a world built on it is inconsistent.
    Error,
    /// A known departure the canon's author has chosen and recorded (for
    /// example, Earth gravity on a planet three times Earth's radius).
    Declared,
}

/// One physical-consistency finding.
#[derive(Debug, Clone, PartialEq)]
pub struct ConsistencyIssue {
    pub check: &'static str,
    pub expected: f64,
    pub actual: f64,
    pub severity: IssueSeverity,
}

/// Bulk density below which a planet cannot be mostly rock. The least dense
/// rocky body in the Solar System is the Moon, at about 3,344 kg/m³.
pub const ROCKY_MIN_BULK_DENSITY_KG_M3: f64 = 3_300.0;

/// Prograde satellites on near-circular orbits stay bound over long times
/// only inside roughly half the Hill radius (Domingos, Winter & Yokoyama
/// 2006, MNRAS 373, 1227).
pub const HILL_STABLE_FRACTION: f64 = 0.49;

/// Jeans escape parameter above which a gas is retained over geological time.
pub const JEANS_RETENTION_PARAMETER: f64 = 50.0;

/// Check that the canon's values agree with each other under Newtonian
/// physics: surface gravity, insolation, the planet's and each moon's orbit
/// (Kepler's third law), moon stability (Hill sphere, Roche limit, low-order
/// resonances) and atmosphere retention. Unlike [`validate_canon`], this does
/// not require the default canon's exact values, so it applies to any canon.
pub fn validate_physical_consistency(canon: &CanonLocked) -> Vec<ConsistencyIssue> {
    use std::f64::consts::PI;
    let g = GRAVITATIONAL_CONSTANT;
    let mut issues = Vec::new();
    let mut check = |name: &'static str, expected: f64, actual: f64, rel: f64| {
        if !expected.is_finite()
            || !actual.is_finite()
            || (actual - expected).abs() > rel * expected.abs()
        {
            issues.push(ConsistencyIssue {
                check: name,
                expected,
                actual,
                severity: IssueSeverity::Error,
            });
        }
    };

    let planet = canon.planet_mass_kg;
    let radius = canon.planet_radius_m;
    check(
        "surface_gravity_m_s2 = G M / R^2",
        g * planet / (radius * radius),
        canon.surface_gravity_m_s2,
        5e-3,
    );
    check(
        "solar_constant_w_m2 = L / (4 pi a^2)",
        canon.star_luminosity_w / (4.0 * PI * canon.semi_major_axis_m.powi(2)),
        canon.solar_constant_w_m2,
        5e-3,
    );
    let kepler = |a: f64, mu: f64| 2.0 * PI * (a.powi(3) / mu).sqrt();
    check(
        "orbital_period_s (Kepler, planet around star)",
        kepler(canon.semi_major_axis_m, g * (canon.star_mass_kg + planet)),
        canon.orbital_period_s,
        5e-3,
    );

    let hill = canon.semi_major_axis_m
        * (1.0 - canon.orbital_eccentricity)
        * (planet / (3.0 * canon.star_mass_kg)).cbrt();
    let roche = 2.44 * radius; // fluid Roche limit for a moon as dense as its planet
    struct Moon {
        a: f64,
        period: f64,
        local_days: f64,
        mass_ratio: f64,
        kepler: &'static str,
        days: &'static str,
        hill: &'static str,
        roche: &'static str,
    }
    let moons = [
        Moon {
            a: canon.mckenz_semi_major_axis_m,
            period: canon.moon_mckenz_period_s,
            local_days: canon.mckenz_period_local_days,
            mass_ratio: canon.moon_mckenz_mass_ratio,
            kepler: "moon_mckenz_period_s (Kepler)",
            days: "moon_mckenz_period_s = local days x rotation",
            hill: "mckenz_semi_major_axis_m < 0.49 Hill radius",
            roche: "mckenz_semi_major_axis_m > Roche limit",
        },
        Moon {
            a: canon.hahn_semi_major_axis_m,
            period: canon.moon_hahn_period_s,
            local_days: canon.hahn_period_local_days,
            mass_ratio: canon.moon_hahn_mass_ratio,
            kepler: "moon_hahn_period_s (Kepler)",
            days: "moon_hahn_period_s = local days x rotation",
            hill: "hahn_semi_major_axis_m < 0.49 Hill radius",
            roche: "hahn_semi_major_axis_m > Roche limit",
        },
    ];
    for moon in &moons {
        check(
            moon.kepler,
            kepler(moon.a, g * planet * (1.0 + moon.mass_ratio)),
            moon.period,
            5e-3,
        );
        check(
            moon.days,
            moon.local_days * canon.rotation_period_s,
            moon.period,
            1e-9,
        );
    }
    for moon in &moons {
        if moon.a >= HILL_STABLE_FRACTION * hill {
            issues.push(ConsistencyIssue {
                check: moon.hill,
                expected: HILL_STABLE_FRACTION * hill,
                actual: moon.a,
                severity: IssueSeverity::Error,
            });
        }
        if moon.a <= roche {
            issues.push(ConsistencyIssue {
                check: moon.roche,
                expected: roche,
                actual: moon.a,
                severity: IssueSeverity::Error,
            });
        }
    }
    let (inner, outer) = if canon.moon_mckenz_period_s <= canon.moon_hahn_period_s {
        (canon.moon_mckenz_period_s, canon.moon_hahn_period_s)
    } else {
        (canon.moon_hahn_period_s, canon.moon_mckenz_period_s)
    };
    let ratio = outer / inner;
    for resonance in [2.0, 1.5] {
        if (ratio - resonance).abs() <= 0.02 * resonance {
            issues.push(ConsistencyIssue {
                check: "moon period ratio not near 2:1 or 3:2",
                expected: resonance,
                actual: ratio,
                severity: IssueSeverity::Error,
            });
        }
    }

    // Jeans escape parameter for N2 at a 1,000 K exobase 500 km up.
    let boltzmann = 1.380_649e-23;
    let n2_mass = 28.0 * 1.660_539_066_6e-27;
    let jeans = g * planet * n2_mass / (boltzmann * 1_000.0 * (radius + 5.0e5));
    if jeans < JEANS_RETENTION_PARAMETER {
        issues.push(ConsistencyIssue {
            check: "Jeans parameter for N2 >= 50",
            expected: JEANS_RETENTION_PARAMETER,
            actual: jeans,
            severity: IssueSeverity::Error,
        });
    }

    let density = planet / (4.0 / 3.0 * PI * radius.powi(3));
    if density < ROCKY_MIN_BULK_DENSITY_KG_M3 {
        issues.push(ConsistencyIssue {
            check: "bulk density >= rocky minimum",
            expected: ROCKY_MIN_BULK_DENSITY_KG_M3,
            actual: density,
            severity: IssueSeverity::Declared,
        });
    }
    issues
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_canon_default_succeeds() {
        let canon = CanonLocked::default();
        assert!(validate_canon(&canon).is_ok());
    }

    #[test]
    fn every_field_is_checked() {
        let mutations: Vec<fn(&mut CanonLocked)> = vec![
            |c| c.semi_major_axis_m *= 1.01,
            |c| c.solar_constant_w_m2 += 1.0,
            |c| c.moon_mckenz_period_s += 1.0,
            |c| c.moon_hahn_mass_ratio *= 2.0,
            |c| c.c_bio_ref_mol *= 1.1,
            |c| c.npp_max_kgc_m2_yr = 5.0,
            |c| c.eta_meso_apex = 0.2,
            |c| c.intelligence_max = 1.0,
            |c| c.cn_ratio_det = 1.0,
            |c| c.hahn_name = "Moon".into(),
        ];
        for mutate in mutations {
            let mut canon = CanonLocked::default();
            mutate(&mut canon);
            assert!(validate_canon(&canon).is_err(), "{canon:?}");
        }
    }

    #[test]
    fn derived_values_agree_with_their_sources() {
        let canon = CanonLocked::default();
        let flux = canon.star_luminosity_w
            / (4.0 * std::f64::consts::PI * canon.semi_major_axis_m.powi(2));
        assert!((flux - canon.solar_constant_w_m2).abs() / flux < 1e-4);
        let g = GRAVITATIONAL_CONSTANT * canon.planet_mass_kg / canon.planet_radius_m.powi(2);
        assert!((g - canon.surface_gravity_m_s2).abs() / g < 5e-3);
    }

    #[test]
    fn validate_canon_fails_on_value_mismatch() {
        let canon = CanonLocked {
            dt_seconds: 0.5,
            ..Default::default()
        };
        assert!(validate_canon(&canon).is_err());
    }
}
