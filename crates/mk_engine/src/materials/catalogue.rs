//! The island's material catalogue (Phase 3 Task 2).
//!
//! Every resource and material the island can yield, by name and
//! category, with its density and -- for minerals, gems and metals -- its
//! Mohs hardness. The numbers are not here: they are rows of
//! `fixtures/reference/geology/material_properties.json`, each from a
//! handbook the pack cites, and [`MaterialCatalogue`] reads them from the
//! reference library at load. This module only says which materials exist,
//! what kind each is, and whether its density is of the material itself or
//! of it as gathered, pore space and water included. `tests` hold the two
//! in step: every material has its row, and every row a material.

use crate::validation::{ReferenceDomain, ReferenceLibrary};

/// The broad kind of a material, as the economy and its tools treat it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MaterialCategory {
    PreciousMetalOre,
    BaseMetalOre,
    Gem,
    Crystal,
    Stone,
    Sediment,
    IndustrialMineral,
    FossilFuel,
    Salt,
    Plant,
    Animal,
    Marine,
    Water,
    Processed,
}

/// Whether a density is of the material itself or of it as gathered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DensityBasis {
    /// The material without pore space: a mineral, a metal, a log.
    Intrinsic,
    /// Loose or porous material as gathered: sand, clay, charcoal, peat.
    Bulk,
}

/// One material of the catalogue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CatalogueItem {
    GoldOre,
    PlacerGold,
    SilverOre,
    PlatinumGroupConcentrate,
    IronOre,
    MagnetiteOre,
    Ironsand,
    CopperOre,
    TinOre,
    LeadOre,
    ZincOre,
    NickelOre,
    Chromite,
    TungstenOre,
    Molybdenite,
    Cinnabar,
    Uraninite,
    LithiumOre,
    Bauxite,
    Diamond,
    Ruby,
    Sapphire,
    Emerald,
    Topaz,
    Tourmaline,
    Garnet,
    Opal,
    Nephrite,
    QuartzCrystal,
    Amethyst,
    Agate,
    Chalcedony,
    Obsidian,
    Granite,
    Basalt,
    Limestone,
    Marble,
    Sandstone,
    Slate,
    Flint,
    Pumice,
    Stone,
    Clay,
    Kaolin,
    Sand,
    SilicaSand,
    Gravel,
    Peat,
    Sulfur,
    Gypsum,
    PhosphateRock,
    Feldspar,
    Mica,
    Graphite,
    Zeolite,
    Serpentine,
    Kyanite,
    Pyrite,
    Coal,
    CrudeOil,
    NaturalGas,
    RockSalt,
    SeaSalt,
    Wood,
    Resin,
    Fibre,
    Herbs,
    Fruit,
    Nuts,
    Fungi,
    PlantFood,
    Honey,
    Beeswax,
    Seaweed,
    Meat,
    Hide,
    Bone,
    Feather,
    Fish,
    Shellfish,
    Water,
    Planks,
    Masonry,
    Rope,
    Fuel,
    Charcoal,
    Quicklime,
    Glass,
    GoldIngot,
    SilverIngot,
    CopperIngot,
    TinIngot,
    LeadIngot,
    ZincIngot,
    IronIngot,
    Steel,
    Bronze,
    Brass,
    CutGem,
    PolishedStone,
    Tools,
    Weapons,
    Structures,
}

