//! Per-cell perceptual signals derived from live world state for
//! [`crate::agents::AgentWorldObservation`].
//!
//! Two observation channels used to be fixed constants for every agent and
//! human on the planet (`social_density: 0.3`, `daylight_fraction: 0.5`), so
//! no one ever perceived a crowd, solitude, dawn or night. They are computed
//! here instead:
//!
//! - **Daylight** is this cell's instantaneous top-of-atmosphere insolation
//!   as a fraction of the current global peak (the subsolar cell), read from
//!   the same [`InsolationField`] the climate step uses. `0.0` is night,
//!   `1.0` is the sun directly overhead.
//! - **Social density** counts the *other* living beings (humans and world
//!   agents) within [`SOCIAL_RADIUS_CELLS`] of the observer, saturating as
//!   `n / (n + SOCIAL_HALF_SATURATION)` so a handful of neighbours already
//!   reads as company while a crowd approaches but never exceeds `1.0`.
//!   Longitude wraps around the planet; latitude does not.
//! - **Hydration access** is drinkable water at the cell from the live
//!   [`HydrologyState`]: standing surface water (saturating at
//!   [`SURFACE_WATER_HALF_SATURATION_MM`]) dominates, soil moisture (seeps,
//!   dew, digging) contributes the rest. Ocean cells offer no drinkable
//!   water — seawater dehydrates rather than hydrates. This replaces a copy
//!   of the biome's *food* abundance, which made water exactly as available
//!   as food everywhere.

use crate::agents::GridPosition;
use crate::hydrology::HydrologyState;
use crate::insolation::InsolationField;
use mk_core::biomes::BiomeType;
use mk_core::grid::GridSpec;

/// Chebyshev radius (in grid cells) within which another being counts as
/// socially present.
pub const SOCIAL_RADIUS_CELLS: i32 = 2;
/// Neighbour count at which perceived social density reaches `0.5`.
pub const SOCIAL_HALF_SATURATION: f64 = 4.0;

/// `ClimateState` temperatures are Kelvin; `AgentWorldObservation::
/// ambient_temperature_c` and every physiology/cognition model reading it
/// are Celsius.
pub const KELVIN_TO_CELSIUS_OFFSET: f64 = 273.15;

/// Standing surface water (mm) at which it alone gives an even chance of
/// finding drinkable water in the cell.
pub const SURFACE_WATER_HALF_SATURATION_MM: f64 = 5.0;
/// Stream flow leaving the cell (mm/day of runoff over its area) at which
/// streams alone give an even chance of finding water.
pub const RUNOFF_HALF_SATURATION_MM_DAY: f64 = 0.1;
/// Rainfall (mm/day) at which rain, dew ponds and seeps alone give an even
/// chance of finding water.
pub const PRECIPITATION_HALF_SATURATION_MM_DAY: f64 = 1.0;

/// Chance, `0.0..=1.0`, of finding drinkable water in cell `(row, col)`.
///
/// Each water source is an independent chance: standing surface water,
/// streams (the cell's routed runoff), rainfall, and soil moisture (wet soil
/// means springs and seeps). The combined chance is `1 − Π(1 − pᵢ)`. A cell
/// of this grid spans hundreds of kilometres, so water is rarely absent
/// outside true desert, but a dry cell with no streams, little rain and dry
/// soil offers almost none. Aquatic cells are sea water and offer none.
pub fn hydration_access(
    hydrology: &HydrologyState,
    precipitation_mm_day: &mk_core::grid::Grid2<f64>,
    biome: Option<BiomeType>,
    row: usize,
    col: usize,
) -> f64 {
    if biome.is_some_and(|b| b.is_aquatic()) {
        return 0.0;
    }
    let surface = &hydrology.surface_water;
    let soil = &hydrology.soil_water;
    let runoff = &hydrology.runoff;
    let in_range = |nlat: usize, nlon: usize| row < nlat && col < nlon;
    if !in_range(surface.nlat(), surface.nlon())
        || !in_range(soil.nlat(), soil.nlon())
        || !in_range(runoff.nlat(), runoff.nlon())
    {
        return 0.0;
    }
    let saturating = |value: f64, half: f64| {
        let v = value.max(0.0);
        v / (v + half)
    };
    let rain = if in_range(precipitation_mm_day.nlat(), precipitation_mm_day.nlon()) {
        *precipitation_mm_day.get(row, col)
    } else {
        0.0
    };
    let chances = [
        saturating(*surface.get(row, col), SURFACE_WATER_HALF_SATURATION_MM),
        saturating(*runoff.get(row, col), RUNOFF_HALF_SATURATION_MM_DAY),
        saturating(rain, PRECIPITATION_HALF_SATURATION_MM_DAY),
        soil.get(row, col).moisture_fraction.clamp(0.0, 1.0),
    ];
    let none_found: f64 = chances.iter().map(|p| 1.0 - p).product();
    (1.0 - none_found).clamp(0.0, 1.0)
}

