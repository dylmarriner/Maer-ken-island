//! Regional geometry: flat Cartesian cells in metres with a local
//! latitude/longitude mapping. Row 0 is the southern edge and latitude
//! increases with row; col 0 is the western edge. There is no east–west
//! wrap: the domain is a region, not a planet.

use std::fmt;

use mk_core::grid::GridSpec;
use serde::{Deserialize, Serialize};

use crate::profile::{IslandProfile, IslandProfileError};

/// Largest high-detail patch, in cells: the 4 km estate at 5 m is
/// 640,000 cells; four times that keeps any patch under ~100 MB of state.
pub const MAX_LOCAL_PATCH_CELLS: usize = 2_560_000;

/// The two regional grids.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DomainLevel {
    /// Atmosphere and ocean.
    Coarse,
    /// Terrain, rivers, ecology and resources.
    Medium,
}

/// Why a domain or patch cannot be built.
#[derive(Debug)]
pub enum IslandDomainError {
    Profile(IslandProfileError),
    InvalidPatch(String),
    CellOutOfRange {
        level: DomainLevel,
        row: usize,
        col: usize,
    },
}

impl fmt::Display for IslandDomainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Profile(err) => write!(f, "{err}"),
            Self::InvalidPatch(reason) => write!(f, "invalid local patch: {reason}"),
            Self::CellOutOfRange { level, row, col } => {
                write!(f, "cell ({row}, {col}) lies outside the {level:?} grid")
            }
        }
    }
}

impl std::error::Error for IslandDomainError {}

impl From<IslandProfileError> for IslandDomainError {
    fn from(err: IslandProfileError) -> Self {
        Self::Profile(err)
    }
}

/// A high-detail window (estate, town, refinery site) on its own fine grid,
/// placed in domain metres. It allocates nothing itself.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LocalPatchSpec {
    /// South-west corner (m from the domain's south-west corner).
    pub origin_x_m: f64,
    pub origin_y_m: f64,
    pub width_m: f64,
    pub height_m: f64,
    pub cell_size_m: f64,
    pub rows: usize,
    pub cols: usize,
}

impl LocalPatchSpec {
    /// Domain metres of the centre of patch cell (`row`, `col`).
    pub fn cell_center_m(&self, row: usize, col: usize) -> (f64, f64) {
        (
            self.origin_x_m + (col as f64 + 0.5) * self.cell_size_m,
            self.origin_y_m + (row as f64 + 0.5) * self.cell_size_m,
        )
    }
}

/// A validated profile with its grid dimensions and active cell lists.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IslandDomain {
    profile: IslandProfile,
    coarse: (usize, usize),
    medium: (usize, usize),
    active_coarse: Vec<(usize, usize)>,
    active_medium: Vec<(usize, usize)>,
}

fn all_cells(rows: usize, cols: usize) -> Vec<(usize, usize)> {
    (0..rows)
        .flat_map(|r| (0..cols).map(move |c| (r, c)))
        .collect()
}

impl IslandDomain {
    /// Validates the profile, then sizes the grids. Every cell starts
    /// active; the land mask narrows the medium grid later.
    pub fn from_profile(profile: IslandProfile) -> Result<Self, IslandDomainError> {
        profile.validate()?;
        let dims = |cell: f64| {
            (
                (profile.height_m / cell).round() as usize,
                (profile.width_m / cell).round() as usize,
            )
        };
        let coarse = dims(profile.coarse_cell_m);
        let medium = dims(profile.medium_cell_m);
        Ok(Self {
            active_coarse: all_cells(coarse.0, coarse.1),
            active_medium: all_cells(medium.0, medium.1),
            profile,
            coarse,
            medium,
        })
    }

    pub fn profile(&self) -> &IslandProfile {
        &self.profile
    }

    fn dims(&self, level: DomainLevel) -> (usize, usize) {
        match level {
            DomainLevel::Coarse => self.coarse,
            DomainLevel::Medium => self.medium,
        }
    }

    pub fn rows(&self, level: DomainLevel) -> usize {
        self.dims(level).0
    }

    pub fn cols(&self, level: DomainLevel) -> usize {
        self.dims(level).1
    }

    /// Storage shape for a `Grid2` on this level (`nlat` = rows).
    pub fn storage_spec(&self, level: DomainLevel) -> GridSpec {
        let (rows, cols) = self.dims(level);
        GridSpec::new(rows, cols)
    }

    pub fn cell_size_m(&self, level: DomainLevel) -> f64 {
        match level {
            DomainLevel::Coarse => self.profile.coarse_cell_m,
            DomainLevel::Medium => self.profile.medium_cell_m,
        }
    }

    /// Every cell has the same flat area.
    pub fn cell_area_m2(&self, level: DomainLevel) -> f64 {
        self.cell_size_m(level).powi(2)
    }

    /// Metres east and north of the south-west corner of a cell's centre.
    pub fn cell_center_m(&self, level: DomainLevel, row: usize, col: usize) -> (f64, f64) {
        let size = self.cell_size_m(level);
        ((col as f64 + 0.5) * size, (row as f64 + 0.5) * size)
    }

