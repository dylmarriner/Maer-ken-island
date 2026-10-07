use crate::math::{FloatExt, TICK_RESIDUAL_LIMIT};
use serde::{Deserialize, Serialize};
/**
 * Purpose
 * - Double-entry ledger of energy and mass transfers between reservoirs.
 *
 * What each check proves
 * - Every `FluxEntry` debits its source and credits its sink by the same
 *   amount, so the ledger can never create or destroy anything by itself.
 *   A check that only reads the ledger therefore cannot detect a model
 *   that changes a stock without recording the transfer.
 * - `Ledger::audit_against_stocks` is the real conservation check: for
 *   every interior reservoir it compares the measured change in stock over
 *   the tick with the net inflow the ledger recorded.
 * - `Ledger::audit` is the stricter pass-through check: it requires every
 *   interior reservoir's inflow to equal its outflow within the tick.
 * - `Ledger::net_imbalance` is the net exchange of the interior with the
 *   boundary reservoirs, i.e. the ledger-implied change in total interior
 *   storage. It is not a conservation residual.
 *
 * Amounts are f64 in the flux kind's SI unit (joules for energy, kilograms
 * for mass). Tolerances default to TICK_RESIDUAL_LIMIT.
 */
use std::collections::BTreeMap;
use std::fmt;

/// Double-entry record of every flux transfer between reservoirs in a tick.
///
/// Every transfer the simulation makes must be recorded here, and the
/// ledger is cleared after the tick's audit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ledger {
    pub entries: Vec<FluxEntry>,
}

impl Ledger {
    /// Create new empty ledger
    ///
    /// # Returns
    ///
    /// New Ledger with no entries
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Summarize all flux transfers in the ledger
    pub fn summarize(&self) -> LedgerSummary {
        let mut summary = LedgerSummary::default();
        for entry in &self.entries {
            match entry.kind {
                FluxKind::Energy => summary.total_flux.energy += entry.amount,
                FluxKind::Water => summary.total_flux.water += entry.amount,
                FluxKind::Carbon => summary.total_flux.carbon += entry.amount,
                FluxKind::Oxygen => summary.total_flux.oxygen += entry.amount,
                FluxKind::Nitrogen => summary.total_flux.nitrogen += entry.amount,
                FluxKind::Phosphorus => summary.total_flux.phosphorus += entry.amount,
            }
        }
        summary
    }

    /// Add flux entry to ledger
    ///
    /// Records a transfer from source to sink.
    ///
    /// # Arguments
    ///
    /// * `entry` - Flux entry to record
    pub fn push(&mut self, entry: FluxEntry) {
        self.entries.push(entry);
    }

    /// Audit ledger for conservation violations
    ///
    /// Checks that all flux kinds are balanced.
    ///
    /// # Arguments
    ///
    /// * `policy` - Tolerance policy for numerical precision
    ///
    /// # Returns
    ///
    /// Ok if balanced, Err with imbalance details
    pub fn audit(&self, policy: TolerancePolicy) -> Result<(), AuditError> {
        use std::collections::HashMap;

        // Track per-reservoir balance: inflows minus outflows
        let mut reservoir_balances: HashMap<(crate::flux::Reservoir, FluxKind), f64> =
            HashMap::new();

        for entry in &self.entries {
            // Add to sink (inflow)
            *reservoir_balances
                .entry((entry.sink, entry.kind))
                .or_insert(0.0) += entry.amount;
            // Subtract from source (outflow)
            *reservoir_balances
                .entry((entry.source, entry.kind))
                .or_insert(0.0) -= entry.amount;
        }

        // Check in a stable order so multiple violations report the same one
        // across processes and runs.
        let mut ordered: Vec<_> = reservoir_balances.into_iter().collect();
        ordered.sort_by_key(|((reservoir, kind), _)| (reservoir.as_str(), kind.as_str()));
        for ((reservoir, kind), balance) in ordered {
            let tolerance = policy.tolerance(kind);
            if !reservoir.is_boundary() && !balance.is_approx_zero(tolerance) {
                return Err(AuditError::ImbalanceExceeded {
                    kind,
                    balance,
                    tolerance,
                });
            }
        }

        Ok(())
    }

    /// Clear all entries
    ///
    /// MUST clear after successful audit.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Get number of entries
    ///
    /// # Returns
    ///
    /// Number of flux entries in ledger
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if ledger is empty
    ///
    /// # Returns
    ///
    /// True if no entries, false otherwise
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Get entries as slice (read-only)
    ///
    /// # Returns
    ///
    /// Slice of flux entries
    pub fn entries(&self) -> &[FluxEntry] {
        &self.entries
    }

    /// Total amount transferred for a flux kind: the sum of every entry's
    /// amount, regardless of direction. This is throughput, not a balance;
    /// it grows with activity even when everything is conserved.
    pub fn balance(&self, kind: FluxKind) -> f64 {
        self.entries
            .iter()
            .filter(|entry| entry.kind == kind)
            .map(|entry| entry.amount)
            .sum()
    }

