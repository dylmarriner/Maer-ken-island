//! The house's gas stove and its bottled LPG (realism programme R4, in
//! part; deviation D59).
//!
//! The founders' kitchen has a four-burner gas stove with an oven
//! (`organisms::property`), and until this nothing it cooked burned
//! anything. Now it burns what a New Zealand house spends on cooking:
//!
//! - **Use** is HEEP's average cooking energy per house (BRANZ SR155,
//!   Table 66), drawn as a steady rate while anyone is at home. It is
//!   averaged over every fuel and house, not measured on gas stoves, and
//!   has no meals in it; D59 records both.
//! - **Supply** is the typical home installation, two 45 kg bottles
//!   (Elgas), full at the start. Nobody delivers more: when they are empty
//!   the stove is out, and the time the household spends at home without
//!   gas is counted.
//! - **Burning** takes LPG's carbon from the crust to the air as CO₂,
//!   binding the oxygen it needs, at IPCC's net calorific value and carbon
//!   content, the way the generator's diesel is booked.
//! - **A wood fire** cooks when the bottles are empty: the household burns
//!   the measured firewood of households cooking on open fires, per person
//!   at home, from the store's air-dry wood. With no wood either, the time
//!   at home without any cooking fuel is counted.

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
    /// Air-dry firewood a person's cooking burns on an open fire (kg per
    /// person per day). 0 in an island saved before the fire.
    #[serde(default)]
    pub firewood_kg_per_person_day: f64,
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
            firewood_kg_per_person_day: library
                .item(d, "household_firewood_use")
                .and_then(|i| i.central())
                .ok_or("household_firewood_use")?,
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
    /// Air-dry wood the fire has burned (kg), and the seconds at home with
    /// neither gas nor wood.
    #[serde(default)]
    pub wood_burned_kg: f64,
    #[serde(default)]
    pub without_fuel_s: f64,
}

impl HouseholdGas {
    /// The house's bottles, full.
    pub fn new(spec: HouseholdGasSpec) -> Self {
        Self {
            stored_kg: spec.cylinder_kg * spec.cylinders,
            spec,
            burned_kg: 0.0,
            without_gas_s: 0.0,
            wood_burned_kg: 0.0,
            without_fuel_s: 0.0,
        }
    }

    /// Air-dry wood a fire for `people` at home needs to cover `short_s`
    /// seconds the gas could not (kg).
    pub fn firewood_wanted_kg(&self, people: usize, short_s: f64) -> f64 {
        self.spec.firewood_kg_per_person_day * people as f64 * short_s.max(0.0) / 86_400.0
    }

    /// Record a fire that burned `burned_kg` of the `wanted_kg` it needed
    /// over `short_s`: any shortfall is time without cooking fuel.
    pub fn record_fire(&mut self, wanted_kg: f64, burned_kg: f64, short_s: f64) {
        self.wood_burned_kg += burned_kg.max(0.0);
        if wanted_kg > 0.0 && burned_kg < wanted_kg {
            self.without_fuel_s += short_s * (1.0 - burned_kg.max(0.0) / wanted_kg);
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
    fn a_fire_covers_the_gas_it_lacks_at_the_measured_firewood_rate() {
        let mut gas = HouseholdGas::new(spec());
        let rate = gas.spec.firewood_kg_per_person_day;
        assert!(rate > 0.0);
        // Two people, a day without gas: two people's day of firewood.
        let wanted = gas.firewood_wanted_kg(2, 86_400.0);
        assert!((wanted - 2.0 * rate).abs() < 1e-12);
        // Half of it in the store: half the day without any fuel.
        gas.record_fire(wanted, 0.5 * wanted, 86_400.0);
        assert_eq!(gas.wood_burned_kg, 0.5 * wanted);
        assert!((gas.without_fuel_s - 43_200.0).abs() < 1e-9);
    }

    #[test]
    fn lpg_carries_ipccs_carbon() {
        // 17.2 kg C/GJ at 47.3 MJ/kg: 0.81 kg C per kg, against 0.817
        // for pure propane (3 × 12.011 / 44.097).
        let c = spec().carbon_per_kg();
        assert!((c - 0.81356).abs() < 1e-5, "{c}");
    }
}
