//! Sheep and beef breeding flocks and herds (Phase 4b Task 7b, food chain).
//!
//! A North Island finishing farm tails 140.3 lambs and weans 87.5 calves
//! per hundred females mated (Beef + Lamb New Zealand 2024-25, Class 5).
//! Lambs go to slaughter in their first season at the national average
//! carcass, 19.93 kg (provisional 2025-26); steers before their second
//! winter at 18-22 months, at the export steer's 316 kg (Williamson 2021;
//! heifers, 220-250 kg, are not told apart yet). The flock keeps itself:
//! each season ewe lambs replace the 25-35% of ewes lost to deaths and
//! culls (Ridler, Massey, in Country-Wide 2019) and the rest of the lambs
//! are killed. No cattle replacement rate has been read yet, so every calf
//! is finished and the cows are held constant. Cull ewes' mutton is not
//! counted.
//!
//! Sows farrow all year. The average New Zealand sow yields 1.7 t of pork
//! carcass a year at 71.1 kg a pig (NZ Pork 2022), so 23.9 pigs, killed at
//! five to six months (NZ Pork). The sow herd is held constant: no sow
//! replacement rate has been read.

use serde::{Deserialize, Serialize};

use super::petroleum::MissingReference;
use crate::validation::{ReferenceDomain, ReferenceLibrary};

const CLASS: &str = "class5_ni_finishing";
const MONTH_S: f64 = 365.25 * 86_400.0 / 12.0;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LivestockModel {
    pub lambs_per_ewe: f64,
    /// Share of the ewes replaced by ewe lambs each season.
    pub ewe_replacement: f64,
    pub calves_per_cow: f64,
    pub lamb_carcass_kg: f64,
    pub steer_carcass_kg: f64,
    /// Months from birth to slaughter: steers finish at 18-22.
    pub steer_finish_months: f64,
    pub pigs_per_sow_year: f64,
    pub pig_carcass_kg: f64,
    /// Months from birth to slaughter: pigs go at five to six.
    pub pig_finish_months: f64,
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
        let pig = |row: &str| cell("pig_production_per_sow", row, "kg_carcass");
        let pig_carcass_kg = pig("average_carcass")?;
        let (pig_min, pig_max) = library
            .item(ReferenceDomain::Industry, "pig_slaughter_age")
            .and_then(|i| i.range())
            .ok_or_else(|| MissingReference("pig_slaughter_age".into()))?;
        Ok(Self {
            lambs_per_ewe: cell(stock, CLASS, "lambing_percent")? / 100.0,
            ewe_replacement: library
                .item(ReferenceDomain::Industry, "ewe_flock_replacement_rate")
                .and_then(|i| i.central())
                .ok_or_else(|| MissingReference("ewe_flock_replacement_rate".into()))?
                / 100.0,
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
            pigs_per_sow_year: pig("pork_per_sow_per_year")? / pig_carcass_kg,
            pig_carcass_kg,
            pig_finish_months: 0.5 * (pig_min + pig_max),
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
    pub sows: f64,
    pub piglets: Vec<Cohort>,
}

/// Carcass meat sent to slaughter.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct Meat {
    pub lamb_kg: f64,
    pub beef_kg: f64,
    pub pork_kg: f64,
}

impl Stock {
    /// The year's lambing and calving at `now_s`. Ewe lambs kept as
    /// replacements take the places of the ewes lost this year, so the
    /// flock holds; the other lambs are finished the same season and go
    /// straight to slaughter. Calves join a cohort.
    pub fn birth_season(&mut self, model: &LivestockModel, now_s: f64) -> Meat {
        let kept = self.ewes * model.ewe_replacement;
        let lambs = (self.ewes * model.lambs_per_ewe - kept).max(0.0);
        let calves = self.cows * model.calves_per_cow;
        if calves > 0.0 {
            self.calves.push(Cohort {
                head: calves,
                born_s: now_s,
            });
        }
        Meat {
            lamb_kg: lambs * model.lamb_carcass_kg,
            ..Meat::default()
        }
    }

    /// The pigs farrowed over `dt_s` ending at `now_s`, as one cohort.
    pub fn farrow(&mut self, model: &LivestockModel, dt_s: f64, now_s: f64) {
        let head = self.sows * model.pigs_per_sow_year * dt_s / (12.0 * MONTH_S);
        if head > 0.0 {
            self.piglets.push(Cohort {
                head,
                born_s: now_s,
            });
        }
    }

    /// Sends every cohort that has reached finishing age by `now_s`.
    pub fn finish(&mut self, model: &LivestockModel, now_s: f64) -> Meat {
        let send = |young: &mut Vec<Cohort>, months: f64, carcass_kg: f64| {
            let mut kg = 0.0;
            young.retain(|c| {
                let ready = now_s - c.born_s >= months * MONTH_S;
                if ready {
                    kg += c.head * carcass_kg;
                }
                !ready
            });
            kg
        };
        Meat {
            lamb_kg: 0.0,
            beef_kg: send(
                &mut self.calves,
                model.steer_finish_months,
                model.steer_carcass_kg,
            ),
            pork_kg: send(
                &mut self.piglets,
                model.pig_finish_months,
                model.pig_carcass_kg,
            ),
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
        assert!((m.ewe_replacement - 0.30).abs() < 1e-12);
        assert_eq!(m.calves_per_cow, 0.875);
        assert_eq!(m.lamb_carcass_kg, 19.93);
        assert_eq!(m.steer_carcass_kg, 316.0);
        assert_eq!(m.steer_finish_months, 20.0);
        assert_eq!(m.pig_carcass_kg, 71.1);
        assert!((m.pigs_per_sow_year - 1_700.0 / 71.1).abs() < 1e-9);
        assert_eq!(m.pig_finish_months, 5.5);
    }

    #[test]
    fn a_sow_herd_in_steady_farrowing_sends_its_pork_a_year() {
        let m = LivestockModel::load_default().expect("the pack loads");
        let mut stock = Stock {
            sows: 50.0,
            ..Stock::default()
        };
        let day_s = 86_400.0;
        let mut pork = 0.0;
        let mut now_s = 0.0;
        // A year to fill the pipeline, then a year counted.
        for day in 0..(2 * 365) {
            now_s += day_s;
            stock.farrow(&m, day_s, now_s);
            let sent = stock.finish(&m, now_s).pork_kg;
            if day >= 365 {
                pork += sent;
            }
        }
        let expected = 50.0 * 1_700.0 * 365.0 * day_s / YEAR_S;
        assert!(
            (pork - expected).abs() < 1e-6 * expected,
            "{pork} vs {expected}"
        );
        assert!(
            stock.piglets.len() > 100,
            "five months of litters growing on"
        );
    }

    #[test]
    fn calves_finish_before_their_second_winter_and_lambs_the_same_season() {
        let m = LivestockModel::load_default().expect("the pack loads");
        let mut stock = Stock {
            ewes: 1_000.0,
            cows: 100.0,
            ..Stock::default()
        };
        let lamb = stock.birth_season(&m, 0.0);
        assert!((lamb.lamb_kg - (1_403.0 - 300.0) * 19.93).abs() < 1e-6);
        assert_eq!(stock.ewes, 1_000.0, "replacements keep the flock");
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
