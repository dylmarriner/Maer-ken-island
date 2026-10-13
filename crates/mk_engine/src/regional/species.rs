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

use std::sync::Arc;

use mk_core::biomes::BiomeType;
use mk_core::canon::CanonLocked;
use mk_core::grid::Grid2;
use mk_island::{DomainLevel, IslandDomain};

use super::ecology::{residence_years, RegionalEcologyState};
use super::hydrology::{discharge_m3_s, RIVER_MIN_DISCHARGE_M3_S};
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
    /// Land a river runs through: discharge at or above the hydrology's
    /// own river threshold (`RIVER_MIN_DISCHARGE_M3_S`). A 2 km cell is a
    /// `River` biome only when the channel fills half of it, which no
    /// river on this island does, but a river running through a forest
    /// cell is still a river to what lives in it.
    pub has_a_river: bool,
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
        TerrainAffinity::OpenOcean => matches!(biome, DeepOcean | ShallowOcean),
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

/// Upstream's species on the island's ground.
#[derive(Debug, Clone)]
pub struct IslandBiota {
    pub species: Vec<IslandSpecies>,
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

/// The 64-bit seed upstream's biosphere takes, from the scenario's 32
/// bytes: the first eight, little-endian, as `BiosphereSystem::new` lays
/// a `u64` into its own seed.
fn biosphere_seed(seed: &[u8; 32]) -> u64 {
    let mut first = [0u8; 8];
    first.copy_from_slice(&seed[..8]);
    u64::from_le_bytes(first)
}

impl IslandBiota {
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
                    has_a_river: !is_sea(r, c)
                        && discharge_m3_s(&physical.hydrology, domain, r, c)
                            >= RIVER_MIN_DISCHARGE_M3_S,
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

        let mut producer_weight = [0.0; 10];
        let mut grazer_weight = [0.0; 10];
        for s in &species {
            let at = slot(&s.affinity);
            match s.abundance {
                Abundance::ProducerCarbon => producer_weight[at] += s.upstream_population as f64,
                Abundance::GrazerCarbon => grazer_weight[at] += s.upstream_population as f64,
                Abundance::UpstreamDensity => {}
            }
        }

        Self {
            species,
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
    };
    const EVERYTHING: CellSetting = CellSetting {
        on_the_coast: true,
        has_a_river: true,
    };

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
}
