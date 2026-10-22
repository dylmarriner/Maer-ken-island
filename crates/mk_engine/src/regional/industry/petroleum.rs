//! Oil and gas fields (Phase 4b Task 4).
//!
//! Every Phase-1 petroleum deposit on the island ([`DepositKind::Petroleum`])
//! is a field. Its size is that of one of New Zealand's Taranaki fields --
//! their ultimate recoverable oil and gas, as reported to 1 January 2019
//! (`nz_oil_field_ultimate_remaining_2019`, `nz_gas_field_ultimate_
//! remaining_2019`) -- picked for the deposit from the seed, so the island's
//! fields are drawn from real ones rather than from a fitted curve. Fields
//! with gas but no oil row have no oil, and the reverse.
//!
//! Production declines exponentially at the median annual decline of
//! onshore giant fields (Höök et al. 2009, Table 2: 4.4% a year), so a
//! field produces `D × remaining` a year. Over a step of `dt` it yields
//! `remaining × (1 - exp(-D dt))` exactly, whatever the step's length, and
//! it never produces more than is recoverable.
//!
//! Carbon comes out of the crust. Crude oil holds 20.0 kg C per GJ at
//! 42.3 GJ per tonne, and natural gas 15.3 kg C per GJ (IPCC 2006, Vol. 2,
//! Tables 1.2 and 1.3); a cubic metre of crude weighs the Troll assay's
//! 0.8307 t. A share of the gas produced, 2.3% (Alvarez et al. 2018, the
//! US supply chain), leaks as methane to [`Reservoir::AtmosCH4`]; the rest
//! goes into the field's tanks.
//!
//! Leaked methane oxidises to CO₂ in the air with its perturbation lifetime,
//! 11.8 years (IPCC AR6 WG1, Table 7.15; [`super::chain`]).
//!
//! Not modelled (D58): water cut, reservoir pressure, wells and their
//! blowouts (there is no per-well rate to count wells by), and the leaked
//! methane's warming (the planet's air carries no methane to add it to,
//! and an island's leaks would change a planet-wide concentration by far
//! less than a part per billion). Who drills and runs the field is Task 6;
//! nothing on the island can make a drill rig (D19).

use serde::{Deserialize, Serialize};

use mk_core::flux::{FluxEntry, FluxKind, Ledger, Reservoir};

use crate::regional::deposits::{DepositKind, MineralDeposit};
use crate::regional::terrain::{hash01, terrain_key};
use crate::validation::{ReferenceDomain, ReferenceLibrary};

/// A US barrel: 42 US gallons of 3.785411784 litres, exact by definition.
pub const BARREL_M3: f64 = 42.0 * 3.785_411_784e-3;
/// Joules in a petajoule.
const GJ_PER_PJ: f64 = 1.0e6;

/// One of the fields the island's are drawn from.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FieldAnalogue {
    pub name: String,
    pub ultimate_oil_m3: f64,
    pub ultimate_gas_gj: f64,
}

/// The reference numbers a field runs on.
#[derive(Debug, Clone, PartialEq)]
pub struct PetroleumModel {
    pub analogues: Vec<FieldAnalogue>,
    /// Annual exponential decline rate (fraction per year).
    pub decline_per_year: f64,
    /// Crude oil's density (t/m³).
    pub crude_density_t_m3: f64,
    /// Crude oil's carbon (kg C per kg).
    pub crude_carbon_per_kg: f64,
    /// Natural gas's carbon (kg C per GJ).
    pub gas_carbon_kg_per_gj: f64,
    /// Share of gross gas production lost as methane.
    pub methane_loss_fraction: f64,
    /// Methane's perturbation lifetime in the air (years): the e-folding
    /// time of an emitted pulse (IPCC AR6 WG1, Table 7.15).
    pub methane_lifetime_years: f64,
}

/// Why the model could not be read.
#[derive(Debug, Clone, PartialEq)]
pub struct MissingReference(pub String);

impl std::fmt::Display for MissingReference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "reference item {} is missing", self.0)
    }
}

impl std::error::Error for MissingReference {}

