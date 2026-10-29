//! The house's log burner with a wetback (the owner's rule, 2026-10-23;
//! deviation D59).
//!
//! The island is the start of a civilisation: a house begins with a log
//! burner that cooks on its top and in its oven, heats the house and,
//! through the wetback coil behind its firebox, the hot water cylinder.
//! Gas or electric stoves come only when someone buys one and has it put
//! in. The burner's use is what New Zealand houses with each solid-fuel
//! end-use measured (BRANZ SR155, Table 3, gross energy):
//!
//! - **Cooking**, a solid-fuel range's 942 kWh a year.
//! - **Space heating**, a solid-fuel heater's 4,446 kWh a year.
//! - **Hot water**, a wetback's 908 kWh a year.
//!
//! It burns that energy as a steady rate while anyone is at home, from wood
//! someone has split (D63), at air-dry wood's net calorific value (BRANZ
//! SR141, §6.6.1). With no split wood the burner is cold, and the time at
//! home without it is counted.

use serde::{Deserialize, Serialize};

use crate::validation::{ReferenceDomain, ReferenceLibrary};

/// The length of the year HEEP's annual figures are per (an Earth year, s).
const EARTH_YEAR_S: f64 = 365.25 * 86_400.0;

/// The figures the burner runs on, read from the packs.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct WoodBurnerSpec {
    /// Gross wood energy a year for cooking, space heating and the wetback
    /// (kWh).
    pub cooking_kwh_per_year: f64,
    pub heating_kwh_per_year: f64,
    pub wetback_kwh_per_year: f64,
    /// Net calorific value of air-dry wood (kWh/kg).
    pub wood_kwh_per_kg: f64,
}

impl WoodBurnerSpec {
    pub fn from_reference(library: &ReferenceLibrary) -> Result<Self, String> {
        let d = ReferenceDomain::Industry;
        let use_kwh = |label: &str| {
            library
                .item(d, "household_solid_fuel_end_uses")
                .and_then(|i| i.table())
                .and_then(|t| t.get(label, "annual_kwh"))
                .ok_or_else(|| format!("household_solid_fuel_end_uses/{label}"))
        };
        let wood_kwh_per_kg = library
            .item(d, "air_dry_wood_net_calorific_value")
            .and_then(|i| i.point())
            .ok_or("air_dry_wood_net_calorific_value")?;
        if !(wood_kwh_per_kg > 0.0) {
            return Err("air_dry_wood_net_calorific_value must be positive".to_string());
        }
        Ok(Self {
            cooking_kwh_per_year: use_kwh("range")?,
            heating_kwh_per_year: use_kwh("heating")?,
            wetback_kwh_per_year: use_kwh("wetback")?,
            wood_kwh_per_kg,
        })
    }

    /// Wood the burner takes per second while anyone is at home (kg/s).
    pub fn burn_kg_per_s(&self) -> f64 {
        (self.cooking_kwh_per_year + self.heating_kwh_per_year + self.wetback_kwh_per_year)
            / self.wood_kwh_per_kg.max(f64::MIN_POSITIVE)
            / EARTH_YEAR_S
    }
}

/// The burner: what it has burned, and the time it was cold.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WoodBurner {
    pub spec: WoodBurnerSpec,
    /// Air-dry wood burned so far (kg).
    pub wood_burned_kg: f64,
    /// Seconds someone was at home with no split wood to burn.
    pub cold_s: f64,
}

impl WoodBurner {
    pub fn new(spec: WoodBurnerSpec) -> Self {
        Self {
            spec,
            wood_burned_kg: 0.0,
            cold_s: 0.0,
        }
    }

    /// Wood the burner wants for `dt_s` (kg): none if nobody is at home.
    pub fn wanted_kg(&self, anyone_home: bool, dt_s: f64) -> f64 {
        if !anyone_home || !(dt_s.is_finite() && dt_s > 0.0) {
            return 0.0;
        }
        self.spec.burn_kg_per_s() * dt_s
    }

    /// Record a step that burned `burned_kg` of the `wanted_kg` it wanted
    /// over `dt_s`: any shortfall is time with the burner cold.
    pub fn record(&mut self, wanted_kg: f64, burned_kg: f64, dt_s: f64) {
        self.wood_burned_kg += burned_kg.max(0.0);
        if wanted_kg > 0.0 && burned_kg < wanted_kg {
            self.cold_s += dt_s * (1.0 - burned_kg.max(0.0) / wanted_kg);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec() -> WoodBurnerSpec {
        let library = ReferenceLibrary::load(&crate::validation::default_reference_dir()).unwrap();
        WoodBurnerSpec::from_reference(&library).unwrap()
    }

    #[test]
    fn a_year_at_home_burns_heeps_solid_fuel_cooking_heating_and_wetback() {
        let mut burner = WoodBurner::new(spec());
        let mut kg = 0.0;
        for _ in 0..365 {
            let want = burner.wanted_kg(true, 86_400.0);
            burner.record(want, want, 86_400.0);
            kg += want;
        }
        // 942 + 4,446 + 908 kWh at 4.0 kWh/kg: 1,574 kg a year.
        let kwh = kg * burner.spec.wood_kwh_per_kg;
        assert!((kwh / 6_296.0 - 365.0 / 365.25).abs() < 1e-9, "{kwh} kWh");
        assert_eq!(burner.wood_burned_kg, kg);
        assert_eq!(burner.cold_s, 0.0);
        // An empty house burns nothing.
        assert_eq!(burner.wanted_kg(false, 86_400.0), 0.0);
    }

    #[test]
    fn without_split_wood_the_burner_is_cold() {
        let mut burner = WoodBurner::new(spec());
        let want = burner.wanted_kg(true, 3_600.0);
        burner.record(want, 0.25 * want, 3_600.0);
        assert!((burner.cold_s - 2_700.0).abs() < 1e-9);
        assert_eq!(burner.wood_burned_kg, 0.25 * want);
    }
}
