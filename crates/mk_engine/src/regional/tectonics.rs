//! Regional tectonics on the island domain (Phase 1 Task 3).
//!
//! Plates are planar Voronoi regions in domain metres with warped
//! boundaries. Four far-field plates, one beyond each edge, carry the
//! boundary forcing into the domain; two core plates straddle the domain
//! centre and converge, so the region holds the convergent margin an
//! island like New Zealand owes its existence to; further interior plates
//! add ridges and transforms, including a back-arc rift behind the
//! overriding plate. Boundaries are classified from the plates'
//! relative motion across non-wrapping neighbours. Everything is a pure
//! function of the domain and the tectonic forcing (which is itself
//! seeded), so identical inputs give identical bytes.

use mk_core::canon::CanonLocked;
use mk_core::grid::Grid2;
use mk_core::time::Tick;
use mk_island::{CrustKind, DomainLevel, Edge, IslandDomain, TectonicBoundaryForcing};
use serde::{Deserialize, Serialize};

use crate::tectonics::{BoundaryType, PlateCell, PlateType, TectonicsState};

/// Interior plates besides the two core plates and the back-arc plate.
const EXTRA_INTERIOR_PLATES: usize = 2;
/// Opening rate of the back-arc rift behind the overriding plate (m/yr):
/// the Taupō Volcanic Zone extends at ~5-15 mm/yr and the Havre Trough at
/// ~15-20 mm/yr (Wallace et al. 2004, J. Geophys. Res. 109:B12406).
const BACK_ARC_OPENING_M_YR: (f64, f64) = (0.008, 0.02);
/// Relative convergence of the two core plates (m/yr): active convergent
/// margins close at ~20-80 mm/yr (DeMets et al. 2010, MORVEL; the
/// Hikurangi margin ~40-50 mm/yr).
const CORE_CONVERGENCE_M_YR: (f64, f64) = (0.03, 0.06);
/// Amplitude of the boundary warp as a fraction of the smaller domain side:
/// real plate boundaries bend over hundreds of kilometres.
const BOUNDARY_WARP_FRACTION: f64 = 0.06;

/// Surface heat flow bounds (mW/m²) for a 12 km cell average. Pollack et
/// al. (1993): continental mean 65, oceanic mean 101; cell averages near
/// ridges are capped at 200 because the 490·t^-1/2 law diverges at t → 0.
const HEAT_FLOW_RANGE_MW_M2: (f64, f64) = (20.0, 200.0);

/// One plate of the regional model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RegionalPlate {
    pub id: u8,
    pub crust: PlateType,
    pub age_ma: f64,
    /// Velocity (m/yr), east and north.
    pub velocity_east_m_yr: f64,
    pub velocity_north_m_yr: f64,
    /// Voronoi seed in domain metres (may lie outside the domain for
    /// far-field plates).
    pub seed_x_m: f64,
    pub seed_y_m: f64,
    /// The edge a far-field plate lies beyond, `None` for interior plates.
    pub far_field_edge: Option<Edge>,
}

/// Deterministic values derived from the tectonic forcing (which is seeded).
struct Stream {
    key: [u8; 32],
}

impl Stream {
    fn new(forcing: &TectonicBoundaryForcing) -> Self {
        let bytes = serde_json::to_vec(forcing).expect("forcing serialises");
        Self {
            key: *blake3::hash(&bytes).as_bytes(),
        }
    }

    fn uniform(&self, label: &str, index: u64) -> f64 {
        let mut hasher = blake3::Hasher::new_keyed(&self.key);
        hasher.update(label.as_bytes());
        hasher.update(&index.to_le_bytes());
        let word = u64::from_le_bytes(
            hasher.finalize().as_bytes()[..8]
                .try_into()
                .expect("8 bytes"),
        );
        (word >> 11) as f64 / (1u64 << 53) as f64
    }

    fn range(&self, label: &str, index: u64, (lo, hi): (f64, f64)) -> f64 {
        lo + (hi - lo) * self.uniform(label, index)
    }
}

fn crust_of(kind: CrustKind) -> PlateType {
    match kind {
        CrustKind::Oceanic => PlateType::Oceanic,
        CrustKind::Continental => PlateType::Continental,
    }
}

