//! Placing the founders' estate on the island (Phase 3 Task 4).
//!
//! Upstream scores every land cell for the founders' home from the
//! year-round climate: a comfortable temperature, a small seasonal swing
//! (from the canon's obliquity), adequate rain, and a mild penalty for
//! height. The island does the same over its medium cells, with the extra
//! constraints the island needs: the estate is a 2 × 2 block of medium cells
//! (the 4 km high-detail patch), all dry, non-volcanic, not a lake or river,
//! with a perennial river within 1 km of the block (the estate's water
//! and, in Phase 4b, its micro-hydro supply).
//!
//! The founders' inventory is instantiated by upstream
//! `PropertySystem::new(Some(location))`; nothing is copied.

use mk_core::biomes::BiomeType;
use mk_core::canon::CanonLocked;
use mk_island::{DomainLevel, EstatePatchConfig, IslandDomain, LocalPatchSpec};

use super::ecology::{cell_climate, RegionalEcologyState};
use super::estate_layout::{layout_estate, EstateLayout, EstateLayoutError};
use super::hydrology::discharge_m3_s;
use super::physical::RegionalPhysicalState;
use crate::climate::seasonal_temperature_amplitude;
use crate::organisms::property::{PropertySystem, StarterProperty};

/// Annual-mean temperature (K) the scoring prefers, as upstream's
/// `ESTATE_IDEAL_TEMP_K`.
const IDEAL_TEMPERATURE_K: f64 = 288.0;
/// Smallest discharge (m³/s) of a river the estate may draw on. Phase 4b
/// sizes the micro-hydro plant against the real flow and environmental
/// flow; until then a river must at least be one the model calls a river.
pub const ESTATE_MIN_RIVER_DISCHARGE_M3_S: f64 = 1.0;
/// Reach (m) from the block to a river cell's centre.
const RIVER_REACH_M: f64 = 1_000.0;
/// Sites tried, best first, before giving up.
const MAX_SITES_TRIED: usize = 64;

#[derive(Debug, Clone, PartialEq)]
pub enum EstatePatchError {
    /// `extent_m` must be twice the medium cell size.
    ExtentIsNotTwoMediumCells {
        extent_m: f64,
        two_cells_m: f64,
    },
    /// The block runs off the domain.
    BlockOutsideDomain,
    Patch(String),
}

impl std::fmt::Display for EstatePatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ExtentIsNotTwoMediumCells {
                extent_m,
                two_cells_m,
            } => write!(f, "estate patch extent {extent_m} m is not {two_cells_m} m"),
            Self::BlockOutsideDomain => write!(f, "the estate block lies outside the domain"),
            Self::Patch(why) => write!(f, "{why}"),
        }
    }
}

impl std::error::Error for EstatePatchError {}

#[derive(Debug, Clone, PartialEq)]
pub enum PlaceEstateError {
    /// No medium-cell block qualifies.
    NoEstateSite,
    Patch(EstatePatchError),
    Layout(EstateLayoutError),
}

