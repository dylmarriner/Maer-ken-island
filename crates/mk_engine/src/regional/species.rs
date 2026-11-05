//! Upstream's species at island scale (Phase 3 Task 1b).
//!
//! Upstream's biosphere generates its founding species for a whole planet:
//! 4,096 of them in fifteen guilds, each with a terrain affinity, a body
//! plan and a planetary population spread over a share of the planet's
//! surface (`BiosphereSystem::founding_species`; D7 accepts that they are
//! Marr'Kena's species and not Earth's). This places the same species on
//! the island's own ground and gives each an island-sized abundance,
//! without inventing a number:
//!
//! - **Where.** Each terrain affinity is a set of medium cells, decided by
//!   the island's own biome grid and coastline ([`habitat_of`]). Upstream
//!   has no landform model to borrow, so its landform affinities are read
//!   through the biomes (D55): `Mountain` is alpine ground and montane
//!   forest, `Hill` is every vegetated cell that is not forest (the open
//!   ground its flowers and omnivores live on), `Cliff` is the land along
//!   the sea. Terrestrial species never stand in the sea and marine ones
//!   never on land.
//! - **How much, for plants.** The island already knows its producer
//!   carbon cell by cell (`RegionalEcologyState::biomass_kgc_m2`). The
//!   plant species are a *view* of that field: each cell's carbon is
//!   shared among the plant species that live there in proportion to
//!   upstream's populations, so the species' carbon always sums to the
//!   field's, before and after any step, and nothing is counted twice.
//! - **How much, for grazers.** The same, from the grazer carbon pool
//!   (`herbivore_kgc_m2`) on the grazers' plains. Grazer carbon on ground
//!   no grazer species lives on (forest, mountain) stays unassigned and is
//!   reported as such rather than spread over species that do not live
//!   there.
//! - **How much, for everything else.** The island has no carbon pool for
//!   birds, insects, predators, fish or amphibians yet, so these keep
//!   upstream's own density -- its population over its range's share of
//!   the planet -- times the island's habitat area. Upstream's densities
//!   are not calibrated to Earth's (D55); this is upstream's figure,
//!   scaled, and said to be.
//!
//! Derived, not stored: the species are regenerated from the scenario's
//! seed and the fields whenever they are asked for, so snapshots and the
//! week digest do not move, and an older snapshot needs no migration.

use std::collections::BTreeMap;
use std::sync::Arc;

use mk_core::biomes::BiomeType;
use mk_core::canon::CanonLocked;
use mk_core::grid::Grid2;
use mk_island::{DomainLevel, IslandDomain};

use super::ecology::{residence_years, RegionalEcologyState};
use super::physical::RegionalPhysicalState;
use crate::biosphere::{BiosphereSystem, SpeciesCategory, TerrainAffinity};

const MEDIUM: DomainLevel = DomainLevel::Medium;

/// Every terrain affinity upstream has, in a fixed order: the bit each
/// takes in a cell's habitat mask.
pub const AFFINITIES: [TerrainAffinity; 10] = [
    TerrainAffinity::OpenOcean,
    TerrainAffinity::ReefSea,
    TerrainAffinity::River,
    TerrainAffinity::Wetland,
    TerrainAffinity::Hill,
    TerrainAffinity::Mountain,
    TerrainAffinity::Volcanic,
    TerrainAffinity::ForestCanopy,
    TerrainAffinity::Plains,
    TerrainAffinity::Cliff,
];

/// `affinity`'s place in [`AFFINITIES`].
fn slot(affinity: &TerrainAffinity) -> usize {
    AFFINITIES
        .iter()
        .position(|a| a == affinity)
        .expect("AFFINITIES lists every TerrainAffinity")
}

fn bit(affinity: &TerrainAffinity) -> u16 {
    1 << slot(affinity)
}

/// The forest biomes: where upstream's canopy guilds (trees, birds, bees)
/// live.
pub fn is_forest(biome: BiomeType) -> bool {
    use BiomeType::*;
    matches!(
        biome,
        TropicalRainforest
            | TropicalDryForest
            | TemperateForest
            | BorealForest
            | Woodland
            | MontaneForest
    )
}

/// Ground where producers stand: every biome Whittaker & Likens give a
/// standing biomass for (`residence_years` above zero).
fn is_vegetated(biome: BiomeType) -> bool {
    residence_years(biome) > 0.0
}

/// What decides a cell's habitats besides its biome.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CellSetting {
    /// Land with the sea beside it.
    pub on_the_coast: bool,
    /// Land a river runs through all year: perennial in the island's
    /// simulated year of flows, with a mean at or above the hydrology's own
    /// river threshold (`RiverFlowRecord::is_river`). A 2 km cell is a
    /// `River` biome only when the channel fills half of it, which no
    /// river on this island does, but a river running through a forest
    /// cell is still a river to what lives in it.
    pub has_a_river: bool,
    /// Below sea level: the sea, not a lake. The classifier calls both
    /// shallow sea and lakes `CoastalWaters`, and only one is the sea.
    pub in_the_sea: bool,
}

/// Whether a cell of `biome` in `setting` is habitat for `affinity`.
///
/// Water affinities are their own biomes. `Plains` is open lowland
/// vegetation; `Hill` is every vegetated cell that is not forest, so the
/// forest's trees and the open ground's flowers between them cover every
/// cell plants stand on; `Mountain` is the alpine and montane biomes the
/// classifier already decides from elevation; `Cliff` is land on the
/// sea's edge; `River` is a river biome or a land cell a river runs
/// through.
pub fn habitat_of(affinity: &TerrainAffinity, biome: BiomeType, setting: CellSetting) -> bool {
    use BiomeType::*;
    match affinity {
        // Pelagic life swims over reefs and shallow coastal sea too, as
        // upstream's own biome-to-affinity map has it
        // (`interventions::affinities_for_biome`), but not in a lake.
        TerrainAffinity::OpenOcean => {
            matches!(biome, DeepOcean | ShallowOcean | ReefSea)
                || (biome == CoastalWaters && setting.in_the_sea)
        }
        TerrainAffinity::ReefSea => biome == ReefSea,
        TerrainAffinity::River => biome == River || (setting.has_a_river && biome.is_terrestrial()),
        TerrainAffinity::Wetland => biome == Wetland,
        TerrainAffinity::Volcanic => biome == Volcanic,
        TerrainAffinity::ForestCanopy => is_forest(biome),
        TerrainAffinity::Mountain => matches!(biome, Alpine | MontaneForest),
        TerrainAffinity::Plains => matches!(
            biome,
            Grassland | Savanna | Shrubland | SemiDesert | Desert | Tundra
        ),
        TerrainAffinity::Hill => is_vegetated(biome) && !is_forest(biome),
        TerrainAffinity::Cliff => setting.on_the_coast && biome.is_terrestrial(),
    }
}