/// Every material, in catalogue order.
pub const ALL_ITEMS: [CatalogueItem; 103] = [
    CatalogueItem::GoldOre,
    CatalogueItem::PlacerGold,
    CatalogueItem::SilverOre,
    CatalogueItem::PlatinumGroupConcentrate,
    CatalogueItem::IronOre,
    CatalogueItem::MagnetiteOre,
    CatalogueItem::Ironsand,
    CatalogueItem::CopperOre,
    CatalogueItem::TinOre,
    CatalogueItem::LeadOre,
    CatalogueItem::ZincOre,
    CatalogueItem::NickelOre,
    CatalogueItem::Chromite,
    CatalogueItem::TungstenOre,
    CatalogueItem::Molybdenite,
    CatalogueItem::Cinnabar,
    CatalogueItem::Uraninite,
    CatalogueItem::LithiumOre,
    CatalogueItem::Bauxite,
    CatalogueItem::Diamond,
    CatalogueItem::Ruby,
    CatalogueItem::Sapphire,
    CatalogueItem::Emerald,
    CatalogueItem::Topaz,
    CatalogueItem::Tourmaline,
    CatalogueItem::Garnet,
    CatalogueItem::Opal,
    CatalogueItem::Nephrite,
    CatalogueItem::QuartzCrystal,
    CatalogueItem::Amethyst,
    CatalogueItem::Agate,
    CatalogueItem::Chalcedony,
    CatalogueItem::Obsidian,
    CatalogueItem::Granite,
    CatalogueItem::Basalt,
    CatalogueItem::Limestone,
    CatalogueItem::Marble,
    CatalogueItem::Sandstone,
    CatalogueItem::Slate,
    CatalogueItem::Flint,
    CatalogueItem::Pumice,
    CatalogueItem::Stone,
    CatalogueItem::Clay,
    CatalogueItem::Kaolin,
    CatalogueItem::Sand,
    CatalogueItem::SilicaSand,
    CatalogueItem::Gravel,
    CatalogueItem::Peat,
    CatalogueItem::Sulfur,
    CatalogueItem::Gypsum,
    CatalogueItem::PhosphateRock,
    CatalogueItem::Feldspar,
    CatalogueItem::Mica,
    CatalogueItem::Graphite,
    CatalogueItem::Zeolite,
    CatalogueItem::Serpentine,
    CatalogueItem::Kyanite,
    CatalogueItem::Pyrite,
    CatalogueItem::Coal,
    CatalogueItem::CrudeOil,
    CatalogueItem::NaturalGas,
    CatalogueItem::RockSalt,
    CatalogueItem::SeaSalt,
    CatalogueItem::Wood,
    CatalogueItem::Resin,
    CatalogueItem::Fibre,
    CatalogueItem::Herbs,
    CatalogueItem::Fruit,
    CatalogueItem::Nuts,
    CatalogueItem::Fungi,
    CatalogueItem::PlantFood,
    CatalogueItem::Honey,
    CatalogueItem::Beeswax,
    CatalogueItem::Seaweed,
    CatalogueItem::Meat,
    CatalogueItem::Hide,
    CatalogueItem::Bone,
    CatalogueItem::Feather,
    CatalogueItem::Fish,
    CatalogueItem::Shellfish,
    CatalogueItem::Water,
    CatalogueItem::Planks,
    CatalogueItem::Masonry,
    CatalogueItem::Rope,
    CatalogueItem::Fuel,
    CatalogueItem::Charcoal,
    CatalogueItem::Quicklime,
    CatalogueItem::Glass,
    CatalogueItem::GoldIngot,
    CatalogueItem::SilverIngot,
    CatalogueItem::CopperIngot,
    CatalogueItem::TinIngot,
    CatalogueItem::LeadIngot,
    CatalogueItem::ZincIngot,
    CatalogueItem::IronIngot,
    CatalogueItem::Steel,
    CatalogueItem::Bronze,
    CatalogueItem::Brass,
    CatalogueItem::CutGem,
    CatalogueItem::PolishedStone,
    CatalogueItem::Tools,
    CatalogueItem::Weapons,
    CatalogueItem::Structures,
];

