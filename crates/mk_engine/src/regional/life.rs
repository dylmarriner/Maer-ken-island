//! The island's living world, integrated (Phase 3 Task 9).
//!
//! One struct bundles every Phase 1-3 system and steps them together at the
//! plan's cadences, calling the step functions directly (Phase 4's scheduler
//! will replace the loop):
//!
//! - every **60 s**: the founders (`step_regional_humans`);
//! - every **hour**: the physical world (`RegionalPhysicalState::step`), each
//!   founder's respiration, and the estate's electricity;
//! - every **6 hours**: the ecology and the estate patch's vegetation, and
//!   the households' food and water (gathered from the cell, eaten and
//!   drunk through the material ledger).
//!
//! Every household step is audited: the ledger's net flow into
//! `MaterialCarbon`, `HumanCarbon` and `MaterialWater` must equal the change
//! in those stocks, or the step fails.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use mk_core::canon::CanonLocked;
use mk_core::flux::{FluxKind, Ledger, Reservoir};
use mk_core::rng::RngRegistry;
use mk_island::{
    DomainLevel, IslandDomain, IslandDomainError, IslandScenario, IslandScenarioError,
};

use super::ecology::{RegionalEcologyError, RegionalEcologyState};
use super::energy::{solar_output_kw, EstateEnergy, ROOM_LIGHT_LUX};
use super::estate_layout::{Space, SpaceId};
use super::household_gas::{HouseholdGas, HouseholdGasSpec};
use super::household_water::{
    installed_capacity_kg, storage_needed_kg, HouseholdWater, HouseholdWaterSpec,
};
use super::human_store::{open_run, HumanStore, HumanStoreError, RunId};
use super::humans::{
    bootstrap_regional_humans, step_regional_humans, HumanEstatePositions, RegionalHumanContext,
    RegionalHumanError,
};
use super::hydrology::discharge_m3_s;
use super::labour::{LabourBody, LabourError, LabourTable, LabourTool};
use super::local_vegetation::{seed_local_vegetation, LocalVegetationError, LocalVegetationPatch};
use super::materials::{
    cells_around, composition, water_search_cells, Material, MaterialError, MaterialLedger,
};
use super::physical::{RegionalPhysicalError, RegionalPhysicalState};
use super::property::{place_regional_estate, PlaceEstateError, PlacedEstate};
use super::scheduler::IslandScheduler;
use super::timber::{self, HaulJob, HaulTask};
use crate::humans::lifecycle::WALKING_MET;
use crate::humans::HumanSystem;
use crate::resource_economy::ResourceEconomyState;
use crate::topology::GridTopology;

/// What a founder at home is doing, as the reference packs' activity names.
/// A sleeping body burns measurably less than a waking one (Compendium of
/// Physical Activities: sleeping 0.9-1.0 MET against 1.0-1.3 sitting
/// quietly), and the circadian clock already knows which of the two this
/// human is doing, so the tick asks it rather than assuming one rate all
/// day. On a 36-hour day that difference is not academic: how long a person
/// sleeps depends on whether they are in a house they can darken (D23), and
/// expenditure has to follow it.
const ASLEEP_ACTIVITY: &str = "sleeping";
const AWAKE_AT_HOME_ACTIVITY: &str = "sitting_quietly";

/// Water an adult turns over per kcal they spend: 1 mL/kcal under average
/// conditions (National Research Council 1989, *Recommended Dietary
/// Allowances*, 10th ed., ch. 11). What a founder burns in an hour is what
/// they lose in water, and what the next drink has to put back.
pub const WATER_KG_PER_KCAL: f64 = 0.001;
/// Children younger than this stay with their mother: they do not choose
/// where to go (they are carried, then toddle behind her).
const DEPENDENT_AGE_YEARS: f64 = 5.0;
/// How far from where they stand a person looks for plants to eat (cells).
const FORAGE_RADIUS_CELLS: usize = 1;
/// How far from where they stand a person looks for liquid water (cells).
const WATER_RADIUS_CELLS: usize = 2;
/// Within this many cells of the household's stream a person also uses it (cells).
const HOME_RANGE_CELLS: usize = 3;

/// Wall-clock time spent in each major part of the island step. Runtime-only
/// instrumentation: never hashed, saved or read by the simulation.
#[derive(Debug, Clone, Copy, Default)]
pub struct SubsystemTimings {
    /// Human substeps run.
    pub substeps: u64,
    /// The people: movement, needs, cognition, estate interaction (every substep).
    pub humans: std::time::Duration,
    /// Burying the dead, children following mothers and newborn adoption
    /// (every substep).
    pub households: std::time::Duration,
    /// Weather, ocean, tides, insolation, hydrology step and the estate energy and respiration (hourly).
    pub physics_hourly: std::time::Duration,
    /// Ecology, vegetation and grazers (six-hourly).
    pub ecology_six_hourly: std::time::Duration,
    /// Per-human folder writes (store cadence).
    pub human_store: std::time::Duration,
}

/// A child's mother, from its birth record.
fn mother_of(human: &crate::humans::HumanBeing) -> Option<String> {
    human
        .profile
        .canonical_schema
        .as_ref()
        .and_then(|s| {
            s.reproductive_systems
                .genetics_system
                .birth_records
                .last()
                .map(|r| r.mother_id.clone())
        })
        .filter(|m| !m.is_empty())
}

/// Share of the body's water loss that is excretion to the soil.
const EXCRETION_TO_SOIL_FRACTION: f64 = 0.6;

#[derive(Debug)]
pub enum IslandLifeError {
    Physical(RegionalPhysicalError),
    Ecology(RegionalEcologyError),
    Estate(PlaceEstateError),
    Vegetation(LocalVegetationError),
    Human(RegionalHumanError),
    Labour(LabourError),
    /// A household step's carbon, oxygen or water stocks did not close.
    Audit(String),
    /// A snapshot written by a format this build does not know. Guessing at
    /// an unknown layout would quietly restore the wrong island.
    SnapshotVersion(u32),
    /// A snapshot's scenario profile no longer describes a valid domain.
    Domain(IslandDomainError),
    /// A snapshot's scenario is not one this build can run: a format version
    /// it does not know, or cadences and estate settings outside their
    /// bounds.
    Scenario(IslandScenarioError),
    /// The deep-time spin-up could not run or its cache could not be used.
    DeepTime(super::deep_time::DeepTimeError),
}

impl std::fmt::Display for IslandLifeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for IslandLifeError {}

/// Shortfalls the households met: nothing to harvest, a dry cell.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Shortfalls {
    pub food: u64,
    pub water: u64,
}

/// A walk to water (realism programme R1b): out to `source` at the packs'
/// pace, a drink there, and, from home with the house's jerrycans, back
/// carrying `carry_kg` for the tank. Someone who sets out for water keeps
/// going until they get there and back: the upstream needs model counts a
/// person hydrated as soon as they choose to drink, so waiting for that
/// choice to persist would strand them halfway (D42).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WaterTrip {
    pub source: (usize, usize),
    /// Seconds out to the source, and back home (0 for a trip that ends
    /// at the water).
    pub out_s: f64,
    pub back_s: f64,
    pub worked_s: f64,
    /// Water to bring home for the tank (kg); 0 when not going home.
    pub carry_kg: f64,
    pub reached: bool,
    /// The empty jerrycans' own mass (kg), carried both ways when water
    /// is fetched for the tank. 0 in a trip saved before it was kept.
    #[serde(default)]
    pub cans_kg: f64,
}

/// A tree a founder is felling: which, and the seconds of work put in.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct FellingJob {
    pub tree_id: u64,
    pub worked_s: f64,
}

/// A placer a founder is panning (D9): the walk there and back and the
/// packs' working day at the pan, as one job that resumes where it was
/// left if the founder stops to do something else.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MiningJob {
    pub site_id: u32,
    /// Seconds of walking each way, set when the job was planned.
    pub walk_s: f64,
    pub worked_s: f64,
}

/// The shed tool a founder pans with.
pub const PANNING_TOOL: &str = "Gold Pan";

/// The shed tool a founder fells with: the estate's axe, at the felling
/// pack's steel-axe times. The two-person crosscut saw needs a second
/// person on the other end, which nothing here arranges yet.
pub const FELLING_TOOL: &str = "Felling Axe";
/// The estate's road vehicle that carries cut timber: the ute.
pub const HAUL_UTE: &str = "Ford Raptor 4x4 Ute";
/// The estate's machine that lifts whole logs: the excavator.
pub const HAUL_EXCAVATOR: &str = "30-Ton Excavator (CAT 330)";

/// The snapshot format. A snapshot written by another version is refused
/// rather than guessed at.
pub const ISLAND_SNAPSHOT_VERSION: u32 = 1;

/// An island's whole persisted state. See [`IslandLife::snapshot`] for what
/// is deliberately absent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IslandLifeSnapshot {
    pub version: u32,
    pub scenario: IslandScenario,
    pub physical: RegionalPhysicalState,
    pub ecology: RegionalEcologyState,
    pub placed: PlacedEstate,
    pub vegetation: LocalVegetationPatch,
    pub humans: HumanSystem,
    pub positions: HumanEstatePositions,
    pub energy: EstateEnergy,
    pub materials: MaterialLedger,
    pub economy: ResourceEconomyState,
    pub scheduler: IslandScheduler,
    pub rng: RngRegistry,
    pub sim_time_s: u64,
    pub tick: u64,
    pub audits_closed: u64,
    pub shortfalls: Shortfalls,
    pub respired_since_meal: BTreeMap<String, f64>,
    #[serde(default)]
    pub lost_since_drink: BTreeMap<String, f64>,
    #[serde(default)]
    pub felling: BTreeMap<String, FellingJob>,
    #[serde(default)]
    pub resources: Option<super::resources::IslandResources>,
    #[serde(default)]
    pub mining: BTreeMap<String, MiningJob>,
    #[serde(default)]
    pub hauling: BTreeMap<String, HaulJob>,
    #[serde(default)]
    pub labour_kcal: BTreeMap<String, f64>,
    #[serde(default)]
    pub water_drunk_kg: BTreeMap<String, f64>,
    pub sleep_seconds: BTreeMap<String, (f64, f64)>,
    pub food_cell: (usize, usize),
    pub water_cell: Option<(usize, usize)>,
    /// The house's roof and rainwater tank. `None` in an island saved
    /// before the house had one; restoring installs it.
    #[serde(default)]
    pub household_water: Option<HouseholdWater>,
    #[serde(default)]
    pub water_trips: BTreeMap<String, WaterTrip>,
    #[serde(default)]
    pub household_gas: Option<HouseholdGas>,
}

pub struct IslandLife {
    pub canon: Arc<CanonLocked>,
    pub domain: IslandDomain,
    pub scenario: IslandScenario,
    pub physical: RegionalPhysicalState,
    pub ecology: RegionalEcologyState,
    pub placed: PlacedEstate,
    pub vegetation: LocalVegetationPatch,
    pub humans: HumanSystem,
    pub positions: HumanEstatePositions,
    pub energy: EstateEnergy,
    pub materials: MaterialLedger,
    pub economy: ResourceEconomyState,
    pub sim_time_s: u64,
    pub tick: u64,
    pub audits_closed: u64,
    pub shortfalls: Shortfalls,
    /// Carbon each founder has respired since their last meal (kg), which is
    /// what the next meal has to put back.
    respired_since_meal: BTreeMap<String, f64>,
    /// Water each founder has lost since their last drink (kg), which is
    /// what the next drink has to put back.
    lost_since_drink: BTreeMap<String, f64>,
    /// The tree each founder is felling and how long they have worked at
    /// it: felling a stem takes the felling pack's time, across as many
    /// steps as that is, and a founder who stops comes back to the same
    /// tree.
    felling: BTreeMap<String, FellingJob>,
    /// The island's mineral sites, as surveyed and as worked since
    /// (`regional::resources`).
    pub resources: Option<super::resources::IslandResources>,
    /// The placer each founder is panning and how long they have been at
    /// it, walks included (D9).
    mining: BTreeMap<String, MiningJob>,
    /// The log each founder is moving to the store and the tasks they
    /// still have to do (`regional::timber`).
    hauling: BTreeMap<String, HaulJob>,
    /// Energy (kcal) each founder's labour has cost since their respiration
    /// was last charged, above what the same time awake at home costs.
    labour_kcal: BTreeMap<String, f64>,
    /// Water each founder has drunk since the island began (kg): the record
    /// the intake checks read.
    pub water_drunk_kg: BTreeMap<String, f64>,
    /// Seconds each founder has spent asleep and awake since their
    /// respiration was last charged. The sleep state can turn over at any
    /// human substep, so sampling it once an hour would charge a whole hour
    /// at whichever state happened to be current at the end of it.
    sleep_seconds: BTreeMap<String, (f64, f64)>,
    pub scheduler: IslandScheduler,
    labour: LabourTable,
    /// The world's keyed streams. Readable because creating a person in the
    /// world draws from them (`create_human`), and a creation keyed on
    /// anything else would not replay.
    pub(crate) rng: RngRegistry,
    /// A real bridge to the outside, when an operator attached one.
    ///
    /// Like the folders and the replay log, it is not in the snapshot and
    /// not in the state digest -- but unlike them it is not merely a
    /// record, and the difference matters. With one attached, a human may
    /// reach the real internet, and what comes back is not reproducible.
    /// That is the whole point of the feature and the whole reason it is
    /// opt-in: an island that did it would stop being an island two runs
    /// of which agree.
    ///
    /// `replay` and `inspect` never attach one, so a log replays the same
    /// way whether or not a live human sent real email in the run that
    /// produced it. A search's *results* enter only through this bridge;
    /// nothing else in the island can see out.
    computer_bridge: Option<crate::humans::computer_bridge::ComputerBridgeHandle>,
    /// This run's folder tree, once somebody asks for one. Records rather
    /// than state: it is absent from the snapshot and from the state digest,
    /// and an island with one behaves exactly like an island without.
    human_store: Option<HumanStore>,
    /// Everything that has reached this island from outside, in order.
    ///
    /// A record rather than state, like the folders: it is not hashed and
    /// not snapshotted, because it describes how the island got here rather
    /// than where it is. The runner writes it beside a snapshot, and
    /// `replay::replay_island` turns it back into this island.
    pub(crate) replay_log: super::replay::IslandReplayLog,
    /// Where the step time goes (runtime-only; not hashed or saved).
    pub timings: SubsystemTimings,
    food_cell: (usize, usize),
    pub water_cell: Option<(usize, usize)>,
    /// The house's roof and rainwater tank (realism programme R1).
    pub household_water: Option<HouseholdWater>,
    /// Who is walking to water, or home with it (realism programme R1b).
    pub water_trips: BTreeMap<String, WaterTrip>,
    /// The kitchen stove's bottled LPG (realism programme R4, D59).
    pub household_gas: Option<HouseholdGas>,
    topology: GridTopology,
    /// How the living world was spun up before the founders arrived, when
    /// this island was bootstrapped rather than restored (a record, not
    /// state: not hashed and not snapshotted).
    pub spin_up: Option<super::deep_time::SpinUpReport>,
}