/// Which island field a species' abundance is a share of.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Abundance {
    /// A share of the producer carbon on its habitat.
    ProducerCarbon,
    /// A share of the grazer carbon on its habitat.
    GrazerCarbon,
    /// Upstream's planetary density times the island's habitat area.
    UpstreamDensity,
}

/// One of upstream's species, on the island.
#[derive(Debug, Clone)]
pub struct IslandSpecies {
    pub species_id: u64,
    pub name: String,
    pub category: SpeciesCategory,
    pub affinity: TerrainAffinity,
    /// Upstream's body mass for the species (kg).
    pub body_mass_kg: f64,
    /// Upstream's planetary population.
    pub upstream_population: u64,
    /// Upstream's population over its range: `population / (range ×
    /// planet surface)`, individuals per m².
    pub upstream_density_per_m2: f64,
    pub abundance: Abundance,
}

/// How much of a species a resource node holds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NodeStock {
    /// Its carbon on the cell (kg C), for a grazer.
    CarbonKgc(f64),
    /// How many live on the cell, for an animal counted by density.
    Head(f64),
}

/// One species' animals on one medium cell, as something to hunt or fish.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AnimalNode {
    pub cell: usize,
    /// Index into [`IslandBiota::species`].
    pub species: usize,
    pub stock: NodeStock,
}

/// A marine aquatic species' population and live biomass on one medium cell.
///
/// This is a read-only projection of upstream density, not a second stock.
/// Biomass is in tonnes for the fishery model's unit convention; no carrying
/// capacity, resilience class or harvest rate is inferred here.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MarineAquaticNode {
    pub cell: usize,
    /// Index into [`IslandBiota::species`].
    pub species: usize,
    /// Individuals represented by this cell's share of upstream density.
    pub individuals: f64,
    /// Live biomass represented by those individuals (tonnes).
    pub biomass_t: f64,
}

/** A marine population-to-biomass projection must remain finite and physically valid. */
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarineProjectionError {
    InvalidCellIndex {
        cell: usize,
    },
    InvalidCellArea,
    InvalidSpeciesIndex {
        index: usize,
    },
    StaleSpeciesIndex {
        index: usize,
        expected_species_id: u64,
        actual_species_id: u64,
    },
    /// The cached marine index no longer matches this species' surveyed category or affinity.
    StaleSpeciesMetadata {
        index: usize,
        species_id: u64,
    },
    InvalidMarineAbundance {
        index: usize,
        species_id: u64,
    },
    DuplicateSpeciesId {
        first_index: usize,
        duplicate_index: usize,
        species_id: u64,
    },
    InvalidDensity {
        species_id: u64,
    },
    InvalidBodyMass {
        species_id: u64,
    },
    NonFiniteIndividuals {
        species_id: u64,
    },
    NonFiniteBiomass {
        species_id: u64,
    },
}

impl std::fmt::Display for MarineProjectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidCellIndex { cell } => {
                write!(f, "marine projection cell {cell} is outside the habitat grid")
            }
            Self::InvalidCellArea => {
                f.write_str("marine projection cell area must be finite and positive")
            }
            Self::InvalidSpeciesIndex { index } => write!(
                f,
                "marine projection index {index} is outside the species catalogue"
            ),
            Self::StaleSpeciesIndex {
                index,
                expected_species_id,
                actual_species_id,
            } => write!(
                f,
                "marine projection index {index} expected species {expected_species_id} but found {actual_species_id}"
            ),
            Self::StaleSpeciesMetadata { index, species_id } => write!(
                f,
                "marine projection index {index} for species {species_id} no longer matches its surveyed aquatic metadata"
            ),
            Self::InvalidMarineAbundance { index, species_id } => write!(
                f,
                "marine aquatic species {species_id} at index {index} must use upstream-density abundance"
            ),
            Self::DuplicateSpeciesId {
                first_index,
                duplicate_index,
                species_id,
            } => write!(
                f,
                "species catalogue repeats id {species_id} at indices {first_index} and {duplicate_index}"
            ),
            Self::InvalidDensity { species_id } => write!(
                f,
                "marine species {species_id} has invalid upstream density"
            ),
            Self::InvalidBodyMass { species_id } => {
                write!(f, "marine species {species_id} has invalid body mass")
            }
            Self::NonFiniteIndividuals { species_id } => {
                write!(f, "marine species {species_id} population overflowed")
            }
            Self::NonFiniteBiomass { species_id } => {
                write!(f, "marine species {species_id} biomass overflowed")
            }
        }
    }
}

impl std::error::Error for MarineProjectionError {}

/// Upstream's species on the island's ground.
#[derive(Debug, Clone)]
pub struct IslandBiota {
    // Keep the catalogue private: the cached marine index and carbon weights
    // must never be bypassed by direct external Vec mutation.
    species: Vec<IslandSpecies>,
    /// Stable indices of OpenOcean/ReefSea aquatic species, built once at survey
    /// so a cell lookup does not rescan every founding species.
    marine_aquatic_species: Vec<MarineAquaticSpeciesIndex>,
    /// Per medium cell, a bit per [`AFFINITIES`] entry it is habitat for.
    habitat: Vec<u16>,
    cell_area_m2: f64,
    /// Per affinity, the summed upstream population of the producer and of
    /// the grazer species with that affinity: the denominators of a cell's
    /// shares.
    producer_weight: [f64; 10],
    grazer_weight: [f64; 10],
}

