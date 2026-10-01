pub mod apex_predators;
pub mod biodiversity_catalogue;
pub mod body_plans;
pub mod catalogue_individuals;
pub mod deep_time_evolution;
pub mod energy_metabolism;
pub mod evolution;
pub mod extinction;
/// MARR'KENA Phase 3 Biosphere: Animals Only
pub mod genetics;
pub mod habitat;
pub mod marine;
pub mod marine_terrestrial;
pub mod nervous_system;

// Re-export key types
pub use apex_predators::ApexPredatorSystem;
pub use biodiversity_catalogue::{BiodiversityCatalogue, BiodiversityEntry};
pub use catalogue_individuals::{CatalogueIndividual, CatalogueIndividualRegistry};
pub use evolution::{is_fauna_category, Sex, Species, SpeciesCategory, TerrainAffinity};
pub use extinction::ExtinctionRecoverySystem;
pub use genetics::{Genome, PRE_SAPIENT_CEILING};

use mk_core::canon::CanonLocked;
use mk_core::ids::KenzIeSubclass;
use mk_core::rng::RngRegistry;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Main biosphere system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiosphereSystem {
    pub canon: Arc<CanonLocked>,
    pub rng: RngRegistry,
    pub species: Vec<Species>,
    pub biodiversity_catalogue: Option<BiodiversityCatalogue>,
    pub terrain_profile: BiosphereTerrainProfile,
    pub extinction_system: ExtinctionRecoverySystem,
    pub apex_predator_system: ApexPredatorSystem,
    pub current_time_myr: f64,
    pub statistics: BiosphereStatistics,
    /// Net primary productivity per grid cell, kgC m⁻² yr⁻¹, row-major
    /// (`row * nlon + col`, `GridSpec` layout): the Miami model on land and
    /// the [`marine`] light/temperature/nutrient model at sea. Empty until
    /// the first biosphere step. See
    /// [`BiosphereSystem::update_primary_production`].
    #[serde(default)]
    pub npp_field_kgc_m2_yr: Vec<f64>,
    /// Carbon in dead organic matter (litter and soil organic carbon), kg C:
    /// biomass that died and has not yet decomposed back to the atmosphere.
    /// See [`crate::conservation::decompose_detritus`].
    #[serde(default)]
    pub detritus_carbon_kg: f64,
    /// Nitrogen in dead organic matter, kg N.
    #[serde(default)]
    pub detritus_nitrogen_kg: f64,
    /// Phosphorus in dead organic matter, kg P.
    #[serde(default)]
    pub detritus_phosphorus_kg: f64,
    /// Bioavailable soil nitrogen, kg N: what living biomass takes up as it
    /// grows and what mineralization returns. `None` until the world sizes it
    /// from the canon (`mk_engine::conservation::canon_soil_nutrients_kg`).
    #[serde(default)]
    pub soil_nitrogen_kg: Option<f64>,
    /// Bioavailable soil phosphorus, kg P (as [`Self::soil_nitrogen_kg`]).
    #[serde(default)]
    pub soil_phosphorus_kg: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiosphereTerrainProfile {
    pub planet_scale_factor: f64,
    pub ocean_basins: u32,
    pub river_systems: u32,
    pub mountain_ranges: u32,
    pub volcanic_arcs: u32,
    pub hill_belts: u32,
    pub supercontinents: u32,
    pub terrain_description: String,
}

impl Default for BiosphereTerrainProfile {
    fn default() -> Self {
        Self {
            planet_scale_factor: 1.35,
            ocean_basins: 9,
            river_systems: 28_000,
            mountain_ranges: 144,
            volcanic_arcs: 37,
            hill_belts: 920,
            supercontinents: 6,
            terrain_description: "A larger Earth-like world with storm-cut oceans, continent-spanning rivers, immense hills, knife-backed mountain chains, and incandescent volcanic crowns.".to_string(),
        }
    }
}

/// Total surface area of the planet (km²), the range the apex tier spans.
fn planet_surface_km2(canon: &mk_core::canon::CanonLocked) -> f64 {
    4.0 * std::f64::consts::PI * canon.planet_radius_m.powi(2) / 1.0e6
}

/// Miami-model terrestrial NPP (Lieth 1975) in kgC m⁻² yr⁻¹ for a mean
/// temperature (°C) and annual precipitation (mm), given the carbon fraction
/// of dry matter. See [`BiosphereSystem::update_primary_production`].
pub fn miami_npp_kgc_m2_yr(
    temperature_c: f64,
    precipitation_mm_yr: f64,
    carbon_fraction: f64,
) -> f64 {
    let temperature_limited = 3000.0 / (1.0 + (1.315 - 0.119 * temperature_c).exp());
    let precipitation_limited = 3000.0 * (1.0 - (-0.000664 * precipitation_mm_yr.max(0.0)).exp());
    // g dry matter → kg carbon.
    temperature_limited.min(precipitation_limited) * carbon_fraction / 1000.0
}

impl Default for BiosphereSystem {
    fn default() -> Self {
        use mk_core::canon::CanonLocked;
        use std::sync::Arc;

        let canon = Arc::new(CanonLocked::default());
        Self {
            apex_predator_system: ApexPredatorSystem::new(planet_surface_km2(&canon)),
            canon,
            rng: RngRegistry::new([0u8; 32]),
            species: Vec::new(),
            biodiversity_catalogue: None,
            terrain_profile: BiosphereTerrainProfile::default(),
            extinction_system: ExtinctionRecoverySystem::new(),
            current_time_myr: 0.0,
            statistics: BiosphereStatistics::default(),
            npp_field_kgc_m2_yr: Vec::new(),
            detritus_carbon_kg: 0.0,
            detritus_nitrogen_kg: 0.0,
            detritus_phosphorus_kg: 0.0,
            soil_nitrogen_kg: None,
            soil_phosphorus_kg: None,
        }
    }
}

/// Biosphere statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiosphereStatistics {
    pub total_species: usize,
    pub total_population: u64,
    pub total_biomass_kg: f64,
    /// Planet-total NPP, land plus ocean, kgC yr⁻¹.
    pub current_npp_kgc_yr: f64,
    /// Share of `current_npp_kgc_yr` produced on land, kgC yr⁻¹.
    #[serde(default)]
    pub terrestrial_npp_kgc_yr: f64,
    /// Share of `current_npp_kgc_yr` produced in the ocean, kgC yr⁻¹.
    #[serde(default)]
    pub marine_npp_kgc_yr: f64,
    pub average_intelligence: f64,
    pub maximum_intelligence: f64,
    pub marine_species_count: usize,
    pub terrestrial_species_count: usize,
    pub floral_species_count: usize,
    pub pollinator_species_count: usize,
    pub tree_species_count: usize,
    pub mountain_species_count: usize,
    pub volcanic_species_count: usize,
    pub river_species_count: usize,
    pub stability_index: f64,
}

