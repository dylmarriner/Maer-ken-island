//! Deterministic terrain-slope occupancy gating.
//!
//! Ported from markenz's `crates/physics/` (`can_occupy`/
//! `collides_with_terrain`), which had no equivalent anywhere in Maer-Ken
//! prior to this — see finding 4 in
//! `audit-results/maerken-vs-gemini-markenz-gap-audit.md`: without it,
//! agents could build on (and, if a movement system is added later, walk
//! onto) arbitrarily steep terrain with no physics check at all.
//!
//! This is a pure function of the existing `mk_core::grid::Grid2<f64>`
//! elevation field — no RNG, no wall-clock, deterministic and reproducible
//! for a given elevation grid.
//!
//! **Out-of-bounds policy** (formalized in `docs/decisions.md` — "Terrain
//! out-of-bounds policy differs by question asked" — after
//! `audit-results/reconciled-findings-2026-09-03.md` finding 4 flagged it
//! as an informal, code-comment-only decision): [`collides_with_terrain`]
//! and [`is_buildable`] deliberately answer different questions and so
//! resolve an out-of-bounds neighbor differently —
//! [`collides_with_terrain`] blocks any move to/from an out-of-bounds cell
//! (you cannot occupy off-grid terrain, so movement there is always
//! illegal); [`is_buildable`] ignores an out-of-bounds *neighbor* of an
//! in-bounds cell being founded (there is no slope data for terrain that
//! doesn't exist, so it can't be violated). This is not accidental drift —
//! do not "fix" one to match the other without re-reading the ADR.

use mk_core::grid::Grid2;

/// Maximum elevation difference (in the same units as the elevation grid,
/// meters) between a cell and any of its 4-connected neighbors for that
/// cell to be considered occupiable/buildable. Mirrors markenz's "max climb
/// of 2 height units" gate; the exact threshold is a first port and may
/// need tuning against Maer-Ken's actual elevation scale/canon.
pub const MAX_CLIMB_HEIGHT_M: f64 = 2.0;

/// Row/col offsets of the 4-connected neighborhood.
const NEIGHBOR_OFFSETS: [(i32, i32); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];

/// True if `(row, col)` is inside the elevation grid's bounds.
pub fn can_occupy(elevation_grid: &Grid2<f64>, row: i32, col: i32) -> bool {
    row >= 0
        && col >= 0
        && (row as usize) < elevation_grid.nlat()
        && (col as usize) < elevation_grid.nlon()
}

/// True if moving from `(from_row, from_col)` to `(to_row, to_col)` is
/// blocked by terrain: either cell is out of bounds, or the elevation
/// difference between them exceeds [`MAX_CLIMB_HEIGHT_M`].
///
/// Only meaningful for adjacent cells (the two positions one 4-connected
/// step apart); a caller checking a longer path should test it one step at
/// a time.
pub fn collides_with_terrain(
    elevation_grid: &Grid2<f64>,
    from_row: i32,
    from_col: i32,
    to_row: i32,
    to_col: i32,
) -> bool {
    if !can_occupy(elevation_grid, from_row, from_col)
        || !can_occupy(elevation_grid, to_row, to_col)
    {
        return true;
    }
    let from_elev = *elevation_grid.get(from_row as usize, from_col as usize);
    let to_elev = *elevation_grid.get(to_row as usize, to_col as usize);
    (to_elev - from_elev).abs() > MAX_CLIMB_HEIGHT_M
}