fn abundance_of(category: &SpeciesCategory) -> Abundance {
    match category {
        SpeciesCategory::Tree | SpeciesCategory::Flower => Abundance::ProducerCarbon,
        SpeciesCategory::Herbivore => Abundance::GrazerCarbon,
        _ => Abundance::UpstreamDensity,
    }
}

/// A cached species index carries its identity as well as its position.
/// IslandBiota exposes its species catalogue for read access, so this guard
/// prevents a later reorder through a mutable owner from silently aliasing a
/// different species at the same vector index.
#[derive(Debug, Clone, PartialEq, Eq)]
struct MarineAquaticSpeciesIndex {
    index: usize,
    species_id: u64,
    affinity: TerrainAffinity,
}

/// Species eligible for the read-only marine biomass projection. Keeping this
/// stable, identity-checked index avoids scanning all founding species for every
/// single-cell dashboard request.
fn marine_aquatic_species_indices(species: &[IslandSpecies]) -> Vec<MarineAquaticSpeciesIndex> {
    species
        .iter()
        .enumerate()
        .filter(|(_, s)| {
            s.category == SpeciesCategory::Aquatic
                && matches!(
                    &s.affinity,
                    TerrainAffinity::OpenOcean | TerrainAffinity::ReefSea
                )
        })
        .map(|(index, s)| MarineAquaticSpeciesIndex {
            index,
            species_id: s.species_id,
            affinity: s.affinity.clone(),
        })
        .collect()
}

/// Rebuild the per-affinity carbon denominators after a controlled catalogue edit.
fn species_weights(species: &[IslandSpecies]) -> ([f64; 10], [f64; 10]) {
    let mut producer_weight = [0.0; 10];
    let mut grazer_weight = [0.0; 10];
    for s in species {
        let at = slot(&s.affinity);
        match s.abundance {
            Abundance::ProducerCarbon => producer_weight[at] += s.upstream_population as f64,
            Abundance::GrazerCarbon => grazer_weight[at] += s.upstream_population as f64,
            Abundance::UpstreamDensity => {}
        }
    }
    (producer_weight, grazer_weight)
}

/// Validate catalogue-wide identity and the marine projection's abundance source.
fn validate_species_catalogue(species: &[IslandSpecies]) -> Result<(), MarineProjectionError> {
    let mut first_index_by_id = BTreeMap::new();
    for (index, s) in species.iter().enumerate() {
        if let Some(first_index) = first_index_by_id.get(&s.species_id).copied() {
            return Err(MarineProjectionError::DuplicateSpeciesId {
                first_index,
                duplicate_index: index,
                species_id: s.species_id,
            });
        }
        first_index_by_id.insert(s.species_id, index);
        if s.category == SpeciesCategory::Aquatic
            && matches!(
                &s.affinity,
                TerrainAffinity::OpenOcean | TerrainAffinity::ReefSea
            )
            && s.abundance != Abundance::UpstreamDensity
        {
            return Err(MarineProjectionError::InvalidMarineAbundance {
                index,
                species_id: s.species_id,
            });
        }
    }
    Ok(())
}

/// The 64-bit seed upstream's biosphere takes, from the scenario's 32
/// bytes: the first eight, little-endian, as `BiosphereSystem::new` lays
/// a `u64` into its own seed.
fn biosphere_seed(seed: &[u8; 32]) -> u64 {
    let mut first = [0u8; 8];
    first.copy_from_slice(&seed[..8]);
    u64::from_le_bytes(first)
}

impl IslandBiota {
    /// Read-only view of the derived species catalogue.
    pub fn species(&self) -> &[IslandSpecies] {
        &self.species
    }

    /// Replace the derived catalogue while rebuilding every cache that depends
    /// on species membership or abundance. Duplicate identities and marine
    /// aquatic entries using a carbon-pool abundance are rejected before state changes.
    pub fn replace_species_catalogue(
        &mut self,
        species: Vec<IslandSpecies>,
    ) -> Result<(), MarineProjectionError> {
        validate_species_catalogue(&species)?;
        let marine_aquatic_species = marine_aquatic_species_indices(&species);
        let (producer_weight, grazer_weight) = species_weights(&species);

        self.species = species;
        self.marine_aquatic_species = marine_aquatic_species;
        self.producer_weight = producer_weight;
        self.grazer_weight = grazer_weight;
        Ok(())
    }

