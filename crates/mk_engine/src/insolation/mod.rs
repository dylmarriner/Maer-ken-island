//! INSOLATION MODULE — PHASE 2
//!
//! Purpose
//! - Deterministic insolation (solar radiation) for Marr'Kena MK-I
//! - Top-of-atmosphere (TOA) energy input to global grid
//! - Ledger-tracked reflected shortwave, TOAInsolation → SpaceRadiation (the
//!   absorbed share is booked by the climate step)
//!
//! Invariants
//! - All arithmetic uses f64 with deterministic substrate rules
//! - TOA insolation computed from orbital position and stellar luminosity
//! - Same orbit + tick → same InsolationField (deterministic)
//! - All energy must be pushed to ledger
//!
//! Authority
//! - MAERKEN_PHASE_2_PHYSICAL_WORLD.md § 3.2
//! - PLANET_CONSTANTS.md § 4.6 (relative stellar flux)
//! - COSMOS_CONSTANTS.md § 3-4 (stellar luminosity)

use mk_core::canon::CanonLocked;
use mk_core::flux::{FluxEntry, FluxKind, Ledger, Reservoir};
use mk_core::grid::Grid2;
use serde::{Deserialize, Serialize};

/// Insolation field for Marr'Kena MK-I
///
/// Type: Computed struct (no persistent state)
/// Determinism: same orbit + tick → same InsolationField
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InsolationField {
    /// Total top-of-atmosphere insolation per cell (W/m²)
    pub toa_w_m2: Grid2<f64>,
    /// Visible band insolation (W/m²) — 35% of TOA (locked Phase 2 spectral partition)
    pub band1: Grid2<f64>,
    /// Infrared band insolation (W/m²) — 55% of TOA (locked Phase 2 spectral partition)
    pub band2: Grid2<f64>,
}

/// Compute insolation field for Marr'Kena MK-I at given tick
pub fn step_insolation(
    canon: &CanonLocked,
    orbit: &crate::orbit::OrbitState,
    rotation: &crate::rotation::RotationState,
    grid_spec: &mk_core::grid::GridSpec,
    dt_seconds: f64,
    ledger: &mut Ledger,
) -> InsolationField {
    let latitudes: Vec<f64> = (0..grid_spec.nlat).map(|r| grid_spec.lat_rad(r)).collect();
    let longitudes: Vec<f64> = (0..grid_spec.nlon).map(|c| grid_spec.lon_rad(c)).collect();
    step_insolation_on(
        canon,
        orbit,
        rotation,
        grid_spec,
        &latitudes,
        &longitudes,
        &|row| {
            grid_spec
                .cell_area_at_row_m2(row, canon.planet_radius_m)
                .expect("row index is within the grid")
        },
        dt_seconds,
        ledger,
    )
}

