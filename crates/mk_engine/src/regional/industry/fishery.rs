//! A fishery on one stock (Phase 4b Task 7b, food chain).
//!
//! The stock grows by the Schaefer surplus-production model as Froese et
//! al. (2017) write it: next year's biomass is this year's plus
//! `r (1 - B/k) B` less the catch, and below a quarter of `k` the surplus
//! falls off linearly (`× 4 B/k`), as recruitment fails. `r` is the middle
//! of the paper's prior range for the species' FishBase resilience class;
//! the most a stock can give year on year is `MSY = r k / 4`, at `B = k/2`.
//!
//! The fishery never takes more than the stock grows: its catch is capped
//! at the surplus of the moment, so harvest never exceeds regrowth.
//! Unexploited biomass `k` comes from the caller. The island's species
//! keep upstream's uncalibrated densities (D55), so a stock built from
//! them is only as good as those densities, and resilience is a class the
//! caller supplies, since no species carries FishBase's.

use serde::{Deserialize, Serialize};

use super::petroleum::MissingReference;
use crate::validation::{ReferenceDomain, ReferenceLibrary};

const YEAR_S: f64 = 365.25 * 86_400.0;

/// FishBase's resilience classes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Resilience {
    High,
    Medium,
    Low,
    VeryLow,
}

impl Resilience {
    fn label(self) -> &'static str {
        match self {
            Self::High => "high",
            Self::Medium => "medium",
            Self::Low => "low",
            Self::VeryLow => "very_low",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FisheryModel {
    /// Middle of the prior `r` range per class, per year, in
    /// High/Medium/Low/VeryLow order.
    pub r_per_year: [f64; 4],
    /// Share of `k` below which surplus declines linearly.
    pub depensation_below: f64,
}

impl FisheryModel {
    pub fn from_reference(library: &ReferenceLibrary) -> Result<Self, MissingReference> {
        const R: &str = "fish_intrinsic_growth_rate_by_resilience";
        let table = library
            .item(ReferenceDomain::Industry, R)
            .and_then(|i| i.table())
            .ok_or_else(|| MissingReference(R.into()))?;
        let mut r_per_year = [0.0; 4];
        let classes = [
            Resilience::High,
            Resilience::Medium,
            Resilience::Low,
            Resilience::VeryLow,
        ];
        for (slot, class) in r_per_year.iter_mut().zip(classes) {
            let get = |col: &str| {
                table
                    .get(class.label(), col)
                    .ok_or_else(|| MissingReference(format!("{R}/{}", class.label())))
            };
            *slot = 0.5 * (get("r_min")? + get("r_max")?);
        }
        const D: &str = "schaefer_depensation_threshold_of_k";
        let depensation_below = library
            .item(ReferenceDomain::Industry, D)
            .and_then(|i| i.point())
            .ok_or_else(|| MissingReference(D.into()))?;
        Ok(Self {
            r_per_year,
            depensation_below,
        })
    }

    pub fn load_default() -> Result<Self, MissingReference> {
        let library = ReferenceLibrary::load(&crate::validation::default_reference_dir())
            .map_err(|e| MissingReference(e.to_string()))?;
        Self::from_reference(&library)
    }

    pub fn r(&self, class: Resilience) -> f64 {
        self.r_per_year[class as usize]
    }
}

/// One stock and what has been landed from it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FishStock {
    pub resilience: Resilience,
    /// Unexploited biomass (t).
    pub k_t: f64,
    pub biomass_t: f64,
    pub landed_t: f64,
}

impl FishStock {
    /// An unfished stock, at `k`.
    pub fn unexploited(resilience: Resilience, k_t: f64) -> Self {
        Self {
            resilience,
            k_t,
            biomass_t: k_t,
            landed_t: 0.0,
        }
    }

    /// The stock's surplus production at its present biomass (t a year).
    pub fn surplus_t_per_year(&self, model: &FisheryModel) -> f64 {
        if self.k_t <= 0.0 {
            return 0.0;
        }
        let p = self.biomass_t / self.k_t;
        let surplus = model.r(self.resilience) * (1.0 - p) * self.biomass_t;
        if p < model.depensation_below {
            surplus * p / model.depensation_below
        } else {
            surplus
        }
    }

    /// The most the stock can give year on year (t), `r k / 4`.
    pub fn msy_t(&self, model: &FisheryModel) -> f64 {
        model.r(self.resilience) * self.k_t / 4.0
    }

    /// Step `dt_s`, fishing for `wanted_t` over the step. The catch is
    /// capped at the stock's surplus over the step, so the fishery never
    /// takes more than grows; returns what was landed.
    pub fn fish(&mut self, model: &FisheryModel, wanted_t: f64, dt_s: f64) -> f64 {
        let grown = self.surplus_t_per_year(model).max(0.0) * dt_s / YEAR_S;
        let catch = wanted_t.clamp(0.0, grown);
        self.biomass_t = (self.biomass_t + grown - catch).min(self.k_t);
        self.landed_t += catch;
        catch
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_growth_rates_are_the_middle_of_froese_s_priors() {
        let m = FisheryModel::load_default().expect("the pack loads");
        assert!((m.r(Resilience::High) - 1.05).abs() < 1e-12);
        assert!((m.r(Resilience::Medium) - 0.5).abs() < 1e-12);
        assert!((m.r(Resilience::VeryLow) - 0.0575).abs() < 1e-12);
        assert_eq!(m.depensation_below, 0.25);
    }

    #[test]
    fn surplus_peaks_at_half_k_and_falls_away_when_depleted() {
        let m = FisheryModel::load_default().expect("the pack loads");
        let mut stock = FishStock::unexploited(Resilience::Medium, 1_000.0);
        assert_eq!(stock.surplus_t_per_year(&m), 0.0);
        stock.biomass_t = 500.0;
        assert!((stock.surplus_t_per_year(&m) - stock.msy_t(&m)).abs() < 1e-9);
        stock.biomass_t = 100.0;
        // 0.5 × 0.9 × 100 = 45, then × 4 × 0.1 for depensation.
        assert!((stock.surplus_t_per_year(&m) - 18.0).abs() < 1e-9);
    }

    #[test]
    fn a_fishery_cannot_take_more_than_the_stock_grows() {
        let m = FisheryModel::load_default().expect("the pack loads");
        let mut stock = FishStock::unexploited(Resilience::Medium, 1_000.0);
        stock.biomass_t = 500.0;
        let day = 86_400.0;
        for _ in 0..3_653 {
            let before = stock.biomass_t;
            let landed = stock.fish(&m, 10.0, day);
            assert!(stock.biomass_t >= before - 1e-9, "the stock never shrinks");
            assert!(landed <= stock.msy_t(&m) * day / YEAR_S + 1e-9);
        }
        assert!(stock.landed_t > 0.0);
        assert!(stock.landed_t <= stock.msy_t(&m) * 10.0 + 1e-6);
    }
}