impl PetroleumModel {
    pub fn from_reference(library: &ReferenceLibrary) -> Result<Self, MissingReference> {
        let industry = |key: &str| {
            library
                .item(ReferenceDomain::Industry, key)
                .ok_or_else(|| MissingReference(key.into()))
        };
        let table = |key: &str| {
            industry(key)?
                .table()
                .cloned()
                .ok_or_else(|| MissingReference(key.into()))
        };
        let oil = table("nz_oil_field_ultimate_remaining_2019")?;
        let gas = table("nz_gas_field_ultimate_remaining_2019")?;
        let mut names: Vec<String> = oil
            .rows
            .iter()
            .chain(gas.rows.iter())
            .map(|r| r.label.clone())
            .collect();
        names.sort();
        names.dedup();
        let analogues = names
            .into_iter()
            .map(|name| FieldAnalogue {
                ultimate_oil_m3: oil.get(&name, "ultimate_mmbbl").unwrap_or(0.0) * 1e6 * BARREL_M3,
                ultimate_gas_gj: gas.get(&name, "ultimate_pj").unwrap_or(0.0) * GJ_PER_PJ,
                name,
            })
            .collect();
        let decline = table("giant_field_decline_by_type")?
            .get("land", "median")
            .ok_or_else(|| MissingReference("giant_field_decline_by_type/land/median".into()))?;
        let ipcc = table("ipcc_ncv_and_carbon_content")?;
        let ipcc_cell = |row: &str, column: &str| {
            ipcc.get(row, column)
                .ok_or_else(|| MissingReference(format!("ipcc_ncv_and_carbon_content/{row}")))
        };
        // GJ per tonne (TJ/Gg) x kg C per GJ / 1,000 kg per tonne.
        let crude_carbon_per_kg = ipcc_cell("Crude Oil", "ncv_tj_per_gg")?
            * ipcc_cell("Crude Oil", "carbon_kg_per_gj")?
            / 1_000.0;
        let point = |key: &str| {
            industry(key)?
                .point()
                .ok_or_else(|| MissingReference(key.into()))
        };
        Ok(Self {
            analogues,
            decline_per_year: decline / 100.0,
            crude_density_t_m3: point("troll_density_15c")?,
            crude_carbon_per_kg,
            gas_carbon_kg_per_gj: ipcc_cell("Natural Gas", "carbon_kg_per_gj")?,
            methane_loss_fraction: point("methane_loss_us_supply_chain")? / 100.0,
            methane_lifetime_years: point("methane_perturbation_lifetime_yr")?,
        })
    }

    pub fn load_default() -> Result<Self, MissingReference> {
        let library = ReferenceLibrary::load(&crate::validation::default_reference_dir())
            .map_err(|e| MissingReference(e.to_string()))?;
        Self::from_reference(&library)
    }

    /// Carbon (kg C) in a cubic metre of crude.
    pub fn crude_carbon_kg_per_m3(&self) -> f64 {
        self.crude_density_t_m3 * 1_000.0 * self.crude_carbon_per_kg
    }
}

/// One field on the island.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PetroleumField {
    /// The Phase-1 deposit it is.
    pub deposit_id: u32,
    /// Medium-grid cell.
    pub cell: (usize, usize),
    /// The New Zealand field whose size it has.
    pub analogue: String,
    pub ultimate_oil_m3: f64,
    pub ultimate_gas_gj: f64,
    pub produced_oil_m3: f64,
    pub produced_gas_gj: f64,
    /// Oil and gas produced and not yet taken away.
    pub tank_oil_m3: f64,
    pub tank_gas_gj: f64,
    /// Methane leaked so far (kg C).
    pub leaked_kgc: f64,
}

/// What one step of production gave.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Production {
    pub oil_m3: f64,
    /// Gross gas, leaks included (GJ).
    pub gas_gj: f64,
    /// Carbon of the oil and the kept gas, into the tanks (kg C).
    pub tanked_kgc: f64,
    /// Carbon of the leaked methane (kg C).
    pub leaked_kgc: f64,
}

impl PetroleumField {
    /// The field a petroleum deposit is, sized from one of the model's
    /// analogues picked by the seed. `None` for any other deposit.
    pub fn from_deposit(
        deposit: &MineralDeposit,
        model: &PetroleumModel,
        seed: &[u8; 32],
    ) -> Option<Self> {
        if deposit.kind != DepositKind::Petroleum || model.analogues.is_empty() {
            return None;
        }
        let draw = hash01(
            terrain_key(seed) ^ 0x7065_7472_6f6c,
            u64::from(deposit.id),
            0,
        );
        let n = model.analogues.len();
        let analogue = &model.analogues[((draw * n as f64) as usize).min(n - 1)];
        Some(Self {
            deposit_id: deposit.id,
            cell: deposit.cell,
            analogue: analogue.name.clone(),
            ultimate_oil_m3: analogue.ultimate_oil_m3,
            ultimate_gas_gj: analogue.ultimate_gas_gj,
            produced_oil_m3: 0.0,
            produced_gas_gj: 0.0,
            tank_oil_m3: 0.0,
            tank_gas_gj: 0.0,
            leaked_kgc: 0.0,
        })
    }