/// Plate ages (Ma): oceanic lithosphere is 0-180 Ma old (Müller et al.
/// 2008); continental crust at active margins is typically Phanerozoic.
const OCEANIC_AGE_MA: (f64, f64) = (10.0, 120.0);
const CONTINENTAL_AGE_MA: (f64, f64) = (100.0, 500.0);

/// The plates of the region: far-field plates from the forcing, two core
/// plates converging across the centre, and extra interior plates.
pub fn regional_plates(
    domain: &IslandDomain,
    forcing: &TectonicBoundaryForcing,
) -> Vec<RegionalPlate> {
    let s = Stream::new(forcing);
    let p = domain.profile();
    let (w, h) = (p.width_m, p.height_m);
    let (cx, cy) = (0.5 * w, 0.5 * h);
    let mut plates = Vec::new();

    for (i, far) in forcing.far_field.iter().enumerate() {
        let along = s.range("far_along", i as u64, (0.3, 0.7));
        let (x, y) = match far.edge {
            Edge::South => (along * w, -0.6 * h),
            Edge::North => (along * w, 1.6 * h),
            Edge::West => (-0.6 * w, along * h),
            Edge::East => (1.6 * w, along * h),
        };
        let crust = crust_of(far.crust);
        let age = match crust {
            PlateType::Oceanic => s.range("far_age", i as u64, OCEANIC_AGE_MA),
            PlateType::Continental => s.range("far_age", i as u64, CONTINENTAL_AGE_MA),
        };
        plates.push(RegionalPlate {
            id: plates.len() as u8,
            crust,
            age_ma: age,
            velocity_east_m_yr: far.velocity_east_m_yr,
            velocity_north_m_yr: far.velocity_north_m_yr,
            seed_x_m: x,
            seed_y_m: y,
            far_field_edge: Some(far.edge),
        });
    }

    // The core margin: two plates either side of the centre along a random
    // axis, closing on each other with some oblique (transform) motion.
    let axis = std::f64::consts::TAU * s.uniform("core_axis", 0);
    let (ax, ay) = (axis.cos(), axis.sin());
    let offset = 0.22 * w.min(h);
    let closing = s.range("core_closing", 0, CORE_CONVERGENCE_M_YR);
    let oblique = closing * s.range("core_oblique", 0, (-0.6, 0.6));
    // The overriding plate is continental; the downgoing one is oceanic
    // (subduction) or, one time in four, continental too (collision).
    let downgoing = if s.uniform("core_collision", 0) < 0.25 {
        PlateType::Continental
    } else {
        PlateType::Oceanic
    };
    for (side, crust) in [(-1.0, PlateType::Continental), (1.0, downgoing)] {
        let index = plates.len() as u64;
        // Each plate moves half the closing speed toward the other, plus
        // half the oblique slip along the margin.
        let (ve, vn) = (
            -side * 0.5 * closing * ax + side * 0.5 * oblique * -ay,
            -side * 0.5 * closing * ay + side * 0.5 * oblique * ax,
        );
        let age = match crust {
            PlateType::Oceanic => s.range("core_age", index, OCEANIC_AGE_MA),
            PlateType::Continental => s.range("core_age", index, CONTINENTAL_AGE_MA),
        };
        plates.push(RegionalPlate {
            id: index as u8,
            crust,
            age_ma: age,
            velocity_east_m_yr: ve,
            velocity_north_m_yr: vn,
            seed_x_m: cx + side * offset * ax,
            seed_y_m: cy + side * offset * ay,
            far_field_edge: None,
        });
    }

    // Behind the overriding (continental) plate, a back-arc plate rifts
    // away from it: the region's divergent boundary.
    {
        let overriding = plates[plates.len() - 2].clone();
        let index = plates.len() as u64;
        let opening = s.range("back_arc_opening", index, BACK_ARC_OPENING_M_YR);
        let distance = s.range("back_arc_distance", index, (0.40, 0.48)) * w.min(h);
        plates.push(RegionalPlate {
            id: index as u8,
            crust: PlateType::Continental,
            age_ma: s.range("back_arc_age", index, CONTINENTAL_AGE_MA),
            velocity_east_m_yr: overriding.velocity_east_m_yr - opening * ax,
            velocity_north_m_yr: overriding.velocity_north_m_yr - opening * ay,
            seed_x_m: cx - distance * ax,
            seed_y_m: cy - distance * ay,
            far_field_edge: None,
        });
    }

    for k in 0..EXTRA_INTERIOR_PLATES {
        let index = plates.len() as u64;
        let crust = if s.uniform("extra_crust", index) < 0.6 {
            PlateType::Oceanic
        } else {
            PlateType::Continental
        };
        let age = match crust {
            PlateType::Oceanic => s.range("extra_age", index, OCEANIC_AGE_MA),
            PlateType::Continental => s.range("extra_age", index, CONTINENTAL_AGE_MA),
        };
        // Interior plates sit off the core axis, nearer the edges.
        let angle = axis
            + std::f64::consts::FRAC_PI_2
            + std::f64::consts::PI * k as f64
            + s.range("extra_angle", index, (-0.5, 0.5));
        let radius = s.range("extra_radius", index, (0.30, 0.42)) * w.min(h);
        let speed = s.range("extra_speed", index, (0.01, 0.07));
        let heading = std::f64::consts::TAU * s.uniform("extra_heading", index);
        plates.push(RegionalPlate {
            id: index as u8,
            crust,
            age_ma: age,
            velocity_east_m_yr: speed * heading.cos(),
            velocity_north_m_yr: speed * heading.sin(),
            seed_x_m: cx + radius * angle.cos(),
            seed_y_m: cy + radius * angle.sin(),
            far_field_edge: None,
        });
    }
    plates
}

