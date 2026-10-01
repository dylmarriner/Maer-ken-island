use crate::biosphere::genetics::Genome;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SpeciesCategory {
    Aquatic,
    Flying,
    Bird,
    Bee,
    Insect,
    Herbivore,
    Omnivore,
    Carnivore,
    Tree,
    Flower,
    Fungoid,
    Amphibious,
}

/// Real biological sex, used by individually-tracked fauna (this project's
/// stationary plant/fungal categories reproduce asexually/via spore-,
/// seed-, or division-based strategies per their real
/// `reproduction_strategy` catalogue field, not sexual pairing, so they
/// never carry this).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Sex {
    Male,
    Female,
}

/// True for categories that reproduce sexually (the categories eligible for
/// real per-individual `Sex` and pairing-based reproduction). Trees,
/// flowers, and fungoids reproduce via the real strategies their catalogue
/// entries already document (spores, seeds, division, etc.), not sexual
/// pairing, so they're excluded per explicit project direction.
pub fn is_fauna_category(category: &SpeciesCategory) -> bool {
    !matches!(
        category,
        SpeciesCategory::Tree | SpeciesCategory::Flower | SpeciesCategory::Fungoid
    )
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TerrainAffinity {
    OpenOcean,
    ReefSea,
    River,
    Wetland,
    Hill,
    Mountain,
    Volcanic,
    ForestCanopy,
    Plains,
    Cliff,
}

/// Species definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Species {
    pub species_id: u64,
    pub species_name: String,
    pub category: SpeciesCategory,
    pub terrain_affinity: TerrainAffinity,
    pub ecological_niche: String,
    pub representative_genome: Genome,
    pub population_size: u64,
    pub geographic_range: f64,
    pub evolutionary_pressures: Vec<EvolutionaryPressure>,
}

impl Species {
    pub fn new(
        species_id: u64,
        genome: Genome,
        population_size: u64,
        geographic_range: f64,
    ) -> Self {
        Self {
            species_id,
            species_name: format!("Species-{}", species_id),
            category: SpeciesCategory::Aquatic,
            terrain_affinity: TerrainAffinity::OpenOcean,
            ecological_niche: "baseline niche".to_string(),
            representative_genome: genome,
            population_size,
            geographic_range,
            evolutionary_pressures: Vec::new(),
        }
    }

    pub fn is_marine(&self) -> bool {
        self.representative_genome
            .structural
            .environmental_adaptation
            == 0
    }

    pub fn is_terrestrial(&self) -> bool {
        self.representative_genome
            .structural
            .environmental_adaptation
            == 1
    }
}

/// Evolutionary pressure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EvolutionaryPressure {
    Environmental {
        stress_factor: f64,
        temperature_change: f64,
        pressure_change: f64,
    },
    Competition {
        resource_pressure: f64,
    },
    Intelligence {
        selection_for_intelligence: f64,
    },
    Predation {
        predation_pressure: f64,
    },
}
