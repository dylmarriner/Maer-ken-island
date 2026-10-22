//! Solar salt works (Phase 3 Task 2 salt pans, for the food chain).
//!
//! Seawater evaporates through concentrating ponds into crystallisers,
//! and the salt crust is harvested once a year. A works yields Dominion
//! Salt's Lake Grassmere rate per hectare of its whole production area
//! (the pack's 42.4-49.4 t/ha/yr, derived from 60,000-70,000 t a year
//! over 1,416 ha); this module uses the middle. Grassmere's harvest has
//! run from nil (1986) to 130,000 t (1998) with the rain, which a mean
//! rate leaves out: the works does not yet read the island's weather.

use serde::{Deserialize, Serialize};

use super::petroleum::MissingReference;
use crate::validation::{ReferenceDomain, ReferenceLibrary};

const YEAR_S: f64 = 365.25 * 86_400.0;
const YIELD: &str = "solar_salt_yield_lake_grassmere_t_per_ha_yr";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SaltModel {
    /// Salt per hectare of total production area (t/ha/yr).
    pub t_per_ha_year: f64,
}

impl SaltModel {
    pub fn from_reference(library: &ReferenceLibrary) -> Result<Self, MissingReference> {
        let t_per_ha_year = library
            .item(ReferenceDomain::Geology, YIELD)
            .and_then(|i| i.central())
            .ok_or_else(|| MissingReference(YIELD.into()))?;
        Ok(Self { t_per_ha_year })
    }

    pub fn load_default() -> Result<Self, MissingReference> {
        let library = ReferenceLibrary::load(&crate::validation::default_reference_dir())
            .map_err(|e| MissingReference(e.to_string()))?;
        Self::from_reference(&library)
    }
}

/// A works and the salt it has made.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SaltWorks {
    /// Total production area: ponds, crystallisers and storage (ha).
    pub area_ha: f64,
    pub salt_t: f64,
}

impl SaltWorks {
    pub fn new(area_ha: f64) -> Self {
        Self {
            area_ha,
            salt_t: 0.0,
        }
    }

    /// Make salt for `dt_s`; returns the tonnes made.
    pub fn step(&mut self, model: &SaltModel, dt_s: f64) -> f64 {
        let made = self.area_ha * model.t_per_ha_year * dt_s / YEAR_S;
        self.salt_t += made;
        made
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grassmere_s_area_makes_grassmere_s_harvest() {
        let m = SaltModel::load_default().expect("the pack loads");
        assert!((m.t_per_ha_year - 0.5 * (42.4 + 49.4)).abs() < 1e-9);
        let mut works = SaltWorks::new(1_416.0);
        for _ in 0..365 {
            works.step(&m, YEAR_S / 365.0);
        }
        assert!(
            (60_000.0..=70_000.0).contains(&works.salt_t),
            "{}",
            works.salt_t
        );
    }
}
