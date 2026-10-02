/**
 * Purpose
 * - Canon definitions for Maer'Ken world and Marr'Kena planet.
 * - Contains immutable canon values and derived computations.
 *
 * Invariants
 * - CanonLocked values never change after initialization.
 * - All numeric fields use f64 for simulation precision and determinism.
 * - Canon digest is deterministic and computed once.
 *
 * Failure Modes
 * - Canon validation failure → simulation halt.
 * - Non-deterministic digest → system integrity failure.
 *
 * Debug Notes
 * - validate_canon() provides detailed field mismatch information.
 * - Canon digest should be identical across all runs.
 */
use crate::ids::*;
use serde::{Deserialize, Serialize};

pub mod validator;

/// Immutable canon values for Maer'Ken world and Marr'Kena planet
///
/// This struct contains ALL canon parameters that define the world.
/// Values are fixed and must never change after initialization.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CanonLocked {
    // ========== WORLD-LEVEL IDENTIFIERS ==========
    pub continuum: ContinuumId, // "MK-CONTINUUM-0" (Maer'Ken)
    pub system: SystemId,       // "MK-SYS-01"
    pub star: StarId,           // "MK-STAR-01" (Aurelion Marr)

    // ========== PLANET-LEVEL IDENTIFIERS ==========
    pub planet: PlanetId,         // "MK-I" (Marr'Kena)
    pub class: PlanetClass,       // PlanetClass::KenzIe
    pub subclass: KenzIeSubclass, // KenzIeSubclass::Alpha
    pub epoch: Epoch,             // Epoch::MarrKenBloom

    // ========== ORBITAL MECHANICS (PLANET-SPECIFIC) ==========
    pub dt_seconds: f64,           // 0.1 s
    pub star_luminosity_w: f64,    // 5.1678e26 W
    pub star_mass_kg: f64,         // 2.2270864e30 kg
    pub star_radius_m: f64,        // 7.51356e8 m
    pub planet_radius_m: f64,      // 1.9113e7 m
    pub planet_mass_kg: f64,       // 1.07532e26 kg
    pub surface_gravity_m_s2: f64, // 19.62 m s^-2
    pub rotation_period_s: f64,    // 129600 s
    pub orbital_period_s: f64,     // 46656000 s (360 local days)
    pub orbital_eccentricity: f64, // 0.031
    pub obliquity_deg: f64,        // 27.0 deg
    pub semi_major_axis_m: f64,    // 1.75029508719e11 m
    pub solar_constant_w_m2: f64,  // 1342.33 W m^-2 (derived)

    // ========== MOONS (PLANET-SPECIFIC) ==========
    // Mckenz (Primary Moon)
    pub mckenz_name: String,
    pub mckenz_semi_major_axis_m: f64, // 8.9e8 m
    pub mckenz_period_local_days: f64, // 30.0
    pub moon_mckenz_period_s: f64,     // 3,888,000 s
    pub moon_mckenz_mass_ratio: f64,   // 0.0123 (Earth-Moon like)

    // Hahn (Secondary Moon)
    pub hahn_name: String,
    pub hahn_semi_major_axis_m: f64, // 1.72e9 m
    pub hahn_period_local_days: f64, // 90.0
    pub moon_hahn_period_s: f64,     // 11,664,000 s
    pub moon_hahn_mass_ratio: f64,   // 0.005

    // ========== PLATES & GEOLOGY (PLANET-SPECIFIC) ==========
    pub major_plates: u8, // 11
    pub minor_plates: u8, // 19

    // ========== ATMOSPHERE & OCEAN (PHASE 2) ==========
    pub sea_level_pressure_pa: f64, // 182000 Pa
    pub ref_temp_k: f64,            // 286.0 K
    pub albedo_baseline: f64,       // 0.287
    pub ocean_fraction: f64,        // 0.68 (68%)
    pub mean_salinity_psu: f64,     // 30.6 PSU

    // ========== BIOSPHERE (PHASE 3) ==========
    pub c_bio_ref_mol: f64,     // 3.10e18 mol
    pub n_bio_ref_mol: f64,     // 6.80e16 mol
    pub p_bio_ref_mol: f64,     // 4.20e14 mol
    pub o_bio_ref_mol: f64,     // 1.95e19 mol
    pub gpp_max_kgc_m2_yr: f64, // 4.8 kgC m^-2 yr^-1
    pub npp_max_kgc_m2_yr: f64, // 2.6 kgC m^-2 yr^-1
    pub resp_frac_prod: f64,    // 0.46
    pub eta_prod_herb: f64,     // 0.118
    pub eta_herb_meso: f64,     // 0.103
    pub eta_meso_apex: f64,     // 0.087
    pub intelligence_max: f64,  // 0.38
    pub cn_ratio_prod: f64,     // 18.0
    pub cn_ratio_cons: f64,     // 7.2
    pub cn_ratio_det: f64,      // 24.0
}

