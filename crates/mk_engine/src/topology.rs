//! Grid topology for movement, perception and birthplaces (Phase 3 Task 6).
//!
//! The human and organism runtimes were written for the planetary grid:
//! spherical cell sizes, east/west wrap, latitude and longitude from the
//! row and column. The island is flat and bounded. [`GridTopology`] holds
//! exactly the geometry those runtimes use, so both worlds run the same
//! code:
//!
//! - [`GridTopology::planetary`] reproduces the old behaviour exactly:
//!   spherical cell area, `rem_euclid` wrap in longitude, rows that stop at
//!   the poles, and the old birthplace mapping (row 0 at 90° N).
//! - [`GridTopology::regional`] uses flat cells, no wrap, and the domain's
//!   own latitudes and longitudes (row 0 at the south edge).

use mk_core::grid::GridSpec;
use mk_island::{DomainLevel, IslandDomain};

#[derive(Debug, Clone)]
enum Geometry {
    Planetary {
        spec: GridSpec,
        planet_radius_m: f64,
    },
    Regional {
        cell_m: f64,
        /// Degrees at each row's and column's centre.
        latitudes_deg: Vec<f64>,
        longitudes_deg: Vec<f64>,
    },
}

#[derive(Debug, Clone)]
pub struct GridTopology {
    pub rows: usize,
    pub cols: usize,
    pub wraps_east_west: bool,
    geometry: Geometry,
}

impl GridTopology {
    /// The planetary grid, as the runtimes always saw it.
    pub fn planetary(grid_spec: &GridSpec, planet_radius_m: f64) -> Self {
        Self {
            rows: grid_spec.nlat,
            cols: grid_spec.nlon,
            wraps_east_west: true,
            geometry: Geometry::Planetary {
                spec: grid_spec.clone(),
                planet_radius_m,
            },
        }
    }

    /// One level of the island domain: flat cells, no wrap.
    pub fn regional(domain: &IslandDomain, level: DomainLevel) -> Self {
        let (rows, cols) = (domain.rows(level), domain.cols(level));
        Self {
            rows,
            cols,
            wraps_east_west: false,
            geometry: Geometry::Regional {
                cell_m: domain.cell_size_m(level),
                latitudes_deg: (0..rows)
                    .map(|r| domain.latitude_rad_for_row(level, r).to_degrees())
                    .collect(),
                longitudes_deg: (0..cols)
                    .map(|c| domain.longitude_rad_for_col(level, c).to_degrees())
                    .collect(),
            },
        }
    }

    fn cols_i32(&self) -> i32 {
        self.cols.max(1) as i32
    }

    /// Area of a cell in `row` (m²).
    pub fn cell_area_m2(&self, row: usize) -> f64 {
        match &self.geometry {
            Geometry::Planetary {
                spec,
                planet_radius_m,
            } => spec
                .cell_area_at_row_m2(row, *planet_radius_m)
                .unwrap_or(0.0),
            Geometry::Regional { cell_m, .. } => cell_m * cell_m,
        }
    }

    /// Width of a cell in `row` (m), as the old code took it: the side of
    /// a square of the cell's area.
    pub fn cell_width_m(&self, row: usize) -> f64 {
        self.cell_area_m2(row).sqrt()
    }

    /// Height of a cell (m).
    pub fn cell_height_m(&self) -> f64 {
        match &self.geometry {
            Geometry::Planetary {
                spec,
                planet_radius_m,
            } => {
                // Rows are equal in latitude: pi / rows of arc.
                let _ = spec;
                std::f64::consts::PI * planet_radius_m / self.rows.max(1) as f64
            }
            Geometry::Regional { cell_m, .. } => *cell_m,
        }
    }

    /// Clamp a row index into the grid.
    pub fn clamp_row(&self, row: i32) -> usize {
        row.clamp(0, self.rows.saturating_sub(1) as i32) as usize
    }

