//! A softwood sawmill (Phase 4b Task 7b, timber chain).
//!
//! Logs go in by volume of roundwood and come out as sawnwood and residues.
//! The sawnwood share is New Zealand's own: 1.81 m³ of coniferous roundwood
//! per m³ of sawnwood (FAO/ITTO/UN 2020, section 4.4.1). New Zealand
//! reports no material balance, so the rest of the log is shared out in the
//! proportions of North America's coniferous balance from the same table
//! (chips and slabs 33, sawdust 9, shavings 8, shrinkage 3), a stand-in until
//! a New Zealand balance is cited. Every m³ of log is accounted for.

use serde::{Deserialize, Serialize};

use super::petroleum::MissingReference;
use crate::validation::{ReferenceDomain, ReferenceLibrary};

/// What a sawn log becomes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Output {
    Sawnwood,
    ChipsSlabs,
    Sawdust,
    Shavings,
    /// Volume lost as the sawnwood dries; the wood is still there, its
    /// water is not.
    Shrinkage,
}

pub const OUTPUTS: [Output; 5] = [
    Output::Sawnwood,
    Output::ChipsSlabs,
    Output::Sawdust,
    Output::Shavings,
    Output::Shrinkage,
];

impl Output {
    fn balance_row(self) -> &'static str {
        match self {
            Self::Sawnwood => "sawnwood",
            Self::ChipsSlabs => "chips_slabs",
            Self::Sawdust => "sawdust",
            Self::Shavings => "shavings",
            Self::Shrinkage => "shrinkage_loss",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SawmillModel {
    /// m³ of roundwood per m³ of sawnwood.
    pub roundwood_per_sawnwood: f64,
    /// Each output's share of the log, in [`OUTPUTS`] order; sums to one.
    pub shares: [f64; 5],
}

impl SawmillModel {
    pub fn from_reference(library: &ReferenceLibrary) -> Result<Self, MissingReference> {
        let item = |key: &str| {
            library
                .item(ReferenceDomain::Industry, key)
                .ok_or_else(|| MissingReference(key.into()))
        };
        let roundwood_per_sawnwood = item("softwood_roundwood_per_sawnwood_nz")?
            .point()
            .ok_or_else(|| MissingReference("softwood_roundwood_per_sawnwood_nz".into()))?;
        const BALANCE: &str = "softwood_sawmill_material_balance_north_america";
        let balance = item(BALANCE)?
            .table()
            .ok_or_else(|| MissingReference(BALANCE.into()))?;
        let mut residue = [0.0; 5];
        for (k, o) in OUTPUTS.iter().enumerate().skip(1) {
            residue[k] = balance
                .get(o.balance_row(), "percent")
                .ok_or_else(|| MissingReference(format!("{BALANCE}/{}", o.balance_row())))?;
        }
        let sawn = 1.0 / roundwood_per_sawnwood;
        let residue_total: f64 = residue.iter().sum();
        let mut shares = residue.map(|r| (1.0 - sawn) * r / residue_total);
        shares[0] = sawn;
        Ok(Self {
            roundwood_per_sawnwood,
            shares,
        })
    }

    pub fn load_default() -> Result<Self, MissingReference> {
        let library = ReferenceLibrary::load(&crate::validation::default_reference_dir())
            .map_err(|e| MissingReference(e.to_string()))?;
        Self::from_reference(&library)
    }

    /// m³ of each output, in [`OUTPUTS`] order, from `log_m3` of roundwood.
    pub fn saw(&self, log_m3: f64) -> [f64; 5] {
        self.shares.map(|s| s * log_m3)
    }

    /// Tonnes of carbon in each output, in [`OUTPUTS`] order, from logs
    /// holding `log_carbon_t`. Shrinkage is water leaving the sawnwood, so
    /// its wood's carbon stays in the sawnwood and its own entry is zero.
    pub fn carbon(&self, log_carbon_t: f64) -> [f64; 5] {
        let mut c = self.shares.map(|s| s * log_carbon_t);
        c[0] += c[4];
        c[4] = 0.0;
        c
    }

    /// m³ of roundwood needed for `sawnwood_m3` of sawnwood.
    pub fn logs_for(&self, sawnwood_m3: f64) -> f64 {
        sawnwood_m3 * self.roundwood_per_sawnwood
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_log_is_all_accounted_for_at_new_zealands_recovery() {
        let mill = SawmillModel::load_default().expect("the pack loads");
        assert_eq!(mill.roundwood_per_sawnwood, 1.81);
        let out = mill.saw(10.0);
        assert!((out.iter().sum::<f64>() - 10.0).abs() < 1e-12);
        assert!((out[0] - 10.0 / 1.81).abs() < 1e-12);
        // The residues keep North America's 33 : 9 : 8 : 3.
        assert!((out[1] / out[2] - 33.0 / 9.0).abs() < 1e-12);
        assert!((out[3] / out[4] - 8.0 / 3.0).abs() < 1e-12);
        assert!((mill.logs_for(out[0]) - 10.0).abs() < 1e-12);
        let carbon = mill.carbon(2.0);
        assert!((carbon.iter().sum::<f64>() - 2.0).abs() < 1e-12);
        assert_eq!(carbon[4], 0.0);
    }
}
