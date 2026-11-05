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

/// Invalid fishery inputs are explicit errors; a failed step never mutates its stock.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FisheryError {
    InvalidModel,
    InvalidCarryingCapacity,
    InvalidCatchRequest,
    InvalidTimeStep,
    InvalidStockState,
    NonFiniteGrowth,
    LandedTotalOverflow,
}

impl std::fmt::Display for FisheryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            Self::InvalidModel => "fishery model contains invalid parameters",
            Self::InvalidCarryingCapacity => {
                "fish stock carrying capacity must be finite and positive"
            }
            Self::InvalidCatchRequest => "fish catch request must be finite and non-negative",
            Self::InvalidTimeStep => "fishery time step must be finite and non-negative",
            Self::InvalidStockState => "fish stock state is outside its valid domain",
            Self::NonFiniteGrowth => "fish stock growth is not finite",
            Self::LandedTotalOverflow => "cumulative landed biomass overflowed",
        };
        f.write_str(message)
    }
}

impl std::error::Error for FisheryError {}

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
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FishStock {
    pub resilience: Resilience,
    /// Unexploited biomass (t).
    pub k_t: f64,
    pub biomass_t: f64,
    pub landed_t: f64,
}

#[derive(Deserialize)]
struct FishStockSnapshot {
    resilience: Resilience,
    k_t: f64,
    biomass_t: f64,
    landed_t: f64,
}

impl<'de> Deserialize<'de> for FishStock {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let snapshot = FishStockSnapshot::deserialize(deserializer)?;
        let stock = Self {
            resilience: snapshot.resilience,
            k_t: snapshot.k_t,
            biomass_t: snapshot.biomass_t,
            landed_t: snapshot.landed_t,
        };
        stock.validate_state().map_err(serde::de::Error::custom)?;
        Ok(stock)
    }
}

impl FishStock {
    /// An unfished stock, at `k`. Invalid capacities are rejected at construction.
    pub fn unexploited(resilience: Resilience, k_t: f64) -> Result<Self, FisheryError> {
        if !k_t.is_finite() || k_t <= 0.0 {
            return Err(FisheryError::InvalidCarryingCapacity);
        }
        Ok(Self {
            resilience,
            k_t,
            biomass_t: k_t,
            landed_t: 0.0,
        })
    }

    fn validate_state(&self) -> Result<(), FisheryError> {
        if !self.k_t.is_finite() || self.k_t <= 0.0 {
            return Err(FisheryError::InvalidCarryingCapacity);
        }
        if !self.biomass_t.is_finite()
            || self.biomass_t < 0.0
            || self.biomass_t > self.k_t
            || !self.landed_t.is_finite()
            || self.landed_t < 0.0
        {
            return Err(FisheryError::InvalidStockState);
        }
        Ok(())
    }

    /// The stock's surplus production at its present biomass (t a year).
    ///
    /// Invalid model/state and non-finite arithmetic are explicit errors; zero
    /// is reserved for a valid stock with no surplus.
    pub fn surplus_t_per_year(&self, model: &FisheryModel) -> Result<f64, FisheryError> {
        if !model.is_valid() {
            return Err(FisheryError::InvalidModel);
        }
        self.validate_state()?;

        let p = self.biomass_t / self.k_t;
        let surplus = model.r(self.resilience) * (1.0 - p) * self.biomass_t;
        let surplus = if p < model.depensation_below {
            surplus * p / model.depensation_below
        } else {
            surplus
        };
        if !surplus.is_finite() || surplus < 0.0 {
            return Err(FisheryError::NonFiniteGrowth);
        }
        Ok(surplus)
    }

    /// The most the stock can give year on year (t), `r k / 4`.
    ///
    /// Divide carrying capacity before multiplying so a representable result
    /// does not overflow in the intermediate `r * k`.
    pub fn msy_t(&self, model: &FisheryModel) -> Result<f64, FisheryError> {
        if !model.is_valid() {
            return Err(FisheryError::InvalidModel);
        }
        self.validate_state()?;

        let msy = model.r(self.resilience) * (self.k_t / 4.0);
        if !msy.is_finite() || msy < 0.0 {
            return Err(FisheryError::NonFiniteGrowth);
        }
        Ok(msy)
    }