    /// Place upstream's founding species, generated from the scenario's
    /// `seed`, on the island's habitats.
    pub fn survey(
        canon: Arc<CanonLocked>,
        seed: &[u8; 32],
        domain: &IslandDomain,
        ecology: &RegionalEcologyState,
        physical: &RegionalPhysicalState,
    ) -> Self {
        let elevation_m = &physical.geophysics.elevation_m;
        let planet_m2 = 4.0 * std::f64::consts::PI * canon.planet_radius_m.powi(2);
        let biosphere = BiosphereSystem::new(canon, biosphere_seed(seed));
        let species: Vec<IslandSpecies> = biosphere
            .founding_species()
            .into_iter()
            .map(|s| IslandSpecies {
                body_mass_kg: s.representative_genome.body_mass_kg(),
                upstream_density_per_m2: s.population_size as f64
                    / (s.geographic_range.max(f64::MIN_POSITIVE) * planet_m2),
                abundance: abundance_of(&s.category),
                species_id: s.species_id,
                name: s.species_name,
                category: s.category,
                affinity: s.terrain_affinity,
                upstream_population: s.population_size,
            })
            .collect();
        validate_species_catalogue(&species).unwrap_or_else(|error| {
            panic!("invalid upstream species catalogue: {error}");
        });
        let marine_aquatic_species = marine_aquatic_species_indices(&species);

        let (rows, cols) = (domain.rows(MEDIUM), domain.cols(MEDIUM));
        let is_sea = |r: usize, c: usize| *elevation_m.get(r, c) <= 0.0;
        let mut habitat = vec![0u16; rows * cols];
        for r in 0..rows {
            for c in 0..cols {
                let on_the_coast = !is_sea(r, c)
                    && [(-1, 0), (1, 0), (0, -1), (0, 1)].iter().any(|(dr, dc)| {
                        let (nr, nc) = (r as isize + dr, c as isize + dc);
                        nr >= 0
                            && nc >= 0
                            && (nr as usize) < rows
                            && (nc as usize) < cols
                            && is_sea(nr as usize, nc as usize)
                    });
                let setting = CellSetting {
                    on_the_coast,
                    in_the_sea: is_sea(r, c),
                    has_a_river: !is_sea(r, c) && physical.river_flows.is_river(r * cols + c),
                };
                let biome = *ecology.biome_grid.get(r, c);
                let mut mask = 0u16;
                for affinity in &AFFINITIES {
                    if habitat_of(affinity, biome, setting) {
                        mask |= bit(affinity);
                    }
                }
                habitat[r * cols + c] = mask;
            }
        }

        let (producer_weight, grazer_weight) = species_weights(&species);

        Self {
            species,
            marine_aquatic_species,
            habitat,
            cell_area_m2: domain.cell_area_m2(MEDIUM),
            producer_weight,
            grazer_weight,
        }
    }

    /// Whether medium cell `cell` (`row * cols + col`) is habitat for
    /// `affinity`.
    pub fn is_habitat(&self, cell: usize, affinity: &TerrainAffinity) -> bool {
        self.habitat
            .get(cell)
            .is_some_and(|mask| mask & bit(affinity) != 0)
    }

