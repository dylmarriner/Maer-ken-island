//! One island from regional geophysics (Phase 1 Task 4).
//!
//! Tectonics and volcanism on the coarse grid, raw terrain on the medium
//! grid, then sea level fitted so the largest connected landmass meets the
//! area target inside the ocean buffer. Every other piece of land is
//! submerged, and the island's outline must meet the profile's shape
//! requirements.

use std::fmt;

use mk_core::canon::CanonLocked;
use mk_core::grid::Grid2;
use mk_core::rng::RngRegistry;
use mk_island::{DomainLevel, IslandDomain, RegionalBoundaryState, ShapeMetrics};
use serde::{Deserialize, Serialize};

use super::deposits::{generate_primary_deposits, MineralDeposit};
use super::geology::{generate_lithology, place_basement, Basement, Lithology};
use super::shape::{label_components, measure_shape};
use super::tectonics::{regional_plates, step_regional_tectonics, RegionalPlate};
use super::terrain::{fbm, raw_elevation, terrain_key, GeologicalSetting, Volcano};
use super::volcanism::step_regional_volcanism;
use crate::tectonics::TectonicsState;
use crate::volcanism::{VolcanismState, INITIAL_DEGASSED_FRACTION};

/// Depth (m) below sea level forced on edge-buffer cells: they stay open
/// ocean whatever the terrain.
const EDGE_BAND_MIN_DEPTH_M: f64 = 50.0;
/// Width (fraction of the smaller domain side) inside the ocean buffer over
/// which a ceiling holds the ground below sea level, so coasts end
/// naturally rather than against the buffer's straight edge.
const EDGE_TAPER_FRACTION: f64 = 0.15;
/// The ceiling relative to the fitted sea level (m): this deep at the
/// buffer's inner edge, rising quadratically to [`EDGE_CEILING_TOP_M`]
/// across the taper. A ceiling, not a subtraction: seafloor below it keeps
/// its depth.
const EDGE_CEILING_FLOOR_M: f64 = -300.0;
const EDGE_CEILING_TOP_M: f64 = 9_000.0;
/// Depth (m) of land cut off from the main island and submerged: shoals.
const SUBMERGED_SATELLITE_DEPTH_M: f64 = 2.0;
/// Bisection steps of the sea-level fit (the bracket starts at the full
/// elevation range, ~10 km; 60 halvings reach far below a millimetre).
const FIT_ITERATIONS: usize = 60;

/// The region's solid-earth state at the start of a world.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegionalGeophysics {
    pub tectonics: TectonicsState,
    pub volcanism: VolcanismState,
    pub plates: Vec<RegionalPlate>,
    /// Medium-grid elevation relative to sea level (m): land > 0.
    pub elevation_m: Grid2<f64>,
    /// Medium-grid water depth below sea level (m, negative over the sea,
    /// 0 on land).
    pub bathymetry_m: Grid2<f64>,
    pub land_mask: Grid2<bool>,
    pub land_area_m2: f64,
    /// How far sea level was raised above the raw terrain's datum (m).
    pub sea_level_offset_m: f64,
    pub shape: ShapeMetrics,
    pub volcanoes: Vec<Volcano>,
    /// The ancient craton fragment, if the profile has one.
    pub basement: Option<Basement>,
    /// Medium-grid rock types.
    pub lithology: Grid2<Lithology>,
    /// Primary mineral deposits, ordered by cell then kind.
    pub deposits: Vec<MineralDeposit>,
}

/// Why a seed cannot make a valid island.
#[derive(Debug, Clone, PartialEq)]
pub enum RegionalGeophysicsError {
    /// No sea level gives a primary landmass inside the area window.
    AreaUnreachable { best_area_m2: f64, target_m2: f64 },
    /// The island's outline fails the profile's shape requirements.
    ShapeRequirementsUnmet {
        metrics: ShapeMetrics,
        reasons: Vec<String>,
    },
}

impl fmt::Display for RegionalGeophysicsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AreaUnreachable {
                best_area_m2,
                target_m2,
            } => write!(
                f,
                "no sea level gives one island of {:.0} km² (closest {:.0} km²)",
                target_m2 / 1e6,
                best_area_m2 / 1e6
            ),
            Self::ShapeRequirementsUnmet { reasons, .. } => {
                write!(f, "island shape fails: {}", reasons.join("; "))
            }
        }
    }
}