    /// Net exchange of the interior (non-boundary) reservoirs with the
    /// boundary for a flux kind: inflows from boundary reservoirs minus
    /// outflows to them.
    ///
    /// Transfers between two interior reservoirs cancel out here by
    /// construction, so this is the ledger-implied change in total interior
    /// storage over the tick. It is zero only when the interior neither
    /// gained nor lost anything across the boundary. It cannot reveal a
    /// conservation error, because the ledger itself is always balanced; use
    /// [`Ledger::audit_against_stocks`] for that.
    pub fn net_imbalance(&self, kind: FluxKind) -> f64 {
        use std::collections::HashMap;

        let mut balances: HashMap<Reservoir, f64> = HashMap::new();
        for entry in self.entries.iter().filter(|e| e.kind == kind) {
            *balances.entry(entry.sink).or_insert(0.0) += entry.amount;
            *balances.entry(entry.source).or_insert(0.0) -= entry.amount;
        }

        // Sum in a stable order (by reservoir name) rather than raw HashMap
        // iteration order (randomized per-process): float addition is
        // non-associative, so the final sum's bit pattern would otherwise
        // vary run-to-run even with identical input, and this value is
        // persisted into audit_trail's closure fields.
        let mut ordered: Vec<_> = balances
            .into_iter()
            .filter(|(reservoir, _)| !reservoir.is_boundary())
            .collect();
        ordered.sort_by_key(|(reservoir, _)| reservoir.as_str());

        ordered.into_iter().map(|(_, balance)| balance).sum()
    }

    /// Net recorded inflow (inflows minus outflows) for every reservoir that
    /// appears in the ledger for `kind`, in a stable order.
    pub fn net_flow_by_reservoir(&self, kind: FluxKind) -> BTreeMap<Reservoir, f64> {
        let mut flows = BTreeMap::new();
        for entry in self.entries.iter().filter(|e| e.kind == kind) {
            *flows.entry(entry.sink).or_insert(0.0) += entry.amount;
            *flows.entry(entry.source).or_insert(0.0) -= entry.amount;
        }
        flows
    }

    /// Check conservation against measured stocks.
    ///
    /// For each interior (non-boundary) reservoir, the measured change in
    /// stock over the tick must equal the net inflow the ledger recorded,
    /// within `policy`. A reservoir that the ledger moved flux through but
    /// whose stock was not measured is an error, because its closure can't
    /// be checked. So is a stock that changed with no recorded transfer.
    ///
    /// # Arguments
    ///
    /// * `before` - Stocks at the start of the tick
    /// * `after` - Stocks at the end of the tick
    /// * `policy` - Absolute tolerance per flux kind
    pub fn audit_against_stocks(
        &self,
        before: &ReservoirStocks,
        after: &ReservoirStocks,
        policy: &TolerancePolicy,
    ) -> Result<(), StockAuditError> {
        self.audit_kinds_against_stocks(FluxKind::all(), before, after, policy)
    }

    /// [`Ledger::audit_against_stocks`] for the given flux kinds only, for a
    /// caller that measures stocks for some kinds and not others.
    pub fn audit_kinds_against_stocks(
        &self,
        kinds: &[FluxKind],
        before: &ReservoirStocks,
        after: &ReservoirStocks,
        policy: &TolerancePolicy,
    ) -> Result<(), StockAuditError> {
        for &kind in kinds {
            let flows = self.net_flow_by_reservoir(kind);
            let tolerance = policy.tolerance(kind);

            let mut reservoirs: Vec<Reservoir> = flows.keys().copied().collect();
            reservoirs.extend(before.reservoirs(kind));
            reservoirs.extend(after.reservoirs(kind));
            reservoirs.sort();
            reservoirs.dedup();

            for reservoir in reservoirs {
                if reservoir.is_boundary() {
                    continue;
                }
                let (Some(start), Some(end)) =
                    (before.get(reservoir, kind), after.get(reservoir, kind))
                else {
                    return Err(StockAuditError::UnmeasuredReservoir { reservoir, kind });
                };
                let recorded = flows.get(&reservoir).copied().unwrap_or(0.0);
                let measured = end - start;
                let residual = measured - recorded;
                if !residual.is_finite() || residual.abs() > tolerance {
                    return Err(StockAuditError::StockMismatch {
                        reservoir,
                        kind,
                        measured_change: measured,
                        recorded_net_inflow: recorded,
                        tolerance,
                    });
                }
            }
        }
        Ok(())
    }

    /// Per-reservoir conservation residual for `kind`: measured stock change
    /// minus the net inflow the ledger recorded, for every interior reservoir
    /// that the ledger moved flux through or that has a measured stock.
    ///
    /// A conserved tick gives residuals of (floating-point) zero. Unlike
    /// [`Ledger::audit_against_stocks`], this returns the magnitudes, so a
    /// caller can apply a tolerance relative to the size of each stock.
    ///
    /// # Errors
    ///
    /// [`StockAuditError::UnmeasuredReservoir`] if an interior reservoir in
    /// play has no measured stock at one end of the tick.
    pub fn stock_residuals(
        &self,
        kind: FluxKind,
        before: &ReservoirStocks,
        after: &ReservoirStocks,
    ) -> Result<BTreeMap<Reservoir, f64>, StockAuditError> {
        let flows = self.net_flow_by_reservoir(kind);
        let mut reservoirs: Vec<Reservoir> = flows.keys().copied().collect();
        reservoirs.extend(before.reservoirs(kind));
        reservoirs.extend(after.reservoirs(kind));
        reservoirs.sort();
        reservoirs.dedup();

        let mut residuals = BTreeMap::new();
        for reservoir in reservoirs {
            if reservoir.is_boundary() {
                continue;
            }
            let (Some(start), Some(end)) =
                (before.get(reservoir, kind), after.get(reservoir, kind))
            else {
                return Err(StockAuditError::UnmeasuredReservoir { reservoir, kind });
            };
            let recorded = flows.get(&reservoir).copied().unwrap_or(0.0);
            residuals.insert(reservoir, (end - start) - recorded);
        }
        Ok(residuals)
    }

