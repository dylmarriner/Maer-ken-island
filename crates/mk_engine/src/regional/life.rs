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
use super::energy::{solar_output_kw, EstateEnergy};
use super::estate_layout::Space;
use super::human_store::{open_run, HumanStore, HumanStoreError, RunId};
use super::humans::{
    bootstrap_regional_humans, step_regional_humans, HumanEstatePositions, RegionalHumanContext,
    RegionalHumanError,
};
use super::hydrology::discharge_m3_s;
use super::labour::{LabourBody, LabourError, LabourTable};
use super::local_vegetation::{seed_local_vegetation, LocalVegetationError, LocalVegetationPatch};
use super::materials::{composition, Material, MaterialError, MaterialLedger};
use super::physical::{RegionalPhysicalError, RegionalPhysicalState};
use super::property::{place_regional_estate, PlaceEstateError, PlacedEstate};
use super::scheduler::IslandScheduler;
use crate::humans::HumanSystem;
use crate::resource_economy::ResourceEconomyState;
use crate::topology::GridTopology;

/// What a founder at home is doing, as the reference packs' activity names.
/// A sleeping body burns measurably less than a waking one (Compendium of
/// Physical Activities: sleeping 0.9-1.0 MET against 1.0-1.3 sitting
/// quietly), and the circadian clock already knows which of the two this
/// human is doing, so the tick asks it rather than assuming one rate all
/// day. On a 36-hour day that difference is not academic: sleep is already
/// short and broken here (D23), and expenditure has to follow it.
const ASLEEP_ACTIVITY: &str = "sleeping";
const AWAKE_AT_HOME_ACTIVITY: &str = "sitting_quietly";

const WATER_KG_PER_DRINK: f64 = 0.65;
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
    pub sleep_seconds: BTreeMap<String, (f64, f64)>,
    pub food_cell: (usize, usize),
    pub water_cell: Option<(usize, usize)>,
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
    food_cell: (usize, usize),
    water_cell: Option<(usize, usize)>,
    topology: GridTopology,
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

