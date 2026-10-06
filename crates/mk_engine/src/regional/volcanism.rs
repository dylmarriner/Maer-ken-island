//! Volcanism on the island domain (Phase 1 Task 4): the upstream volcanism
//! step on flat regional cells with non-wrapping neighbours.

use mk_core::canon::CanonLocked;
use mk_core::rng::RngRegistry;
use mk_core::time::Tick;
use mk_island::{DomainLevel, IslandDomain};

use super::tectonics::regional_neighbours;
use crate::tectonics::TectonicsState;
use crate::volcanism::{step_volcanism_on, VolcanismGeometry, VolcanismState};

/// Flat square cells of the regional coarse grid.
pub struct RegionalVolcanismGeometry {
    pub cell_km: f64,
    pub rows: usize,
    pub cols: usize,
}

impl RegionalVolcanismGeometry {
    pub fn coarse(domain: &IslandDomain) -> Self {
        let level = DomainLevel::Coarse;
        Self {
            cell_km: domain.cell_size_m(level) / 1000.0,
            rows: domain.rows(level),
            cols: domain.cols(level),
        }
    }
}

impl VolcanismGeometry for RegionalVolcanismGeometry {
    fn cell_height_km(&self, _row: usize) -> f64 {
        self.cell_km
    }

    fn cell_width_km(&self, _row: usize) -> f64 {
        self.cell_km
    }

    fn neighbours(&self, row: usize, col: usize) -> Vec<(usize, usize, i64, i64)> {
        regional_neighbours(row, col, self.rows, self.cols)
            .map(|(r, c)| (r, c, r as i64 - row as i64, c as i64 - col as i64))
            .collect()
    }
}

/// Volcanism of the region's coarse cells from its tectonic state.
pub fn step_regional_volcanism(
    canon: &CanonLocked,
    tick: Tick,
    domain: &IslandDomain,
    tectonics: &TectonicsState,
    previous_degassed_fraction: f64,
    dt_seconds: f64,
    rng: &RngRegistry,
) -> VolcanismState {
    let spec = domain.storage_spec(DomainLevel::Coarse);
    step_volcanism_on(
        canon,
        tick,
        &tectonics.plates,
        &tectonics.heat_flow,
        previous_degassed_fraction,
        dt_seconds,
        rng,
        &spec,
        &RegionalVolcanismGeometry::coarse(domain),
    )
}