    /// The cell holding a point, or `None` outside the domain.
    pub fn cell_containing_m(
        &self,
        level: DomainLevel,
        x_m: f64,
        y_m: f64,
    ) -> Option<(usize, usize)> {
        let size = self.cell_size_m(level);
        if !(x_m.is_finite() && y_m.is_finite()) || x_m < 0.0 || y_m < 0.0 {
            return None;
        }
        let (row, col) = ((y_m / size).floor() as usize, (x_m / size).floor() as usize);
        let (rows, cols) = self.dims(level);
        (row < rows && col < cols).then_some((row, col))
    }

    /// Latitude and longitude (radians) of a point, linear in metres about
    /// the reference point at the domain centre.
    pub fn lat_lon_at_m(&self, x_m: f64, y_m: f64) -> (f64, f64) {
        let p = &self.profile;
        let lat0 = p.reference_latitude_deg.to_radians();
        let lat = lat0 + (y_m - 0.5 * p.height_m) / p.planet_radius_m;
        let lon = p.reference_longitude_deg.to_radians()
            + (x_m - 0.5 * p.width_m) / (p.planet_radius_m * lat0.cos());
        (lat, lon)
    }

    pub fn latitude_rad_for_row(&self, level: DomainLevel, row: usize) -> f64 {
        let (_, y) = self.cell_center_m(level, row, 0);
        self.lat_lon_at_m(0.5 * self.profile.width_m, y).0
    }

    pub fn longitude_rad_for_col(&self, level: DomainLevel, col: usize) -> f64 {
        let (x, _) = self.cell_center_m(level, 0, col);
        self.lat_lon_at_m(x, 0.5 * self.profile.height_m).1
    }

    /// Whether any part of a cell lies within the ocean buffer of an edge.
    pub fn is_edge_buffer_cell(&self, level: DomainLevel, row: usize, col: usize) -> bool {
        let size = self.cell_size_m(level);
        let b = self.profile.minimum_ocean_buffer_m;
        let (x0, y0) = (col as f64 * size, row as f64 * size);
        x0 < b
            || y0 < b
            || x0 + size > self.profile.width_m - b
            || y0 + size > self.profile.height_m - b
    }

    /// Cells processes iterate on this level: all coarse cells; on the
    /// medium grid, land plus the coastal band once the land mask is known.
    pub fn active_cells(&self, level: DomainLevel) -> &[(usize, usize)] {
        match level {
            DomainLevel::Coarse => &self.active_coarse,
            DomainLevel::Medium => &self.active_medium,
        }
    }

    /// Replaces a level's active cells (sorted row-major, deduplicated).
    pub fn set_active_cells(
        &mut self,
        level: DomainLevel,
        mut cells: Vec<(usize, usize)>,
    ) -> Result<(), IslandDomainError> {
        let (rows, cols) = self.dims(level);
        if let Some(&(row, col)) = cells.iter().find(|&&(r, c)| r >= rows || c >= cols) {
            return Err(IslandDomainError::CellOutOfRange { level, row, col });
        }
        cells.sort_unstable();
        cells.dedup();
        match level {
            DomainLevel::Coarse => self.active_coarse = cells,
            DomainLevel::Medium => self.active_medium = cells,
        }
        Ok(())
    }

    /// A square high-detail patch of side `extent_m` centred on a point. It
    /// must lie wholly inside the domain, be a whole number of cells, and
    /// stay under [`MAX_LOCAL_PATCH_CELLS`].
    pub fn local_patch(
        &self,
        center_x_m: f64,
        center_y_m: f64,
        extent_m: f64,
        cell_size_m: f64,
    ) -> Result<LocalPatchSpec, IslandDomainError> {
        let bad = |reason: String| Err(IslandDomainError::InvalidPatch(reason));
        if ![center_x_m, center_y_m, extent_m, cell_size_m]
            .iter()
            .all(|v| v.is_finite())
        {
            return bad("values must be finite".into());
        }
        if extent_m <= 0.0 || cell_size_m <= 0.0 {
            return bad("extent and cell size must be positive".into());
        }
        let n = extent_m / cell_size_m;
        if (n - n.round()).abs() > 1e-9 || n.round() < 1.0 {
            return bad(format!("{cell_size_m} m cells do not divide {extent_m} m"));
        }
        let side = n.round() as usize;
        if side.saturating_mul(side) > MAX_LOCAL_PATCH_CELLS {
            return bad(format!(
                "{side} × {side} cells exceeds {MAX_LOCAL_PATCH_CELLS}"
            ));
        }
        let (x0, y0) = (center_x_m - 0.5 * extent_m, center_y_m - 0.5 * extent_m);
        if x0 < 0.0
            || y0 < 0.0
            || x0 + extent_m > self.profile.width_m
            || y0 + extent_m > self.profile.height_m
        {
            return bad("the patch extends outside the domain".into());
        }
        Ok(LocalPatchSpec {
            origin_x_m: x0,
            origin_y_m: y0,
            width_m: extent_m,
            height_m: extent_m,
            cell_size_m,
            rows: side,
            cols: side,
        })
    }
}
