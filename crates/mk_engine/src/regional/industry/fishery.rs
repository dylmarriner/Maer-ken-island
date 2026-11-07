//! A fishery on one stock (Phase 4b Task 7b, food chain).
//!
//! The stock grows by the Schaefer surplus-production model as Froese et
//! al. (2017) write it: next year's biomass is this year's plus
//! `r (1 - B/k) B` less the catch. Below the configured depensation threshold
//! `d k`, surplus is scaled by `B/(d k)` as recruitment fails. `r` is the middle
//! of the paper's prior range for the species' FishBase resilience class;
//! the maximum is the analytic piecewise MSY; with the default depensation threshold (0.25), it is `r k / 4` at `B = k/2`.
//!
//! The fishery never takes more than the stock grows: its catch is capped
//! at the surplus of the moment, so harvest never exceeds regrowth.
//! Unexploited biomass `k` comes from the caller. The island's species
//! keep upstream's uncalibrated densities (D55), so a stock built from
//! them is only as good as those densities, and resilience is a class the
//! caller supplies, since no species carries FishBase's.

use serde::ser::SerializeStruct;
use serde::{Deserialize, Serialize};

use super::petroleum::MissingReference;
use crate::validation::{ReferenceDomain, ReferenceLibrary};

const YEAR_S: f64 = 365.25 * 86_400.0;

// Repeated binary64 additions can move a cumulative total by a few ULPs.
// Bound the recorded-delta error relative to the catch, rather than demanding
// bit equality (which rejects ordinary repeated catches), while still refusing
// materially distorted increments such as 2^53 + 3 rounding to 4.
const LANDED_TOTAL_RELATIVE_TOLERANCE: f64 = 1.0e-12;

/// Invalid fishery inputs are explicit errors; a failed step never mutates its stock.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FisheryError {
    InvalidModel,
    InvalidCarryingCapacity,
    InvalidCatchRequest,
    InvalidTimeStep,
    InvalidStockState,
    NonFiniteGrowth,
    UnrepresentableGrowth,
    LandedTotalOverflow,
    LandedTotalUnrepresentable,
    BiomassRemainderUnrepresentable,
    /// Positive mathematical maximum sustainable yield rounded below binary64 range.
    UnrepresentableMsy,
    /// Positive analytic peak-biomass target rounded below binary64 range.
    UnrepresentableMsyBiomassTarget,
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
            Self::UnrepresentableGrowth => {
                "positive fish stock growth is below binary64 representable range"
            }
            Self::LandedTotalOverflow => "cumulative landed biomass overflowed",
            Self::LandedTotalUnrepresentable => {
                "cumulative landed biomass increment is not representable at stored precision"
            }
            Self::BiomassRemainderUnrepresentable => {
                "remaining biomass growth is not representable at stored precision"
            }
            Self::UnrepresentableMsy => {
                "positive maximum sustainable yield is below binary64 representable range"
            }
            Self::UnrepresentableMsyBiomassTarget => {
                "positive maximum sustainable yield biomass target is below binary64 representable range"
            }
        };
        f.write_str(message)
    }
}

impl std::error::Error for FisheryError {}

