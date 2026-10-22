//! Farms on the estate (Phase 4b Task 7b, food chain).
//!
//! A dairy farm milks New Zealand's average 2.76 cows to the effective
//! hectare, each giving 400 kg of milksolids a season (LIC and DairyNZ
//! 2024). Arable and market-garden land yields New Zealand's 2023 average
//! per hectare (FAOSTAT; the vegetables are FAO estimates). These are
//! modern national averages; nothing yet scales them to the founders' land
//! or technology.

use serde::{Deserialize, Serialize};

use super::petroleum::MissingReference;
use crate::validation::{ReferenceDomain, ReferenceLibrary};

const CROP_YEAR: &str = "y2023";
/// FAOSTAT's milk-per-animal row, the same year.
const MILK_YEAR: &str = "2023";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Crop {
    Wheat,
    Barley,
    MaizeGrain,
    Oats,
    Potatoes,
    Onions,
    Pumpkins,
    Cabbages,
    Carrots,
}

pub const CROPS: [Crop; 9] = [
    Crop::Wheat,
    Crop::Barley,
    Crop::MaizeGrain,
    Crop::Oats,
    Crop::Potatoes,
    Crop::Onions,
    Crop::Pumpkins,
    Crop::Cabbages,
    Crop::Carrots,
];

impl Crop {
    /// The pack table and row holding the crop's yield.
    fn pack_row(self) -> (&'static str, &'static str) {
        const GRAIN: &str = "crop_yield_official_fao";
        const VEG: &str = "vegetable_yield_fao_estimated";
        match self {
            Self::Wheat => (GRAIN, "wheat"),
            Self::Barley => (GRAIN, "barley"),
            Self::MaizeGrain => (GRAIN, "maize_grain"),
            Self::Oats => (GRAIN, "oats"),
            Self::Potatoes => (VEG, "potatoes"),
            Self::Onions => (VEG, "onions_and_shallots_green"),
            Self::Pumpkins => (VEG, "pumpkins_squash_and_gourds"),
            Self::Cabbages => (VEG, "cabbages"),
            Self::Carrots => (VEG, "carrots_and_turnips"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FarmModel {
    pub dairy_cows_per_ha: f64,
    pub milksolids_kg_per_cow: f64,
    /// Whole milk a cow gives a year (FAOSTAT's New Zealand yield).
    pub milk_kg_per_cow: f64,
    /// kg a hectare yields a year, in [`CROPS`] order.
    pub crop_kg_per_ha: [f64; 9],
}

impl FarmModel {
    pub fn from_reference(library: &ReferenceLibrary) -> Result<Self, MissingReference> {
        let item = |key: &str| {
            library
                .item(ReferenceDomain::Industry, key)
                .ok_or_else(|| MissingReference(key.into()))
        };
        let central = |key: &str| {
            item(key)?
                .central()
                .ok_or_else(|| MissingReference(key.into()))
        };
        let mut crop_kg_per_ha = [0.0; 9];
        for (slot, crop) in crop_kg_per_ha.iter_mut().zip(CROPS) {
            let (key, row) = crop.pack_row();
            *slot = item(key)?
                .table()
                .and_then(|t| t.get(row, CROP_YEAR))
                .ok_or_else(|| MissingReference(format!("{key}/{row}")))?;
        }
        Ok(Self {
            dairy_cows_per_ha: central("dairy_stocking_rate_national")?,
            milksolids_kg_per_cow: central("dairy_milksolids_per_cow")?,
            milk_kg_per_cow: item("dairy_milk_volume_per_cow_fao")?
                .table()
                .and_then(|t| t.get(MILK_YEAR, "kg_milk_per_animal"))
                .ok_or_else(|| MissingReference("dairy_milk_volume_per_cow_fao".into()))?,
            crop_kg_per_ha,
        })
    }

    pub fn load_default() -> Result<Self, MissingReference> {
        let library = ReferenceLibrary::load(&crate::validation::default_reference_dir())
            .map_err(|e| MissingReference(e.to_string()))?;
        Self::from_reference(&library)
    }

    pub fn crop_kg_per_ha(&self, crop: Crop) -> f64 {
        self.crop_kg_per_ha[CROPS.iter().position(|c| *c == crop).expect("every crop")]
    }
}

/// A farm's land by use.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Farm {
    pub dairy_ha: f64,
    pub crops_ha: Vec<(Crop, f64)>,
}

/// What a farm produces in a year.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Harvest {
    pub cows: f64,
    pub milksolids_kg: f64,
    pub milk_kg: f64,
    pub crops_kg: Vec<(Crop, f64)>,
}

impl Farm {
    pub fn area_ha(&self) -> f64 {
        self.dairy_ha + self.crops_ha.iter().map(|(_, ha)| ha).sum::<f64>()
    }

    /// One year's milk and crops; `fraction` of a year for a shorter step.
    pub fn produce(&self, model: &FarmModel, fraction: f64) -> Harvest {
        let cows = self.dairy_ha * model.dairy_cows_per_ha;
        Harvest {
            cows,
            milksolids_kg: cows * model.milksolids_kg_per_cow * fraction,
            milk_kg: cows * model.milk_kg_per_cow * fraction,
            crops_kg: self
                .crops_ha
                .iter()
                .map(|&(crop, ha)| (crop, ha * model.crop_kg_per_ha(crop) * fraction))
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_farm_rates_are_the_packs() {
        let m = FarmModel::load_default().expect("the pack loads");
        assert_eq!(m.dairy_cows_per_ha, 2.76);
        assert_eq!(m.milksolids_kg_per_cow, 400.0);
        assert_eq!(m.milk_kg_per_cow, 4_593.0);
        assert_eq!(m.crop_kg_per_ha(Crop::Wheat), 9668.1);
        assert_eq!(m.crop_kg_per_ha(Crop::Potatoes), 51080.7);
        assert!(m.crop_kg_per_ha.iter().all(|y| *y > 0.0));
    }

    #[test]
    fn a_dairy_hectare_gives_about_the_national_milksolids_per_hectare() {
        // LIC and DairyNZ report 1,105 kg MS/ha; 2.76 cows × 400 kg = 1,104.
        let m = FarmModel::load_default().expect("the pack loads");
        let farm = Farm {
            dairy_ha: 100.0,
            crops_ha: vec![(Crop::Wheat, 10.0)],
        };
        let year = farm.produce(&m, 1.0);
        assert!((year.cows - 276.0).abs() < 1e-9);
        assert!((year.milksolids_kg / 100.0 - 1_105.0).abs() < 5.0);
        let half = farm.produce(&m, 0.5);
        assert!((half.crops_kg[0].1 - 0.5 * 96_681.0).abs() < 1e-6);
        assert_eq!(farm.area_ha(), 110.0);
    }
}