    /// Compute net closure residuals from the ledger
    pub fn closure_residuals(&self) -> ClosureResiduals {
        ClosureResiduals {
            energy: self.net_imbalance(FluxKind::Energy),
            water: self.net_imbalance(FluxKind::Water),
            carbon: self.net_imbalance(FluxKind::Carbon),
        }
    }
}

impl Default for Ledger {
    fn default() -> Self {
        Self::new()
    }
}

/// Single flux transfer from a source reservoir to a sink reservoir.
///
/// `amount` is in the flux kind's SI unit (joules or kilograms).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct FluxEntry {
    pub source: Reservoir,
    pub sink: Reservoir,
    pub amount: f64,
    pub kind: FluxKind,
}

impl FluxEntry {
    /// Create new flux entry
    ///
    /// # Arguments
    ///
    /// * `source` - Source reservoir (where flux comes from)
    /// * `sink` - Sink reservoir (where flux goes to)
    /// * `amount` - Transfer amount in the flux kind's SI unit
    /// * `kind` - Type of flux
    ///
    /// # Returns
    ///
    /// New FluxEntry
    pub fn new(source: Reservoir, sink: Reservoir, amount: f64, kind: FluxKind) -> Self {
        Self {
            source,
            sink,
            amount,
            kind,
        }
    }

    /// Get source reservoir
    pub fn source(&self) -> Reservoir {
        self.source
    }

    /// Get sink reservoir
    pub fn sink(&self) -> Reservoir {
        self.sink
    }

    /// Get amount
    pub fn amount(&self) -> f64 {
        self.amount
    }

    /// Get flux kind
    pub fn kind(&self) -> FluxKind {
        self.kind
    }
}

/// Conservation compartments for flux tracking
///
/// MUST use generic names (not "MarrKenaOceanHeat").
/// MAY parameterize by PlanetId if multiple planets exist (future-proofing).
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub enum Reservoir {
    // Energy reservoirs
    TOAInsolation,
    AtmosEnergy,
    SurfaceEnergy,
    OceanHeat,
    TidalHeat,
    VolcanicHeat,
    SpaceRadiation,
    /// Kinetic energy an impactor brings from outside the planet; a
    /// boundary, like `TOAInsolation`.
    ImpactorKinetic,

    // Water reservoirs
    Atmosphere,
    SoilWater,
    Groundwater,
    Rivers,
    Ocean,

    // Carbon reservoirs
    AtmosCO2,
    BiomassCarbon,
    DetritusCarbon,
    OceanDIC,
    CrustCarbon,

    // Oxygen reservoirs
    AtmosO2,
    BiomassOxygen,
    OceanDO,

    // Nitrogen reservoirs
    AtmosN2,
    BiomassNitrogen,
    SoilNitrogen,

    // Phosphorus reservoirs
    BiomassPhosphorus,
    SoilPhosphorus,
    OceanPhosphorus,

    /// External operator intervention (see `mk_engine::interventions`).
    ///
    /// A boundary reservoir. Interventions inject matter/energy that did not
    /// come from anywhere inside the simulated system, so recording them as
    /// flowing *from* this reservoir keeps [`Ledger::audit`] closing while
    /// leaving the inflow visible and attributable in the ledger, rather
    /// than either breaking closure or going unrecorded.
    OperatorIntervention,

    // Nutrients held in dead organic matter. Declared after
    // `OperatorIntervention` so the variant indices of every earlier
    // reservoir, and so any serialized ledger, are unchanged.
    DetritusNitrogen,
    DetritusPhosphorus,

    /// Phosphorus held in rock and marine sediment: the boundary that
    /// weathering releases soil phosphorus from and burial returns it to,
    /// as [`Reservoir::CrustCarbon`] is for carbon.
    CrustPhosphorus,

    /// Standing land water and the runoff routed between cells.
    SurfaceWater,

    // Materials and bodies (Phase 3 Task 3). Declared last so every earlier
    // variant's index, and any serialized ledger, is unchanged.
    /// Carbon in gathered, crafted and built materials (wood, charcoal,
    /// coal, limestone, food, structures): carbon taken out of living
    /// biomass or the crust and not yet returned to the air or the soil.
    MaterialCarbon,
    /// Water held in gathered materials and in human bodies.
    MaterialWater,
    /// Carbon held in human bodies: eaten food in, respiration out.
    HumanCarbon,
}

impl Reservoir {
    /// Check if this reservoir is a boundary (source or sink)
    ///
    /// Boundary reservoirs are where material enters or exits the system.
    /// Intermediate reservoirs should never accumulate material persistently.
    pub fn is_boundary(&self) -> bool {
        matches!(
            self,
            // Energy boundaries
            Reservoir::TOAInsolation
                | Reservoir::SpaceRadiation
                // Heat from the planet's interior and from tidal forcing by
                // the moons enters the surface system from outside it
                | Reservoir::VolcanicHeat
                | Reservoir::TidalHeat
                | Reservoir::ImpactorKinetic
                // Water boundaries (atmosphere is a boundary for water cycle)
                | Reservoir::Ocean
                | Reservoir::Rivers
                | Reservoir::Atmosphere
                // Carbon boundaries
                | Reservoir::CrustCarbon
                // Nutrient boundaries: atmospheric N₂ (fixation draws from
                // it, denitrification returns to it) and rock phosphorus
                // (weathering and burial)
                | Reservoir::AtmosN2
                | Reservoir::CrustPhosphorus
                // External operator interventions enter at the boundary
                | Reservoir::OperatorIntervention
        )
    }

