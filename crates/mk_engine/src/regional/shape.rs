//! Measuring an island's outline (Phase 1 Task 4).
//!
//! - Perimeter: length of the marching-squares coastline contour through
//!   cell centres of the box-smoothed mask (the raster staircase would
//!   inflate it by ~4/π).
//! - Compactness: `4πA/P²` (circle = 1).
//! - Convexity: land area over the area of the convex hull of the land
//!   cells' corners.
//! - Bays: connected pieces of sea inside the convex hull ("hull
//!   deficits"); headlands: pieces of land a morphological opening removes
//!   (protrusions narrower than the opening disc), the disc's radius being
//!   [`HEADLAND_OPENING_DEPTH_FACTOR`] times the median distance from the
//!   island's land to the sea, so a peninsula counts when it is thinner
//!   than the island's own body. Both count only pieces larger than
//!   `max(2 coarse cells, 1% of hull area)`.
//!
//! All passes are linear in the number of cells (two-pass chamfer distance
//! transforms, scanline hull fill), so the 1.15-million-cell medium grid
//! measures in well under a second.

use std::collections::VecDeque;

use mk_core::grid::Grid2;
use mk_island::ShapeMetrics;

/// Radius of the opening that separates headlands from the island's body,
/// in multiples of the median land-to-sea distance (for a long strip of
/// width W that median is W/4, so protrusions narrower than ~0.75 W count).
pub const HEADLAND_OPENING_DEPTH_FACTOR: f64 = 1.5;
/// Smallest major headland or bay, as a fraction of the hull area (with a
/// floor of two coarse cells): ~0.5% admits a Coromandel-sized peninsula
/// (~2,000 km²) on an NZ-sized island.
pub const MAJOR_FEATURE_HULL_FRACTION: f64 = 0.005;

/// Two-pass chamfer distance (in cells, weights 1 and √2) from every cell
/// to the nearest `target` cell. With `outside_is_target`, the space
/// beyond the grid counts as target.
pub fn chamfer_distance(
    target: &[bool],
    rows: usize,
    cols: usize,
    outside_is_target: bool,
) -> Vec<f64> {
    const D: f64 = std::f64::consts::SQRT_2;
    let mut dist = vec![f64::INFINITY; rows * cols];
    for r in 0..rows {
        for c in 0..cols {
            let i = r * cols + c;
            if target[i] {
                dist[i] = 0.0;
            } else if outside_is_target {
                let edge = (r + 1).min(c + 1).min(rows - r).min(cols - c);
                dist[i] = edge as f64;
            }
        }
    }
    for r in 0..rows {
        for c in 0..cols {
            let i = r * cols + c;
            let mut d = dist[i];
            if c > 0 {
                d = d.min(dist[i - 1] + 1.0);
            }
            if r > 0 {
                d = d.min(dist[i - cols] + 1.0);
                if c > 0 {
                    d = d.min(dist[i - cols - 1] + D);
                }
                if c + 1 < cols {
                    d = d.min(dist[i - cols + 1] + D);
                }
            }
            dist[i] = d;
        }
    }
    for r in (0..rows).rev() {
        for c in (0..cols).rev() {
            let i = r * cols + c;
            let mut d = dist[i];
            if c + 1 < cols {
                d = d.min(dist[i + 1] + 1.0);
            }
            if r + 1 < rows {
                d = d.min(dist[i + cols] + 1.0);
                if c + 1 < cols {
                    d = d.min(dist[i + cols + 1] + D);
                }
                if c > 0 {
                    d = d.min(dist[i + cols - 1] + D);
                }
            }
            dist[i] = d;
        }
    }
    dist
}

/// Sizes (in cells) of the connected components of `mask`, 4- or
/// 8-connected, each with the index of its first cell in row-major order.
/// Components are listed in order of that first cell.
pub fn component_sizes(
    mask: &[bool],
    rows: usize,
    cols: usize,
    eight: bool,
) -> Vec<(usize, usize)> {
    let mut seen = vec![false; rows * cols];
    let mut out = Vec::new();
    let mut queue = VecDeque::new();
    for start in 0..rows * cols {
        if !mask[start] || seen[start] {
            continue;
        }
        seen[start] = true;
        queue.push_back(start);
        let mut size = 0;
        while let Some(i) = queue.pop_front() {
            size += 1;
            let (r, c) = ((i / cols) as i64, (i % cols) as i64);
            for (dr, dc) in [
                (0i64, 1i64),
                (0, -1),
                (1, 0),
                (-1, 0),
                (1, 1),
                (1, -1),
                (-1, 1),
                (-1, -1),
            ]
            .iter()
            .take(if eight { 8 } else { 4 })
            {
                let (nr, nc) = (r + dr, c + dc);
                if nr < 0 || nc < 0 || nr as usize >= rows || nc as usize >= cols {
                    continue;
                }
                let j = nr as usize * cols + nc as usize;
                if mask[j] && !seen[j] {
                    seen[j] = true;
                    queue.push_back(j);
                }
            }
        }
        out.push((start, size));
    }
    out
}