    /// Step `dt_s`, fishing for `wanted_t` over the step.
    ///
    /// The catch is capped at the stock's surplus over the step, so the fishery
    /// never takes more than grows. Invalid inputs and arithmetic failures are
    /// returned explicitly and leave the stock unchanged.
    pub fn fish(
        &mut self,
        model: &FisheryModel,
        wanted_t: f64,
        dt_s: f64,
    ) -> Result<f64, FisheryError> {
        if !model.is_valid() {
            return Err(FisheryError::InvalidModel);
        }
        if !wanted_t.is_finite() || wanted_t < 0.0 {
            return Err(FisheryError::InvalidCatchRequest);
        }
        if !dt_s.is_finite() || dt_s < 0.0 {
            return Err(FisheryError::InvalidTimeStep);
        }
        self.validate_state()?;

        let raw_growth = self.surplus_t_per_year(model)? * dt_s / YEAR_S;
        if !raw_growth.is_finite() || raw_growth < 0.0 {
            return Err(FisheryError::NonFiniteGrowth);
        }
        // A single aggregate step cannot add more biomass than the stock's
        // remaining headroom to carrying capacity. The instantaneous Schaefer
        // rate is an Euler estimate; cap it so a coarse dt cannot create fish.
        let grown = raw_growth.min(self.k_t - self.biomass_t);
        let post_growth = self.biomass_t + grown;
        if !post_growth.is_finite() {
            return Err(FisheryError::NonFiniteGrowth);
        }

        let catch = wanted_t.min(grown);
        let biomass_t = (post_growth - catch).min(self.k_t);
        let landed_t = self.landed_t + catch;
        if !biomass_t.is_finite() {
            return Err(FisheryError::NonFiniteGrowth);
        }
        if !landed_t.is_finite() {
            return Err(FisheryError::LandedTotalOverflow);
        }

        self.biomass_t = biomass_t;
        self.landed_t = landed_t;
        Ok(catch)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fish_stock_snapshots_round_trip_without_changing_json_or_binary_layout() {
        let mut stock = FishStock::unexploited(Resilience::Medium, 1_000.0)
            .expect("positive carrying capacity");
        stock.biomass_t = 420.0;
        stock.landed_t = 17.0;

        let json = serde_json::to_value(&stock).expect("stock serializes");
        assert_eq!(
            json,
            serde_json::json!({
                "resilience": "Medium",
                "k_t": 1_000.0,
                "biomass_t": 420.0,
                "landed_t": 17.0
            })
        );
        let restored: FishStock =
            serde_json::from_value(json).expect("valid JSON stock restores");
        assert_eq!(restored, stock);

        let binary = bincode::serialize(&stock).expect("stock serializes to bincode");
        let restored: FishStock =
            bincode::deserialize(&binary).expect("valid binary stock restores");
        assert_eq!(restored, stock);
    }

    #[test]
    fn fish_stock_deserialization_rejects_invalid_physical_state() {
        for value in [
            serde_json::json!({
                "resilience": "Medium",
                "k_t": 0.0,
                "biomass_t": 0.0,
                "landed_t": 0.0
            }),
            serde_json::json!({
                "resilience": "Medium",
                "k_t": -1.0,
                "biomass_t": 0.0,
                "landed_t": 0.0
            }),
            serde_json::json!({
                "resilience": "Medium",
                "k_t": 100.0,
                "biomass_t": 101.0,
                "landed_t": 0.0
            }),
            serde_json::json!({
                "resilience": "Medium",
                "k_t": 100.0,
                "biomass_t": -1.0,
                "landed_t": 0.0
            }),
            serde_json::json!({
                "resilience": "Medium",
                "k_t": 100.0,
                "biomass_t": 50.0,
                "landed_t": -1.0
            }),
        ] {
            assert!(
                serde_json::from_value::<FishStock>(value).is_err(),
                "invalid stock snapshot must fail to deserialize"
            );
        }
    }

    #[test]
    fn invalid_carrying_capacity_is_rejected_at_stock_construction() {
        for k_t in [0.0, -1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert_eq!(
                FishStock::unexploited(Resilience::Medium, k_t),
                Err(FisheryError::InvalidCarryingCapacity)
            );
        }
    }

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
        let mut stock = FishStock::unexploited(Resilience::Medium, 1_000.0)
            .expect("positive carrying capacity");
        assert_eq!(stock.surplus_t_per_year(&m).unwrap(), 0.0);
        stock.biomass_t = 500.0;
        assert!((stock.surplus_t_per_year(&m).unwrap() - stock.msy_t(&m).unwrap()).abs() < 1e-9);
        stock.biomass_t = 100.0;
        // 0.5 × 0.9 × 100 = 45, then × 4 × 0.1 for depensation.
        assert!((stock.surplus_t_per_year(&m).unwrap() - 18.0).abs() < 1e-9);
    }

