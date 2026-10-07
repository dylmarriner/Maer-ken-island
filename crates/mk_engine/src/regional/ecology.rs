//! Regional ecology (Phase 3 Task 1): biomes, primary production and
//! standing producer biomass on the medium grid, derived from the island's
//! physical state.
//!
//! Upstream classifies biomes from climate, soil moisture and elevation
//! (`mk_core::biomes::classify_biome`) and grows biomass from the Miami
//! NPP model (`biosphere::miami_npp_kgc_m2_yr`); both are pure functions
//! of their inputs, so they run here unchanged on regional grids:
//!
//! - **Biomes** from the annual-mean climatology (temperature, rain),
//!   lapse-corrected from the coarse cell's mean elevation to each medium
//!   cell, with soil moisture from the hydrology and volcanic cones from
//!   the geophysics. Two island corrections to upstream's classifier:
//!   cold dry ground between [`PERMANENT_ICE_BELOW_K`] and freezing is cold
//!   steppe or tundra, not ice (upstream calls anything under 283 K and
//!   1 mm/day an ice sheet, which on this island labelled half the land
//!   as ice at 2-7 °C); and a cell is `River` only if the river's width
//!   fills at least half of it ([`river_width_m`]), not whenever a stream
//!   crosses it (channels are tens of metres wide, cells are 2 km). Lakes
//!   are `CoastalWaters` (there is no lake biome, and this keeps land
//!   plants out).
//! - **NPP** (kgC m⁻² yr⁻¹) on land cells only.
//! - **Biomass** (kgC m⁻²) is the *distribution* of producer carbon: it
//!   starts at `NPP × residence_years(biome)`, then follows
//!   `dB/dt = NPP − B / residence − harvest − disturbance`, and
//!   [`RegionalEcologyState::renormalise_to_total`] rescales it so the
//!   land total equals the producer species' carbon.
//! - A **sparse vegetation view** (`VegetationSystem::seed_from_biomes`,
//!   about one plant per 11 cells) for the upstream plant runtime.
//!
//! Species populations are not seeded here: upstream's are planetary
//! totals, and reseeding them at island scale is the next step of Task 1
//! (see the plan's implementation notes).

use mk_core::biomes::{classify_biome, BiomeType};
use mk_core::grid::Grid2;
use mk_island::{DomainLevel, IslandDomain};

use super::climate::LAPSE_RATE_K_PER_M;
use super::hydrology::discharge_m3_s;
use super::levels::sample_coarse_at_medium;
use super::physical::RegionalPhysicalState;
use crate::biosphere::miami_npp_kgc_m2_yr;
use crate::organisms::vegetation::VegetationSystem;

/// Carbon fraction of dry matter, as upstream's `update_primary_production`.
pub const CARBON_FRACTION_OF_DRY_MASS: f64 = 0.475;
/// Days in the Earth year the Miami model's annual rain is per.
const EARTH_YEAR_DAYS: f64 = 365.25;
/// Annual-mean temperature (K) below which cold ground is permanent ice:
/// upstream's own hard limit for `IceSheet` (`classify_biome`: `< 260`).
pub const PERMANENT_ICE_BELOW_K: f64 = 260.0;
/// Cold ground warmer than this (K) is steppe shrubland, colder is tundra.
const COLD_STEPPE_ABOVE_K: f64 = 278.0;
/// A river's channel width (m) from its discharge (m³/s): Leopold &
/// Maddock (1953) hydraulic geometry, `w = 3.0 Q^0.5` for a typical river.
pub fn river_width_m(discharge_m3_s: f64) -> f64 {
    3.0 * discharge_m3_s.max(0.0).sqrt()
}

/// The island's biome from upstream's classification: dry cold ground that
/// is not frozen year-round is not an ice sheet.
fn island_biome(classified: BiomeType, temperature_k: f64) -> BiomeType {
    if classified == BiomeType::IceSheet && temperature_k >= PERMANENT_ICE_BELOW_K {
        if temperature_k < COLD_STEPPE_ABOVE_K {
            BiomeType::Tundra
        } else {
            BiomeType::Shrubland
        }
    } else {
        classified
    }
}

/// A cone's volcanic core, as a fraction of its radius.
const VOLCANIC_CORE_FRACTION: f64 = 0.35;
const SECONDS_PER_YEAR: f64 = 365.25 * 86_400.0;

