//! A fishery on one stock (Phase 4b Task 7b, food chain).
//!
//! The stock grows by the Schaefer surplus-production model as Froese et
//! al. (2017) write it: next year's biomass is this year's plus
//! `r (1 - B/k) B` less the catch, and below a quarter of `k` the surplus
//! falls off linearly (`× 4 B/k`), as recruitment fails. `r` is the middle
//! of the paper's prior range for the species' FishBase resilience class;
//! the maximum is the analytic piecewise MSY; with the default depensation threshold (0.25), it is `r k / 4` at `B = k/2`.
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

fn msy_peak_fractions(depensation_below: f64) -> (f64, f64) {
    if depensation_below <= 0.5 {
        (0.5, 0.25)
    } else if depensation_below <= 2.0 / 3.0 {
        (
            depensation_below,
            depensation_below * (1.0 - depensation_below),
        )
    } else {
        (2.0 / 3.0, 4.0 / (27.0 * depensation_below))
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
        let rate = model.r(self.resilience);
        // Apply the low-biomass depensation factor before multiplying by the
        // growth rate. The old order could overflow at rate * biomass even
        // when the final depensated surplus was representable.
        let surplus = if p < model.depensation_below {
            if p == 0.0 && self.biomass_t > 0.0 && rate > 0.0 {
                // B / K can underflow to zero even when r * B² / (K * d)
                // is representable. In that rare case evaluate the equivalent
                // positive product in log space rather than silently reporting
                // zero surplus for a non-empty stock.
                let log_surplus = rate.log2() + 2.0 * self.biomass_t.log2()
                    - self.k_t.log2()
                    - model.depensation_below.log2();
                log_surplus.exp2()
            } else {
                let biomass_factor = (1.0 - p) * (p / model.depensation_below);
                rate * (self.biomass_t * biomass_factor)
            }
        } else {
            rate * (1.0 - p) * self.biomass_t
        };
        if !surplus.is_finite() || surplus < 0.0 {
            return Err(FisheryError::NonFiniteGrowth);
        }
        Ok(surplus)
    }

    /// The maximum annual surplus production (t/year) over every valid biomass.
    ///
    /// With a depensation threshold at or below half of carrying capacity, the
    /// usual Schaefer peak remains at B = K/2 and equals rK/4. Above half K,
    /// that peak lies inside the depensated region: the maximum moves to B = dK
    /// through d = 2/3, then to B = 2K/3 for larger thresholds.
    ///
    /// Scale K by the bounded peak fraction before multiplying by r so a
    /// representable result does not overflow in an intermediate r * K product.
    pub fn msy_t(&self, model: &FisheryModel) -> Result<f64, FisheryError> {
        if !model.is_valid() {
            return Err(FisheryError::InvalidModel);
        }
        self.validate_state()?;

        let (_, peak_surplus_fraction) = msy_peak_fractions(model.depensation_below);
        let msy = model.r(self.resilience) * (self.k_t * peak_surplus_fraction);
        if !msy.is_finite() || msy < 0.0 {
            return Err(FisheryError::NonFiniteGrowth);
        }
        Ok(msy)
    }

    /// Biomass at which the stock produces its maximum sustainable surplus (tonnes).
    ///
    /// This is a diagnostic target for a future harvest decision layer. It does not
    /// change the stock or imply that a real-world species has been calibrated.
    pub fn msy_biomass_t(&self, model: &FisheryModel) -> Result<f64, FisheryError> {
        if !model.is_valid() {
            return Err(FisheryError::InvalidModel);
        }
        self.validate_state()?;

        let (peak_biomass_fraction, _) = msy_peak_fractions(model.depensation_below);
        let biomass = self.k_t * peak_biomass_fraction;
        if !biomass.is_finite() || biomass < 0.0 {
            return Err(FisheryError::NonFiniteGrowth);
        }
        Ok(biomass)
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

        let surplus = self.surplus_t_per_year(model)?;
        // Divide whichever factor is at least one year before multiplying.
        // This avoids overflow for a large annual surplus and underflow when a
        // tiny time step is paired with a large, otherwise representable rate.
        let raw_growth = if surplus >= YEAR_S {
            (surplus / YEAR_S) * dt_s
        } else if dt_s >= YEAR_S {
            surplus * (dt_s / YEAR_S)
        } else {
            (surplus * dt_s) / YEAR_S
        };
        if raw_growth.is_nan() || raw_growth < 0.0 {
            return Err(FisheryError::NonFiniteGrowth);
        }
        // A single aggregate step cannot add more biomass than the stock's
        // remaining headroom to carrying capacity. The instantaneous Schaefer
        // rate is an Euler estimate; cap it so a coarse dt cannot create fish.
        // Positive infinity means the unconstrained interval growth exceeds f64::MAX.
        // The stock's remaining headroom is finite, so the physically valid result
        // is still representable: cap at carrying capacity instead of rejecting the step.
        let headroom = self.k_t - self.biomass_t;
        let grown = if raw_growth.is_infinite() {
            headroom
        } else {
            raw_growth.min(headroom)
        };
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
        let restored: FishStock = serde_json::from_value(json).expect("valid JSON stock restores");
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

        // Binary snapshots can encode non-finite floats even though JSON cannot.
        let invalid = FishStock {
            resilience: Resilience::Medium,
            k_t: f64::INFINITY,
            biomass_t: 0.0,
            landed_t: 0.0,
        };
        let binary = bincode::serialize(&invalid).expect("invalid fixture serializes");
        assert!(
            bincode::deserialize::<FishStock>(&binary).is_err(),
            "non-finite binary stock snapshot must fail to deserialize"
        );
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
        assert_eq!(
            stock.msy_biomass_t(&invalid_model),
            Err(FisheryError::InvalidModel)
        );

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
        assert_eq!(
            invalid_capacity.msy_biomass_t(&m),
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
        assert_eq!(
            invalid_biomass.msy_biomass_t(&m),
            Err(FisheryError::InvalidStockState)
        );
    }

    #[test]
    fn depensation_is_applied_before_large_growth_products_can_overflow() {
        let mut model = FisheryModel::load_default().expect("the pack loads");
        model.r_per_year[Resilience::Medium as usize] = 1.0e154;
        model.depensation_below = 0.25;

        let mut stock = FishStock::unexploited(Resilience::Medium, 1.0e157)
            .expect("finite positive carrying capacity");
        stock.biomass_t = 1.0e155;

        // The unscaled intermediate r * (1 - B/k) * B exceeds f64::MAX,
        // but the low-biomass depensation factor makes the final surplus
        // approximately 3.96e307, which is representable.
        let surplus = stock
            .surplus_t_per_year(&model)
            .expect("finite depensated surplus");

        assert!(surplus.is_finite());
        assert!((surplus / 3.96e307 - 1.0).abs() < 1e-12);
    }

    #[test]
    fn depensation_preserves_representable_surplus_and_catch_when_biomass_ratio_underflows() {
        let mut model = FisheryModel::load_default().expect("the pack loads");
        model.r_per_year[Resilience::Medium as usize] = 1.0e308;
        model.depensation_below = 0.25;

        let mut stock = FishStock::unexploited(Resilience::Medium, 1.0e308)
            .expect("positive carrying capacity");
        stock.biomass_t = 1.0e-100;

        // B / K underflows to zero, but the mathematically equivalent
        // r * B² / (K * d) remains a finite, positive value.
        assert_eq!(stock.biomass_t / stock.k_t, 0.0);
        let surplus = stock
            .surplus_t_per_year(&model)
            .expect("representable low-biomass surplus");

        assert!(surplus.is_finite() && surplus > 0.0);
        assert!((surplus / 4.0e-200 - 1.0).abs() < 1.0e-12);

        let landed = stock
            .fish(&model, 1.0e-200, YEAR_S)
            .expect("the fishery can land a positive request from representable growth");
        assert_eq!(landed, 1.0e-200);
        assert_eq!(stock.landed_t, 1.0e-200);
    }

    #[test]
    fn msy_avoids_intermediate_overflow_when_the_result_is_finite() {
        let mut model = FisheryModel::load_default().expect("the pack loads");
        model.r_per_year[Resilience::Medium as usize] = 4.0;
        let stock = FishStock::unexploited(Resilience::Medium, f64::MAX * 0.5)
            .expect("finite positive carrying capacity");

        assert_eq!(stock.msy_t(&model), Ok(f64::MAX * 0.5));
        assert_eq!(stock.msy_biomass_t(&model), Ok(f64::MAX * 0.5));
    }

    #[test]
    fn msy_tracks_the_piecewise_peak_for_every_allowed_depensation_threshold() {
        let mut model = FisheryModel::load_default().expect("the pack loads");
        model.r_per_year[Resilience::Medium as usize] = 1.0;
        let k_t = 1_200.0;

        for (threshold, peak_biomass_fraction, expected_peak_fraction) in [
            (0.25, 0.5, 0.25),
            (0.5, 0.5, 0.25),
            (0.6, 0.6, 0.24),
            (2.0 / 3.0, 2.0 / 3.0, 2.0 / 9.0),
            (0.75, 2.0 / 3.0, 4.0 / (27.0 * 0.75)),
            (1.0, 2.0 / 3.0, 4.0 / 27.0),
        ] {
            model.depensation_below = threshold;
            let mut stock = FishStock::unexploited(Resilience::Medium, k_t)
                .expect("positive carrying capacity");
            stock.biomass_t = k_t * peak_biomass_fraction;

            let msy = stock.msy_t(&model).expect("valid model has finite MSY");
            let target_biomass = stock
                .msy_biomass_t(&model)
                .expect("valid model has a finite peak biomass target");
            let surplus = stock
                .surplus_t_per_year(&model)
                .expect("the analytic peak has finite surplus");
            let expected_target_biomass = k_t * peak_biomass_fraction;
            let expected = k_t * expected_peak_fraction;
            let tolerance = expected.max(1.0) * 1e-12;

            assert!(
                (target_biomass - expected_target_biomass).abs() <= tolerance,
                "threshold {threshold}: target biomass {target_biomass} vs {expected_target_biomass}"
            );
            assert!(
                (msy - expected).abs() <= tolerance,
                "threshold {threshold}: {msy} vs {expected}"
            );
            assert!(
                (surplus - msy).abs() <= tolerance,
                "threshold {threshold}: peak surplus {surplus} vs MSY {msy}"
            );

            // The reported MSY must bound the complete piecewise curve, not
            // merely the stock state at the analytic peak.
            for step in 0..=1_000 {
                stock.biomass_t = k_t * (step as f64 / 1_000.0);
                let candidate = stock
                    .surplus_t_per_year(&model)
                    .expect("sampled biomass has finite surplus");
                assert!(
                    candidate <= msy + tolerance,
                    "threshold {threshold}, biomass fraction {}: {candidate} exceeds {msy}",
                    step as f64 / 1_000.0
                );
            }
        }
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
    fn unrepresentable_annual_surplus_is_rejected_without_mutating_the_stock() {
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
    fn interval_growth_above_f64_range_caps_at_finite_stock_headroom() {
        let m = FisheryModel::load_default().expect("the pack loads");
        let initial = f64::MAX / 2.0;
        let headroom = f64::MAX - initial;
        let mut harvested = FishStock {
            resilience: Resilience::High,
            k_t: f64::MAX,
            biomass_t: initial,
            landed_t: 0.0,
        };

        let landed = harvested
            .fish(&m, f64::MAX, f64::MAX)
            .expect("overflowing unconstrained growth is capped at finite headroom");

        assert_eq!(landed, headroom);
        assert_eq!(harvested.biomass_t, initial);
        assert_eq!(harvested.landed_t, headroom);

        let mut unharvested = FishStock {
            resilience: Resilience::High,
            k_t: f64::MAX,
            biomass_t: initial,
            landed_t: 0.0,
        };
        assert_eq!(
            unharvested.fish(&m, 0.0, f64::MAX),
            Ok(0.0),
            "growth above the float range still fills the finite stock capacity"
        );
        assert_eq!(unharvested.biomass_t, f64::MAX);
        assert_eq!(unharvested.landed_t, 0.0);
    }

    #[test]
    fn large_representable_annual_growth_avoids_intermediate_overflow() {
        let m = FisheryModel::load_default().expect("the pack loads");
        let mut stock = FishStock::unexploited(Resilience::Medium, 1.0e308)
            .expect("positive carrying capacity");
        stock.biomass_t = 5.0e307;

        let landed = stock
            .fish(&m, 1.0e307, YEAR_S)
            .expect("representable one-year growth");

        assert!((landed / 1.0e307 - 1.0).abs() < 1e-12);
        assert!((stock.biomass_t / 1.0e307 - 5.25).abs() < 1e-12);
        assert!((stock.landed_t / 1.0e307 - 1.0).abs() < 1e-12);
    }

    #[test]
    fn tiny_time_step_preserves_representable_growth_for_a_large_stock() {
        let m = FisheryModel::load_default().expect("the pack loads");
        let mut stock = FishStock::unexploited(Resilience::Medium, 1.0e308)
            .expect("positive carrying capacity");
        stock.biomass_t = 5.0e307;

        let landed = stock
            .fish(&m, 1.0e-23, f64::from_bits(1))
            .expect("representable subnormal time step");

        assert!(landed > 0.0);
        assert!(stock.landed_t > 0.0);
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
