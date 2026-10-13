//! The house's gas stove and its bottled LPG (realism programme R4, in
//! part; deviation D58).
//!
//! The founders' kitchen has a four-burner gas stove with an oven
//! (`organisms::property`), and until this nothing it cooked burned
//! anything. Now it burns what a New Zealand house spends on cooking:
//!
//! - **Use** is HEEP's average cooking energy per house (BRANZ SR155,
//!   Table 66), drawn as a steady rate while anyone is at home. It is
//!   averaged over every fuel and house, not measured on gas stoves, and
//!   has no meals in it; D58 records both.
//! - **Supply** is the typical home installation, two 45 kg bottles
//!   (Elgas), full at the start. Nobody delivers more: when they are empty
//!   the stove is out, and the time the household spends at home without
//!   gas is counted.
//! - **Burning** takes LPG's carbon from the crust to the air as CO₂,
//!   binding the oxygen it needs, at IPCC's net calorific value and carbon
//!   content, the way the generator's diesel is booked.

use serde::{Deserialize, Serialize};

use crate::validation::{ReferenceDomain, ReferenceLibrary};

/// The length of the year HEEP's annual figure is per (an Earth year, s).
const EARTH_YEAR_S: f64 = 365.25 * 86_400.0;

/// The figures the stove runs on, read from the packs.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct HouseholdGasSpec {
    /// Cooking energy a house uses in a year (kWh, gross).
    pub cooking_kwh_per_year: f64,
    /// LPG's net calorific value (MJ/kg).
    pub lpg_mj_per_kg: f64,
    /// LPG's carbon per unit energy (kg C/GJ).
    pub lpg_carbon_kg_per_gj: f64,
    /// One bottle's fill (kg) and how many the house has.
    pub cylinder_kg: f64,
    pub cylinders: f64,
}

impl HouseholdGasSpec {
    pub fn from_reference(library: &ReferenceLibrary) -> Result<Self, String> {
        let d = ReferenceDomain::Industry;
        let cooking = library
            .item(d, "household_cooking_energy")
            .and_then(|i| i.central())
            .ok_or("household_cooking_energy")?;
        let lpg = |label: &str| {
            library
                .item(d, "lpg_fuel_properties")
                .and_then(|i| i.table())
                .and_then(|t| t.get(label, "value"))
                .ok_or_else(|| format!("lpg_fuel_properties/{label}"))
        };
        let bottles = |label: &str| {
            library
                .item(d, "lpg_home_cylinders")
                .and_then(|i| i.table())
                .and_then(|t| t.get(label, "value"))
                .ok_or_else(|| format!("lpg_home_cylinders/{label}"))
        };
        Ok(Self {
            cooking_kwh_per_year: cooking,
            lpg_mj_per_kg: lpg("net_calorific_value_mj_per_kg")?,
            lpg_carbon_kg_per_gj: lpg("carbon_kg_per_gj")?,
            cylinder_kg: bottles("cylinder_kg")?,
            cylinders: bottles("cylinders")?,
        })
    }

    /// LPG the stove burns per second of cooking at the house's average
    /// rate (kg/s).
    pub fn burn_kg_per_s(&self) -> f64 {
        self.cooking_kwh_per_year * 3.6 / self.lpg_mj_per_kg.max(f64::MIN_POSITIVE) / EARTH_YEAR_S
    }

    /// Carbon in a kilogram of LPG (kg C/kg).
    pub fn carbon_per_kg(&self) -> f64 {
        self.lpg_carbon_kg_per_gj * self.lpg_mj_per_kg / 1_000.0
    }
}

/// The stove's gas: what is left in the bottles, and what it has done.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HouseholdGas {
    pub spec: HouseholdGasSpec,
    /// LPG left in the bottles (kg).
    pub stored_kg: f64,
    /// LPG burned so far (kg).
    pub burned_kg: f64,
    /// Seconds the household was at home with the bottles empty.
    pub without_gas_s: f64,
}

impl HouseholdGas {
    /// The house's bottles, full.
    pub fn new(spec: HouseholdGasSpec) -> Self {
        Self {
            stored_kg: spec.cylinder_kg * spec.cylinders,
            spec,
            burned_kg: 0.0,
            without_gas_s: 0.0,
        }
    }

    /// Cook for `dt_s` if anyone is at home: burn the stove's share of the
    /// gas left, and count the time without any. Returns the kg burned.
    pub fn cook(&mut self, anyone_home: bool, dt_s: f64) -> f64 {
        if !anyone_home || !(dt_s.is_finite() && dt_s > 0.0) {
            return 0.0;
        }
        let want = self.spec.burn_kg_per_s() * dt_s;
        let burned = want.min(self.stored_kg.max(0.0));
        self.stored_kg -= burned;
        self.burned_kg += burned;
        if burned < want {
            self.without_gas_s += dt_s * (1.0 - burned / want.max(f64::MIN_POSITIVE));
        }
        burned
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec() -> HouseholdGasSpec {
        let library = ReferenceLibrary::load(&crate::validation::default_reference_dir()).unwrap();
        HouseholdGasSpec::from_reference(&library).unwrap()
    }

    #[test]
    fn a_year_at_home_burns_heeps_cooking_energy_and_two_bottles_last_over_a_year() {
        let spec = spec();
        let mut gas = HouseholdGas::new(spec);
        assert_eq!(gas.stored_kg, 90.0);
        let mut burned = 0.0;
        for _ in 0..365 {
            burned += gas.cook(true, 86_400.0);
        }
        // 900 kWh at 47.3 MJ/kg: 68.5 kg a year.
        let kwh = burned * spec.lpg_mj_per_kg / 3.6;
        assert!((kwh / 900.0 - 365.0 / 365.25).abs() < 1e-9, "{kwh} kWh");
        assert!(gas.stored_kg > 0.0 && gas.without_gas_s == 0.0);
        // An empty house cooks nothing.
        assert_eq!(gas.cook(false, 86_400.0), 0.0);
    }

    #[test]
    fn empty_bottles_stop_the_stove_and_count_the_time_without() {
        let mut gas = HouseholdGas::new(spec());
        gas.stored_kg = 0.0;
        assert_eq!(gas.cook(true, 3_600.0), 0.0);
        assert_eq!(gas.without_gas_s, 3_600.0);
        assert_eq!(gas.burned_kg, 0.0);
    }

    #[test]
    fn lpg_carries_ipccs_carbon() {
        // 17.2 kg C/GJ at 47.3 MJ/kg: 0.81 kg C per kg, against 0.817
        // for pure propane (3 × 12.011 / 44.097).
        let c = spec().carbon_per_kg();
        assert!((c - 0.81356).abs() < 1e-5, "{c}");
    }
}
