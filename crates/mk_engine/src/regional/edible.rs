//! Plant food people can actually eat (realism R3, first step).
//!
//! Before this, a meal took plant matter from a cell's whole standing
//! biomass, so a forest's wood and bark fed people as well as its fruit.
//! Here a cell holds an **edible stock**: the fruit and seed its plants
//! make, which is the only plant food a person can gather.
//!
//! How much: a forest puts a share of its production into reproduction.
//! Ward et al. (2025) measure it as reproductive litterfall over
//! reproductive plus leaf litterfall, R/(R+L), and validate it against
//! R/NPP; the means are 0.082 for boreal, 0.116 for temperate and 0.129 for
//! tropical forests (`fixtures/reference/ecology/food_plants.json`). The
//! stock grows at NPP times that share. R counts flowers, husks and cone
//! scales as well as fruit and seed, so as the edible share it is an upper
//! bound (D57).
//!
//! Where: forest biomes only. No source read gives a human-edible share of
//! grass leaf, shrubland, wetland or alpine production, so those cells hold
//! no plant food (D57). Bracken rhizome, raupo and the other foods the
//! research found are left for when their extent per cell is sourced.
//!
//! How long it keeps: the stock holds at most one year's production, since
//! fruit and seed fall, rot or are eaten by animals within the year; no
//! source was read for how fast, so the cap is the year itself (D57). It is
//! never more than the cell's standing biomass, of which it is a part:
//! eating it takes the same carbon from both.

use mk_core::biomes::BiomeType;
use serde::{Deserialize, Serialize};

use crate::validation::{ReferenceDomain, ReferenceLibrary};

/// Reproductive allocation by forest biome, from the reference packs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReproductiveShares {
    pub boreal: f64,
    pub temperate: f64,
    pub tropical: f64,
}

impl ReproductiveShares {
    pub fn from_reference(library: &ReferenceLibrary) -> Result<Self, String> {
        let mean = |row: &str| {
            library
                .item(
                    ReferenceDomain::Ecology,
                    "reproductive_allocation_by_forest_biome",
                )
                .and_then(|i| i.table())
                .and_then(|t| t.get(row, "mean"))
                .ok_or_else(|| format!("reproductive_allocation_by_forest_biome/{row}/mean"))
        };
        Ok(Self {
            boreal: mean("boreal")?,
            temperate: mean("temperate")?,
            tropical: mean("tropical")?,
        })
    }

    /// The share of a cell's production that is fruit and seed, or `None`
    /// where no source gives one. Montane forest and woodland are taken as
    /// temperate forest: the island's are temperate (D57).
    pub fn for_biome(&self, biome: BiomeType) -> Option<f64> {
        use BiomeType::*;
        match biome {
            BorealForest => Some(self.boreal),
            TemperateForest | MontaneForest | Woodland => Some(self.temperate),
            TropicalRainforest | TropicalDryForest => Some(self.tropical),
            _ => None,
        }
    }
}

/// Each land cell's edible plant food.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EdibleFood {
    /// Share of the cell's NPP that becomes fruit and seed; 0 where there
    /// is none.
    pub share: Vec<f64>,
    /// Standing fruit and seed, kg C per m².
    pub kgc_m2: Vec<f64>,
}

impl EdibleFood {
    /// A full year's crop on every cell that makes one, as if the island
    /// starts after its fruiting season: no source gives the stock at an
    /// arbitrary moment of the year (D57), and starting empty would starve
    /// the founders on their first day for want of a calendar.
    pub fn install(
        shares: &ReproductiveShares,
        biomes: &[BiomeType],
        npp_kgc_m2_yr: &[f64],
        biomass_kgc_m2: &[f64],
    ) -> Self {
        let share: Vec<f64> = biomes
            .iter()
            .map(|b| shares.for_biome(*b).unwrap_or(0.0))
            .collect();
        let kgc_m2 = share
            .iter()
            .zip(npp_kgc_m2_yr)
            .zip(biomass_kgc_m2)
            .map(|((s, npp), held)| (s * npp.max(0.0)).min(held.max(0.0)))
            .collect();
        Self { share, kgc_m2 }
    }

