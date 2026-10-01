use serde::{Deserialize, Serialize};
/**
 * Purpose
 * - Grid container for lat-lon data structures in Maer'Ken simulation.
 * - Provides deterministic 2D array with bounds checking.
 *
 * Invariants
 * - Uses row-major order (lat outer loop, lon inner loop).
 * - Bounds checking enforced.
 * - Deterministic initialization.
 *
 * Failure Modes
 * - Bounds violation → panic.
 * - Incorrect indexing → data corruption.
 * - Non-deterministic iteration → replay failure.
 *
 * Debug Notes
 * - Grid2::get() and Grid2::set() enforce bounds.
 * - Iteration order is always lat outer, lon inner.
 * - All operations are deterministic.
 */
use std::fmt;

/// Canon planet radius (Marr'Kena), in metres. It must equal
/// `CanonLocked::default().planet_radius_m`, which a test checks. It is a
/// constant here so that per-cell loops don't have to build a canon record.
pub const CANON_PLANET_RADIUS_M: f64 = 1.9113e7;

/// Grid configuration specification
///
/// Defines the dimensions of a lat-lon grid.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GridSpec {
    pub nlat: usize,
    pub nlon: usize,
}

impl GridSpec {
    /// Create new grid specification
    ///
    /// # Arguments
    ///
    /// * `nlat` - Number of latitude points
    /// * `nlon` - Number of longitude points
    ///
    /// # Returns
    ///
    /// New GridSpec
    pub fn new(nlat: usize, nlon: usize) -> Self {
        Self { nlat, nlon }
    }

    /// Get total number of grid points
    ///
    /// # Returns
    ///
    /// Total points = nlat * nlon
    pub fn total_points(&self) -> usize {
        self.nlat * self.nlon
    }

    /// Check if indices are within bounds
    ///
    /// # Arguments
    ///
    /// * `ilat` - Latitude index
    /// * `ilon` - Longitude index
    ///
    /// # Returns
    ///
    /// True if indices are valid, false otherwise
    pub fn is_valid_index(&self, ilat: usize, ilon: usize) -> bool {
        ilat < self.nlat && ilon < self.nlon
    }

    /// Get number of latitude points
    pub fn nlat(&self) -> usize {
        self.nlat
    }

    /// Get number of longitude points
    pub fn nlon(&self) -> usize {
        self.nlon
    }

    /// Get latitude in radians for a given row index
    ///
    /// # Arguments
    ///
    /// * `row` - Row index (latitude index)
    ///
    /// # Returns
    ///
    /// Latitude in radians (-π/2 to π/2)
    pub fn lat_rad(&self, row: usize) -> f64 {
        if row >= self.nlat {
            return 0.0;
        }
        let frac = (row as f64 + 0.5) / (self.nlat as f64);
        (frac - 0.5) * std::f64::consts::PI
    }

    /// Get longitude in radians for a given column index
    ///
    /// # Arguments
    ///
    /// * `col` - Column index (longitude index)
    ///
    /// # Returns
    ///
    /// Longitude in radians (0 to 2π)
    pub fn lon_rad(&self, col: usize) -> f64 {
        if col >= self.nlon {
            return 0.0;
        }
        let frac = (col as f64 + 0.5) / (self.nlon as f64);
        frac * 2.0 * std::f64::consts::PI
    }

    /// Resolve a geographic latitude/longitude (in degrees) to the grid cell
    /// whose centre it falls in.
    ///
    /// This is the exact inverse of [`GridSpec::lat_rad`] / [`GridSpec::lon_rad`]:
    /// those place cell centres at `((row + 0.5) / nlat - 0.5) * PI` and
    /// `((col + 0.5) / nlon) * 2 * PI`, so this maps a coordinate back to the
    /// row/col band containing it.
    ///
    /// # Arguments
    ///
    /// * `lat_deg` - Latitude in degrees, clamped to `[-90, 90]`
    /// * `lon_deg` - Longitude in degrees; wrapped into `[0, 360)`, so both the
    ///   `-180..180` and `0..360` conventions are accepted
    ///
    /// # Returns
    ///
    /// `(row, col)`, always a valid index for this spec.
    pub fn cell_for_lat_lon(&self, lat_deg: f64, lon_deg: f64) -> (usize, usize) {
        let lat = if lat_deg.is_finite() {
            lat_deg.clamp(-90.0, 90.0)
        } else {
            0.0
        };
        let lon = if lon_deg.is_finite() {
            lon_deg.rem_euclid(360.0)
        } else {
            0.0
        };

        let row_frac = (lat + 90.0) / 180.0 * self.nlat as f64;
        let col_frac = lon / 360.0 * self.nlon as f64;

        let row = (row_frac.floor().max(0.0) as usize).min(self.nlat.saturating_sub(1));
        let col = (col_frac.floor().max(0.0) as usize).min(self.nlon.saturating_sub(1));
        (row, col)
    }