/// RothC's parameters and every cell's year-average temperature modifier
/// `a`, from the reference packs and the cell's climatology.
fn soil_for(
    domain: &IslandDomain,
    physical: &RegionalPhysicalState,
    ecology: &RegionalEcologyState,
    labour: &LabourTable,
) -> Result<(super::soil::RothC, Vec<f64>), IslandLifeError> {
    let params = super::soil::RothC::from_reference(labour.library())
        .map_err(|e| IslandLifeError::Labour(LabourError::MissingReference(e)))?;
    let (rows, cols) = (
        domain.rows(DomainLevel::Medium),
        domain.cols(DomainLevel::Medium),
    );
    let mut factor = vec![0.0; rows * cols];
    for (i, f) in factor.iter_mut().enumerate() {
        if params
            .plant_dpm_share(ecology.biome_grid.data()[i])
            .is_none()
        {
            continue;
        }
        let year = super::ecology::cell_year(domain, physical, i / cols, i % cols);
        let months = year.temperature_k.len() as f64;
        *f = year
            .temperature_k
            .iter()
            .map(|k| params.temperature_factor(k - 273.15))
            .sum::<f64>()
            / months;
    }
    Ok((params, factor))
}

/// Spin the living world up for the scenario's years, from the cache under
/// `save_root` when one is there and whole.
fn spin_up_cached(
    canon: &CanonLocked,
    scenario: &IslandScenario,
    domain: &IslandDomain,
    ecology: &mut RegionalEcologyState,
    save_root: Option<&Path>,
) -> Result<super::deep_time::SpinUpReport, IslandLifeError> {
    use super::deep_time as dt;
    let settings = dt::SpinUpSettings::new(canon, scenario.spin_up_years);
    let key = dt::cache_key(canon, scenario, settings, ecology);
    let path = save_root.map(|root| dt::cache_path(root, &key));
    if let Some(path) = &path {
        match dt::load_cached(path, &key) {
            Ok(Some((report, cached))) => {
                *ecology = cached;
                return Ok(report);
            }
            Ok(None) => {}
            // A damaged file is not the island: spin up again and replace it.
            Err(e) => tracing::warn!("ignoring spin-up cache {}: {e}", path.display()),
        }
    }
    let report = dt::spin_up(ecology, domain.cell_area_m2(DomainLevel::Medium), settings)
        .map_err(IslandLifeError::DeepTime)?;
    if let Some(path) = &path {
        // A cache that cannot be written costs the next run time, not this
        // run its island.
        if let Err(e) = dt::save_cached(path, &key, &report, ecology) {
            tracing::warn!("could not save spin-up cache {}: {e}", path.display());
        }
    }
    Ok(report)
}

fn net(ledger: &Ledger, reservoir: Reservoir, kind: FluxKind) -> f64 {
    ledger
        .entries()
        .iter()
        .filter(|e| e.kind == kind)
        .map(|e| {
            (if e.sink == reservoir { e.amount } else { 0.0 })
                - (if e.source == reservoir { e.amount } else { 0.0 })
        })
        .sum()
}

/// An environment variable naming a directory where [`IslandLife::bootstrap`]
/// keeps deep-time spin-ups, as [`IslandLife::bootstrap_cached`] does under
/// a save root. The workspace's `.cargo/config.toml` points it into
/// `target/` for everything cargo runs, so the many test binaries that
/// bootstrap the default island pay for its thousand years once rather than
/// each (CI's fast tier went from 24 to 40 minutes without it). Unset, as
/// it is for an installed binary, nothing is cached. Safe to share because
/// the cache key covers the model's source and the ecology it starts from
/// (`deep_time::cache_key`).
pub const SPIN_UP_CACHE_ENV: &str = "MK_SPINUP_CACHE";

impl IslandLife {
    /// Bootstrap the whole island from a scenario: physics, ecology, the
    /// estate and its trees, and the founders at home. The spin-up is
    /// cached under [`SPIN_UP_CACHE_ENV`] when that names a directory.
    pub fn bootstrap(
        scenario: IslandScenario,
        canon: Arc<CanonLocked>,
    ) -> Result<Self, IslandLifeError> {
        let shared = std::env::var_os(SPIN_UP_CACHE_ENV)
            .filter(|dir| !dir.is_empty())
            .map(std::path::PathBuf::from);
        Self::bootstrap_cached(scenario, canon, shared.as_deref())
    }

    /// [`Self::bootstrap`], keeping the deep-time spin-up under
    /// `save_root/spinup/` so a later run of the same canon, scenario and
    /// settings loads it instead of repeating it. A damaged cache file is
    /// spun up again and replaced.
    ///
    /// Bootstrap order: geophysics, physical state, ecology and soil, the
    /// deep-time spin-up, then the estate, its trees and the founders.
    pub fn bootstrap_cached(
        scenario: IslandScenario,
        canon: Arc<CanonLocked>,
        save_root: Option<&Path>,
    ) -> Result<Self, IslandLifeError> {
        let domain =
            IslandDomain::from_profile(scenario.profile.clone()).expect("a validated scenario");
        let physical = RegionalPhysicalState::bootstrap(&canon, &domain, scenario.seed)
            .map_err(IslandLifeError::Physical)?;
        let mut ecology = RegionalEcologyState::bootstrap(&domain, &physical)
            .map_err(IslandLifeError::Ecology)?;
        let labour = LabourTable::load_default().map_err(IslandLifeError::Labour)?;
        let (params, factor) = soil_for(&domain, &physical, &ecology, &labour)?;
        ecology.install_soil(params, factor);
        let spin_up = if scenario.spin_up_years > 0 {
            Some(spin_up_cached(
                &canon,
                &scenario,
                &domain,
                &mut ecology,
                save_root,
            )?)
        } else {
            None
        };
        // The fruit and seed people eat, laid down on the spun-up forest.
        let shares = super::edible::ReproductiveShares::from_reference(labour.library())
            .map_err(|e| IslandLifeError::Labour(LabourError::MissingReference(e)))?;
        ecology.install_edible(&shares);
        let placed = place_regional_estate(
            &canon,
            &physical,
            &ecology,
            &domain,
            &scenario.estate_patch,
            scenario.seed,
        )
        .map_err(IslandLifeError::Estate)?;
        let vegetation = seed_local_vegetation(
            &ecology,
            &placed.layout,
            &domain,
            &scenario.estate_patch,
            scenario.seed,
        )
        .map_err(IslandLifeError::Vegetation)?;
        let (humans, positions) =
            bootstrap_regional_humans(&placed.property, &placed.layout, &domain)
                .map_err(IslandLifeError::Human)?;
        let estate = placed
            .property
            .properties
            .iter()
            .find(|p| p.owner_agent_ids.iter().any(|i| i == "Gem-D"))
            .expect("the estate was placed");
        let mut energy = EstateEnergy::from_config(&scenario.estate_energy, estate);
        energy.wire_lighting(&placed.layout, estate);
        attach_estate_hydro(&mut energy, &placed, &domain);

        let mut materials = MaterialLedger::new();
        for h in humans.registry.iter() {
            materials.register_human(h.agent_id(), h.body.weight_kg);
        }
        let (row, col) = placed.location;
        let medium = DomainLevel::Medium;
        let block = [
            (row, col),
            (row, col + 1),
            (row + 1, col),
            (row + 1, col + 1),
        ];
        let food_cell = block
            .iter()
            .copied()
            .max_by(|a, b| {
                ecology
                    .biomass_kgc_m2
                    .get(a.0, a.1)
                    .total_cmp(ecology.biomass_kgc_m2.get(b.0, b.1))
            })
            .expect("a four-cell block");
        let (rows, cols) = (domain.rows(medium), domain.cols(medium));
        let water_cell =
            (row.saturating_sub(2)..=(row + 3).min(rows - 1))
                .flat_map(|r| {
                    (col.saturating_sub(2)..=(col + 3).min(cols - 1)).map(move |c| (r, c))
                })
                .filter(|&(r, c)| discharge_m3_s(&physical.hydrology, &domain, r, c) >= 1.0)
                .max_by(|a, b| {
                    discharge_m3_s(&physical.hydrology, &domain, a.0, a.1)
                        .total_cmp(&discharge_m3_s(&physical.hydrology, &domain, b.0, b.1))
                });
        let topology = GridTopology::regional(&domain, medium);
        let scenario_cadences = scenario.cadences;
        let mut life = Self {
            rng: RngRegistry::new(scenario.seed),
            canon,
            domain,
            scenario,
            physical,
            ecology,
            placed,
            vegetation,
            humans,
            positions,
            energy,
            materials,
            economy: ResourceEconomyState::new(),
            sim_time_s: 0,
            tick: 0,
            audits_closed: 0,
            shortfalls: Shortfalls::default(),
            respired_since_meal: BTreeMap::new(),
            lost_since_drink: BTreeMap::new(),
            felling: BTreeMap::new(),
            resources: None,
            mining: BTreeMap::new(),
            hauling: BTreeMap::new(),
            labour_kcal: BTreeMap::new(),
            water_drunk_kg: BTreeMap::new(),
            sleep_seconds: BTreeMap::new(),
            scheduler: IslandScheduler::new(scenario_cadences),
            labour,
            computer_bridge: None,
            human_store: None,
            // Filled in below: the digest needs the built island.
            replay_log: super::replay::IslandReplayLog::new(String::new()),
            timings: SubsystemTimings::default(),
            food_cell,
            water_cell,
            household_water: None,
            water_trips: BTreeMap::new(),
            household_gas: None,
            topology,
            spin_up,
        };
        // A log says which scenario it belongs to, so replaying it against
        // a different island refuses rather than reproducing nothing and
        // looking like it had.
        if life.energy.lighting.is_empty() {
            // Saved before the house was wired for lamps: wire it now, from
            // the layout the snapshot carries, so a reload is not an island
            // in the dark.
            if let Some(estate) = life
                .placed
                .property
                .properties
                .iter()
                .find(|p| p.owner_agent_ids.iter().any(|i| i == "Gem-D"))
            {
                life.energy.wire_lighting(&life.placed.layout, estate);
            }
        }
        if life.energy.hydro.is_empty() {
            // Saved before the river's plant was connected.
            attach_estate_hydro(&mut life.energy, &life.placed, &life.domain);
        }
        if life.resources.is_none() {
            // Saved before the island's mineral sites were surveyed, or
            // newly built.
            life.install_resources()?;
        }
        if life.household_water.is_none() {
            life.install_household_water()?;
        }
        if life.household_gas.is_none() {
            // Saved before the stove burned gas, or newly built.
            life.install_household_gas()?;
        }
        if life.ecology.soil.is_none() {
            life.install_soil()?;
        }
        if life.ecology.edible.is_none() {
            life.install_edible()?;
        }
        life.replay_log = super::replay::IslandReplayLog::new(life.scenario_digest_hex());
        Ok(life)
    }

