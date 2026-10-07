//! Crossing grid levels (Phase 2 Task 3). Climate, weather and ocean run
//! on `Coarse`; hydrology and tides on `Medium`. Fields cross levels only
//! through these two functions: bilinear interpolation going finer, and
//! the area mean going coarser, which conserves the area integral exactly.

use mk_core::grid::Grid2;
use mk_island::{DomainLevel, IslandDomain};

/// Medium cells per coarse cell along one side. The domain guarantees the
/// coarse cell is a whole multiple of the medium cell.
fn ratio(domain: &IslandDomain) -> usize {
    let r = domain.cell_size_m(DomainLevel::Coarse) / domain.cell_size_m(DomainLevel::Medium);
    (r.round() as usize).max(1)
}

/// A coarse field on the medium grid, bilinear between coarse cell
/// centres and held constant beyond the outermost ones (so no value
/// outside the coarse range appears).
pub fn resample_coarse_to_medium(coarse: &Grid2<f64>, domain: &IslandDomain) -> Grid2<f64> {
    let medium = DomainLevel::Medium;
    let (rows, cols) = (domain.rows(medium), domain.cols(medium));
    let (cr, cc) = (coarse.nlat(), coarse.nlon());
    let size = domain.cell_size_m(DomainLevel::Coarse);
    let axis = |centre_m: f64, n: usize| -> (usize, usize, f64) {
        let x = (centre_m / size - 0.5).clamp(0.0, (n - 1) as f64);
        let i = (x.floor() as usize).min(n - 1);
        (i, (i + 1).min(n - 1), x - i as f64)
    };
    let col_axis: Vec<_> = (0..cols)
        .map(|c| axis(domain.cell_center_m(medium, 0, c).0, cc))
        .collect();
    let mut data = Vec::with_capacity(rows * cols);
    for r in 0..rows {
        let (r0, r1, fr) = axis(domain.cell_center_m(medium, r, 0).1, cr);
        for &(c0, c1, fc) in &col_axis {
            let lo = coarse.get(r0, c0) * (1.0 - fc) + coarse.get(r0, c1) * fc;
            let hi = coarse.get(r1, c0) * (1.0 - fc) + coarse.get(r1, c1) * fc;
            data.push(lo * (1.0 - fr) + hi * fr);
        }
    }
    Grid2::from_data(&domain.storage_spec(medium), data)
}

/// The value at medium cell `(row, col)`'s centre of a coarse field given
/// as `get(coarse_row, coarse_col)`, bilinear exactly as
/// [`resample_coarse_to_medium`] — without building the medium grid. The
/// land-only medium processes use this: they read ~70,000 cells, not
/// 1.15 million.
pub fn sample_coarse_at_medium(
    get: impl Fn(usize, usize) -> f64,
    domain: &IslandDomain,
    row: usize,
    col: usize,
) -> f64 {
    let size = domain.cell_size_m(DomainLevel::Coarse);
    let (rows, cols) = (
        domain.rows(DomainLevel::Coarse),
        domain.cols(DomainLevel::Coarse),
    );
    let (x, y) = domain.cell_center_m(DomainLevel::Medium, row, col);
    let axis = |centre_m: f64, n: usize| {
        let v = (centre_m / size - 0.5).clamp(0.0, (n - 1) as f64);
        let i = (v.floor() as usize).min(n - 1);
        (i, (i + 1).min(n - 1), v - i as f64)
    };
    let (r0, r1, fr) = axis(y, rows);
    let (c0, c1, fc) = axis(x, cols);
    let lo = get(r0, c0) * (1.0 - fc) + get(r0, c1) * fc;
    let hi = get(r1, c0) * (1.0 - fc) + get(r1, c1) * fc;
    lo * (1.0 - fr) + hi * fr
}