/// Fraction of the current peak insolation reaching cell `(row, col)`.
pub fn daylight_fraction(insolation: &InsolationField, row: usize, col: usize) -> f64 {
    let field = &insolation.toa_w_m2;
    if row >= field.nlat() || col >= field.nlon() {
        return 0.0;
    }
    let peak = field.data().iter().copied().fold(0.0_f64, f64::max);
    if peak <= 0.0 {
        return 0.0;
    }
    (*field.get(row, col) / peak).clamp(0.0, 1.0)
}

/// Count of living beings per grid cell, snapshotted once per step so every
/// observer in that step perceives the same population layout.
#[derive(Debug, Clone)]
pub struct Occupancy {
    nlat: usize,
    nlon: usize,
    counts: Vec<u32>,
}

impl Occupancy {
    pub fn new(grid_spec: &GridSpec, positions: impl IntoIterator<Item = GridPosition>) -> Self {
        let (nlat, nlon) = (grid_spec.nlat, grid_spec.nlon);
        let mut counts = vec![0u32; nlat * nlon];
        if nlat > 0 && nlon > 0 {
            for position in positions {
                let (row, col) = clamp_cell(position, nlat, nlon);
                counts[row * nlon + col] += 1;
            }
        }
        Self { nlat, nlon, counts }
    }

    /// Perceived social density for an observer standing at `position`, who
    /// is themselves one of the counted beings when `observer_counted` is
    /// true (and so must not count as their own company).
    pub fn social_density(&self, position: GridPosition, observer_counted: bool) -> f64 {
        if self.nlat == 0 || self.nlon == 0 {
            return 0.0;
        }
        let (row, col) = clamp_cell(position, self.nlat, self.nlon);
        let lon_span = (2 * SOCIAL_RADIUS_CELLS + 1).min(self.nlon as i32);
        let lon_start = col as i32 - lon_span / 2;
        let mut neighbours: u64 = 0;
        for d_row in -SOCIAL_RADIUS_CELLS..=SOCIAL_RADIUS_CELLS {
            let r = row as i32 + d_row;
            if r < 0 || r >= self.nlat as i32 {
                continue;
            }
            for offset in 0..lon_span {
                let c = (lon_start + offset).rem_euclid(self.nlon as i32) as usize;
                neighbours += u64::from(self.counts[r as usize * self.nlon + c]);
            }
        }
        if observer_counted {
            neighbours = neighbours.saturating_sub(1);
        }
        let n = neighbours as f64;
        n / (n + SOCIAL_HALF_SATURATION)
    }
}

fn clamp_cell(position: GridPosition, nlat: usize, nlon: usize) -> (usize, usize) {
    (
        position.row.clamp(0, nlat as i32 - 1) as usize,
        position.col.clamp(0, nlon as i32 - 1) as usize,
    )
}

/// Net primary production (kgC m⁻² yr⁻¹) at which a forager gathers half
/// of what it needs. Forager intake is a saturating (Holling type II)
/// function of food density: it rises steeply in sparse vegetation and
/// levels off once food is plentiful, so hunter-gatherers subsist in
/// semi-arid land (NPP ≈ 0.1–0.3) but not in true desert or on ice.
pub const FORAGE_HALF_SATURATION_NPP_KGC_M2_YR: f64 = 0.03;

