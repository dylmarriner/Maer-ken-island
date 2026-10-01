//! Major rivers from flow accumulation over the terrain.
//!
//! Each land cell drains to its steepest-descent neighbour (D8 on the
//! lat-lon grid, with longitude wrapping). Visiting cells from highest to
//! lowest, every cell passes its own runoff plus everything that reached it
//! downstream, so a cell's discharge is the runoff of its whole upstream
//! catchment. Ocean cells (elevation below 0) are sinks; a land pit keeps
//! its water (an endorheic basin).
//!
//! A grid cell here is hundreds of kilometres across, so its own runoff can
//! already be a large volume. A river cell is therefore the trunk of a
//! continental basin: its discharge reaches [`RIVER_DISCHARGE_THRESHOLD_M3_S`]
//! *and* is at least [`TRUNK_CATCHMENT_CELLS`] times what the cell itself
//! generates, i.e. most of it arrives from upstream.

use super::BiomeType;
use crate::grid::{Grid2, GridSpec};

/// Mean discharge (m³/s) at which a cell counts as a river cell. Roughly
/// forty of Earth's rivers exceed it (the Mississippi carries ~17,000,
/// the Rhine ~2,300).
pub const RIVER_DISCHARGE_THRESHOLD_M3_S: f64 = 5_000.0;

/// How many cells' worth of runoff a river cell must carry relative to its
/// own: a catchment of this many cells (several million km² on the default
/// grid, Amazon- or Congo-class).
pub const TRUNK_CATCHMENT_CELLS: f64 = 5.0;

/// Flow through every cell, m³/s.
#[derive(Debug, Clone)]
pub struct RiverFlow {
    /// Total discharge: local runoff plus everything from upstream.
    pub discharge_m3_s: Grid2<f64>,
    /// The cell's own runoff as a volume flux.
    pub local_m3_s: Grid2<f64>,
}

impl RiverFlow {
    /// Whether `(row, col)` is the trunk of a major river.
    pub fn is_river(&self, row: usize, col: usize) -> bool {
        let total = self
            .discharge_m3_s
            .get_safe(row, col)
            .copied()
            .unwrap_or(0.0);
        let local = self.local_m3_s.get_safe(row, col).copied().unwrap_or(0.0);
        total >= RIVER_DISCHARGE_THRESHOLD_M3_S && total >= TRUNK_CATCHMENT_CELLS * local
    }
}

const SECONDS_PER_DAY: f64 = 86_400.0;

