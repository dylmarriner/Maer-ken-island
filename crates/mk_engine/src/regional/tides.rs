//! Regional tides (Phase 2 Task 4).
//!
//! The tide phase is analytic (the moons' sub-lunar longitudes from
//! `tides::sub_lunar_longitude`, the same function the boundary sampler
//! uses for `AstronomyForcing`), so it needs no state. Upstream's
//! degree-2 potential and equilibrium height are evaluated at the domain's
//! latitudes and longitudes (`tides::step_tides_on`), and the dissipation
//! ledger entry is the domain's share of the planet's tidal heating: the
//! ocean's fraction of the planet's surface.
//!
//! The field lives on the **coarse** grid, not the medium one: it varies
//! over thousands of kilometres, so 12 km cells resolve it, and a medium
//! grid would spend ~46 MB per snapshot on near-constant fields.
//! Consumers that need medium cells resample with
//! `levels::resample_coarse_to_medium`.

use std::sync::Arc;

use mk_core::canon::CanonLocked;
use mk_core::flux::Ledger;
use mk_core::grid::Grid2;
use mk_island::{DomainLevel, IslandDomain};

use crate::tides::{step_tides_on, TidalState};

/// The tide at `sim_time_seconds` over the domain's coarse cells.
/// `elevation_m` (coarse) gives the ocean area the heating share is
/// taken over: sea-level-or-below cells.
pub fn step_regional_tides(
    canon: &Arc<CanonLocked>,
    sim_time_seconds: f64,
    dt_seconds: u64,
    domain: &IslandDomain,
    elevation_m: &Grid2<f64>,
    ledger: &mut Ledger,
) -> TidalState {
    let coarse = DomainLevel::Coarse;
    let spec = domain.storage_spec(coarse);
    let latitudes: Vec<f64> = (0..spec.nlat)
        .map(|r| domain.latitude_rad_for_row(coarse, r))
        .collect();
    let longitudes: Vec<f64> = (0..spec.nlon)
        .map(|c| domain.longitude_rad_for_col(coarse, c))
        .collect();
    let ocean_cells = elevation_m.data().iter().filter(|&&h| h <= 0.0).count() as f64;
    let ocean_area_m2 = ocean_cells * domain.cell_area_m2(coarse);
    let planet_area_m2 = 4.0 * std::f64::consts::PI * canon.planet_radius_m.powi(2);
    let share = if planet_area_m2 > 0.0 {
        ocean_area_m2 / planet_area_m2
    } else {
        0.0
    };
    step_tides_on(
        canon,
        sim_time_seconds,
        dt_seconds,
        &spec,
        &latitudes,
        &longitudes,
        share,
        ledger,
    )
}
