//! PLANET MODULE — PHASE 2
//!
//! Purpose
//! - Deterministic planetary fields for Marr'Kena MK-I
//! - Rotation angular velocity (ω)
//! - Gravitational parameter (GM)
//! - Coriolis parameter per grid cell
//!
//! Invariants
//! - All arithmetic uses f64 with deterministic substrate rules
//! - Fields are deterministically computed from canon
//! - Same canon + grid_spec → same PlanetFields
//!
//! Authority
//! - MAERKEN_PHASE_2_PHYSICAL_WORLD.md § 3.3
//! - PLANET_CONSTANTS.md (planetary body constants)

use mk_core::canon::{CanonDerived, CanonLocked};
use mk_core::grid::{Grid2, GridSpec};
use serde::{Deserialize, Serialize};

/// Planet fields for Marr'Kena MK-I
///
/// Type: Computed struct (no persistent state)
/// Determinism: same canon + grid_spec → same PlanetFields
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlanetFields {
    /// Rotation angular velocity ω (radians/second)
    pub omega_rad_s: f64,
    /// Gravitational parameter μ = GM (m³/s²)
    pub mu_planet_m3_s2: f64,
    /// Coriolis parameter f = 2ω sin(lat) per cell
    pub coriolis: Grid2<f64>,
}

/// Compute planetary fields for Marr'Kena MK-I
pub fn step_planet(
    canon: &CanonLocked,
    _derived: &CanonDerived,
    grid_spec: &GridSpec,
) -> PlanetFields {
    // Angular velocity: ω = 2π / rotation_period_s (rad/s)
    let omega_rad_s = if canon.rotation_period_s > 0.0 {
        std::f64::consts::TAU / canon.rotation_period_s
    } else {
        0.0
    };

    // Gravitational parameter GM
    let mu_planet_m3_s2 = 6.67430e-11 * canon.planet_mass_kg;

    // Compute Coriolis parameter grid
    // f = 2ω sin(lat)
    let mut coriolis_data = vec![0.0f64; grid_spec.nlat * grid_spec.nlon];

    for ilat in 0..grid_spec.nlat {
        let lat_rad = grid_spec.lat_rad(ilat);
        let f = 2.0 * omega_rad_s * lat_rad.sin();

        // Set all longitude cells at this latitude
        for ilon in 0..grid_spec.nlon {
            let idx = ilat * grid_spec.nlon + ilon;
            coriolis_data[idx] = f;
        }
    }

    let coriolis = Grid2::from_data(grid_spec, coriolis_data);

    PlanetFields {
        omega_rad_s,
        mu_planet_m3_s2,
        coriolis,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn planet_fields_deterministic() {
        let canon = CanonLocked::default();
        let derived = CanonDerived::from(&canon);
        let grid_spec = GridSpec { nlat: 18, nlon: 36 };

        let fields1 = step_planet(&canon, &derived, &grid_spec);
        let fields2 = step_planet(&canon, &derived, &grid_spec);

        assert_eq!(fields1.omega_rad_s, fields2.omega_rad_s);
        assert_eq!(fields1.mu_planet_m3_s2, fields2.mu_planet_m3_s2);
    }

    #[test]
    fn coriolis_zero_at_equator() {
        let canon = CanonLocked::default();
        let derived = CanonDerived::from(&canon);
        let grid_spec = GridSpec { nlat: 18, nlon: 36 };

        let fields = step_planet(&canon, &derived, &grid_spec);

        // Equator is around nlat/2
        let f_equator = *fields
            .coriolis
            .get_safe(grid_spec.nlat / 2, 0)
            .unwrap_or(&0.0);

        // Coriolis at equator should be close to zero
        assert!(
            f_equator.abs() < 1e-4,
            "Coriolis at equator should be near zero, got {}",
            f_equator
        );
    }

    #[test]
    fn coriolis_nonzero_at_poles() {
        let canon = CanonLocked::default();
        let derived = CanonDerived::from(&canon);
        let grid_spec = GridSpec { nlat: 18, nlon: 36 };

        let fields = step_planet(&canon, &derived, &grid_spec);

        // North pole is at highest latitude index
        let f_north = *fields
            .coriolis
            .get_safe(grid_spec.nlat - 1, 0)
            .unwrap_or(&0.0);

        // Coriolis at pole should be larger than at equator
        let f_equator = *fields
            .coriolis
            .get_safe(grid_spec.nlat / 2, 0)
            .unwrap_or(&0.0);
        assert!(
            f_north.abs() > f_equator.abs(),
            "Coriolis should be larger at pole"
        );
    }
}