impl Default for BiosphereStatistics {
    fn default() -> Self {
        Self {
            total_species: 0,
            total_population: 0,
            total_biomass_kg: 0.0,
            current_npp_kgc_yr: 0.0,
            terrestrial_npp_kgc_yr: 0.0,
            marine_npp_kgc_yr: 0.0,
            average_intelligence: 0.0,
            maximum_intelligence: 0.0,
            marine_species_count: 0,
            terrestrial_species_count: 0,
            floral_species_count: 0,
            pollinator_species_count: 0,
            tree_species_count: 0,
            mountain_species_count: 0,
            volcanic_species_count: 0,
            river_species_count: 0,
            stability_index: 0.0,
        }
    }
}

impl BiosphereSystem {
    pub fn new(canon: Arc<CanonLocked>, rng_seed: u64) -> Self {
        let mut seed = [0u8; 32];
        seed[..8].copy_from_slice(&rng_seed.to_le_bytes());
        let rng = RngRegistry::new(seed);

        Self {
            apex_predator_system: ApexPredatorSystem::new(planet_surface_km2(&canon)),
            canon,
            rng,
            species: Vec::new(),
            biodiversity_catalogue: None,
            terrain_profile: BiosphereTerrainProfile::default(),
            extinction_system: ExtinctionRecoverySystem::new(),
            current_time_myr: 0.0,
            statistics: BiosphereStatistics::default(),
            npp_field_kgc_m2_yr: Vec::new(),
            detritus_carbon_kg: 0.0,
            detritus_nitrogen_kg: 0.0,
            detritus_phosphorus_kg: 0.0,
            soil_nitrogen_kg: None,
            soil_phosphorus_kg: None,
        }
    }

    pub fn initialize(&mut self) -> Result<(), String> {
        self.species = self.generate_founding_species();

        self.update_statistics();

        // Log species creation events
        for species in &self.species {
            crate::io::global_events::log_species_created(
                0,
                species.species_id,
                &species.species_name,
            );
        }

        Ok(())
    }

    /// Load biodiversity catalogue from JSONL file
    pub fn load_biodiversity_catalogue(&mut self, path: &std::path::Path) -> Result<(), String> {
        let catalogue = BiodiversityCatalogue::load_from_jsonl(path)?;
        self.biodiversity_catalogue = Some(catalogue);
        Ok(())
    }

    fn generate_founding_species(&self) -> Vec<Species> {
        const TARGET_SPECIES_COUNT: usize = 4096;

        #[derive(Clone)]
        struct GuildTemplate {
            guild: &'static str,
            roots: &'static [&'static str],
            category: SpeciesCategory,
            terrain: TerrainAffinity,
            adaptation: u8,
            base_size: u8,
            base_population: u64,
            base_range: f64,
            niche: &'static str,
        }