    /// Get all reservoirs
    ///
    /// # Returns
    ///
    /// Array of all reservoir identifiers
    pub fn all() -> &'static [Reservoir] {
        use Reservoir::*;
        &[
            // Energy
            TOAInsolation,
            AtmosEnergy,
            SurfaceEnergy,
            OceanHeat,
            TidalHeat,
            VolcanicHeat,
            SpaceRadiation,
            ImpactorKinetic,
            // Water
            Atmosphere,
            SoilWater,
            Groundwater,
            Rivers,
            Ocean,
            // Carbon
            AtmosCO2,
            BiomassCarbon,
            DetritusCarbon,
            OceanDIC,
            CrustCarbon,
            // Oxygen
            AtmosO2,
            BiomassOxygen,
            OceanDO,
            // Nitrogen
            AtmosN2,
            BiomassNitrogen,
            SoilNitrogen,
            // Phosphorus
            BiomassPhosphorus,
            SoilPhosphorus,
            OceanPhosphorus,
            // External
            OperatorIntervention,
            // Detritus nutrients
            DetritusNitrogen,
            DetritusPhosphorus,
            // Rock phosphorus
            CrustPhosphorus,
            // Surface water
            SurfaceWater,
            // Materials and bodies
            MaterialCarbon,
            MaterialWater,
            HumanCarbon,
        ][..]
    }

    /// Get reservoir as string
    ///
    /// # Returns
    ///
    /// String representation
    pub fn as_str(&self) -> &'static str {
        match self {
            Reservoir::TOAInsolation => "TOAInsolation",
            Reservoir::AtmosEnergy => "AtmosEnergy",
            Reservoir::SurfaceEnergy => "SurfaceEnergy",
            Reservoir::OceanHeat => "OceanHeat",
            Reservoir::TidalHeat => "TidalHeat",
            Reservoir::VolcanicHeat => "VolcanicHeat",
            Reservoir::SpaceRadiation => "SpaceRadiation",
            Reservoir::ImpactorKinetic => "ImpactorKinetic",
            Reservoir::Atmosphere => "Atmosphere",
            Reservoir::SoilWater => "SoilWater",
            Reservoir::Groundwater => "Groundwater",
            Reservoir::Rivers => "Rivers",
            Reservoir::Ocean => "Ocean",
            Reservoir::AtmosCO2 => "AtmosCO2",
            Reservoir::BiomassCarbon => "BiomassCarbon",
            Reservoir::DetritusCarbon => "DetritusCarbon",
            Reservoir::OceanDIC => "OceanDIC",
            Reservoir::CrustCarbon => "CrustCarbon",
            Reservoir::AtmosO2 => "AtmosO2",
            Reservoir::BiomassOxygen => "BiomassOxygen",
            Reservoir::OceanDO => "OceanDO",
            Reservoir::AtmosN2 => "AtmosN2",
            Reservoir::BiomassNitrogen => "BiomassNitrogen",
            Reservoir::SoilNitrogen => "SoilNitrogen",
            Reservoir::BiomassPhosphorus => "BiomassPhosphorus",
            Reservoir::SoilPhosphorus => "SoilPhosphorus",
            Reservoir::OceanPhosphorus => "OceanPhosphorus",
            Reservoir::OperatorIntervention => "OperatorIntervention",
            Reservoir::DetritusNitrogen => "DetritusNitrogen",
            Reservoir::DetritusPhosphorus => "DetritusPhosphorus",
            Reservoir::CrustPhosphorus => "CrustPhosphorus",
            Reservoir::SurfaceWater => "SurfaceWater",
            Reservoir::MaterialCarbon => "MaterialCarbon",
            Reservoir::MaterialWater => "MaterialWater",
            Reservoir::HumanCarbon => "HumanCarbon",
        }
    }
}

impl fmt::Display for Reservoir {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Transfer types for flux tracking
///
/// Each type represents a conserved quantity.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub enum FluxKind {
    Energy,
    Water,
    Carbon,
    Oxygen,
    Nitrogen,
    Phosphorus,
}

impl FluxKind {
    /// Get all flux kinds
    ///
    /// # Returns
    ///
    /// Array of all flux kinds
    pub fn all() -> &'static [FluxKind] {
        &[
            FluxKind::Energy,
            FluxKind::Water,
            FluxKind::Carbon,
            FluxKind::Oxygen,
            FluxKind::Nitrogen,
            FluxKind::Phosphorus,
        ][..]
    }

    /// Get flux kind as string
    ///
    /// # Returns
    ///
    /// String representation
    pub fn as_str(&self) -> &'static str {
        match self {
            FluxKind::Energy => "Energy",
            FluxKind::Water => "Water",
            FluxKind::Carbon => "Carbon",
            FluxKind::Oxygen => "Oxygen",
            FluxKind::Nitrogen => "Nitrogen",
            FluxKind::Phosphorus => "Phosphorus",
        }
    }
}

