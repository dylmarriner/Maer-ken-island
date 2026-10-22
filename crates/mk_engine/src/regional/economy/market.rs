//! Prices and metered bills (Phase 4b Task 7, third part).
//!
//! The founders' power and water companies bill each household **monthly
//! from its meters** at sourced New Zealand prices (`industry/prices.json`):
//! - electricity at MBIE's modelled-household retail price (15 August 2026),
//!   which already spreads the plan's daily charge over the kilowatt-hours;
//! - water at Watercare's 2026-27 volumetric charge, and wastewater on
//!   78.5% of the water metered plus a twelfth of its fixed yearly charge.
//!
//! All prices include GST, which on the island goes to the supplier: there
//! is no government to collect it (D62).

use serde::{Deserialize, Serialize};

use super::money::{Cents, MoneyError, CENTS_PER_DOLLAR};
use crate::validation::{ReferenceDomain, ReferenceLibrary};

/// The sourced household tariffs, in cents.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Tariffs {
    pub electricity_cents_per_kwh: f64,
    pub water_cents_per_m3: f64,
    pub wastewater_cents_per_m3: f64,
    /// Share of the water metered that is charged as wastewater.
    pub wastewater_share: f64,
    pub wastewater_fixed_cents_per_year: f64,
}

impl Tariffs {
    pub fn from_reference(library: &ReferenceLibrary) -> Result<Self, MoneyError> {
        let point = |key: &str| {
            library
                .item(ReferenceDomain::Industry, key)
                .and_then(|i| i.point())
                .ok_or_else(|| MoneyError::Reference(format!("industry/prices.json: {key}")))
        };
        let cents = CENTS_PER_DOLLAR as f64;
        Ok(Self {
            electricity_cents_per_kwh: point("electricity_retail_nz_2026_08")? * cents,
            water_cents_per_m3: point("water_volumetric_2026_27")? * cents,
            wastewater_cents_per_m3: point("wastewater_volumetric_2026_27")? * cents,
            wastewater_share: point("wastewater_share_of_water_2026_27")?,
            wastewater_fixed_cents_per_year: point("wastewater_fixed_2026_27")? * cents,
        })
    }

    /// A month's electricity bill for `kwh` used.
    pub fn electricity_bill(&self, kwh: f64) -> Cents {
        (kwh.max(0.0) * self.electricity_cents_per_kwh).round() as Cents
    }

    /// A month's water and wastewater bill for `m3` of water used.
    pub fn water_bill(&self, m3: f64) -> Cents {
        let m3 = m3.max(0.0);
        (m3 * self.water_cents_per_m3
            + m3 * self.wastewater_share * self.wastewater_cents_per_m3
            + self.wastewater_fixed_cents_per_year / 12.0)
            .round() as Cents
    }
}

/// A household's (or business's) meters: running totals, and what has been
/// billed so far.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Meters {
    pub kwh_total: f64,
    pub water_m3_total: f64,
    kwh_billed: f64,
    water_m3_billed: f64,
}

/// One month's metered bill.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MeteredBill {
    pub kwh: f64,
    pub water_m3: f64,
    pub electricity: Cents,
    pub water: Cents,
}

impl Meters {
    pub fn record_electricity(&mut self, kwh: f64) {
        if kwh.is_finite() && kwh > 0.0 {
            self.kwh_total += kwh;
        }
    }

    pub fn record_water(&mut self, m3: f64) {
        if m3.is_finite() && m3 > 0.0 {
            self.water_m3_total += m3;
        }
    }

    /// Read the meters and bill what was used since the last reading.
    pub fn read(&mut self, tariffs: &Tariffs) -> MeteredBill {
        let kwh = self.kwh_total - self.kwh_billed;
        let water_m3 = self.water_m3_total - self.water_m3_billed;
        self.kwh_billed = self.kwh_total;
        self.water_m3_billed = self.water_m3_total;
        MeteredBill {
            kwh,
            water_m3,
            electricity: tariffs.electricity_bill(kwh),
            water: tariffs.water_bill(water_m3),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tariffs() -> Tariffs {
        let library = ReferenceLibrary::load(&crate::validation::default_reference_dir()).unwrap();
        Tariffs::from_reference(&library).unwrap()
    }

    #[test]
    fn the_sourced_tariffs_load() {
        let t = tariffs();
        assert!((t.electricity_cents_per_kwh - 42.1).abs() < 1e-9);
        assert!((t.water_cents_per_m3 - 246.0).abs() < 1e-9);
        assert!((t.wastewater_cents_per_m3 - 428.0).abs() < 1e-9);
        assert_eq!(t.wastewater_share, 0.785);
        assert!((t.wastewater_fixed_cents_per_year - 35_590.0).abs() < 1e-9);
    }

    #[test]
    fn a_month_of_the_model_household_costs_what_mbie_says() {
        // MBIE's model household uses 8,000 kWh a year: a month is 666.7 kWh
        // at 42.1 c, $280.67.
        let t = tariffs();
        assert_eq!(t.electricity_bill(8_000.0 / 12.0), 28_067);
        // 15 m3 of water: 15 x 2.46 + 15 x 0.785 x 4.28 + 355.90 / 12
        // = 36.90 + 50.397 + 29.658 = $116.96.
        assert_eq!(t.water_bill(15.0), 11_696);
    }

    #[test]
    fn meters_bill_only_what_was_used_since_the_last_reading() {
        let t = tariffs();
        let mut meters = Meters::default();
        meters.record_electricity(500.0);
        meters.record_water(10.0);
        let first = meters.read(&t);
        assert_eq!((first.kwh, first.water_m3), (500.0, 10.0));
        meters.record_electricity(100.0);
        meters.record_electricity(-5.0);
        let second = meters.read(&t);
        assert_eq!((second.kwh, second.water_m3), (100.0, 0.0));
        assert_eq!(second.electricity, 4_210);
        // No water used still pays the wastewater fixed charge.
        assert_eq!(second.water, 2_966);
    }
}
