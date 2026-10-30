//! The island's oil chain run as one plant (Phase 4b Tasks 4-5).
//!
//! [`IndustryChain`] holds every field and the refinery's product store.
//! Each step every field produces ([`PetroleumField::produce`]) and all the
//! crude in their tanks is refined ([`refine`]); the products wait in the
//! store until something takes them. The chain's material carbon -- the
//! fields' tanks and the store -- changes by exactly the ledger's net flow
//! into [`Reservoir::MaterialCarbon`], so it can be audited with the rest
//! of the island's materials.
//!
//! The refinery is the plan's default skid-mounted unit of about 500
//! barrels a day of nameplate capacity (the owner may change it), and the
//! fields are choked back to what it can take, oil first from the largest
//! field. The plant runs itself: nobody is employed to run it until Task
//! 6's jobs (D58).
//!
//! The fields' gas goes to the gas plant ([`super::gas_plant`]), which
//! burns its own use and stores sales gas and LPG beside the refinery's
//! products.

use serde::{Deserialize, Serialize};

use mk_core::flux::{FluxEntry, FluxKind, Ledger, Reservoir};

use crate::conservation::O2_PER_CARBON;

use super::gas_plant::{process, GasPlantModel, GasStore};
use super::petroleum::{PetroleumField, PetroleumModel, BARREL_M3};
use super::refinery::{products_kgc, refine, Product, RefineryModel, PRODUCTS};
use crate::regional::deposits::MineralDeposit;

const SECONDS_PER_YEAR: f64 = 365.25 * 86_400.0;

/// The plan's default refinery nameplate (barrels of crude a day): the
/// smallest real modular refineries (Phase 4b plan, Defaults). The owner may
/// change it.
pub const DEFAULT_REFINERY_BBL_PER_DAY: f64 = 500.0;

/// The models the chain runs on.
#[derive(Debug, Clone, PartialEq)]
pub struct ChainModels {
    pub petroleum: PetroleumModel,
    pub refinery: RefineryModel,
    pub gas_plant: GasPlantModel,
    /// Nameplate capacity (barrels of crude a day).
    pub refinery_bbl_per_day: f64,
}

impl ChainModels {
    pub fn from_reference(
        library: &crate::validation::ReferenceLibrary,
    ) -> Result<Self, super::petroleum::MissingReference> {
        Ok(Self {
            petroleum: PetroleumModel::from_reference(library)?,
            refinery: RefineryModel::from_reference(library)?,
            gas_plant: GasPlantModel::from_reference(library)?,
            refinery_bbl_per_day: DEFAULT_REFINERY_BBL_PER_DAY,
        })
    }

    pub fn load_default() -> Result<Self, super::petroleum::MissingReference> {
        Ok(Self {
            petroleum: PetroleumModel::load_default()?,
            refinery: RefineryModel::load_default()?,
            gas_plant: GasPlantModel::load_default()?,
            refinery_bbl_per_day: DEFAULT_REFINERY_BBL_PER_DAY,
        })
    }
}

/// The fields and the refinery's store.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct IndustryChain {
    pub fields: Vec<PetroleumField>,
    /// Products in store (t), in [`PRODUCTS`] order.
    pub store_t: [f64; 4],
    /// Their carbon (kg C), kept exactly as each run made it.
    pub store_kgc: [f64; 4],
    /// Crude refined since the island began (m³).
    pub refined_m3: f64,
    /// Carbon of the leaked methane still in the air (kg C); it oxidises to
    /// CO₂ with methane's perturbation lifetime.
    #[serde(default)]
    pub atmos_ch4_kgc: f64,
    /// The gas plant's sales gas and LPG waiting to be taken.
    #[serde(default)]
    pub gas: GasStore,
}

/// What a step did.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ChainStep {
    pub oil_m3: f64,
    pub gas_gj: f64,
    pub leaked_kgc: f64,
    pub refined_m3: f64,
    pub burned_kgc: f64,
    /// Methane carbon oxidised to CO₂ this step (kg C).
    pub oxidised_kgc: f64,
    /// Gas the plant took from the fields (GJ).
    pub plant_gas_gj: f64,
    /// Gas the plant burned to run itself, as carbon (kg C).
    pub plant_burned_kgc: f64,
    pub lpg_t: f64,
    pub sales_gas_gj: f64,
}

fn slot(product: Product) -> usize {
    PRODUCTS.iter().position(|p| *p == product).expect("listed")
}

impl IndustryChain {
    /// One field per petroleum deposit; an island without petroleum has an
    /// empty chain.
    pub fn survey(deposits: &[MineralDeposit], models: &ChainModels, seed: &[u8; 32]) -> Self {
        Self {
            fields: PetroleumField::survey(deposits, &models.petroleum, seed),
            ..Self::default()
        }
    }

