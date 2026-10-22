//! A radiata pine plantation (Phase 4b Task 7b, timber chain).
//!
//! Stands are even-aged, as more than nine-tenths of New Zealand's planted
//! forest is. A stand is clearfelled only once it reaches the national
//! average clearfell age, 28.2 years, for the national average clearfell
//! yield, 486 m³/ha of recoverable wood (NEFD 2006, Lane 2007), and is
//! replanted the same year. So the plantation never cuts more than it has
//! grown: a forest of evenly spread age classes yields its area times
//! 486 / 28.2 m³ a year, about 17 m³/ha, and no more.

use serde::{Deserialize, Serialize};

use super::petroleum::MissingReference;
use crate::validation::{ReferenceDomain, ReferenceLibrary};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlantationModel {
    pub rotation_years: f64,
    pub clearfell_m3_ha: f64,
}

impl PlantationModel {
    pub fn from_reference(library: &ReferenceLibrary) -> Result<Self, MissingReference> {
        let point = |key: &str| {
            library
                .item(ReferenceDomain::Industry, key)
                .and_then(|i| i.point())
                .ok_or_else(|| MissingReference(key.into()))
        };
        Ok(Self {
            rotation_years: point("radiata_clearfell_age_years")?,
            clearfell_m3_ha: point("radiata_clearfell_yield_m3_ha")?,
        })
    }

    pub fn load_default() -> Result<Self, MissingReference> {
        let library = ReferenceLibrary::load(&crate::validation::default_reference_dir())
            .map_err(|e| MissingReference(e.to_string()))?;
        Self::from_reference(&library)
    }

    /// m³ a hectare grows a year over a rotation.
    pub fn mean_annual_increment(&self) -> f64 {
        self.clearfell_m3_ha / self.rotation_years
    }
}

/// One even-aged stand.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Stand {
    pub area_ha: f64,
    /// Simulation year it was (re)planted; may be negative for a stand
    /// planted before the simulation began.
    pub planted_year: f64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Plantation {
    pub stands: Vec<Stand>,
}

impl Plantation {
    /// `area_ha` planted in one stand a year, so the forest's age classes
    /// are even and its clearfells steady from the first year.
    pub fn even_aged_classes(model: &PlantationModel, area_ha: f64) -> Self {
        let classes = model.rotation_years.ceil() as usize;
        let each = area_ha / classes as f64;
        Self {
            stands: (0..classes)
                .map(|k| Stand {
                    area_ha: each,
                    planted_year: -(k as f64),
                })
                .collect(),
        }
    }

    pub fn area_ha(&self) -> f64 {
        self.stands.iter().map(|s| s.area_ha).sum()
    }

    /// Clearfells every stand at or past rotation age in `year`, replants
    /// it, and returns the m³ of logs cut.
    pub fn clearfell(&mut self, model: &PlantationModel, year: f64) -> f64 {
        let mut cut = 0.0;
        for stand in &mut self.stands {
            if year - stand.planted_year >= model.rotation_years {
                cut += stand.area_ha * model.clearfell_m3_ha;
                stand.planted_year = year;
            }
        }
        cut
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_nefd_rotation_and_yield_come_from_the_pack() {
        let m = PlantationModel::load_default().expect("the pack loads");
        assert_eq!(m.rotation_years, 28.2);
        assert_eq!(m.clearfell_m3_ha, 486.0);
        assert!((m.mean_annual_increment() - 17.23).abs() < 0.01);
    }

    #[test]
    fn a_plantation_never_cuts_more_than_it_grows() {
        let m = PlantationModel::load_default().expect("the pack loads");
        let mut forest = Plantation::even_aged_classes(&m, 290.0);
        let mut cut = 0.0;
        let years = 300;
        for year in 1..=years {
            cut += forest.clearfell(&m, year as f64);
            // Nothing standing is ever older than the rotation after a cut.
            for s in &forest.stands {
                assert!(year as f64 - s.planted_year < m.rotation_years);
            }
        }
        let grown = forest.area_ha() * m.mean_annual_increment() * years as f64;
        assert!(cut > 0.0);
        assert!(cut <= grown, "cut {cut} m³, grown {grown} m³");
    }

    #[test]
    fn a_young_stand_is_not_felled() {
        let m = PlantationModel::load_default().expect("the pack loads");
        let mut forest = Plantation {
            stands: vec![Stand {
                area_ha: 10.0,
                planted_year: 0.0,
            }],
        };
        assert_eq!(forest.clearfell(&m, 28.0), 0.0);
        assert_eq!(forest.clearfell(&m, 29.0), 4_860.0);
        assert_eq!(forest.stands[0].planted_year, 29.0);
    }
}