impl IslandLife {
    /// Bootstrap the whole island from a scenario: physics, ecology, the
    /// estate and its trees, and the founders at home.
    pub fn bootstrap(
        scenario: IslandScenario,
        canon: Arc<CanonLocked>,
    ) -> Result<Self, IslandLifeError> {
        let domain =
            IslandDomain::from_profile(scenario.profile.clone()).expect("a validated scenario");
        let physical = RegionalPhysicalState::bootstrap(&canon, &domain, scenario.seed)
            .map_err(IslandLifeError::Physical)?;
        let ecology = RegionalEcologyState::bootstrap(&domain, &physical)
            .map_err(IslandLifeError::Ecology)?;
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
        let energy = EstateEnergy::from_config(&scenario.estate_energy, estate);

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
            sleep_seconds: BTreeMap::new(),
            scheduler: IslandScheduler::new(scenario_cadences),
            labour: LabourTable::load_default().map_err(IslandLifeError::Labour)?,
            human_store: None,
            // Filled in below: the digest needs the built island.
            replay_log: super::replay::IslandReplayLog::new(String::new()),
            food_cell,
            water_cell,
            topology,
        };
        // A log says which scenario it belongs to, so replaying it against
        // a different island refuses rather than reproducing nothing and
        // looking like it had.
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
            sleep_seconds: self.sleep_seconds.clone(),
            food_cell: self.food_cell,
            water_cell: self.water_cell,
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
            sleep_seconds: snapshot.sleep_seconds,
            labour: LabourTable::load_default().map_err(IslandLifeError::Labour)?,
            // A restored island keeps no records until someone attaches
            // some: the folders belong to the run that wrote them, and a
            // reload is a new run.
            human_store: None,
            // A restored island starts a fresh log: the commands that got
            // it here are in the log beside the snapshot, not in the file.
            // Its scenario is filled in below, as at bootstrap.
            replay_log: super::replay::IslandReplayLog::new(String::new()),
            food_cell: snapshot.food_cell,
            water_cell: snapshot.water_cell,
            topology,
        };
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
    /// The two of them are hashed into the state digest, so a person who
    /// has left the world has to leave these too or two islands that agree
    /// about who is alive will disagree about their digest. `advance`
    /// already does this for the dead; a removal needs the same.
    pub(super) fn forget_accumulators(&mut self, id: &str) {
        self.respired_since_meal.remove(id);
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
        let toa = *self.physical.insolation.toa_w_m2.get(cr, cc);
        let cloud = (self.physical.weather.precipitation.get(cr, cc) / 5.0).clamp(0.0, 1.0);
        let solar_kw: f64 = self
            .energy
            .solar
            .iter()
            .map(|s| solar_output_kw(s.rated_kw, toa, cloud))
            .sum();
        let founders = self.founders();
        let dt_physical = self.scenario.cadences.weather_ocean_seconds;
        let asleep_met = self
            .labour
            .met(ASLEEP_ACTIVITY)
            .map_err(IslandLifeError::Labour)?;
        let awake_met = self
            .labour
            .met(AWAKE_AT_HOME_ACTIVITY)
            .map_err(IslandLifeError::Labour)?;
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
                        .unwrap_or(0.0);
                if let Ok(kgc) = life.materials.respire(id, kcal, ledger) {
                    *life.respired_since_meal.entry(id.clone()).or_insert(0.0) += kgc;
                }
            }
        })
    }

    fn six_hourly(&mut self) -> Result<(), IslandLifeError> {
        let dt_ecology = self.scenario.cadences.hydrology_ecology_resource_seconds;
        self.ecology.step(dt_ecology as f64, None, None);
        self.vegetation.step(&self.ecology, dt_ecology);
        let founders = self.founders();
        let area = self.domain.cell_area_m2(DomainLevel::Medium);
        self.audited(|life, ledger| {
            for (id, _, _) in &founders {
                // A meal is what this human has actually burned since the
                // last one, not a fixed portion: the hourly tick records the
                // carbon each of them respires, and the harvest asks the food
                // cell for exactly the plant matter that carbon takes to
                // replace. Intake and expenditure therefore come from one
                // model instead of two, and an adult's body carbon holds
                // steady by construction rather than by a constant that
                // happened to be close.
                //
                // Which human harvests is still the routine's choice, not
                // theirs — see D10.
                let owed_kgc = life.respired_since_meal.get(id).copied().unwrap_or(0.0);
                let wanted_kg = owed_kgc / composition(Material::PlantFood).carbon_per_kg;
                if wanted_kg > 0.0 {
                    match life.materials.gather_biotic(
                        Material::PlantFood,
                        wanted_kg,
                        &mut life.ecology,
                        life.food_cell,
                        area,
                        ledger,
                    ) {
                        Ok(kg) => {
                            if life
                                .materials
                                .eat(id, Material::PlantFood, kg, ledger)
                                .is_ok()
                            {
                                let eaten_kgc = kg * composition(Material::PlantFood).carbon_per_kg;
                                let left = (owed_kgc - eaten_kgc).max(0.0);
                                life.respired_since_meal.insert(id.clone(), left);
                            }
                        }
                        Err(_) => life.shortfalls.food += 1,
                    }
                }
                {
                    let drawn = life
                        .water_cell
                        .ok_or(MaterialError::DryCell)
                        .and_then(|cell| {
                            life.materials.gather_water(
                                WATER_KG_PER_DRINK,
                                &mut life.physical.hydrology,
                                cell,
                                area,
                                ledger,
                            )
                        });
                    match drawn {
                        Ok(kg) => {
                            let _ = life.materials.drink(id, kg);
                            let _ = life.materials.lose_water(
                                id,
                                kg,
                                EXCRETION_TO_SOIL_FRACTION,
                                ledger,
                            );
                        }
                        Err(_) => life.shortfalls.water += 1,
                    }
                }
            }
        })
    }

    /// Advance by `seconds` of simulated time in human substeps; a
    /// remainder shorter than a substep is carried to the next call.
    pub fn advance(&mut self, seconds: u64) -> Result<(), IslandLifeError> {
        let steps = self.scheduler.take_substeps(seconds);
        let human_s = self.scenario.cadences.human_seconds;
        for _ in 0..steps {
            self.sim_time_s += human_s;
            self.tick += 1;
            {
                let energy = self.energy.clone();
                let ctx = RegionalHumanContext {
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
            }
            // A death is written at once: every other record can lag a
            // cadence and catch up, but a dead person's folder never will.
            for id in died {
                self.sync_one_human(&id);
            }
            let due = self.scheduler.due(self.sim_time_s);
            if due.weather_ocean {
                self.hourly()?;
            }
            if due.hydrology_ecology_resource {
                self.six_hourly()?;
            }
            if due.human_store {
                self.sync_humans();
            }
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
        h.update(canonical(&self.humans).as_bytes());
        h.update(canonical(&self.positions).as_bytes());
        h.update(canonical(&self.energy).as_bytes());
        h.update(canonical(&self.economy).as_bytes());
        h.update(format!("{:?}", self.materials).as_bytes());
        h.update(canonical(&self.scheduler).as_bytes());
        // Both of these decide what happens next — the size of the next meal
        // and the cost of the next hour — so a digest without them would call
        // two states identical and then watch them diverge.
        h.update(canonical(&self.respired_since_meal).as_bytes());
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

#[cfg(test)]
mod tests {
    use super::*;

    use std::path::PathBuf;

    fn repo(path: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(path)
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