    /// The column `col` resolves to: wrapped east/west on the planet,
    /// clamped to the grid on the island.
    pub fn resolve_col(&self, col: i32) -> usize {
        if self.wraps_east_west {
            col.rem_euclid(self.cols_i32()) as usize
        } else {
            col.clamp(0, self.cols.saturating_sub(1) as i32) as usize
        }
    }

    /// The cell `(drow, dcol)` away, or `None` past an edge (rows always
    /// stop at the edge; columns only on the island).
    pub fn neighbour(
        &self,
        row: usize,
        col: usize,
        drow: i32,
        dcol: i32,
    ) -> Option<(usize, usize)> {
        let r = row as i32 + drow;
        if r < 0 || r >= self.rows as i32 {
            return None;
        }
        let c = col as i32 + dcol;
        if self.wraps_east_west {
            Some((r as usize, c.rem_euclid(self.cols_i32()) as usize))
        } else if c < 0 || c >= self.cols as i32 {
            None
        } else {
            Some((r as usize, c as usize))
        }
    }

    /// The signed column step from `from` to `to`: the shortest way round
    /// on the planet, the direct difference on the island.
    pub fn col_delta(&self, from: i32, to: i32) -> i32 {
        if self.wraps_east_west {
            let n = self.cols_i32();
            let d = (to - from).rem_euclid(n);
            if d > n / 2 {
                d - n
            } else {
                d
            }
        } else {
            to - from
        }
    }

    /// Latitude and longitude (degrees) of a cell's centre. The planetary
    /// mapping is the runtimes' original one (row 0 at 90° N); the island
    /// uses the domain's (row 0 at the south edge).
    pub fn lat_lon(&self, row: i32, col: i32) -> (f64, f64) {
        match &self.geometry {
            Geometry::Planetary { .. } => {
                let nlat = self.rows.max(1) as f64;
                let nlon = self.cols.max(1) as f64;
                let row = f64::from(row).clamp(0.0, nlat - 1.0);
                let col = f64::from(col).clamp(0.0, nlon - 1.0);
                let latitude = 90.0 - (row + 0.5) / nlat * 180.0;
                let longitude = (col + 0.5) / nlon * 360.0 - 180.0;
                (latitude.clamp(-90.0, 90.0), longitude.clamp(-180.0, 180.0))
            }
            Geometry::Regional {
                latitudes_deg,
                longitudes_deg,
                ..
            } => (
                latitudes_deg[self.clamp_row(row)],
                longitudes_deg[self.resolve_col(col)],
            ),
        }
    }