    /// Everything an island carries that cannot be derived again.
    ///
    /// What is left out is left out deliberately: the canon is supplied by
    /// whoever loads the snapshot, the domain follows from the scenario's
    /// profile, the grid topology follows from the domain, and the labour
    /// table is read from the reference packs. Storing those would let a
    /// snapshot disagree with the canon it is loaded against, which is the
    /// one thing a saved world must not be able to do.
    ///
    /// This clones the state, which on a full island is a few hundred MB
    /// held twice for as long as the save takes. A borrowing twin of
    /// `IslandLifeSnapshot` would avoid it, at the cost of two field lists
    /// that have to stay in step — and a field missing from the writing half
    /// would silently write an incomplete island. The copy is the safer
    /// price until a measurement says otherwise.
    pub fn snapshot(&self) -> IslandLifeSnapshot {
        IslandLifeSnapshot {
            version: ISLAND_SNAPSHOT_VERSION,
            scenario: self.scenario.clone(),
            physical: self.physical.clone(),
            ecology: self.ecology.clone(),
            placed: self.placed.clone(),
            vegetation: self.vegetation.clone(),
            humans: self.humans.clone(),
            positions: self.positions.clone(),
            energy: self.energy.clone(),
            materials: self.materials.clone(),
            economy: self.economy.clone(),
            scheduler: self.scheduler.clone(),
            rng: self.rng.clone(),
            sim_time_s: self.sim_time_s,
            tick: self.tick,
            audits_closed: self.audits_closed,
            shortfalls: self.shortfalls,
            respired_since_meal: self.respired_since_meal.clone(),
            lost_since_drink: self.lost_since_drink.clone(),
            felling: self.felling.clone(),
            resources: self.resources.clone(),
            mining: self.mining.clone(),
            hauling: self.hauling.clone(),
            labour_kcal: self.labour_kcal.clone(),
            water_drunk_kg: self.water_drunk_kg.clone(),
            sleep_seconds: self.sleep_seconds.clone(),
            food_cell: self.food_cell,
            water_cell: self.water_cell,
            household_water: self.household_water.clone(),
            water_trips: self.water_trips.clone(),
            household_gas: self.household_gas.clone(),
        }
    }

    /// Rebuild an island from a snapshot and the canon it must agree with.
    ///
    /// The derived parts are rebuilt here rather than trusted from the file,
    /// so a snapshot cannot smuggle in a domain or a topology that does not
    /// follow from its own scenario.
    pub fn restore(
        canon: Arc<CanonLocked>,
        snapshot: IslandLifeSnapshot,
    ) -> Result<Self, IslandLifeError> {
        if snapshot.version != ISLAND_SNAPSHOT_VERSION {
            return Err(IslandLifeError::SnapshotVersion(snapshot.version));
        }
        // The scenario came out of a file, so it is checked exactly as one
        // read from disk would be before anything is built from it.
        snapshot
            .scenario
            .validate()
            .map_err(IslandLifeError::Scenario)?;
        let domain = IslandDomain::from_profile(snapshot.scenario.profile.clone())
            .map_err(IslandLifeError::Domain)?;
        let topology = GridTopology::regional(&domain, DomainLevel::Medium);
        let mut life = Self {
            canon,
            domain,
            scenario: snapshot.scenario,
            physical: snapshot.physical,
            ecology: snapshot.ecology,
            placed: snapshot.placed,
            vegetation: snapshot.vegetation,
            humans: snapshot.humans,
            positions: snapshot.positions,
            energy: snapshot.energy,
            materials: snapshot.materials,
            economy: snapshot.economy,
            scheduler: snapshot.scheduler,
            rng: snapshot.rng,
            sim_time_s: snapshot.sim_time_s,
            tick: snapshot.tick,
            audits_closed: snapshot.audits_closed,
            shortfalls: snapshot.shortfalls,
            respired_since_meal: snapshot.respired_since_meal,
            lost_since_drink: snapshot.lost_since_drink,
            felling: snapshot.felling,
            resources: snapshot.resources,
            mining: snapshot.mining,
            hauling: snapshot.hauling,
            labour_kcal: snapshot.labour_kcal,
            water_drunk_kg: snapshot.water_drunk_kg,
            sleep_seconds: snapshot.sleep_seconds,
            labour: LabourTable::load_default().map_err(IslandLifeError::Labour)?,
            // A restored island keeps no records until someone attaches
            // some: the folders belong to the run that wrote them, and a
            // reload is a new run.
            computer_bridge: None,
            human_store: None,
            // A restored island starts a fresh log: the commands that got
            // it here are in the log beside the snapshot, not in the file.
            // Its scenario is filled in below, as at bootstrap.
            replay_log: super::replay::IslandReplayLog::new(String::new()),
            timings: SubsystemTimings::default(),
            food_cell: snapshot.food_cell,
            water_cell: snapshot.water_cell,
            household_water: snapshot.household_water,
            water_trips: snapshot.water_trips,
            household_gas: snapshot.household_gas,
            topology,
            spin_up: None,
        };
        if life.resources.is_none() {
            // Saved before the island's mineral sites were surveyed, or
            // newly built.
            life.install_resources()?;
        }
        if life.household_water.is_none() {
            // Saved before the house had a tank.
            life.install_household_water()?;
        }
        if life.household_gas.is_none() {
            // Saved before the stove burned gas, or newly built.
            life.install_household_gas()?;
        }
        if life.ecology.soil.is_none() {
            // Saved before the island had soil carbon.
            life.install_soil()?;
        }
        if life.ecology.edible.is_none() {
            // Saved before people ate only what is edible.
            life.install_edible()?;
        }
        life.replay_log = super::replay::IslandReplayLog::new(life.scenario_digest_hex());
        Ok(life)
    }

    /// Start keeping a folder for every human, under a run of this
    /// island's own.
    ///
    /// Returns the run's id, which is also the name of its directory under
    /// `save_root`. Calling this twice starts a second run with its own
    /// folders rather than writing into the first one's.
    ///
    /// The island's behaviour does not change: with the store attached, the
    /// state digest after any number of steps is the same as without it, and
    /// the same again if every write fails. The folders are a record of the
    /// run, not an input to it.
    /// Attach a real bridge to the outside, so a human at their own
    /// machine in a powered computer room can search the web or send an
    /// email for real.
    ///
    /// **This makes the island irreproducible, on purpose.** Nothing else
    /// in it can see out; what comes back through here is whatever the
    /// internet said at that moment, and two runs of the same seed will
    /// not agree once a human has used it. That is the feature, and it is
    /// why nothing attaches this unless an operator sets
    /// `COMPUTER_ACTIONS_ENABLED=1`, and why `replay` and `inspect` never
    /// attach it at all: a log replays identically whether or not a live
    /// human sent real email in the run that produced it.
    ///
    /// Attaching it alone changes nothing. Until a human actually
    /// succeeds at one of the two actions, the state digest is the same as
    /// an island with no bridge -- including when the service is
    /// unreachable, because the costs are charged on success only.
    /// `tests/island_computer_service.rs` holds all three cases.
    pub fn attach_computer_bridge(
        &mut self,
        bridge: crate::humans::computer_bridge::ComputerBridgeHandle,
    ) {
        self.computer_bridge = Some(bridge);
    }

    /// Whether a bridge is attached, for a dashboard saying what this
    /// island can do.
    pub fn has_computer_bridge(&self) -> bool {
        self.computer_bridge.is_some()
    }

    pub fn enable_human_store(&mut self, save_root: &Path) -> Result<RunId, HumanStoreError> {
        let (mut store, storage) =
            open_run(save_root, &self.scenario_digest(), &self.scenario.seed)?;
        // Storage first, and only then the policy that depends on it. The
        // other order leaves a state nobody can see: if `set_storage` fails,
        // `auto_sync` is already off and `human_store` is still `None`, so
        // the island runs with nothing syncing it and no counter to say so.
        self.humans
            .registry
            .set_storage(storage)
            .map_err(HumanStoreError::Storage)?;
        // Upstream rewrites every human's files on every step. At a
        // 60-second human step that is 1,440 full rewrites per simulated
        // day, so the island syncs on `human_store_seconds` instead — and
        // immediately when somebody dies, which is the one moment a stale
        // file would be a lie rather than a lag.
        self.humans.set_auto_sync(false);
        for human in self.humans.registry.iter() {
            if matches!(human.profile.status, mk_core::human::HumanStatus::Alive) {
                store.note_status(human.agent_id(), true, &mut Vec::new());
            }
        }
        let id = store.run_id().clone();
        self.human_store = Some(store);
        Ok(id)
    }

    /// This run's records, if any are being kept.
    pub fn human_store(&self) -> Option<&HumanStore> {
        self.human_store.as_ref()
    }

    /// Identifies the scenario this island is running, so two runs of the
    /// same one share a prefix and two different ones do not.
    pub(crate) fn scenario_digest(&self) -> [u8; 32] {
        let canonical = serde_json::to_vec(&self.scenario).unwrap_or_default();
        *blake3::hash(&canonical).as_bytes()
    }

    /// Write every human's full state to their folder now.
    ///
    /// Failures are counted on the store and logged, never returned: a disk
    /// that will not take a record is not a reason for the island to stop
    /// having a history.
    pub fn sync_humans(&mut self) {
        let Some(mut store) = self.human_store.take() else {
            return;
        };
        store.note_failures(&self.humans.registry.sync_to_storage());
        self.human_store = Some(store);
    }

    /// Write one human's final state the moment they die.
    fn sync_one_human(&mut self, agent_id: &str) {
        let Some(mut store) = self.human_store.take() else {
            return;
        };
        if let Err(e) = self.humans.registry.sync_human_to_storage(agent_id) {
            store.note_failure(&e);
        }
        self.human_store = Some(store);
    }

    /// Run `f` as one audited household step: the ledger's net flows must
    /// equal the change in each material stock.
    pub(super) fn audited(
        &mut self,
        f: impl FnOnce(&mut Self, &mut Ledger),
    ) -> Result<(), IslandLifeError> {
        let stocks = |m: &MaterialLedger| {
            [
                m.stock_of(Reservoir::MaterialCarbon),
                m.stock_of(Reservoir::HumanCarbon),
                m.stock_of(Reservoir::MaterialWater),
            ]
        };
        let before = stocks(&self.materials);
        let mut ledger = Ledger::new();
        f(self, &mut ledger);
        let after = stocks(&self.materials);
        for (i, (reservoir, kind)) in [
            (Reservoir::MaterialCarbon, FluxKind::Carbon),
            (Reservoir::HumanCarbon, FluxKind::Carbon),
            (Reservoir::MaterialWater, FluxKind::Water),
        ]
        .into_iter()
        .enumerate()
        {
            let (delta, flow) = (after[i] - before[i], net(&ledger, reservoir, kind));
            if (delta - flow).abs() > 1e-9 * (1.0 + delta.abs()) {
                return Err(IslandLifeError::Audit(format!(
                    "{reservoir:?} at t={} s: stock changed {delta}, ledger says {flow}",
                    self.sim_time_s
                )));
            }
        }
        self.audits_closed += 1;
        Ok(())
    }

    /// Drop every per-person accumulator for `id`.
    ///
    /// All of them are hashed into the state digest, so a person who
    /// has left the world has to leave these too or two islands that agree
    /// about who is alive will disagree about their digest. `advance`
    /// already does this for the dead; a removal needs the same.
    pub(super) fn forget_accumulators(&mut self, id: &str) {
        self.respired_since_meal.remove(id);
        self.lost_since_drink.remove(id);
        self.felling.remove(id);
        self.hauling.remove(id);
        self.water_trips.remove(id);
        self.labour_kcal.remove(id);
        self.water_drunk_kg.remove(id);
        self.sleep_seconds.remove(id);
    }

    /// Every living human, with the body the labour model costs work against
    /// and whether they are asleep right now.
    fn founders(&self) -> Vec<(String, LabourBody, bool)> {
        self.humans
            .registry
            .iter()
            .filter(|h| matches!(h.profile.status, mk_core::human::HumanStatus::Alive))
            .map(|h| {
                (
                    h.agent_id().to_string(),
                    LabourBody::from_human(h),
                    h.circadian.asleep,
                )
            })
            .collect()
    }