    /// Mean area of one grid cell on the canon planet, in square metres.
    ///
    /// This is the planet's true surface area, `4 * PI * R^2`, divided evenly
    /// over every cell, so summing it over the grid gives exactly the sphere's
    /// area. Real lat-lon cells shrink towards the poles; when the row is
    /// known, use [`GridSpec::cell_area_at_row_m2`] for the exact area of that
    /// cell.
    pub fn cell_area_m2(&self) -> f64 {
        self.mean_cell_area_m2(CANON_PLANET_RADIUS_M)
    }

    /// Mean area of one grid cell on a sphere of radius `radius_m`, in square
    /// metres: `4 * PI * R^2 / (nlat * nlon)`. Returns 0 for an empty grid.
    pub fn mean_cell_area_m2(&self, radius_m: f64) -> f64 {
        let cells = self.total_points();
        if cells == 0 {
            return 0.0;
        }
        4.0 * std::f64::consts::PI * radius_m * radius_m / cells as f64
    }

    /// Exact area of a cell in latitude row `row` on a sphere of radius
    /// `radius_m`, in square metres.
    ///
    /// Row `row` spans latitudes `phi1..phi2`, where `phi = (row / nlat - 0.5) * PI`,
    /// matching [`GridSpec::lat_rad`]. Its area is
    /// `R^2 * dlon * (sin(phi2) - sin(phi1))`. Summed over every row and
    /// column, this gives exactly `4 * PI * R^2`.
    ///
    /// Returns `None` if `row` is out of range.
    pub fn cell_area_at_row_m2(&self, row: usize, radius_m: f64) -> Option<f64> {
        if row >= self.nlat || self.nlon == 0 {
            return None;
        }
        let nlat = self.nlat as f64;
        let phi1 = (row as f64 / nlat - 0.5) * std::f64::consts::PI;
        let phi2 = ((row + 1) as f64 / nlat - 0.5) * std::f64::consts::PI;
        let dlon = 2.0 * std::f64::consts::PI / self.nlon as f64;
        Some(radius_m * radius_m * dlon * (phi2.sin() - phi1.sin()))
    }
}

/// 2D lat-lon grid container
///
/// Generic container for 2D data on a latitude-longitude grid.
/// MUST use row-major order (lat outer loop, lon inner loop).
/// Bounds checking enforced.
/// Deterministic initialization.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Grid2<T> {
    data: Vec<T>,
    nlat: usize,
    nlon: usize,
}