/// Flow through every land cell, from local runoff in mm/day. Ocean cells
/// report 0.
pub fn river_flow(
    elevation_m: &Grid2<f64>,
    runoff_mm_day: &Grid2<f64>,
    radius_m: f64,
) -> RiverFlow {
    let (nlat, nlon) = (elevation_m.nlat(), elevation_m.nlon());
    let spec = GridSpec::new(nlat, nlon);
    let mut discharge = Grid2::new(&spec, 0.0);
    if nlat == 0 || nlon == 0 {
        return RiverFlow {
            local_m3_s: discharge.clone(),
            discharge_m3_s: discharge,
        };
    }

    let dlat_m = std::f64::consts::PI / nlat as f64 * radius_m;
    let dlon_rad = std::f64::consts::TAU / nlon as f64;
    let is_land = |row: usize, col: usize| *elevation_m.get(row, col) >= 0.0;

    // Local runoff as a volume flux: mm/day over the cell's true area.
    for row in 0..nlat {
        let area = spec.cell_area_at_row_m2(row, radius_m).unwrap_or(0.0);
        for col in 0..nlon {
            if is_land(row, col) {
                let runoff = runoff_mm_day.get(row, col).max(0.0);
                *discharge.get_mut(row, col) = runoff / 1000.0 * area / SECONDS_PER_DAY;
            }
        }
    }

    let local = discharge.clone();

    // Steepest-descent receiver of each cell, if it drains anywhere.
    let receiver = |row: usize, col: usize| -> Option<(usize, usize)> {
        let here = *elevation_m.get(row, col);
        let lat = spec.lat_rad(row);
        let mut best: Option<((usize, usize), f64)> = None;
        for d_row in [-1i64, 0, 1] {
            for d_col in [-1i64, 0, 1] {
                if d_row == 0 && d_col == 0 {
                    continue;
                }
                let n_row = row as i64 + d_row;
                if n_row < 0 || n_row >= nlat as i64 {
                    continue;
                }
                let n_row = n_row as usize;
                let n_col = (col as i64 + d_col).rem_euclid(nlon as i64) as usize;
                let east_m = d_col as f64 * dlon_rad * radius_m * lat.cos().abs();
                let north_m = d_row as f64 * dlat_m;
                let distance = east_m.hypot(north_m).max(1.0);
                let slope = (here - *elevation_m.get(n_row, n_col)) / distance;
                if slope > 0.0 && best.map_or(true, |(_, s)| slope > s) {
                    best = Some(((n_row, n_col), slope));
                }
            }
        }
        best.map(|(cell, _)| cell)
    };

    // Highest first, so every upstream contribution has arrived before a
    // cell passes its total on.
    let mut order: Vec<(usize, usize)> = (0..nlat)
        .flat_map(|row| (0..nlon).map(move |col| (row, col)))
        .filter(|&(row, col)| is_land(row, col))
        .collect();
    order.sort_by(|&a, &b| {
        elevation_m
            .get(b.0, b.1)
            .partial_cmp(elevation_m.get(a.0, a.1))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    for (row, col) in order {
        if let Some((n_row, n_col)) = receiver(row, col) {
            if is_land(n_row, n_col) {
                let passed = *discharge.get(row, col);
                *discharge.get_mut(n_row, n_col) += passed;
            }
        }
    }
    RiverFlow {
        discharge_m3_s: discharge,
        local_m3_s: local,
    }
}

/// Mark the trunk cells of major rivers ([`RiverFlow::is_river`]) as
/// [`BiomeType::River`]. Ice sheets keep their biome: water there moves as
/// ice, not as a river.
pub fn mark_river_biomes(biomes: &mut Grid2<BiomeType>, flow: &RiverFlow) {
    for (row, col, biome) in biomes.indexed_iter_mut() {
        if flow.is_river(row, col) && !matches!(*biome, BiomeType::IceSheet) {
            *biome = BiomeType::River;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RADIUS_M: f64 = 6.371e6;
    const NLAT: usize = 18;
    const NLON: usize = 12;

    /// A continent between two ocean columns (0 and `NLON - 1`, which are
    /// neighbours across the longitude wrap), rising inland.
    fn continent() -> Grid2<f64> {
        let spec = GridSpec::new(NLAT, NLON);
        let mut elevation = Grid2::new(&spec, 0.0);
        for row in 0..NLAT {
            for col in 0..NLON {
                let height = if col == 0 || col + 1 == NLON {
                    -100.0
                } else {
                    let inland = col.min(NLON - 1 - col) as f64;
                    100.0 * inland + (row as f64 - NLAT as f64 / 2.0).abs() * 10.0
                };
                *elevation.get_mut(row, col) = height;
            }
        }
        elevation
    }

    fn wet() -> Grid2<f64> {
        Grid2::new(&GridSpec::new(NLAT, NLON), 1.0)
    }

    #[test]
    fn discharge_accumulates_downstream_and_conserves_runoff() {
        let spec = GridSpec::new(NLAT, NLON);
        let flow = river_flow(&continent(), &wet(), RADIUS_M);
        let discharge = &flow.discharge_m3_s;

        let mid = NLAT / 2;
        // Toward the coast carries more than inland.
        assert!(discharge.get(mid, 1) > discharge.get(mid, 4));
        // All of it reaches the sea through the two coastal columns, whose
        // discharge equals the total local runoff over the land.
        let land_cols = (NLON - 2) as f64;
        let total_local: f64 = (0..NLAT)
            .map(|row| {
                spec.cell_area_at_row_m2(row, RADIUS_M).unwrap() * land_cols
                    / 1000.0
                    / SECONDS_PER_DAY
            })
            .sum();
        let coast: f64 = (0..NLAT)
            .map(|row| *discharge.get(row, 1) + *discharge.get(row, NLON - 2))
            .sum();
        assert!(
            (coast / total_local - 1.0).abs() < 1e-9,
            "{coast} vs {total_local}"
        );
        assert_eq!(*discharge.get(mid, 0), 0.0, "ocean is a sink");
    }

    #[test]
    fn only_major_flows_become_river_cells() {
        let spec = GridSpec::new(NLAT, NLON);
        let flow = river_flow(&continent(), &wet(), RADIUS_M);
        let discharge = &flow.discharge_m3_s;
        let mut biomes = Grid2::new(&spec, BiomeType::Grassland);
        *biomes.get_mut(0, 1) = BiomeType::IceSheet;
        mark_river_biomes(&mut biomes, &flow);

        for (row, col, biome) in biomes.indexed_iter() {
            if *biome == BiomeType::River {
                assert!(*discharge.get(row, col) >= RIVER_DISCHARGE_THRESHOLD_M3_S);
                assert!(
                    *discharge.get(row, col)
                        >= TRUNK_CATCHMENT_CELLS * *flow.local_m3_s.get(row, col)
                );
            } else if (row, col) != (0, 1) {
                assert!(!flow.is_river(row, col));
            }
        }
        assert!(
            biomes.data().contains(&BiomeType::River),
            "the trunk rivers are found"
        );
        assert_eq!(*biomes.get(0, 1), BiomeType::IceSheet);
    }

    #[test]
    fn a_headwater_cell_is_not_a_river_however_wet() {
        // One isolated high cell draining straight to the sea carries only
        // its own runoff, so it never counts as a trunk river.
        let spec = GridSpec::new(NLAT, NLON);
        let mut elevation = Grid2::new(&spec, -100.0);
        *elevation.get_mut(9, 5) = 500.0;
        let soaked = Grid2::new(&spec, 1_000.0);
        let flow = river_flow(&elevation, &soaked, RADIUS_M);
        assert!(*flow.discharge_m3_s.get(9, 5) >= RIVER_DISCHARGE_THRESHOLD_M3_S);
        assert!(!flow.is_river(9, 5));
    }

    #[test]
    fn a_dry_world_has_no_rivers() {
        let spec = GridSpec::new(NLAT, NLON);
        let flow = river_flow(&continent(), &Grid2::new(&spec, 0.0), RADIUS_M);
        assert!(flow.discharge_m3_s.data().iter().all(|&q| q == 0.0));
    }
}