    fn hourly(&mut self) -> Result<(), IslandLifeError> {
        // The estate's plants take their water this hour at the flow they
        // last measured, returning it at their tailraces.
        let cols = self.domain.cols(DomainLevel::Medium);
        self.physical.diversions = self
            .energy
            .hydro
            .iter()
            .map(|plant| {
                let (ir, ic) = plant.site.intake_cell;
                let (tr, tc) = plant.site.powerhouse_cell;
                super::hydrology::Diversion {
                    intake: (ir * cols + ic) as u32,
                    tailrace: (tr * cols + tc) as u32,
                    flow_m3_s: plant.diverted_m3_s(),
                }
            })
            .collect();
        self.physical
            .step(
                &self.canon,
                &self.domain,
                self.scenario.seed,
                self.sim_time_s as f64,
                self.tick,
                self.scenario.cadences.weather_ocean_seconds,
            )
            .map_err(IslandLifeError::Physical)?;
        let size = self.domain.cell_size_m(DomainLevel::Medium);
        let coarse = self.domain.cell_size_m(DomainLevel::Coarse);
        let (row, col) = self.placed.location;
        let (cr, cc) = (
            ((row as f64 + 1.0) * size / coarse) as usize,
            ((col as f64 + 1.0) * size / coarse) as usize,
        );
        let (cr, cc) = (
            cr.min(self.domain.rows(DomainLevel::Coarse) - 1),
            cc.min(self.domain.cols(DomainLevel::Coarse) - 1),
        );
        self.dry_logs(cr, cc)?;
        let toa = *self.physical.insolation.toa_w_m2.get(cr, cc);
        let cloud = (self.physical.weather.precipitation.get(cr, cc) / 5.0).clamp(0.0, 1.0);
        let solar_kw: f64 = self
            .energy
            .solar
            .iter()
            .map(|s| solar_output_kw(s.rated_kw, toa, cloud))
            .sum();
        let founders = self.founders();
        // Lamps: a room is lit while someone in it wants light -- awake and
        // not yet at bedtime -- and less daylight comes through its windows
        // than the lamps give. Nobody deliberates over it; it is what people
        // do with a switch by the door.
        let daylight_lux = crate::perception::daylight_fraction(&self.physical.insolation, cr, cc)
            * crate::humans::circadian::OUTDOOR_DAYLIGHT_LUX;
        let wanted: std::collections::BTreeSet<SpaceId> = self
            .humans
            .registry
            .iter()
            .filter(|h| matches!(h.profile.status, mk_core::human::HumanStatus::Alive))
            .filter(|h| h.circadian.wants_lamp(h.needs.fatigue))
            .filter_map(|h| match self.positions.0.get(h.agent_id())?.space {
                Space::Inside(space) => Some(space),
                Space::Outdoors => None,
            })
            .filter(|space| {
                self.placed
                    .layout
                    .spaces
                    .iter()
                    .find(|s| s.id == *space)
                    .is_some_and(|room| {
                        daylight_lux * super::humans::room_daylight_factor(&room.label)
                            < ROOM_LIGHT_LUX
                    })
            })
            .collect();
        let rooms: Vec<SpaceId> = self.energy.lighting.iter().map(|l| l.space).collect();
        for space in rooms {
            self.energy.set_lit(space, wanted.contains(&space));
        }
        let dt_physical = self.scenario.cadences.weather_ocean_seconds;
        let asleep_met = self
            .labour
            .met(ASLEEP_ACTIVITY)
            .map_err(IslandLifeError::Labour)?;
        let awake_met = self
            .labour
            .met(AWAKE_AT_HOME_ACTIVITY)
            .map_err(IslandLifeError::Labour)?;
        // Each plant runs on the river's present flow at its intake.
        let cols = self.domain.cols(DomainLevel::Medium);
        for plant in &mut self.energy.hydro {
            let (r, c) = plant.site.intake_cell;
            plant.river_m3_s = if r * cols + c < self.physical.hydrology.runoff.data().len() {
                super::hydrology::discharge_m3_s(&self.physical.hydrology, &self.domain, r, c)
            } else {
                0.0
            };
        }
        self.audited(|life, ledger| {
            life.energy.step(dt_physical as f64, solar_kw, ledger);
            for (id, body, _) in &founders {
                // Every second is charged at the MET of the state it was
                // actually spent in, not at whichever state the clock
                // happened to be in when this hour's accounting ran.
                let (asleep_s, awake_s) = life.sleep_seconds.remove(id).unwrap_or((0.0, 0.0));
                let kcal = life
                    .labour
                    .energy_kcal(body, asleep_met, asleep_s)
                    .unwrap_or(0.0)
                    + life
                        .labour
                        .energy_kcal(body, awake_met, awake_s)
                        .unwrap_or(0.0)
                    + life.labour_kcal.remove(id).unwrap_or(0.0);
                if let Ok(kgc) = life.materials.respire(id, kcal, ledger) {
                    *life.respired_since_meal.entry(id.clone()).or_insert(0.0) += kgc;
                }
                *life.lost_since_drink.entry(id.clone()).or_insert(0.0) += kcal * WATER_KG_PER_KCAL;
            }
        })
    }

    /// Run the labour founders have chosen. A founder who chooses to gather
    /// while outdoors on the estate, with the shed's axe to hand, works the
    /// estate's wood:
    ///
    /// - a log lying on the ground that nobody else is moving is hauled to
    ///   the store, the founder's quickest way the estate allows
    ///   (`regional::timber`: the excavator, the ute, or on foot), task by
    ///   task across steps;
    /// - otherwise they fell the tree nearest them: the work carries across
    ///   steps until the felling pack's time for that trunk is met
    ///   (`LabourTable::fell_tree`), and the stem comes down as a green log
    ///   lying where it fell, its carbon and water booked through the
    ///   ledger.
    ///
    /// While they work their needs drain at the MET of what they are doing
    /// (`HumanBeing::labour_met`, read by the next step's lifecycle) and
    /// their respiration is charged for it.
    fn work(&mut self, dt_s: f64) -> Result<(), IslandLifeError> {
        let has_axe = self.estate_has_tool(FELLING_TOOL);
        let awake_met = self
            .labour
            .met(AWAKE_AT_HOME_ACTIVITY)
            .map_err(IslandLifeError::Labour)?;
        let ids: Vec<String> = self
            .humans
            .registry
            .iter()
            .filter(|h| matches!(h.profile.status, mk_core::human::HumanStatus::Alive))
            .map(|h| h.agent_id().to_string())
            .collect();
        let mut felled: Vec<(String, u64)> = Vec::new();
        let mut done_tasks: Vec<(String, HaulTask)> = Vec::new();
        for id in ids {
            let outdoors_at = self.positions.0.get(&id).and_then(|p| match p.space {
                Space::Outdoors => Some(p.position_m),
                Space::Inside(_) => None,
            });
            let human = self
                .humans
                .registry
                .get_human(&id)
                .expect("a listed human exists");
            let gathering = human.economy_action.kind == crate::humans::ActionKind::Gather;
            let body = LabourBody::from_human(human);
            let Some(at) = outdoors_at.filter(|_| has_axe && gathering) else {
                if let Some(h) = self.humans.registry.get_human_mut(&id) {
                    h.labour_met = None;
                }
                continue;
            };

            // A log on the ground nobody is moving: haul it.
            if !self.hauling.contains_key(&id) && !self.felling.contains_key(&id) {
                if let Some(job) = self.plan_haul(at, &body)? {
                    self.hauling.insert(id.clone(), job);
                }
            }
            let met = if let Some(job) = self.hauling.get_mut(&id) {
                job.worked_s += dt_s;
                let met = job.tasks.front().map_or(WALKING_MET, |t| t.met);
                while let Some(task) = job.tasks.front() {
                    if job.worked_s + 1e-9 < task.duration_s {
                        break;
                    }
                    job.worked_s -= task.duration_s;
                    let task = job.tasks.pop_front().expect("a front task");
                    done_tasks.push((id.clone(), task));
                }
                if job.tasks.is_empty() {
                    self.hauling.remove(&id);
                }
                met
            } else {
                // Fell: back to the tree already started, if it still
                // stands; otherwise the nearest.
                let tree = self
                    .felling
                    .get(&id)
                    .and_then(|job| {
                        self.vegetation
                            .trees
                            .iter()
                            .find(|t| t.id == job.tree_id && t.alive)
                    })
                    .or_else(|| self.vegetation.nearest_tree(at))
                    .map(|t| (t.id, t.stem_diameter_m, t.position_m));
                let Some((tree_id, diameter_m, tree_at)) = tree else {
                    if let Some(h) = self.humans.registry.get_human_mut(&id) {
                        h.labour_met = None;
                    }
                    continue;
                };
                let spec = self
                    .labour
                    .fell_tree(LabourTool::SteelAxe, diameter_m * 100.0)
                    .map_err(IslandLifeError::Labour)?;
                // The tree is where it stands: the job starts with the walk
                // to it from wherever the founder is, at the packs' pace.
                let distance_m = ((tree_at.0 - at.0).powi(2) + (tree_at.1 - at.1).powi(2)).sqrt();
                let walk_s = self
                    .labour
                    .walking_time_s(&body, distance_m, self.climb_m(at, tree_at), 0.0)
                    .map_err(IslandLifeError::Labour)?;
                let job = self.felling.entry(id.clone()).or_insert(FellingJob {
                    tree_id,
                    worked_s: 0.0,
                });
                if job.tree_id != tree_id {
                    *job = FellingJob {
                        tree_id,
                        worked_s: 0.0,
                    };
                }
                let met = if job.worked_s < walk_s {
                    WALKING_MET
                } else {
                    spec.met
                };
                job.worked_s += dt_s;
                if job.worked_s >= walk_s + spec.duration_s {
                    felled.push((id.clone(), tree_id));
                }
                met
            };
            let extra = self
                .labour
                .energy_kcal(&body, met, dt_s)
                .map_err(IslandLifeError::Labour)?
                - self
                    .labour
                    .energy_kcal(&body, awake_met, dt_s)
                    .map_err(IslandLifeError::Labour)?;
            *self.labour_kcal.entry(id.clone()).or_insert(0.0) += extra.max(0.0);
            if let Some(h) = self.humans.registry.get_human_mut(&id) {
                h.labour_met = Some(met);
            }
        }
        if felled.is_empty() && done_tasks.is_empty() {
            return Ok(());
        }
        self.audited(|life, ledger| {
            for (id, tree_id) in &felled {
                life.felling.remove(id);
                let Some(tree) = life
                    .vegetation
                    .trees
                    .iter()
                    .find(|t| t.id == *tree_id && t.alive)
                    .cloned()
                else {
                    continue;
                };
                // Conifer stands are softwood; every other forest here is
                // broadleaf.
                let group = match life.vegetation.biome_at(tree.position_m) {
                    Some(mk_core::biomes::BiomeType::BorealForest) => "softwoods",
                    _ => "hardwoods",
                };
                let (Ok(mc), Ok(sg)) = (
                    life.labour.green_moisture_content(group),
                    life.labour.basic_specific_gravity(group),
                ) else {
                    continue;
                };
                if let Ok(wood) = life
                    .vegetation
                    .fell_tree(*tree_id, &mut life.ecology, ledger)
                {
                    let log = timber::felled_log(
                        0,
                        tree.position_m,
                        timber::fall_heading_rad(*tree_id, &life.scenario.seed),
                        tree.stem_diameter_m,
                        tree.height_m,
                        wood.wood_kg,
                        wood.usable_carbon_kgc,
                        mc,
                        sg,
                    );
                    life.materials.receive_felled_log(log, ledger);
                }
            }
            for (id, task) in &done_tasks {
                life.finish_haul_task(task, ledger);
                if let Some(p) = life.positions.0.get_mut(id) {
                    p.space = Space::Outdoors;
                    p.position_m = task.ends_at_m;
                }
            }
        })
    }

    /// The places a haul runs between: the garage's and the building
    /// workshop's outside doors (the workshop is where timber is kept and
    /// used), and how far it is from the workshop's door to the middle of
    /// its floor, where the wood is stacked.
    fn haul_site(&self) -> Option<timber::HaulSite> {
        use crate::organisms::property::PropertyBuildingKind as Kind;
        let layout = &self.placed.layout;
        let door_of = |kind: Kind| {
            let building = layout.buildings.iter().find(|b| b.kind == kind)?;
            let r = &building.rect_m;
            let centre = (0.5 * (r.x0 + r.x1), 0.5 * (r.y0 + r.y1));
            let door = layout
                .doors
                .iter()
                .find(|d| {
                    let inside = |s: &Space| match s {
                        Space::Inside(id) => layout
                            .spaces
                            .iter()
                            .any(|sp| sp.id == *id && sp.building_id == building.building_id),
                        Space::Outdoors => false,
                    };
                    (d.a == Space::Outdoors && inside(&d.b))
                        || (d.b == Space::Outdoors && inside(&d.a))
                })
                .map_or(centre, |d| d.position_m);
            Some((door, centre))
        };
        let (garage_m, _) = door_of(Kind::Garage)?;
        let (store_m, store_centre) = door_of(Kind::Workshop)?;
        Some(timber::HaulSite {
            garage_m,
            store_m,
            store_carry_m: (store_centre.0 - store_m.0).hypot(store_centre.1 - store_m.1),
        })
    }

    /// The climb (m) walking straight from `from` to `to` over the ground:
    /// the estate patch's terrain where the walk crosses it, the medium
    /// grid's elsewhere, sampled every patch cell (§1c routes).
    fn climb_m(&self, from: (f64, f64), to: (f64, f64)) -> f64 {
        let layout = &self.placed.layout;
        let elevation = &self.physical.geophysics.elevation_m;
        let medium_m = self.domain.cell_size_m(DomainLevel::Medium);
        timber::ascent_m(
            |p| layout.ground_m(elevation, medium_m, p),
            from,
            to,
            layout.patch.cell_size_m,
        )
    }

    /// The quickest haul of the log on the ground nearest `at` that no
    /// founder is already moving, or `None` if there is none.
    fn plan_haul(
        &mut self,
        at: (f64, f64),
        body: &LabourBody,
    ) -> Result<Option<timber::HaulJob>, IslandLifeError> {
        let claimed: std::collections::BTreeSet<u64> =
            self.hauling.values().map(|j| j.log_id).collect();
        let Some(log) = self
            .materials
            .logs()
            .filter(|l| l.place == timber::LogPlace::Ground && !claimed.contains(&l.id))
            .min_by(|a, b| {
                let d = |l: &timber::Log| (l.position_m.0 - at.0).hypot(l.position_m.1 - at.1);
                d(a).total_cmp(&d(b)).then(a.id.cmp(&b.id))
            })
            .cloned()
        else {
            return Ok(None);
        };
        let Some(site) = self.haul_site() else {
            return Ok(None);
        };
        let road = |name: &str| {
            let v = self.energy.vehicle_named(name)?;
            Some(timber::RoadVehicle {
                item_id: v.item_id,
                litres: v.litres,
                litres_per_km: self
                    .energy
                    .litres_per_km(v.item_id, super::energy::Surface::OffRoadTrack)?,
            })
        };
        let tracked = |name: &str| {
            let v = self.energy.vehicle_named(name)?;
            Some(timber::TrackedMachine {
                item_id: v.item_id,
                litres: v.litres,
                litres_per_hour: self.energy.litres_per_hour(v.item_id)?,
            })
        };
        let fleet = timber::Fleet {
            ute: road(HAUL_UTE),
            excavator: tracked(HAUL_EXCAVATOR),
        };
        // Pieces cut from this log take ids reserved now, so a plan names
        // them before they exist: as many as the most pieces any plan cuts.
        let carry = self
            .labour
            .sustainable_load_kg(body)
            .map_err(IslandLifeError::Labour)?;
        let bed = self
            .labour
            .ute_bed_length_m()
            .map_err(IslandLifeError::Labour)?;
        let most = (log.green_kg() / carry.max(1e-6))
            .ceil()
            .max((log.length_m / bed.max(1e-6)).ceil())
            .max(1.0) as u64;
        let first = self.materials.reserve_log_ids(most);
        let climb = |from: (f64, f64), to: (f64, f64)| self.climb_m(from, to);
        let plans =
            timber::haul_plans_over(&log, at, body, &self.labour, &site, &fleet, first, &climb)
                .map_err(IslandLifeError::Labour)?;
        Ok(timber::quickest(plans))
    }