impl<T> Grid2<T>
where
    T: Clone,
{
    /// Create new grid with initial value
    ///
    /// # Arguments
    ///
    /// * `spec` - Grid specification
    /// * `init` - Initial value for all grid points
    ///
    /// # Returns
    ///
    /// New Grid2 filled with initial value
    pub fn new(spec: &GridSpec, init: T) -> Self {
        Self {
            data: vec![init; spec.total_points()],
            nlat: spec.nlat,
            nlon: spec.nlon,
        }
    }

    /// Create new grid from existing data
    ///
    /// # Arguments
    ///
    /// * `spec` - Grid specification
    /// * `data` - Grid data (must have correct length)
    ///
    /// # Returns
    ///
    /// New Grid2 with provided data
    ///
    /// # Panics
    ///
    /// Panics if data length doesn't match spec
    pub fn from_data(spec: &GridSpec, data: Vec<T>) -> Self {
        assert_eq!(
            data.len(),
            spec.total_points(),
            "Data length must match grid specification"
        );
        Self {
            data,
            nlat: spec.nlat,
            nlon: spec.nlon,
        }
    }

    /// Get value at grid point
    ///
    /// # Arguments
    ///
    /// * `ilat` - Latitude index (0-based)
    /// * `ilon` - Longitude index (0-based)
    ///
    /// # Returns
    ///
    /// Reference to value at specified location
    ///
    /// # Panics
    ///
    /// Panics if indices are out of bounds
    pub fn get(&self, ilat: usize, ilon: usize) -> &T {
        self.check_bounds(ilat, ilon);
        let index = self.linear_index(ilat, ilon);
        &self.data[index]
    }

    /// Get mutable value at grid point
    ///
    /// # Arguments
    ///
    /// * `ilat` - Latitude index (0-based)
    /// * `ilon` - Longitude index (0-based)
    ///
    /// # Returns
    ///
    /// Mutable reference to value at specified location
    ///
    /// # Panics
    ///
    /// Panics if indices are out of bounds
    pub fn get_mut(&mut self, ilat: usize, ilon: usize) -> &mut T {
        self.check_bounds(ilat, ilon);
        let index = self.linear_index(ilat, ilon);
        &mut self.data[index]
    }

    /// Set value at grid point
    ///
    /// # Arguments
    ///
    /// * `ilat` - Latitude index (0-based)
    /// * `ilon` - Longitude index (0-based)
    /// * `val` - Value to set
    ///
    /// # Panics
    ///
    /// Panics if indices are out of bounds
    pub fn set(&mut self, ilat: usize, ilon: usize, val: T) {
        self.check_bounds(ilat, ilon);
        let index = self.linear_index(ilat, ilon);
        self.data[index] = val;
    }

    /// Get value at grid point (safe version)
    ///
    /// # Arguments
    ///
    /// * `ilat` - Latitude index (0-based)
    /// * `ilon` - Longitude index (0-based)
    ///
    /// # Returns
    ///
    /// Option with reference to value, or None if out of bounds
    pub fn get_safe(&self, ilat: usize, ilon: usize) -> Option<&T> {
        if self.is_valid_index(ilat, ilon) {
            let index = self.linear_index(ilat, ilon);
            Some(&self.data[index])
        } else {
            None
        }
    }

    /// Get mutable value at grid point (safe version)
    ///
    /// # Arguments
    ///
    /// * `ilat` - Latitude index (0-based)
    /// * `ilon` - Longitude index (0-based)
    ///
    /// # Returns
    ///
    /// Option with mutable reference to value, or None if out of bounds
    pub fn get_mut_safe(&mut self, ilat: usize, ilon: usize) -> Option<&mut T> {
        if self.is_valid_index(ilat, ilon) {
            let index = self.linear_index(ilat, ilon);
            Some(&mut self.data[index])
        } else {
            None
        }
    }

    /// Set value at grid point (safe version)
    ///
    /// # Arguments
    ///
    /// * `ilat` - Latitude index (0-based)
    /// * `ilon` - Longitude index (0-based)
    /// * `val` - Value to set
    ///
    /// # Returns
    ///
    /// True if value was set, false if out of bounds
    pub fn set_safe(&mut self, ilat: usize, ilon: usize, val: T) -> bool {
        if self.is_valid_index(ilat, ilon) {
            let index = self.linear_index(ilat, ilon);
            self.data[index] = val;
            true
        } else {
            false
        }
    }

    /// Get grid specification
    ///
    /// # Returns
    ///
    /// GridSpec for this grid
    pub fn spec(&self) -> GridSpec {
        GridSpec::new(self.nlat, self.nlon)
    }

    /// Get number of latitude points
    pub fn nlat(&self) -> usize {
        self.nlat
    }

    /// Get number of longitude points
    pub fn nlon(&self) -> usize {
        self.nlon
    }

    /// Get total number of grid points
    pub fn total_points(&self) -> usize {
        self.nlat * self.nlon
    }

    /// Check if indices are valid
    ///
    /// # Arguments
    ///
    /// * `ilat` - Latitude index
    /// * `ilon` - Longitude index
    ///
    /// # Returns
    ///
    /// True if indices are within bounds
    pub fn is_valid_index(&self, ilat: usize, ilon: usize) -> bool {
        ilat < self.nlat && ilon < self.nlon
    }

    /// Get underlying data as slice (read-only)
    ///
    /// # Returns
    ///
    /// Slice of all grid data in row-major order
    pub fn data(&self) -> &[T] {
        &self.data
    }

    /// Get underlying data as mutable slice
    ///
    /// # Returns
    ///
    /// Mutable slice of all grid data in row-major order
    pub fn data_mut(&mut self) -> &mut [T] {
        &mut self.data
    }

    /// Fill grid with value
    ///
    /// # Arguments
    ///
    /// * `value` - Value to fill with
    pub fn fill(&mut self, value: T) {
        self.data.fill(value);
    }

    /// Clear grid (replace with default values)
    ///
    /// Requires T to implement Default
    pub fn clear(&mut self)
    where
        T: Default,
    {
        self.data.fill(T::default());
    }

    /// Iterate over all grid points with indices
    ///
    /// Returns iterator yielding (ilat, ilon, &value) tuples
    /// Iteration order: lat outer loop, lon inner loop (deterministic)
    pub fn indexed_iter(&'_ self) -> GridIter<'_, T> {
        GridIter {
            grid: self,
            ilat: 0,
            ilon: 0,
        }
    }

    /// Iterate over all grid points with mutable indices
    ///
    /// Returns iterator yielding (ilat, ilon, &mut value) tuples
    /// Iteration order: lat outer loop, lon inner loop (deterministic)
    pub fn indexed_iter_mut(&'_ mut self) -> GridIterMut<'_, T> {
        GridIterMut {
            nlon: self.nlon,
            inner: self.data.iter_mut().enumerate(),
        }
    }

    /// Convert linear index to (ilat, ilon)
    ///
    /// # Arguments
    ///
    /// * `linear_index` - Linear index in data array
    ///
    /// # Returns
    ///
    /// Tuple of (ilat, ilon)
    ///
    /// # Panics
    ///
    /// Panics if linear_index is out of bounds
    pub fn linear_to_grid_index(&self, linear_index: usize) -> (usize, usize) {
        assert!(
            linear_index < self.total_points(),
            "Linear index out of bounds"
        );
        let ilat = linear_index / self.nlon;
        let ilon = linear_index % self.nlon;
        (ilat, ilon)
    }

    /// Convert (ilat, ilon) to linear index
    ///
    /// # Arguments
    ///
    /// * `ilat` - Latitude index
    /// * `ilon` - Longitude index
    ///
    /// # Returns
    ///
    /// Linear index in data array
    ///
    /// # Panics
    ///
    /// Panics if indices are out of bounds
    pub fn grid_to_linear_index(&self, ilat: usize, ilon: usize) -> usize {
        self.check_bounds(ilat, ilon);
        self.linear_index(ilat, ilon)
    }

    /// Check bounds and panic if invalid
    fn check_bounds(&self, ilat: usize, ilon: usize) {
        assert!(
            ilat < self.nlat && ilon < self.nlon,
            "Grid index out of bounds: ilat={}, ilon={}, nlat={}, nlon={}",
            ilat,
            ilon,
            self.nlat,
            self.nlon
        );
    }

    /// Convert 2D indices to linear index (no bounds checking)
    fn linear_index(&self, ilat: usize, ilon: usize) -> usize {
        ilat * self.nlon + ilon
    }
}

/// Iterator over grid points with indices
pub struct GridIter<'a, T> {
    grid: &'a Grid2<T>,
    ilat: usize,
    ilon: usize,
}

impl<'a, T> Iterator for GridIter<'a, T>
where
    T: Clone,
{
    type Item = (usize, usize, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        if self.ilat >= self.grid.nlat {
            return None;
        }

        let result = (self.ilat, self.ilon, self.grid.get(self.ilat, self.ilon));

        // Advance to next position (lat outer, lon inner)
        self.ilon += 1;
        if self.ilon >= self.grid.nlon {
            self.ilon = 0;
            self.ilat += 1;
        }

        Some(result)
    }
}

/// Mutable iterator over grid points with indices
pub struct GridIterMut<'a, T> {
    nlon: usize,
    inner: std::iter::Enumerate<std::slice::IterMut<'a, T>>,
}