impl std::error::Error for RegionalGeophysicsError {}

/// The largest 4-connected landmass (`elevation > 0`); ties go to the one
/// whose first cell comes first in row-major order.
/// Steepest ground an ordinary footing goes on, as a gradient (rise over
/// run) between a cell and a land neighbour.
///
/// One in three — about 18° — is where ground is conventionally classed
/// very steep and building stops being a footing on a slope and becomes
/// engineered terracing or piling. It is a *planning* threshold, not a
/// physical limit: dry soil stands much steeper than this (its angle of
/// repose is nearer 30–35°, a gradient of 0.58–0.70), so this is about what
/// can be built on cheaply, not what stands up.
///
/// This replaces `crate::physics::MAX_CLIMB_HEIGHT_M` for the island, and
/// is not a matter of taste. That constant is an **absolute** 2 m of relief
/// between neighbouring cells, ported from a game whose cells are small and
/// carrying its own note that it "may need tuning against Maer-Ken's actual
/// elevation scale". The island's medium cells are 2 km apart, so 2 m of
/// relief is a gradient of 0.1% — flatter than a drainage ditch. Measured
/// across the default island it admits **0 of 66,116 land cells**: not
/// "almost nothing", nothing at all, including the estate's own cell at
/// 8.6%, where the founders' house already stands.
pub const MAX_BUILD_GRADIENT: f64 = 1.0 / 3.0;

/// Whether a medium cell is gentle enough to build on.
///
/// A gradient rather than a height difference, because the answer has to
/// survive a change of resolution: the same hillside is 2 m per cell on one
/// grid and 200 m per cell on another, and only the ratio is the hillside.
///
/// # What this cannot tell you
///
/// At 2 km per cell this is a *mean* gradient across kilometres of ground,
/// so it cannot judge a building plot — a 15% cell holds flat benches and
/// steep faces, and this sees neither. What it can honestly do is exclude
/// mountainside, and that is all it claims. On the default island it admits
/// 99.9% of land, which is not the gate being lax: the island really is
/// gentle at this scale, with a steepest land gradient of 42%. A rule that
/// turned down ordinary ground to look strict would be the lie, not this.
///
/// Sea neighbours are skipped rather than counted as a drop to the sea
/// floor, which would make every coast unbuildable — the coast is where
/// people build.
pub fn is_buildable_cell(
    elevation_m: &Grid2<f64>,
    land_mask: &Grid2<bool>,
    cell_size_m: f64,
    row: usize,
    col: usize,
) -> bool {
    let (rows, cols) = (elevation_m.nlat(), elevation_m.nlon());
    if row >= rows || col >= cols || !*land_mask.get(row, col) || cell_size_m <= 0.0 {
        return false;
    }
    let here = *elevation_m.get(row, col);
    [(0i64, 1i64), (0, -1), (1, 0), (-1, 0)]
        .iter()
        .all(|(drow, dcol)| {
            let (r, c) = (row as i64 + drow, col as i64 + dcol);
            if r < 0 || c < 0 || r >= rows as i64 || c >= cols as i64 {
                return true;
            }
            let (r, c) = (r as usize, c as usize);
            if !*land_mask.get(r, c) {
                return true;
            }
            (here - *elevation_m.get(r, c)).abs() / cell_size_m <= MAX_BUILD_GRADIENT
        })
}

pub fn primary_land_component(elevation_m: &Grid2<f64>) -> Grid2<bool> {
    let (rows, cols) = (elevation_m.nlat(), elevation_m.nlon());
    let land: Vec<bool> = elevation_m.data().iter().map(|&e| e > 0.0).collect();
    let (labels, sizes) = label_components(&land, rows, cols);
    let best = sizes
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.cmp(b.1).then(b.0.cmp(&a.0)))
        .map(|(id, _)| id);
    let mask = labels
        .iter()
        .map(|&l| best.is_some_and(|b| l == b))
        .collect();
    Grid2::from_data(&elevation_m.spec(), mask)
}