    /// What a finished haul task does to the world.
    fn finish_haul_task(&mut self, task: &HaulTask, ledger: &mut Ledger) {
        use timber::{HaulAction, LogPlace};
        let mut put = |ids: &[u64], place: LogPlace, at: Option<(f64, f64)>| {
            for id in ids {
                if let Some(log) = self.materials.log_mut(*id) {
                    log.place = place;
                    if let Some(at) = at {
                        log.position_m = at;
                    }
                }
            }
        };
        match &task.action {
            HaulAction::Walk => {}
            HaulAction::CrossCut {
                log_id,
                pieces,
                first_id,
            } => {
                self.materials.cross_cut_log(*log_id, *pieces, *first_id);
            }
            HaulAction::Load { vehicle_id, logs } => {
                put(logs, LogPlace::Vehicle(*vehicle_id), None);
            }
            HaulAction::Drive {
                vehicle_id,
                km,
                logs,
            } => {
                put(logs, LogPlace::Vehicle(*vehicle_id), Some(task.ends_at_m));
                self.energy.drive_km(
                    *vehicle_id,
                    super::energy::Surface::OffRoadTrack,
                    *km,
                    ledger,
                );
            }
            HaulAction::Unload { logs } => put(logs, LogPlace::Stored, Some(task.ends_at_m)),
            HaulAction::CarryToStore { log_id } => {
                put(&[*log_id], LogPlace::Stored, Some(task.ends_at_m));
            }
            HaulAction::Track {
                vehicle_id,
                hours,
                log,
            } => {
                if let Some(log) = log {
                    put(
                        &[*log],
                        LogPlace::Vehicle(*vehicle_id),
                        Some(task.ends_at_m),
                    );
                }
                self.energy.run_engine_hours(*vehicle_id, *hours, ledger);
            }
            HaulAction::Lift { vehicle_id, log_id } => {
                put(&[*log_id], LogPlace::Vehicle(*vehicle_id), None);
                self.energy
                    .run_engine_hours(*vehicle_id, task.duration_s / 3_600.0, ledger);
            }
            HaulAction::SetDown { log_id } => {
                put(&[*log_id], LogPlace::Stored, Some(task.ends_at_m));
                if let Some(id) = self.energy.vehicle_named(HAUL_EXCAVATOR).map(|v| v.item_id) {
                    self.energy
                        .run_engine_hours(id, task.duration_s / 3_600.0, ledger);
                }
            }
        }
    }

    /// Whether the founders' estate holds a tool of this name.
    fn estate_has_tool(&self, name: &str) -> bool {
        self.placed
            .property
            .properties
            .iter()
            .filter(|p| p.owner_agent_ids.iter().any(|i| i == "Gem-D"))
            .flat_map(|p| p.items.iter())
            .any(|item| item.name == name)
    }

    /// Feed and water every living founder who is eating or drinking right
    /// now — the ones whose own choice this substep was to go and eat
    /// (`humans::lifecycle::is_eating`) or drink (`is_drinking`), the same
    /// choice that lets their needs take in food or water.
    ///
    /// A meal is what its eater has actually burned since the last one, not
    /// a fixed portion: the hourly tick records the carbon each of them
    /// respires, and the harvest asks the food cell for exactly the plant
    /// matter that carbon takes to replace. Intake and expenditure come from
    /// one model, so an adult who eats holds steady by construction, and one
    /// who does not carries the debt until they do. A drink is the same for
    /// water, at [`WATER_KG_PER_KCAL`] of what was burned.
    fn meals(&mut self) -> Result<(), IslandLifeError> {
        let drinkers: Vec<(String, f64)> = self
            .humans
            .registry
            .iter()
            .filter(|h| matches!(h.profile.status, mk_core::human::HumanStatus::Alive))
            .filter(|h| crate::humans::lifecycle::is_drinking(h.economy_action.kind))
            .filter_map(|h| {
                let owed = self.lost_since_drink.get(h.agent_id()).copied()?;
                (owed > 0.0).then(|| (h.agent_id().to_string(), owed))
            })
            .collect();
        let eaters: Vec<(String, f64)> = self
            .humans
            .registry
            .iter()
            .filter(|h| matches!(h.profile.status, mk_core::human::HumanStatus::Alive))
            .filter(|h| crate::humans::lifecycle::is_eating(h.economy_action.kind))
            .filter_map(|h| {
                let owed = self.respired_since_meal.get(h.agent_id()).copied()?;
                (owed > 0.0).then(|| (h.agent_id().to_string(), owed))
            })
            .collect();
        if eaters.is_empty() && drinkers.is_empty() {
            return Ok(());
        }
        let area = self.domain.cell_area_m2(DomainLevel::Medium);
        let carbon_per_kg = composition(Material::PlantFood).carbon_per_kg;
        let (rows, cols) = (
            self.domain.rows(DomainLevel::Medium),
            self.domain.cols(DomainLevel::Medium),
        );
        self.audited(|life, ledger| {
            for (id, owed_kgc) in &eaters {
                // Food is taken where the person stands: the cell under them,
                // else the nearest one around it that has plants.
                let stands = life.standing_cell(id, rows, cols);
                let mut ate = Err(MaterialError::NothingToHarvest);
                for cell in cells_around(stands, FORAGE_RADIUS_CELLS, rows, cols) {
                    ate = life.materials.gather_edible(
                        owed_kgc / carbon_per_kg,
                        &mut life.ecology,
                        cell,
                        area,
                        ledger,
                    );
                    if ate.is_ok() {
                        break;
                    }
                }
                match ate {
                    Ok(kg) => {
                        if life
                            .materials
                            .eat(id, Material::PlantFood, kg, ledger)
                            .is_ok()
                        {
                            let left = (owed_kgc - kg * carbon_per_kg).max(0.0);
                            life.respired_since_meal.insert(id.clone(), left);
                        }
                    }
                    Err(_) => life.shortfalls.food += 1,
                }
            }
            // The body's water in the ledger is what passes through it, not
            // its standing stock, so a drink is drawn, drunk and its owed
            // losses booked together: excretion to the soil, breath and
            // sweat to the air.
            for (id, owed_kg) in &drinkers {
                let stands = life.standing_cell(id, rows, cols);
                // At home, the tap: the tank's water is already in the
                // stock, so drinking it moves it from the tank into the body.
                let mut from_tank = 0.0;
                if life.at_home(stands) && life.materials.is_registered(id) {
                    if let Some(tank) = life.household_water.as_mut() {
                        let kg = tank.draw(*owed_kg);
                        if kg > 0.0 && life.materials.drink(id, kg).is_ok() {
                            from_tank = kg;
                            let _ = life.materials.lose_water(
                                id,
                                kg,
                                EXCRETION_TO_SOIL_FRACTION,
                                ledger,
                            );
                            *life.water_drunk_kg.entry(id.clone()).or_insert(0.0) += kg;
                            life.lost_since_drink
                                .insert(id.clone(), (owed_kg - kg).max(0.0));
                            if kg + 1e-9 >= *owed_kg {
                                continue;
                            }
                        }
                    }
                }
                // Away from home, or the tank is dry. Water where they stand
                // is drunk there; water within reach has to be walked to
                // (realism R1b): a trip that takes the packs' walking time and
                // ends in a drink at the source, so nothing is drunk here.
                let owed_kg = &(*owed_kg - from_tank);
                // At home the estate's own cell is not the tap: its water is
                // still a walk from the house.
                let mut drawn = if life.at_home(stands) {
                    Err(MaterialError::DryCell)
                } else {
                    life.materials.gather_water(
                        *owed_kg,
                        &mut life.physical.hydrology,
                        stands,
                        area,
                        ledger,
                    )
                };
                if drawn.is_ok() {
                    life.water_trips.remove(id);
                } else if life.water_trips.contains_key(id) {
                    continue;
                } else if let Some(source) = life.water_source_near(stands, rows, cols, area) {
                    if life.plan_water_trip(id, stands, source).is_some() {
                        continue;
                    }
                }
                // Nothing wet in reach: dig for water where they stand (the
                // needs model counts soil moisture as water access too).
                if drawn.is_err() {
                    drawn = life.materials.gather_soil_water(
                        *owed_kg,
                        &mut life.physical.hydrology,
                        stands,
                        area,
                        ledger,
                    );
                }
                match drawn {
                    Ok(kg) => {
                        if life.materials.drink(id, kg).is_ok() {
                            let _ = life.materials.lose_water(
                                id,
                                kg,
                                EXCRETION_TO_SOIL_FRACTION,
                                ledger,
                            );
                            *life.water_drunk_kg.entry(id.clone()).or_insert(0.0) += kg;
                            life.lost_since_drink
                                .insert(id.clone(), (owed_kg - kg).max(0.0));
                        }
                    }
                    Err(_) => life.shortfalls.water += 1,
                }
            }
        })
    }

    /// The nearest cell with liquid water within reach of `stands` (and,
    /// near home, the household's stream and the cells beside it), by the
    /// distance between cell centres.
    fn water_source_near(
        &self,
        stands: (usize, usize),
        rows: usize,
        cols: usize,
        area: f64,
    ) -> Option<(usize, usize)> {
        let mut candidates = cells_around(stands, WATER_RADIUS_CELLS, rows, cols);
        if let Some(home) = self.water_cell {
            if stands.0.abs_diff(home.0).max(stands.1.abs_diff(home.1)) <= HOME_RANGE_CELLS {
                candidates.extend(water_search_cells(home, rows, cols));
            }
        }
        let here = self.place_m(stands);
        candidates
            .into_iter()
            .filter(|&cell| {
                super::materials::water_available_kg(&self.physical.hydrology, cell, area) > 0.0
            })
            .min_by(|a, b| {
                let d = |c: &(usize, usize)| {
                    let at = self.domain.cell_center_m(DomainLevel::Medium, c.0, c.1);
                    (at.0 - here.0).hypot(at.1 - here.1)
                };
                d(a).total_cmp(&d(b)).then(a.cmp(b))
            })
    }

    /// Plan `id`'s walk from `stands` to `source`. From home with the
    /// house's jerrycans and room in the tank, they bring back as much as
    /// the cans hold, the tank takes and their body can carry.
    fn plan_water_trip(
        &mut self,
        id: &str,
        stands: (usize, usize),
        source: (usize, usize),
    ) -> Option<()> {
        let human = self.humans.registry.get_human(id)?;
        let body = LabourBody::from_human(human);
        let here = self.place_m(stands);
        let there = self
            .domain
            .cell_center_m(DomainLevel::Medium, source.0, source.1);
        let distance_m = (there.0 - here.0).hypot(there.1 - here.1);
        let cans = self
            .placed
            .property
            .properties
            .iter()
            .filter(|p| p.owner_agent_ids.iter().any(|i| i == "Gem-D"))
            .flat_map(|p| p.items.iter())
            .filter(|item| item.name == crate::organisms::property::WATER_JERRYCAN)
            .count() as f64;
        let mut carry_kg = 0.0;
        let mut back_s = 0.0;
        let mut cans_kg = 0.0;
        if let (true, Some(tank)) = (self.at_home(stands), self.household_water.as_ref()) {
            let can = tank.spec.jerrycan_l;
            let empty_kg = cans * tank.spec.jerrycan_empty_kg;
            let max_kg = self.labour.max_load_kg(&body).ok()?;
            carry_kg = (cans * can)
                .min(tank.room_kg())
                .min((max_kg - empty_kg).max(0.0));
            if carry_kg > 0.0 {
                cans_kg = empty_kg;
                back_s = self
                    .labour
                    .walking_time_s(
                        &body,
                        distance_m,
                        self.climb_m(there, here),
                        carry_kg + empty_kg,
                    )
                    .ok()?;
            }
        }
        // The empty cans go out too, and a light body walks slower for them.
        let out_s = self
            .labour
            .walking_time_s(&body, distance_m, self.climb_m(here, there), cans_kg)
            .ok()?;
        self.water_trips.insert(
            id.to_string(),
            WaterTrip {
                source,
                out_s,
                back_s,
                worked_s: 0.0,
                carry_kg,
                reached: false,
                cans_kg,
            },
        );
        Some(())
    }