impl std::fmt::Display for PlaceEstateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoEstateSite => {
                write!(f, "the island has no valid site for the founders' estate")
            }
            Self::Patch(e) => write!(f, "{e}"),
            Self::Layout(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for PlaceEstateError {}

/// The 2 × 2 medium-cell block whose south-west cell is `cell`, as the
/// high-detail patch. `config.extent_m` must be twice the medium cell size.
pub fn estate_patch_spec(
    domain: &IslandDomain,
    cell: (usize, usize),
    config: &EstatePatchConfig,
) -> Result<LocalPatchSpec, EstatePatchError> {
    let medium = DomainLevel::Medium;
    let size = domain.cell_size_m(medium);
    if (config.extent_m - 2.0 * size).abs() > 1e-9 {
        return Err(EstatePatchError::ExtentIsNotTwoMediumCells {
            extent_m: config.extent_m,
            two_cells_m: 2.0 * size,
        });
    }
    let (row, col) = cell;
    if row + 1 >= domain.rows(medium) || col + 1 >= domain.cols(medium) {
        return Err(EstatePatchError::BlockOutsideDomain);
    }
    // The block's centre is the shared corner of its four cells.
    domain
        .local_patch(
            (col + 1) as f64 * size,
            (row + 1) as f64 * size,
            config.extent_m,
            config.cell_size_m,
        )
        .map_err(|e| EstatePatchError::Patch(e.to_string()))
}

/// Upstream's founders' property at `location`, with every building, room,
/// vehicle, tool, computer and account upstream defines.
pub fn bootstrap_regional_property(location: (usize, usize)) -> PropertySystem {
    PropertySystem::new(Some(location))
}

/// A cell the estate may stand on: dry, not a lake or river, not volcanic,
/// not ice or bare alpine ground.
fn standable(
    physical: &RegionalPhysicalState,
    ecology: &RegionalEcologyState,
    row: usize,
    col: usize,
) -> bool {
    *physical.geophysics.elevation_m.get(row, col) > 0.0
        && physical.flow_network().lake_depth_m(row, col) == 0.0
        && !matches!(
            ecology.biome_grid.get(row, col),
            BiomeType::Volcanic
                | BiomeType::Alpine
                | BiomeType::IceSheet
                | BiomeType::River
                | BiomeType::Wetland
                | BiomeType::CoastalWaters
        )
}

/// Every qualifying block, best first: `(score, row, col)` of the block's
/// south-west cell. Ties break on the lowest `(row, col)`.
pub fn ranked_estate_sites(
    canon: &CanonLocked,
    physical: &RegionalPhysicalState,
    ecology: &RegionalEcologyState,
    domain: &IslandDomain,
) -> Vec<(f64, usize, usize)> {
    let medium = DomainLevel::Medium;
    let (rows, cols) = (domain.rows(medium), domain.cols(medium));
    let max_rain = physical
        .climatology
        .precipitation_mm_day
        .iter()
        .copied()
        .fold(0.0_f64, f64::max);
    let obliquity = canon.obliquity_deg.to_radians();
    let elevation = &physical.geophysics.elevation_m;

    let mut sites = Vec::new();
    for row in 0..rows.saturating_sub(1) {
        for col in 0..cols.saturating_sub(1) {
            let block = [
                (row, col),
                (row, col + 1),
                (row + 1, col),
                (row + 1, col + 1),
            ];
            if !block
                .iter()
                .all(|&(r, c)| standable(physical, ecology, r, c))
            {
                continue;
            }
            // A perennial river within reach of the block: a river cell in
            // the ring around it (adjacent cells' centres are 1 km away).
            let river_near = (row.saturating_sub(1)..=(row + 2).min(rows - 1))
                .flat_map(|r| {
                    (col.saturating_sub(1)..=(col + 2).min(cols - 1)).map(move |c| (r, c))
                })
                .filter(|cell| !block.contains(cell))
                .any(|(r, c)| {
                    let (bx, by) = domain.cell_center_m(medium, row, col);
                    let (x, y) = domain.cell_center_m(medium, r, c);
                    // Distance from the block's footprint to the cell centre.
                    let size = domain.cell_size_m(medium);
                    let dx = (x - (bx + 0.5 * size)).abs() - size;
                    let dy = (y - (by + 0.5 * size)).abs() - size;
                    dx.max(0.0).hypot(dy.max(0.0)) <= RIVER_REACH_M + 1e-9
                        && discharge_m3_s(&physical.hydrology, domain, r, c)
                            >= ESTATE_MIN_RIVER_DISCHARGE_M3_S
                });
            if !river_near {
                continue;
            }
            let (mut temp, mut rain, mut height) = (0.0, 0.0, 0.0);
            for &(r, c) in &block {
                let (t, p) = cell_climate(domain, physical, r, c);
                temp += t / 4.0;
                rain += p / 4.0;
                height += elevation.get(r, c) / 4.0;
            }
            let lat = domain.latitude_rad_for_row(medium, row);
            let temperature_comfort =
                1.0 - ((temp - IDEAL_TEMPERATURE_K).abs() / 25.0).clamp(0.0, 1.0);
            let season_comfort =
                1.0 - (seasonal_temperature_amplitude(obliquity, lat) / 15.0).clamp(0.0, 1.0);
            let wet = if max_rain > 0.0 {
                (rain / max_rain).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let penalty = (height / 2_000.0).clamp(0.0, 1.0);
            let score =
                0.45 * temperature_comfort + 0.35 * season_comfort + 0.20 * wet - 0.25 * penalty;
            sites.push((score, row, col));
        }
    }
    sites.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));
    sites
}

/// The best-scoring estate site (south-west medium cell of the block), or
/// `None` when no block qualifies (an all-ocean island).
pub fn choose_regional_estate_location(
    canon: &CanonLocked,
    physical: &RegionalPhysicalState,
    ecology: &RegionalEcologyState,
    domain: &IslandDomain,
) -> Option<(usize, usize)> {
    ranked_estate_sites(canon, physical, ecology, domain)
        .first()
        .map(|&(_, row, col)| (row, col))
}

/// The founders' estate placed and laid out.
#[derive(Debug, Clone)]
pub struct PlacedEstate {
    pub location: (usize, usize),
    pub patch: LocalPatchSpec,
    pub property: PropertySystem,
    pub layout: EstateLayout,
}

/// Choose the best site where the buildings also fit on the patch's
/// buildable ground, then lay the estate out.
pub fn place_regional_estate(
    canon: &CanonLocked,
    physical: &RegionalPhysicalState,
    ecology: &RegionalEcologyState,
    domain: &IslandDomain,
    config: &EstatePatchConfig,
    seed: [u8; 32],
) -> Result<PlacedEstate, PlaceEstateError> {
    let sites = ranked_estate_sites(canon, physical, ecology, domain);
    if sites.is_empty() {
        return Err(PlaceEstateError::NoEstateSite);
    }
    let mut last = PlaceEstateError::NoEstateSite;
    for &(_, row, col) in sites.iter().take(MAX_SITES_TRIED) {
        let patch =
            estate_patch_spec(domain, (row, col), config).map_err(PlaceEstateError::Patch)?;
        let property = bootstrap_regional_property((row, col));
        let founders: &StarterProperty = property
            .properties
            .iter()
            .find(|p| p.owner_agent_ids.iter().any(|id| id == "Gem-D"))
            .expect("the founders' estate exists when a location is given");
        match layout_estate(founders, physical, domain, &patch, seed) {
            Ok(layout) => {
                return Ok(PlacedEstate {
                    location: (row, col),
                    patch,
                    property,
                    layout,
                })
            }
            Err(e) => last = PlaceEstateError::Layout(e),
        }
    }
    Err(last)
}
