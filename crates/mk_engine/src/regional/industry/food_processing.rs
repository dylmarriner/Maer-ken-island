//! First-stage food processing (Phase 4b Task 7b, food chain).
//!
//! The farms' grain and milk go through a flour mill, a maltings, an oat
//! mill and a dairy factory at New Zealand's extraction rates, from FAO's
//! Technical Conversion Factors: wheat gives 78% flour and 22% bran,
//! barley 80% malt, oats 57% rolled oats and 25% bran (the rest hulls and
//! loss). Each dairy product is its own use of the milk: a kilogram of
//! milk makes 61 g of butter, or 90 g of cheese, or 140 g of whole milk
//! powder. Bread, the supermarket and the energy each plant burns are not
//! modelled yet.

use serde::{Deserialize, Serialize};

use super::farm::Crop;
use super::petroleum::MissingReference;
use crate::validation::{ReferenceDomain, ReferenceLibrary};

/// What a mill or the dairy factory makes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Product {
    WheatFlour,
    WheatBran,
    BarleyMalt,
    RolledOats,
    OatBran,
    Cream,
    Butter,
    CondensedMilk,
    EvaporatedMilk,
    WholeMilkPowder,
    Cheese,
}

impl Product {
    /// The pack's row label and the table it is in.
    fn row(self) -> (&'static str, &'static str) {
        const CEREAL: &str = "nz_cereal_extraction_rates";
        const MILK: &str = "nz_milk_extraction_rates";
        match self {
            Self::WheatFlour => (CEREAL, "wheat_flour"),
            Self::WheatBran => (CEREAL, "wheat_bran"),
            Self::BarleyMalt => (CEREAL, "barley_malt"),
            Self::RolledOats => (CEREAL, "oats_rolled"),
            Self::OatBran => (CEREAL, "oats_bran"),
            Self::Cream => (MILK, "cream"),
            Self::Butter => (MILK, "butter"),
            Self::CondensedMilk => (MILK, "condensed_whole_milk"),
            Self::EvaporatedMilk => (MILK, "evaporated_whole_milk"),
            Self::WholeMilkPowder => (MILK, "whole_milk_powder"),
            Self::Cheese => (MILK, "cheese"),
        }
    }
}

pub const PRODUCTS: [Product; 11] = [
    Product::WheatFlour,
    Product::WheatBran,
    Product::BarleyMalt,
    Product::RolledOats,
    Product::OatBran,
    Product::Cream,
    Product::Butter,
    Product::CondensedMilk,
    Product::EvaporatedMilk,
    Product::WholeMilkPowder,
    Product::Cheese,
];

pub const DAIRY: [Product; 6] = [
    Product::Cream,
    Product::Butter,
    Product::CondensedMilk,
    Product::EvaporatedMilk,
    Product::WholeMilkPowder,
    Product::Cheese,
];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProcessingModel {
    /// Kilograms of product per kilogram of parent, in [`PRODUCTS`] order.
    pub extraction: [f64; 11],
}

impl ProcessingModel {
    pub fn from_reference(library: &ReferenceLibrary) -> Result<Self, MissingReference> {
        let mut extraction = [0.0; 11];
        for (slot, product) in extraction.iter_mut().zip(PRODUCTS) {
            let (key, row) = product.row();
            *slot = library
                .item(ReferenceDomain::Industry, key)
                .and_then(|i| i.table())
                .and_then(|t| t.get(row, "extraction_percent"))
                .ok_or_else(|| MissingReference(format!("{key}/{row}")))?
                / 100.0;
        }
        Ok(Self { extraction })
    }

    pub fn load_default() -> Result<Self, MissingReference> {
        let library = ReferenceLibrary::load(&crate::validation::default_reference_dir())
            .map_err(|e| MissingReference(e.to_string()))?;
        Self::from_reference(&library)
    }

    pub fn extraction(&self, product: Product) -> f64 {
        let i = PRODUCTS.iter().position(|p| *p == product).unwrap_or(0);
        self.extraction[i]
    }

    /// Mill or malt `kg` of a cereal: what comes out. Crops with no
    /// first-stage product here (vegetables, maize) give nothing.
    pub fn mill(&self, crop: Crop, kg: f64) -> Vec<(Product, f64)> {
        let products: &[Product] = match crop {
            Crop::Wheat => &[Product::WheatFlour, Product::WheatBran],
            Crop::Barley => &[Product::BarleyMalt],
            Crop::Oats => &[Product::RolledOats, Product::OatBran],
            _ => &[],
        };
        products
            .iter()
            .map(|&p| (p, kg * self.extraction(p)))
            .collect()
    }

    /// Kilograms of a dairy product made from `milk_kg` of whole milk.
    pub fn dairy(&self, product: Product, milk_kg: f64) -> f64 {
        if DAIRY.contains(&product) {
            milk_kg * self.extraction(product)
        } else {
            0.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_rates_are_fao_new_zealand_s() {
        let m = ProcessingModel::load_default().expect("the pack loads");
        assert_eq!(m.extraction(Product::WheatFlour), 0.78);
        assert!((m.extraction(Product::Butter) - 0.061).abs() < 1e-12);
        assert_eq!(m.extraction(Product::WholeMilkPowder), 0.14);
    }

    #[test]
    fn milling_wheat_loses_nothing_and_oats_lose_their_hulls() {
        let m = ProcessingModel::load_default().expect("the pack loads");
        let wheat: f64 = m.mill(Crop::Wheat, 1_000.0).iter().map(|p| p.1).sum();
        assert!((wheat - 1_000.0).abs() < 1e-9);
        let oats: f64 = m.mill(Crop::Oats, 1_000.0).iter().map(|p| p.1).sum();
        assert!((oats - 820.0).abs() < 1e-9);
        assert!(m.mill(Crop::Potatoes, 1_000.0).is_empty());
    }

    #[test]
    fn the_dairy_factory_makes_only_dairy_products() {
        let m = ProcessingModel::load_default().expect("the pack loads");
        assert!((m.dairy(Product::Cheese, 1_000.0) - 90.0).abs() < 1e-9);
        assert_eq!(m.dairy(Product::WheatFlour, 1_000.0), 0.0);
    }
}