/// Raw terrain minus `level`, under the edge ceiling, with the edge band
/// pushed under the sea.
fn relative(raw: &[f64], band: &[bool], ceiling: &[f64], level: f64) -> Vec<f64> {
    raw.iter()
        .zip(band)
        .zip(ceiling)
        .map(|((&e, &b), &cap)| {
            let rel = (e - level).min(cap);
            if b {
                rel.min(-EDGE_BAND_MIN_DEPTH_M)
            } else {
                rel
            }
        })
        .collect()
}

/// The edge ceiling of every medium cell, relative to sea level. The
/// distance to the edge is perturbed by up to ±25% of the taper width so a
/// range running toward the edge ends raggedly, not along a straight line.
fn edge_ceiling(domain: &IslandDomain, key: u64) -> Vec<f64> {
    let medium = DomainLevel::Medium;
    let p = domain.profile();
    let taper_m = EDGE_TAPER_FRACTION * p.width_m.min(p.height_m);
    (0..domain.rows(medium))
        .flat_map(|r| (0..domain.cols(medium)).map(move |c| (r, c)))
        .map(|(r, c)| {
            let (x, y) = domain.cell_center_m(medium, r, c);
            let edge = x.min(y).min(p.width_m - x).min(p.height_m - y)
                + 0.25 * taper_m * fbm(key ^ 0xED6E, x, y, 0.5 * taper_m, 4);
            let t = ((edge - p.minimum_ocean_buffer_m) / taper_m).clamp(0.0, 1.0);
            EDGE_CEILING_FLOOR_M + (EDGE_CEILING_TOP_M - EDGE_CEILING_FLOOR_M) * t * t
        })
        .collect()
}

fn primary_area_cells(rel: &[f64], rows: usize, cols: usize) -> usize {
    let land: Vec<bool> = rel.iter().map(|&e| e > 0.0).collect();
    label_components(&land, rows, cols)
        .1
        .into_iter()
        .max()
        .unwrap_or(0)
}