/// Labels of the components of `mask` (4-connected), `usize::MAX` off the
/// mask, with each component's size by label.
pub fn label_components(mask: &[bool], rows: usize, cols: usize) -> (Vec<usize>, Vec<usize>) {
    let mut label = vec![usize::MAX; rows * cols];
    let mut sizes = Vec::new();
    let mut queue = VecDeque::new();
    for start in 0..rows * cols {
        if !mask[start] || label[start] != usize::MAX {
            continue;
        }
        let id = sizes.len();
        label[start] = id;
        queue.push_back(start);
        let mut size = 0;
        while let Some(i) = queue.pop_front() {
            size += 1;
            let (r, c) = (i / cols, i % cols);
            let mut visit = |j: usize| {
                if mask[j] && label[j] == usize::MAX {
                    label[j] = id;
                    queue.push_back(j);
                }
            };
            if c + 1 < cols {
                visit(i + 1);
            }
            if c > 0 {
                visit(i - 1);
            }
            if r + 1 < rows {
                visit(i + cols);
            }
            if r > 0 {
                visit(i - cols);
            }
        }
        sizes.push(size);
    }
    (label, sizes)
}

/// Contour length (in cells) of a land mask's coastline: marching squares
/// with linear interpolation at the 0.5 level of the mask smoothed by a 3×3
/// box (sea beyond the grid). Smoothing removes the raster staircase, so a
/// digitised circle measures within ~1% of 2πr instead of +27% (staircase)
/// or +5% (unsmoothed marching squares).
fn marching_squares_length(land: &[bool], rows: usize, cols: usize) -> f64 {
    let raw = |r: i64, c: i64| -> f64 {
        if r >= 0
            && c >= 0
            && (r as usize) < rows
            && (c as usize) < cols
            && land[r as usize * cols + c as usize]
        {
            1.0
        } else {
            0.0
        }
    };
    // Smoothed field on a grid padded by two cells on every side.
    let (pr, pc) = (rows + 4, cols + 4);
    let mut field = vec![0.0; pr * pc];
    for r in 0..pr {
        for c in 0..pc {
            let (gr, gc) = (r as i64 - 2, c as i64 - 2);
            let mut sum = 0.0;
            for dr in -1..=1 {
                for dc in -1..=1 {
                    sum += raw(gr + dr, gc + dc);
                }
            }
            field[r * pc + c] = sum / 9.0;
        }
    }
    let mut length = 0.0;
    for r in 0..pr - 1 {
        for c in 0..pc - 1 {
            // Corners: (x, y, value) in cell units.
            let v = [
                (0.0, 0.0, field[r * pc + c]),
                (1.0, 0.0, field[r * pc + c + 1]),
                (1.0, 1.0, field[(r + 1) * pc + c + 1]),
                (0.0, 1.0, field[(r + 1) * pc + c]),
            ];
            let mut points = [(0.0, 0.0); 4];
            let mut n = 0;
            for k in 0..4 {
                let (a, b) = (v[k], v[(k + 1) % 4]);
                if (a.2 >= 0.5) != (b.2 >= 0.5) {
                    let t = (0.5 - a.2) / (b.2 - a.2);
                    points[n] = (a.0 + t * (b.0 - a.0), a.1 + t * (b.1 - a.1));
                    n += 1;
                }
            }
            let seg = |p: (f64, f64), q: (f64, f64)| (p.0 - q.0).hypot(p.1 - q.1);
            length += match n {
                2 => seg(points[0], points[1]),
                // Saddle: pair the crossings so the segments do not cross.
                4 => seg(points[0], points[1]) + seg(points[2], points[3]),
                _ => 0.0,
            };
        }
    }
    length
}

fn cross(o: (f64, f64), a: (f64, f64), b: (f64, f64)) -> f64 {
    (a.0 - o.0) * (b.1 - o.1) - (a.1 - o.1) * (b.0 - o.0)
}

/// Convex hull (counter-clockwise) by Andrew's monotone chain.
fn convex_hull(mut points: Vec<(f64, f64)>) -> Vec<(f64, f64)> {
    points.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.total_cmp(&b.1)));
    points.dedup();
    if points.len() < 3 {
        return points;
    }
    let mut lower: Vec<(f64, f64)> = Vec::new();
    for &p in &points {
        while lower.len() >= 2 && cross(lower[lower.len() - 2], lower[lower.len() - 1], p) <= 0.0 {
            lower.pop();
        }
        lower.push(p);
    }
    let mut upper: Vec<(f64, f64)> = Vec::new();
    for &p in points.iter().rev() {
        while upper.len() >= 2 && cross(upper[upper.len() - 2], upper[upper.len() - 1], p) <= 0.0 {
            upper.pop();
        }
        upper.push(p);
    }
    lower.pop();
    upper.pop();
    lower.extend(upper);
    lower
}