    #[test]
    fn diagnostic_methods_reject_invalid_models_and_stock_state() {
        let m = FisheryModel::load_default().expect("the pack loads");
        let stock = FishStock::unexploited(Resilience::Medium, 1_000.0)
            .expect("positive carrying capacity");

        let mut invalid_model = m.clone();
        invalid_model.r_per_year[Resilience::Medium as usize] = f64::NAN;
        assert_eq!(
            stock.surplus_t_per_year(&invalid_model),
            Err(FisheryError::InvalidModel)
        );
        assert_eq!(stock.msy_t(&invalid_model), Err(FisheryError::InvalidModel));

        let mut invalid_capacity = stock.clone();
        invalid_capacity.k_t = f64::INFINITY;
        assert_eq!(
            invalid_capacity.surplus_t_per_year(&m),
            Err(FisheryError::InvalidCarryingCapacity)
        );
        assert_eq!(
            invalid_capacity.msy_t(&m),
            Err(FisheryError::InvalidCarryingCapacity)
        );

        let mut invalid_biomass = stock;
        invalid_biomass.biomass_t = f64::NAN;
        assert_eq!(
            invalid_biomass.surplus_t_per_year(&m),
            Err(FisheryError::InvalidStockState)
        );
        assert_eq!(
            invalid_biomass.msy_t(&m),
            Err(FisheryError::InvalidStockState)
        );
    }

    #[test]
    fn msy_avoids_intermediate_overflow_when_the_result_is_finite() {
        let mut model = FisheryModel::load_default().expect("the pack loads");
        model.r_per_year[Resilience::Medium as usize] = 4.0;
        let stock = FishStock::unexploited(Resilience::Medium, f64::MAX * 0.5)
            .expect("finite positive carrying capacity");

        assert_eq!(stock.msy_t(&model), Ok(f64::MAX * 0.5));
    }

    #[test]
    fn a_fishery_cannot_take_more_than_the_stock_grows() {
        let m = FisheryModel::load_default().expect("the pack loads");
        let mut stock = FishStock::unexploited(Resilience::Medium, 1_000.0)
            .expect("positive carrying capacity");
        stock.biomass_t = 500.0;
        let day = 86_400.0;
        for _ in 0..3_653 {
            let before = stock.biomass_t;
            let landed = stock.fish(&m, 10.0, day).expect("valid fishing step");
            assert!(stock.biomass_t >= before - 1e-9, "the stock never shrinks");
            assert!(landed <= stock.msy_t(&m).unwrap() * day / YEAR_S + 1e-9);
        }
        assert!(stock.landed_t > 0.0);
        assert!(stock.landed_t <= stock.msy_t(&m).unwrap() * 3_653.0 * day / YEAR_S + 1e-6);
    }