    /// Every field on an island, one per petroleum deposit, in deposit
    /// order. Empty if the island has no petroleum.
    pub fn survey(
        deposits: &[MineralDeposit],
        model: &PetroleumModel,
        seed: &[u8; 32],
    ) -> Vec<Self> {
        deposits
            .iter()
            .filter_map(|d| Self::from_deposit(d, model, seed))
            .collect()
    }

    pub fn remaining_oil_m3(&self) -> f64 {
        (self.ultimate_oil_m3 - self.produced_oil_m3).max(0.0)
    }

    pub fn remaining_gas_gj(&self) -> f64 {
        (self.ultimate_gas_gj - self.produced_gas_gj).max(0.0)
    }

    /// Present rates (per year): the decline rate times what is left.
    pub fn rates_per_year(&self, model: &PetroleumModel) -> (f64, f64) {
        (
            model.decline_per_year * self.remaining_oil_m3(),
            model.decline_per_year * self.remaining_gas_gj(),
        )
    }

    /// Carbon held in the tanks (kg C).
    pub fn tank_kgc(&self, model: &PetroleumModel) -> f64 {
        self.tank_oil_m3 * model.crude_carbon_kg_per_m3()
            + self.tank_gas_gj * model.gas_carbon_kg_per_gj
    }

    /// Take up to `m3` of crude out of the tanks, for the refinery; returns
    /// what was taken. Its carbon stays material.
    pub fn take_oil(&mut self, m3: f64) -> f64 {
        let taken = m3.max(0.0).min(self.tank_oil_m3);
        self.tank_oil_m3 -= taken;
        taken
    }

    /// Take up to `gj` of the gas kept at the field (for the gas plant);
    /// returns the GJ and the carbon (kg C) they hold.
    pub fn take_gas(&mut self, gj: f64, model: &PetroleumModel) -> (f64, f64) {
        let taken = gj.max(0.0).min(self.tank_gas_gj);
        self.tank_gas_gj -= taken;
        (taken, taken * model.gas_carbon_kg_per_gj)
    }

    /// Produce for `dt_years`, booking the carbon in `ledger`.
    pub fn produce(
        &mut self,
        dt_years: f64,
        model: &PetroleumModel,
        ledger: &mut Ledger,
    ) -> Production {
        self.produce_up_to(dt_years, f64::INFINITY, model, ledger)
    }

    /// Produce for `dt_years`, but no more than `max_oil_m3` of oil: a
    /// field is choked back to what its offtake can take. Gas comes with
    /// the oil in the same share of what remains; a field with no oil is
    /// not choked.
    pub fn produce_up_to(
        &mut self,
        dt_years: f64,
        max_oil_m3: f64,
        model: &PetroleumModel,
        ledger: &mut Ledger,
    ) -> Production {
        if dt_years.is_nan() || dt_years <= 0.0 {
            return Production::default();
        }
        let mut share = 1.0 - (-model.decline_per_year * dt_years).exp();
        let potential = self.remaining_oil_m3() * share;
        if potential > max_oil_m3.max(0.0) && potential > 0.0 {
            share *= max_oil_m3.max(0.0) / potential;
        }
        let oil_m3 = self.remaining_oil_m3() * share;
        let gas_gj = self.remaining_gas_gj() * share;
        let leaked_gj = gas_gj * model.methane_loss_fraction;
        let kept_gj = gas_gj - leaked_gj;
        self.produced_oil_m3 += oil_m3;
        self.produced_gas_gj += gas_gj;
        self.tank_oil_m3 += oil_m3;
        self.tank_gas_gj += kept_gj;
        let tanked_kgc =
            oil_m3 * model.crude_carbon_kg_per_m3() + kept_gj * model.gas_carbon_kg_per_gj;
        let leaked_kgc = leaked_gj * model.gas_carbon_kg_per_gj;
        self.leaked_kgc += leaked_kgc;
        for (to, kgc) in [
            (Reservoir::MaterialCarbon, tanked_kgc),
            (Reservoir::AtmosCH4, leaked_kgc),
        ] {
            if kgc > 0.0 {
                ledger.push(FluxEntry::new(
                    Reservoir::CrustCarbon,
                    to,
                    kgc,
                    FluxKind::Carbon,
                ));
            }
        }
        Production {
            oil_m3,
            gas_gj,
            tanked_kgc,
            leaked_kgc,
        }
    }
}
