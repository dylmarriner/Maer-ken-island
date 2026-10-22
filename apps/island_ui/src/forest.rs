//! The island's woods beyond the estate, as tree marks (Phase 5 Task 3
//! Step 2a).
//!
//! Outside the patch the island holds no individual trees, only standing
//! biomass per medium cell (`/api/cover.bin`). The ground is already tinted
//! by it (`terrain::chunk_colours`); this adds marks standing up from the
//! ground, so a wood reads as a wood from an angle as well as from above.
//!
//! They are symbols, as on a map: their number in a block follows its
//! biomass against the island's densest cell, and their size follows the
//! block they stand in, not any tree's height or crown. At 10 km to a
//! regional unit a real tree would be far smaller than a pixel. Where each
//! one stands inside its block comes from a hash of the block and the mark's
//! number, so the same cover always draws the same wood on every machine.

use mk_island_api::{Cover, Terrain};

use crate::scene::{regional_of, Point};

/// Cells along one edge of a block, the unit one set of marks stands for.
/// Eight keeps the island's 1,200 x 960 cells to 18,000 blocks.
pub const BLOCK_CELLS: usize = 8;

/// Marks in a block as dense as the island's densest cell. A drawing
/// choice: enough to read as a wood, few enough to merge into one mesh per
/// ground chunk.
pub const MARKS_AT_PEAK: usize = 6;

/// How many marks a block gets, from how fully stocked it is: 0 (open) to
/// 1 (the island's densest stand), rounded to the nearest whole mark.
pub fn marks_for(stocked: f32) -> usize {
    if !stocked.is_finite() {
        return 0;
    }
    (stocked.clamp(0.0, 1.0) * MARKS_AT_PEAK as f32).round() as usize
}

/// A stable pseudo-random fraction in [0, 1) for a block and a mark,
/// using SplitMix64's finaliser. It is the same on every machine and for
/// every read of the same cover.
pub fn jitter(block_row: usize, block_col: usize, mark: usize, axis: u64) -> f32 {
    let mut z = (block_row as u64)
        .wrapping_mul(0x9E37_79B9_7F4A_7C15)
        .wrapping_add((block_col as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F))
        .wrapping_add((mark as u64).wrapping_mul(0x1656_67B1_9E37_79F9))
        .wrapping_add(axis);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^= z >> 31;
    (z >> 40) as f32 / (1u64 << 24) as f32
}

/// One mark, placed in the regional frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mark {
    /// Where it stands on the ground.
    pub base: Point,
    /// The block's width in regional units, which the mark is drawn
    /// against.
    pub block_units: f32,
}

/// The marks for the blocks whose rows and columns of cells lie in `rows`
/// and `cols`, which should be whole blocks (a chunk's range is, because
/// `CHUNK_CELLS` is a multiple of `BLOCK_CELLS`). A block's stocking is the
/// mean over its land cells. A mark that would fall in the sea is left out,
/// and a cover of another shape than the terrain draws none, rather than
/// guessing which cell is which.
pub fn marks_in(
    terrain: &Terrain,
    cover: &Cover,
    peak_kgc_m2: f32,
    rows: std::ops::Range<usize>,
    cols: std::ops::Range<usize>,
) -> Vec<Mark> {
    let mut marks = Vec::new();
    if cover.rows != terrain.rows || cover.cols != terrain.cols {
        return marks;
    }
    let size_m = f64::from(terrain.cell_size_m);
    let block_m = size_m * BLOCK_CELLS as f64;
    let block_units = {
        let a = regional_of(terrain, 0.0, 0.0, 0.0);
        let b = regional_of(terrain, block_m, 0.0, 0.0);
        b.x - a.x
    };
    let first = |range: &std::ops::Range<usize>| range.start / BLOCK_CELLS;
    let last = |range: &std::ops::Range<usize>, limit: usize| {
        range.end.min(limit).div_ceil(BLOCK_CELLS)
    };
    for block_row in first(&rows)..last(&rows, terrain.rows) {
        for block_col in first(&cols)..last(&cols, terrain.cols) {
            let (row0, col0) = (block_row * BLOCK_CELLS, block_col * BLOCK_CELLS);
            let (mut sum, mut land) = (0.0_f32, 0usize);
            for row in row0..(row0 + BLOCK_CELLS).min(terrain.rows) {
                for col in col0..(col0 + BLOCK_CELLS).min(terrain.cols) {
                    if terrain.is_land(row, col) == Some(true) {
                        sum += cover.fraction_at(row, col, peak_kgc_m2).unwrap_or(0.0);
                        land += 1;
                    }
                }
            }
            if land == 0 {
                continue;
            }
            for mark in 0..marks_for(sum / land as f32) {
                let u = jitter(block_row, block_col, mark, 1);
                let v = jitter(block_row, block_col, mark, 2);
                let row = row0 + ((v * BLOCK_CELLS as f32) as usize).min(BLOCK_CELLS - 1);
                let col = col0 + ((u * BLOCK_CELLS as f32) as usize).min(BLOCK_CELLS - 1);
                if terrain.is_land(row, col) != Some(true) {
                    continue;
                }
                let elevation = f64::from(terrain.elevation_at(row, col).unwrap_or(0.0));
                let x_m = (col0 as f64 + f64::from(u) * BLOCK_CELLS as f64) * size_m;
                let y_m = (row0 as f64 + f64::from(v) * BLOCK_CELLS as f64) * size_m;
                marks.push(Mark {
                    base: regional_of(terrain, x_m, y_m, elevation),
                    block_units,
                });
            }
        }
    }
    marks
}

