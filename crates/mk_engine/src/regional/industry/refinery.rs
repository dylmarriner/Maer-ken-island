//! A topping refinery (Phase 4b Task 5, core).
//!
//! Crude is distilled into the cuts of a published assay, the Equinor Troll
//! Blend's atmospheric cuts by weight (`troll_atmospheric_cut_yields`), and
//! each cut is a product by where it boils, against New Zealand's fuel
//! specifications (Engine Fuel Specifications Regulations 2011, as amended
//! to 2026):
//!
//! - the light gases (to C4) are refinery gas;
//! - cuts boiling off by petrol's 210 °C final boiling point are naphtha.
//!   Straight-run naphtha is not petrol: its octane needs a reformer,
//!   which this plant does not have (D16);
//! - cuts boiling off by diesel's 360 °C T95 are diesel-range distillate;
//! - the rest is atmospheric residue.
//!
//! The plant runs on its own crude. Refining uses 606 MJ per barrel, the
//! world's volume-weighted average (Jing et al. 2020); the plant burns its
//! refinery gas first and residue for the rest, at IPCC 2006's calorific
//! values (Vol. 2, Table 1.2), so its fuel is about a tenth of the crude,
//! as Downey's 8-12% has it. The burned carbon leaves as CO₂, binding
//! oxygen. Each product carries IPCC's carbon content for its kind (Table
//! 1.3), scaled by one factor so the products and the fuel hold exactly
//! the crude's carbon; the factor is reported and is close to one.

use serde::{Deserialize, Serialize};

use mk_core::flux::{FluxEntry, FluxKind, Ledger, Reservoir};

use super::petroleum::{MissingReference, BARREL_M3};
use crate::conservation::O2_PER_CARBON;
use crate::validation::{ReferenceDomain, ReferenceLibrary, ReferenceTable};

/// What a cut becomes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Product {
    RefineryGas,
    Naphtha,
    Distillate,
    Residue,
}

pub const PRODUCTS: [Product; 4] = [
    Product::RefineryGas,
    Product::Naphtha,
    Product::Distillate,
    Product::Residue,
];

impl Product {
    /// The IPCC 2006 fuel row whose calorific value and carbon content the
    /// product takes.
    fn ipcc_row(self) -> &'static str {
        match self {
            Self::RefineryGas => "Refinery Gas",
            Self::Naphtha => "Naphtha",
            Self::Distillate => "Gas/Diesel Oil",
            Self::Residue => "Residual Fuel Oil",
        }
    }
}

/// One cut of the assay.
#[derive(Debug, Clone, PartialEq)]
pub struct Cut {
    pub label: String,
    /// Where it stops boiling (°C); `None` for the light gases' cut, which
    /// the assay names by carbon number, and infinite for the residue.
    pub end_c: Option<f64>,
    /// Share of the crude by weight.
    pub weight_fraction: f64,
}

/// The end temperature of an assay cut from its label: `IBP-C4` is the
/// light gases; `a-b C` ends at `b`; `C5-65 C` at 65; `370 C-FBP` boils to
/// the end.
fn cut_end_c(label: &str) -> Option<f64> {
    let head = label.split(" (").next().unwrap_or(label).trim();
    if head.starts_with("IBP-C") {
        return None;
    }
    if head.ends_with("FBP") {
        return Some(f64::INFINITY);
    }
    let range = head.trim_end_matches('C').trim();
    range
        .rsplit('-')
        .next()
        .and_then(|end| end.trim().parse::<f64>().ok())
}

/// The reference numbers the refinery runs on.
#[derive(Debug, Clone, PartialEq)]
pub struct RefineryModel {
    pub cuts: Vec<Cut>,
    pub crude_density_t_m3: f64,
    /// Crude's carbon (kg C per kg).
    pub crude_carbon_per_kg: f64,
    pub energy_mj_per_bbl: f64,
    pub petrol_final_boiling_c: f64,
    pub diesel_t95_c: f64,
    /// Per product: net calorific value (GJ/t) and carbon (kg C per kg).
    ncv_gj_t: [f64; 4],
    carbon_per_kg: [f64; 4],
}

impl RefineryModel {
    pub fn from_reference(library: &ReferenceLibrary) -> Result<Self, MissingReference> {
        let item = |key: &str| {
            library
                .item(ReferenceDomain::Industry, key)
                .ok_or_else(|| MissingReference(key.into()))
        };
        let table = |key: &str| -> Result<ReferenceTable, MissingReference> {
            item(key)?
                .table()
                .cloned()
                .ok_or_else(|| MissingReference(key.into()))
        };
        let point = |key: &str| {
            item(key)?
                .point()
                .ok_or_else(|| MissingReference(key.into()))
        };
        let assay = table("troll_atmospheric_cut_yields")?;
        let cuts = assay
            .rows
            .iter()
            .map(|r| {
                let weight = assay
                    .get(&r.label, "yield_wt_pct")
                    .ok_or_else(|| MissingReference(format!("cut {}", r.label)))?;
                Ok(Cut {
                    label: r.label.clone(),
                    end_c: cut_end_c(&r.label),
                    weight_fraction: weight / 100.0,
                })
            })
            .collect::<Result<Vec<_>, MissingReference>>()?;
        let ipcc = table("ipcc_ncv_and_carbon_content")?;
        let cell = |row: &str, column: &str| {
            ipcc.get(row, column)
                .ok_or_else(|| MissingReference(format!("ipcc_ncv_and_carbon_content/{row}")))
        };
        let mut ncv_gj_t = [0.0; 4];
        let mut carbon_per_kg = [0.0; 4];
        for (k, p) in PRODUCTS.iter().enumerate() {
            ncv_gj_t[k] = cell(p.ipcc_row(), "ncv_tj_per_gg")?;
            carbon_per_kg[k] = ncv_gj_t[k] * cell(p.ipcc_row(), "carbon_kg_per_gj")? / 1_000.0;
        }
        Ok(Self {
            cuts,
            crude_density_t_m3: point("troll_density_15c")?,
            crude_carbon_per_kg: cell("Crude Oil", "ncv_tj_per_gg")?
                * cell("Crude Oil", "carbon_kg_per_gj")?
                / 1_000.0,
            energy_mj_per_bbl: point("refinery_energy_use_global")?,
            petrol_final_boiling_c: point("petrol_final_boiling_point_max")?,
            diesel_t95_c: point("diesel_t95_max")?,
            ncv_gj_t,
            carbon_per_kg,
        })
    }