impl CatalogueItem {
    /// The key of this material's rows in the reference pack.
    pub fn key(self) -> &'static str {
        match self {
            Self::GoldOre => "gold_ore",
            Self::PlacerGold => "placer_gold",
            Self::SilverOre => "silver_ore",
            Self::PlatinumGroupConcentrate => "platinum_group_concentrate",
            Self::IronOre => "iron_ore",
            Self::MagnetiteOre => "magnetite_ore",
            Self::Ironsand => "ironsand",
            Self::CopperOre => "copper_ore",
            Self::TinOre => "tin_ore",
            Self::LeadOre => "lead_ore",
            Self::ZincOre => "zinc_ore",
            Self::NickelOre => "nickel_ore",
            Self::Chromite => "chromite",
            Self::TungstenOre => "tungsten_ore",
            Self::Molybdenite => "molybdenite",
            Self::Cinnabar => "cinnabar",
            Self::Uraninite => "uraninite",
            Self::LithiumOre => "lithium_ore",
            Self::Bauxite => "bauxite",
            Self::Diamond => "diamond",
            Self::Ruby => "ruby",
            Self::Sapphire => "sapphire",
            Self::Emerald => "emerald",
            Self::Topaz => "topaz",
            Self::Tourmaline => "tourmaline",
            Self::Garnet => "garnet",
            Self::Opal => "opal",
            Self::Nephrite => "nephrite",
            Self::QuartzCrystal => "quartz_crystal",
            Self::Amethyst => "amethyst",
            Self::Agate => "agate",
            Self::Chalcedony => "chalcedony",
            Self::Obsidian => "obsidian",
            Self::Granite => "granite",
            Self::Basalt => "basalt",
            Self::Limestone => "limestone",
            Self::Marble => "marble",
            Self::Sandstone => "sandstone",
            Self::Slate => "slate",
            Self::Flint => "flint",
            Self::Pumice => "pumice",
            Self::Stone => "stone",
            Self::Clay => "clay",
            Self::Kaolin => "kaolin",
            Self::Sand => "sand",
            Self::SilicaSand => "silica_sand",
            Self::Gravel => "gravel",
            Self::Peat => "peat",
            Self::Sulfur => "sulfur",
            Self::Gypsum => "gypsum",
            Self::PhosphateRock => "phosphate_rock",
            Self::Feldspar => "feldspar",
            Self::Mica => "mica",
            Self::Graphite => "graphite",
            Self::Zeolite => "zeolite",
            Self::Serpentine => "serpentine",
            Self::Kyanite => "kyanite",
            Self::Pyrite => "pyrite",
            Self::Coal => "coal",
            Self::CrudeOil => "crude_oil",
            Self::NaturalGas => "natural_gas",
            Self::RockSalt => "rock_salt",
            Self::SeaSalt => "sea_salt",
            Self::Wood => "wood",
            Self::Resin => "resin",
            Self::Fibre => "fibre",
            Self::Herbs => "herbs",
            Self::Fruit => "fruit",
            Self::Nuts => "nuts",
            Self::Fungi => "fungi",
            Self::PlantFood => "plant_food",
            Self::Honey => "honey",
            Self::Beeswax => "beeswax",
            Self::Seaweed => "seaweed",
            Self::Meat => "meat",
            Self::Hide => "hide",
            Self::Bone => "bone",
            Self::Feather => "feather",
            Self::Fish => "fish",
            Self::Shellfish => "shellfish",
            Self::Water => "water",
            Self::Planks => "planks",
            Self::Masonry => "masonry",
            Self::Rope => "rope",
            Self::Fuel => "fuel",
            Self::Charcoal => "charcoal",
            Self::Quicklime => "quicklime",
            Self::Glass => "glass",
            Self::GoldIngot => "gold_ingot",
            Self::SilverIngot => "silver_ingot",
            Self::CopperIngot => "copper_ingot",
            Self::TinIngot => "tin_ingot",
            Self::LeadIngot => "lead_ingot",
            Self::ZincIngot => "zinc_ingot",
            Self::IronIngot => "iron_ingot",
            Self::Steel => "steel",
            Self::Bronze => "bronze",
            Self::Brass => "brass",
            Self::CutGem => "cut_gem",
            Self::PolishedStone => "polished_stone",
            Self::Tools => "tools",
            Self::Weapons => "weapons",
            Self::Structures => "structures",
        }
    }

    /// The material's name, as a person would say it.
    pub fn name(self) -> &'static str {
        match self {
            Self::GoldOre => "gold ore (quartz-gold)",
            Self::PlacerGold => "placer gold (nuggets and flakes)",
            Self::SilverOre => "silver ore (acanthite)",
            Self::PlatinumGroupConcentrate => "platinum-group concentrate",
            Self::IronOre => "iron ore (hematite)",
            Self::MagnetiteOre => "iron ore (magnetite)",
            Self::Ironsand => "ironsand (titanomagnetite)",
            Self::CopperOre => "copper ore (chalcopyrite)",
            Self::TinOre => "tin ore (cassiterite)",
            Self::LeadOre => "lead ore (galena)",
            Self::ZincOre => "zinc ore (sphalerite)",
            Self::NickelOre => "nickel ore (pentlandite)",
            Self::Chromite => "chromite",
            Self::TungstenOre => "tungsten ore (wolframite)",
            Self::Molybdenite => "molybdenite",
            Self::Cinnabar => "cinnabar (mercury)",
            Self::Uraninite => "uraninite",
            Self::LithiumOre => "lithium ore (spodumene)",
            Self::Bauxite => "bauxite (gibbsite)",
            Self::Diamond => "diamond",
            Self::Ruby => "ruby (corundum)",
            Self::Sapphire => "sapphire (corundum)",
            Self::Emerald => "emerald (beryl)",
            Self::Topaz => "topaz",
            Self::Tourmaline => "tourmaline",
            Self::Garnet => "garnet (almandine)",
            Self::Opal => "opal",
            Self::Nephrite => "nephrite jade (pounamu)",
            Self::QuartzCrystal => "quartz crystal",
            Self::Amethyst => "amethyst",
            Self::Agate => "agate",
            Self::Chalcedony => "chalcedony",
            Self::Obsidian => "obsidian",
            Self::Granite => "granite",
            Self::Basalt => "basalt",
            Self::Limestone => "limestone",
            Self::Marble => "marble",
            Self::Sandstone => "sandstone",
            Self::Slate => "slate",
            Self::Flint => "flint and chert",
            Self::Pumice => "pumice",
            Self::Stone => "stone (rubble)",
            Self::Clay => "clay",
            Self::Kaolin => "kaolin",
            Self::Sand => "sand",
            Self::SilicaSand => "silica sand",
            Self::Gravel => "gravel",
            Self::Peat => "peat (as cut, wet)",
            Self::Sulfur => "sulfur",
            Self::Gypsum => "gypsum",
            Self::PhosphateRock => "phosphate rock (apatite)",
            Self::Feldspar => "feldspar (orthoclase)",
            Self::Mica => "mica (muscovite)",
            Self::Graphite => "graphite",
            Self::Zeolite => "zeolite (clinoptilolite)",
            Self::Serpentine => "serpentine (antigorite)",
            Self::Kyanite => "kyanite",
            Self::Pyrite => "pyrite",
            Self::Coal => "coal (bituminous)",
            Self::CrudeOil => "crude oil",
            Self::NaturalGas => "natural gas (methane, 0 C and 1 atm)",
            Self::RockSalt => "rock salt (halite)",
            Self::SeaSalt => "sea salt",
            Self::Wood => "wood (air-dry softwood)",
            Self::Resin => "resin",
            Self::Fibre => "fibre",
            Self::Herbs => "herbs",
            Self::Fruit => "fruit",
            Self::Nuts => "nuts",
            Self::Fungi => "fungi",
            Self::PlantFood => "plant food",
            Self::Honey => "honey",
            Self::Beeswax => "beeswax",
            Self::Seaweed => "seaweed",
            Self::Meat => "meat",
            Self::Hide => "hide",
            Self::Bone => "bone",
            Self::Feather => "feather",
            Self::Fish => "fish",
            Self::Shellfish => "shellfish",
            Self::Water => "water",
            Self::Planks => "planks",
            Self::Masonry => "masonry (fired brick)",
            Self::Rope => "rope",
            Self::Fuel => "fuel (diesel)",
            Self::Charcoal => "charcoal (lump)",
            Self::Quicklime => "quicklime",
            Self::Glass => "glass (soda-lime)",
            Self::GoldIngot => "gold ingot",
            Self::SilverIngot => "silver ingot",
            Self::CopperIngot => "copper ingot",
            Self::TinIngot => "tin ingot",
            Self::LeadIngot => "lead ingot",
            Self::ZincIngot => "zinc ingot",
            Self::IronIngot => "iron ingot",
            Self::Steel => "steel (carbon)",
            Self::Bronze => "bronze (88 Cu : 12 Sn)",
            Self::Brass => "brass",
            Self::CutGem => "cut gem",
            Self::PolishedStone => "polished stone",
            Self::Tools => "tools",
            Self::Weapons => "weapons",
            Self::Structures => "structures",
        }
    }

    pub fn category(self) -> MaterialCategory {
        match self {
            Self::GoldOre => MaterialCategory::PreciousMetalOre,
            Self::PlacerGold => MaterialCategory::PreciousMetalOre,
            Self::SilverOre => MaterialCategory::PreciousMetalOre,
            Self::PlatinumGroupConcentrate => MaterialCategory::PreciousMetalOre,
            Self::IronOre => MaterialCategory::BaseMetalOre,
            Self::MagnetiteOre => MaterialCategory::BaseMetalOre,
            Self::Ironsand => MaterialCategory::BaseMetalOre,
            Self::CopperOre => MaterialCategory::BaseMetalOre,
            Self::TinOre => MaterialCategory::BaseMetalOre,
            Self::LeadOre => MaterialCategory::BaseMetalOre,
            Self::ZincOre => MaterialCategory::BaseMetalOre,
            Self::NickelOre => MaterialCategory::BaseMetalOre,
            Self::Chromite => MaterialCategory::BaseMetalOre,
            Self::TungstenOre => MaterialCategory::BaseMetalOre,
            Self::Molybdenite => MaterialCategory::BaseMetalOre,
            Self::Cinnabar => MaterialCategory::BaseMetalOre,
            Self::Uraninite => MaterialCategory::BaseMetalOre,
            Self::LithiumOre => MaterialCategory::BaseMetalOre,
            Self::Bauxite => MaterialCategory::BaseMetalOre,
            Self::Diamond => MaterialCategory::Gem,
            Self::Ruby => MaterialCategory::Gem,
            Self::Sapphire => MaterialCategory::Gem,
            Self::Emerald => MaterialCategory::Gem,
            Self::Topaz => MaterialCategory::Gem,
            Self::Tourmaline => MaterialCategory::Gem,
            Self::Garnet => MaterialCategory::Gem,
            Self::Opal => MaterialCategory::Gem,
            Self::Nephrite => MaterialCategory::Gem,
            Self::QuartzCrystal => MaterialCategory::Crystal,
            Self::Amethyst => MaterialCategory::Crystal,
            Self::Agate => MaterialCategory::Crystal,
            Self::Chalcedony => MaterialCategory::Crystal,
            Self::Obsidian => MaterialCategory::Crystal,
            Self::Granite => MaterialCategory::Stone,
            Self::Basalt => MaterialCategory::Stone,
            Self::Limestone => MaterialCategory::Stone,
            Self::Marble => MaterialCategory::Stone,
            Self::Sandstone => MaterialCategory::Stone,
            Self::Slate => MaterialCategory::Stone,
            Self::Flint => MaterialCategory::Stone,
            Self::Pumice => MaterialCategory::Stone,
            Self::Stone => MaterialCategory::Stone,
            Self::Clay => MaterialCategory::Sediment,
            Self::Kaolin => MaterialCategory::Sediment,
            Self::Sand => MaterialCategory::Sediment,
            Self::SilicaSand => MaterialCategory::Sediment,
            Self::Gravel => MaterialCategory::Sediment,
            Self::Peat => MaterialCategory::Sediment,
            Self::Sulfur => MaterialCategory::IndustrialMineral,
            Self::Gypsum => MaterialCategory::IndustrialMineral,
            Self::PhosphateRock => MaterialCategory::IndustrialMineral,
            Self::Feldspar => MaterialCategory::IndustrialMineral,
            Self::Mica => MaterialCategory::IndustrialMineral,
            Self::Graphite => MaterialCategory::IndustrialMineral,
            Self::Zeolite => MaterialCategory::IndustrialMineral,
            Self::Serpentine => MaterialCategory::IndustrialMineral,
            Self::Kyanite => MaterialCategory::IndustrialMineral,
            Self::Pyrite => MaterialCategory::IndustrialMineral,
            Self::Coal => MaterialCategory::FossilFuel,
            Self::CrudeOil => MaterialCategory::FossilFuel,
            Self::NaturalGas => MaterialCategory::FossilFuel,
            Self::RockSalt => MaterialCategory::Salt,
            Self::SeaSalt => MaterialCategory::Salt,
            Self::Wood => MaterialCategory::Plant,
            Self::Resin => MaterialCategory::Plant,
            Self::Fibre => MaterialCategory::Plant,
            Self::Herbs => MaterialCategory::Plant,
            Self::Fruit => MaterialCategory::Plant,
            Self::Nuts => MaterialCategory::Plant,
            Self::Fungi => MaterialCategory::Plant,
            Self::PlantFood => MaterialCategory::Plant,
            Self::Honey => MaterialCategory::Plant,
            Self::Beeswax => MaterialCategory::Plant,
            Self::Seaweed => MaterialCategory::Plant,
            Self::Meat => MaterialCategory::Animal,
            Self::Hide => MaterialCategory::Animal,
            Self::Bone => MaterialCategory::Animal,
            Self::Feather => MaterialCategory::Animal,
            Self::Fish => MaterialCategory::Marine,
            Self::Shellfish => MaterialCategory::Marine,
            Self::Water => MaterialCategory::Water,
            Self::Planks => MaterialCategory::Processed,
            Self::Masonry => MaterialCategory::Processed,
            Self::Rope => MaterialCategory::Processed,
            Self::Fuel => MaterialCategory::Processed,
            Self::Charcoal => MaterialCategory::Processed,
            Self::Quicklime => MaterialCategory::Processed,
            Self::Glass => MaterialCategory::Processed,
            Self::GoldIngot => MaterialCategory::Processed,
            Self::SilverIngot => MaterialCategory::Processed,
            Self::CopperIngot => MaterialCategory::Processed,
            Self::TinIngot => MaterialCategory::Processed,
            Self::LeadIngot => MaterialCategory::Processed,
            Self::ZincIngot => MaterialCategory::Processed,
            Self::IronIngot => MaterialCategory::Processed,
            Self::Steel => MaterialCategory::Processed,
            Self::Bronze => MaterialCategory::Processed,
            Self::Brass => MaterialCategory::Processed,
            Self::CutGem => MaterialCategory::Processed,
            Self::PolishedStone => MaterialCategory::Processed,
            Self::Tools => MaterialCategory::Processed,
            Self::Weapons => MaterialCategory::Processed,
            Self::Structures => MaterialCategory::Processed,
        }
    }

    pub fn density_basis(self) -> DensityBasis {
        match self {
            Self::Pumice
            | Self::Clay
            | Self::Kaolin
            | Self::Sand
            | Self::SilicaSand
            | Self::Gravel
            | Self::Peat
            | Self::Herbs
            | Self::Rope
            | Self::Charcoal => DensityBasis::Bulk,
            _ => DensityBasis::Intrinsic,
        }
    }

    /// The material with this pack key.
    pub fn from_key(key: &str) -> Option<Self> {
        ALL_ITEMS.iter().copied().find(|item| item.key() == key)
    }
}

