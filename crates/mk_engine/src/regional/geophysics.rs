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

use super::shape::{label_components, measure_shape};
use super::tectonics::{regional_plates, step_regional_tectonics, RegionalPlate};
use super::terrain::{raw_elevation, terrain_key, Volcano};
use super::volcanism::step_regional_volcanism;
use crate::tectonics::TectonicsState;
use crate::volcanism::{VolcanismState, INITIAL_DEGASSED_FRACTION};

/// Depth (m) below sea level forced on edge-buffer cells: they stay open
/// ocean whatever the terrain.
const EDGE_BAND_MIN_DEPTH_M: f64 = 50.0;
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
    /// Volcano centres, relief and radius (m).
    pub volcanoes: Vec<(f64, f64, f64, f64)>,
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

/// Raw terrain minus `level`, with the edge band pushed under the sea.
fn relative(raw: &[f64], band: &[bool], level: f64) -> Vec<f64> {
    raw.iter()
        .zip(band)
        .map(|(&e, &b)| {
            let rel = e - level;
            if b {
                rel.min(-EDGE_BAND_MIN_DEPTH_M)
            } else {
                rel
            }
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
    let (lo_target, hi_target) = (
        target_land_area_m2 * (1.0 - tolerance_fraction),
        target_land_area_m2 * (1.0 + tolerance_fraction),
    );
    let area_at = |level: f64| {
        primary_area_cells(&relative(raw, &band, level), rows, cols) as f64 * cell_area
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

    let rel = Grid2::from_data(&raw_elevation_m.spec(), relative(raw, &band, level));
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
    let (raw, volcanoes) =
        raw_elevation(domain, &tectonics, &volcanism, &plates, terrain_key(&seed));

    let p = domain.profile();
    let (elevation_m, sea_level_offset_m) = fit_sea_level_to_target(
        &raw,
        domain,
        p.target_land_area_m2,
        p.land_area_tolerance_fraction,
        p.minimum_ocean_buffer_m,
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
    let reasons = p.shape.unmet(&shape);
    if !reasons.is_empty() {
        return Err(RegionalGeophysicsError::ShapeRequirementsUnmet {
            metrics: shape,
            reasons,
        });
    }
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
        volcanoes: volcanoes
            .iter()
            .map(|v: &Volcano| (v.x_m, v.y_m, v.relief_m, v.radius_m))
            .collect(),
    })
}
