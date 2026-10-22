//! Sheep and beef breeding flocks and herds (Phase 4b Task 7b, food chain).
//!
//! A North Island finishing farm tails 140.3 lambs and weans 87.5 calves
//! per hundred females mated (Beef + Lamb New Zealand 2024-25, Class 5).
//! Lambs go to slaughter in their first season at the national average
//! carcass, 19.93 kg (provisional 2025-26); steers before their second
//! winter at 18-22 months, at the export steer's 316 kg (Williamson 2021;
//! heifers, 220-250 kg, are not told apart yet). Every lamb and calf
//! is sold for now: no source read gives a replacement rate, so the
//! breeding stock is held constant and its own deaths and culls are not
//! modelled (D-register Task 7b note).

use serde::{Deserialize, Serialize};

use super::petroleum::MissingReference;
use crate::validation::{ReferenceDomain, ReferenceLibrary};

const CLASS: &str = "class5_ni_finishing";
const MONTH_S: f64 = 365.25 * 86_400.0 / 12.0;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LivestockModel {
    pub lambs_per_ewe: f64,
    pub calves_per_cow: f64,
    pub lamb_carcass_kg: f64,
    pub steer_carcass_kg: f64,
    /// Months from birth to slaughter: steers finish at 18-22.
    pub steer_finish_months: f64,
}

impl LivestockModel {
    pub fn from_reference(library: &ReferenceLibrary) -> Result<Self, MissingReference> {
        let cell = |key: &str, row: &str, column: &str| {
            library
                .item(ReferenceDomain::Industry, key)
                .and_then(|i| i.table())
                .and_then(|t| t.get(row, column))
                .ok_or_else(|| MissingReference(format!("{key}/{row}/{column}")))
        };
        let stock = "sheep_beef_stocking_by_class";
        let finish = "beef_slaughter_age";
        let ideal = "ideal_finish_before_second_winter";
        Ok(Self {
            lambs_per_ewe: cell(stock, CLASS, "lambing_percent")? / 100.0,
            calves_per_cow: cell(stock, CLASS, "calving_percent")? / 100.0,
            lamb_carcass_kg: cell(
                "lamb_hot_carcass_weight_nz",
                "cumulative_2025_26_season_to_week_51",
                "kg_hot_weight",
            )?,
            steer_carcass_kg: cell(
                "beef_carcass_weight_nz",
                "average_steer_export",
                "kg_carcass",
            )?,
            steer_finish_months: 0.5
                * (cell(finish, ideal, "min_months")? + cell(finish, ideal, "max_months")?),
        })
    }

    pub fn load_default() -> Result<Self, MissingReference> {
        let library = ReferenceLibrary::load(&crate::validation::default_reference_dir())
            .map_err(|e| MissingReference(e.to_string()))?;
        Self::from_reference(&library)
    }
}

/// Young stock born on one date, growing to slaughter.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Cohort {
    pub head: f64,
    pub born_s: f64,
}

/// A breeding flock and herd and the young growing on.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Stock {
    pub ewes: f64,
    pub cows: f64,
    pub calves: Vec<Cohort>,
}

/// Carcass meat sent to slaughter.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct Meat {
    pub lamb_kg: f64,
    pub beef_kg: f64,
}

impl Stock {
    /// The year's lambing and calving at `now_s`. Lambs are finished the
    /// same season, so they go straight to slaughter; calves join a cohort.
    pub fn birth_season(&mut self, model: &LivestockModel, now_s: f64) -> Meat {
        let lambs = self.ewes * model.lambs_per_ewe;
        let calves = self.cows * model.calves_per_cow;
        if calves > 0.0 {
            self.calves.push(Cohort {
                head: calves,
                born_s: now_s,
            });
        }
        Meat {
            lamb_kg: lambs * model.lamb_carcass_kg,
            beef_kg: 0.0,
        }
    }

    /// Sends every cohort that has reached finishing age by `now_s`.
    pub fn finish(&mut self, model: &LivestockModel, now_s: f64) -> Meat {
        let age_s = model.steer_finish_months * MONTH_S;
        let mut beef_kg = 0.0;
        self.calves.retain(|c| {
            let ready = now_s - c.born_s >= age_s;
            if ready {
                beef_kg += c.head * model.steer_carcass_kg;
            }
            !ready
        });
        Meat {
            lamb_kg: 0.0,
            beef_kg,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const YEAR_S: f64 = 12.0 * MONTH_S;

    #[test]
    fn the_flock_and_herd_rates_are_the_packs() {
        let m = LivestockModel::load_default().expect("the pack loads");
        assert!((m.lambs_per_ewe - 1.403).abs() < 1e-12);
        assert_eq!(m.calves_per_cow, 0.875);
        assert_eq!(m.lamb_carcass_kg, 19.93);
        assert_eq!(m.steer_carcass_kg, 316.0);
        assert_eq!(m.steer_finish_months, 20.0);
    }

    #[test]
    fn calves_finish_before_their_second_winter_and_lambs_the_same_season() {
        let m = LivestockModel::load_default().expect("the pack loads");
        let mut stock = Stock {
            ewes: 1_000.0,
            cows: 100.0,
            calves: vec![],
        };
        let lamb = stock.birth_season(&m, 0.0);
        assert!((lamb.lamb_kg - 1_403.0 * 19.93).abs() < 1e-6);
        assert_eq!(stock.finish(&m, YEAR_S).beef_kg, 0.0);
        stock.birth_season(&m, YEAR_S);
        let beef = stock.finish(&m, 2.0 * YEAR_S);
        assert!((beef.beef_kg - 87.5 * 316.0).abs() < 1e-6);
        assert_eq!(
            stock.calves.len(),
            1,
            "the second year's calves still growing"
        );
    }
}