/// [`step_insolation`] on cells at the given row `latitudes` and column
/// `longitudes` (rad), with `row_area_m2(row)` the area of a cell in a row
/// (the island's is flat). With the grid's own geometry it is exactly
/// `step_insolation`.
#[allow(clippy::too_many_arguments)]
pub fn step_insolation_on(
    canon: &CanonLocked,
    orbit: &crate::orbit::OrbitState,
    rotation: &crate::rotation::RotationState,
    grid_spec: &mk_core::grid::GridSpec,
    latitudes: &[f64],
    longitudes: &[f64],
    row_area_m2: &dyn Fn(usize) -> f64,
    dt_seconds: f64,
    ledger: &mut Ledger,
) -> InsolationField {
    let solar_constant = canon.solar_constant_w_m2;

    // Relative flux = (a/r)²
    let relative_flux = 1.0 / (orbit.r_over_a * orbit.r_over_a);
    let toa_base = solar_constant * relative_flux;

    // Seasonal declination supplied by the deterministic rotation/orbit model.
    let declination = rotation.subsolar_latitude;
    let sin_decl = declination.sin();
    let cos_decl = declination.cos();

    let mut toa_data = vec![0.0f64; grid_spec.nlat * grid_spec.nlon];
    let mut total_power_w = 0.0;

    for (ilat, &lat_rad) in latitudes.iter().enumerate().take(grid_spec.nlat) {
        // Area of a cell in this latitude row (m²).
        let cell_area = row_area_m2(ilat);
        let sin_lat = lat_rad.sin();
        let cos_lat = lat_rad.cos();

        for (ilon, &lon_rad) in longitudes.iter().enumerate().take(grid_spec.nlon) {
            // Hour angle H = lon + rotation_angle
            let hour_angle = lon_rad - rotation.subsolar_longitude;

            // cos(zenith) = sin(lat)sin(decl) + cos(lat)cos(decl)cos(H)
            let cos_zenith = sin_lat * sin_decl + cos_lat * cos_decl * hour_angle.cos();

            if cos_zenith > 0.0 {
                let flux = toa_base * cos_zenith;
                let idx = ilat * grid_spec.nlon + ilon;
                toa_data[idx] = flux;

                total_power_w += flux * cell_area;
            }
        }
    }

    let toa_grid = Grid2::from_data(grid_spec, toa_data);

    // Split into radiative bands using the locked spectral partition for Phase 2.
    let mut band1_data = vec![0.0f64; grid_spec.nlat * grid_spec.nlon];
    let mut band2_data = vec![0.0f64; grid_spec.nlat * grid_spec.nlon];

    for i in 0..toa_grid.data().len() {
        let val = toa_grid.data()[i];
        band1_data[i] = val * 0.35; // Visible band share
        band2_data[i] = val * 0.55; // Infrared band share
    }

    let band1_grid = Grid2::from_data(grid_spec, band1_data);
    let band2_grid = Grid2::from_data(grid_spec, band2_data);

    // The ledger records energy (J). Of the sunlight the planet intercepts
    // over the step, the albedo share is reflected straight back to space;
    // the absorbed rest reaches the surface and is booked by the climate
    // step against the heat it actually adds (`conservation::book_climate_step`).
    let intercepted_j = total_power_w * dt_seconds.max(0.0);
    ledger.push(FluxEntry::new(
        Reservoir::TOAInsolation,
        Reservoir::SpaceRadiation,
        intercepted_j * canon.albedo_baseline.clamp(0.0, 1.0),
        FluxKind::Energy,
    ));

    InsolationField {
        toa_w_m2: toa_grid,
        band1: band1_grid,
        band2: band2_grid,
    }
}

impl InsolationField {
    /// Get average insolation across globe
    pub fn get_average_insolation(&self) -> f64 {
        self.toa_w_m2.average()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insolation_deterministic() {
        let canon = CanonLocked::default();
        let orbit = crate::orbit::step_orbit(&canon, 0.0);
        let rotation = crate::rotation::step_rotation(0.0, 0.0, &canon);
        let grid_spec = mk_core::grid::GridSpec { nlat: 18, nlon: 36 };

        let mut ledger1 = Ledger::new();
        let mut ledger2 = Ledger::new();
        let field1 = step_insolation(&canon, &orbit, &rotation, &grid_spec, 1.0, &mut ledger1);
        let field2 = step_insolation(&canon, &orbit, &rotation, &grid_spec, 1.0, &mut ledger2);

        // Compare grids
        assert_eq!(field1.toa_w_m2.data(), field2.toa_w_m2.data());
    }

    #[test]
    fn insolation_ledger_entry() {
        let canon = CanonLocked::default();
        let orbit = crate::orbit::step_orbit(&canon, 0.0);
        let rotation = crate::rotation::step_rotation(0.0, 0.0, &canon);
        let grid_spec = mk_core::grid::GridSpec { nlat: 18, nlon: 36 };

        let mut ledger = Ledger::new();
        let field = step_insolation(&canon, &orbit, &rotation, &grid_spec, 1.0, &mut ledger);

        assert!(
            !field.toa_w_m2.data().is_empty(),
            "Insolation field must be computed"
        );
        assert_eq!(ledger.len(), 1);
    }

    #[test]
    fn insolation_nonzero() {
        let canon = CanonLocked::default();
        let orbit = crate::orbit::step_orbit(&canon, 0.0);
        let rotation = crate::rotation::step_rotation(0.0, 0.0, &canon);
        let grid_spec = mk_core::grid::GridSpec { nlat: 18, nlon: 36 };

        let mut ledger = Ledger::new();
        let field = step_insolation(&canon, &orbit, &rotation, &grid_spec, 1.0, &mut ledger);

        // At least some cells should have nonzero insolation
        let has_nonzero = field.toa_w_m2.data().iter().any(|&v| v > 0.0);
        assert!(has_nonzero, "Insolation should be nonzero");
    }
}