/// The Moore neighbourhood of a cell, clipped at the domain edges: the
/// region never wraps east-west or north-south.
pub fn regional_neighbours(
    row: usize,
    col: usize,
    rows: usize,
    cols: usize,
) -> impl Iterator<Item = (usize, usize)> {
    (-1i64..=1)
        .flat_map(|dr| (-1i64..=1).map(move |dc| (dr, dc)))
        .filter(|&(dr, dc)| dr != 0 || dc != 0)
        .filter_map(move |(dr, dc)| {
            let (r, c) = (row as i64 + dr, col as i64 + dc);
            (r >= 0 && c >= 0 && (r as usize) < rows && (c as usize) < cols)
                .then_some((r as usize, c as usize))
        })
}

/// Index of the plate owning a point: nearest seed under a smooth warp, so
/// boundaries bend instead of running straight.
fn owning_plate(plates: &[RegionalPlate], s: &Stream, x: f64, y: f64, warp_m: f64) -> usize {
    let phase = |label: &str, i: u64| std::f64::consts::TAU * s.uniform(label, i);
    let scale = 1.0 / (3.0 * warp_m.max(1.0));
    let wx = warp_m
        * ((x * scale * 0.7 + phase("wx1", 0)).sin()
            + 0.5 * (y * scale * 1.9 + phase("wx2", 0)).sin());
    let wy = warp_m
        * ((y * scale * 0.8 + phase("wy1", 0)).sin()
            + 0.5 * (x * scale * 1.7 + phase("wy2", 0)).sin());
    let (px, py) = (x + wx, y + wy);
    let mut best = (f64::INFINITY, 0);
    for (i, p) in plates.iter().enumerate() {
        let d = (px - p.seed_x_m).powi(2) + (py - p.seed_y_m).powi(2);
        if d < best.0 {
            best = (d, i);
        }
    }
    best.1
}

/// The boundary a cell lies on, from the strongest relative motion with a
/// different plate across its four edge-sharing neighbours.
fn classify(
    owner: &[usize],
    plates: &[RegionalPlate],
    rows: usize,
    cols: usize,
    row: usize,
    col: usize,
) -> Option<BoundaryType> {
    let here = &plates[owner[row * cols + col]];
    let mut strongest: Option<(f64, BoundaryType)> = None;
    for (dr, dc, ne, nn) in [
        (0i64, 1i64, 1.0, 0.0),
        (0, -1, -1.0, 0.0),
        (1, 0, 0.0, 1.0),
        (-1, 0, 0.0, -1.0),
    ] {
        let (r, c) = (row as i64 + dr, col as i64 + dc);
        if r < 0 || c < 0 || r as usize >= rows || c as usize >= cols {
            continue;
        }
        let other = &plates[owner[r as usize * cols + c as usize]];
        if other.id == here.id {
            continue;
        }
        let (re, rn) = (
            here.velocity_east_m_yr - other.velocity_east_m_yr,
            here.velocity_north_m_yr - other.velocity_north_m_yr,
        );
        let closing = re * ne + rn * nn;
        let sliding = (re * nn - rn * ne).abs();
        let kind = if sliding > closing.abs() {
            BoundaryType::Transform
        } else if closing < 0.0 {
            BoundaryType::Ridge
        } else if here.crust == PlateType::Continental && other.crust == PlateType::Continental {
            BoundaryType::Collision
        } else {
            BoundaryType::Subduction
        };
        let strength = closing.hypot(sliding);
        if strongest.is_none_or(|(best, _)| strength > best) {
            strongest = Some((strength, kind));
        }
    }
    strongest.map(|(_, kind)| kind)
}