/// Why the catalogue could not be read.
#[derive(Debug, Clone, PartialEq)]
pub enum CatalogueError {
    MissingReference(String),
}

impl std::fmt::Display for CatalogueError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingReference(what) => {
                write!(f, "the material catalogue's reference is missing: {what}")
            }
        }
    }
}

impl std::error::Error for CatalogueError {}

const PACK_DENSITY_INTRINSIC: &str = "material_density_intrinsic";
const PACK_DENSITY_BULK: &str = "material_density_bulk";
const PACK_HARDNESS: &str = "mohs_hardness";

/// The catalogue's numbers, read from the reference packs.
#[derive(Debug, Clone)]
pub struct MaterialCatalogue {
    density_kg_m3: Vec<Option<f64>>,
    mohs: Vec<Option<f64>>,
}

impl MaterialCatalogue {
    /// Read every material's density and hardness from `library`.
    pub fn from_reference(library: &ReferenceLibrary) -> Result<Self, CatalogueError> {
        let table = |key: &str| {
            library
                .item(ReferenceDomain::Geology, key)
                .and_then(|item| item.table())
                .cloned()
                .ok_or_else(|| CatalogueError::MissingReference(key.to_string()))
        };
        let intrinsic = table(PACK_DENSITY_INTRINSIC)?;
        let bulk = table(PACK_DENSITY_BULK)?;
        let hardness = table(PACK_HARDNESS)?;
        let density_kg_m3 = ALL_ITEMS
            .iter()
            .map(|item| match item.density_basis() {
                DensityBasis::Intrinsic => intrinsic.get(item.key(), "density_kg_m3"),
                DensityBasis::Bulk => bulk.get(item.key(), "density_kg_m3"),
            })
            .collect();
        let mohs = ALL_ITEMS
            .iter()
            .map(|item| hardness.get(item.key(), "mohs"))
            .collect();
        Ok(Self {
            density_kg_m3,
            mohs,
        })
    }