    pub fn load_default() -> Result<Self, MissingReference> {
        let library = ReferenceLibrary::load(&crate::validation::default_reference_dir())
            .map_err(|e| MissingReference(e.to_string()))?;
        Self::from_reference(&library)
    }

    /// IPCC's carbon (kg C per kg) for a product, before a run's scale.
    pub fn carbon_per_kg(&self, product: Product) -> f64 {
        self.carbon_per_kg[PRODUCTS.iter().position(|p| *p == product).unwrap()]
    }

    /// The product a cut is.
    pub fn product_of(&self, cut: &Cut) -> Product {
        match cut.end_c {
            None => Product::RefineryGas,
            Some(end) if end <= self.petrol_final_boiling_c => Product::Naphtha,
            Some(end) if end <= self.diesel_t95_c => Product::Distillate,
            Some(_) => Product::Residue,
        }
    }

    /// Each product's share of the crude by weight, as distilled.
    pub fn distilled_fractions(&self) -> [f64; 4] {
        let mut shares = [0.0; 4];
        let total: f64 = self.cuts.iter().map(|c| c.weight_fraction).sum();
        for cut in &self.cuts {
            let k = PRODUCTS
                .iter()
                .position(|p| *p == self.product_of(cut))
                .expect("every product is listed");
            shares[k] += cut.weight_fraction / total;
        }
        shares
    }
}

/// What one batch of crude made.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RefineryRun {
    pub crude_t: f64,
    /// Product made, by [`PRODUCTS`] order (t).
    pub products_t: [f64; 4],
    /// Burned as the plant's fuel (t): refinery gas, then residue.
    pub fuel_gas_t: f64,
    pub fuel_residue_t: f64,
    /// Carbon burned to CO₂ (kg C).
    pub burned_kgc: f64,
    /// The factor on IPCC's product carbon contents that closes the
    /// crude's carbon.
    pub carbon_scale: f64,
}

impl RefineryRun {
    pub fn product_t(&self, product: Product) -> f64 {
        self.products_t[PRODUCTS.iter().position(|p| *p == product).unwrap()]
    }
}

/// Refine `crude_m3` of crude, booking the fuel's carbon and oxygen in
/// `ledger`. The crude's carbon is already material; only what is burned
/// moves.
pub fn refine(model: &RefineryModel, crude_m3: f64, ledger: &mut Ledger) -> RefineryRun {
    let crude_m3 = crude_m3.max(0.0);
    let crude_t = crude_m3 * model.crude_density_t_m3;
    let distilled = model.distilled_fractions();
    let mut made: [f64; 4] = std::array::from_fn(|k| distilled[k] * crude_t);
    // Fuel: refinery gas first, residue for the rest of the energy.
    let energy_gj = crude_m3 / BARREL_M3 * model.energy_mj_per_bbl / 1_000.0;
    let (gas, residue) = (0, 3);
    let fuel_gas_t = made[gas].min(energy_gj / model.ncv_gj_t[gas]);
    let left_gj = (energy_gj - fuel_gas_t * model.ncv_gj_t[gas]).max(0.0);
    let fuel_residue_t = made[residue].min(left_gj / model.ncv_gj_t[residue]);
    made[gas] -= fuel_gas_t;
    made[residue] -= fuel_residue_t;
    // One factor on every IPCC content so the carbon closes.
    let raw_kgc = |tonnes: f64, k: usize| tonnes * 1_000.0 * model.carbon_per_kg[k];
    let raw_total: f64 = (0..4).map(|k| raw_kgc(made[k], k)).sum::<f64>()
        + raw_kgc(fuel_gas_t, gas)
        + raw_kgc(fuel_residue_t, residue);
    let crude_kgc = crude_t * 1_000.0 * model.crude_carbon_per_kg;
    let carbon_scale = if raw_total > 0.0 {
        crude_kgc / raw_total
    } else {
        1.0
    };
    let burned_kgc = (raw_kgc(fuel_gas_t, gas) + raw_kgc(fuel_residue_t, residue)) * carbon_scale;
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
    RefineryRun {
        crude_t,
        products_t: made,
        fuel_gas_t,
        fuel_residue_t,
        burned_kgc,
        carbon_scale,
    }
}

/// Carbon (kg C) in a run's products, on the run's own scale.
pub fn products_kgc(model: &RefineryModel, run: &RefineryRun) -> f64 {
    (0..4)
        .map(|k| run.products_t[k] * 1_000.0 * model.carbon_per_kg[k])
        .sum::<f64>()
        * run.carbon_scale
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cut_labels_give_their_end_temperatures() {
        assert_eq!(cut_end_c("IBP-C4 (C1-C4 gases incl. LPG range)"), None);
        assert_eq!(cut_end_c("C5-65 C (light naphtha)"), Some(65.0));
        assert_eq!(cut_end_c("150-200 C"), Some(200.0));
        assert_eq!(cut_end_c("370 C-FBP (residue)"), Some(f64::INFINITY));
    }
}