impl<'a, T> Iterator for GridIterMut<'a, T> {
    type Item = (usize, usize, &'a mut T);

    fn next(&mut self) -> Option<Self::Item> {
        // The data is row-major (lat outer, lon inner), so walking the slice
        // in order visits cells in that order. Each element is borrowed
        // exactly once, which the slice iterator guarantees without unsafe.
        let (index, value) = self.inner.next()?;
        Some((index / self.nlon, index % self.nlon, value))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<T> fmt::Display for Grid2<T>
where
    T: fmt::Display + Clone,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Grid2<{}x{}>", self.nlat, self.nlon)?;
        for ilat in 0..self.nlat {
            for ilon in 0..self.nlon {
                let val = self.get(ilat, ilon);
                write!(f, "({},{}): {} ", ilat, ilon, val)?;
            }
            writeln!(f)?;
        }
        Ok(())
    }
}

/// Specialized implementations for f64 grids
impl Grid2<f64> {
    /// Sum all values
    pub fn sum(&self) -> f64 {
        self.data.iter().sum()
    }

    /// Get average value across grid
    pub fn average(&self) -> f64 {
        if self.data.is_empty() {
            0.0
        } else {
            self.sum() / (self.data.len() as f64)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indexed_iter_mut_yields_every_cell_once_in_row_major_order() {
        let spec = GridSpec::new(3, 4);
        let mut grid = Grid2::new(&spec, 0usize);
        // Holding every &mut at once is sound only if each is distinct.
        let cells: Vec<_> = grid.indexed_iter_mut().collect();
        assert_eq!(cells.len(), 12);
        for (i, (row, col, value)) in cells.into_iter().enumerate() {
            assert_eq!((row, col), (i / 4, i % 4));
            *value = i;
        }
        assert_eq!(*grid.get(2, 3), 11);
        assert_eq!(*grid.get(1, 0), 4);
    }

    #[test]
    fn canon_planet_radius_matches_canon() {
        assert_eq!(
            CANON_PLANET_RADIUS_M,
            crate::canon::CanonLocked::default().planet_radius_m
        );
    }

    #[test]
    fn cell_areas_sum_to_sphere_area() {
        let r = CANON_PLANET_RADIUS_M;
        let sphere = 4.0 * std::f64::consts::PI * r * r;
        for (nlat, nlon) in [(32, 64), (1, 1), (7, 13), (180, 360)] {
            let spec = GridSpec::new(nlat, nlon);
            let mean_total = spec.cell_area_m2() * spec.total_points() as f64;
            assert!((mean_total - sphere).abs() / sphere < 1e-12);

            let exact_total: f64 = (0..nlat)
                .map(|row| spec.cell_area_at_row_m2(row, r).unwrap() * nlon as f64)
                .sum();
            assert!(
                (exact_total - sphere).abs() / sphere < 1e-12,
                "{nlat}x{nlon}: {exact_total} vs {sphere}"
            );
        }
    }

    #[test]
    fn row_cell_area_shrinks_towards_poles() {
        let spec = GridSpec::new(32, 64);
        let r = CANON_PLANET_RADIUS_M;
        let polar = spec.cell_area_at_row_m2(0, r).unwrap();
        let equatorial = spec.cell_area_at_row_m2(16, r).unwrap();
        let north_polar = spec.cell_area_at_row_m2(31, r).unwrap();
        assert!(polar < equatorial);
        assert!((polar - north_polar).abs() / polar < 1e-9);
        assert_eq!(spec.cell_area_at_row_m2(32, r), None);
    }

    #[test]
    fn empty_grid_has_no_area() {
        let spec = GridSpec::new(0, 0);
        assert_eq!(spec.cell_area_m2(), 0.0);
        assert_eq!(spec.cell_area_at_row_m2(0, CANON_PLANET_RADIUS_M), None);
    }

    #[test]
    fn grid_spec_creation() {
        let spec = GridSpec::new(10, 20);
        assert_eq!(spec.nlat, 10);
        assert_eq!(spec.nlon, 20);
        assert_eq!(spec.total_points(), 200);
    }

    #[test]
    fn grid_spec_valid_index() {
        let spec = GridSpec::new(10, 20);

        assert!(spec.is_valid_index(0, 0));
        assert!(spec.is_valid_index(9, 19));
        assert!(!spec.is_valid_index(10, 0));
        assert!(!spec.is_valid_index(0, 20));
        assert!(!spec.is_valid_index(10, 20));
    }

    #[test]
    fn grid2_creation() {
        let spec = GridSpec::new(3, 4);
        let grid = Grid2::new(&spec, 42);

        assert_eq!(grid.nlat(), 3);
        assert_eq!(grid.nlon(), 4);
        assert_eq!(grid.total_points(), 12);

        // All values should be 42
        for ilat in 0..3 {
            for ilon in 0..4 {
                assert_eq!(*grid.get(ilat, ilon), 42);
            }
        }
    }

    #[test]
    fn grid2_get_set() {
        let spec = GridSpec::new(3, 4);
        let mut grid = Grid2::new(&spec, 0);

        grid.set(1, 2, 123);
        assert_eq!(*grid.get(1, 2), 123);

        *grid.get_mut(2, 3) = 456;
        assert_eq!(*grid.get(2, 3), 456);
    }

    #[test]
    #[should_panic(expected = "Grid index out of bounds")]
    fn grid2_get_out_of_bounds() {
        let spec = GridSpec::new(3, 4);
        let grid = Grid2::new(&spec, 0);
        grid.get(3, 0); // ilat out of bounds
    }

    #[test]
    #[should_panic(expected = "Grid index out of bounds")]
    fn grid2_set_out_of_bounds() {
        let spec = GridSpec::new(3, 4);
        let mut grid = Grid2::new(&spec, 0);
        grid.set(0, 4, 999); // ilon out of bounds
    }

    #[test]
    fn grid2_safe_operations() {
        let spec = GridSpec::new(3, 4);
        let mut grid = Grid2::new(&spec, 0);

        // Valid operations
        assert!(grid.get_safe(1, 2).is_some());
        assert!(grid.get_mut_safe(1, 2).is_some());
        assert!(grid.set_safe(1, 2, 123));

        // Invalid operations
        assert!(grid.get_safe(3, 0).is_none());
        assert!(grid.get_mut_safe(0, 4).is_none());
        assert!(!grid.set_safe(3, 0, 123));
    }

    #[test]
    fn grid2_fill() {
        let spec = GridSpec::new(3, 4);
        let mut grid = Grid2::new(&spec, 0);

        grid.fill(999);

        for ilat in 0..3 {
            for ilon in 0..4 {
                assert_eq!(*grid.get(ilat, ilon), 999);
            }
        }
    }

    #[test]
    fn grid2_clear() {
        let spec = GridSpec::new(3, 4);
        let mut grid = Grid2::new(&spec, 123);

        grid.clear();

        for ilat in 0..3 {
            for ilon in 0..4 {
                assert_eq!(*grid.get(ilat, ilon), 0);
            }
        }
    }

    #[test]
    fn grid2_index_conversion() {
        let spec = GridSpec::new(3, 4);
        let grid = Grid2::new(&spec, 0);

        // Test linear to grid index
        assert_eq!(grid.linear_to_grid_index(0), (0, 0));
        assert_eq!(grid.linear_to_grid_index(3), (0, 3));
        assert_eq!(grid.linear_to_grid_index(4), (1, 0));
        assert_eq!(grid.linear_to_grid_index(11), (2, 3));

        // Test grid to linear index
        assert_eq!(grid.grid_to_linear_index(0, 0), 0);
        assert_eq!(grid.grid_to_linear_index(0, 3), 3);
        assert_eq!(grid.grid_to_linear_index(1, 0), 4);
        assert_eq!(grid.grid_to_linear_index(2, 3), 11);
    }

    #[test]
    fn grid2_indexed_iter() {
        let spec = GridSpec::new(2, 3);
        let mut grid = Grid2::new(&spec, 0);

        // Set some values
        grid.set(0, 0, 10);
        grid.set(0, 1, 20);
        grid.set(1, 2, 30);

        let mut collected = Vec::new();
        for (ilat, ilon, val) in grid.indexed_iter() {
            collected.push((ilat, ilon, *val));
        }

        assert_eq!(collected.len(), 6);
        assert_eq!(collected[0], (0, 0, 10));
        assert_eq!(collected[1], (0, 1, 20));
        assert_eq!(collected[2], (0, 2, 0));
        assert_eq!(collected[3], (1, 0, 0));
        assert_eq!(collected[4], (1, 1, 0));
        assert_eq!(collected[5], (1, 2, 30));
    }

    #[test]
    fn grid2_indexed_iter_mut() {
        let spec = GridSpec::new(2, 2);
        let mut grid = Grid2::new(&spec, 0);

        for (ilat, ilon, val) in grid.indexed_iter_mut() {
            *val = (ilat * 10 + ilon) as i32;
        }

        assert_eq!(*grid.get(0, 0), 0);
        assert_eq!(*grid.get(0, 1), 1);
        assert_eq!(*grid.get(1, 0), 10);
        assert_eq!(*grid.get(1, 1), 11);
    }

    #[test]
    fn grid2_from_data() {
        let spec = GridSpec::new(2, 3);
        let data = vec![1, 2, 3, 4, 5, 6];
        let grid = Grid2::from_data(&spec, data);

        assert_eq!(*grid.get(0, 0), 1);
        assert_eq!(*grid.get(0, 1), 2);
        assert_eq!(*grid.get(0, 2), 3);
        assert_eq!(*grid.get(1, 0), 4);
        assert_eq!(*grid.get(1, 1), 5);
        assert_eq!(*grid.get(1, 2), 6);
    }

    #[test]
    #[should_panic(expected = "Data length must match grid specification")]
    fn grid2_from_data_wrong_length() {
        let spec = GridSpec::new(2, 3);
        let data = vec![1, 2, 3]; // Wrong length
        Grid2::from_data(&spec, data);
    }

    #[test]
    fn grid2_data_access() {
        let spec = GridSpec::new(2, 3);
        let grid = Grid2::new(&spec, 42);

        let data = grid.data();
        assert_eq!(data.len(), 6);
        assert!(data.iter().all(|&x| x == 42));

        let mut grid = Grid2::new(&spec, 0);
        let data_mut = grid.data_mut();
        data_mut.fill(123);

        for ilat in 0..2 {
            for ilon in 0..3 {
                assert_eq!(*grid.get(ilat, ilon), 123);
            }
        }
    }

    #[test]
    fn grid2_display() {
        let spec = GridSpec::new(2, 2);
        let mut grid = Grid2::new(&spec, 0);
        grid.set(0, 0, 1);
        grid.set(0, 1, 2);
        grid.set(1, 0, 3);
        grid.set(1, 1, 4);

        let display = format!("{}", grid);
        assert!(display.contains("Grid2<2x2>"));
        assert!(display.contains("(0,0): 1"));
        assert!(display.contains("(0,1): 2"));
        assert!(display.contains("(1,0): 3"));
        assert!(display.contains("(1,1): 4"));
    }

    #[test]
    fn grid2_deterministic_iteration_order() {
        let spec = GridSpec::new(3, 2);
        let grid = Grid2::new(&spec, 0);

        let mut indices = Vec::new();
        for (ilat, ilon, _) in grid.indexed_iter() {
            indices.push((ilat, ilon));
        }

        // Should be row-major: lat outer, lon inner
        let expected = vec![
            (0, 0),
            (0, 1), // First row
            (1, 0),
            (1, 1), // Second row
            (2, 0),
            (2, 1), // Third row
        ];

        assert_eq!(indices, expected);
    }

    #[test]
    fn cell_for_lat_lon_inverts_cell_centres() {
        let spec = GridSpec::new(32, 64);
        for row in 0..spec.nlat() {
            for col in 0..spec.nlon() {
                let lat_deg = spec.lat_rad(row).to_degrees();
                let lon_deg = spec.lon_rad(col).to_degrees();
                assert_eq!(
                    spec.cell_for_lat_lon(lat_deg, lon_deg),
                    (row, col),
                    "round-trip failed for cell ({row}, {col})"
                );
            }
        }
    }

    #[test]
    fn cell_for_lat_lon_accepts_signed_longitude() {
        let spec = GridSpec::new(32, 64);
        // -90 deg and +270 deg are the same meridian.
        assert_eq!(
            spec.cell_for_lat_lon(0.0, -90.0),
            spec.cell_for_lat_lon(0.0, 270.0)
        );
    }

    #[test]
    fn cell_for_lat_lon_clamps_poles_and_nonfinite() {
        let spec = GridSpec::new(32, 64);
        assert_eq!(spec.cell_for_lat_lon(90.0, 0.0).0, spec.nlat() - 1);
        assert_eq!(spec.cell_for_lat_lon(-90.0, 0.0).0, 0);
        assert_eq!(spec.cell_for_lat_lon(1000.0, 0.0).0, spec.nlat() - 1);
        // Non-finite input resolves to a valid cell rather than panicking.
        let (row, col) = spec.cell_for_lat_lon(f64::NAN, f64::INFINITY);
        assert!(spec.is_valid_index(row, col));
    }
}