/// Why a canon file could not be loaded.
#[derive(Debug)]
pub enum CanonLoadError {
    Io(std::io::Error),
    Parse(serde_json::Error),
}

impl std::fmt::Display for CanonLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(err) => write!(f, "could not read canon file: {err}"),
            Self::Parse(err) => write!(f, "could not parse canon file: {err}"),
        }
    }
}

impl std::error::Error for CanonLoadError {}

impl CanonLocked {
    /// Load a canon from a JSON file in the same format `CanonLocked`
    /// serializes to. Upstream code validates canon identity with
    /// [`validator::validate_canon`], which accepts only the default canon;
    /// an alternative canon (such as the island's) is checked with
    /// [`validator::validate_physical_consistency`] instead.
    pub fn load(path: &std::path::Path) -> Result<Self, CanonLoadError> {
        let bytes = std::fs::read(path).map_err(CanonLoadError::Io)?;
        serde_json::from_slice(&bytes).map_err(CanonLoadError::Parse)
    }

    /// Whole seconds one `WorldState::step_world` tick advances when the
    /// caller does not choose its own step: the canon `dt_seconds` rounded
    /// up, never zero (the canon value is sub-second, and a zero-second step
    /// advances nothing).
    pub fn step_seconds(&self) -> u64 {
        if self.dt_seconds.is_finite() && self.dt_seconds > 0.0 {
            (self.dt_seconds.ceil() as u64).max(1)
        } else {
            1
        }
    }
}

impl Default for CanonLocked {
    fn default() -> Self {
        Self {
            // World-Level Identifiers
            continuum: ContinuumId::MK_CONTINUUM_0,
            system: SystemId::MK_SYS_01,
            star: StarId::MK_STAR_01,

            // Planet-Level Identifiers
            planet: PlanetId::MK_I,
            class: PlanetClass::KenzIe,
            subclass: KenzIeSubclass::Alpha,
            epoch: Epoch::MarrKenBloom,

            // Orbital Mechanics
            dt_seconds: 0.1,
            star_luminosity_w: 5.1678e26,
            star_mass_kg: 2.2270864e30,
            star_radius_m: 7.51356e8,
            planet_radius_m: 1.9113e7,
            planet_mass_kg: 1.07532e26,
            surface_gravity_m_s2: 19.62,
            rotation_period_s: 129600.0,
            orbital_period_s: 46656000.0,
            orbital_eccentricity: 0.031,
            obliquity_deg: 27.0,
            semi_major_axis_m: 1.75029508719e11,
            solar_constant_w_m2: 1342.33,

            // Moons
            mckenz_name: "Mckenz".to_string(),
            mckenz_semi_major_axis_m: 8.9e8,
            mckenz_period_local_days: 30.0,
            moon_mckenz_period_s: 30.0 * 129600.0,
            moon_mckenz_mass_ratio: 0.0123,

            hahn_name: "Hahn".to_string(),
            hahn_semi_major_axis_m: 1.72e9,
            hahn_period_local_days: 90.0,
            moon_hahn_period_s: 90.0 * 129600.0,
            moon_hahn_mass_ratio: 0.005,

            // Plates & Geology
            major_plates: 11,
            minor_plates: 19,

            // Atmosphere & Ocean
            sea_level_pressure_pa: 182000.0,
            ref_temp_k: 286.0,
            albedo_baseline: 0.287,
            ocean_fraction: 0.68,
            mean_salinity_psu: 30.6,

            // Biosphere
            c_bio_ref_mol: 3.10e18,
            n_bio_ref_mol: 6.80e16,
            p_bio_ref_mol: 4.20e14,
            o_bio_ref_mol: 1.95e19,
            gpp_max_kgc_m2_yr: 4.8,
            npp_max_kgc_m2_yr: 2.6,
            resp_frac_prod: 0.46,
            eta_prod_herb: 0.118,
            eta_herb_meso: 0.103,
            eta_meso_apex: 0.087,
            intelligence_max: 0.38,
            cn_ratio_prod: 18.0,
            cn_ratio_cons: 7.2,
            cn_ratio_det: 24.0,
        }
    }
}