    /// Move everyone on a water trip along by `dt_s`: out to the source, a
    /// drink there (and the jerrycans filled), then home. Someone not going
    /// home stays at the water. Walking costs the walking MET above sitting
    /// at home, as felling's walk does, raised by Pandolf's equation for
    /// the cans and the water they carry.
    fn water_trips_step(&mut self, dt_s: f64) -> Result<(), IslandLifeError> {
        if self.water_trips.is_empty() {
            return Ok(());
        }
        let awake_met = self
            .labour
            .met(AWAKE_AT_HOME_ACTIVITY)
            .map_err(IslandLifeError::Labour)?;
        let area = self.domain.cell_area_m2(DomainLevel::Medium);
        let estate = self.placed.location;
        let ids: Vec<String> = self.water_trips.keys().cloned().collect();
        // Who walked, for how long, carrying what (kg).
        let mut walked: Vec<(String, f64, f64)> = Vec::new();
        self.audited(|life, ledger| {
            for id in ids {
                let Some(human) = life.humans.registry.get_human(&id) else {
                    life.water_trips.remove(&id);
                    continue;
                };
                if !matches!(human.profile.status, mk_core::human::HumanStatus::Alive) {
                    life.water_trips.remove(&id);
                    continue;
                }
                let trip = life.water_trips.get_mut(&id).expect("listed");
                trip.worked_s += dt_s;
                let load_kg = trip.cans_kg + if trip.reached { trip.carry_kg } else { 0.0 };
                walked.push((id.clone(), dt_s, load_kg));
                if !trip.reached && trip.worked_s >= trip.out_s {
                    trip.reached = true;
                    let (source, carry) = (trip.source, trip.carry_kg);
                    let owed = life.lost_since_drink.get(&id).copied().unwrap_or(0.0);
                    match life.materials.gather_water(
                        owed,
                        &mut life.physical.hydrology,
                        source,
                        area,
                        ledger,
                    ) {
                        Ok(kg) if life.materials.drink(&id, kg).is_ok() => {
                            let _ = life.materials.lose_water(
                                &id,
                                kg,
                                EXCRETION_TO_SOIL_FRACTION,
                                ledger,
                            );
                            *life.water_drunk_kg.entry(id.clone()).or_insert(0.0) += kg;
                            life.lost_since_drink
                                .insert(id.clone(), (owed - kg).max(0.0));
                        }
                        _ => life.shortfalls.water += 1,
                    }
                    let filled = if carry > 0.0 {
                        life.materials
                            .gather_water(carry, &mut life.physical.hydrology, source, area, ledger)
                            .unwrap_or(0.0)
                    } else {
                        0.0
                    };
                    let trip = life.water_trips.get_mut(&id).expect("listed");
                    trip.carry_kg = filled;
                    if filled <= 0.0 {
                        // Not going home with water: they stay at the source.
                        life.water_trips.remove(&id);
                        if let Some(h) = life.humans.registry.get_human_mut(&id) {
                            h.position.row = source.0 as i32;
                            h.position.col = source.1 as i32;
                        }
                    }
                    continue;
                }
                if trip.reached && trip.worked_s >= trip.out_s + trip.back_s {
                    let carried = trip.carry_kg;
                    life.water_trips.remove(&id);
                    if let Some(tank) = life.household_water.as_mut() {
                        let poured = carried.min(tank.room_kg());
                        tank.stored_kg += poured;
                        tank.fetched_kg += poured;
                        let spilled = carried - poured;
                        if spilled > 0.0 {
                            let _ = life.materials.discharge_water(
                                spilled,
                                &mut life.physical.hydrology,
                                estate,
                                area,
                                ledger,
                            );
                        }
                    }
                }
            }
        })?;
        for (id, s, load_kg) in walked {
            let Some(human) = self.humans.registry.get_human(&id) else {
                continue;
            };
            let body = LabourBody::from_human(human);
            let met = self
                .labour
                .loaded_walking_met(&body, load_kg, WALKING_MET)
                .map_err(IslandLifeError::Labour)?;
            let extra = self
                .labour
                .energy_kcal(&body, met, s)
                .map_err(IslandLifeError::Labour)?
                - self
                    .labour
                    .energy_kcal(&body, awake_met, s)
                    .map_err(IslandLifeError::Labour)?;
            *self.labour_kcal.entry(id).or_insert(0.0) += extra.max(0.0);
        }
        Ok(())
    }

    /// Where a walk from `stands` starts (island metres): the house, for
    /// someone at home; otherwise the cell's centre.
    fn place_m(&self, stands: (usize, usize)) -> (f64, f64) {
        let house = self
            .placed
            .layout
            .buildings
            .iter()
            .find(|b| b.kind == crate::organisms::property::PropertyBuildingKind::House);
        match house {
            Some(h) if self.at_home(stands) => (
                0.5 * (h.rect_m.x0 + h.rect_m.x1),
                0.5 * (h.rect_m.y0 + h.rect_m.y1),
            ),
            _ => self
                .domain
                .cell_center_m(DomainLevel::Medium, stands.0, stands.1),
        }
    }

    /// Whether a medium cell is home: one of the four cells the estate's
    /// patch covers.
    fn at_home(&self, cell: (usize, usize)) -> bool {
        let (row, col) = self.placed.location;
        (row..=row + 1).contains(&cell.0) && (col..=col + 1).contains(&cell.1)
    }

    /// The living who are at home now.
    fn people_at_home(&self) -> usize {
        let (rows, cols) = (
            self.domain.rows(DomainLevel::Medium),
            self.domain.cols(DomainLevel::Medium),
        );
        self.humans
            .registry
            .iter()
            .filter(|h| matches!(h.profile.status, mk_core::human::HumanStatus::Alive))
            .filter(|h| self.at_home(self.standing_cell(h.agent_id(), rows, cols)))
            .count()
    }

    /// Put the typical home's bottled LPG on the kitchen stove, full.
    fn install_household_gas(&mut self) -> Result<(), IslandLifeError> {
        let spec = HouseholdGasSpec::from_reference(self.labour.library())
            .map_err(|e| IslandLifeError::Labour(LabourError::MissingReference(e)))?;
        self.household_gas = Some(HouseholdGas::new(spec));
        Ok(())
    }

    /// The stove for `dt_s`: while anyone is at home it burns the house's
    /// average cooking gas, carbon to the air (D59).
    fn household_gas_step(&mut self, dt_s: f64) -> Result<(), IslandLifeError> {
        if self.household_gas.is_none() {
            return Ok(());
        }
        let anyone_home = self.people_at_home() > 0;
        self.audited(|life, ledger| {
            if let Some(gas) = life.household_gas.as_mut() {
                let kg = gas.cook(anyone_home, dt_s);
                super::energy::book_fossil_carbon(ledger, kg * gas.spec.carbon_per_kg());
            }
        })
    }

    /// Put a rainwater tank on the house: its roof is the house's footprint,
    /// and its size is what the estate's climatology needs for the people
    /// at home (whole tanks, at least one). It starts full, as a lived-in
    /// house's would: the water comes out of the estate cell's soil, where
    /// earlier rain put it. An estate with no house gets no tank.
    /// Survey the island's mineral sites (`regional::resources`).
    fn install_resources(&mut self) -> Result<(), IslandLifeError> {
        let models = super::resources::SecondaryModels::from_reference(self.labour.library())
            .map_err(|e| {
                IslandLifeError::Labour(LabourError::MissingReference(format!("{e:?}")))
            })?;
        self.resources = Some(super::resources::IslandResources::survey(
            &self.physical,
            &self.domain,
            &self.scenario.seed,
            &models,
        ));
        Ok(())
    }

    /// Everyone whose choice is to mine, with the shed's gold pan, works
    /// the placer nearest them by hand (D9): they walk there over the
    /// ground, pan for the packs' working day at the panning MET, walk
    /// home, and the day's gold goes into the store. A job left part-way
    /// is taken up again where it was; a placer worked out ends it.
    fn mine(&mut self, dt_s: f64) -> Result<(), IslandLifeError> {
        if self.resources.is_none() || !self.estate_has_tool(PANNING_TOOL) {
            return Ok(());
        }
        let awake_met = self
            .labour
            .met(AWAKE_AT_HOME_ACTIVITY)
            .map_err(IslandLifeError::Labour)?;
        let panning = self
            .labour
            .pan_gravel(LabourTool::GoldPan, 0.0, false)
            .map_err(IslandLifeError::Labour)?;
        let miners: Vec<(String, (f64, f64), LabourBody)> = self
            .humans
            .registry
            .iter()
            .filter(|h| matches!(h.profile.status, mk_core::human::HumanStatus::Alive))
            .filter(|h| h.economy_action.kind == crate::humans::ActionKind::Mine)
            .filter_map(|h| {
                let at = self
                    .positions
                    .0
                    .get(h.agent_id())
                    .and_then(|p| match p.space {
                        Space::Outdoors => Some(p.position_m),
                        Space::Inside(_) => None,
                    })?;
                Some((h.agent_id().to_string(), at, LabourBody::from_human(h)))
            })
            .collect();
        for (id, at, body) in miners {
            if !self.mining.contains_key(&id) {
                let Some((site_id, there)) = self.nearest_placer(at) else {
                    continue;
                };
                let distance = (there.0 - at.0).hypot(there.1 - at.1);
                let out = self
                    .labour
                    .walking_time_s(&body, distance, self.climb_m(at, there), 0.0)
                    .map_err(IslandLifeError::Labour)?;
                let back = self
                    .labour
                    .walking_time_s(&body, distance, self.climb_m(there, at), 0.0)
                    .map_err(IslandLifeError::Labour)?;
                self.mining.insert(
                    id.clone(),
                    MiningJob {
                        site_id,
                        walk_s: 0.5 * (out + back),
                        worked_s: 0.0,
                    },
                );
            }
            let job = self.mining.get_mut(&id).expect("planned above");
            let at_pan = (job.walk_s..job.walk_s + panning.duration_s).contains(&job.worked_s);
            let met = if at_pan { panning.met } else { WALKING_MET };
            job.worked_s += dt_s;
            let done = job.worked_s >= 2.0 * job.walk_s + panning.duration_s;
            let site_id = job.site_id;
            let extra = self
                .labour
                .energy_kcal(&body, met, dt_s)
                .map_err(IslandLifeError::Labour)?
                - self
                    .labour
                    .energy_kcal(&body, awake_met, dt_s)
                    .map_err(IslandLifeError::Labour)?;
            *self.labour_kcal.entry(id.clone()).or_insert(0.0) += extra.max(0.0);
            if let Some(h) = self.humans.registry.get_human_mut(&id) {
                h.labour_met = Some(met);
            }
            if done {
                self.mining.remove(&id);
                let models =
                    super::resources::SecondaryModels::from_reference(self.labour.library())
                        .map_err(|e| {
                            IslandLifeError::Labour(LabourError::MissingReference(format!("{e:?}")))
                        })?;
                let resources = self.resources.as_mut().expect("checked above");
                // A site worked out while they walked yields nothing.
                if let Ok(work) =
                    resources.work_day(site_id, LabourTool::GoldPan, false, &self.labour, &models)
                {
                    self.materials
                        .bank_mined(super::materials::Material::Gold, work.product_kg)
                        .expect("gold carries no carbon or water");
                }
            }
        }
        Ok(())
    }

    /// The placer gold site nearest `at` (domain metres) that still holds
    /// gravel, and where it is.
    fn nearest_placer(&self, at: (f64, f64)) -> Option<(u32, (f64, f64))> {
        self.resources
            .as_ref()?
            .sites()
            .iter()
            .filter(|s| s.kind == super::resources::SiteKind::PlacerGold && !s.is_exhausted())
            .map(|s| {
                let c = self
                    .domain
                    .cell_center_m(DomainLevel::Medium, s.cell.0, s.cell.1);
                (s.id, c)
            })
            .min_by(|a, b| {
                let d = |p: (f64, f64)| (p.0 - at.0).hypot(p.1 - at.1);
                d(a.1).total_cmp(&d(b.1)).then(a.0.cmp(&b.0))
            })
    }

    fn install_household_water(&mut self) -> Result<(), IslandLifeError> {
        let Some(house) = self
            .placed
            .layout
            .buildings
            .iter()
            .find(|b| b.kind == crate::organisms::property::PropertyBuildingKind::House)
        else {
            return Ok(());
        };
        let roof_m2 =
            (house.rect_m.x1 - house.rect_m.x0).abs() * (house.rect_m.y1 - house.rect_m.y0).abs();
        let spec = HouseholdWaterSpec::from_reference(self.labour.library())
            .map_err(|e| IslandLifeError::Labour(LabourError::MissingReference(e)))?;
        let (row, col) = self.placed.location;
        let year = super::ecology::cell_year(&self.domain, &self.physical, row, col);
        let bins = year.precipitation_mm_day.len() as f64;
        let days_per_bin = self.canon.orbital_period_s / bins / 86_400.0;
        let needed = storage_needed_kg(
            &spec,
            roof_m2,
            self.people_at_home().max(1),
            &year.precipitation_mm_day,
            days_per_bin,
        );
        let mut tank = HouseholdWater::new(spec, roof_m2, installed_capacity_kg(&spec, needed));
        let area = self.domain.cell_area_m2(DomainLevel::Medium);
        let capacity = tank.capacity_kg;
        let mut filled = 0.0;
        self.audited(|life, ledger| {
            filled = life.materials.capture_roof_water(
                capacity,
                &mut life.physical.hydrology,
                (row, col),
                area,
                ledger,
            );
        })?;
        tank.stored_kg = filled;
        self.household_water = Some(tank);
        Ok(())
    }