impl fmt::Display for FluxKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Absolute tolerance per flux kind for conservation checks, in the flux
/// kind's SI unit. It must be strict enough to catch a violation and loose
/// enough to absorb floating-point rounding.
#[derive(Debug, Clone, PartialEq)]
pub struct TolerancePolicy {
    pub energy_abs_tol: f64,
    pub water_abs_tol: f64,
    pub carbon_abs_tol: f64,
    pub oxygen_abs_tol: f64,
    pub nitrogen_abs_tol: f64,
    pub phosphorus_abs_tol: f64,
}

impl TolerancePolicy {
    /// Create new tolerance policy
    ///
    /// # Arguments
    ///
    /// Each argument is an absolute tolerance for that flux kind.
    ///
    /// # Returns
    ///
    /// New TolerancePolicy
    pub fn new(
        energy_abs_tol: f64,
        water_abs_tol: f64,
        carbon_abs_tol: f64,
        oxygen_abs_tol: f64,
        nitrogen_abs_tol: f64,
        phosphorus_abs_tol: f64,
    ) -> Self {
        Self {
            energy_abs_tol,
            water_abs_tol,
            carbon_abs_tol,
            oxygen_abs_tol,
            nitrogen_abs_tol,
            phosphorus_abs_tol,
        }
    }

    /// Get tolerance for specific flux kind
    ///
    /// # Arguments
    ///
    /// * `kind` - Flux kind
    ///
    /// # Returns
    ///
    /// Absolute tolerance for that flux kind
    pub fn tolerance(&self, kind: FluxKind) -> f64 {
        match kind {
            FluxKind::Energy => self.energy_abs_tol,
            FluxKind::Water => self.water_abs_tol,
            FluxKind::Carbon => self.carbon_abs_tol,
            FluxKind::Oxygen => self.oxygen_abs_tol,
            FluxKind::Nitrogen => self.nitrogen_abs_tol,
            FluxKind::Phosphorus => self.phosphorus_abs_tol,
        }
    }
}

impl Default for TolerancePolicy {
    fn default() -> Self {
        Self::new(
            TICK_RESIDUAL_LIMIT, // Energy
            TICK_RESIDUAL_LIMIT, // Water
            TICK_RESIDUAL_LIMIT, // Carbon
            TICK_RESIDUAL_LIMIT, // Oxygen
            TICK_RESIDUAL_LIMIT, // Nitrogen
            TICK_RESIDUAL_LIMIT, // Phosphorus
        )
    }
}

/// Audit errors for conservation checking
#[derive(Debug, Clone, PartialEq)]
pub enum AuditError {
    /// Conservation imbalance exceeded tolerance
    ImbalanceExceeded {
        kind: FluxKind,
        balance: f64,
        tolerance: f64,
    },
}

/// Measured stock of each reservoir, per flux kind, at one instant. Units
/// match the ledger's (joules or kilograms).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ReservoirStocks {
    stocks: BTreeMap<(Reservoir, FluxKind), f64>,
}

impl ReservoirStocks {
    /// Create an empty set of measurements.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record the stock of `reservoir` for `kind`.
    pub fn set(&mut self, reservoir: Reservoir, kind: FluxKind, amount: f64) {
        self.stocks.insert((reservoir, kind), amount);
    }

    /// Measured stock of `reservoir` for `kind`, if it was recorded.
    pub fn get(&self, reservoir: Reservoir, kind: FluxKind) -> Option<f64> {
        self.stocks.get(&(reservoir, kind)).copied()
    }

    /// Reservoirs with a recorded stock for `kind`.
    pub fn reservoirs(&self, kind: FluxKind) -> impl Iterator<Item = Reservoir> + '_ {
        self.stocks
            .keys()
            .filter(move |(_, k)| *k == kind)
            .map(|(r, _)| *r)
    }
}

/// Why [`Ledger::audit_against_stocks`] failed.
#[derive(Debug, Clone, PartialEq)]
pub enum StockAuditError {
    /// The ledger moved flux through an interior reservoir, or it has a stock
    /// on one side of the tick only, so its closure cannot be checked.
    UnmeasuredReservoir {
        reservoir: Reservoir,
        kind: FluxKind,
    },
    /// The measured stock change differs from the recorded net inflow.
    StockMismatch {
        reservoir: Reservoir,
        kind: FluxKind,
        measured_change: f64,
        recorded_net_inflow: f64,
        tolerance: f64,
    },
}

impl fmt::Display for StockAuditError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnmeasuredReservoir { reservoir, kind } => write!(
                f,
                "{kind} closure for {reservoir} cannot be checked: its stock was not measured at both ends of the tick"
            ),
            Self::StockMismatch {
                reservoir,
                kind,
                measured_change,
                recorded_net_inflow,
                tolerance,
            } => write!(
                f,
                "{kind} not conserved in {reservoir}: stock changed by {measured_change} but the ledger recorded a net inflow of {recorded_net_inflow} (tolerance {tolerance})"
            ),
        }
    }
}

impl std::error::Error for StockAuditError {}

/// Summary report of all flux in a ledger
#[derive(Debug, Clone, Default)]
pub struct LedgerSummary {
    pub total_flux: FluxReport,
}

/// Flattened flux totals
#[derive(Debug, Clone, Default)]
pub struct FluxReport {
    pub energy: f64,
    pub water: f64,
    pub carbon: f64,
    pub oxygen: f64,
    pub nitrogen: f64,
    pub phosphorus: f64,
}