/// A medium field on the coarse grid as the mean over each coarse cell's
/// medium cells. Flat equal areas make `Σ coarse·coarse_area` equal
/// `Σ medium·medium_area` exactly (to rounding).
pub fn aggregate_medium_to_coarse(medium: &Grid2<f64>, domain: &IslandDomain) -> Grid2<f64> {
    let coarse = DomainLevel::Coarse;
    let (rows, cols) = (domain.rows(coarse), domain.cols(coarse));
    let k = ratio(domain);
    let mut sums = vec![0.0; rows * cols];
    let mut counts = vec![0u32; rows * cols];
    for r in 0..medium.nlat() {
        let cr = (r / k).min(rows - 1);
        for c in 0..medium.nlon() {
            let cc = (c / k).min(cols - 1);
            sums[cr * cols + cc] += medium.get(r, c);
            counts[cr * cols + cc] += 1;
        }
    }
    let data = sums
        .into_iter()
        .zip(counts)
        .map(|(s, n)| if n > 0 { s / f64::from(n) } else { 0.0 })
        .collect();
    Grid2::from_data(&domain.storage_spec(coarse), data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mk_island::IslandProfile;

    fn domains() -> Vec<IslandDomain> {
        [
            IslandProfile::test_small(),
            IslandProfile::default_nz_scale(),
        ]
        .into_iter()
        .map(|p| IslandDomain::from_profile(p).unwrap())
        .collect()
    }

    #[test]
    fn point_sampling_matches_the_resampled_grid() {
        let d = &domains()[0];
        let c = DomainLevel::Coarse;
        let coarse = Grid2::from_data(
            &d.storage_spec(c),
            (0..d.rows(c) * d.cols(c))
                .map(|i| ((i * 7919) % 97) as f64)
                .collect(),
        );
        let medium = resample_coarse_to_medium(&coarse, d);
        for (r, col) in [(0, 0), (13, 77), (100, 150), (191, 239)] {
            let got = sample_coarse_at_medium(|a, b| *coarse.get(a, b), d, r, col);
            assert!((got - medium.get(r, col)).abs() < 1e-12);
        }
    }

    #[test]
    fn levels_nest_exactly() {
        for d in domains() {
            let k = ratio(&d);
            assert_eq!(d.rows(DomainLevel::Medium), k * d.rows(DomainLevel::Coarse));
            assert_eq!(d.cols(DomainLevel::Medium), k * d.cols(DomainLevel::Coarse));
        }
    }

    #[test]
    fn a_linear_field_resamples_exactly_inside_the_coarse_centres() {
        let d = &domains()[0];
        let c = DomainLevel::Coarse;
        let f = |x: f64, y: f64| 2.0 * x + 3.0 * y;
        let coarse = Grid2::from_data(
            &d.storage_spec(c),
            (0..d.rows(c))
                .flat_map(|r| (0..d.cols(c)).map(move |col| (r, col)))
                .map(|(r, col)| {
                    let (x, y) = d.cell_center_m(c, r, col);
                    f(x, y)
                })
                .collect(),
        );
        let medium = resample_coarse_to_medium(&coarse, d);
        let size = d.cell_size_m(c);
        let m = DomainLevel::Medium;
        for r in 0..d.rows(m) {
            for col in 0..d.cols(m) {
                let (x, y) = d.cell_center_m(m, r, col);
                let inside = x >= 0.5 * size
                    && y >= 0.5 * size
                    && x <= d.profile().width_m - 0.5 * size
                    && y <= d.profile().height_m - 0.5 * size;
                if inside {
                    let want = f(x, y);
                    assert!((medium.get(r, col) - want).abs() < 1e-6 * want.abs().max(1.0));
                }
            }
        }
    }

    #[test]
    fn coarse_to_medium_to_coarse_round_trips_a_smooth_field() {
        let d = &domains()[0];
        let c = DomainLevel::Coarse;
        let coarse = Grid2::from_data(
            &d.storage_spec(c),
            (0..d.rows(c) * d.cols(c))
                .map(|i| ((i % d.cols(c)) as f64 * 0.2).sin() + (i / d.cols(c)) as f64 * 0.1)
                .collect(),
        );
        let back = aggregate_medium_to_coarse(&resample_coarse_to_medium(&coarse, d), d);
        let worst = coarse
            .data()
            .iter()
            .zip(back.data())
            .map(|(a, b)| (a - b).abs())
            .fold(0.0, f64::max);
        assert!(worst < 0.05, "worst round-trip error {worst}");
    }

    #[test]
    fn medium_to_coarse_conserves_the_area_integral() {
        for d in domains() {
            let m = DomainLevel::Medium;
            let medium = Grid2::from_data(
                &d.storage_spec(m),
                (0..d.rows(m) * d.cols(m))
                    .map(|i| ((i * 2_654_435_761) % 1000) as f64)
                    .collect(),
            );
            let coarse = aggregate_medium_to_coarse(&medium, &d);
            let medium_total: f64 = medium.data().iter().sum::<f64>() * d.cell_area_m2(m);
            let coarse_total: f64 =
                coarse.data().iter().sum::<f64>() * d.cell_area_m2(DomainLevel::Coarse);
            assert!(
                (medium_total - coarse_total).abs() <= 1e-9 * medium_total.abs(),
                "{medium_total} vs {coarse_total}"
            );
        }
    }
}