    /// The cell containing a latitude and longitude (degrees). The planet
    /// always has one; the island returns `None` outside its domain.
    pub fn cell_for_lat_lon(
        &self,
        latitude_deg: f64,
        longitude_deg: f64,
    ) -> Option<(usize, usize)> {
        match &self.geometry {
            Geometry::Planetary { .. } => {
                let nlat = self.rows.max(1) as f64;
                let nlon = self.cols.max(1) as f64;
                let lat = latitude_deg.clamp(-90.0, 90.0);
                let lon = (longitude_deg + 180.0).rem_euclid(360.0) - 180.0;
                let row = (((90.0 - lat) / 180.0) * nlat)
                    .floor()
                    .clamp(0.0, nlat - 1.0);
                let col = (((lon + 180.0) / 360.0) * nlon)
                    .floor()
                    .clamp(0.0, nlon - 1.0);
                Some((row as usize, col as usize))
            }
            Geometry::Regional {
                latitudes_deg,
                longitudes_deg,
                ..
            } => {
                let nearest = |values: &[f64], v: f64| -> Option<usize> {
                    let step = if values.len() > 1 {
                        (values[values.len() - 1] - values[0]) / (values.len() - 1) as f64
                    } else {
                        return None;
                    };
                    let i = ((v - values[0]) / step + 0.5).floor();
                    (i >= 0.0 && (i as usize) < values.len()).then_some(i as usize)
                };
                Some((
                    nearest(latitudes_deg, latitude_deg)?,
                    nearest(longitudes_deg, longitude_deg)?,
                ))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mk_island::IslandProfile;

    fn planet() -> (GridSpec, GridTopology) {
        let spec = GridSpec::new(32, 64);
        let topo = GridTopology::planetary(&spec, mk_core::grid::CANON_PLANET_RADIUS_M);
        (spec, topo)
    }

    #[test]
    fn planetary_area_neighbours_and_lat_lon_equal_the_old_functions() {
        let (spec, topo) = planet();
        for row in 0..32usize {
            let old = spec
                .cell_area_at_row_m2(row, mk_core::grid::CANON_PLANET_RADIUS_M)
                .unwrap();
            assert_eq!(topo.cell_area_m2(row), old);
            assert_eq!(topo.cell_width_m(row), old.sqrt());
            for col in 0..64i32 {
                // Old: east/west `rem_euclid`, rows clamp at the poles.
                for (dr, dc) in [(0, 1), (0, -1), (1, 0), (-1, 1), (1, -1)] {
                    let want_row = row as i32 + dr;
                    let want = (0..32)
                        .contains(&want_row)
                        .then(|| (want_row as usize, (col + dc).rem_euclid(64) as usize));
                    assert_eq!(topo.neighbour(row, col as usize, dr, dc), want);
                }
                // Old birthplace mapping: row 0 at 90 N.
                let (lat, lon) = topo.lat_lon(row as i32, col);
                assert_eq!(lat, 90.0 - (row as f64 + 0.5) / 32.0 * 180.0);
                assert_eq!(lon, (f64::from(col) + 0.5) / 64.0 * 360.0 - 180.0);
                assert_eq!(topo.cell_for_lat_lon(lat, lon), Some((row, col as usize)));
            }
        }
        // The shortest way round, as the old `rem_euclid` step computed it.
        let old = |from: i32, to: i32| {
            let d = (to - from).rem_euclid(64);
            if d > 32 {
                d - 64
            } else {
                d
            }
        };
        for from in 0..64 {
            for to in 0..64 {
                assert_eq!(topo.col_delta(from, to), old(from, to));
            }
        }
    }

    #[test]
    fn regional_cells_are_flat_bounded_and_round_trip() {
        let domain = IslandDomain::from_profile(IslandProfile::test_small()).unwrap();
        let topo = GridTopology::regional(&domain, DomainLevel::Medium);
        assert!(!topo.wraps_east_west);
        let (rows, cols) = (topo.rows, topo.cols);
        // Flat area, the same in every row.
        assert_eq!(topo.cell_area_m2(0), topo.cell_area_m2(rows - 1));
        assert_eq!(
            topo.cell_area_m2(0),
            domain.cell_area_m2(DomainLevel::Medium)
        );
        // No wrap: west-edge cells have no east-edge neighbours.
        assert_eq!(topo.neighbour(5, 0, 0, -1), None);
        assert_eq!(topo.neighbour(5, cols - 1, 0, 1), None);
        assert_eq!(topo.neighbour(0, 5, -1, 0), None);
        assert_eq!(topo.neighbour(5, 0, 0, 1), Some((5, 1)));
        assert_eq!(topo.col_delta(0, cols as i32 - 1), cols as i32 - 1);
        assert_eq!(topo.resolve_col(-3), 0);
        assert_eq!(topo.resolve_col(cols as i32 + 3), cols - 1);
        // Row 0 is the southern edge: latitude rises with the row.
        assert!(topo.lat_lon(rows as i32 - 1, 0).0 > topo.lat_lon(0, 0).0);
        // lat/lon <-> cell round-trips, and outside the domain is None.
        for (r, c) in [(0, 0), (7, 91), (rows - 1, cols - 1), (rows / 2, cols / 2)] {
            let (lat, lon) = topo.lat_lon(r as i32, c as i32);
            assert_eq!(topo.cell_for_lat_lon(lat, lon), Some((r, c)));
        }
        let (lat, lon) = topo.lat_lon(0, 0);
        assert_eq!(topo.cell_for_lat_lon(lat - 5.0, lon), None);
        assert_eq!(topo.cell_for_lat_lon(lat, lon + 50.0), None);
    }
}
