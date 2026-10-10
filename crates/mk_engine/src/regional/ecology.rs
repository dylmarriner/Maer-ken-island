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
use serde::{Deserialize, Serialize};

use super::climate::LAPSE_RATE_K_PER_M;
use super::hydrology::discharge_m3_s;
use super::levels::sample_coarse_at_medium;
use super::physical::RegionalPhysicalState;
use crate::biosphere::miami_npp_kgc_m2_yr;
use crate::climate::CLIMATOLOGY_PHASE_BINS;
use crate::organisms::vegetation::VegetationSystem;

/// Carbon fraction of dry matter, as upstream's `update_primary_production`.
pub const CARBON_FRACTION_OF_DRY_MASS: f64 = 0.475;
/// Days in the Earth year the Miami model's annual rain is per.
const EARTH_YEAR_DAYS: f64 = 365.25;
/// Köppen's polar group (E): a climate whose warmest month is below 10 °C.
///
/// Köppen (1936), *Das geographische System der Klimate*, with the
/// criteria as restated by Peel, Finlayson & McMahon (2007), *Hydrol. Earth
/// Syst. Sci.* 11, 1633, Table 1. The boundary is the one the treeline
/// follows -- trees do not establish where no month reaches 10 °C (Körner
/// 1998 for its physiological basis) -- so a cell below it is tundra or ice
/// whatever its annual mean says, and a cell above it is not.
pub const KOPPEN_POLAR_WARMEST_MONTH_BELOW_K: f64 = 283.15;
/// Köppen's ice cap (EF): a climate whose warmest month is below 0 °C.
/// Between this and the polar boundary is tundra (ET).
pub const KOPPEN_ICE_CAP_WARMEST_MONTH_BELOW_K: f64 = 273.15;
/// Months at or above 10 °C that separate Köppen's warm-summer third letter
/// (b, four or more) from the subarctic one (c, one to three): Peel et al.
/// (2007) Table 1. The first is hemiboreal mixed forest, the second taiga.
pub const KOPPEN_WARM_SUMMER_MONTHS: usize = 4;
/// Share of the year's precipitation that makes a climate summer- or
/// winter-dry for Köppen's aridity threshold: Peel et al. (2007) Table 1.
pub const KOPPEN_SEASONAL_PRECIPITATION_SHARE: f64 = 0.7;

/// A river's channel width (m) from its discharge (m³/s): Leopold &
/// Maddock (1953) hydraulic geometry, `w = 3.0 Q^0.5` for a typical river.
pub fn river_width_m(discharge_m3_s: f64) -> f64 {
    3.0 * discharge_m3_s.max(0.0).sqrt()
}

/// A cell's year month by month: the twelve phase bins of the
/// climatology, which is what Köppen's classification is defined on.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct CellYear {
    pub temperature_k: [f64; CLIMATOLOGY_PHASE_BINS],
    pub precipitation_mm_day: [f64; CLIMATOLOGY_PHASE_BINS],
}

/// Köppen's dryness classes (B group and not).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Aridity {
    /// BW.
    Desert,
    /// BS.
    Steppe,
    /// Not B.
    Humid,
}

impl CellYear {
    pub fn warmest_k(&self) -> f64 {
        self.temperature_k
            .iter()
            .copied()
            .fold(f64::NEG_INFINITY, f64::max)
    }

    pub fn mean_k(&self) -> f64 {
        self.temperature_k.iter().sum::<f64>() / CLIMATOLOGY_PHASE_BINS as f64
    }

    /// The year's precipitation (mm).
    pub fn annual_precipitation_mm(&self) -> f64 {
        self.precipitation_mm_day.iter().sum::<f64>() / CLIMATOLOGY_PHASE_BINS as f64
            * EARTH_YEAR_DAYS
    }

    /// Months whose mean is at or above `k`.
    pub fn months_at_least(&self, k: f64) -> usize {
        self.temperature_k.iter().filter(|t| **t >= k).count()
    }

    /// The share of the year's precipitation that falls in its summer half.
    ///
    /// Peel et al. define summer by calendar (April–September north of the
    /// equator, October–March south). The island has no calendar months,
    /// only orbital phase, so summer here is the six consecutive bins with
    /// the warmest mean: the same half-year, found from the temperatures
    /// rather than from which hemisphere the cell is in. A year with no
    /// precipitation has no seasonal share and reads as an even split.
    pub fn summer_precipitation_share(&self) -> f64 {
        const HALF: usize = CLIMATOLOGY_PHASE_BINS / 2;
        let window = |start: usize, values: &[f64; CLIMATOLOGY_PHASE_BINS]| -> f64 {
            (0..HALF)
                .map(|k| values[(start + k) % CLIMATOLOGY_PHASE_BINS])
                .sum()
        };
        let summer_start = (0..CLIMATOLOGY_PHASE_BINS)
            .max_by(|&a, &b| {
                window(a, &self.temperature_k).total_cmp(&window(b, &self.temperature_k))
            })
            .unwrap_or(0);
        let total: f64 = self.precipitation_mm_day.iter().sum();
        if total > 0.0 {
            window(summer_start, &self.precipitation_mm_day) / total
        } else {
            0.5
        }
    }