    /// Grow the stock by `dt_years` of production, up to a year's crop and
    /// never past what stands on the cell.
    pub fn step(&mut self, npp_kgc_m2_yr: &[f64], biomass_kgc_m2: &[f64], dt_years: f64) {
        if !(dt_years.is_finite() && dt_years > 0.0) {
            return;
        }
        for (((stock, share), npp), held) in self
            .kgc_m2
            .iter_mut()
            .zip(&self.share)
            .zip(npp_kgc_m2_yr)
            .zip(biomass_kgc_m2)
        {
            let year = share * npp.max(0.0);
            *stock = (*stock + year * dt_years).min(year).min(held.max(0.0));
        }
    }

    /// The edible carbon on cell `i` (kg C per m²).
    pub fn at(&self, i: usize) -> f64 {
        self.kgc_m2.get(i).copied().unwrap_or(0.0)
    }

    /// Take `kgc_m2` of it from cell `i`; returns what was there to take.
    pub fn take(&mut self, i: usize, kgc_m2: f64) -> f64 {
        let Some(stock) = self.kgc_m2.get_mut(i) else {
            return 0.0;
        };
        let taken = kgc_m2.max(0.0).min(*stock);
        *stock -= taken;
        taken
    }

    /// All of it, kg C, over cells of `cell_area_m2`.
    pub fn total_kgc(&self, cell_area_m2: f64) -> f64 {
        self.kgc_m2.iter().sum::<f64>() * cell_area_m2
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shares() -> ReproductiveShares {
        let library = ReferenceLibrary::load(&crate::validation::default_reference_dir()).unwrap();
        ReproductiveShares::from_reference(&library).unwrap()
    }

    #[test]
    fn the_shares_are_the_sources_biome_means() {
        let s = shares();
        assert_eq!((s.boreal, s.temperate, s.tropical), (0.082, 0.116, 0.129));
        // Rising from the boreal to the tropics, as the source finds.
        assert!(s.boreal < s.temperate && s.temperate < s.tropical);
    }

    #[test]
    fn only_forest_makes_food_and_a_forest_starts_with_a_years_crop() {
        use BiomeType::*;
        let food = EdibleFood::install(
            &shares(),
            &[TemperateForest, Grassland, Wetland, ShallowOcean],
            &[0.8, 0.6, 1.0, 0.0],
            &[20.0, 0.5, 2.0, 0.0],
        );
        assert!((food.at(0) - 0.8 * 0.116).abs() < 1e-12);
        for i in 1..4 {
            assert_eq!(food.at(i), 0.0, "cell {i} has no sourced plant food");
        }
    }

    #[test]
    fn the_stock_regrows_at_its_share_of_production_up_to_a_year() {
        let mut food =
            EdibleFood::install(&shares(), &[BiomeType::TemperateForest], &[1.0], &[20.0]);
        let year = 0.116;
        food.take(0, year);
        assert_eq!(food.at(0), 0.0);
        food.step(&[1.0], &[20.0], 0.25);
        assert!((food.at(0) - year * 0.25).abs() < 1e-12);
        // And no further than a year's crop, however long it is left.
        food.step(&[1.0], &[20.0], 5.0);
        assert!((food.at(0) - year).abs() < 1e-12);
    }

    #[test]
    fn the_stock_is_never_more_than_what_stands() {
        let mut food =
            EdibleFood::install(&shares(), &[BiomeType::TemperateForest], &[1.0], &[0.05]);
        assert_eq!(food.at(0), 0.05);
        food.step(&[1.0], &[0.01], 0.1);
        assert_eq!(food.at(0), 0.01);
    }

    #[test]
    fn taking_more_than_is_there_takes_what_is_there() {
        let mut food =
            EdibleFood::install(&shares(), &[BiomeType::TemperateForest], &[1.0], &[20.0]);
        assert!((food.take(0, 9.0) - 0.116).abs() < 1e-12);
        assert_eq!(food.take(0, 1.0), 0.0);
        assert_eq!(food.take(7, 1.0), 0.0, "no such cell");
    }
}