fn polygon_area(poly: &[(f64, f64)]) -> f64 {
    let n = poly.len();
    (0..n)
        .map(|i| {
            let (a, b) = (poly[i], poly[(i + 1) % n]);
            a.0 * b.1 - b.0 * a.1
        })
        .sum::<f64>()
        .abs()
        / 2.0
}

/// Cells (row-major mask) whose centres lie inside a convex polygon given
/// in cell units (x = col, y = row, corners at integers).
fn inside_convex(poly: &[(f64, f64)], rows: usize, cols: usize) -> Vec<bool> {
    let mut inside = vec![false; rows * cols];
    if poly.len() < 3 {
        return inside;
    }
    for r in 0..rows {
        let y = r as f64 + 0.5;
        let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
        for i in 0..poly.len() {
            let (a, b) = (poly[i], poly[(i + 1) % poly.len()]);
            if (a.1 <= y && b.1 >= y) || (b.1 <= y && a.1 >= y) {
                let x = if (b.1 - a.1).abs() < 1e-12 {
                    lo = lo.min(a.0.min(b.0));
                    hi = hi.max(a.0.max(b.0));
                    continue;
                } else {
                    a.0 + (y - a.1) * (b.0 - a.0) / (b.1 - a.1)
                };
                lo = lo.min(x);
                hi = hi.max(x);
            }
        }
        if lo > hi {
            continue;
        }
        for c in 0..cols {
            let x = c as f64 + 0.5;
            if x >= lo && x <= hi {
                inside[r * cols + c] = true;
            }
        }
    }
    inside
}

/// The outline metrics of a land mask with square cells of `cell_m`.
pub fn measure_shape(land: &Grid2<bool>, cell_m: f64, coarse_cell_m: f64) -> ShapeMetrics {
    let (rows, cols) = (land.nlat(), land.nlon());
    let mask = land.data();
    let land_cells = mask.iter().filter(|&&l| l).count();
    let cell_area = cell_m * cell_m;
    let area_m2 = land_cells as f64 * cell_area;
    if land_cells == 0 {
        return ShapeMetrics {
            area_m2: 0.0,
            perimeter_m: 0.0,
            compactness: 0.0,
            convexity: 0.0,
            major_headlands: 0,
            major_bays: 0,
        };
    }

    let perimeter_m = marching_squares_length(mask, rows, cols) * cell_m;
    let compactness =
        4.0 * std::f64::consts::PI * area_m2 / (perimeter_m * perimeter_m).max(f64::MIN_POSITIVE);

    // Hull of the outer corners of each row's westmost and eastmost land.
    let mut corners = Vec::new();
    for r in 0..rows {
        let row = &mask[r * cols..(r + 1) * cols];
        if let (Some(first), Some(last)) =
            (row.iter().position(|&l| l), row.iter().rposition(|&l| l))
        {
            for (x, y) in [(first, r), (first, r + 1), (last + 1, r), (last + 1, r + 1)] {
                corners.push((x as f64, y as f64));
            }
        }
    }
    let hull = convex_hull(corners);
    let hull_area_m2 = polygon_area(&hull) * cell_area;
    let convexity = (area_m2 / hull_area_m2.max(f64::MIN_POSITIVE)).min(1.0);
    let threshold_cells = ((2.0 * coarse_cell_m * coarse_cell_m)
        .max(MAJOR_FEATURE_HULL_FRACTION * hull_area_m2)
        / cell_area)
        .ceil() as usize;

    // Bays: sea inside the hull.
    let inside = inside_convex(&hull, rows, cols);
    let bay_mask: Vec<bool> = inside.iter().zip(mask).map(|(&i, &l)| i && !l).collect();
    let major_bays = component_sizes(&bay_mask, rows, cols, false)
        .iter()
        .filter(|(_, size)| *size >= threshold_cells)
        .count() as u32;

    // Headlands: land removed by an opening (erode, then dilate).
    let sea: Vec<bool> = mask.iter().map(|&l| !l).collect();
    let to_sea = chamfer_distance(&sea, rows, cols, true);
    let mut depths: Vec<f64> = to_sea
        .iter()
        .zip(mask)
        .filter(|(_, &l)| l)
        .map(|(&d, _)| d)
        .collect();
    depths.sort_by(f64::total_cmp);
    let radius = (HEADLAND_OPENING_DEPTH_FACTOR * depths[depths.len() / 2]).max(1.0);
    let eroded: Vec<bool> = to_sea.iter().map(|&d| d > radius).collect();
    let to_core = chamfer_distance(&eroded, rows, cols, false);
    let protrusion: Vec<bool> = mask
        .iter()
        .zip(&to_core)
        .map(|(&l, &d)| l && d > radius)
        .collect();
    let major_headlands = component_sizes(&protrusion, rows, cols, true)
        .iter()
        .filter(|(_, size)| *size >= threshold_cells)
        .count() as u32;

    ShapeMetrics {
        area_m2,
        perimeter_m,
        compactness,
        convexity,
        major_headlands,
        major_bays,
    }
}