/// Derived canon values computed once from CanonLocked
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CanonDerived {
    /// Rotation angular velocity (rad/s)
    pub omega_rad_s: f64,
    /// Gravitational parameter GM (m^3/s^2)
    pub mu_planet_m3_s2: f64,
    /// BLAKE3 digest of CanonLocked serialization
    pub canon_digest: [u8; 32],
}

impl CanonDerived {
    /// Compute derived values from canon
    pub fn from(canon: &CanonLocked) -> Self {
        // Compute omega = 2π / day_seconds
        let tau = 2.0 * std::f64::consts::PI;
        let rotation_period = canon.rotation_period_s;
        let omega_rad_s = if rotation_period > 0.0 {
            tau / rotation_period
        } else {
            0.0
        };

        // Compute gravitational parameter μ = GM ≈ g * r^2
        let mu_planet_m3_s2 =
            canon.surface_gravity_m_s2 * canon.planet_radius_m * canon.planet_radius_m;

        // Compute BLAKE3 digest of serialized CanonLocked
        let canon_digest = Self::compute_canon_digest(canon);

        Self {
            omega_rad_s,
            mu_planet_m3_s2,
            canon_digest,
        }
    }

    /// Compute deterministic BLAKE3 digest of CanonLocked
    fn compute_canon_digest(canon: &CanonLocked) -> [u8; 32] {
        // serde_json provides deterministic serialization for this use case
        let bytes = serde_json::to_vec(canon).unwrap_or_default();

        let mut hasher = blake3::Hasher::new();
        hasher.update(&bytes);
        let hash = hasher.finalize();
        *hash.as_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canon_locked_instantiates() {
        let canon = CanonLocked::default();
        assert_eq!(canon.continuum.as_str(), "MK-CONTINUUM-0");
        assert_eq!(canon.planet.as_str(), "MK-I");
        assert_eq!(canon.rotation_period_s, 129600.0);
        assert_eq!(canon.orbital_period_s, 46656000.0);
        assert_eq!(canon.obliquity_deg, 27.0);
        assert_eq!(canon.surface_gravity_m_s2, 19.62);
        assert_eq!(canon.major_plates, 11);
        assert_eq!(canon.minor_plates, 19);
    }

    #[test]
    fn canon_derived_computes() {
        let canon = CanonLocked::default();
        let derived = CanonDerived::from(&canon);
        assert!(derived.omega_rad_s > 0.0);
        assert!(derived.mu_planet_m3_s2 > 0.0);
        assert_ne!(derived.canon_digest, [0u8; 32]);
    }

    #[test]
    fn canon_digest_deterministic() {
        let canon = CanonLocked::default();
        let digest1 = CanonDerived::from(&canon).canon_digest;
        let digest2 = CanonDerived::from(&canon).canon_digest;
        assert_eq!(digest1, digest2);
    }

    #[test]
    fn validate_canon_succeeds() {
        let canon = CanonLocked::default();
        assert!(crate::canon::validator::validate_canon(&canon).is_ok());
    }
}
