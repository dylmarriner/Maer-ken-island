//! Phase 1 Task 1b: per-cell parallelism gives identical results at any
//! thread count.

use mk_core::grid::{Grid2, GridSpec};
use mk_engine::regional::par::{det_sum, det_sum_by, par_for_each_cell_mut, par_map_cells};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

fn with_threads<T: Send>(threads: usize, f: impl FnOnce() -> T + Send) -> T {
    rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .build()
        .expect("thread pool")
        .install(f)
}

fn values() -> Vec<f64> {
    let mut rng = ChaCha8Rng::seed_from_u64(42);
    // Wildly different magnitudes make float sums order-sensitive.
    (0..100_003)
        .map(|_| rng.gen_range(-1.0..1.0) * 10f64.powi(rng.gen_range(-8..8)))
        .collect()
}

#[test]
fn det_sum_is_bit_identical_at_any_thread_count() {
    let v = values();
    let one = with_threads(1, || det_sum(&v));
    for threads in [2, 8] {
        assert_eq!(
            with_threads(threads, || det_sum(&v)).to_bits(),
            one.to_bits()
        );
    }
    assert_eq!(det_sum(&v).to_bits(), one.to_bits());

    let cells: Vec<(usize, usize)> = (0..300)
        .flat_map(|r| (0..300).map(move |c| (r, c)))
        .collect();
    let f = |r: usize, c: usize| v[(r * 300 + c) % v.len()];
    let by_one = with_threads(1, || det_sum_by(&cells, f));
    assert_eq!(
        with_threads(8, || det_sum_by(&cells, f)).to_bits(),
        by_one.to_bits()
    );
}

#[test]
fn par_map_cells_keeps_input_order() {
    let cells: Vec<(usize, usize)> = (0..50)
        .rev()
        .flat_map(|r| (0..40).map(move |c| (r, c)))
        .collect();
    let mapped = with_threads(8, || par_map_cells(&cells, |r, c| r * 1000 + c));
    let expected: Vec<usize> = cells.iter().map(|&(r, c)| r * 1000 + c).collect();
    assert_eq!(mapped, expected);
}

#[test]
fn a_per_cell_step_gives_identical_grids_with_one_and_eight_threads() {
    let spec = GridSpec::new(192, 240);
    let v = values();
    let run = |threads: usize| {
        with_threads(threads, || {
            let mut grid = Grid2::new(&spec, 0.0f64);
            let cells: Vec<(usize, usize)> = (0..192)
                .flat_map(|r| (0..240).map(move |c| (r, c)))
                .filter(|(r, c)| (r + c) % 3 != 0)
                .collect();
            for step in 0..5 {
                par_for_each_cell_mut(&mut grid, &cells, |r, c, x| {
                    *x = (*x + v[(r * 240 + c + step) % v.len()]).sin() * 1.5;
                });
            }
            grid.data()
                .iter()
                .flat_map(|x| x.to_le_bytes())
                .collect::<Vec<u8>>()
        })
    };
    assert_eq!(run(1), run(8));
}
