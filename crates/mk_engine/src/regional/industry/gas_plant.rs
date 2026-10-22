//! The gas plant (Phase 4b Task 5).
//!
//! The fields' gas goes to a plant that burns part of it to run itself and
//! splits the rest into sales gas and LPG. Its own use is US gas plants'
//! fuel as a share of marketed gas, 1.17% in 2024 (EIA Natural Gas Annual
//! 2024, Tables 1 and 14; `us_gas_plant_fuel_share_of_marketed`). The LPG it
//! extracts follows a New Zealand plant, Kupe's 90,000 t of LPG for every
//! 20 PJ of sales gas (Offshore Technology; `kupe_annual_products`), at
//! LPG's 47.3 GJ a tonne (IPCC 2006, Vol. 2, Table 1.2).
//!
//! Carbon closes as the refinery's does: each stream takes its IPCC carbon
//! content (natural gas 15.3, LPG 17.2 kg C per GJ), scaled by one factor so
//! the streams hold exactly the raw gas's carbon. The plant fuel's carbon
//! goes to the air as CO₂.
//!
//! Not modelled (D16): the raw gas's CO₂ and nitrogen (the fields count
//! their gas in GJ, so Kapuni's 42.6% CO₂ never enters the plant), ethane
//! as a product (it stays in the sales gas), and condensate, which the
//! fields already count with their oil. The plant has no nameplate: it
//! takes all the gas the fields make.

use serde::{Deserialize, Serialize};

use mk_core::flux::{FluxEntry, FluxKind, Ledger, Reservoir};

use crate::conservation::O2_PER_CARBON;
use crate::validation::{ReferenceDomain, ReferenceLibrary};

use super::petroleum::MissingReference;

/// GJ in a PJ.
const GJ_PER_PJ: f64 = 1e6;

/// The plant's sourced rates.
#[derive(Debug, Clone, PartialEq)]
pub struct GasPlantModel {
    /// Gas burned by the plant, as a share of the gas it takes.
    pub fuel_share: f64,
    /// LPG extracted for every GJ of sales gas (t/GJ).
    pub lpg_t_per_sales_gj: f64,
    /// LPG's net calorific value (GJ/t).
    pub lpg_ncv_gj_t: f64,
    /// Natural gas's carbon (kg C per GJ).
    pub gas_carbon_kg_per_gj: f64,
    /// LPG's carbon (kg C per GJ).
    pub lpg_carbon_kg_per_gj: f64,
}

impl GasPlantModel {
    pub fn from_reference(library: &ReferenceLibrary) -> Result<Self, MissingReference> {
        let industry = |key: &str| {
            library
                .item(ReferenceDomain::Industry, key)
                .ok_or_else(|| MissingReference(key.into()))
        };
        let cell = |key: &str, row: &str, column: &str| {
            industry(key)?
                .table()
                .and_then(|t| t.get(row, column))
                .ok_or_else(|| MissingReference(format!("{key}/{row}/{column}")))
        };
        let fuel_pct = industry("us_gas_plant_fuel_share_of_marketed")?
            .point()
            .ok_or_else(|| MissingReference("us_gas_plant_fuel_share_of_marketed".into()))?;
        let kupe_sales_pj = cell(
            "kupe_annual_products",
            "sales gas (PJ/yr)",
            "annual_quantity",
        )?;
        let kupe_lpg_t = cell("kupe_annual_products", "LPG (t/yr)", "annual_quantity")?;
        let ipcc = "ipcc_ncv_and_carbon_content";
        Ok(Self {
            fuel_share: fuel_pct / 100.0,
            lpg_t_per_sales_gj: kupe_lpg_t / (kupe_sales_pj * GJ_PER_PJ),
            lpg_ncv_gj_t: cell(ipcc, "Liquefied Petroleum Gases", "ncv_tj_per_gg")?,
            gas_carbon_kg_per_gj: cell(ipcc, "Natural Gas", "carbon_kg_per_gj")?,
            lpg_carbon_kg_per_gj: cell(ipcc, "Liquefied Petroleum Gases", "carbon_kg_per_gj")?,
        })
    }

    pub fn load_default() -> Result<Self, MissingReference> {
        let library = ReferenceLibrary::load(&crate::validation::default_reference_dir())
            .map_err(|e| MissingReference(e.to_string()))?;
        Self::from_reference(&library)
    }
}

/// What one run of the plant made.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct GasPlantRun {
    /// Raw gas taken (GJ).
    pub raw_gj: f64,
    /// Burned to run the plant (GJ).
    pub fuel_gj: f64,
    pub sales_gj: f64,
    pub lpg_t: f64,
    pub burned_kgc: f64,
    pub sales_kgc: f64,
    pub lpg_kgc: f64,
}

