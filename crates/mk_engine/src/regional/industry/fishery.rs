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

/// Midpoint of a published growth-rate prior after validating both source bounds.
/// Subtracting first avoids overflowing an otherwise representable midpoint.
fn checked_rate_midpoint(min: f64, max: f64) -> Option<f64> {
    if !min.is_finite() || !max.is_finite() || min < 0.0 || max < min {
        return None;
    }
    let midpoint = min + (max - min) * 0.5;
    if midpoint.is_finite() {
        Some(midpoint)
    } else {
        None
    }
}

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
            let r_min = get("r_min")?;
            let r_max = get("r_max")?;
            *slot = checked_rate_midpoint(r_min, r_max).ok_or_else(|| {
                MissingReference(format!(
                    "{R}/{} has invalid growth-rate bounds",
                    class.label()
                ))
            })?;
        }
        const D: &str = "schaefer_depensation_threshold_of_k";
        let depensation_below = library
            .item(ReferenceDomain::Industry, D)
            .and_then(|i| i.point())
            .ok_or_else(|| MissingReference(D.into()))?;
        let model = Self {
            r_per_year,
            depensation_below,
        };
        if !model.is_valid() {
            return Err(MissingReference(
                "fishery model contains invalid growth or depensation parameters".into(),
            ));
        }
        Ok(model)
    }

    fn is_valid(&self) -> bool {
        self.r_per_year
            .iter()
            .all(|rate| rate.is_finite() && *rate >= 0.0)
            && self.depensation_below.is_finite()
            && (0.0..=1.0).contains(&self.depensation_below)
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
        // Invalid inputs or state must not reach f64::clamp (which can panic
        // when its bounds are unordered) or poison the persisted stock.
        if !model.is_valid()
            || !wanted_t.is_finite()
            || wanted_t < 0.0
            || !dt_s.is_finite()
            || dt_s < 0.0
            || !self.k_t.is_finite()
            || self.k_t < 0.0
            || !self.biomass_t.is_finite()
            || self.biomass_t < 0.0
            || self.biomass_t > self.k_t
            || !self.landed_t.is_finite()
            || self.landed_t < 0.0
        {
            return 0.0;
        }

        let grown = self.surplus_t_per_year(model).max(0.0) * dt_s / YEAR_S;
        if !grown.is_finite() || grown < 0.0 {
            return 0.0;
        }
        let post_growth = self.biomass_t + grown;
        if !post_growth.is_finite() {
            return 0.0;
        }

        let catch = wanted_t.min(grown);
        let biomass_t = (post_growth - catch).min(self.k_t);
        let landed_t = self.landed_t + catch;
        if !biomass_t.is_finite() || !landed_t.is_finite() {
            return 0.0;
        }

        self.biomass_t = biomass_t;
        self.landed_t = landed_t;
        catch
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn growth_rate_prior_bounds_must_be_finite_nonnegative_and_ordered() {
        for (min, max) in [
            (-0.01, 1.0),
            (0.5, 0.4),
            (f64::NAN, 1.0),
            (0.0, f64::INFINITY),
            (f64::NEG_INFINITY, 0.5),
        ] {
            assert_eq!(checked_rate_midpoint(min, max), None);
        }
    }

    #[test]
    fn growth_rate_prior_midpoint_avoids_intermediate_overflow() {
        let midpoint = checked_rate_midpoint(f64::MAX * 0.5, f64::MAX).unwrap();
        assert!(midpoint.is_finite());
        assert!(midpoint >= f64::MAX * 0.5);
        assert!(midpoint <= f64::MAX);
        assert!((checked_rate_midpoint(0.6, 1.5).unwrap() - 1.05).abs() < 1e-12);
    }

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
        assert!(stock.landed_t <= stock.msy_t(&m) * 3_653.0 * day / YEAR_S + 1e-6);
    }

    #[test]
    fn invalid_fishing_inputs_are_rejected_without_mutating_the_stock() {
        let m = FisheryModel::load_default().expect("the pack loads");
        let mut stock = FishStock::unexploited(Resilience::Medium, 1_000.0);
        stock.biomass_t = 500.0;
        stock.landed_t = 25.0;
        let before = stock.clone();

        for (wanted, dt) in [
            (f64::NAN, YEAR_S),
            (f64::INFINITY, YEAR_S),
            (f64::NEG_INFINITY, YEAR_S),
            (-1.0, YEAR_S),
            (1.0, f64::NAN),
            (1.0, f64::INFINITY),
            (1.0, f64::NEG_INFINITY),
            (1.0, -1.0),
        ] {
            assert_eq!(stock.fish(&m, wanted, dt), 0.0);
            assert_eq!(
                stock, before,
                "invalid input ({wanted}, {dt}) mutated the stock"
            );
        }
    }

    #[test]
    fn invalid_model_and_stock_state_are_rejected_without_mutation() {
        let m = FisheryModel::load_default().expect("the pack loads");
        let mut stock = FishStock::unexploited(Resilience::Medium, 1_000.0);
        stock.biomass_t = 500.0;
        let before = stock.clone();

        let mut invalid_model = m.clone();
        invalid_model.r_per_year[Resilience::Medium as usize] = f64::NAN;
        assert!(!invalid_model.is_valid());
        assert_eq!(stock.fish(&invalid_model, 10.0, YEAR_S), 0.0);
        assert_eq!(stock, before);

        invalid_model = m.clone();
        invalid_model.depensation_below = 1.01;
        assert!(!invalid_model.is_valid());
        assert_eq!(stock.fish(&invalid_model, 10.0, YEAR_S), 0.0);
        assert_eq!(stock, before);

        let mut invalid_stock = before.clone();
        invalid_stock.biomass_t = invalid_stock.k_t + 1.0;
        let invalid_before = invalid_stock.clone();
        assert_eq!(invalid_stock.fish(&m, 10.0, YEAR_S), 0.0);
        assert_eq!(invalid_stock, invalid_before);
    }

    #[test]
    fn overflowing_growth_is_rejected_without_mutating_the_stock() {
        let m = FisheryModel::load_default().expect("the pack loads");
        let mut explosive = m.clone();
        explosive.r_per_year[Resilience::Medium as usize] = f64::MAX;
        let mut stock = FishStock::unexploited(Resilience::Medium, 1.0e308);
        stock.biomass_t = 5.0e307;
        let before = stock.clone();

        assert_eq!(stock.fish(&explosive, 1.0, YEAR_S), 0.0);
        assert_eq!(stock, before);
    }
}