/// True if a structure can be founded at `(row, col)`: the cell itself must
/// be in-bounds, and the local slope (elevation difference to every
/// in-bounds 4-connected neighbor) must not exceed [`MAX_CLIMB_HEIGHT_M`].
/// A cell with no in-bounds neighbors (a 1xN/Nx1 degenerate grid) is
/// treated as buildable — there is no slope to violate.
pub fn is_buildable(elevation_grid: &Grid2<f64>, row: i32, col: i32) -> bool {
    if !can_occupy(elevation_grid, row, col) {
        return false;
    }
    let elev = *elevation_grid.get(row as usize, col as usize);
    NEIGHBOR_OFFSETS.iter().all(|(dr, dc)| {
        let nr = row + dr;
        let nc = col + dc;
        if !can_occupy(elevation_grid, nr, nc) {
            return true;
        }
        let neighbor_elev = *elevation_grid.get(nr as usize, nc as usize);
        (elev - neighbor_elev).abs() <= MAX_CLIMB_HEIGHT_M
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use mk_core::grid::{Grid2, GridSpec};

    fn flat_grid(nlat: usize, nlon: usize, value: f64) -> Grid2<f64> {
        Grid2::new(&GridSpec::new(nlat, nlon), value)
    }

    #[test]
    fn can_occupy_respects_bounds() {
        let grid = flat_grid(4, 4, 0.0);
        assert!(can_occupy(&grid, 0, 0));
        assert!(can_occupy(&grid, 3, 3));
        assert!(!can_occupy(&grid, -1, 0));
        assert!(!can_occupy(&grid, 0, 4));
        assert!(!can_occupy(&grid, 4, 0));
    }

    #[test]
    fn flat_terrain_is_always_buildable_and_traversable() {
        let grid = flat_grid(4, 4, 100.0);
        for r in 0..4 {
            for c in 0..4 {
                assert!(is_buildable(&grid, r, c), "({r},{c}) should be buildable");
            }
        }
        assert!(!collides_with_terrain(&grid, 1, 1, 1, 2));
    }

    #[test]
    fn out_of_bounds_policy_deliberately_differs_by_question_asked() {
        // Pins the ADR'd policy in `docs/decisions.md` ("Terrain
        // out-of-bounds policy differs by question asked"): an edge cell
        // with an out-of-bounds neighbor is buildable (no slope data for
        // terrain that doesn't exist), but moving to/from an out-of-bounds
        // cell is always blocked (you cannot occupy off-grid terrain). If
        // this test starts failing because someone unified the two
        // policies, re-read the ADR before "fixing" it.
        let grid = flat_grid(1, 3, 50.0); // 1xN degenerate grid: every cell has an OOB row-neighbor.
        assert!(
            is_buildable(&grid, 0, 1),
            "an in-bounds cell with only out-of-bounds neighbors must remain buildable"
        );
        assert!(
            collides_with_terrain(&grid, 0, 1, -1, 1),
            "moving to an out-of-bounds cell must always be blocked"
        );
        assert!(
            collides_with_terrain(&grid, -1, 1, 0, 1),
            "moving from an out-of-bounds cell must always be blocked"
        );
    }

    #[test]
    fn steep_step_blocks_traversal_and_building() {
        let mut grid = flat_grid(3, 3, 0.0);
        // A cliff: (1,1) is far higher than its neighbors.
        *grid.get_mut(1, 1) = MAX_CLIMB_HEIGHT_M * 10.0;

        assert!(collides_with_terrain(&grid, 1, 0, 1, 1));
        assert!(collides_with_terrain(&grid, 1, 1, 1, 2));
        assert!(!is_buildable(&grid, 1, 1));
        // Neighbors are also un-buildable, since (1,1) violates their slope too.
        assert!(!is_buildable(&grid, 1, 0));
    }

    #[test]
    fn gentle_step_within_max_climb_is_fine() {
        let mut grid = flat_grid(3, 3, 0.0);
        *grid.get_mut(1, 1) = MAX_CLIMB_HEIGHT_M * 0.5;

        assert!(!collides_with_terrain(&grid, 1, 0, 1, 1));
        assert!(is_buildable(&grid, 1, 1));
    }

    #[test]
    fn out_of_bounds_positions_block_traversal_and_building() {
        let grid = flat_grid(2, 2, 0.0);
        assert!(collides_with_terrain(&grid, 0, 0, -1, 0));
        assert!(!is_buildable(&grid, -1, 0));
        assert!(!is_buildable(&grid, 5, 5));
    }
}