/// Process `raw_gj` of gas holding `raw_kgc` of carbon, booking the plant
/// fuel's carbon to the air.
pub fn process(
    model: &GasPlantModel,
    raw_gj: f64,
    raw_kgc: f64,
    ledger: &mut Ledger,
) -> GasPlantRun {
    if raw_gj.is_nan() || raw_gj <= 0.0 {
        // Carbon taken with no energy (a rounding remnant) stays in the
        // sales gas, so the plant never loses carbon.
        return GasPlantRun {
            sales_kgc: raw_kgc.max(0.0),
            ..GasPlantRun::default()
        };
    }
    let fuel_gj = raw_gj * model.fuel_share;
    let rest = raw_gj - fuel_gj;
    // Each GJ of sales gas comes with `k` GJ of LPG.
    let k = model.lpg_t_per_sales_gj * model.lpg_ncv_gj_t;
    let sales_gj = rest / (1.0 + k);
    let lpg_gj = rest - sales_gj;
    let raw = [
        fuel_gj * model.gas_carbon_kg_per_gj,
        sales_gj * model.gas_carbon_kg_per_gj,
        lpg_gj * model.lpg_carbon_kg_per_gj,
    ];
    let total: f64 = raw.iter().sum();
    let scale = if total > 0.0 {
        raw_kgc.max(0.0) / total
    } else {
        0.0
    };
    let burned_kgc = raw[0] * scale;
    let sales_kgc = raw[1] * scale;
    let lpg_kgc = raw_kgc.max(0.0) - burned_kgc - sales_kgc;
    if burned_kgc > 0.0 {
        ledger.push(FluxEntry::new(
            Reservoir::MaterialCarbon,
            Reservoir::AtmosCO2,
            burned_kgc,
            FluxKind::Carbon,
        ));
        ledger.push(FluxEntry::new(
            Reservoir::AtmosO2,
            Reservoir::AtmosCO2,
            burned_kgc * O2_PER_CARBON,
            FluxKind::Oxygen,
        ));
    }
    GasPlantRun {
        raw_gj,
        fuel_gj,
        sales_gj,
        lpg_t: lpg_gj / model.lpg_ncv_gj_t,
        burned_kgc,
        sales_kgc,
        lpg_kgc,
    }
}

/// The plant's products waiting to be taken.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct GasStore {
    pub sales_gj: f64,
    pub sales_kgc: f64,
    pub lpg_t: f64,
    pub lpg_kgc: f64,
}

impl GasStore {
    pub fn add(&mut self, run: &GasPlantRun) {
        self.sales_gj += run.sales_gj;
        self.sales_kgc += run.sales_kgc;
        self.lpg_t += run.lpg_t;
        self.lpg_kgc += run.lpg_kgc;
    }

    pub fn kgc(&self) -> f64 {
        self.sales_kgc + self.lpg_kgc
    }

    /// Take up to `tonnes` of LPG; returns the tonnes and their carbon.
    pub fn take_lpg(&mut self, tonnes: f64) -> (f64, f64) {
        let taken = tonnes.max(0.0).min(self.lpg_t);
        if taken <= 0.0 {
            return (0.0, 0.0);
        }
        let kgc = self.lpg_kgc * taken / self.lpg_t;
        self.lpg_t -= taken;
        self.lpg_kgc -= kgc;
        (taken, kgc)
    }

    /// Take up to `gj` of sales gas; returns the GJ and their carbon.
    pub fn take_sales_gas(&mut self, gj: f64) -> (f64, f64) {
        let taken = gj.max(0.0).min(self.sales_gj);
        if taken <= 0.0 {
            return (0.0, 0.0);
        }
        let kgc = self.sales_kgc * taken / self.sales_gj;
        self.sales_gj -= taken;
        self.sales_kgc -= kgc;
        (taken, kgc)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_plant_burns_its_own_use_and_makes_kupe_s_lpg() {
        let model = GasPlantModel::load_default().unwrap();
        assert!((model.fuel_share - 0.0117).abs() < 1e-12);
        assert!((model.lpg_t_per_sales_gj - 0.0045).abs() < 1e-12);
        let raw_gj = 1_000.0;
        let raw_kgc = raw_gj * model.gas_carbon_kg_per_gj;
        let mut ledger = Ledger::new();
        let run = process(&model, raw_gj, raw_kgc, &mut ledger);
        assert!((run.fuel_gj - 11.7).abs() < 1e-9);
        // Kupe's ratio: 4,500 t of LPG per PJ of sales gas.
        assert!((run.lpg_t / run.sales_gj - 0.0045).abs() < 1e-12);
        let energy = run.fuel_gj + run.sales_gj + run.lpg_t * model.lpg_ncv_gj_t;
        assert!((energy - raw_gj).abs() < 1e-9);
        let carbon = run.burned_kgc + run.sales_kgc + run.lpg_kgc;
        assert!((carbon - raw_kgc).abs() < 1e-9);
        let to_air: f64 = ledger
            .entries()
            .iter()
            .filter(|e| e.kind == FluxKind::Carbon && e.sink == Reservoir::AtmosCO2)
            .map(|e| e.amount)
            .sum();
        assert!((to_air - run.burned_kgc).abs() < 1e-12);
    }

    #[test]
    fn carbon_with_no_energy_is_kept_in_the_sales_gas() {
        let model = GasPlantModel::load_default().unwrap();
        let mut ledger = Ledger::new();
        let run = process(&model, 0.0, 2.5, &mut ledger);
        assert_eq!(run.sales_kgc, 2.5);
        assert_eq!(run.burned_kgc + run.lpg_kgc, 0.0);
        assert!(ledger.entries().is_empty());
    }

    #[test]
    fn the_store_gives_up_its_carbon_with_what_is_taken() {
        let mut store = GasStore {
            sales_gj: 100.0,
            sales_kgc: 1_530.0,
            lpg_t: 2.0,
            lpg_kgc: 1_600.0,
        };
        assert_eq!(store.take_lpg(0.5), (0.5, 400.0));
        assert_eq!(store.take_sales_gas(1_000.0), (100.0, 1_530.0));
        assert_eq!(store.kgc(), 1_200.0);
    }
}