#[cfg(test)]
mod tests {
    use super::*;

    fn island(rows: usize, cols: usize, biomass: impl Fn(usize, usize) -> f32) -> (Terrain, Cover) {
        let n = rows * cols;
        let terrain = Terrain {
            rows,
            cols,
            cell_size_m: 100.0,
            elevation_m: vec![10.0; n],
            land: vec![true; n],
        };
        let cover = Cover {
            rows,
            cols,
            biomass_kgc_m2: (0..n).map(|i| biomass(i / cols, i % cols)).collect(),
        };
        (terrain, cover)
    }

    #[test]
    fn a_block_gets_marks_in_proportion_to_how_fully_it_is_stocked() {
        assert_eq!(marks_for(0.0), 0);
        assert_eq!(marks_for(1.0), MARKS_AT_PEAK);
        assert_eq!(marks_for(0.5), MARKS_AT_PEAK / 2);
        assert_eq!(marks_for(2.0), MARKS_AT_PEAK);
        assert_eq!(marks_for(f32::NAN), 0);
        // Over a whole island: open ground gets none, the densest block
        // the most, and one half as stocked half as many.
        let b = BLOCK_CELLS;
        let (terrain, cover) = island(b, 3 * b, |_, col| match col / BLOCK_CELLS {
            0 => 0.0,
            1 => 5.0,
            _ => 10.0,
        });
        let marks = marks_in(&terrain, &cover, cover.peak_kgc_m2(), 0..b, 0..3 * b);
        let in_block = |k: usize| {
            let block = regional_of(&terrain, (k * b) as f64 * 100.0, 0.0, 0.0).x;
            let next = regional_of(&terrain, ((k + 1) * b) as f64 * 100.0, 0.0, 0.0).x;
            marks
                .iter()
                .filter(|m| m.base.x >= block && m.base.x < next)
                .count()
        };
        assert_eq!(in_block(0), 0);
        assert_eq!(in_block(1), MARKS_AT_PEAK / 2);
        assert_eq!(in_block(2), MARKS_AT_PEAK);
    }

    #[test]
    fn the_same_cover_draws_the_same_wood_and_nothing_stands_in_the_sea() {
        let b = BLOCK_CELLS;
        let (mut terrain, cover) = island(2 * b, 2 * b, |_, _| 3.0);
        let once = marks_in(&terrain, &cover, 3.0, 0..2 * b, 0..2 * b);
        let again = marks_in(&terrain, &cover, 3.0, 0..2 * b, 0..2 * b);
        assert_eq!(once, again);
        assert_eq!(once.len(), 4 * MARKS_AT_PEAK);
        // Offsets differ from mark to mark, and stay inside [0, 1).
        let fractions: Vec<f32> = (0..MARKS_AT_PEAK).map(|k| jitter(3, 7, k, 1)).collect();
        assert!(fractions.iter().all(|f| (0.0..1.0).contains(f)));
        assert!(fractions.windows(2).any(|w| w[0] != w[1]));
        // The whole island under the sea: no marks.
        terrain.land.iter_mut().for_each(|l| *l = false);
        assert!(marks_in(&terrain, &cover, 3.0, 0..2 * b, 0..2 * b).is_empty());
        // A cover read from another island's grid draws nothing.
        let (other, _) = island(b, b, |_, _| 3.0);
        assert!(marks_in(&other, &cover, 3.0, 0..b, 0..b).is_empty());
    }
}
