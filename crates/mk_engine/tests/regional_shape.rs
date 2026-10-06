//! Phase 1 Task 4: shape metrics reject blobs and ellipses and accept an
//! indented coastline.

use mk_core::grid::{Grid2, GridSpec};
use mk_engine::regional::shape::measure_shape;
use mk_island::ShapeRequirements;

const N: usize = 400;
const CELL_M: f64 = 2_000.0;
const COARSE_M: f64 = 12_000.0;

fn mask(f: impl Fn(f64, f64) -> bool) -> Grid2<bool> {
    let spec = GridSpec::new(N, N);
    let mut g = Grid2::new(&spec, false);
    for r in 0..N {
        for c in 0..N {
            let (x, y) = (
                c as f64 + 0.5 - N as f64 / 2.0,
                r as f64 + 0.5 - N as f64 / 2.0,
            );
            g.set(r, c, f(x, y));
        }
    }
    g
}

#[test]
fn a_disc_fails_and_measures_like_a_circle() {
    let disc = mask(|x, y| x.hypot(y) < 150.0);
    let m = measure_shape(&disc, CELL_M, COARSE_M);
    assert!((m.compactness - 1.0).abs() < 0.05, "{m:?}");
    assert!(m.convexity > 0.97, "{m:?}");
    assert_eq!((m.major_bays, m.major_headlands), (0, 0), "{m:?}");
    // Perimeter within 2% of 2πr: marching squares, not the staircase.
    let expected = std::f64::consts::TAU * 150.0 * CELL_M;
    assert!(
        (m.perimeter_m - expected).abs() / expected < 0.02,
        "{}",
        m.perimeter_m
    );
    assert!(!ShapeRequirements::default().unmet(&m).is_empty());
}

#[test]
fn a_four_to_one_ellipse_fails_on_convexity() {
    let ellipse = mask(|x, y| (x / 180.0).powi(2) + (y / 45.0).powi(2) < 1.0);
    let m = measure_shape(&ellipse, CELL_M, COARSE_M);
    let reasons = ShapeRequirements::default().unmet(&m);
    assert!(
        reasons.iter().any(|r| r.contains("convexity")),
        "{reasons:?}"
    );
}

#[test]
fn an_indented_coast_passes() {
    // Six lobes and six bays: r(θ) = R (0.62 + 0.38 cos 6θ), roughened.
    let star = mask(|x, y| {
        let theta = y.atan2(x);
        let r = 160.0 * (0.62 + 0.38 * (6.0 * theta).cos()) + 4.0 * (23.0 * theta).sin();
        x.hypot(y) < r
    });
    let m = measure_shape(&star, CELL_M, COARSE_M);
    let reasons = ShapeRequirements::default().unmet(&m);
    assert!(reasons.is_empty(), "{reasons:?} {m:?}");
    assert!(m.major_bays >= 6 && m.major_headlands >= 3, "{m:?}");
}

#[test]
fn empty_land_measures_zero() {
    let m = measure_shape(&mask(|_, _| false), CELL_M, COARSE_M);
    assert_eq!(m.area_m2, 0.0);
}
