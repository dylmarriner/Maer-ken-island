use crate::agents::GridPosition;
use mk_core::biomes::BiomeType;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlantBeing {
    pub id: u64,
    pub kind: PlantKind,
    pub biome: BiomeType,
    pub position: GridPosition,
    pub maturity: f32,
    pub alive: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum PlantKind {
    Tree,
    Shrub,
    Grass,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VegetationSystem {
    pub plants: Vec<PlantBeing>,
    pub next_id: u64,
}

impl VegetationSystem {
    pub fn new() -> Self {
        Self {
            plants: Vec::new(),
            next_id: 1,
        }
    }

    /// Materialize deterministic plant instances from the authoritative biome
    /// grid. This is a bounded view population, not a replacement for biomass
    /// and nutrient aggregate accounting.
    pub fn seed_from_biomes(&mut self, biomes: &mk_core::grid::Grid2<BiomeType>) {
        if !self.plants.is_empty() {
            return;
        }
        for (row, col, biome) in biomes.indexed_iter() {
            let kind = plant_kind(biome);
            if kind.is_none() || (row * 31 + col * 17) % 11 != 0 {
                continue;
            }
            self.plants.push(PlantBeing {
                id: self.next_id,
                kind: kind.unwrap(),
                biome: *biome,
                position: GridPosition::new(row as i32, col as i32),
                maturity: 0.5,
                alive: true,
            });
            self.next_id += 1;
        }
        if self.plants.is_empty() {
            if let Some((row, col, biome)) = biomes
                .indexed_iter()
                .find(|(_, _, biome)| biome.is_terrestrial())
            {
                self.plants.push(PlantBeing {
                    id: self.next_id,
                    kind: PlantKind::Grass,
                    biome: *biome,
                    position: GridPosition::new(row as i32, col as i32),
                    maturity: 0.5,
                    alive: true,
                });
                self.next_id += 1;
            }
        }
    }

    pub fn step(&mut self, dt_years: f32) {
        for plant in &mut self.plants {
            if plant.alive {
                plant.maturity = (plant.maturity + dt_years.max(0.0) * 0.01).min(1.0);
            }
        }
    }
}

fn plant_kind(biome: &BiomeType) -> Option<PlantKind> {
    match biome {
        BiomeType::TropicalRainforest
        | BiomeType::TropicalDryForest
        | BiomeType::TemperateForest
        | BiomeType::BorealForest
        | BiomeType::Woodland
        | BiomeType::MontaneForest => Some(PlantKind::Tree),
        BiomeType::Shrubland | BiomeType::Savanna | BiomeType::Wetland => Some(PlantKind::Shrub),
        BiomeType::Grassland | BiomeType::Tundra => Some(PlantKind::Grass),
        _ => None,
    }
}