    #[test]
    fn invalid_fishing_inputs_return_typed_errors_without_mutating_the_stock() {
        let m = FisheryModel::load_default().expect("the pack loads");
        let mut stock = FishStock::unexploited(Resilience::Medium, 1_000.0)
            .expect("positive carrying capacity");
        stock.biomass_t = 500.0;
        stock.landed_t = 25.0;
        let before = stock.clone();

        for wanted in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -1.0] {
            assert_eq!(
                stock.fish(&m, wanted, YEAR_S),
                Err(FisheryError::InvalidCatchRequest)
            );
            assert_eq!(stock, before, "invalid catch request mutated the stock");
        }

        for dt in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -1.0] {
            assert_eq!(stock.fish(&m, 1.0, dt), Err(FisheryError::InvalidTimeStep));
            assert_eq!(stock, before, "invalid time step mutated the stock");
        }
    }

    #[test]
    fn invalid_model_and_stock_state_are_rejected_without_mutation() {
        let m = FisheryModel::load_default().expect("the pack loads");
        let mut stock = FishStock::unexploited(Resilience::Medium, 1_000.0)
            .expect("positive carrying capacity");
        stock.biomass_t = 500.0;
        let before = stock.clone();

        let mut invalid_model = m.clone();
        invalid_model.r_per_year[Resilience::Medium as usize] = f64::NAN;
        assert!(!invalid_model.is_valid());
        assert_eq!(
            stock.fish(&invalid_model, 10.0, YEAR_S),
            Err(FisheryError::InvalidModel)
        );
        assert_eq!(stock, before);

        invalid_model = m.clone();
        invalid_model.depensation_below = 1.01;
        assert!(!invalid_model.is_valid());
        assert_eq!(
            stock.fish(&invalid_model, 10.0, YEAR_S),
            Err(FisheryError::InvalidModel)
        );
        assert_eq!(stock, before);

        let mut invalid_stock = before.clone();
        invalid_stock.biomass_t = invalid_stock.k_t + 1.0;
        let invalid_before = invalid_stock.clone();
        assert_eq!(
            invalid_stock.fish(&m, 10.0, YEAR_S),
            Err(FisheryError::InvalidStockState)
        );
        assert_eq!(invalid_stock, invalid_before);
    }

    #[test]
    fn overflowing_growth_is_rejected_without_mutating_the_stock() {
        let m = FisheryModel::load_default().expect("the pack loads");
        let mut explosive = m.clone();
        explosive.r_per_year[Resilience::Medium as usize] = f64::MAX;
        let mut stock = FishStock::unexploited(Resilience::Medium, 1.0e308)
            .expect("positive carrying capacity");
        stock.biomass_t = 5.0e307;
        let before = stock.clone();

        assert_eq!(
            stock.fish(&explosive, 1.0, YEAR_S),
            Err(FisheryError::NonFiniteGrowth)
        );
        assert_eq!(stock, before);
    }

    #[test]
    fn landed_total_overflow_returns_an_error_without_mutating_the_stock() {
        let m = FisheryModel::load_default().expect("the pack loads");
        let mut stock = FishStock::unexploited(Resilience::Medium, 1.0e308)
            .expect("positive carrying capacity");
        stock.biomass_t = 5.0e307;
        stock.landed_t = f64::MAX * 0.99;
        let before = stock.clone();

        assert_eq!(
            stock.fish(&m, 1.0e307, YEAR_S),
            Err(FisheryError::LandedTotalOverflow)
        );
        assert_eq!(stock, before);
    }

    #[test]
    fn coarse_fishing_interval_cannot_harvest_more_than_remaining_capacity() {
        let m = FisheryModel::load_default().expect("the pack loads");
        let mut stock = FishStock::unexploited(Resilience::Medium, 1_000.0)
            .expect("positive carrying capacity");
        stock.biomass_t = 500.0;

        let landed = stock
            .fish(&m, 10_000.0, 100.0 * YEAR_S)
            .expect("valid coarse fishing step");

        assert_eq!(landed, 500.0);
        assert_eq!(stock.biomass_t, 500.0);
        assert_eq!(stock.landed_t, 500.0);
        assert!(stock.biomass_t <= stock.k_t);
    }
}