        const GUILDS: &[GuildTemplate] = &[
            GuildTemplate {
                guild: "Pelagic",
                roots: &[
                    "Abyr", "Lunefin", "Veyrcoil", "Thalor", "Nacreon", "Zephray",
                ],
                category: SpeciesCategory::Aquatic,
                terrain: TerrainAffinity::OpenOcean,
                adaptation: 0,
                base_size: 4,
                base_population: 2_400_000,
                base_range: 0.78,
                niche: "open-ocean filter and pursuit swimmers",
            },
            GuildTemplate {
                guild: "Reef",
                roots: &[
                    "Coralisk",
                    "Prismjaw",
                    "Spindlefin",
                    "Mirelace",
                    "Glasstide",
                    "Whorlmaw",
                ],
                category: SpeciesCategory::Aquatic,
                terrain: TerrainAffinity::ReefSea,
                adaptation: 0,
                base_size: 3,
                base_population: 1_700_000,
                base_range: 0.44,
                niche: "warm reef browsers and ambushers",
            },
            GuildTemplate {
                guild: "River",
                roots: &[
                    "Torrentbeak",
                    "Siltscale",
                    "Runeel",
                    "Foamclaw",
                    "Braidsnout",
                    "Rillspine",
                ],
                category: SpeciesCategory::Aquatic,
                terrain: TerrainAffinity::River,
                adaptation: 2,
                base_size: 3,
                base_population: 1_050_000,
                base_range: 0.36,
                niche: "braided-river migrants and current hunters",
            },
            GuildTemplate {
                guild: "Wetland",
                roots: &[
                    "Fenmir",
                    "Muckskip",
                    "Bogflare",
                    "Reedmantle",
                    "Loamfin",
                    "Misttoad",
                ],
                category: SpeciesCategory::Amphibious,
                terrain: TerrainAffinity::Wetland,
                adaptation: 2,
                base_size: 3,
                base_population: 780_000,
                base_range: 0.31,
                niche: "marsh-edge omnivores and mud lurkers",
            },
            GuildTemplate {
                guild: "Skylance",
                roots: &[
                    "Aeroshard",
                    "Thermash",
                    "Vaporwing",
                    "Cliffdart",
                    "Aurafeather",
                    "Mistrake",
                ],
                category: SpeciesCategory::Flying,
                terrain: TerrainAffinity::Cliff,
                adaptation: 1,
                base_size: 3,
                base_population: 220_000,
                base_range: 0.40,
                niche: "high-atmosphere fliers riding giant thermal rivers",
            },
            GuildTemplate {
                guild: "Bird",
                roots: &[
                    "Cantorix",
                    "Stonebill",
                    "Galecrest",
                    "Dawnquill",
                    "Ironsong",
                    "Flutelark",
                ],
                category: SpeciesCategory::Bird,
                terrain: TerrainAffinity::ForestCanopy,
                adaptation: 1,
                base_size: 4,
                base_population: 180_000,
                base_range: 0.34,
                niche: "forest-canopy and cliff-nesting avians",
            },
            GuildTemplate {
                guild: "Bee",
                roots: &[
                    "Aurabee",
                    "Hiveflare",
                    "Mosshoney",
                    "Velorb",
                    "Dawnhex",
                    "Bloomspun",
                    "Stonewax",
                    "Goldreed",
                ],
                category: SpeciesCategory::Bee,
                terrain: TerrainAffinity::ForestCanopy,
                adaptation: 1,
                base_size: 1,
                base_population: 5_400_000,
                base_range: 0.24,
                niche: "eusocial pollinators of radiant alien flowers",
            },
            GuildTemplate {
                guild: "Insect",
                roots: &[
                    "Shardmidge",
                    "Dustscarab",
                    "Glassmoth",
                    "Needleant",
                    "Reedhopper",
                    "Shelltick",
                ],
                category: SpeciesCategory::Insect,
                terrain: TerrainAffinity::Plains,
                adaptation: 1,
                base_size: 1,
                base_population: 7_600_000,
                base_range: 0.29,
                niche: "micro-herds of armored and winged invertebrates",
            },
            GuildTemplate {
                guild: "Grazer",
                roots: &[
                    "Valehorn",
                    "Mosshoof",
                    "Rootmuzzle",
                    "Dunechewer",
                    "Brambleback",
                    "Canopygraze",
                ],
                category: SpeciesCategory::Herbivore,
                terrain: TerrainAffinity::Plains,
                adaptation: 1,
                base_size: 6,
                base_population: 510_000,
                base_range: 0.47,
                niche: "megafloral browsers across giant hills and plains",
            },
            GuildTemplate {
                guild: "Omnivore",
                roots: &[
                    "Riverscout",
                    "Brushsnout",
                    "Clawforager",
                    "Hollowsnout",
                    "Shardtooth",
                    "Cairnrunner",
                ],
                category: SpeciesCategory::Omnivore,
                terrain: TerrainAffinity::Hill,
                adaptation: 3,
                base_size: 4,
                base_population: 320_000,
                base_range: 0.41,
                niche: "adaptive foragers between forest, river, and upland",
            },
            GuildTemplate {
                guild: "Predator",
                roots: &[
                    "Shadefang",
                    "Ridgeclaw",
                    "Stormmaw",
                    "Ashstalker",
                    "Swifttalon",
                    "Nightrake",
                ],
                category: SpeciesCategory::Carnivore,
                terrain: TerrainAffinity::Mountain,
                adaptation: 1,
                base_size: 7,
                base_population: 92_000,
                base_range: 0.30,
                niche: "stalkers of mountain passes and shadow forests",
            },
            GuildTemplate {
                guild: "Volcanic",
                roots: &[
                    "Cinderjaw",
                    "Basaltalon",
                    "Emberhide",
                    "Sulfurclaw",
                    "Lavastride",
                    "Ashmantis",
                ],
                category: SpeciesCategory::Carnivore,
                terrain: TerrainAffinity::Volcanic,
                adaptation: 1,
                base_size: 5,
                base_population: 38_000,
                base_range: 0.18,
                niche: "heat-tolerant hunters of caldera rims and lava deltas",
            },
            GuildTemplate {
                guild: "Tree",
                roots: &[
                    "Crownspire",
                    "Glassbark",
                    "Lanternwood",
                    "Mistcedar",
                    "Emberfrond",
                    "Starroot",
                ],
                category: SpeciesCategory::Tree,
                terrain: TerrainAffinity::ForestCanopy,
                adaptation: 1,
                base_size: 8,
                base_population: 860_000,
                base_range: 0.53,
                niche: "towering alien trees anchoring supercontinental forests",
            },
            GuildTemplate {
                guild: "Flower",
                roots: &[
                    "Bloomflare",
                    "Nectarveil",
                    "Suncoil",
                    "Morrowpetal",
                    "Prismblossom",
                    "Glowthistle",
                ],
                category: SpeciesCategory::Flower,
                terrain: TerrainAffinity::Hill,
                adaptation: 1,
                base_size: 1,
                base_population: 9_200_000,
                base_range: 0.33,
                niche: "chromatic flowering meadows feeding pollinator swarms",
            },
        ];