/// Fraction of a human's food need that foraging at cell (`row`, `col`)
/// can meet, from the cell's live net primary production
/// (`BiosphereSystem::npp_field_kgc_m2_yr`, Miami model). The intake curve
/// is `npp / (npp + half_saturation)`. Ocean cells and cells with no
/// production return 0; a missing field (before the first biosphere step)
/// also returns 0 rather than an assumed value.
pub fn caloric_access(
    biosphere: &crate::biosphere::BiosphereSystem,
    nlon: usize,
    row: usize,
    col: usize,
) -> f64 {
    let npp = biosphere
        .npp_field_kgc_m2_yr
        .get(row * nlon + col)
        .copied()
        .unwrap_or(0.0)
        .max(0.0);
    if npp <= 0.0 {
        return 0.0;
    }
    npp / (npp + FORAGE_HALF_SATURATION_NPP_KGC_M2_YR)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mk_core::grid::Grid2;

    #[test]
    fn caloric_access_follows_primary_production() {
        // 1×4 grid: ocean, desert, grassland, rainforest.
        let biosphere = crate::biosphere::BiosphereSystem {
            npp_field_kgc_m2_yr: vec![0.0, 0.02, 0.5, 1.8],
            ..Default::default()
        };
        let access: Vec<f64> = (0..4)
            .map(|c| caloric_access(&biosphere, 4, 0, c))
            .collect();
        assert_eq!(access[0], 0.0, "no food at sea");
        assert!(access[1] < 0.5, "desert forage is scarce: {}", access[1]);
        assert!(access[2] > 0.9 && access[3] > access[2]);
        assert!(access.iter().all(|a| (0.0..1.0).contains(a)));
        // No field yet (before the first biosphere step): no assumed food.
        let empty = crate::biosphere::BiosphereSystem::default();
        assert_eq!(caloric_access(&empty, 4, 0, 2), 0.0);
    }

    fn spec(nlat: usize, nlon: usize) -> GridSpec {
        GridSpec::new(nlat, nlon)
    }

    #[test]
    fn a_lone_observer_perceives_no_company() {
        let grid = spec(10, 10);
        let occupancy = Occupancy::new(&grid, [GridPosition::new(5, 5)]);
        assert_eq!(occupancy.social_density(GridPosition::new(5, 5), true), 0.0);
    }

    #[test]
    fn nearby_beings_raise_density_and_distant_ones_do_not() {
        let grid = spec(20, 20);
        let observer = GridPosition::new(10, 10);
        let near = Occupancy::new(
            &grid,
            [
                observer,
                GridPosition::new(11, 10),
                GridPosition::new(9, 12),
            ],
        );
        let far = Occupancy::new(&grid, [observer, GridPosition::new(18, 2)]);

        assert!((near.social_density(observer, true) - 2.0 / 6.0).abs() < 1e-12);
        assert_eq!(far.social_density(observer, true), 0.0);
    }

    #[test]
    fn density_saturates_below_one() {
        let grid = spec(10, 10);
        let crowd = std::iter::repeat_n(GridPosition::new(3, 3), 500);
        let occupancy = Occupancy::new(&grid, crowd);
        let density = occupancy.social_density(GridPosition::new(3, 3), true);
        assert!(density > 0.99 && density < 1.0);
    }

    #[test]
    fn neighbours_across_the_date_line_count() {
        let grid = spec(10, 36);
        let observer = GridPosition::new(5, 0);
        let occupancy = Occupancy::new(&grid, [observer, GridPosition::new(5, 35)]);
        assert!(occupancy.social_density(observer, true) > 0.0);
    }

    #[test]
    fn daylight_is_relative_to_the_subsolar_peak() {
        let grid = spec(2, 2);
        let field = InsolationField {
            toa_w_m2: Grid2::from_data(&grid, vec![0.0, 400.0, 800.0, 1200.0]),
            band1: Grid2::from_data(&grid, vec![0.0; 4]),
            band2: Grid2::from_data(&grid, vec![0.0; 4]),
        };
        assert_eq!(daylight_fraction(&field, 0, 0), 0.0);
        assert!((daylight_fraction(&field, 1, 0) - 2.0 / 3.0).abs() < 1e-12);
        assert_eq!(daylight_fraction(&field, 1, 1), 1.0);
    }

    fn hydrology(surface_mm: f64, moisture: f64) -> HydrologyState {
        let grid = spec(1, 1);
        let mut state = HydrologyState::new(&grid, moisture);
        state.surface_water = Grid2::from_data(&grid, vec![surface_mm]);
        state.runoff = Grid2::from_data(&grid, vec![0.0]);
        state
    }

    fn rain(mm_day: f64) -> Grid2<f64> {
        Grid2::from_data(&spec(1, 1), vec![mm_day])
    }

    #[test]
    fn every_water_source_raises_hydration_access() {
        let desert = hydration_access(
            &hydrology(0.0, 0.02),
            &rain(0.01),
            Some(BiomeType::Desert),
            0,
            0,
        );
        assert!(desert < 0.05, "desert {desert}");
        let pond = hydration_access(
            &hydrology(50.0, 0.02),
            &rain(0.01),
            Some(BiomeType::Desert),
            0,
            0,
        );
        assert!(pond > 0.9, "oasis {pond}");
        let wet_forest = hydration_access(
            &hydrology(0.0, 0.8),
            &rain(3.8),
            Some(BiomeType::TropicalDryForest),
            0,
            0,
        );
        assert!(wet_forest > 0.9, "rainy forest {wet_forest}");
        let mut river = hydrology(0.0, 0.1);
        river.runoff = Grid2::from_data(&spec(1, 1), vec![2.0]);
        let with_river = hydration_access(&river, &rain(0.1), Some(BiomeType::Desert), 0, 0);
        assert!(with_river > 0.9, "a river through the desert {with_river}");
    }

    #[test]
    fn seawater_is_not_drinkable() {
        for biome in [
            BiomeType::ShallowOcean,
            BiomeType::CoastalWaters,
            BiomeType::ReefSea,
        ] {
            let sea = hydration_access(&hydrology(500.0, 1.0), &rain(5.0), Some(biome), 0, 0);
            assert_eq!(sea, 0.0);
        }
    }
}