#[derive(Debug, Clone, PartialEq)]
pub enum RegionalEcologyError {
    /// The physical state has no land, so there is nothing to populate.
    NoLand,
}

impl std::fmt::Display for RegionalEcologyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoLand => write!(f, "the island has no land to populate"),
        }
    }
}

impl std::error::Error for RegionalEcologyError {}

/// Years of NPP a biome's standing producer carbon amounts to (biomass ÷
/// NPP at steady state). Round estimates in the range of the biome means
/// of Whittaker & Likens (1975) and Saugier, Roy & Mooney (2001, in
/// *Terrestrial Global Productivity*); each is good to about ±40%
/// (deviation D30). Zero where producers do not stand on land.
pub fn residence_years(biome: BiomeType) -> f64 {
    use BiomeType::*;
    match biome {
        TropicalRainforest => 18.0,
        TropicalDryForest => 12.0,
        TemperateForest => 22.0,
        BorealForest => 25.0,
        MontaneForest => 20.0,
        Woodland => 10.0,
        Shrubland => 8.0,
        Savanna => 4.0,
        Grassland => 2.5,
        Tundra => 4.0,
        Alpine => 3.0,
        Desert => 5.0,
        SemiDesert => 4.0,
        Wetland => 8.0,
        Volcanic => 6.0,
        IceSheet => 1.0,
        River | DeepOcean | ShallowOcean | CoastalWaters | ReefSea => 0.0,
    }
}

/// The island's producers over the medium grid.
#[derive(Debug, Clone)]
pub struct RegionalEcologyState {
    pub biome_grid: Grid2<BiomeType>,
    /// kgC m⁻² yr⁻¹, zero off land.
    pub npp_kgc_m2_yr: Grid2<f64>,
    /// Standing producer carbon, kgC m⁻², zero off land.
    pub biomass_kgc_m2: Grid2<f64>,
    pub vegetation: VegetationSystem,
}

/// Biome of a land or sea cell from the annual-mean climate.
fn is_volcanic(domain: &IslandDomain, p: &RegionalPhysicalState, row: usize, col: usize) -> bool {
    let (x, y) = domain.cell_center_m(DomainLevel::Medium, row, col);
    p.geophysics
        .volcanoes
        .iter()
        .any(|v| (x - v.x_m).hypot(y - v.y_m) <= VOLCANIC_CORE_FRACTION * v.radius_m)
}

/// The annual-mean temperature (K) and rain (mm/day) at a medium cell, the
/// coarse climatology sampled bilinearly and the temperature lapse-
/// corrected from the coarse mean elevation to the cell's own.
pub(crate) fn cell_climate(
    domain: &IslandDomain,
    p: &RegionalPhysicalState,
    row: usize,
    col: usize,
) -> (f64, f64) {
    let cols = domain.cols(DomainLevel::Coarse);
    let temperature = sample_coarse_at_medium(
        |r, c| p.climatology.temperature_k[r * cols + c],
        domain,
        row,
        col,
    );
    let rain = sample_coarse_at_medium(
        |r, c| p.climatology.precipitation_mm_day[r * cols + c],
        domain,
        row,
        col,
    );
    let coarse_h =
        sample_coarse_at_medium(|r, c| *p.coarse_elevation_m().get(r, c), domain, row, col);
    let h = *p.geophysics.elevation_m.get(row, col);
    // The coarse mean already carries the lapse effect of its own mean
    // elevation; correct for how far this cell sits from it (land only).
    let lapse = if h > 0.0 {
        LAPSE_RATE_K_PER_M * (coarse_h.max(0.0) - h)
    } else {
        0.0
    };
    (temperature + lapse, rain.max(0.0))
}