        let mut species = Vec::with_capacity(TARGET_SPECIES_COUNT);

        for idx in 0..TARGET_SPECIES_COUNT {
            let species_id = (idx + 1) as u64;
            let template = &GUILDS[idx % GUILDS.len()];
            let root = template.roots[(idx / GUILDS.len()) % template.roots.len()];
            let mut genome = Genome::new(species_id, &self.rng, KenzIeSubclass::Alpha);

            genome.structural.environmental_adaptation = template.adaptation;
            genome.structural.size_modifier = (template.base_size + (idx % 4) as u8).min(12);
            genome.metabolic.basal_rate = 2 + ((idx * 3) % 8) as u8;
            genome.metabolic.neural_cost = 1 + ((idx * 5) % 4) as u8;
            genome.metabolic.active_rate = 2 + ((idx * 7) % 6) as u8;
            genome.metabolic.thermal_tolerance = 2 + ((idx * 11) % 8) as u8;
            genome.metabolic.water_conservation = 1 + ((idx * 13) % 8) as u8;
            genome.sensory.visual_bandwidth = 2 + ((idx * 2) % 10) as u8;
            genome.sensory.acoustic_bandwidth = 1 + ((idx * 3) % 10) as u8;
            genome.sensory.pressure_sensitivity = 1 + ((idx * 5) % 10) as u8;
            genome.sensory.chemosensation_range = 2 + ((idx * 7) % 10) as u8;
            genome.sensory.electrosensitivity = ((idx * 11) % 5) as u8;
            genome.neural.processing_capacity = 2 + ((idx * 2) % 9) as u8;
            genome.neural.memory_capacity = 1 + ((idx * 3) % 8) as u8;
            genome.neural.intel_index = 1 + ((idx * 5) % 6) as u8;
            genome.neural.reflex_complexity = 2 + ((idx * 7) % 8) as u8;
            genome.neural.learning_capacity = 1 + ((idx * 11) % 6) as u8;
            genome.neural.memory_duration = 1 + ((idx * 13) % 6) as u8;

            if genome.exceeds_intelligence_ceiling() {
                genome.neural.processing_capacity = 8;
                genome.neural.intel_index = 4;
                genome.neural.learning_capacity = 4;
            }

            let population_size = template
                .base_population
                .saturating_sub((idx as u64 % 97) * 511);
            let geographic_range = template.base_range + ((idx % 23) as f64 * 0.0125);
            let mut sp = Species::new(
                species_id,
                genome,
                population_size.max(500),
                geographic_range,
            );
            let terrain_word = match template.terrain {
                TerrainAffinity::OpenOcean => "deeps",
                TerrainAffinity::ReefSea => "reef",
                TerrainAffinity::River => "river",
                TerrainAffinity::Wetland => "fen",
                TerrainAffinity::Hill => "hill",
                TerrainAffinity::Mountain => "mount",
                TerrainAffinity::Volcanic => "volc",
                TerrainAffinity::ForestCanopy => "grove",
                TerrainAffinity::Plains => "plain",
                TerrainAffinity::Cliff => "cliff",
            };
            sp.species_name = format!(
                "{} {}-{} {:04}",
                template.guild,
                root,
                terrain_word,
                idx + 1
            );
            sp.category = template.category.clone();
            sp.terrain_affinity = template.terrain.clone();
            sp.ecological_niche = format!(
                "{} on {} terrain of a super-Earth biosphere",
                template.niche, terrain_word
            );
            species.push(sp);
        }