    /// Carbon the chain holds as material (kg C): crude and gas in the
    /// fields' tanks, the refinery's products and the gas plant's.
    pub fn material_kgc(&self, models: &ChainModels) -> f64 {
        self.fields
            .iter()
            .map(|f| f.tank_kgc(&models.petroleum))
            .sum::<f64>()
            + self.store_kgc.iter().sum::<f64>()
            + self.gas.kgc()
    }

    /// Run the chain for `dt_s` seconds: produce, send every field's gas
    /// through the gas plant, then refine every field's crude.
    pub fn step(&mut self, dt_s: f64, models: &ChainModels, ledger: &mut Ledger) -> ChainStep {
        let mut out = ChainStep::default();
        if dt_s.is_nan() || dt_s <= 0.0 {
            return out;
        }
        let dt_years = dt_s / SECONDS_PER_YEAR;
        let mut capacity_m3 = models.refinery_bbl_per_day.max(0.0) * BARREL_M3 * dt_s / 86_400.0;
        // The biggest field first, so the refinery runs on one field while
        // it can.
        let mut order: Vec<usize> = (0..self.fields.len()).collect();
        order.sort_by(|&a, &b| {
            self.fields[b]
                .remaining_oil_m3()
                .total_cmp(&self.fields[a].remaining_oil_m3())
                .then(a.cmp(&b))
        });
        for i in order {
            let field = &mut self.fields[i];
            let made = field.produce_up_to(dt_years, capacity_m3, &models.petroleum, ledger);
            capacity_m3 = (capacity_m3 - made.oil_m3).max(0.0);
            out.oil_m3 += made.oil_m3;
            out.gas_gj += made.gas_gj;
            out.leaked_kgc += made.leaked_kgc;
            let (gas_gj, gas_kgc) = field.take_gas(f64::INFINITY, &models.petroleum);
            if gas_gj > 0.0 || gas_kgc > 0.0 {
                let plant = process(&models.gas_plant, gas_gj, gas_kgc, ledger);
                self.gas.add(&plant);
                out.plant_gas_gj += plant.raw_gj;
                out.plant_burned_kgc += plant.burned_kgc;
                out.lpg_t += plant.lpg_t;
                out.sales_gas_gj += plant.sales_gj;
            }
            let crude = field.take_oil(f64::INFINITY);
            if crude <= 0.0 {
                continue;
            }
            let run = refine(&models.refinery, crude, ledger);
            // What the run's products hold, split by product on the run's
            // own carbon scale, so the store keeps every kilogram exactly.
            let total = products_kgc(&models.refinery, &run);
            let raw: Vec<f64> = PRODUCTS
                .iter()
                .map(|p| run.product_t(*p) * models.refinery.carbon_per_kg(*p))
                .collect();
            let raw_sum: f64 = raw.iter().sum();
            for (k, r) in raw.iter().enumerate() {
                self.store_t[k] += run.products_t[k];
                if raw_sum > 0.0 {
                    self.store_kgc[k] += total * r / raw_sum;
                }
            }
            self.refined_m3 += crude;
            out.refined_m3 += crude;
            out.burned_kgc += run.burned_kgc;
        }
        // Methane in the air, last step's and this step's leaks, oxidises:
        // CH4 + 2 O2 -> CO2 + 2 H2O. The oxygen bound into the CO2 is booked;
        // the water's is not, as for respiration (`regional::materials`).
        self.atmos_ch4_kgc += out.leaked_kgc;
        let tau = models.petroleum.methane_lifetime_years;
        if tau > 0.0 && self.atmos_ch4_kgc > 0.0 {
            let oxidised = self.atmos_ch4_kgc * (1.0 - (-dt_years / tau).exp());
            self.atmos_ch4_kgc -= oxidised;
            out.oxidised_kgc = oxidised;
            ledger.push(FluxEntry::new(
                Reservoir::AtmosCH4,
                Reservoir::AtmosCO2,
                oxidised,
                FluxKind::Carbon,
            ));
            ledger.push(FluxEntry::new(
                Reservoir::AtmosO2,
                Reservoir::AtmosCO2,
                oxidised * O2_PER_CARBON,
                FluxKind::Oxygen,
            ));
        }
        out
    }

    /// Take up to `tonnes` of a product out of the store; returns the tonnes
    /// and the carbon (kg C) taken, in proportion to what is held.
    pub fn take(&mut self, product: Product, tonnes: f64) -> (f64, f64) {
        let k = slot(product);
        let taken = tonnes.max(0.0).min(self.store_t[k]);
        if taken <= 0.0 {
            return (0.0, 0.0);
        }
        let kgc = self.store_kgc[k] * taken / self.store_t[k];
        self.store_t[k] -= taken;
        self.store_kgc[k] -= kgc;
        (taken, kgc)
    }
}