    /// Load from the repository's reference packs.
    pub fn load_default() -> Result<Self, CatalogueError> {
        let library = ReferenceLibrary::load(&crate::validation::default_reference_dir())
            .map_err(|e| CatalogueError::MissingReference(e.to_string()))?;
        Self::from_reference(&library)
    }

    fn index(item: CatalogueItem) -> usize {
        ALL_ITEMS
            .iter()
            .position(|i| *i == item)
            .expect("every catalogue item is in ALL_ITEMS")
    }

    /// Density (kg/m3) on the item's [`DensityBasis`]. `None` for what has
    /// no single density: things made of several materials (tools,
    /// weapons, structures, rope), and those whose density is another
    /// material's (a cut gem is its gem, polished stone its stone), and
    /// fresh herbs, which are mostly air as gathered.
    pub fn density_kg_m3(&self, item: CatalogueItem) -> Option<f64> {
        self.density_kg_m3[Self::index(item)]
    }

    /// Mohs hardness, where the scale applies.
    pub fn mohs(&self, item: CatalogueItem) -> Option<f64> {
        self.mohs[Self::index(item)]
    }

    /// Mass (kg) of `volume_m3` of the item, where it has a density.
    pub fn mass_kg(&self, item: CatalogueItem, volume_m3: f64) -> Option<f64> {
        self.density_kg_m3(item).map(|rho| rho * volume_m3.max(0.0))
    }
}