    /// Köppen's B test (Peel et al. 2007, Table 1): arid where the year's
    /// precipitation in mm is under ten times a threshold of twice the
    /// annual mean in °C, plus 28 if at least 70 % of it falls in summer,
    /// plus 14 if neither half takes 70 %, plus nothing if winter does;
    /// desert under five times that threshold, steppe between.
    pub fn aridity(&self) -> Aridity {
        let mean_c = self.mean_k() - 273.15;
        let summer = self.summer_precipitation_share();
        let offset = if summer >= KOPPEN_SEASONAL_PRECIPITATION_SHARE {
            28.0
        } else if 1.0 - summer >= KOPPEN_SEASONAL_PRECIPITATION_SHARE {
            0.0
        } else {
            14.0
        };
        let threshold = 2.0 * mean_c + offset;
        let precipitation = self.annual_precipitation_mm();
        if precipitation < 5.0 * threshold {
            Aridity::Desert
        } else if precipitation < 10.0 * threshold {
            Aridity::Steppe
        } else {
            Aridity::Humid
        }
    }
}

/// The island's biome: upstream's classification, with the polar classes
/// decided by Köppen's warmest-month rule instead of an annual mean.
///
/// Upstream calls ground polar from its *annual-mean* temperature (below
/// 260 K, or below 283 K and dry). That is the wrong quantity: a cell can
/// have a cold annual mean and a mild summer, and Köppen classifies by the
/// warmest month, as the treeline does. This used to split upstream's
/// reclassified ice into tundra and shrubland at a round 278 K annual mean,
/// which was the D32 deviation; every number below has a source.
///
/// Cold ground that upstream called polar and Köppen does not is classified
/// the rest of the way by Köppen too: its B test for aridity, which at a
/// cold annual mean needs very little rain to fail, and otherwise its
/// third letter -- warm-summer (b) is hemiboreal forest, subarctic (c) is
/// taiga. Polar is tested first: the two can only meet where the mean is
/// above freezing and yet no month reaches 10 °C, a year with almost no
/// seasons, and there the treeline decides it.
///
/// Landform and water classes -- alpine, volcanic, wetland, river and the
/// seas -- are left as they are, because they are decided by what the
/// ground is rather than by the climate over it.
pub(crate) fn island_biome(classified: BiomeType, year: &CellYear) -> BiomeType {
    use BiomeType::*;
    if matches!(
        classified,
        Alpine | Volcanic | Wetland | River | CoastalWaters | ShallowOcean | DeepOcean | ReefSea
    ) {
        return classified;
    }
    let warmest = year.warmest_k();
    if warmest < KOPPEN_ICE_CAP_WARMEST_MONTH_BELOW_K {
        return IceSheet;
    }
    if warmest < KOPPEN_POLAR_WARMEST_MONTH_BELOW_K {
        return Tundra;
    }
    if matches!(classified, IceSheet | Tundra) {
        return match year.aridity() {
            Aridity::Desert => Desert,
            Aridity::Steppe => Grassland,
            Aridity::Humid
                if year.months_at_least(KOPPEN_POLAR_WARMEST_MONTH_BELOW_K)
                    >= KOPPEN_WARM_SUMMER_MONTHS =>
            {
                TemperateForest
            }
            Aridity::Humid => BorealForest,
        };
    }
    classified
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

/// Mean NPP (g dry m⁻² yr⁻¹) and mean plant biomass (kg dry m⁻²) of the
/// ecosystem types of Whittaker & Likens (1975), Table 15-1, as
/// `fixtures/reference/ecology` carries them (`npp_and_biomass_by_biome`;
/// `tests/regional_ecology.rs` holds the two in step).
pub mod whittaker_likens {
    pub const TROPICAL_RAIN_FOREST: (f64, f64) = (2200.0, 45.0);
    pub const TROPICAL_SEASONAL_FOREST: (f64, f64) = (1600.0, 35.0);
    pub const TEMPERATE_EVERGREEN_FOREST: (f64, f64) = (1300.0, 35.0);
    pub const TEMPERATE_DECIDUOUS_FOREST: (f64, f64) = (1200.0, 30.0);
    pub const BOREAL_FOREST: (f64, f64) = (800.0, 20.0);
    pub const WOODLAND_AND_SHRUBLAND: (f64, f64) = (700.0, 6.0);
    pub const SAVANNA: (f64, f64) = (900.0, 4.0);
    pub const TEMPERATE_GRASSLAND: (f64, f64) = (600.0, 1.6);
    pub const TUNDRA_AND_ALPINE: (f64, f64) = (140.0, 0.6);
    pub const DESERT_AND_SEMIDESERT_SCRUB: (f64, f64) = (90.0, 0.7);
    pub const EXTREME_DESERT_ROCK_SAND_ICE: (f64, f64) = (3.0, 0.02);
    pub const SWAMP_AND_MARSH: (f64, f64) = (2000.0, 15.0);
    /// World areas (10⁶ km²) of the two temperate forest types, which
    /// weight them into one temperate forest.
    pub const TEMPERATE_EVERGREEN_AREA: f64 = 5.0;
    pub const TEMPERATE_DECIDUOUS_AREA: f64 = 7.0;
}

/// Biomass ÷ NPP of one Whittaker & Likens row, in years: both columns are
/// dry matter, so the carbon fraction cancels.
fn turnover_years((npp_g_m2_yr, biomass_kg_m2): (f64, f64)) -> f64 {
    biomass_kg_m2 * 1000.0 / npp_g_m2_yr
}

/// The island's one temperate forest: Whittaker & Likens's evergreen and
/// deciduous rows together, their world totals summed (area × mean), so
/// the ratio is total biomass over total production.
fn temperate_forest_turnover_years() -> f64 {
    use whittaker_likens::*;
    let (ev, de) = (TEMPERATE_EVERGREEN_AREA, TEMPERATE_DECIDUOUS_AREA);
    let biomass = ev * TEMPERATE_EVERGREEN_FOREST.1 + de * TEMPERATE_DECIDUOUS_FOREST.1;
    let npp = ev * TEMPERATE_EVERGREEN_FOREST.0 + de * TEMPERATE_DECIDUOUS_FOREST.0;
    biomass * 1000.0 / npp
}

/// Years of NPP a biome's standing producer carbon amounts to (biomass ÷
/// NPP at steady state), from the mean rows of Whittaker & Likens (1975).
///
/// Each biome takes the row that describes it. Where the island has a
/// class the table does not: montane forest is temperate forest on a
/// mountain; desert and semi-desert are both the vegetated desert scrub
/// row (the island's are Köppen deserts with plants on them, not the
/// table's bare extreme desert); volcanic ground and ice are the bare row.
/// The table's own ranges span about an order of magnitude in both columns
/// within a type, and a cell's departure from the mean ratio is not
/// modelled (deviation D30). Zero where producers do not stand on land.
pub fn residence_years(biome: BiomeType) -> f64 {
    use whittaker_likens::*;
    use BiomeType::*;
    match biome {
        TropicalRainforest => turnover_years(TROPICAL_RAIN_FOREST),
        TropicalDryForest => turnover_years(TROPICAL_SEASONAL_FOREST),
        TemperateForest | MontaneForest => temperate_forest_turnover_years(),
        BorealForest => turnover_years(BOREAL_FOREST),
        Woodland | Shrubland => turnover_years(WOODLAND_AND_SHRUBLAND),
        Savanna => turnover_years(SAVANNA),
        Grassland => turnover_years(TEMPERATE_GRASSLAND),
        Tundra | Alpine => turnover_years(TUNDRA_AND_ALPINE),
        Desert | SemiDesert => turnover_years(DESERT_AND_SEMIDESERT_SCRUB),
        Wetland => turnover_years(SWAMP_AND_MARSH),
        Volcanic | IceSheet => turnover_years(EXTREME_DESERT_ROCK_SAND_ICE),
        River | DeepOcean | ShallowOcean | CoastalWaters | ReefSea => 0.0,
    }
}

/// The island's producers over the medium grid.
#[derive(Debug, Clone, Serialize, Deserialize)]
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

/// The cell's year month by month, sampled from the coarse climatology as
/// [`cell_climate`] samples the annual mean, every month's temperature
/// lapse-corrected by the same amount: a peak is colder in summer by the
/// rate it is colder on average.
pub(crate) fn cell_year(
    domain: &IslandDomain,
    p: &RegionalPhysicalState,
    row: usize,
    col: usize,
) -> CellYear {
    let cols = domain.cols(DomainLevel::Coarse);
    let coarse_h =
        sample_coarse_at_medium(|r, c| *p.coarse_elevation_m().get(r, c), domain, row, col);
    let h = *p.geophysics.elevation_m.get(row, col);
    let lapse = if h > 0.0 {
        LAPSE_RATE_K_PER_M * (coarse_h.max(0.0) - h)
    } else {
        0.0
    };
    let mut year = CellYear {
        temperature_k: [0.0; CLIMATOLOGY_PHASE_BINS],
        precipitation_mm_day: [0.0; CLIMATOLOGY_PHASE_BINS],
    };
    for month in 0..CLIMATOLOGY_PHASE_BINS {
        year.temperature_k[month] = sample_coarse_at_medium(
            |r, c| p.climatology.month_temperature_k(month, r * cols + c),
            domain,
            row,
            col,
        ) + lapse;
        year.precipitation_mm_day[month] = sample_coarse_at_medium(
            |r, c| {
                p.climatology
                    .month_precipitation_mm_day(month, r * cols + c)
            },
            domain,
            row,
            col,
        )
        .max(0.0);
    }
    year
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
                            &cell_year(domain, physical, row, col),
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

#[cfg(test)]
mod koppen_tests {
    use super::*;
    use mk_core::biomes::BiomeType::*;

    /// A year whose monthly mean follows a cosine about `mean_k`, warmest
    /// in bin `peak`, with `rain` mm/day in every month.
    fn seasonal(mean_k: f64, amplitude_k: f64, peak: f64, rain: f64) -> CellYear {
        let mut year = flat(mean_k, rain);
        for (month, t) in year.temperature_k.iter_mut().enumerate() {
            let phase = (month as f64 - peak) / CLIMATOLOGY_PHASE_BINS as f64;
            *t = mean_k + amplitude_k * (std::f64::consts::TAU * phase).cos();
        }
        year
    }

    fn flat(t_k: f64, rain: f64) -> CellYear {
        CellYear {
            temperature_k: [t_k; CLIMATOLOGY_PHASE_BINS],
            precipitation_mm_day: [rain; CLIMATOLOGY_PHASE_BINS],
        }
    }

    /// Rain of `annual_mm` with `summer_share` of it in bins 3 to 8, on a
    /// year of `mean_k` with a 10 K swing peaking between bins 5 and 6 --
    /// so those six are unambiguously the warm half.
    fn with_rain_season(mean_k: f64, annual_mm: f64, summer_share: f64) -> CellYear {
        let mut year = seasonal(mean_k, 10.0, 5.5, 0.0);
        let per_month = |share: f64| share * annual_mm / EARTH_YEAR_DAYS * 2.0;
        for (month, p) in year.precipitation_mm_day.iter_mut().enumerate() {
            let summer = (3..9).contains(&month);
            *p = per_month(if summer {
                summer_share
            } else {
                1.0 - summer_share
            });
        }
        year
    }

    #[test]
    fn a_cold_mean_with_a_mild_summer_and_little_rain_is_taiga() {
        // The case both earlier rules got wrong. The annual mean is -2 °C,
        // so upstream called it polar; its warmest month is 12 °C, so
        // Köppen does not. 365 mm a year is under 1 mm/day -- which the
        // temperate thresholds call semi-desert -- but at that mean the B
        // threshold is 2(-2) + 14 = 10, and 365 mm is far above 100.
        // Three months reach 10 °C: subarctic, Dfc, taiga.
        let year = seasonal(271.15, 14.0, 6.0, 1.0);
        assert_eq!(year.months_at_least(KOPPEN_POLAR_WARMEST_MONTH_BELOW_K), 3);
        assert_eq!(year.aridity(), Aridity::Humid);
        assert_eq!(island_biome(Tundra, &year), BorealForest);
        assert_eq!(island_biome(IceSheet, &year), BorealForest);
    }

    #[test]
    fn a_longer_summer_is_hemiboreal_forest() {
        // Mean 3 °C, warmest 18 °C: five months at or above 10 °C, Dfb.
        let year = seasonal(276.15, 15.0, 6.0, 1.5);
        assert_eq!(year.months_at_least(KOPPEN_POLAR_WARMEST_MONTH_BELOW_K), 5);
        assert_eq!(island_biome(Tundra, &year), TemperateForest);
    }

    #[test]
    fn cold_ground_that_is_dry_by_koppens_measure_is_desert_or_steppe() {
        // Mean 0 °C, rain spread evenly: threshold 14, so desert under
        // 70 mm a year and steppe under 140.
        let mm_day = |annual_mm: f64| annual_mm / EARTH_YEAR_DAYS;
        let dry = |annual_mm| seasonal(273.15, 12.0, 6.0, mm_day(annual_mm));
        assert_eq!(island_biome(IceSheet, &dry(50.0)), Desert);
        assert_eq!(island_biome(IceSheet, &dry(100.0)), Grassland);
        assert_ne!(island_biome(IceSheet, &dry(300.0)), Desert);
        assert_ne!(island_biome(IceSheet, &dry(300.0)), Grassland);
    }

    #[test]
    fn rain_that_falls_in_summer_counts_for_less() {
        // Mean 5 °C and 150 mm a year. Spread evenly the threshold is 24
        // (steppe); with 80 % in summer it is 38 and 150 < 190 is desert;
        // with 80 % in winter it is 10 and 150 > 100 is not arid at all.
        let even = with_rain_season(278.15, 150.0, 0.5);
        let summer = with_rain_season(278.15, 150.0, 0.8);
        let winter = with_rain_season(278.15, 150.0, 0.2);
        assert!((even.annual_precipitation_mm() - 150.0).abs() < 1e-9);
        assert!((summer.summer_precipitation_share() - 0.8).abs() < 1e-9);
        assert_eq!(even.aridity(), Aridity::Steppe);
        assert_eq!(summer.aridity(), Aridity::Desert);
        assert_eq!(winter.aridity(), Aridity::Humid);
    }

    #[test]
    fn summer_is_found_from_the_temperatures_not_the_calendar() {
        // The same year with its warm season six bins later -- the other
        // hemisphere -- has the same summer share.
        let north = with_rain_season(278.15, 300.0, 0.75);
        let mut south = north;
        south.temperature_k.rotate_left(6);
        south.precipitation_mm_day.rotate_left(6);
        assert!(
            (north.summer_precipitation_share() - south.summer_precipitation_share()).abs() < 1e-9
        );
        assert_eq!(flat(280.0, 0.0).summer_precipitation_share(), 0.5);
    }

    #[test]
    fn no_forest_stands_where_no_month_reaches_ten_degrees() {
        // The treeline. Upstream can call a cool wet cell forest from its
        // annual mean; if its warmest month is 8 °C, nothing woody
        // establishes there and it is tundra.
        for forest in [TemperateForest, BorealForest, MontaneForest, Woodland] {
            assert_eq!(
                island_biome(forest, &seasonal(276.15, 5.0, 2.0, 6.0)),
                Tundra,
                "{forest:?}"
            );
        }
    }

    #[test]
    fn ice_is_where_no_month_thaws() {
        assert_eq!(island_biome(Tundra, &flat(270.0, 2.0)), IceSheet);
        assert_eq!(
            island_biome(TemperateForest, &seasonal(262.0, 10.0, 0.0, 6.0)),
            IceSheet
        );
    }

    #[test]
    fn the_boundaries_are_koppens_to_the_hundredth_of_a_degree() {
        // 10 °C exactly is the first temperature that is *not* polar, and
        // 0 °C the first that is not ice cap.
        assert_eq!(island_biome(Shrubland, &flat(283.15, 3.0)), Shrubland);
        assert_eq!(island_biome(Shrubland, &flat(283.14, 3.0)), Tundra);
        assert_eq!(island_biome(Shrubland, &flat(273.15, 3.0)), Tundra);
        assert_eq!(island_biome(Shrubland, &flat(273.14, 3.0)), IceSheet);
        // And four months at 10 °C is warm-summer, three is subarctic.
        let mut year = flat(270.0, 3.0);
        year.temperature_k[..4].fill(283.15);
        assert_eq!(island_biome(Tundra, &year), TemperateForest);
        year.temperature_k[3] = 283.14;
        assert_eq!(island_biome(Tundra, &year), BorealForest);
    }

    #[test]
    fn what_the_ground_is_outranks_the_climate_over_it() {
        // Landform and water are not reclassified by temperature: a
        // freezing summit is still alpine, a frozen marsh still wetland.
        for landform in [Alpine, Volcanic, Wetland, River, CoastalWaters] {
            assert_eq!(
                island_biome(landform, &flat(260.0, 2.0)),
                landform,
                "{landform:?}"
            );
        }
    }

    #[test]
    fn a_warm_cell_upstream_got_right_is_left_alone() {
        assert_eq!(
            island_biome(TemperateForest, &seasonal(285.0, 10.0, 6.0, 7.0)),
            TemperateForest
        );
        assert_eq!(
            island_biome(Grassland, &seasonal(290.0, 10.0, 6.0, 3.5)),
            Grassland
        );
    }
}
