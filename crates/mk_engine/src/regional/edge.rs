//! Explicit boundary cells (Phase 2 Task 3): the outermost cells of the
//! domain relax toward the edge forcing, and the interior never reads
//! across an edge. The relaxation weight falls linearly from 1 on the
//! edge ring to 0 at [`EDGE_RELAX_CELLS`] cells in, so the forcing enters
//! only through the ocean buffer.

use mk_island::Edge;

/// Cells (coarse, 12 km) over which an edge's forcing fades out: ~60 km,
/// well inside the 300 km ocean buffer.
pub const EDGE_RELAX_CELLS: usize = 5;

/// Relax `values` (row-major, `rows × cols`) toward the edge targets:
/// `target(edge, index_along_edge)` is the forcing there, `None` when the
/// edge has no value. Where two edges overlap (corners) their targets mix
/// by weight; the cell's pull is the strongest weight.
pub fn relax_to_edges(
    values: &mut [f64],
    rows: usize,
    cols: usize,
    target: impl Fn(Edge, usize) -> Option<f64>,
) {
    for row in 0..rows {
        for col in 0..cols {
            let (mut weight_sum, mut weighted_target, mut strongest) = (0.0, 0.0, 0.0_f64);
            for edge in Edge::ALL {
                let (distance, along) = match edge {
                    Edge::South => (row, col),
                    Edge::North => (rows - 1 - row, col),
                    Edge::West => (col, row),
                    Edge::East => (cols - 1 - col, row),
                };
                if distance >= EDGE_RELAX_CELLS {
                    continue;
                }
                let Some(t) = target(edge, along) else {
                    continue;
                };
                let w = 1.0 - distance as f64 / EDGE_RELAX_CELLS as f64;
                weight_sum += w;
                weighted_target += w * t;
                strongest = strongest.max(w);
            }
            if weight_sum > 0.0 {
                let v = &mut values[row * cols + col];
                *v += strongest * (weighted_target / weight_sum - *v);
            }
        }
    }
}