    /// Lay the soil down under every land cell at RothC's steady state for
    /// its production (`super::soil`, D43). Its temperature modifier is the
    /// cell's year-average of RothC's monthly factor over the climatology.
    /// An hour of the estate's air on every log (D40): the air temperature
    /// now (the day's mean and the hour's departure from it) and its
    /// humidity at the estate's coarse cell `(cr, cc)`.
    fn dry_logs(&mut self, cr: usize, cc: usize) -> Result<(), IslandLifeError> {
        if self.materials.logs().next().is_none() {
            return Ok(());
        }
        let drying = super::drying::WoodDrying::from_reference(self.labour.library())
            .map_err(|e| IslandLifeError::Labour(LabourError::MissingReference(e)))?;
        let t_k = self.physical.climate.surface_temperature.get(cr, cc)
            + self.physical.diurnal_offset_k.get(cr, cc);
        let rh = super::weather::relative_humidity_at(
            &self.canon,
            &self.physical.weather,
            self.physical.vapour_remaining.as_ref(),
            t_k,
            cr,
            cc,
        );
        let dt_s = self.scenario.cadences.weather_ocean_seconds as f64;
        self.audited(|life, ledger| {
            life.materials
                .dry_logs(&drying, t_k - 273.15, rh, dt_s, ledger);
            life.materials.season_stored_logs(ledger);
        })
    }

    fn install_edible(&mut self) -> Result<(), IslandLifeError> {
        let shares = super::edible::ReproductiveShares::from_reference(self.labour.library())
            .map_err(|e| IslandLifeError::Labour(LabourError::MissingReference(e)))?;
        self.ecology.install_edible(&shares);
        Ok(())
    }

    fn install_soil(&mut self) -> Result<(), IslandLifeError> {
        let (params, factor) = soil_for(&self.domain, &self.physical, &self.ecology, &self.labour)?;
        self.ecology.install_soil(params, factor);
        Ok(())
    }

    /// One step of the house's water: rain off the roof into the tank (what
    /// does not fit overflows and stays on the ground), and the household's
    /// washing, flushing and cooking out of it into the ground, as from a
    /// septic tank. Drinking is in [`Self::meals`].
    fn household_water_step(&mut self, dt_s: f64) -> Result<(), IslandLifeError> {
        if self.household_water.is_none() {
            return Ok(());
        }
        let (row, col) = self.placed.location;
        let rain_mm_day = super::levels::sample_coarse_at_medium(
            |r, c| *self.physical.weather.precipitation.get(r, c),
            &self.domain,
            row,
            col,
        );
        let people = self.people_at_home();
        let area = self.domain.cell_area_m2(DomainLevel::Medium);
        self.audited(|life, ledger| {
            let Some(tank) = life.household_water.as_mut() else {
                return;
            };
            let delivered = tank.roof_delivers_kg(rain_mm_day, dt_s);
            let caught = life.materials.capture_roof_water(
                delivered.min(tank.room_kg()),
                &mut life.physical.hydrology,
                (row, col),
                area,
                ledger,
            );
            tank.fill(caught, delivered);
            let (want, rationed) = tank.household_use_kg(people, dt_s);
            if rationed && people > 0 {
                tank.rationed_s += dt_s;
            }
            let used = tank.draw(want);
            if used > 0.0 {
                // The tank's water is in the stock, so this cannot fall short.
                let _ = life.materials.discharge_water(
                    used,
                    &mut life.physical.hydrology,
                    (row, col),
                    area,
                    ledger,
                );
            }
        })
    }

    /// The medium-grid cell a person stands on; the food cell for anyone
    /// the registry does not know.
    fn standing_cell(&self, id: &str, rows: usize, cols: usize) -> (usize, usize) {
        self.humans
            .registry
            .get_human(id)
            .map_or(self.food_cell, |h| {
                (
                    h.position.row.clamp(0, rows as i32 - 1) as usize,
                    h.position.col.clamp(0, cols as i32 - 1) as usize,
                )
            })
    }

    fn six_hourly(&mut self) -> Result<(), IslandLifeError> {
        let dt_ecology = self.scenario.cadences.hydrology_ecology_resource_seconds;
        let cell_area = self.domain.cell_area_m2(DomainLevel::Medium);
        self.ecology.step_with_grazers(dt_ecology as f64, cell_area);
        self.vegetation.step(&self.ecology, dt_ecology);
        Ok(())
    }

    /// A person who died this step: the body's carbon goes to detritus and its
    /// water to the soil (through the audit, so the books must close), and
    /// the person no longer stands anywhere in the estate. Without this a dead
    /// person's carbon would sit in the ledger as human carbon forever.
    fn bury_the_dead(&mut self) -> Result<(), IslandLifeError> {
        let dead: Vec<String> = self
            .humans
            .registry
            .iter()
            .filter(|h| {
                matches!(h.profile.status, mk_core::human::HumanStatus::Dead)
                    && self.materials.is_registered(h.agent_id())
            })
            .map(|h| h.agent_id().to_string())
            .collect();
        if dead.is_empty() {
            return Ok(());
        }
        self.audited(|life, ledger| {
            for id in &dead {
                let _ = life.materials.on_death(id, ledger);
            }
        })?;
        for id in &dead {
            self.positions.0.remove(id);
        }
        Ok(())
    }

    /// A child born during the step joins the household books: its body
    /// enters the materials ledger with carbon taken from its mother, and it
    /// starts where she stands. Without this a newborn never eats, drinks or
    /// respires in the ledger.
    fn adopt_newborns(&mut self) {
        let newcomers: Vec<(String, f64, Option<String>)> = self
            .humans
            .registry
            .iter()
            .filter(|h| {
                matches!(h.profile.status, mk_core::human::HumanStatus::Alive)
                    && !self.materials.is_registered(h.agent_id())
            })
            .map(|h| {
                let mother = mother_of(h);
                (h.agent_id().to_string(), h.body.weight_kg, mother)
            })
            .collect();
        for (id, weight_kg, mother) in newcomers {
            self.materials
                .register_birth(&id, weight_kg, mother.as_deref());
            if let Some(at) = mother
                .as_ref()
                .and_then(|m| self.positions.0.get(m))
                .copied()
            {
                self.positions.0.insert(id, at);
            }
        }
    }

    /// Children under `DEPENDENT_AGE_YEARS` go where their mother goes: they
    /// are carried, then toddle behind her, instead of wandering off alone.
    /// A child whose mother is dead or absent keeps its own movement.
    fn keep_children_with_mothers(&mut self) {
        let following: Vec<(
            String,
            crate::humans::GridPosition,
            Option<super::humans::EstatePosition>,
        )> = self
            .humans
            .registry
            .iter()
            .filter(|h| h.development.age_years < DEPENDENT_AGE_YEARS)
            .filter_map(|h| {
                let mother = self.humans.registry.get_human(&mother_of(h)?)?;
                matches!(mother.profile.status, mk_core::human::HumanStatus::Alive).then(|| {
                    (
                        h.agent_id().to_string(),
                        mother.position,
                        self.positions.0.get(mother.agent_id()).copied(),
                    )
                })
            })
            .collect();
        for (id, grid, estate) in following {
            if let Some(child) = self.humans.registry.get_human_mut(&id) {
                child.set_runtime_position(grid);
            }
            match estate {
                Some(at) => {
                    self.positions.0.insert(id, at);
                }
                None => {
                    self.positions.0.remove(&id);
                }
            }
        }
    }

    /// Advance by `seconds` of simulated time in human substeps; a
    /// remainder shorter than a substep is carried to the next call.
    pub fn advance(&mut self, seconds: u64) -> Result<(), IslandLifeError> {
        let steps = self.scheduler.take_substeps(seconds);
        let human_s = self.scenario.cadences.human_seconds;
        for _ in 0..steps {
            self.sim_time_s += human_s;
            self.tick += 1;
            let started = std::time::Instant::now();
            {
                let energy = self.energy.clone();
                let ctx = RegionalHumanContext {
                    computer_bridge: self.computer_bridge.as_ref().map(|h| h.as_bridge()),
                    property: &self.placed.property,
                    layout: &self.placed.layout,
                    physical: &self.physical,
                    ecology: &self.ecology,
                    energy: &energy,
                    domain: &self.domain,
                    solar_kw: 0.0,
                };
                step_regional_humans(
                    &mut self.humans,
                    &mut self.positions,
                    &mut self.economy,
                    &ctx,
                    &self.topology,
                    self.tick,
                    human_s,
                    &self.rng,
                );
            }
            let humans_done = std::time::Instant::now();
            self.timings.humans += humans_done - started;
            self.bury_the_dead()?;
            self.keep_children_with_mothers();
            self.adopt_newborns();
            self.timings.households += humans_done.elapsed();
            // Taken out and put back so the registry can be read while the
            // tally is written, without rebuilding a body and cloning an
            // agent id per human per substep just to look at one bool.
            let mut spent = std::mem::take(&mut self.sleep_seconds);
            // Taken out for the same reason, and only when records are being
            // kept: an island with no store walks this loop exactly as it
            // did before.
            let mut store = self.human_store.take();
            let mut died: Vec<String> = Vec::new();
            // Accumulators belonging to people who are no longer alive. Both
            // are hashed, and `hourly()` only clears the living, so without
            // this a dead founder's last unspent hour and unpaid meal would
            // sit in the digest for the rest of the run and the maps would
            // grow with every death the island ever has.
            let mut forget: Vec<String> = Vec::new();
            for human in self.humans.registry.iter() {
                let alive = matches!(human.profile.status, mk_core::human::HumanStatus::Alive);
                if let Some(store) = store.as_mut() {
                    store.note_status(human.agent_id(), alive, &mut died);
                }
                if !alive {
                    if spent.remove(human.agent_id()).is_some()
                        || self.respired_since_meal.contains_key(human.agent_id())
                        || self.lost_since_drink.contains_key(human.agent_id())
                        || self.felling.contains_key(human.agent_id())
                        || self.hauling.contains_key(human.agent_id())
                        || self.labour_kcal.contains_key(human.agent_id())
                    {
                        forget.push(human.agent_id().to_string());
                    }
                    continue;
                }
                let entry = match spent.get_mut(human.agent_id()) {
                    Some(entry) => entry,
                    None => spent
                        .entry(human.agent_id().to_string())
                        .or_insert((0.0, 0.0)),
                };
                if human.circadian.asleep {
                    entry.0 += human_s as f64;
                } else {
                    entry.1 += human_s as f64;
                }
            }
            self.sleep_seconds = spent;
            self.human_store = store;
            for id in forget {
                // The dead respire nothing and are owed no meal. Dropping
                // these moves no carbon: the ledger sent their body's carbon
                // to detritus when they died, and these only ever recorded
                // what the next hour and the next meal would have cost.
                self.respired_since_meal.remove(&id);
                self.lost_since_drink.remove(&id);
                self.felling.remove(&id);
                self.mining.remove(&id);
                self.hauling.remove(&id);
                self.water_trips.remove(&id);
                self.labour_kcal.remove(&id);
            }
            self.work(human_s as f64)?;
            self.mine(human_s as f64)?;
            self.household_water_step(human_s as f64)?;
            self.household_gas_step(human_s as f64)?;
            self.meals()?;
            self.water_trips_step(human_s as f64)?;
            // A death is written at once: every other record can lag a
            // cadence and catch up, but a dead person's folder never will.
            for id in died {
                self.sync_one_human(&id);
            }
            let due = self.scheduler.due(self.sim_time_s);
            if due.weather_ocean {
                let t = std::time::Instant::now();
                self.hourly()?;
                self.timings.physics_hourly += t.elapsed();
            }
            if due.hydrology_ecology_resource {
                let t = std::time::Instant::now();
                self.six_hourly()?;
                self.timings.ecology_six_hourly += t.elapsed();
            }
            if due.human_store {
                let t = std::time::Instant::now();
                self.sync_humans();
                self.timings.human_store += t.elapsed();
            }
            self.timings.substeps += 1;
            self.scheduler.record(due);
        }
        Ok(())
    }

    /// Whether a founder is inside the estate layout (has a metric position
    /// in a space or outdoors in the patch).
    pub fn in_estate(&self, agent_id: &str) -> bool {
        self.positions
            .0
            .get(agent_id)
            .is_some_and(|p| match p.space {
                Space::Inside(_) | Space::Outdoors => self.placed.layout.patch.width_m > 0.0,
            })
    }

    /// A canonical digest of the whole state: serialize to `serde_json::Value`
    /// (whose maps are sorted, so `HashMap`-backed upstream state hashes the
    /// same in every process) and hash the bytes.
    pub fn state_digest(&self) -> [u8; 32] {
        let mut h = blake3::Hasher::new();
        h.update(&self.physical.state_hash());
        let mut feed = |xs: &[f64]| {
            for x in xs {
                h.update(&x.to_le_bytes());
            }
        };
        feed(self.ecology.biomass_kgc_m2.data());
        feed(self.ecology.npp_kgc_m2_yr.data());
        feed(self.ecology.herbivore_kgc_m2.data());
        if let Some(soil) = &self.ecology.soil {
            for pools in &soil.pools {
                feed(pools);
            }
        }
        for t in &self.vegetation.trees {
            feed(&[
                t.position_m.0,
                t.position_m.1,
                t.biomass_kgc,
                t.stem_diameter_m,
                t.height_m,
            ]);
        }
        feed(
            &self
                .vegetation
                .stands
                .data()
                .iter()
                .map(|s| s.biomass_kgc)
                .collect::<Vec<_>>(),
        );
        let canonical = |v: &dyn erased::Json| v.value().to_string();
        // One human at a time, so a large population's digest never holds
        // the whole system as one `serde_json::Value`.
        self.humans.canonical_chunks(|bytes| {
            h.update(bytes);
        });
        h.update(canonical(&self.positions).as_bytes());
        h.update(canonical(&self.energy).as_bytes());
        h.update(canonical(&self.economy).as_bytes());
        h.update(format!("{:?}", self.materials).as_bytes());
        h.update(canonical(&self.scheduler).as_bytes());
        // Both of these decide what happens next — the size of the next meal
        // and the cost of the next hour — so a digest without them would call
        // two states identical and then watch them diverge.
        h.update(canonical(&self.respired_since_meal).as_bytes());
        h.update(canonical(&self.lost_since_drink).as_bytes());
        h.update(canonical(&self.felling).as_bytes());
        h.update(canonical(&self.resources).as_bytes());
        h.update(canonical(&self.mining).as_bytes());
        h.update(canonical(&self.hauling).as_bytes());
        h.update(canonical(&self.labour_kcal).as_bytes());
        h.update(canonical(&self.water_drunk_kg).as_bytes());
        h.update(canonical(&self.household_water).as_bytes());
        h.update(canonical(&self.household_gas).as_bytes());
        h.update(canonical(&self.water_trips).as_bytes());
        h.update(canonical(&self.sleep_seconds).as_bytes());
        h.update(&self.sim_time_s.to_le_bytes());
        // Deliberately absent, and each for a reason that must stay true:
        // the placed estate is only ever read after bootstrap, the food and
        // water cells are chosen once, `RngRegistry` carries nothing but its
        // seed, and the audit and shortfall counts record what happened
        // rather than deciding what happens next. Anything here that starts
        // changing during a run belongs in the digest, and
        // `the_digest_notices_state_that_only_matters_later` is where that
        // gets caught.
        *h.finalize().as_bytes()
    }
}