/// Closure residual summary (energy, water, carbon)
#[derive(Debug, Clone, Default)]
pub struct ClosureResiduals {
    pub energy: f64,
    pub water: f64,
    pub carbon: f64,
}

impl fmt::Display for AuditError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ImbalanceExceeded {
                kind,
                balance,
                tolerance,
            } => {
                write!(
                    f,
                    "Conservation imbalance for {}: balance={} exceeds tolerance={}",
                    kind, balance, tolerance
                )
            }
        }
    }
}

impl AuditError {
    /// Get the flux kind
    pub fn kind(&self) -> FluxKind {
        match self {
            Self::ImbalanceExceeded { kind, .. } => *kind,
        }
    }

    /// Get the balance value
    pub fn balance(&self) -> f64 {
        match self {
            Self::ImbalanceExceeded { balance, .. } => *balance,
        }
    }

    /// Get the tolerance value
    pub fn tolerance(&self) -> f64 {
        match self {
            Self::ImbalanceExceeded { tolerance, .. } => *tolerance,
        }
    }
}

impl std::error::Error for AuditError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn soil_uptake_ledger() -> Ledger {
        // 5 kg rain into soil, 2 kg of it drains to groundwater.
        let mut ledger = Ledger::new();
        ledger.push(FluxEntry::new(
            Reservoir::Atmosphere,
            Reservoir::SoilWater,
            5.0,
            FluxKind::Water,
        ));
        ledger.push(FluxEntry::new(
            Reservoir::SoilWater,
            Reservoir::Groundwater,
            2.0,
            FluxKind::Water,
        ));
        ledger
    }

    fn stocks(soil: f64, ground: f64) -> ReservoirStocks {
        let mut s = ReservoirStocks::new();
        s.set(Reservoir::SoilWater, FluxKind::Water, soil);
        s.set(Reservoir::Groundwater, FluxKind::Water, ground);
        s
    }

    #[test]
    fn stock_audit_passes_when_stocks_match_the_ledger() {
        let ledger = soil_uptake_ledger();
        let policy = TolerancePolicy::default();
        assert_eq!(
            ledger.audit_against_stocks(&stocks(10.0, 50.0), &stocks(13.0, 52.0), &policy),
            Ok(())
        );
        // Soil stored 3 kg, so the pass-through audit rightly rejects it.
        assert!(ledger.audit(policy).is_err());
    }

    #[test]
    fn stock_audit_catches_an_unrecorded_change() {
        let ledger = soil_uptake_ledger();
        let err = ledger
            .audit_against_stocks(
                &stocks(10.0, 50.0),
                &stocks(13.0, 55.0),
                &TolerancePolicy::default(),
            )
            .unwrap_err();
        assert!(matches!(
            err,
            StockAuditError::StockMismatch {
                reservoir: Reservoir::Groundwater,
                ..
            }
        ));
    }

    #[test]
    fn stock_audit_catches_a_compensating_error_net_imbalance_misses() {
        // One reservoir gains 1 kg and another loses 1 kg with no transfer
        // recorded. The interior total is unchanged, so net_imbalance
        // cannot see it; the stock audit can.
        let ledger = soil_uptake_ledger();
        let err = ledger.audit_against_stocks(
            &stocks(10.0, 50.0),
            &stocks(14.0, 51.0),
            &TolerancePolicy::default(),
        );
        assert!(err.is_err());
    }

    #[test]
    fn stock_audit_requires_every_interior_reservoir_to_be_measured() {
        let ledger = soil_uptake_ledger();
        let mut before = ReservoirStocks::new();
        before.set(Reservoir::SoilWater, FluxKind::Water, 10.0);
        let mut after = ReservoirStocks::new();
        after.set(Reservoir::SoilWater, FluxKind::Water, 13.0);
        assert_eq!(
            ledger.audit_against_stocks(&before, &after, &TolerancePolicy::default()),
            Err(StockAuditError::UnmeasuredReservoir {
                reservoir: Reservoir::Groundwater,
                kind: FluxKind::Water,
            })
        );
    }

    #[test]
    fn stock_residuals_report_magnitudes() {
        let ledger = soil_uptake_ledger();
        let r = ledger
            .stock_residuals(FluxKind::Water, &stocks(10.0, 50.0), &stocks(13.5, 52.0))
            .unwrap();
        assert!((r[&Reservoir::SoilWater] - 0.5).abs() < 1e-12);
        assert!(r[&Reservoir::Groundwater].abs() < 1e-12);
    }

    #[test]
    fn interior_and_orbital_heat_are_boundaries() {
        assert!(Reservoir::VolcanicHeat.is_boundary());
        assert!(Reservoir::TidalHeat.is_boundary());
        assert!(!Reservoir::SurfaceEnergy.is_boundary());
        assert!(!Reservoir::OceanHeat.is_boundary());
    }

    #[test]
    fn net_imbalance_is_net_boundary_exchange() {
        let ledger = soil_uptake_ledger();
        // 5 kg entered the interior from the (boundary) atmosphere.
        assert!((ledger.net_imbalance(FluxKind::Water) - 5.0).abs() < 1e-12);
    }
    #[test]
    fn ledger_balanced() {
        let mut ledger = Ledger::new();

        // Add balanced energy flux
        ledger.push(FluxEntry::new(
            Reservoir::TOAInsolation,
            Reservoir::AtmosEnergy,
            1000.0,
            FluxKind::Energy,
        ));
        ledger.push(FluxEntry::new(
            Reservoir::AtmosEnergy,
            Reservoir::SpaceRadiation,
            1000.0,
            FluxKind::Energy,
        ));

        let policy = TolerancePolicy::default();
        assert!(ledger.audit(policy).is_ok());
    }

    #[test]
    fn ledger_imbalanced() {
        let mut ledger = Ledger::new();

        // Add truly unbalanced energy flux (sources exceed sinks)
        ledger.push(FluxEntry::new(
            Reservoir::TOAInsolation,
            Reservoir::AtmosEnergy,
            1000.0,
            FluxKind::Energy,
        ));
        ledger.push(FluxEntry::new(
            Reservoir::AtmosEnergy,
            Reservoir::SpaceRadiation,
            600.0, // Less than input, 400 missing
            FluxKind::Energy,
        ));

        let policy = TolerancePolicy::default();
        let result = ledger.audit(policy);
        assert!(result.is_err());

        match result.unwrap_err() {
            AuditError::ImbalanceExceeded {
                kind,
                balance,
                tolerance,
            } => {
                assert_eq!(kind, FluxKind::Energy);
                assert_eq!(balance, 400.0); // 1000 in - 600 out
                assert!(balance > tolerance);
            }
        }
    }

    #[test]
    fn ledger_multiple_flux_kinds() {
        let mut ledger = Ledger::new();

        // Energy flux (balanced)
        ledger.push(FluxEntry::new(
            Reservoir::TOAInsolation,
            Reservoir::AtmosEnergy,
            1000.0,
            FluxKind::Energy,
        ));
        ledger.push(FluxEntry::new(
            Reservoir::AtmosEnergy,
            Reservoir::SpaceRadiation,
            1000.0,
            FluxKind::Energy,
        ));

        // Water flux (balanced)
        ledger.push(FluxEntry::new(
            Reservoir::Ocean,
            Reservoir::Atmosphere,
            500.0,
            FluxKind::Water,
        ));
        ledger.push(FluxEntry::new(
            Reservoir::Atmosphere,
            Reservoir::Rivers,
            500.0,
            FluxKind::Water,
        ));

        let policy = TolerancePolicy::default();
        assert!(ledger.audit(policy).is_ok());
    }

    #[test]
    fn ledger_clear() {
        let mut ledger = Ledger::new();

        ledger.push(FluxEntry::new(
            Reservoir::TOAInsolation,
            Reservoir::AtmosEnergy,
            1000.0,
            FluxKind::Energy,
        ));

        assert_eq!(ledger.len(), 1);
        assert!(!ledger.is_empty());

        ledger.clear();

        assert_eq!(ledger.len(), 0);
        assert!(ledger.is_empty());
    }

    #[test]
    fn ledger_balance_by_kind() {
        let mut ledger = Ledger::new();

        // Add balanced energy flux
        ledger.push(FluxEntry::new(
            Reservoir::TOAInsolation,
            Reservoir::AtmosEnergy,
            1000.0,
            FluxKind::Energy,
        ));
        ledger.push(FluxEntry::new(
            Reservoir::AtmosEnergy,
            Reservoir::SpaceRadiation,
            1000.0,
            FluxKind::Energy,
        ));

        // Add unbalanced water flux
        ledger.push(FluxEntry::new(
            Reservoir::Ocean,
            Reservoir::Atmosphere,
            500.0,
            FluxKind::Water,
        ));
        ledger.push(FluxEntry::new(
            Reservoir::Atmosphere,
            Reservoir::Rivers,
            300.0,
            FluxKind::Water,
        ));

        // balance() method just sums all amounts (1000 + 1000 = 2000 for energy)
        assert_eq!(ledger.balance(FluxKind::Energy), 2000.0);
        // Water: 500 + 300 = 800
        assert_eq!(ledger.balance(FluxKind::Water), 800.0);
        assert_eq!(ledger.balance(FluxKind::Carbon), 0.0);
    }

    #[test]
    fn flux_entry_accessors() {
        let entry = FluxEntry::new(
            Reservoir::TOAInsolation,
            Reservoir::AtmosEnergy,
            1000.0,
            FluxKind::Energy,
        );

        assert_eq!(entry.source(), Reservoir::TOAInsolation);
        assert_eq!(entry.sink(), Reservoir::AtmosEnergy);
        assert_eq!(entry.amount(), 1000.0);
        assert_eq!(entry.kind(), FluxKind::Energy);
    }

    #[test]
    fn tolerance_policy_accessors() {
        let policy = TolerancePolicy::new(
            0.001,   // Energy
            0.0005,  // Water
            0.00033, // Carbon
            0.00025, // Oxygen
            0.0002,  // Nitrogen
            0.00016, // Phosphorus
        );

        assert_eq!(policy.tolerance(FluxKind::Energy), 0.001);
        assert_eq!(policy.tolerance(FluxKind::Water), 0.0005);
        assert_eq!(policy.tolerance(FluxKind::Carbon), 0.00033);
        assert_eq!(policy.tolerance(FluxKind::Oxygen), 0.00025);
        assert_eq!(policy.tolerance(FluxKind::Nitrogen), 0.0002);
        assert_eq!(policy.tolerance(FluxKind::Phosphorus), 0.00016);
    }

    /// Dense ordinal for every variant. The `match` has no wildcard arm, so
    /// adding a `Reservoir` variant without listing it here (and therefore
    /// without adding it to `Reservoir::all()`) fails to compile.
    fn ordinal(r: Reservoir) -> usize {
        use Reservoir::*;
        match r {
            TOAInsolation => 0,
            AtmosEnergy => 1,
            SurfaceEnergy => 2,
            OceanHeat => 3,
            TidalHeat => 4,
            VolcanicHeat => 5,
            SpaceRadiation => 6,
            ImpactorKinetic => 7,
            Atmosphere => 8,
            SoilWater => 9,
            Groundwater => 10,
            Rivers => 11,
            Ocean => 12,
            AtmosCO2 => 13,
            BiomassCarbon => 14,
            DetritusCarbon => 15,
            OceanDIC => 16,
            CrustCarbon => 17,
            AtmosO2 => 18,
            BiomassOxygen => 19,
            OceanDO => 20,
            AtmosN2 => 21,
            BiomassNitrogen => 22,
            SoilNitrogen => 23,
            BiomassPhosphorus => 24,
            SoilPhosphorus => 25,
            OceanPhosphorus => 26,
            OperatorIntervention => 27,
            DetritusNitrogen => 28,
            DetritusPhosphorus => 29,
            CrustPhosphorus => 30,
            SurfaceWater => 31,
            MaterialCarbon => 32,
            MaterialWater => 33,
            HumanCarbon => 34,
        }
    }

    #[test]
    fn reservoir_all() {
        const VARIANTS: usize = 35;
        let all = Reservoir::all();

        // Every variant appears in `all()` exactly once.
        let mut seen = [0usize; VARIANTS];
        for r in all {
            seen[ordinal(*r)] += 1;
        }
        assert_eq!(
            seen, [1usize; VARIANTS],
            "Reservoir::all() must list every variant exactly once"
        );
        assert_eq!(all.len(), VARIANTS);

        // Check specific reservoirs exist
        assert!(all.contains(&Reservoir::TOAInsolation));
        assert!(all.contains(&Reservoir::Ocean));
        assert!(all.contains(&Reservoir::AtmosCO2));
        assert!(all.contains(&Reservoir::AtmosO2));
        assert!(all.contains(&Reservoir::AtmosN2));
        assert!(all.contains(&Reservoir::BiomassPhosphorus));
        assert!(all.contains(&Reservoir::OperatorIntervention));
        assert!(all.contains(&Reservoir::SurfaceWater));
        assert!(all.contains(&Reservoir::MaterialCarbon));
        assert!(all.contains(&Reservoir::MaterialWater));
        assert!(all.contains(&Reservoir::HumanCarbon));
        // Materials and bodies are interior reservoirs, not boundaries.
        for r in [
            Reservoir::MaterialCarbon,
            Reservoir::MaterialWater,
            Reservoir::HumanCarbon,
        ] {
            assert!(!r.is_boundary());
        }
    }

    #[test]
    fn operator_intervention_is_a_boundary_reservoir() {
        // Injections are sourced from outside the simulated system, so the
        // ledger must treat this reservoir as a boundary or every
        // intervention would show up as a closure violation.
        assert!(Reservoir::OperatorIntervention.is_boundary());
    }

    #[test]
    fn operator_sourced_flux_still_closes_the_audit() {
        let mut ledger = Ledger::new();
        ledger.push(FluxEntry::new(
            Reservoir::OperatorIntervention,
            Reservoir::SoilWater,
            500.0,
            FluxKind::Water,
        ));
        // SoilWater is not a boundary, so it must pass the water straight
        // through to a boundary for the audit to close.
        ledger.push(FluxEntry::new(
            Reservoir::SoilWater,
            Reservoir::Rivers,
            500.0,
            FluxKind::Water,
        ));
        assert!(ledger.audit(TolerancePolicy::default()).is_ok());
    }

    #[test]
    fn flux_kind_all() {
        let all = FluxKind::all();
        assert_eq!(all.len(), 6);

        let expected = [
            FluxKind::Energy,
            FluxKind::Water,
            FluxKind::Carbon,
            FluxKind::Oxygen,
            FluxKind::Nitrogen,
            FluxKind::Phosphorus,
        ];

        for &expected_kind in &expected {
            assert!(all.contains(&expected_kind));
        }
    }

    #[test]
    fn audit_error_display() {
        let error = AuditError::ImbalanceExceeded {
            kind: FluxKind::Energy,
            balance: 1000.0,
            tolerance: 0.001,
        };

        let display = format!("{}", error);
        assert!(display.contains("Energy"));
        assert!(display.contains("balance="));
        assert!(display.contains("tolerance="));
    }

    #[test]
    fn audit_reports_violations_in_stable_order() {
        let mut ledger = Ledger::new();
        ledger.push(FluxEntry::new(
            Reservoir::SurfaceEnergy,
            Reservoir::AtmosEnergy,
            1.0,
            FluxKind::Energy,
        ));
        ledger.push(FluxEntry::new(
            Reservoir::BiomassCarbon,
            Reservoir::AtmosCO2,
            2.0,
            FluxKind::Carbon,
        ));

        let first = format!(
            "{:?}",
            ledger
                .audit(TolerancePolicy::default())
                .expect_err("imbalanced ledger must fail")
        );
        let second = format!(
            "{:?}",
            ledger
                .audit(TolerancePolicy::default())
                .expect_err("imbalanced ledger must fail")
        );
        assert_eq!(first, second);
    }
}
