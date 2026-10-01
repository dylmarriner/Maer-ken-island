use serde::{Deserialize, Serialize};

pub mod rivers;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum BiomeType {
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
}

impl BiomeType {
    pub fn name(&self) -> &'static str {
        match self {
            BiomeType::DeepOcean => "Deep Ocean",
            BiomeType::ShallowOcean => "Shallow Ocean",
            BiomeType::CoastalWaters => "Coastal Waters",
            BiomeType::ReefSea => "Reef Sea",
            BiomeType::Desert => "Desert",
            BiomeType::SemiDesert => "Semi-Desert",
            BiomeType::Savanna => "Savanna",
            BiomeType::Grassland => "Grassland",
            BiomeType::Shrubland => "Shrubland",
            BiomeType::Tundra => "Tundra",
            BiomeType::TropicalRainforest => "Tropical Rainforest",
            BiomeType::TropicalDryForest => "Tropical Dry Forest",
            BiomeType::TemperateForest => "Temperate Forest",
            BiomeType::BorealForest => "Boreal Forest",
            BiomeType::Woodland => "Woodland",
            BiomeType::MontaneForest => "Montane Forest",
            BiomeType::Alpine => "Alpine",
            BiomeType::Wetland => "Wetland",
            BiomeType::River => "River",
            BiomeType::Volcanic => "Volcanic",
            BiomeType::IceSheet => "Ice Sheet",
        }
    }

    pub fn is_aquatic(&self) -> bool {
        matches!(
            self,
            BiomeType::DeepOcean
                | BiomeType::ShallowOcean
                | BiomeType::CoastalWaters
                | BiomeType::ReefSea
        )
    }

    pub fn is_terrestrial(&self) -> bool {
        !self.is_aquatic()
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct TerrainProperties {
    pub roughness: f64,
    pub fertility: f64,
    pub traversability: f64,
    pub canopy_cover: f64,
}

impl TerrainProperties {
    pub fn new(roughness: f64, fertility: f64, traversability: f64, canopy_cover: f64) -> Self {
        Self {
            roughness: roughness.clamp(0.0, 1.0),
            fertility: fertility.clamp(0.0, 1.0),
            traversability: traversability.clamp(0.0, 1.0),
            canopy_cover: canopy_cover.clamp(0.0, 1.0),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum ResourceKind {
    Wood,
    Stone,
    Food,
    Herbs,
    Minerals,
    Water,
    Fiber,
    Clay,
    Sand,
    MetalOre,
    Gem,
    Salt,
    Meat,
    Hide,
    Bone,
    Fruit,
    Nuts,
    Fungi,
    Fish,
    Shellfish,
    Wax,
    Honey,
    Resin,
    Feather,
    Sulfur,
}

impl ResourceKind {
    pub fn name(&self) -> &'static str {
        match self {
            ResourceKind::Wood => "Wood",
            ResourceKind::Stone => "Stone",
            ResourceKind::Food => "Food",
            ResourceKind::Herbs => "Herbs",
            ResourceKind::Minerals => "Minerals",
            ResourceKind::Water => "Water",
            ResourceKind::Fiber => "Fiber",
            ResourceKind::Clay => "Clay",
            ResourceKind::Sand => "Sand",
            ResourceKind::MetalOre => "Metal Ore",
            ResourceKind::Gem => "Gem",
            ResourceKind::Salt => "Salt",
            ResourceKind::Meat => "Meat",
            ResourceKind::Hide => "Hide",
            ResourceKind::Bone => "Bone",
            ResourceKind::Fruit => "Fruit",
            ResourceKind::Nuts => "Nuts",
            ResourceKind::Fungi => "Fungi",
            ResourceKind::Fish => "Fish",
            ResourceKind::Shellfish => "Shellfish",
            ResourceKind::Wax => "Wax",
            ResourceKind::Honey => "Honey",
            ResourceKind::Resin => "Resin",
            ResourceKind::Feather => "Feather",
            ResourceKind::Sulfur => "Sulfur",
        }
    }
}

impl ResourceKind {
    pub fn category(&self) -> &'static str {
        match self {
            ResourceKind::Wood => "plant",
            ResourceKind::Stone => "mineral",
            ResourceKind::Food => "plant",
            ResourceKind::Herbs => "plant",
            ResourceKind::Minerals => "mineral",
            ResourceKind::Water => "liquid",
            ResourceKind::Fiber => "plant",
            ResourceKind::Clay => "mineral",
            ResourceKind::Sand => "mineral",
            ResourceKind::MetalOre => "mineral",
            ResourceKind::Gem => "mineral",
            ResourceKind::Salt => "mineral",
            ResourceKind::Meat => "animal",
            ResourceKind::Hide => "animal",
            ResourceKind::Bone => "animal",
            ResourceKind::Fruit => "plant",
            ResourceKind::Nuts => "plant",
            ResourceKind::Fungi => "plant",
            ResourceKind::Fish => "animal",
            ResourceKind::Shellfish => "animal",
            ResourceKind::Wax => "animal",
            ResourceKind::Honey => "animal",
            ResourceKind::Resin => "plant",
            ResourceKind::Feather => "animal",
            ResourceKind::Sulfur => "mineral",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ResourceYield {
    pub kind: ResourceKind,
    pub base_yield: f64,
    pub gatherable: bool,
    pub mineable: bool,
    pub harvestable: bool,
}

impl ResourceYield {
    pub const fn gather(kind: ResourceKind, base_yield: f64) -> Self {
        Self {
            kind,
            base_yield,
            gatherable: true,
            mineable: false,
            harvestable: false,
        }
    }
    pub const fn mine(kind: ResourceKind, base_yield: f64) -> Self {
        Self {
            kind,
            base_yield,
            gatherable: false,
            mineable: true,
            harvestable: false,
        }
    }
    pub const fn harvest(kind: ResourceKind, base_yield: f64) -> Self {
        Self {
            kind,
            base_yield,
            gatherable: false,
            mineable: false,
            harvestable: true,
        }
    }
}

pub fn classify_biome(
    elevation_m: f64,
    temperature_k: f64,
    precipitation_mm_day: f64,
    soil_moisture: f64,
    is_volcanic: bool,
) -> BiomeType {
    if elevation_m < 0.0 {
        let depth = -elevation_m;
        if depth > 3000.0 {
            BiomeType::DeepOcean
        } else if depth > 200.0 {
            if temperature_k > 295.0 && depth < 500.0 {
                BiomeType::ReefSea
            } else {
                BiomeType::ShallowOcean
            }
        } else {
            BiomeType::CoastalWaters
        }
    } else {
        if is_volcanic {
            return BiomeType::Volcanic;
        }
        if elevation_m > 3000.0 {
            return BiomeType::Alpine;
        }
        if temperature_k < 260.0 {
            return BiomeType::IceSheet;
        }
        // 260..283 K falls through to the cold tier below, which splits
        // boreal forest, tundra and ice sheet by precipitation.
        if soil_moisture > 0.7 && elevation_m < 200.0 {
            return BiomeType::Wetland;
        }
        if elevation_m > 2000.0 {
            if precipitation_mm_day >= 3.0 {
                return BiomeType::MontaneForest;
            } else {
                return BiomeType::Alpine;
            }
        }
        // Whittaker-style temperature tiers, each split by precipitation
        // from dry to wet. Every tier handles its own dry end, so hot and
        // warm deserts, semi-deserts, savanna, shrubland and cold polar
        // desert (ice sheet) are all reachable.
        if temperature_k >= 298.0 {
            if precipitation_mm_day >= 6.0 {
                BiomeType::TropicalRainforest
            } else if precipitation_mm_day >= 3.0 {
                BiomeType::TropicalDryForest
            } else if precipitation_mm_day >= 2.0 {
                BiomeType::Savanna
            } else if precipitation_mm_day >= 1.0 {
                BiomeType::SemiDesert
            } else {
                BiomeType::Desert
            }
        } else if temperature_k >= 283.0 {
            if precipitation_mm_day >= 6.0 {
                BiomeType::TemperateForest
            } else if precipitation_mm_day >= 4.5 {
                BiomeType::Woodland
            } else if precipitation_mm_day >= 3.0 {
                BiomeType::Grassland
            } else if precipitation_mm_day >= 2.5 {
                BiomeType::Shrubland
            } else if precipitation_mm_day >= 1.0 {
                BiomeType::SemiDesert
            } else {
                BiomeType::Desert
            }
        } else if precipitation_mm_day >= 4.0 {
            BiomeType::BorealForest
        } else if precipitation_mm_day >= 1.0 {
            BiomeType::Tundra
        } else {
            // Cold and dry: polar desert under permanent ice.
            BiomeType::IceSheet
        }
    }
}

pub fn biome_properties(biome: BiomeType) -> TerrainProperties {
    match biome {
        BiomeType::DeepOcean => TerrainProperties::new(0.3, 0.2, 0.0, 0.0),
        BiomeType::ShallowOcean => TerrainProperties::new(0.4, 0.3, 0.0, 0.0),
        BiomeType::CoastalWaters => TerrainProperties::new(0.3, 0.4, 0.0, 0.0),
        BiomeType::ReefSea => TerrainProperties::new(0.6, 0.5, 0.0, 0.0),
        BiomeType::Desert => TerrainProperties::new(0.5, 0.1, 0.7, 0.05),
        BiomeType::SemiDesert => TerrainProperties::new(0.4, 0.2, 0.6, 0.1),
        BiomeType::Savanna => TerrainProperties::new(0.3, 0.4, 0.8, 0.15),
        BiomeType::Grassland => TerrainProperties::new(0.2, 0.6, 0.9, 0.05),
        BiomeType::Shrubland => TerrainProperties::new(0.3, 0.3, 0.7, 0.2),
        BiomeType::Tundra => TerrainProperties::new(0.4, 0.15, 0.5, 0.0),
        BiomeType::TropicalRainforest => TerrainProperties::new(0.6, 0.8, 0.2, 0.95),
        BiomeType::TropicalDryForest => TerrainProperties::new(0.5, 0.5, 0.4, 0.6),
        BiomeType::TemperateForest => TerrainProperties::new(0.4, 0.7, 0.4, 0.8),
        BiomeType::BorealForest => TerrainProperties::new(0.4, 0.3, 0.3, 0.7),
        BiomeType::Woodland => TerrainProperties::new(0.3, 0.5, 0.6, 0.4),
        BiomeType::MontaneForest => TerrainProperties::new(0.7, 0.4, 0.15, 0.5),
        BiomeType::Alpine => TerrainProperties::new(0.8, 0.1, 0.1, 0.0),
        BiomeType::Wetland => TerrainProperties::new(0.3, 0.5, 0.1, 0.3),
        BiomeType::River => TerrainProperties::new(0.2, 0.5, 0.0, 0.1),
        BiomeType::Volcanic => TerrainProperties::new(0.9, 0.05, 0.1, 0.0),
        BiomeType::IceSheet => TerrainProperties::new(0.2, 0.0, 0.1, 0.0),
    }
}

pub fn biome_resources(biome: BiomeType) -> Vec<ResourceYield> {
    match biome {
        BiomeType::DeepOcean => vec![
            ResourceYield::gather(ResourceKind::Fish, 3.0),
            ResourceYield::gather(ResourceKind::Water, 10.0),
            ResourceYield::gather(ResourceKind::Salt, 5.0),
        ],
        BiomeType::ShallowOcean => vec![
            ResourceYield::gather(ResourceKind::Fish, 5.0),
            ResourceYield::gather(ResourceKind::Shellfish, 4.0),
            ResourceYield::gather(ResourceKind::Water, 10.0),
            ResourceYield::gather(ResourceKind::Salt, 5.0),
        ],
        BiomeType::CoastalWaters => vec![
            ResourceYield::gather(ResourceKind::Fish, 4.0),
            ResourceYield::gather(ResourceKind::Shellfish, 6.0),
            ResourceYield::gather(ResourceKind::Water, 10.0),
            ResourceYield::gather(ResourceKind::Salt, 4.0),
        ],
        BiomeType::ReefSea => vec![
            ResourceYield::gather(ResourceKind::Fish, 6.0),
            ResourceYield::gather(ResourceKind::Shellfish, 7.0),
            ResourceYield::gather(ResourceKind::Water, 10.0),
            ResourceYield::gather(ResourceKind::Herbs, 2.0),
        ],
        BiomeType::Desert => vec![
            ResourceYield::gather(ResourceKind::Sand, 8.0),
            ResourceYield::mine(ResourceKind::Minerals, 3.0),
            ResourceYield::mine(ResourceKind::Salt, 4.0),
            ResourceYield::gather(ResourceKind::Stone, 2.0),
        ],
        BiomeType::SemiDesert => vec![
            ResourceYield::gather(ResourceKind::Fiber, 3.0),
            ResourceYield::gather(ResourceKind::Food, 2.0),
            ResourceYield::gather(ResourceKind::Herbs, 2.0),
            ResourceYield::mine(ResourceKind::Minerals, 3.0),
            ResourceYield::gather(ResourceKind::Stone, 3.0),
        ],
        BiomeType::Savanna => vec![
            ResourceYield::gather(ResourceKind::Fiber, 5.0),
            ResourceYield::gather(ResourceKind::Food, 4.0),
            ResourceYield::gather(ResourceKind::Herbs, 4.0),
            ResourceYield::gather(ResourceKind::Wood, 3.0),
            ResourceYield::gather(ResourceKind::Hide, 3.0),
            ResourceYield::gather(ResourceKind::Bone, 2.0),
            ResourceYield::gather(ResourceKind::Meat, 4.0),
        ],
        BiomeType::Grassland => vec![
            ResourceYield::gather(ResourceKind::Fiber, 7.0),
            ResourceYield::gather(ResourceKind::Food, 5.0),
            ResourceYield::gather(ResourceKind::Herbs, 5.0),
            ResourceYield::gather(ResourceKind::Hide, 4.0),
            ResourceYield::gather(ResourceKind::Bone, 2.0),
            ResourceYield::gather(ResourceKind::Meat, 3.0),
            ResourceYield::mine(ResourceKind::Clay, 3.0),
        ],
        BiomeType::Shrubland => vec![
            ResourceYield::gather(ResourceKind::Fiber, 4.0),
            ResourceYield::gather(ResourceKind::Food, 3.0),
            ResourceYield::gather(ResourceKind::Herbs, 4.0),
            ResourceYield::gather(ResourceKind::Wood, 2.0),
            ResourceYield::gather(ResourceKind::Meat, 2.0),
        ],
        BiomeType::Tundra => vec![
            ResourceYield::gather(ResourceKind::Fiber, 2.0),
            ResourceYield::gather(ResourceKind::Food, 1.0),
            ResourceYield::gather(ResourceKind::Hide, 3.0),
            ResourceYield::gather(ResourceKind::Meat, 3.0),
            ResourceYield::mine(ResourceKind::Minerals, 2.0),
        ],
        BiomeType::TropicalRainforest => vec![
            ResourceYield::gather(ResourceKind::Wood, 8.0),
            ResourceYield::gather(ResourceKind::Food, 7.0),
            ResourceYield::gather(ResourceKind::Fruit, 8.0),
            ResourceYield::gather(ResourceKind::Herbs, 7.0),
            ResourceYield::gather(ResourceKind::Fiber, 6.0),
            ResourceYield::gather(ResourceKind::Fungi, 5.0),
            ResourceYield::gather(ResourceKind::Resin, 4.0),
            ResourceYield::gather(ResourceKind::Nuts, 4.0),
            ResourceYield::gather(ResourceKind::Meat, 3.0),
            ResourceYield::gather(ResourceKind::Hide, 3.0),
        ],
        BiomeType::TropicalDryForest => vec![
            ResourceYield::gather(ResourceKind::Wood, 5.0),
            ResourceYield::gather(ResourceKind::Food, 5.0),
            ResourceYield::gather(ResourceKind::Fruit, 5.0),
            ResourceYield::gather(ResourceKind::Herbs, 4.0),
            ResourceYield::gather(ResourceKind::Fiber, 4.0),
            ResourceYield::gather(ResourceKind::Meat, 3.0),
        ],
        BiomeType::TemperateForest => vec![
            ResourceYield::gather(ResourceKind::Wood, 7.0),
            ResourceYield::gather(ResourceKind::Food, 5.0),
            ResourceYield::gather(ResourceKind::Fruit, 4.0),
            ResourceYield::gather(ResourceKind::Nuts, 5.0),
            ResourceYield::gather(ResourceKind::Herbs, 5.0),
            ResourceYield::gather(ResourceKind::Fungi, 5.0),
            ResourceYield::gather(ResourceKind::Fiber, 4.0),
            ResourceYield::gather(ResourceKind::Resin, 3.0),
            ResourceYield::gather(ResourceKind::Meat, 4.0),
            ResourceYield::gather(ResourceKind::Hide, 4.0),
        ],
        BiomeType::BorealForest => vec![
            ResourceYield::gather(ResourceKind::Wood, 6.0),
            ResourceYield::gather(ResourceKind::Food, 3.0),
            ResourceYield::gather(ResourceKind::Fungi, 4.0),
            ResourceYield::gather(ResourceKind::Herbs, 3.0),
            ResourceYield::gather(ResourceKind::Fiber, 3.0),
            ResourceYield::gather(ResourceKind::Resin, 4.0),
            ResourceYield::gather(ResourceKind::Meat, 4.0),
            ResourceYield::gather(ResourceKind::Hide, 5.0),
        ],
        BiomeType::Woodland => vec![
            ResourceYield::gather(ResourceKind::Wood, 4.0),
            ResourceYield::gather(ResourceKind::Food, 4.0),
            ResourceYield::gather(ResourceKind::Fruit, 3.0),
            ResourceYield::gather(ResourceKind::Herbs, 4.0),
            ResourceYield::gather(ResourceKind::Fiber, 4.0),
            ResourceYield::gather(ResourceKind::Meat, 4.0),
            ResourceYield::gather(ResourceKind::Hide, 4.0),
        ],
        BiomeType::MontaneForest => vec![
            ResourceYield::gather(ResourceKind::Wood, 4.0),
            ResourceYield::gather(ResourceKind::Herbs, 5.0),
            ResourceYield::gather(ResourceKind::Fungi, 5.0),
            ResourceYield::gather(ResourceKind::Food, 3.0),
            ResourceYield::mine(ResourceKind::Stone, 5.0),
            ResourceYield::mine(ResourceKind::Minerals, 4.0),
            ResourceYield::mine(ResourceKind::MetalOre, 3.0),
            ResourceYield::mine(ResourceKind::Gem, 2.0),
            ResourceYield::gather(ResourceKind::Meat, 3.0),
        ],
        BiomeType::Alpine => vec![
            ResourceYield::mine(ResourceKind::Stone, 7.0),
            ResourceYield::mine(ResourceKind::Minerals, 5.0),
            ResourceYield::mine(ResourceKind::MetalOre, 4.0),
            ResourceYield::mine(ResourceKind::Gem, 3.0),
            ResourceYield::gather(ResourceKind::Herbs, 2.0),
        ],
        BiomeType::Wetland => vec![
            ResourceYield::gather(ResourceKind::Fiber, 6.0),
            ResourceYield::gather(ResourceKind::Food, 4.0),
            ResourceYield::gather(ResourceKind::Herbs, 5.0),
            ResourceYield::gather(ResourceKind::Fungi, 4.0),
            ResourceYield::gather(ResourceKind::Clay, 6.0),
            ResourceYield::gather(ResourceKind::Water, 8.0),
            ResourceYield::gather(ResourceKind::Fish, 3.0),
        ],
        BiomeType::River => vec![
            ResourceYield::gather(ResourceKind::Water, 10.0),
            ResourceYield::gather(ResourceKind::Fish, 5.0),
            ResourceYield::gather(ResourceKind::Clay, 5.0),
            ResourceYield::gather(ResourceKind::Sand, 4.0),
            ResourceYield::gather(ResourceKind::Stone, 2.0),
        ],
        BiomeType::Volcanic => vec![
            ResourceYield::mine(ResourceKind::Stone, 8.0),
            ResourceYield::mine(ResourceKind::Minerals, 7.0),
            ResourceYield::mine(ResourceKind::MetalOre, 6.0),
            ResourceYield::mine(ResourceKind::Gem, 5.0),
            ResourceYield::mine(ResourceKind::Sulfur, 4.0),
            ResourceYield::gather(ResourceKind::Salt, 3.0),
        ],
        BiomeType::IceSheet => vec![
            ResourceYield::gather(ResourceKind::Water, 5.0),
            ResourceYield::mine(ResourceKind::Stone, 2.0),
            ResourceYield::gather(ResourceKind::Salt, 2.0),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_deep_ocean() {
        let biome = classify_biome(-4000.0, 280.0, 5.0, 0.5, false);
        assert_eq!(biome, BiomeType::DeepOcean);
    }

    #[test]
    fn test_classify_shallow_ocean() {
        let biome = classify_biome(-500.0, 280.0, 5.0, 0.5, false);
        assert_eq!(biome, BiomeType::ShallowOcean);
    }

    #[test]
    fn test_classify_coastal() {
        let biome = classify_biome(-50.0, 280.0, 5.0, 0.5, false);
        assert_eq!(biome, BiomeType::CoastalWaters);
    }

    #[test]
    fn test_classify_reef() {
        let biome = classify_biome(-300.0, 300.0, 5.0, 0.5, false);
        assert_eq!(biome, BiomeType::ReefSea);
    }

    #[test]
    fn test_classify_desert() {
        let biome = classify_biome(500.0, 310.0, 0.5, 0.1, false);
        assert_eq!(biome, BiomeType::Desert);
    }

    #[test]
    fn test_classify_tropical_rainforest() {
        let biome = classify_biome(200.0, 300.0, 10.0, 0.6, false);
        assert_eq!(biome, BiomeType::TropicalRainforest);
    }

    #[test]
    fn test_classify_temperate_forest() {
        let biome = classify_biome(200.0, 290.0, 8.0, 0.5, false);
        assert_eq!(biome, BiomeType::TemperateForest);
    }

    #[test]
    fn test_classify_alpine() {
        let biome = classify_biome(4000.0, 270.0, 5.0, 0.3, false);
        assert_eq!(biome, BiomeType::Alpine);
    }

    #[test]
    fn test_classify_volcanic() {
        let biome = classify_biome(500.0, 300.0, 5.0, 0.3, true);
        assert_eq!(biome, BiomeType::Volcanic);
    }

    #[test]
    fn test_classify_semi_desert() {
        let biome = classify_biome(100.0, 290.0, 2.0, 0.2, false);
        assert_eq!(biome, BiomeType::SemiDesert);
    }

    #[test]
    fn test_classify_savanna() {
        assert_eq!(
            classify_biome(300.0, 302.0, 2.5, 0.3, false),
            BiomeType::Savanna
        );
    }

    #[test]
    fn test_classify_hot_semi_desert() {
        assert_eq!(
            classify_biome(300.0, 302.0, 1.5, 0.2, false),
            BiomeType::SemiDesert
        );
    }

    #[test]
    fn test_classify_shrubland() {
        assert_eq!(
            classify_biome(300.0, 290.0, 2.7, 0.3, false),
            BiomeType::Shrubland
        );
    }

    #[test]
    fn test_classify_cold_dry_is_ice_sheet() {
        assert_eq!(
            classify_biome(300.0, 270.0, 0.5, 0.2, false),
            BiomeType::IceSheet
        );
    }

    #[test]
    fn every_climate_biome_is_reachable() {
        use std::collections::HashSet;
        let mut seen = HashSet::new();
        for elevation in [-4000.0, -400.0, -300.0, -50.0, 100.0, 2500.0, 3500.0] {
            for t10 in 2400..3150 {
                let t = t10 as f64 / 10.0;
                for p10 in 0..120 {
                    let p = p10 as f64 / 10.0;
                    for moisture in [0.2, 0.9] {
                        for volcanic in [false, true] {
                            seen.insert(classify_biome(elevation, t, p, moisture, volcanic));
                        }
                    }
                }
            }
        }
        for biome in [
            BiomeType::DeepOcean,
            BiomeType::ShallowOcean,
            BiomeType::CoastalWaters,
            BiomeType::ReefSea,
            BiomeType::Desert,
            BiomeType::SemiDesert,
            BiomeType::Savanna,
            BiomeType::Grassland,
            BiomeType::Shrubland,
            BiomeType::Tundra,
            BiomeType::TropicalRainforest,
            BiomeType::TropicalDryForest,
            BiomeType::TemperateForest,
            BiomeType::BorealForest,
            BiomeType::Woodland,
            BiomeType::MontaneForest,
            BiomeType::Alpine,
            BiomeType::Wetland,
            BiomeType::Volcanic,
            BiomeType::IceSheet,
        ] {
            assert!(seen.contains(&biome), "{biome:?} is unreachable");
        }
    }

    #[test]
    fn test_classify_ice_sheet() {
        let biome = classify_biome(500.0, 250.0, 0.5, 0.1, false);
        assert_eq!(biome, BiomeType::IceSheet);
    }

    #[test]
    fn test_classify_wetland() {
        let biome = classify_biome(50.0, 290.0, 8.0, 0.85, false);
        assert_eq!(biome, BiomeType::Wetland);
    }

    #[test]
    fn test_all_biomes_have_names() {
        let biomes = vec![
            BiomeType::DeepOcean,
            BiomeType::ShallowOcean,
            BiomeType::CoastalWaters,
            BiomeType::ReefSea,
            BiomeType::Desert,
            BiomeType::SemiDesert,
            BiomeType::Savanna,
            BiomeType::Grassland,
            BiomeType::Shrubland,
            BiomeType::Tundra,
            BiomeType::TropicalRainforest,
            BiomeType::TropicalDryForest,
            BiomeType::TemperateForest,
            BiomeType::BorealForest,
            BiomeType::Woodland,
            BiomeType::MontaneForest,
            BiomeType::Alpine,
            BiomeType::Wetland,
            BiomeType::River,
            BiomeType::Volcanic,
            BiomeType::IceSheet,
        ];
        for b in biomes {
            assert!(!b.name().is_empty());
        }
    }

    #[test]
    fn test_all_biomes_have_properties() {
        let biomes = [
            BiomeType::DeepOcean,
            BiomeType::ShallowOcean,
            BiomeType::CoastalWaters,
            BiomeType::ReefSea,
            BiomeType::Desert,
            BiomeType::SemiDesert,
            BiomeType::Savanna,
            BiomeType::Grassland,
            BiomeType::Shrubland,
            BiomeType::Tundra,
            BiomeType::TropicalRainforest,
            BiomeType::TropicalDryForest,
            BiomeType::TemperateForest,
            BiomeType::BorealForest,
            BiomeType::Woodland,
            BiomeType::MontaneForest,
            BiomeType::Alpine,
            BiomeType::Wetland,
            BiomeType::River,
            BiomeType::Volcanic,
            BiomeType::IceSheet,
        ];
        for b in biomes {
            let props = biome_properties(b);
            assert!(props.roughness >= 0.0 && props.roughness <= 1.0);
            assert!(props.fertility >= 0.0 && props.fertility <= 1.0);
            assert!(props.traversability >= 0.0 && props.traversability <= 1.0);
            assert!(props.canopy_cover >= 0.0 && props.canopy_cover <= 1.0);
        }
    }

    #[test]
    fn test_all_biomes_have_resources() {
        let biomes = [
            BiomeType::DeepOcean,
            BiomeType::ShallowOcean,
            BiomeType::CoastalWaters,
            BiomeType::ReefSea,
            BiomeType::Desert,
            BiomeType::SemiDesert,
            BiomeType::Savanna,
            BiomeType::Grassland,
            BiomeType::Shrubland,
            BiomeType::Tundra,
            BiomeType::TropicalRainforest,
            BiomeType::TropicalDryForest,
            BiomeType::TemperateForest,
            BiomeType::BorealForest,
            BiomeType::Woodland,
            BiomeType::MontaneForest,
            BiomeType::Alpine,
            BiomeType::Wetland,
            BiomeType::River,
            BiomeType::Volcanic,
            BiomeType::IceSheet,
        ];
        for b in biomes {
            let resources = biome_resources(b);
            assert!(!resources.is_empty(), "{} has no resources", b.name());
        }
    }

    #[test]
    fn test_aquatic_terrestrial() {
        assert!(BiomeType::DeepOcean.is_aquatic());
        assert!(!BiomeType::DeepOcean.is_terrestrial());
        assert!(!BiomeType::Desert.is_aquatic());
        assert!(BiomeType::Desert.is_terrestrial());
    }

    #[test]
    fn test_classify_montane_forest() {
        let biome = classify_biome(2500.0, 285.0, 5.0, 0.4, false);
        assert_eq!(biome, BiomeType::MontaneForest);
    }

    #[test]
    fn test_classify_boreal_forest() {
        let biome = classify_biome(200.0, 275.0, 5.0, 0.4, false);
        assert_eq!(biome, BiomeType::BorealForest);
    }

    #[test]
    fn test_classify_grassland() {
        let biome = classify_biome(200.0, 290.0, 3.5, 0.3, false);
        assert_eq!(biome, BiomeType::Grassland);
    }
}