mod erased {
    pub trait Json {
        fn value(&self) -> serde_json::Value;
    }
    impl<T: serde::Serialize> Json for T {
        fn value(&self) -> serde_json::Value {
            serde_json::to_value(self).expect("state serialises")
        }
    }
}

/// Connect the estate's micro-hydro plant, if its placement found one, by
/// an overhead line from the powerhouse cell's centre to the estate block's
/// centre (the shared corner of its four cells).
fn attach_estate_hydro(energy: &mut EstateEnergy, placed: &PlacedEstate, domain: &IslandDomain) {
    let Some(hydro) = &placed.hydro else {
        return;
    };
    let medium = DomainLevel::Medium;
    let size = domain.cell_size_m(medium);
    let (row, col) = placed.location;
    let (bx, by) = ((col + 1) as f64 * size, (row + 1) as f64 * size);
    let (px, py) = domain.cell_center_m(
        medium,
        hydro.estate.powerhouse_cell.0,
        hydro.estate.powerhouse_cell.1,
    );
    energy.attach_hydro(hydro.estate.clone(), (px - bx).hypot(py - by) / 1_000.0);
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::path::PathBuf;

    fn repo(path: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(path)
    }

    /// A founder who chooses to mine, with the shed's gold pan, walks to
    /// the nearest placer, pans the packs' working day and walks home
    /// (D9): the job takes exactly that long, it is labour throughout, the
    /// day's gold lands in the store and the gravel leaves the site.
    #[test]
    fn a_founder_with_the_pan_works_the_nearest_placer_for_a_day() {
        let mut scenario =
            IslandScenario::load(&repo("fixtures/island/default_scenario.json")).unwrap();
        scenario.estate_patch.tree_cap = 200;
        let canon = Arc::new(CanonLocked::load(&repo("fixtures/island/canon.json")).unwrap());
        let mut life = IslandLife::bootstrap(scenario, canon).expect("the island bootstraps");
        assert!(
            life.estate_has_tool(PANNING_TOOL),
            "the shed has the gold pan"
        );

        let at = life.vegetation.centre_m();
        life.positions.0.insert(
            "Gem-D".into(),
            super::super::humans::EstatePosition {
                space: Space::Outdoors,
                position_m: at,
            },
        );
        life.humans
            .registry
            .get_human_mut("Gem-D")
            .unwrap()
            .economy_action
            .kind = crate::humans::ActionKind::Mine;
        let (site_id, there) = life.nearest_placer(at).expect("the island has a placer");
        let before_t = life
            .resources
            .as_ref()
            .unwrap()
            .site(site_id)
            .unwrap()
            .remaining_t;
        let body = LabourBody::from_human(life.humans.registry.get_human("Gem-D").unwrap());
        let distance = (there.0 - at.0).hypot(there.1 - at.1);
        let day = life
            .labour
            .pan_gravel(LabourTool::GoldPan, 0.0, false)
            .unwrap()
            .duration_s;
        let needed = life
            .labour
            .walking_time_s(&body, distance, life.climb_m(at, there), 0.0)
            .unwrap()
            + day
            + life
                .labour
                .walking_time_s(&body, distance, life.climb_m(there, at), 0.0)
                .unwrap();

        let step = 600.0;
        let mut worked = 0.0;
        life.mine(step).unwrap();
        worked += step;
        assert_eq!(life.mining.get("Gem-D").map(|j| j.site_id), Some(site_id));
        while life.mining.contains_key("Gem-D") {
            assert!(
                worked < needed + step,
                "still out after {worked} s of {needed}"
            );
            assert!(
                life.humans
                    .registry
                    .get_human("Gem-D")
                    .unwrap()
                    .labour_met
                    .is_some(),
                "mining is labour"
            );
            life.mine(step).unwrap();
            worked += step;
        }
        assert!(
            worked >= needed,
            "home after {worked} s, before its {needed}"
        );

        let site = life.resources.as_ref().unwrap().site(site_id).unwrap();
        assert!(site.remaining_t < before_t, "no gravel left the site");
        let gold = life.materials.stock_kg(Material::Gold);
        assert!(gold > 0.0, "a day at the pan banked no gold");
        // No more than the gravel moved held at the site's grade (g/t).
        let moved_t = before_t - site.remaining_t;
        assert!(
            gold * 1_000.0 <= site.grade * moved_t + 1e-9,
            "{gold} kg of gold"
        );
        // The job is in the snapshot and the digest sees the store.
        let digest = life.state_digest();
        life.mining.insert(
            "Gem-K".into(),
            MiningJob {
                site_id,
                walk_s: 1.0,
                worked_s: 0.0,
            },
        );
        assert_ne!(
            life.state_digest(),
            digest,
            "the digest does not see a mining job"
        );
        assert_eq!(life.snapshot().mining, life.mining);
    }

    /// A founder who gathers outdoors with the shed's axe walks to the
    /// nearest tree and fells it after exactly the walk plus the felling
    /// pack's time for that trunk; its usable wood lands in the estate's
    /// stock and the books close.
    #[test]
    fn a_founder_with_the_axe_fells_the_nearest_tree_in_the_packs_time() {
        let mut scenario =
            IslandScenario::load(&repo("fixtures/island/default_scenario.json")).unwrap();
        scenario.estate_patch.tree_cap = 200;
        let canon = Arc::new(CanonLocked::load(&repo("fixtures/island/canon.json")).unwrap());
        let mut life = IslandLife::bootstrap(scenario, canon).expect("the island bootstraps");
        assert!(life.estate_has_tool(FELLING_TOOL), "the shed has the axe");

        // Gem-D outdoors at the patch's centre, choosing to gather.
        let at = life.vegetation.centre_m();
        life.positions.0.insert(
            "Gem-D".into(),
            super::super::humans::EstatePosition {
                space: Space::Outdoors,
                position_m: at,
            },
        );
        life.humans
            .registry
            .get_human_mut("Gem-D")
            .unwrap()
            .economy_action
            .kind = crate::humans::ActionKind::Gather;
        let tree = life.vegetation.nearest_tree(at).expect("a tree").clone();
        let body = LabourBody::from_human(life.humans.registry.get_human("Gem-D").unwrap());
        let distance =
            ((tree.position_m.0 - at.0).powi(2) + (tree.position_m.1 - at.1).powi(2)).sqrt();
        let needed = life
            .labour
            .walking_time_s(&body, distance, life.climb_m(at, tree.position_m), 0.0)
            .unwrap()
            + life
                .labour
                .fell_tree(LabourTool::SteelAxe, tree.stem_diameter_m * 100.0)
                .unwrap()
                .duration_s;
        let audits = life.audits_closed;
        let minute = 60.0;
        let mut worked = 0.0;
        while life.vegetation.trees.iter().any(|t| t.id == tree.id) {
            assert!(
                worked < needed + minute,
                "still standing after {worked} s of {needed}"
            );
            life.work(minute).unwrap();
            worked += minute;
            assert!(
                life.humans
                    .registry
                    .get_human("Gem-D")
                    .unwrap()
                    .labour_met
                    .is_some(),
                "working is labour"
            );
        }
        assert!(
            worked >= needed,
            "felled after {worked} s, before its {needed}"
        );
        // The usable wood is on the ground where the tree fell, green.
        let logs: Vec<timber::Log> = life.materials.logs().cloned().collect();
        assert_eq!(logs.len(), 1, "one log came down");
        let log = &logs[0];
        assert_eq!(log.place, timber::LogPlace::Ground);
        assert!((log.carbon_kgc - tree.biomass_kgc * 0.7).abs() < 1e-9);
        let mc = life.labour.green_moisture_content("hardwoods").unwrap();
        assert!((log.moisture_content() - mc).abs() < 1e-12, "green wood");
        let butt = log.butt_m();
        assert!((butt.0 - tree.position_m.0).abs() < 1e-9);
        assert!((butt.1 - tree.position_m.1).abs() < 1e-9);
        assert_eq!(life.materials.stock_kg(Material::Wood), 0.0);
        assert_eq!(
            life.audits_closed,
            audits + 1,
            "the felling closed its books"
        );
        assert!(!life.felling.contains_key("Gem-D"), "the felling is done");
        assert!(
            *life.labour_kcal.get("Gem-D").unwrap() > 0.0,
            "the work cost energy"
        );

        // Keep working: Gem-D moves the log to the store, the quickest way.
        let (carbon, water) = (log.carbon_kgc, log.water_kg);
        life.work(minute).unwrap();
        let job = life
            .hauling
            .get("Gem-D")
            .expect("a haul is planned")
            .clone();
        let total = job.total_s();
        let mut hauled = minute;
        while life.hauling.contains_key("Gem-D") {
            assert!(
                hauled < total + 2.0 * minute,
                "still hauling after {hauled} s of {total}"
            );
            life.work(minute).unwrap();
            hauled += minute;
        }
        let stored: Vec<&timber::Log> = life.materials.logs().collect();
        assert!(stored.iter().all(|l| l.place == timber::LogPlace::Stored));
        let sum = |f: fn(&timber::Log) -> f64| stored.iter().map(|l| f(l)).sum::<f64>();
        assert!(
            (sum(|l| l.carbon_kgc) - carbon).abs() < 1e-9,
            "no carbon lost"
        );
        assert!(
            (sum(|l| l.water_kg) - water).abs() < 1e-9,
            "the stored wood is still green"
        );
        let site = life.haul_site().unwrap();
        assert!(stored.iter().all(|l| l.position_m == site.store_m));
        println!(
            "{:?} hauled {} piece(s), {:.0} kg green, in {:.0} s",
            job.method,
            stored.len(),
            sum(|l| l.dry_kg) + water,
            total
        );
    }

    /// A dead founder's unspent hour and unpaid meal are dropped rather than
    /// carried for the rest of the run.
    ///
    /// Both maps are hashed into the state digest and `hourly()` only clears
    /// the living, so without this every death the island ever has would
    /// leave a permanent entry behind — growing the maps without bound and
    /// hashing state that stopped meaning anything the moment its owner
    /// died. Found in review on PR #3.
    #[test]
    fn the_dead_stop_accumulating_hours_and_meals() {
        let mut scenario =
            IslandScenario::load(&repo("fixtures/island/default_scenario.json")).unwrap();
        scenario.estate_patch.tree_cap = 200;
        let canon = Arc::new(CanonLocked::load(&repo("fixtures/island/canon.json")).unwrap());
        let mut life = IslandLife::bootstrap(scenario, canon).expect("the island bootstraps");

        // Two hours so both accumulators have something in them: the sleep
        // tally fills every substep, the meal debt on the hour.
        life.advance(2 * 3_600).unwrap();
        assert!(
            life.respired_since_meal.contains_key("Gem-K"),
            "nobody respired, so there is nothing to test"
        );
        assert!(life.lost_since_drink.contains_key("Gem-K"));

        life.humans
            .registry
            .get_human_mut("Gem-K")
            .unwrap()
            .profile
            .status = mk_core::human::HumanStatus::Dead;
        life.advance(life.scenario.cadences.human_seconds).unwrap();

        assert!(
            !life.sleep_seconds.contains_key("Gem-K"),
            "a dead founder is still banking hours"
        );
        assert!(
            !life.respired_since_meal.contains_key("Gem-K"),
            "a dead founder is still owed a meal"
        );
        assert!(
            !life.lost_since_drink.contains_key("Gem-K"),
            "a dead founder is still owed a drink"
        );
        // And the living are untouched.
        assert!(life.sleep_seconds.contains_key("Gem-D"));
    }

    #[test]
    fn both_home_activities_are_named_in_the_reference_packs() {
        // A typo here would not fail to compile; it would fail the tick at
        // runtime, so the names are checked against the pack itself.
        let table = LabourTable::load_default().expect("the labour packs load");
        let asleep = table
            .met(ASLEEP_ACTIVITY)
            .expect("sleeping is in the packs");
        let awake = table
            .met(AWAKE_AT_HOME_ACTIVITY)
            .expect("sitting quietly is in the packs");
        assert!(
            asleep < awake,
            "sleeping ({asleep}) should cost less than sitting quietly ({awake})"
        );
    }
}