/// Raises or lowers sea level until the largest landmass covers
/// `target_land_area_m2 ± tolerance`, keeps the `minimum_ocean_buffer_m`
/// band open ocean, and submerges every other piece of land. Returns the
/// elevation relative to the new sea level and the sea-level offset.
pub fn fit_sea_level_to_target(
    raw_elevation_m: &Grid2<f64>,
    domain: &IslandDomain,
    target_land_area_m2: f64,
    tolerance_fraction: f64,
    _minimum_ocean_buffer_m: f64,
    edge_key: u64,
) -> Result<(Grid2<f64>, f64), RegionalGeophysicsError> {
    // The buffer width itself comes from the domain's profile, which the
    // domain has already validated; the parameter mirrors the plan's
    // interface.
    let level_grid = DomainLevel::Medium;
    let (rows, cols) = (raw_elevation_m.nlat(), raw_elevation_m.nlon());
    let cell_area = domain.cell_area_m2(level_grid);
    let band: Vec<bool> = (0..rows)
        .flat_map(|r| (0..cols).map(move |c| (r, c)))
        .map(|(r, c)| domain.is_edge_buffer_cell(level_grid, r, c))
        .collect();
    let raw = raw_elevation_m.data();
    let ceiling = edge_ceiling(domain, edge_key);
    let (lo_target, hi_target) = (
        target_land_area_m2 * (1.0 - tolerance_fraction),
        target_land_area_m2 * (1.0 + tolerance_fraction),
    );
    let area_at = |level: f64| {
        primary_area_cells(&relative(raw, &band, &ceiling, level), rows, cols) as f64 * cell_area
    };

    let (mut lo, mut hi) = raw
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), &e| {
            (a.min(e), b.max(e))
        });
    let mut best = (f64::INFINITY, lo);
    let mut found = None;
    for _ in 0..FIT_ITERATIONS {
        let mid = 0.5 * (lo + hi);
        let area = area_at(mid);
        let miss = (area - target_land_area_m2).abs();
        if miss < best.0 {
            best = (miss, mid);
        }
        if (lo_target..=hi_target).contains(&area) {
            found = Some(mid);
            break;
        }
        // More land than wanted: raise the sea.
        if area > target_land_area_m2 {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    let Some(level) = found else {
        let best_area = area_at(best.1);
        return Err(RegionalGeophysicsError::AreaUnreachable {
            best_area_m2: best_area,
            target_m2: target_land_area_m2,
        });
    };

    let rel = Grid2::from_data(
        &raw_elevation_m.spec(),
        relative(raw, &band, &ceiling, level),
    );
    let primary = primary_land_component(&rel);
    let fitted: Vec<f64> = rel
        .data()
        .iter()
        .zip(primary.data())
        .map(|(&e, &keep)| {
            if e > 0.0 && !keep {
                -SUBMERGED_SATELLITE_DEPTH_M
            } else {
                e
            }
        })
        .collect();
    Ok((Grid2::from_data(&raw_elevation_m.spec(), fitted), level))
}

/// Builds the region's geophysics for a seed: one island of the profile's
/// area and shape, or the reason the seed cannot make one.
pub fn bootstrap_regional_geophysics(
    canon: &CanonLocked,
    domain: &IslandDomain,
    boundaries: &RegionalBoundaryState,
    seed: [u8; 32],
) -> Result<RegionalGeophysics, RegionalGeophysicsError> {
    let geophysics = generate_regional_geophysics(canon, domain, boundaries, seed)?;
    let reasons = domain.profile().shape.unmet(&geophysics.shape);
    if !reasons.is_empty() {
        return Err(RegionalGeophysicsError::ShapeRequirementsUnmet {
            metrics: geophysics.shape,
            reasons,
        });
    }
    Ok(geophysics)
}

/// [`bootstrap_regional_geophysics`] without the shape check: the island
/// meets its area and buffer, whatever its outline. The gallery and the
/// geology tests use it to inspect every seed.
pub fn generate_regional_geophysics(
    canon: &CanonLocked,
    domain: &IslandDomain,
    boundaries: &RegionalBoundaryState,
    seed: [u8; 32],
) -> Result<RegionalGeophysics, RegionalGeophysicsError> {
    let rng = RngRegistry::new(seed);
    let tectonics = step_regional_tectonics(canon, 0, domain, &boundaries.tectonic);
    let volcanism = step_regional_volcanism(
        canon,
        0,
        domain,
        &tectonics,
        INITIAL_DEGASSED_FRACTION,
        0.0,
        &rng,
    );
    let plates = regional_plates(domain, &boundaries.tectonic);
    let key = terrain_key(&seed);
    let (raw, volcanoes) = raw_elevation(domain, &tectonics, &volcanism, &plates, key);

    let p = domain.profile();
    let (elevation_m, sea_level_offset_m) = fit_sea_level_to_target(
        &raw,
        domain,
        p.target_land_area_m2,
        p.land_area_tolerance_fraction,
        p.minimum_ocean_buffer_m,
        key,
    )?;
    let spec = elevation_m.spec();
    let land_mask = Grid2::from_data(&spec, elevation_m.data().iter().map(|&e| e > 0.0).collect());
    let bathymetry_m = Grid2::from_data(
        &spec,
        elevation_m.data().iter().map(|&e| e.min(0.0)).collect(),
    );
    let medium = DomainLevel::Medium;
    let land_area_m2 =
        land_mask.data().iter().filter(|&&l| l).count() as f64 * domain.cell_area_m2(medium);
    let shape = measure_shape(
        &land_mask,
        domain.cell_size_m(medium),
        domain.cell_size_m(DomainLevel::Coarse),
    );
    let setting = GeologicalSetting::new(domain, &tectonics, &plates, key);
    let basement = place_basement(domain, &setting, &land_mask, land_area_m2, key);
    let lithology = generate_lithology(domain, &setting, &volcanoes, &elevation_m, basement, key);
    let deposits =
        generate_primary_deposits(domain, &lithology, &elevation_m, &setting, &volcanoes, key);
    Ok(RegionalGeophysics {
        tectonics,
        volcanism,
        plates,
        elevation_m,
        bathymetry_m,
        land_mask,
        land_area_m2,
        sea_level_offset_m,
        shape,
        volcanoes,
        basement,
        lithology,
        deposits,
    })
}