    /// The medium cells that are habitat for `affinity`.
    pub fn habitat_cells(&self, affinity: &TerrainAffinity) -> impl Iterator<Item = usize> + '_ {
        let b = bit(affinity);
        self.habitat
            .iter()
            .enumerate()
            .filter(move |(_, mask)| *mask & b != 0)
            .map(|(i, _)| i)
    }

    /// The island's area of `affinity`'s habitat (m²).
    pub fn habitat_area_m2(&self, affinity: &TerrainAffinity) -> f64 {
        self.habitat_cells(affinity).count() as f64 * self.cell_area_m2
    }

    /// The species living in medium cell `cell`.
    pub fn species_in(&self, cell: usize) -> impl Iterator<Item = &IslandSpecies> + '_ {
        let mask = self.habitat.get(cell).copied().unwrap_or(0);
        self.species
            .iter()
            .filter(move |s| mask & bit(&s.affinity) != 0)
    }

    /// Carbon per unit of upstream population, for each affinity's species
    /// of one kind (kg C): one pass over the island. Each cell's carbon in
    /// `field` is divided among the species of that kind living there by
    /// upstream population, so a species' carbon is its own population
    /// times its affinity's entry here, and the species' carbon sums to the
    /// field's wherever any of them lives.
    fn carbon_per_weight(&self, field: &Grid2<f64>, weights: &[f64; 10]) -> [f64; 10] {
        let mut per = [0.0; 10];
        for (mask, carbon) in self.habitat.iter().zip(field.data()) {
            if *mask == 0 || *carbon == 0.0 {
                continue;
            }
            let denominator: f64 = (0..AFFINITIES.len())
                .filter(|i| mask & (1 << i) != 0)
                .map(|i| weights[i])
                .sum();
            if denominator > 0.0 {
                let unit = carbon * self.cell_area_m2 / denominator;
                for (i, p) in per.iter_mut().enumerate() {
                    if mask & (1 << i) != 0 && weights[i] > 0.0 {
                        *p += unit;
                    }
                }
            }
        }
        per
    }

    fn per_weight(&self, kind: Abundance, ecology: &RegionalEcologyState) -> Option<[f64; 10]> {
        match kind {
            Abundance::ProducerCarbon => {
                Some(self.carbon_per_weight(&ecology.biomass_kgc_m2, &self.producer_weight))
            }
            Abundance::GrazerCarbon => {
                Some(self.carbon_per_weight(&ecology.herbivore_kgc_m2, &self.grazer_weight))
            }
            Abundance::UpstreamDensity => None,
        }
    }

    /// The species' carbon on the island (kg C), for a plant or a grazer;
    /// `None` for a species whose abundance is upstream's density. Asking
    /// for many is cheaper through [`Self::carbon_by_species`].
    pub fn carbon_kgc(&self, index: usize, ecology: &RegionalEcologyState) -> Option<f64> {
        let s = self.species.get(index)?;
        let per = self.per_weight(s.abundance, ecology)?;
        Some(s.upstream_population as f64 * per[slot(&s.affinity)])
    }

    /// Every species' carbon (kg C), `None` where its abundance is
    /// upstream's density, in the order of [`Self::species`].
    pub fn carbon_by_species(&self, ecology: &RegionalEcologyState) -> Vec<Option<f64>> {
        let producers = self.per_weight(Abundance::ProducerCarbon, ecology);
        let grazers = self.per_weight(Abundance::GrazerCarbon, ecology);
        self.species
            .iter()
            .map(|s| {
                let per = match s.abundance {
                    Abundance::ProducerCarbon => producers,
                    Abundance::GrazerCarbon => grazers,
                    Abundance::UpstreamDensity => None,
                }?;
                Some(s.upstream_population as f64 * per[slot(&s.affinity)])
            })
            .collect()
    }

    /// How many of the species live on the island, by upstream's density
    /// over the island's habitat; `None` for a plant or a grazer, whose
    /// abundance is carbon ([`Self::carbon_kgc`]).
    pub fn population(&self, index: usize) -> Option<f64> {
        let s = self.species.get(index)?;
        (s.abundance == Abundance::UpstreamDensity)
            .then(|| s.upstream_density_per_m2 * self.habitat_area_m2(&s.affinity))
    }

    /// The animals (and fish) living in medium cell `cell`, as resource
    /// nodes: views of the species' abundance on that cell, never a store
    /// of their own. A grazer's node is its share of the cell's grazer
    /// carbon, split among the grazers there by upstream population as
    /// [`Self::carbon_kgc`] splits the island's; any other animal's is
    /// upstream's density over the cell. Plants are left out: their
    /// carbon is the cell's biomass, which felling and foraging already
    /// work.
    pub fn animal_nodes(&self, cell: usize, ecology: &RegionalEcologyState) -> Vec<AnimalNode> {
        let mask = self.habitat.get(cell).copied().unwrap_or(0);
        if mask == 0 {
            return Vec::new();
        }
        let grazer_kgc = ecology
            .herbivore_kgc_m2
            .data()
            .get(cell)
            .map_or(0.0, |c| c * self.cell_area_m2);
        let grazer_denominator: f64 = (0..AFFINITIES.len())
            .filter(|i| mask & (1 << i) != 0)
            .map(|i| self.grazer_weight[i])
            .sum();
        self.species
            .iter()
            .enumerate()
            .filter(|(_, s)| mask & bit(&s.affinity) != 0)
            .filter_map(|(index, s)| {
                let stock = match s.abundance {
                    Abundance::ProducerCarbon => return None,
                    Abundance::GrazerCarbon if grazer_denominator > 0.0 => NodeStock::CarbonKgc(
                        grazer_kgc * s.upstream_population as f64 / grazer_denominator,
                    ),
                    Abundance::GrazerCarbon => NodeStock::CarbonKgc(0.0),
                    Abundance::UpstreamDensity => {
                        NodeStock::Head(s.upstream_density_per_m2 * self.cell_area_m2)
                    }
                };
                Some(AnimalNode {
                    cell,
                    species: index,
                    stock,
                })
            })
            .collect()
    }

    /// Marine aquatic species on medium cell `cell`, with abundance converted
    /// to live biomass in tonnes. This is a projection of the same upstream
    /// density used by `animal_nodes`; it does not store or mutate a second
    /// population and deliberately does not invent fishery resilience,
    /// unexploited carrying capacity or catch-per-unit-effort.
    ///
    /// Only OpenOcean and ReefSea species are included. River-affinity aquatic
    /// species remain available through `animal_nodes`, but are not treated
    /// as part of the inshore marine fishery.
    /// Checked form of the marine projection. Invalid source values and
    /// arithmetic overflow are reported instead of leaking NaN/infinity into
    /// API responses, maps or later fishery calculations.
    pub fn try_marine_aquatic_nodes(
        &self,
        cell: usize,
    ) -> Result<Vec<MarineAquaticNode>, MarineProjectionError> {
        let mask = *self
            .habitat
            .get(cell)
            .ok_or(MarineProjectionError::InvalidCellIndex { cell })?;
        if !self.cell_area_m2.is_finite() || self.cell_area_m2 <= 0.0 {
            return Err(MarineProjectionError::InvalidCellArea);
        }
        if mask == 0 {
            return Ok(Vec::new());
        }

        let mut nodes = Vec::with_capacity(self.marine_aquatic_species.len());
        for indexed in &self.marine_aquatic_species {
            let species = indexed.index;
            let s = self
                .species
                .get(species)
                .ok_or(MarineProjectionError::InvalidSpeciesIndex { index: species })?;
            if s.species_id != indexed.species_id {
                return Err(MarineProjectionError::StaleSpeciesIndex {
                    index: species,
                    expected_species_id: indexed.species_id,
                    actual_species_id: s.species_id,
                });
            }
            // External callers can only replace the catalogue through
            // replace_species_catalogue, which rebuilds the index and weights.
            // Keep these identity/metadata checks as defence against internal drift.
            if s.category != SpeciesCategory::Aquatic
                || s.affinity != indexed.affinity
                || s.abundance != Abundance::UpstreamDensity
            {
                return Err(MarineProjectionError::StaleSpeciesMetadata {
                    index: species,
                    species_id: s.species_id,
                });
            }
            if mask & bit(&s.affinity) == 0 {
                continue;
            }
            if !s.upstream_density_per_m2.is_finite() || s.upstream_density_per_m2 < 0.0 {
                return Err(MarineProjectionError::InvalidDensity {
                    species_id: s.species_id,
                });
            }
            if !s.body_mass_kg.is_finite() || s.body_mass_kg <= 0.0 {
                return Err(MarineProjectionError::InvalidBodyMass {
                    species_id: s.species_id,
                });
            }

            let individuals = s.upstream_density_per_m2 * self.cell_area_m2;
            if !individuals.is_finite() || individuals < 0.0 {
                return Err(MarineProjectionError::NonFiniteIndividuals {
                    species_id: s.species_id,
                });
            }
            // Divide the larger factor before multiplying. This avoids an
            // overflowing kilogram product without underflowing a tiny body
            // mass before a large population can scale it back into range.
            let (scaled_individuals, scaled_mass_kg) = if individuals >= s.body_mass_kg {
                (individuals / 1_000.0, s.body_mass_kg)
            } else {
                (individuals, s.body_mass_kg / 1_000.0)
            };
            let biomass_t = scaled_individuals * scaled_mass_kg;
            if !biomass_t.is_finite() || biomass_t < 0.0 {
                return Err(MarineProjectionError::NonFiniteBiomass {
                    species_id: s.species_id,
                });
            }
            nodes.push(MarineAquaticNode {
                cell,
                species,
                individuals,
                biomass_t,
            });
        }
        Ok(nodes)
    }

    /// Convenience form for invariant-preserving internal callers. It fails
    /// fast if the upstream-derived projection is invalid; callers that need
    /// a recoverable error should use try_marine_aquatic_nodes.
    pub fn marine_aquatic_nodes(&self, cell: usize) -> Vec<MarineAquaticNode> {
        // Preserve the legacy convenience API's out-of-range empty result.
        // Callers that need a typed bounds error use try_marine_aquatic_nodes.
        if cell >= self.habitat.len() {
            return Vec::new();
        }
        self.try_marine_aquatic_nodes(cell)
            .unwrap_or_else(|error| panic!("invalid marine aquatic projection: {error}"))
    }

    /// The plants on medium cell `cell`, as resource nodes: each plant
    /// species' share of the cell's living biomass carbon, split among the
    /// plants there by upstream population as [`Self::carbon_kgc`] splits
    /// the island's. Views only: felling and foraging draw down the
    /// biomass itself.
    pub fn plant_nodes(&self, cell: usize, ecology: &RegionalEcologyState) -> Vec<AnimalNode> {
        let mask = self.habitat.get(cell).copied().unwrap_or(0);
        let carbon = ecology
            .biomass_kgc_m2
            .data()
            .get(cell)
            .map_or(0.0, |c| c * self.cell_area_m2);
        let denominator: f64 = (0..AFFINITIES.len())
            .filter(|i| mask & (1 << i) != 0)
            .map(|i| self.producer_weight[i])
            .sum();
        if mask == 0 || denominator <= 0.0 {
            return Vec::new();
        }
        self.species
            .iter()
            .enumerate()
            .filter(|(_, s)| s.abundance == Abundance::ProducerCarbon)
            .filter(|(_, s)| mask & bit(&s.affinity) != 0)
            .map(|(index, s)| AnimalNode {
                cell,
                species: index,
                stock: NodeStock::CarbonKgc(carbon * s.upstream_population as f64 / denominator),
            })
            .collect()
    }

    /// All the plant species' carbon (kg C).
    pub fn producer_carbon_kgc(&self, ecology: &RegionalEcologyState) -> f64 {
        self.sum_of(Abundance::ProducerCarbon, ecology)
    }

    /// All the grazer species' carbon (kg C).
    pub fn grazer_carbon_kgc(&self, ecology: &RegionalEcologyState) -> f64 {
        self.sum_of(Abundance::GrazerCarbon, ecology)
    }

    fn sum_of(&self, kind: Abundance, ecology: &RegionalEcologyState) -> f64 {
        let Some(per) = self.per_weight(kind, ecology) else {
            return 0.0;
        };
        self.species
            .iter()
            .filter(|s| s.abundance == kind)
            .map(|s| s.upstream_population as f64 * per[slot(&s.affinity)])
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOTHING: CellSetting = CellSetting {
        on_the_coast: false,
        has_a_river: false,
        in_the_sea: false,
    };
    const EVERYTHING: CellSetting = CellSetting {
        on_the_coast: true,
        has_a_river: true,
        in_the_sea: true,
    };

    fn species(
        species_id: u64,
        category: SpeciesCategory,
        affinity: TerrainAffinity,
    ) -> IslandSpecies {
        IslandSpecies {
            species_id,
            name: format!("species-{species_id}"),
            category,
            affinity,
            body_mass_kg: 1.0,
            upstream_population: 1,
            upstream_density_per_m2: 1.0,
            abundance: Abundance::UpstreamDensity,
        }
    }
    fn marine_biota(
        species: Vec<IslandSpecies>,
        marine_aquatic_species: Vec<MarineAquaticSpeciesIndex>,
        cell_area_m2: f64,
    ) -> IslandBiota {
        IslandBiota {
            species,
            marine_aquatic_species,
            habitat: vec![bit(&TerrainAffinity::OpenOcean)],
            cell_area_m2,
            producer_weight: [0.0; 10],
            grazer_weight: [0.0; 10],
        }
    }

    #[test]
    fn checked_marine_projection_rejects_out_of_range_cells() {
        let biota = marine_biota(Vec::new(), Vec::new(), 1.0);

        assert_eq!(
            biota.try_marine_aquatic_nodes(1),
            Err(MarineProjectionError::InvalidCellIndex { cell: 1 })
        );
        // Existing callers of the infallible compatibility method retain the
        // previous empty result for an out-of-range cell.
        assert!(biota.marine_aquatic_nodes(1).is_empty());
    }

    #[test]
    fn marine_projection_validates_cell_area_before_empty_habitat_short_circuit() {
        let mut invalid_area = marine_biota(Vec::new(), Vec::new(), 0.0);
        invalid_area.habitat[0] = 0;
        assert_eq!(
            invalid_area.try_marine_aquatic_nodes(0),
            Err(MarineProjectionError::InvalidCellArea)
        );

        let mut valid_empty_cell = marine_biota(Vec::new(), Vec::new(), 1.0);
        valid_empty_cell.habitat[0] = 0;
        assert_eq!(valid_empty_cell.try_marine_aquatic_nodes(0), Ok(Vec::new()));
    }

    #[test]
    fn marine_projection_rejects_invalid_density_and_body_mass() {
        let mut invalid_density = species(11, SpeciesCategory::Aquatic, TerrainAffinity::OpenOcean);
        invalid_density.upstream_density_per_m2 = f64::INFINITY;
        let biota = marine_biota(
            vec![invalid_density],
            vec![MarineAquaticSpeciesIndex {
                index: 0,
                species_id: 11,
                affinity: TerrainAffinity::OpenOcean,
            }],
            1.0,
        );
        assert_eq!(
            biota.try_marine_aquatic_nodes(0),
            Err(MarineProjectionError::InvalidDensity { species_id: 11 })
        );

        let mut invalid_mass = species(12, SpeciesCategory::Aquatic, TerrainAffinity::OpenOcean);
        invalid_mass.body_mass_kg = f64::NAN;
        let biota = marine_biota(
            vec![invalid_mass],
            vec![MarineAquaticSpeciesIndex {
                index: 0,
                species_id: 12,
                affinity: TerrainAffinity::OpenOcean,
            }],
            1.0,
        );
        assert_eq!(
            biota.try_marine_aquatic_nodes(0),
            Err(MarineProjectionError::InvalidBodyMass { species_id: 12 })
        );
    }

    #[test]
    fn marine_projection_rejects_population_and_biomass_overflow() {
        let mut invalid_population =
            species(21, SpeciesCategory::Aquatic, TerrainAffinity::OpenOcean);
        invalid_population.upstream_density_per_m2 = f64::MAX;
        let biota = marine_biota(
            vec![invalid_population],
            vec![MarineAquaticSpeciesIndex {
                index: 0,
                species_id: 21,
                affinity: TerrainAffinity::OpenOcean,
            }],
            2.0,
        );
        assert_eq!(
            biota.try_marine_aquatic_nodes(0),
            Err(MarineProjectionError::NonFiniteIndividuals { species_id: 21 })
        );

        let mut invalid_biomass = species(22, SpeciesCategory::Aquatic, TerrainAffinity::OpenOcean);
        invalid_biomass.upstream_density_per_m2 = 1.0e200;
        invalid_biomass.body_mass_kg = 1.0e200;
        let biota = marine_biota(
            vec![invalid_biomass],
            vec![MarineAquaticSpeciesIndex {
                index: 0,
                species_id: 22,
                affinity: TerrainAffinity::OpenOcean,
            }],
            1.0,
        );
        assert_eq!(
            biota.try_marine_aquatic_nodes(0),
            Err(MarineProjectionError::NonFiniteBiomass { species_id: 22 })
        );
    }

    #[test]
    fn marine_projection_scales_the_larger_factor_before_multiplication() {
        let mut large_population =
            species(23, SpeciesCategory::Aquatic, TerrainAffinity::OpenOcean);
        large_population.upstream_density_per_m2 = 1.0e308;
        large_population.body_mass_kg = 10.0;
        let biota = marine_biota(
            vec![large_population],
            vec![MarineAquaticSpeciesIndex {
                index: 0,
                species_id: 23,
                affinity: TerrainAffinity::OpenOcean,
            }],
            1.0,
        );

        let nodes = biota.try_marine_aquatic_nodes(0).unwrap();
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].individuals, 1.0e308);
        assert!(nodes[0].biomass_t.is_finite());
        assert!((nodes[0].biomass_t / 1.0e306 - 1.0).abs() < 1.0e-12);

        // Dividing the tiny mass first would underflow it to zero. Scaling
        // the larger population factor preserves the finite tonne result.
        let mut tiny_mass = species(24, SpeciesCategory::Aquatic, TerrainAffinity::OpenOcean);
        tiny_mass.upstream_density_per_m2 = 1.0e308;
        tiny_mass.body_mass_kg = f64::from_bits(1);
        let biota = marine_biota(
            vec![tiny_mass],
            vec![MarineAquaticSpeciesIndex {
                index: 0,
                species_id: 24,
                affinity: TerrainAffinity::OpenOcean,
            }],
            1.0,
        );
        let nodes = biota.try_marine_aquatic_nodes(0).unwrap();
        assert!(nodes[0].biomass_t.is_finite());
        assert!(nodes[0].biomass_t > 0.0);
        let expected_tonnes = (1.0e308 / 1_000.0) * f64::from_bits(1);
        assert!((nodes[0].biomass_t / expected_tonnes - 1.0).abs() < 1.0e-12);
    }

    #[test]
    fn marine_projection_rejects_cached_indices_that_alias_after_species_reordering() {
        let first = species(41, SpeciesCategory::Aquatic, TerrainAffinity::OpenOcean);
        let second = species(42, SpeciesCategory::Aquatic, TerrainAffinity::OpenOcean);
        let mut biota = marine_biota(
            vec![first, second],
            vec![MarineAquaticSpeciesIndex {
                index: 0,
                species_id: 41,
                affinity: TerrainAffinity::OpenOcean,
            }],
            1.0,
        );

        // The catalogue is publicly mutable; a cached index must not silently
        // become a projection of a different species after a reorder.
        biota.species.swap(0, 1);
        assert_eq!(
            biota.try_marine_aquatic_nodes(0),
            Err(MarineProjectionError::StaleSpeciesIndex {
                index: 0,
                expected_species_id: 41,
                actual_species_id: 42,
            })
        );
    }

    #[test]
    fn marine_projection_rejects_invalid_cell_area_and_stale_species_indices() {
        let valid = species(31, SpeciesCategory::Aquatic, TerrainAffinity::OpenOcean);
        let biota = marine_biota(
            vec![valid.clone()],
            vec![MarineAquaticSpeciesIndex {
                index: 0,
                species_id: 31,
                affinity: TerrainAffinity::OpenOcean,
            }],
            0.0,
        );
        assert_eq!(
            biota.try_marine_aquatic_nodes(0),
            Err(MarineProjectionError::InvalidCellArea)
        );

        let biota = marine_biota(
            vec![valid],
            vec![MarineAquaticSpeciesIndex {
                index: 1,
                species_id: 31,
                affinity: TerrainAffinity::OpenOcean,
            }],
            1.0,
        );
        assert_eq!(
            biota.try_marine_aquatic_nodes(0),
            Err(MarineProjectionError::InvalidSpeciesIndex { index: 1 })
        );
    }

    #[test]
    fn marine_aquatic_index_keeps_only_open_ocean_and_reef_aquatic_species_in_order() {
        let species = vec![
            species(1, SpeciesCategory::Aquatic, TerrainAffinity::OpenOcean),
            species(2, SpeciesCategory::Aquatic, TerrainAffinity::River),
            species(3, SpeciesCategory::Aquatic, TerrainAffinity::ReefSea),
            species(4, SpeciesCategory::Herbivore, TerrainAffinity::OpenOcean),
            species(5, SpeciesCategory::Aquatic, TerrainAffinity::Hill),
        ];

        assert_eq!(
            marine_aquatic_species_indices(&species),
            vec![
                MarineAquaticSpeciesIndex {
                    index: 0,
                    species_id: 1,
                    affinity: TerrainAffinity::OpenOcean,
                },
                MarineAquaticSpeciesIndex {
                    index: 2,
                    species_id: 3,
                    affinity: TerrainAffinity::ReefSea,
                },
            ]
        );
    }

    #[test]
    fn every_affinity_has_a_bit_of_its_own() {
        let mut seen = 0u16;
        for affinity in &AFFINITIES {
            let b = bit(affinity);
            assert_eq!(seen & b, 0, "{affinity:?} shares a bit");
            seen |= b;
        }
    }

    #[test]
    fn plants_stand_on_every_vegetated_cell_and_nowhere_else() {
        // Trees on forest, flowers on the rest: between them, every biome
        // with standing producer carbon has plant species, so the plants'
        // carbon can be the whole field.
        use BiomeType::*;
        for biome in [
            DeepOcean,
            ShallowOcean,
            CoastalWaters,
            ReefSea,
            Desert,
            SemiDesert,
            Savanna,
            Grassland,
            Shrubland,
            Tundra,
            TropicalRainforest,
            TropicalDryForest,
            TemperateForest,
            BorealForest,
            Woodland,
            MontaneForest,
            Alpine,
            Wetland,
            River,
            Volcanic,
            IceSheet,
        ] {
            let plants = habitat_of(&TerrainAffinity::ForestCanopy, biome, NOTHING)
                || habitat_of(&TerrainAffinity::Hill, biome, NOTHING);
            assert_eq!(plants, is_vegetated(biome), "{biome:?}");
        }
    }

    #[test]
    fn nothing_terrestrial_lives_in_the_sea_and_nothing_marine_on_land() {
        use BiomeType::*;
        for biome in [DeepOcean, ShallowOcean, ReefSea, CoastalWaters] {
            for affinity in [
                TerrainAffinity::Hill,
                TerrainAffinity::Mountain,
                TerrainAffinity::Volcanic,
                TerrainAffinity::ForestCanopy,
                TerrainAffinity::Plains,
                TerrainAffinity::Cliff,
                TerrainAffinity::Wetland,
                TerrainAffinity::River,
            ] {
                assert!(
                    !habitat_of(&affinity, biome, EVERYTHING),
                    "{affinity:?} in {biome:?}"
                );
            }
        }
        for biome in [Grassland, TemperateForest, Alpine, Volcanic] {
            assert!(!habitat_of(&TerrainAffinity::OpenOcean, biome, EVERYTHING));
            assert!(!habitat_of(&TerrainAffinity::ReefSea, biome, EVERYTHING));
        }
    }

    #[test]
    fn a_cliff_is_land_on_the_sea_edge() {
        assert!(habitat_of(
            &TerrainAffinity::Cliff,
            BiomeType::Grassland,
            CellSetting {
                on_the_coast: true,
                ..NOTHING
            }
        ));
        assert!(!habitat_of(
            &TerrainAffinity::Cliff,
            BiomeType::Grassland,
            NOTHING
        ));
    }

    #[test]
    fn a_river_through_a_forest_is_river_habitat_and_still_forest() {
        let river = CellSetting {
            has_a_river: true,
            ..NOTHING
        };
        let forest = BiomeType::TemperateForest;
        assert!(habitat_of(&TerrainAffinity::River, forest, river));
        assert!(habitat_of(&TerrainAffinity::ForestCanopy, forest, river));
        assert!(!habitat_of(&TerrainAffinity::River, forest, NOTHING));
        // A river biome is a river whatever the discharge says.
        assert!(habitat_of(
            &TerrainAffinity::River,
            BiomeType::River,
            NOTHING
        ));
        // And the sea is not a river however much flows into it.
        assert!(!habitat_of(
            &TerrainAffinity::River,
            BiomeType::ShallowOcean,
            river
        ));
    }

    #[test]
    fn the_seed_is_laid_in_as_upstream_lays_its_own() {
        let mut seed = [0u8; 32];
        seed[..8].copy_from_slice(&0x0102_0304_0506_0708u64.to_le_bytes());
        seed[8] = 0xff;
        assert_eq!(biosphere_seed(&seed), 0x0102_0304_0506_0708);
    }

    #[test]
    fn the_shallow_sea_is_open_water_and_a_lake_is_not() {
        let sea = CellSetting {
            in_the_sea: true,
            ..NOTHING
        };
        let open = &TerrainAffinity::OpenOcean;
        assert!(habitat_of(open, BiomeType::CoastalWaters, sea));
        assert!(habitat_of(open, BiomeType::ReefSea, sea));
        // The same biome on land is a lake.
        assert!(!habitat_of(open, BiomeType::CoastalWaters, NOTHING));
    }
}

#[cfg(test)]
mod marine_index_tests {
    use super::*;

    #[test]
    fn checked_projection_rejects_internal_stale_metadata() {
        let species = IslandSpecies {
            species_id: 1,
            name: "test marine species".to_owned(),
            category: SpeciesCategory::Aquatic,
            affinity: TerrainAffinity::OpenOcean,
            body_mass_kg: 1.0,
            upstream_population: 1,
            upstream_density_per_m2: 1.0,
            abundance: Abundance::UpstreamDensity,
        };
        let mut biota = IslandBiota {
            species: vec![species.clone()],
            marine_aquatic_species: marine_aquatic_species_indices(&[species]),
            habitat: vec![bit(&TerrainAffinity::OpenOcean)],
            cell_area_m2: 1.0,
            producer_weight: [0.0; 10],
            grazer_weight: [0.0; 10],
        };

        // This is only possible inside the module; public callers cannot
        // bypass replace_species_catalogue and leave the cached index stale.
        biota.species[0].category = SpeciesCategory::Tree;

        assert!(matches!(
            biota.try_marine_aquatic_nodes(0),
            Err(MarineProjectionError::StaleSpeciesMetadata {
                index: 0,
                species_id: 1,
            })
        ));
    }
}
