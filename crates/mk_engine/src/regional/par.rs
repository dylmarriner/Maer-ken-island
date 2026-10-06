//! Deterministic multi-core iteration over regional cells.
//!
//! Per-cell physics within a step is independent, so it runs on every
//! core. Results must be bit-identical at any thread count: maps keep their
//! input order, and floating-point reductions sum fixed
//! [`DET_SUM_CHUNK`]-element chunks in order, then the chunk sums in order,
//! so the order of additions never depends on scheduling. Regional
//! simulation code uses these helpers rather than rayon's `.sum()`.
//!
//! The thread count comes from `RAYON_NUM_THREADS` or all cores; it never
//! changes a result.

use mk_core::grid::Grid2;
use rayon::prelude::*;

/// Elements per chunk of a deterministic sum.
pub const DET_SUM_CHUNK: usize = 4_096;

/// `f(row, col)` for every cell, in parallel, results in input order.
pub fn par_map_cells<T, F>(cells: &[(usize, usize)], f: F) -> Vec<T>
where
    T: Send,
    F: Fn(usize, usize) -> T + Sync,
{
    cells.par_iter().map(|&(row, col)| f(row, col)).collect()
}

/// Runs `f(row, col, &mut value)` on each listed cell of `grid`, in
/// parallel. Each cell is visited once even if listed twice; cells outside
/// the grid are ignored.
pub fn par_for_each_cell_mut<T, F>(grid: &mut Grid2<T>, cells: &[(usize, usize)], f: F)
where
    T: Send + Clone,
    F: Fn(usize, usize, &mut T) + Sync,
{
    let (rows, cols) = (grid.spec().nlat, grid.spec().nlon);
    let mut selected = vec![false; rows * cols];
    for &(row, col) in cells {
        if row < rows && col < cols {
            selected[row * cols + col] = true;
        }
    }
    grid.data_mut()
        .par_iter_mut()
        .enumerate()
        .filter(|(index, _)| selected[*index])
        .for_each(|(index, value)| f(index / cols, index % cols, value));
}

/// Sum whose result does not depend on the thread count.
pub fn det_sum(values: &[f64]) -> f64 {
    let chunk_sums: Vec<f64> = values
        .par_chunks(DET_SUM_CHUNK)
        .map(|chunk| chunk.iter().fold(0.0, |acc, v| acc + v))
        .collect();
    chunk_sums.iter().fold(0.0, |acc, v| acc + v)
}

/// `det_sum` of `f(row, col)` over `cells`, without materialising every
/// term.
pub fn det_sum_by<F>(cells: &[(usize, usize)], f: F) -> f64
where
    F: Fn(usize, usize) -> f64 + Sync,
{
    let chunk_sums: Vec<f64> = cells
        .par_chunks(DET_SUM_CHUNK)
        .map(|chunk| chunk.iter().fold(0.0, |acc, &(row, col)| acc + f(row, col)))
        .collect();
    chunk_sums.iter().fold(0.0, |acc, v| acc + v)
}