impl RegionalEcologyState {
    /// Classify the island's biomes, set NPP and an equilibrium biomass,
    /// and seed the sparse vegetation view.
    pub fn bootstrap(
        domain: &IslandDomain,
        physical: &RegionalPhysicalState,
    ) -> Result<Self, RegionalEcologyError> {
        let medium = DomainLevel::Medium;
        let (rows, cols) = (domain.rows(medium), domain.cols(medium));
        let spec = domain.storage_spec(medium);
        let net = physical.flow_network();
        let elevation = &physical.geophysics.elevation_m;
        if physical.geophysics.land_area_m2 <= 0.0 {
            return Err(RegionalEcologyError::NoLand);
        }

        let mut biomes = Vec::with_capacity(rows * cols);
        let mut npp = vec![0.0; rows * cols];
        for row in 0..rows {
            for col in 0..cols {
                let h = *elevation.get(row, col);
                let (temperature_k, rain_mm_day) = cell_climate(domain, physical, row, col);
                let biome = if h > 0.0 {
                    if net.lake_depth_m(row, col) > 0.0 {
                        BiomeType::CoastalWaters
                    } else if river_width_m(discharge_m3_s(&physical.hydrology, domain, row, col))
                        >= 0.5 * domain.cell_size_m(medium)
                    {
                        BiomeType::River
                    } else {
                        island_biome(
                            classify_biome(
                                h,
                                temperature_k,
                                rain_mm_day,
                                physical
                                    .hydrology
                                    .soil_water
                                    .get(row, col)
                                    .moisture_fraction,
                                is_volcanic(domain, physical, row, col),
                            ),
                            temperature_k,
                        )
                    }
                } else {
                    classify_biome(h, temperature_k, rain_mm_day, 0.0, false)
                };
                if biome.is_terrestrial() && biome != BiomeType::River {
                    npp[row * cols + col] = miami_npp_kgc_m2_yr(
                        temperature_k - 273.15,
                        rain_mm_day * EARTH_YEAR_DAYS,
                        CARBON_FRACTION_OF_DRY_MASS,
                    );
                }
                biomes.push(biome);
            }
        }
        let biome_grid = Grid2::from_data(&spec, biomes);
        let biomass: Vec<f64> = biome_grid
            .data()
            .iter()
            .zip(&npp)
            .map(|(&b, &n)| n * residence_years(b))
            .collect();

        let mut vegetation = VegetationSystem::new();
        vegetation.seed_from_biomes(&biome_grid);
        Ok(Self {
            biome_grid,
            npp_kgc_m2_yr: Grid2::from_data(&spec, npp),
            biomass_kgc_m2: Grid2::from_data(&spec, biomass),
            vegetation,
        })
    }

    /// Total producer carbon on the grid (kgC).
    pub fn total_biomass_kgc(&self, domain: &IslandDomain) -> f64 {
        self.biomass_kgc_m2.data().iter().sum::<f64>() * domain.cell_area_m2(DomainLevel::Medium)
    }

    /// Advance biomass over `dt_seconds`: growth toward `NPP × residence`
    /// with turnover `B / residence`, less `harvest_kgc_m2_yr` and the
    /// loss fraction per year `disturbance_per_yr`. Biomass stays ≥ 0 and
    /// zero where a cell has no producers.
    pub fn step(
        &mut self,
        dt_seconds: f64,
        harvest_kgc_m2_yr: Option<&Grid2<f64>>,
        disturbance_per_yr: Option<&Grid2<f64>>,
    ) {
        if !(dt_seconds.is_finite() && dt_seconds > 0.0) {
            return;
        }
        let dt_years = dt_seconds / SECONDS_PER_YEAR;
        for i in 0..self.biomass_kgc_m2.data().len() {
            let residence = residence_years(self.biome_grid.data()[i]);
            let npp = self.npp_kgc_m2_yr.data()[i];
            if residence <= 0.0 || npp <= 0.0 {
                self.biomass_kgc_m2.data_mut()[i] = 0.0;
                continue;
            }
            let b = self.biomass_kgc_m2.data()[i];
            // Exact solution of dB/dt = NPP - B/res over the step.
            let keep = (-dt_years / residence).exp();
            let mut next = b * keep + npp * residence * (1.0 - keep);
            if let Some(h) = harvest_kgc_m2_yr {
                next -= h.data()[i].max(0.0) * dt_years;
            }
            if let Some(d) = disturbance_per_yr {
                next *= (-d.data()[i].max(0.0) * dt_years).exp();
            }
            self.biomass_kgc_m2.data_mut()[i] = next.max(0.0);
        }
    }

    /// Rescale the biomass field so the land total is `total_kgc` (the
    /// producer species' carbon, which stays authoritative). A field with
    /// no biomass is left alone.
    pub fn renormalise_to_total(&mut self, domain: &IslandDomain, total_kgc: f64) {
        let current = self.total_biomass_kgc(domain);
        if current > 0.0 && total_kgc.is_finite() && total_kgc >= 0.0 {
            let scale = total_kgc / current;
            for b in self.biomass_kgc_m2.data_mut() {
                *b *= scale;
            }
        }
    }
}