/// Thickening of continental crust in a collision belt (km): orogens such
/// as the Alps and Southern Alps carry 45-60 km crust vs ~35 km normal.
const COLLISION_THICKENING_KM: f64 = 15.0;

/// Surface heat flow (mW/m²) of a cell. Oceanic: the plate-cooling law
/// q = 490·t^-1/2 (Stein & Stein 1992, Nature 359:123), floor 48 mW/m² for
/// old seafloor. Continental interiors ~55-65 (Pollack et al. 1993).
/// Boundaries: arcs and back-arcs run hot (~80-110), transforms ~80,
/// collision belts ~90. Clamped to the cell-average range.
fn heat_flow_mw_m2(cell: &PlateCell) -> f64 {
    let q = match (cell.boundary, cell.crust_type) {
        (Some(BoundaryType::Ridge), _) => 200.0,
        (Some(BoundaryType::Subduction), _) => 95.0,
        (Some(BoundaryType::Transform), _) => 80.0,
        (Some(BoundaryType::Collision), _) => 90.0,
        (None, PlateType::Oceanic) => (490.0 / cell.age_ma.max(1.0).sqrt()).max(48.0),
        (None, PlateType::Continental) => {
            // Older continental lithosphere is colder: ~65 for Phanerozoic,
            // ~40 for Archean cratons (Nyblade & Pollack 1993).
            if cell.age_ma > 2_500.0 {
                40.0
            } else {
                65.0 - 10.0 * (cell.age_ma / 2_500.0)
            }
        }
    };
    q.clamp(HEAT_FLOW_RANGE_MW_M2.0, HEAT_FLOW_RANGE_MW_M2.1)
}

/// The tectonic state of the region on the coarse grid. `tick` is accepted
/// for the planetary signature; the regional plate layout is fixed for the
/// spin-up horizon (it changes over millions of years, not within a run).
pub fn step_regional_tectonics(
    _canon: &CanonLocked,
    _tick: Tick,
    domain: &IslandDomain,
    forcing: &TectonicBoundaryForcing,
) -> TectonicsState {
    let plates = regional_plates(domain, forcing);
    let s = Stream::new(forcing);
    let level = DomainLevel::Coarse;
    let (rows, cols) = (domain.rows(level), domain.cols(level));
    let p = domain.profile();
    let warp = BOUNDARY_WARP_FRACTION * p.width_m.min(p.height_m);

    let owner: Vec<usize> = (0..rows)
        .flat_map(|r| (0..cols).map(move |c| (r, c)))
        .map(|(r, c)| {
            let (x, y) = domain.cell_center_m(level, r, c);
            owning_plate(&plates, &s, x, y, warp)
        })
        .collect();

    let spec = domain.storage_spec(level);
    let mut cells = Grid2::new(&spec, PlateCell::new(0, PlateType::Oceanic, 50.0));
    let mut heat = Grid2::new(&spec, 0.0);
    for r in 0..rows {
        for c in 0..cols {
            let plate = &plates[owner[r * cols + c]];
            let mut cell = PlateCell::new(plate.id, plate.crust, plate.age_ma);
            cell.boundary = classify(&owner, &plates, rows, cols, r, c);
            if cell.boundary == Some(BoundaryType::Collision) {
                cell.thickness_km += COLLISION_THICKENING_KM;
                cell.crust_thickness_km += COLLISION_THICKENING_KM;
            }
            heat.set(r, c, heat_flow_mw_m2(&cell));
            cells.set(r, c, cell);
        }
    }

    TectonicsState {
        plates: cells,
        heat_flow: heat,
        surface_relief_m: None,
        // Regional sea level is fitted to the land-area target later
        // (Task 4), not to the planet's ocean fraction.
        sea_level_m: 0.0,
    }
}