/// Midpoint of a published growth-rate prior after validating both source bounds.
/// Sum before halving when bounded; otherwise halve each endpoint to avoid overflow
/// and the double-rounding error from first rounding a wide endpoint difference.
fn checked_rate_midpoint(min: f64, max: f64) -> Option<f64> {
    if !min.is_finite() || !max.is_finite() || min < 0.0 || max < min {
        return None;
    }
    let half_max = f64::MAX * 0.5;
    let midpoint = if min <= half_max && max <= half_max {
        (min + max) * 0.5
    } else {
        min * 0.5 + max * 0.5
    };
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
#[derive(Debug, Clone, PartialEq)]
pub struct FishStock {
    pub resilience: Resilience,
    /// Unexploited biomass (t).
    pub k_t: f64,
    pub biomass_t: f64,
    pub landed_t: f64,
}

/// Persist only physically valid stock states. Keep field order identical to the
/// previous derived serializer so valid JSON and bincode snapshots retain their layout.
impl Serialize for FishStock {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.validate_state().map_err(serde::ser::Error::custom)?;

        let mut state = serializer.serialize_struct("FishStock", 4)?;
        state.serialize_field("resilience", &self.resilience)?;
        state.serialize_field("k_t", &self.k_t)?;
        state.serialize_field("biomass_t", &self.biomass_t)?;
        state.serialize_field("landed_t", &self.landed_t)?;
        state.end()
    }
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
            let biomass_factor = (1.0 - p) * (p / model.depensation_below);
            let biomass_product = self.biomass_t * biomass_factor;
            let direct = rate * biomass_product;
            let subnormal_intermediate = p < f64::MIN_POSITIVE
                || biomass_factor < f64::MIN_POSITIVE
                || biomass_product < f64::MIN_POSITIVE;
            if (direct == 0.0 || subnormal_intermediate) && self.biomass_t > 0.0 && rate > 0.0 {
                // A subnormal ratio, depensation factor or biomass product has
                // too little precision to multiply safely: rounding that
                // intermediate can materially distort a still-normal final
                // surplus. Re-evaluate the original B, K and d inputs in log
                // space instead of silently losing recruitment or precision.
                let log_surplus = rate.log2() + 2.0 * self.biomass_t.log2()
                    - self.k_t.log2()
                    - model.depensation_below.log2()
                    + (1.0 - p).log2();
                log_surplus.exp2()
            } else {
                direct
            }
        } else {
            let remaining_fraction = 1.0 - p;
            let growth_factor = rate * remaining_fraction;
            let direct = growth_factor * self.biomass_t;
            if rate > 0.0
                && self.biomass_t > 0.0
                && p < 1.0
                && (growth_factor < f64::MIN_POSITIVE || direct == 0.0)
            {
                // A subnormal rate/fraction product can round away before a
                // large biomass makes the final surplus representable.
                let log_surplus = rate.log2() + remaining_fraction.log2() + self.biomass_t.log2();
                log_surplus.exp2()
            } else {
                direct
            }
        };
        if surplus == 0.0 && rate > 0.0 && self.biomass_t > 0.0 && self.biomass_t < self.k_t {
            // Zero is reserved for an actual zero-rate/zero-biomass/at-capacity
            // state. A positive mathematical result below the smallest
            // subnormal cannot be persisted faithfully and must fail closed.
            return Err(FisheryError::UnrepresentableGrowth);
        }
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
        let rate = model.r(self.resilience);
        let scaled_capacity = self.k_t * peak_surplus_fraction;
        let mut msy = if scaled_capacity < f64::MIN_POSITIVE && rate > 0.0 {
            // A quarter is an exact binary scale. Apply it to the rate before
            // the subnormal capacity so the final multiplication performs the
            // only rounding; scaling K first can discard a representable result.
            // Other peak fractions retain rate * K first because neither order
            // is universally exact for their non-power-of-two significands.
            if peak_surplus_fraction == 0.25 {
                (rate * peak_surplus_fraction) * self.k_t
            } else {
                (rate * self.k_t) * peak_surplus_fraction
            }
        } else {
            rate * scaled_capacity
        };
        if msy == 0.0 && rate > 0.0 {
            // A non-quarter fraction can make the first product order round to
            // zero even when the exact three-factor result rounds to one
            // subnormal unit. Reorder the factors before declaring it
            // unrepresentable; the zero result is valid only if both orders
            // round to zero.
            msy = (rate * peak_surplus_fraction) * self.k_t;
            if msy == 0.0 {
                return Err(FisheryError::UnrepresentableMsy);
            }
        }
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
        if biomass == 0.0 && peak_biomass_fraction > 0.0 {
            return Err(FisheryError::UnrepresentableMsyBiomassTarget);
        }
        if !biomass.is_finite() || biomass < 0.0 {
            return Err(FisheryError::NonFiniteGrowth);
        }
        Ok(biomass)
    }

    /// Read-only upper bound on catch for one interval (tonnes).
    ///
    /// This uses the same single-step surplus estimate and carrying-capacity
    /// cap as fish(). It is a planning diagnostic, not a quota, stock
    /// assessment or forecast of future recruitment. It does not mutate the
    /// stock, landed total or any simulation state.
    pub fn sustainable_catch_limit_t(
        &self,
        model: &FisheryModel,
        dt_s: f64,
    ) -> Result<f64, FisheryError> {
        if !model.is_valid() {
            return Err(FisheryError::InvalidModel);
        }
        self.growth_capacity_t(model, dt_s)
    }

    /// The maximum interval growth the stock can add without exceeding K.
    /// Shared by the read-only diagnostic and the mutating fish operation so
    /// the displayed limit cannot drift from the enforced catch cap.
    fn growth_capacity_t(&self, model: &FisheryModel, dt_s: f64) -> Result<f64, FisheryError> {
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

        // The instantaneous Schaefer rate is an Euler estimate. Cap a coarse
        // interval at the finite remaining headroom rather than creating fish
        // beyond carrying capacity. Positive infinity is therefore bounded.
        let headroom = self.k_t - self.biomass_t;
        if raw_growth == 0.0 && surplus > 0.0 && dt_s > 0.0 && headroom > 0.0 {
            // Positive annual production over a positive interval is still
            // mathematically positive. If its interval amount rounds to zero,
            // do not silently erase it from the material ledger.
            return Err(FisheryError::UnrepresentableGrowth);
        }
        let grown = if raw_growth.is_infinite() {
            headroom
        } else {
            raw_growth.min(headroom)
        };
        if !grown.is_finite() || grown < 0.0 {
            return Err(FisheryError::NonFiniteGrowth);
        }
        Ok(grown)
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

        let grown = self.growth_capacity_t(model, dt_s)?;
        let post_growth = self.biomass_t + grown;
        if !post_growth.is_finite() {
            return Err(FisheryError::NonFiniteGrowth);
        }

        let catch = wanted_t.min(grown);
        // Subtract the catch from growth before adding the remainder to biomass.
        // (biomass + grown) - catch can lose low bits to cancellation and make a
        // fully harvested stock appear to shrink even though catch == growth.
        let remaining_growth = grown - catch;
        let biomass_t = (self.biomass_t + remaining_growth).min(self.k_t);
        let landed_t = self.landed_t + catch;
        if !biomass_t.is_finite() {
            return Err(FisheryError::NonFiniteGrowth);
        }
        if !landed_t.is_finite() {
            return Err(FisheryError::LandedTotalOverflow);
        }
        // The cumulative total may round by a few bits after repeated ordinary
        // catches. Accept only a tightly bounded relative error, and never accept
        // a positive catch that disappears or is materially rounded up/down.
        let recorded_delta = landed_t - self.landed_t;
        if catch > 0.0
            && (recorded_delta <= 0.0
                || (recorded_delta - catch).abs() > catch * LANDED_TOTAL_RELATIVE_TOLERANCE)
        {
            return Err(FisheryError::LandedTotalUnrepresentable);
        }
        // Every unharvested increment must be represented in the stored stock,
        // including a zero-catch step. Otherwise a valid positive growth event
        // disappears from the material ledger merely because it is below one ULP
        // at the current biomass magnitude.
        if remaining_growth > 0.0 && biomass_t == self.biomass_t {
            return Err(FisheryError::BiomassRemainderUnrepresentable);
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
    fn fish_stock_bincode_snapshot_preserves_the_legacy_derived_field_layout() {
        let mut stock = FishStock::unexploited(Resilience::Medium, 1_000.0)
            .expect("positive carrying capacity");
        stock.biomass_t = 420.0;
        stock.landed_t = 17.0;

        // bincode 1.3's legacy fixed-int, little-endian layout: the u32 enum
        // discriminant followed by k_t, biomass_t and landed_t as f64 values.
        // This is the byte layout emitted by the former derived Serialize impl.
        let expected: [u8; 28] = [
            1, 0, 0, 0, 0, 0, 0, 0, 0, 64, 143, 64, 0, 0, 0, 0, 0, 64, 122, 64, 0, 0, 0, 0, 0, 0,
            49, 64,
        ];

        let encoded = bincode::serialize(&stock).expect("valid stock serializes");
        assert_eq!(encoded.as_slice(), expected.as_slice());

        // Read the fixed bytes directly: a current-version round trip alone would
        // not catch a reader that had drifted together with the current writer.
        let restored: FishStock =
            bincode::deserialize(&expected).expect("legacy derived snapshot remains readable");
        assert_eq!(restored, stock);

        let reencoded = bincode::serialize(&restored).expect("legacy snapshot reserializes");
        assert_eq!(reencoded.as_slice(), expected.as_slice());
    }

    #[test]
    fn fish_stock_serialization_rejects_invalid_physical_state() {
        let valid = FishStock::unexploited(Resilience::Medium, 1_000.0)
            .expect("positive carrying capacity");
        let mut invalid_stocks = Vec::new();

        let mut invalid = valid.clone();
        invalid.k_t = 0.0;
        invalid_stocks.push(invalid);

        let mut invalid = valid.clone();
        invalid.k_t = -1.0;
        invalid_stocks.push(invalid);

        let mut invalid = valid.clone();
        invalid.biomass_t = invalid.k_t + 1.0;
        invalid_stocks.push(invalid);

        let mut invalid = valid.clone();
        invalid.biomass_t = -1.0;
        invalid_stocks.push(invalid);

        let mut invalid = valid.clone();
        invalid.landed_t = -1.0;
        invalid_stocks.push(invalid);

        let mut invalid = valid.clone();
        invalid.biomass_t = f64::NAN;
        invalid_stocks.push(invalid);

        let mut invalid = valid;
        invalid.landed_t = f64::INFINITY;
        invalid_stocks.push(invalid);

        for stock in invalid_stocks {
            assert!(
                serde_json::to_vec(&stock).is_err(),
                "invalid physical state must not be written to a JSON snapshot"
            );
            assert!(
                bincode::serialize(&stock).is_err(),
                "invalid physical state must not be written to a binary snapshot"
            );
        }
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

        // Build the invalid wire payload separately: FishStock's public serializer
        // now correctly refuses to emit an invalid physical state.
        #[derive(Serialize)]
        struct InvalidFishStockSnapshotFixture {
            resilience: Resilience,
            k_t: f64,
            biomass_t: f64,
            landed_t: f64,
        }
        let invalid = InvalidFishStockSnapshotFixture {
            resilience: Resilience::Medium,
            k_t: f64::INFINITY,
            biomass_t: 0.0,
            landed_t: 0.0,
        };
        let binary = bincode::serialize(&invalid).expect("invalid wire fixture serializes");
        assert!(
            bincode::deserialize::<FishStock>(&binary).is_err(),
            "non-finite binary stock snapshot must fail to deserialize"
        );
    }

    #[test]
    fn fish_landed_total_overflow_leaves_the_single_writer_state_unchanged() {
        let mut model = FisheryModel::load_default().expect("the pack loads");
        model.r_per_year[Resilience::Medium as usize] = 0.5;
        model.depensation_below = 0.25;

        let mut stock = FishStock::unexploited(Resilience::Medium, 1.0e308)
            .expect("finite positive carrying capacity");
        stock.biomass_t = 5.0e307;
        stock.landed_t = f64::MAX;
        let before = stock.clone();

        assert_eq!(
            stock.fish(&model, f64::MAX, YEAR_S),
            Err(FisheryError::LandedTotalOverflow),
        );
        assert_eq!(
            stock, before,
            "a failed harvest must not commit biomass or landed-total changes"
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
    fn growth_rate_prior_midpoint_preserves_subnormal_ties_and_wide_exponent_gaps() {
        // Half of the exact sum is 1.5 subnormal units and rounds to the even
        // second unit. Halving the difference first underflows that half-unit.
        let adjacent = checked_rate_midpoint(f64::from_bits(1), f64::from_bits(2))
            .expect("finite non-negative bounds");
        assert_eq!(adjacent.to_bits(), 2);

        // A wide exponent gap makes min + (max - min) / 2 round one ULP low.
        let min = 3.0 * 2.0_f64.powi(969);
        let max = 2.0_f64.powi(1023);
        let midpoint = checked_rate_midpoint(min, max).expect("finite midpoint");
        assert_eq!(midpoint.to_bits(), 0x7fd0_0000_0000_0001);
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

        // A partial catch would leave roughly 3e-200 tonnes of growth, which
        // cannot be represented when added to 1e-100 tonnes of existing biomass.
        // Harvest the full representable surplus here; the dedicated partial-
        // harvest regression below proves that an unrecordable remainder is rejected.
        let landed = stock
            .fish(&model, surplus, YEAR_S)
            .expect("the fishery can land the full representable surplus");
        assert_eq!(landed, surplus);
        assert_eq!(stock.biomass_t, 1.0e-100);
        assert_eq!(stock.landed_t, surplus);
    }

    #[test]
    fn positive_but_unrepresentable_msy_diagnostics_fail_closed() {
        let model = FisheryModel::load_default().expect("the pack loads");
        let smallest_positive = f64::from_bits(1);
        let stock = FishStock::unexploited(Resilience::Medium, smallest_positive)
            .expect("positive subnormal carrying capacity");

        assert_eq!(stock.msy_t(&model), Err(FisheryError::UnrepresentableMsy));
        assert_eq!(
            stock.msy_biomass_t(&model),
            Err(FisheryError::UnrepresentableMsyBiomassTarget)
        );

        // A zero growth rate is a genuine zero MSY, not an underflow error.
        let mut no_growth = model.clone();
        no_growth.r_per_year[Resilience::Medium as usize] = 0.0;
        assert_eq!(stock.msy_t(&no_growth), Ok(0.0));

        // The MSY can be unrepresentable even when its biomass target is
        // exactly one subnormal unit and therefore remains representable.
        let two_units = FishStock::unexploited(Resilience::Medium, f64::from_bits(2))
            .expect("positive subnormal carrying capacity");
        assert_eq!(
            two_units.msy_t(&model),
            Err(FisheryError::UnrepresentableMsy)
        );
        assert_eq!(two_units.msy_biomass_t(&model).unwrap().to_bits(), 1);

        // A positive subnormal target that rounds to the nearest non-zero
        // binary64 value remains usable; only a target rounded to zero fails.
        let three_units = FishStock::unexploited(Resilience::Medium, f64::from_bits(3))
            .expect("positive subnormal carrying capacity");
        assert_eq!(three_units.msy_biomass_t(&model).unwrap().to_bits(), 2);

        // The upper depensation branch has a different positive peak fraction;
        // with one subnormal capacity unit, 2/3 rounds to one non-zero unit.
        let mut upper_branch = model.clone();
        upper_branch.depensation_below = 1.0;
        let upper_peak = FishStock::unexploited(Resilience::Medium, smallest_positive)
            .expect("positive subnormal carrying capacity");
        assert_eq!(
            upper_peak.msy_biomass_t(&upper_branch).unwrap().to_bits(),
            1
        );
    }
    #[test]
    fn msy_retries_non_quarter_scaling_when_first_order_rounds_to_zero() {
        let smallest = f64::from_bits(1);

        let model = FisheryModel {
            r_per_year: [0.0, 1.25, 0.0, 0.0],
            depensation_below: 0.51,
        };
        let stock = FishStock::unexploited(Resilience::Medium, f64::from_bits(2))
            .expect("two positive subnormal capacity units");
        // (r * K) rounds to two units, then multiplying by 0.2499 rounds
        // to zero. Scaling r by the peak fraction first yields one unit.
        assert_eq!(stock.msy_t(&model).unwrap().to_bits(), 1);

        let model = FisheryModel {
            r_per_year: [0.0, 2.5, 0.0, 0.0],
            depensation_below: 0.6,
        };
        let stock = FishStock::unexploited(Resilience::Medium, smallest)
            .expect("smallest positive subnormal capacity");
        // The alternate order preserves the exact 0.6-subnormal result.
        assert_eq!(stock.msy_t(&model).unwrap().to_bits(), 1);
    }

    #[test]
    fn msy_preserves_representable_subnormal_results_when_peak_scaling_would_underflow() {
        let model = FisheryModel {
            r_per_year: [0.0, 4.0, 0.0, 0.0],
            depensation_below: 0.25,
        };

        // The exact result is K because r * peak_fraction is 4 * 0.25.
        // Multiplying K by 0.25 first rounds these values down in the
        // subnormal range, then incorrectly amplifies that rounded result.
        for units in [1_u64, 3, 7] {
            let carrying_capacity = f64::from_bits(units);
            assert!(carrying_capacity * 0.25 < f64::MIN_POSITIVE);
            let stock = FishStock::unexploited(Resilience::Medium, carrying_capacity)
                .expect("positive subnormal carrying capacity");

            assert_eq!(
                stock
                    .msy_t(&model)
                    .expect("representable subnormal MSY")
                    .to_bits(),
                units,
                "MSY must preserve {units} units of subnormal carrying capacity"
            );
        }
    }

    #[test]
    fn msy_uses_exact_quarter_scaling_before_subnormal_capacity_rounding() {
        let model = FisheryModel {
            r_per_year: [0.0, f64::from_bits(0x40f5_a159_2be7_b6d7), 0.0, 0.0],
            depensation_below: 0.25,
        };
        let stock = FishStock::unexploited(Resilience::Medium, f64::from_bits(1))
            .expect("smallest positive subnormal carrying capacity");

        // Exact binary scaling of the rate by 1/4 followed by one final
        // multiplication rounds to 22,149 subnormal units. Multiplying K by
        // 1/4 first and then amplifying the rounded intermediate gives 22,150.
        assert_eq!(
            stock.msy_t(&model).expect("finite subnormal MSY").to_bits(),
            22_149
        );
    }

    #[test]
    fn msy_avoids_intermediate_overflow_when_the_result_is_finite() {
        let mut model = FisheryModel::load_default().expect("the pack loads");
        model.r_per_year[Resilience::Medium as usize] = 4.0;
        let stock = FishStock::unexploited(Resilience::Medium, f64::MAX * 0.5)
            .expect("finite positive carrying capacity");

        assert_eq!(stock.msy_t(&model), Ok(f64::MAX * 0.5));
        assert_eq!(stock.msy_biomass_t(&model), Ok(f64::MAX * 0.25));
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
            let before_target_read = stock.clone();
            let target_biomass = stock
                .msy_biomass_t(&model)
                .expect("valid model has a finite peak biomass target");
            assert_eq!(
                stock, before_target_read,
                "the MSY biomass diagnostic must not mutate stock state"
            );
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
    fn msy_peak_is_stable_at_adjacent_binary64_depensation_boundaries() {
        let mut model = FisheryModel::load_default().expect("the pack loads");
        model.r_per_year[Resilience::Medium as usize] = 1.0;

        let half = 0.5_f64;
        let two_thirds = 2.0_f64 / 3.0;
        let thresholds = [
            half,
            f64::from_bits(half.to_bits() + 1),
            f64::from_bits(two_thirds.to_bits() - 1),
            two_thirds,
            f64::from_bits(two_thirds.to_bits() + 1),
        ];
        let carrying_capacity = 1_200.0;

        for threshold in thresholds {
            model.depensation_below = threshold;
            let stock = FishStock::unexploited(Resilience::Medium, carrying_capacity)
                .expect("positive carrying capacity");
            let peak_biomass_fraction = if threshold <= half {
                0.5
            } else if threshold <= two_thirds {
                threshold
            } else {
                two_thirds
            };
            let peak_surplus_fraction = if threshold <= half {
                0.25
            } else if threshold <= two_thirds {
                threshold * (1.0 - threshold)
            } else {
                4.0 / (27.0 * threshold)
            };

            let target = stock
                .msy_biomass_t(&model)
                .expect("valid threshold has a finite peak target");
            let msy = stock
                .msy_t(&model)
                .expect("valid threshold has finite maximum surplus");
            let mut at_target = stock.clone();
            at_target.biomass_t = target;
            let surplus_at_target = at_target
                .surplus_t_per_year(&model)
                .expect("analytic target has finite surplus");

            let expected_target = carrying_capacity * peak_biomass_fraction;
            let expected_msy = carrying_capacity * peak_surplus_fraction;
            let tolerance = expected_msy.max(1.0) * 1.0e-12;

            assert!(
                (target - expected_target).abs() <= tolerance,
                "threshold {threshold}: target {target} vs {expected_target}"
            );
            assert!(
                (msy - expected_msy).abs() <= tolerance,
                "threshold {threshold}: MSY {msy} vs {expected_msy}"
            );
            assert!(
                (surplus_at_target - msy).abs() <= tolerance,
                "threshold {threshold}: target surplus {surplus_at_target} vs MSY {msy}"
            );
        }
    }

    #[test]
    fn harvesting_all_interval_growth_does_not_reduce_biomass_from_rounding() {
        let model = FisheryModel {
            r_per_year: [0.0, 0.0012, 0.0, 0.0],
            depensation_below: 0.25,
        };
        let biomass = 2.0_f64.powi(62);
        let mut stock = FishStock {
            resilience: Resilience::Medium,
            k_t: biomass * 2.0,
            biomass_t: biomass,
            landed_t: 0.0,
        };
        let before = stock.biomass_t;
        let growth = stock
            .sustainable_catch_limit_t(&model, YEAR_S)
            .expect("valid interval growth");
        assert!(growth > 0.0);

        let landed = stock
            .fish(&model, f64::MAX, YEAR_S)
            .expect("the requested catch is capped at interval growth");

        assert_eq!(landed, growth);
        assert_eq!(
            stock.biomass_t, before,
            "harvesting all growth must leave the original biomass unchanged"
        );
        assert_eq!(stock.landed_t, growth);
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
    fn sustainable_catch_limit_is_read_only_and_matches_the_fish_step() {
        let model = FisheryModel::load_default().expect("the pack loads");
        let mut stock = FishStock::unexploited(Resilience::Medium, 1_000.0)
            .expect("positive carrying capacity");
        stock.biomass_t = 500.0;
        let before = stock.clone();

        let limit = stock
            .sustainable_catch_limit_t(&model, YEAR_S)
            .expect("valid one-year interval");
        assert!((limit - 125.0).abs() < 1e-10);
        assert_eq!(stock, before, "the planning diagnostic must be read-only");

        let landed = stock
            .fish(&model, f64::MAX, YEAR_S)
            .expect("valid fishing step");
        assert_eq!(
            landed, limit,
            "fish() must enforce the reported interval cap"
        );
        assert_eq!(stock.biomass_t, before.biomass_t);
        assert_eq!(stock.landed_t, limit);
    }

    #[test]
    fn sustainable_catch_limit_is_capped_at_remaining_carrying_capacity() {
        let model = FisheryModel::load_default().expect("the pack loads");
        let mut stock = FishStock::unexploited(Resilience::Medium, 1_000.0)
            .expect("positive carrying capacity");
        stock.biomass_t = 999.9;
        let before = stock.clone();

        let limit = stock
            .sustainable_catch_limit_t(&model, 100.0 * YEAR_S)
            .expect("valid coarse interval");
        assert!((limit - 0.1).abs() < 1e-9);
        assert_eq!(stock, before, "the diagnostic must not advance biomass");

        let landed = stock
            .fish(&model, 1.0, 100.0 * YEAR_S)
            .expect("valid coarse fishing step");
        assert!((landed - limit).abs() < 1e-9);
        assert!(stock.biomass_t <= stock.k_t);
    }

    #[test]
    fn sustainable_catch_limit_rejects_invalid_inputs_without_mutating_the_stock() {
        let model = FisheryModel::load_default().expect("the pack loads");
        let stock = FishStock::unexploited(Resilience::Medium, 1_000.0)
            .expect("positive carrying capacity");
        let before = stock.clone();

        for dt in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -1.0] {
            assert_eq!(
                stock.sustainable_catch_limit_t(&model, dt),
                Err(FisheryError::InvalidTimeStep)
            );
            assert_eq!(stock, before);
        }

        let mut invalid_model = model.clone();
        invalid_model.r_per_year[Resilience::Medium as usize] = f64::NAN;
        assert_eq!(
            stock.sustainable_catch_limit_t(&invalid_model, YEAR_S),
            Err(FisheryError::InvalidModel)
        );

        let mut invalid_stock = before.clone();
        invalid_stock.biomass_t = invalid_stock.k_t + 1.0;
        assert_eq!(
            invalid_stock.sustainable_catch_limit_t(&model, YEAR_S),
            Err(FisheryError::InvalidStockState)
        );
        assert_eq!(invalid_stock.biomass_t, invalid_stock.k_t + 1.0);
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
    fn positive_interval_growth_below_binary64_range_is_rejected_without_mutation() {
        let m = FisheryModel::load_default().expect("the pack loads");
        let mut stock = FishStock::unexploited(Resilience::Medium, 1_000.0)
            .expect("positive carrying capacity");
        stock.biomass_t = 500.0;
        let before = stock.clone();
        let smallest_positive_step = f64::from_bits(1);

        assert!(
            stock
                .surplus_t_per_year(&m)
                .expect("positive annual surplus")
                > 0.0
        );
        assert_eq!(
            stock.sustainable_catch_limit_t(&m, smallest_positive_step),
            Err(FisheryError::UnrepresentableGrowth)
        );
        assert_eq!(
            stock.fish(&m, 0.0, smallest_positive_step),
            Err(FisheryError::UnrepresentableGrowth)
        );
        assert_eq!(
            stock, before,
            "an unrepresentable interval increment must not mutate the stock"
        );
    }

    #[test]
    fn low_biomass_surplus_uses_log_space_when_intermediate_product_underflows() {
        let model = FisheryModel {
            r_per_year: [0.0, 1.0e308, 0.0, 0.0],
            depensation_below: 0.25,
        };
        let mut stock = FishStock::unexploited(Resilience::Medium, 1.0e308)
            .expect("positive carrying capacity");
        stock.biomass_t = 5.0e-16;

        let p = stock.biomass_t / stock.k_t;
        let biomass_factor = (1.0 - p) * (p / model.depensation_below);
        assert!(p > 0.0, "the ratio itself remains representable");
        assert!(
            biomass_factor > 0.0,
            "the depensation factor remains representable"
        );
        assert_eq!(
            stock.biomass_t * biomass_factor,
            0.0,
            "the biomass-times-factor intermediate underflows"
        );

        let log_expected = model.r(Resilience::Medium).log2() + 2.0 * stock.biomass_t.log2()
            - stock.k_t.log2()
            - model.depensation_below.log2()
            + (1.0 - p).log2();
        let expected = log_expected.exp2();
        let surplus = stock
            .surplus_t_per_year(&model)
            .expect("the mathematically representable surplus is finite");

        assert!(expected > 0.0);
        assert!(surplus > 0.0, "positive recruitment must not disappear");
        assert!(
            (surplus / expected - 1.0).abs() <= 1.0e-12,
            "log-space fallback should preserve the representable result"
        );
    }

    #[test]
    fn low_biomass_surplus_uses_log_space_for_subnormal_intermediate_precision() {
        let model = FisheryModel {
            r_per_year: [0.0, 1.0e308, 0.0, 0.0],
            depensation_below: 0.25,
        };
        let mut stock = FishStock::unexploited(Resilience::Medium, 4_000.0)
            .expect("positive carrying capacity");
        stock.biomass_t = 1.0e-160;

        let p = stock.biomass_t / stock.k_t;
        let biomass_factor = (1.0 - p) * (p / model.depensation_below);
        let biomass_product = stock.biomass_t * biomass_factor;
        let direct = model.r(Resilience::Medium) * biomass_product;
        let log_expected = model.r(Resilience::Medium).log2() + 2.0 * stock.biomass_t.log2()
            - stock.k_t.log2()
            - model.depensation_below.log2()
            + (1.0 - p).log2();
        let expected = log_expected.exp2();

        assert!(p >= f64::MIN_POSITIVE);
        assert!(biomass_factor >= f64::MIN_POSITIVE);
        assert!(
            biomass_product > 0.0 && biomass_product < f64::MIN_POSITIVE,
            "the intermediate remains positive but is subnormal"
        );
        assert!(expected.is_normal(), "the final surplus remains normal");
        assert!(
            (direct / expected - 1.0).abs() > 1.0e-3,
            "the rounded subnormal intermediate must demonstrate material precision loss"
        );

        let surplus = stock
            .surplus_t_per_year(&model)
            .expect("the mathematically representable surplus is finite");
        assert!(
            (surplus / expected - 1.0).abs() <= 1.0e-12,
            "log-space evaluation should recover the normal final result"
        );
    }

    #[test]
    fn low_biomass_surplus_uses_log_space_when_ratio_is_subnormal_but_product_is_normal() {
        let model = FisheryModel {
            r_per_year: [0.0, 1.0e308, 0.0, 0.0],
            depensation_below: 2.0e-323,
        };
        let mut stock = FishStock::unexploited(Resilience::Medium, 1.0e308)
            .expect("positive carrying capacity");
        stock.biomass_t = 1.0e-15;

        let p = stock.biomass_t / stock.k_t;
        let biomass_factor = (1.0 - p) * (p / model.depensation_below);
        let biomass_product = stock.biomass_t * biomass_factor;
        let direct = model.r(Resilience::Medium) * biomass_product;
        let log_expected = model.r(Resilience::Medium).log2() + 2.0 * stock.biomass_t.log2()
            - stock.k_t.log2()
            - model.depensation_below.log2()
            + (1.0 - p).log2();
        let expected = log_expected.exp2();

        assert_eq!(p, 1.0e-323, "pin the intended positive subnormal ratio");
        assert!(
            biomass_factor.is_normal(),
            "the depensation factor remains normal"
        );
        assert!(
            biomass_product.is_normal(),
            "the biomass product remains normal"
        );
        assert!(expected.is_normal(), "the final surplus remains normal");
        assert!(
            (direct / expected - 1.0).abs() > 1.0e-3,
            "the subnormal ratio must demonstrate material precision loss"
        );

        let surplus = stock
            .surplus_t_per_year(&model)
            .expect("the mathematically representable surplus is finite");
        assert!(
            (surplus / expected - 1.0).abs() <= 1.0e-12,
            "log-space evaluation should recover the normal final result"
        );
    }

    #[test]
    fn landed_increment_below_cumulative_total_precision_is_rejected_without_mutation() {
        let model = FisheryModel::load_default().expect("the pack loads");
        let mut stock = FishStock::unexploited(Resilience::Medium, 1_000.0)
            .expect("positive carrying capacity");
        stock.biomass_t = 500.0;
        stock.landed_t = 9_007_199_254_740_992.0; // 2^53
        let before = stock.clone();

        // At 2^53, adding one tonne rounds back to the same binary64 value.
        assert_eq!(stock.landed_t + 1.0, stock.landed_t);
        assert!(
            stock
                .sustainable_catch_limit_t(&model, YEAR_S)
                .expect("valid one-year growth")
                > 1.0
        );

        assert_eq!(
            stock.fish(&model, 1.0, YEAR_S),
            Err(FisheryError::LandedTotalUnrepresentable)
        );
        assert_eq!(
            stock, before,
            "a catch that cannot be represented in the cumulative total must not commit biomass"
        );
    }

    #[test]
    fn partial_harvest_rejects_unrepresentable_remaining_growth_without_mutation() {
        let mut model = FisheryModel::load_default().expect("the pack loads");
        model.r_per_year[Resilience::Medium as usize] = 0.5;
        model.depensation_below = 0.25;

        // At 2^52 the binary64 spacing is one tonne. A small positive remainder
        // is therefore lost when added to this biomass, even though the catch
        // itself remains representable in the cumulative landed total.
        let mut stock = FishStock::unexploited(
            Resilience::Medium,
            9_007_199_254_740_992.0, // 2^53
        )
        .expect("finite positive carrying capacity");
        stock.biomass_t = 4_503_599_627_370_496.0; // 2^52

        let surplus = stock
            .surplus_t_per_year(&model)
            .expect("valid annual surplus");
        let dt_s = YEAR_S * 0.125 / surplus;
        let grown = stock
            .growth_capacity_t(&model, dt_s)
            .expect("small positive interval growth");
        let wanted = grown / 2.0;
        let remaining_growth = grown - wanted;

        assert!(wanted > 0.0);
        assert!(remaining_growth > 0.0 && remaining_growth < 0.5);
        assert_eq!(stock.biomass_t + remaining_growth, stock.biomass_t);
        assert_ne!(stock.landed_t + wanted, stock.landed_t);

        let before = stock.clone();
        assert_eq!(
            stock.fish(&model, wanted, dt_s),
            Err(FisheryError::BiomassRemainderUnrepresentable)
        );
        assert_eq!(
            stock, before,
            "a partial catch must not discard unrepresentable residual growth"
        );
    }

    #[test]
    fn unharvested_growth_below_stock_ulp_is_rejected_without_mutation() {
        let mut model = FisheryModel::load_default().expect("the pack loads");
        model.r_per_year[Resilience::Medium as usize] = 1.0e308;
        model.depensation_below = 0.25;

        let mut stock = FishStock::unexploited(Resilience::Medium, 1.0e308)
            .expect("finite positive carrying capacity");
        stock.biomass_t = 1.0e-100;

        // The interval growth is positive and representable on its own, but is
        // too small to change the much larger existing biomass. With no catch,
        // silently accepting this step would discard all growth from the ledger.
        let before = stock.clone();
        let growth = stock
            .sustainable_catch_limit_t(&model, YEAR_S)
            .expect("the interval growth itself is representable");
        assert!(growth > 0.0);
        assert_eq!(stock.biomass_t + growth, stock.biomass_t);

        assert_eq!(
            stock.fish(&model, 0.0, YEAR_S),
            Err(FisheryError::BiomassRemainderUnrepresentable)
        );
        assert_eq!(
            stock, before,
            "a zero-catch step must not silently discard unrepresentable stock growth"
        );
    }

    #[test]
    fn landed_increment_rounded_to_a_different_amount_is_rejected_without_mutation() {
        let mut model = FisheryModel::load_default().expect("the pack loads");
        model.r_per_year[Resilience::Medium as usize] = 0.5;
        model.depensation_below = 0.25;

        let mut stock = FishStock::unexploited(Resilience::Medium, 1_000.0)
            .expect("positive carrying capacity");
        stock.biomass_t = 500.0;
        stock.landed_t = 9_007_199_254_740_992.0; // 2^53
        let before = stock.clone();
        let requested = 3.0;

        // At 2^53, adding three rounds up to four tonnes in binary64.
        assert_eq!(stock.landed_t + requested - stock.landed_t, 4.0);
        assert!(
            stock
                .sustainable_catch_limit_t(&model, YEAR_S)
                .expect("valid one-year growth")
                > requested
        );

        assert_eq!(
            stock.fish(&model, requested, YEAR_S),
            Err(FisheryError::LandedTotalUnrepresentable)
        );
        assert_eq!(
            stock, before,
            "a rounded cumulative increment must not commit biomass or landed mass"
        );
    }

    #[test]
    fn landed_increment_rounded_down_is_rejected_without_mutation() {
        let mut model = FisheryModel::load_default().expect("the pack loads");
        model.r_per_year[Resilience::Medium as usize] = 0.5;
        model.depensation_below = 0.25;

        let mut stock = FishStock::unexploited(Resilience::Medium, 1_000.0)
            .expect("positive carrying capacity");
        stock.biomass_t = 500.0;
        stock.landed_t = 9_007_199_254_740_992.0; // 2^53
        let before = stock.clone();
        let requested = 5.0;

        // At 2^53, adding five is a tie that rounds down to a four-tonne delta.
        assert_eq!(stock.landed_t + requested - stock.landed_t, 4.0);
        assert!(
            stock
                .sustainable_catch_limit_t(&model, YEAR_S)
                .expect("valid one-year growth")
                > requested
        );

        assert_eq!(
            stock.fish(&model, requested, YEAR_S),
            Err(FisheryError::LandedTotalUnrepresentable)
        );
        assert_eq!(
            stock, before,
            "a rounded-down cumulative increment must not commit biomass or landed mass"
        );
    }

    #[test]
    fn ordinary_cumulative_rounding_stays_within_the_landed_total_tolerance() {
        let mut model = FisheryModel::load_default().expect("the pack loads");
        model.r_per_year[Resilience::Medium as usize] = 0.5;
        model.depensation_below = 0.25;

        let mut stock = FishStock::unexploited(Resilience::Medium, 1_000.0)
            .expect("positive carrying capacity");
        stock.biomass_t = 500.0;
        stock.landed_t = 0.3;

        let landed = stock
            .fish(&model, 0.1, YEAR_S)
            .expect("ordinary floating-point accumulation remains usable");

        assert_eq!(landed, 0.1);
        let recorded_delta = stock.landed_t - 0.3;
        assert!(recorded_delta > 0.0);
        assert!((recorded_delta - landed).abs() <= landed * LANDED_TOTAL_RELATIVE_TOLERANCE);
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

    #[test]
    fn non_depensated_surplus_recovers_subnormal_rate_factor_before_large_biomass() {
        let min_subnormal = f64::from_bits(1);
        let model = FisheryModel {
            r_per_year: [0.0, min_subnormal, 0.0, 0.0],
            depensation_below: 0.25,
        };
        let mut stock = FishStock::unexploited(Resilience::Medium, 1.0e308)
            .expect("positive carrying capacity");
        stock.biomass_t = 5.0e307;

        let p = stock.biomass_t / stock.k_t;
        let direct_factor = model.r(Resilience::Medium) * (1.0 - p);
        assert_eq!(p, 0.5);
        assert_eq!(direct_factor, 0.0, "the subnormal rate product rounds away");

        let expected =
            (model.r(Resilience::Medium).log2() + (1.0 - p).log2() + stock.biomass_t.log2()).exp2();
        assert!(expected.is_normal(), "the final surplus is representable");

        let surplus = stock
            .surplus_t_per_year(&model)
            .expect("log-space evaluation recovers the representable surplus");
        assert!(
            (surplus / expected - 1.0).abs() <= 1.0e-12,
            "a subnormal intermediate must not erase a normal final surplus"
        );
    }

    #[test]
    fn positive_but_unrepresentable_surplus_is_rejected_without_mutation() {
        let min_subnormal = f64::from_bits(1);
        let model = FisheryModel {
            r_per_year: [0.0, min_subnormal, 0.0, 0.0],
            depensation_below: 0.25,
        };

        for (carrying_capacity, biomass) in
            [(1.0, min_subnormal), (2.0 * min_subnormal, min_subnormal)]
        {
            let mut stock = FishStock::unexploited(Resilience::Medium, carrying_capacity)
                .expect("positive carrying capacity");
            stock.biomass_t = biomass;
            let before = stock.clone();

            assert_eq!(
                stock.surplus_t_per_year(&model),
                Err(FisheryError::UnrepresentableGrowth)
            );
            assert_eq!(
                stock.fish(&model, 1.0, YEAR_S),
                Err(FisheryError::UnrepresentableGrowth)
            );
            assert_eq!(
                stock, before,
                "an unrepresentable step must not mutate the stock"
            );
        }
    }
}