        species
    }

    pub fn step_biosphere(
        &mut self,
        dt_myr: f64,
        climate: &crate::climate::ClimateState,
        weather: &crate::weather::WeatherState,
        ocean: &crate::ocean::OceanState,
        elevation: &mk_core::grid::Grid2<f64>,
    ) -> Result<(), String> {
        let dt_years = dt_myr * 1_000_000.0;

        // Evolution and extinction are *not* run here: the world pipeline's
        // own Evolution (13) and Extinction (14) steps advance the single
        // `WorldState::deep_time` model, so species are not evolved twice
        // per tick with inconsistent time units. Apex predators are bounded
        // here against the live prey biomass.
        self.apex_predator_system.step_apex_predators(
            &mut self.species,
            dt_years,
            &mut self.rng,
        )?;

        self.current_time_myr += dt_myr;
        self.update_primary_production(climate, weather, ocean, elevation);
        self.update_statistics();
        Ok(())
    }

    /// Recompute the aggregate statistics from the live species list. Call
    /// after anything outside [`Self::step_biosphere`] adds or removes
    /// species (evolution, extinction), so projections never read stale
    /// counts.
    pub fn refresh_statistics(&mut self) {
        self.update_statistics();
    }

    /// Recompute per-cell NPP from the live climate. Land uses the
    /// Miami model (Lieth 1975): productivity is the lesser of its
    /// temperature limit `3000 / (1 + e^(1.315 − 0.119 T))` and its
    /// precipitation limit `3000 (1 − e^(−0.000664 P))` (g dry matter m⁻²
    /// yr⁻¹, `T` in °C, `P` in mm yr⁻¹), converted to carbon at 47.5% of dry
    /// mass and capped at the canon's maximum productivity. Ocean cells use
    /// [`marine::marine_npp_kgc_m2_yr`] from the ocean's sea-surface temperature,
    /// annual-mean surface light and upwelling or shelf nutrient supply. The
    /// planet total integrates every cell over its true spherical area.
    pub fn update_primary_production(
        &mut self,
        climate: &crate::climate::ClimateState,
        weather: &crate::weather::WeatherState,
        ocean: &crate::ocean::OceanState,
        elevation: &mk_core::grid::Grid2<f64>,
    ) {
        const CARBON_FRACTION_OF_DRY_MASS: f64 = 0.475;
        const DAYS_PER_YEAR: f64 = 365.25;
        let (nlat, nlon) = (elevation.nlat(), elevation.nlon());
        let radius = self.canon.planet_radius_m;
        let cap = self.canon.npp_max_kgc_m2_yr;
        let dlon = std::f64::consts::TAU / nlon.max(1) as f64;

        let obliquity = self.canon.obliquity_deg.to_radians();
        let marine_inputs = marine::MarineInputs {
            wind: &weather.wind,
            elevation,
            planet_radius_m: radius,
            rotation_rate_rad_s: mk_core::canon::CanonDerived::from(&self.canon).omega_rad_s,
        };

        let nutrient_limit = self.land_nutrient_limitation(elevation, radius, dlon);

        let mut field = Vec::with_capacity(nlat * nlon);
        let (mut land_kgc_yr, mut ocean_kgc_yr) = (0.0, 0.0);
        for row in 0..nlat {
            let latitude = (row as f64 + 0.5) / nlat as f64 * std::f64::consts::PI
                - std::f64::consts::FRAC_PI_2;
            let surface_par = marine::annual_mean_surface_par(
                self.canon.solar_constant_w_m2,
                obliquity,
                latitude,
            );
            let south = (row as f64 / nlat as f64 - 0.5) * std::f64::consts::PI;
            let north = ((row + 1) as f64 / nlat as f64 - 0.5) * std::f64::consts::PI;
            let cell_area_m2 = radius * radius * dlon * (north.sin() - south.sin());
            for col in 0..nlon {
                let elevation_m = *elevation.get(row, col);
                if elevation_m < 0.0 {
                    let npp = marine::marine_npp_kgc_m2_yr(
                        &marine_inputs,
                        ocean.columns.get(row, col).surface_temp,
                        row,
                        col,
                        surface_par,
                    )
                    .min(cap);
                    ocean_kgc_yr += npp * cell_area_m2;
                    field.push(npp);
                } else {
                    let temperature_c = climate.surface_temperature.get(row, col) - 273.15;
                    let precipitation_mm_yr =
                        weather.precipitation.get(row, col).max(0.0) * DAYS_PER_YEAR;
                    let npp = (miami_npp_kgc_m2_yr(
                        temperature_c,
                        precipitation_mm_yr,
                        CARBON_FRACTION_OF_DRY_MASS,
                    ) * nutrient_limit)
                        .min(cap);
                    land_kgc_yr += npp * cell_area_m2;
                    field.push(npp);
                }
            }
        }
        self.npp_field_kgc_m2_yr = field;
        self.statistics.terrestrial_npp_kgc_yr = land_kgc_yr;
        self.statistics.marine_npp_kgc_yr = ocean_kgc_yr;
        self.statistics.current_npp_kgc_yr = land_kgc_yr + ocean_kgc_yr;
    }

    /// Canon nitrogen and phosphorus limitation of land production,
    /// `f_N · f_P` with `f_X = X / (X + K_X)` (`BIOSPHERE_EQUATIONS.md`
    /// § 3.1, 3.5–3.6; `K_N`, `K_P` from `BIOSPHERE_CONSTANTS.md` § 4.2).
    ///
    /// `X` is the bioavailable soil pool per m² of land. Before the world has
    /// sized the pools, it is the canon's mean soil density, the value the
    /// pools start from.
    fn land_nutrient_limitation(
        &self,
        elevation: &mk_core::grid::Grid2<f64>,
        radius_m: f64,
        dlon: f64,
    ) -> f64 {
        use crate::conservation::{SOIL_NITROGEN_DENSITY_KG_M2, SOIL_PHOSPHORUS_DENSITY_KG_M2};
        /// Nitrogen limitation half-saturation, kg N/m² (§ 4.2).
        const K_N_KG_M2: f64 = 0.0048;
        /// Phosphorus limitation half-saturation, kg P/m² (§ 4.2).
        const K_P_KG_M2: f64 = 2.8e-4;

        let (nlat, nlon) = (elevation.nlat(), elevation.nlon());
        let mut land_m2 = 0.0;
        for row in 0..nlat {
            let south = (row as f64 / nlat as f64 - 0.5) * std::f64::consts::PI;
            let north = ((row + 1) as f64 / nlat as f64 - 0.5) * std::f64::consts::PI;
            let cell_area_m2 = radius_m * radius_m * dlon * (north.sin() - south.sin());
            for col in 0..nlon {
                if *elevation.get(row, col) > 0.0 {
                    land_m2 += cell_area_m2;
                }
            }
        }
        let density = |pool: Option<f64>, canon_density: f64| match pool {
            Some(kg) if land_m2 > 0.0 => (kg / land_m2).max(0.0),
            _ => canon_density,
        };
        let nitrogen = density(self.soil_nitrogen_kg, SOIL_NITROGEN_DENSITY_KG_M2);
        let phosphorus = density(self.soil_phosphorus_kg, SOIL_PHOSPHORUS_DENSITY_KG_M2);
        nitrogen / (nitrogen + K_N_KG_M2) * phosphorus / (phosphorus + K_P_KG_M2)
    }

    fn update_statistics(&mut self) {
        self.statistics.total_species = self.species.len();
        self.statistics.total_population = self.species.iter().map(|s| s.population_size).sum();
        self.statistics.marine_species_count =
            self.species.iter().filter(|s| s.is_marine()).count();
        self.statistics.terrestrial_species_count =
            self.species.iter().filter(|s| s.is_terrestrial()).count();
        self.statistics.floral_species_count = self
            .species
            .iter()
            .filter(|s| matches!(s.category, SpeciesCategory::Flower | SpeciesCategory::Tree))
            .count();
        self.statistics.pollinator_species_count = self
            .species
            .iter()
            .filter(|s| {
                matches!(
                    s.category,
                    SpeciesCategory::Bee | SpeciesCategory::Insect | SpeciesCategory::Bird
                )
            })
            .count();
        self.statistics.tree_species_count = self
            .species
            .iter()
            .filter(|s| matches!(s.category, SpeciesCategory::Tree))
            .count();
        self.statistics.mountain_species_count = self
            .species
            .iter()
            .filter(|s| {
                matches!(
                    s.terrain_affinity,
                    TerrainAffinity::Mountain | TerrainAffinity::Cliff
                )
            })
            .count();
        self.statistics.volcanic_species_count = self
            .species
            .iter()
            .filter(|s| matches!(s.terrain_affinity, TerrainAffinity::Volcanic))
            .count();
        self.statistics.river_species_count = self
            .species
            .iter()
            .filter(|s| {
                matches!(
                    s.terrain_affinity,
                    TerrainAffinity::River | TerrainAffinity::Wetland
                )
            })
            .count();

        // Average body mass (from Phase 3 genetics)
        let total_mass: f64 = self
            .species
            .iter()
            .map(|s| s.population_size as f64 * s.representative_genome.body_mass_kg())
            .sum();
        self.statistics.total_biomass_kg = total_mass;

        if !self.species.is_empty() {
            self.statistics.average_intelligence = self
                .species
                .iter()
                .map(|s| s.representative_genome.intelligence_index())
                .sum::<f64>()
                / self.species.len() as f64;
            self.statistics.maximum_intelligence = self
                .species
                .iter()
                .map(|s| s.representative_genome.intelligence_index())
                .fold(0.0, f64::max);
            let diversity_factor = (self.statistics.total_species as f64 / 2048.0).min(1.0);
            let balance_denominator = (self.statistics.total_species.max(1)) as f64;
            let balance = 1.0
                - ((self.statistics.marine_species_count as f64
                    - self.statistics.terrestrial_species_count as f64)
                    .abs()
                    / balance_denominator);
            let terrain_diversity = ((self.statistics.mountain_species_count
                + self.statistics.volcanic_species_count
                + self.statistics.river_species_count) as f64
                / balance_denominator)
                .min(1.0);
            let flora_support =
                (self.statistics.floral_species_count as f64 / balance_denominator).min(1.0);
            self.statistics.stability_index = (0.25
                + diversity_factor * 0.3
                + balance.max(0.0) * 0.2
                + terrain_diversity * 0.15
                + flora_support * 0.1)
                .min(1.0);
        }
    }

    /// Check the biosphere's structural invariants: unique species ids, no
    /// retained zero-population species, statistics consistent with the
    /// live species list, and no species above the pre-sapient
    /// intelligence ceiling.
    pub fn validate(&self) -> Result<(), String> {
        let mut seen = std::collections::BTreeSet::new();
        for species in &self.species {
            if !seen.insert(species.species_id) {
                return Err(format!("duplicate species id {}", species.species_id));
            }
            if species.population_size == 0 {
                return Err(format!(
                    "extinct species {} still listed",
                    species.species_id
                ));
            }
            let intelligence = species.representative_genome.intelligence_index();
            if intelligence > genetics::PRE_SAPIENT_CEILING {
                return Err(format!(
                    "species {} intelligence {intelligence} exceeds the pre-sapient ceiling",
                    species.species_id
                ));
            }
        }
        if self.statistics.total_species != self.species.len() {
            return Err(format!(
                "statistics report {} species but {} are live",
                self.statistics.total_species,
                self.species.len()
            ));
        }
        Ok(())
    }

    pub fn get_statistics(&self) -> &BiosphereStatistics {
        &self.statistics
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn soil_nutrients_limit_land_production_by_the_canon_michaelis_menten() {
        let canon = Arc::new(CanonLocked::default());
        let spec = mk_core::grid::GridSpec::new(4, 8);
        let land = mk_core::grid::Grid2::new(&spec, 500.0);
        let dlon = std::f64::consts::TAU / 8.0;
        let radius = canon.planet_radius_m;
        let mut biosphere = BiosphereSystem::new(canon, 1);
        // At the canon soil densities (0.62 kg N, 0.041 kg P per m²), with
        // K_N 0.0048 and K_P 2.8e-4: f_N·f_P ≈ 0.9923 × 0.9932.
        let full = biosphere.land_nutrient_limitation(&land, radius, dlon);
        let expected = 0.62 / (0.62 + 0.0048) * 0.041 / (0.041 + 2.8e-4);
        assert!((full - expected).abs() < 1e-12, "{full}");
        // A soil drawn down to its half-saturation halves that nutrient's
        // factor.
        let land_m2 = 4.0 * std::f64::consts::PI * radius * radius;
        biosphere.soil_nitrogen_kg = Some(0.0048 * land_m2);
        biosphere.soil_phosphorus_kg = Some(0.041 * land_m2);
        let halved = biosphere.land_nutrient_limitation(&land, radius, dlon);
        assert!(
            (halved - 0.5 * 0.041 / (0.041 + 2.8e-4)).abs() < 1e-9,
            "{halved}"
        );
    }

    #[test]
    fn miami_model_matches_its_published_limits() {
        // Warm and wet: both limits approach the 3000 g/m²/yr ceiling.
        let lush = miami_npp_kgc_m2_yr(28.0, 3000.0, 0.475);
        assert!(lush > 1.2 && lush < 1.43, "tropical NPP {lush}");
        // Desert: water-limited.
        let desert = miami_npp_kgc_m2_yr(25.0, 50.0, 0.475);
        assert!(desert < 0.05, "desert NPP {desert}");
        // Frozen: temperature-limited.
        assert!(miami_npp_kgc_m2_yr(-30.0, 800.0, 0.475) < miami_npp_kgc_m2_yr(15.0, 800.0, 0.475));
    }

    fn calm_ocean(spec: &mk_core::grid::GridSpec) -> crate::ocean::OceanState {
        crate::ocean::OceanState {
            columns: mk_core::grid::Grid2::new(
                spec,
                crate::ocean::OceanColumn::new(4000.0, 293.15, 35.0),
            ),
            currents: mk_core::grid::Grid2::new(
                spec,
                crate::ocean::OceanVelocity {
                    u_east: 0.0,
                    v_north: 0.0,
                },
            ),
            heat_absorbed_w_m2: 0.0,
            heat_released_w_m2: 0.0,
            water_gained_mm_day: 0.0,
            water_lost_mm_day: 0.0,
        }
    }

    #[test]
    fn primary_production_covers_land_and_sea_and_integrates_area() {
        let spec = mk_core::grid::GridSpec::new(4, 8);
        let climate = crate::climate::ClimateState::new(&spec, 293.15);
        let mut weather = crate::weather::WeatherState::new(&spec);
        for (_, _, p) in weather.precipitation.indexed_iter_mut() {
            *p = 3.0;
        }
        let mut elevation = mk_core::grid::Grid2::new(&spec, 100.0);
        // One deep open-ocean cell and one shelf cell in the same row.
        *elevation.get_mut(1, 0) = -4000.0;
        *elevation.get_mut(1, 1) = -50.0;

        let mut biosphere = BiosphereSystem::default();
        biosphere.update_primary_production(&climate, &weather, &calm_ocean(&spec), &elevation);
        let field = &biosphere.npp_field_kgc_m2_yr;
        assert_eq!(field.len(), 32);
        let (open_ocean, shelf, land) = (field[8], field[9], field[10]);
        assert!(
            open_ocean > 0.0,
            "stratified ocean keeps a regenerated baseline"
        );
        assert!(shelf > open_ocean, "shelf mixing supplies nutrients");
        assert!(land > 0.0 && land <= biosphere.canon.npp_max_kgc_m2_yr);

        let stats = &biosphere.statistics;
        assert!(stats.marine_npp_kgc_yr > 0.0 && stats.terrestrial_npp_kgc_yr > 0.0);
        assert!(
            (stats.current_npp_kgc_yr - stats.marine_npp_kgc_yr - stats.terrestrial_npp_kgc_yr)
                .abs()
                < 1e-6 * stats.current_npp_kgc_yr
        );
        // Uniform land NPP over all but two cells: the land total is within
        // a couple of cells' share of the sphere of land_area × land.
        let r = biosphere.canon.planet_radius_m;
        let land_area = 4.0 * std::f64::consts::PI * r * r * (30.0 / 32.0);
        assert!(
            (stats.terrestrial_npp_kgc_yr / (land_area * land) - 1.0).abs() < 0.1,
            "land total {}",
            stats.terrestrial_npp_kgc_yr
        );
    }

    #[test]
    fn test_biosphere_creation() {
        let canon = CanonLocked::default();
        let biosphere = BiosphereSystem::new(std::sync::Arc::new(canon), 12345);
        assert!(biosphere.species.is_empty());
    }

    #[test]
    fn validate_rejects_broken_invariants() {
        let canon = std::sync::Arc::new(mk_core::canon::CanonLocked::default());
        let mut biosphere = BiosphereSystem::new(canon, 7);
        biosphere.initialize().expect("founding species");
        assert!(biosphere.validate().is_ok());

        let mut duplicated = biosphere.clone();
        let copy = duplicated.species[0].clone();
        duplicated.species.push(copy);
        duplicated.update_statistics();
        assert!(duplicated.validate().is_err());

        let mut stale = biosphere.clone();
        stale.species[0].population_size = 0;
        assert!(stale.validate().is_err());
    }
}
