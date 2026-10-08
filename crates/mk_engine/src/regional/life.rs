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

use std::sync::Arc;

use mk_core::canon::CanonLocked;
use mk_core::flux::{FluxKind, Ledger, Reservoir};
use mk_core::rng::RngRegistry;
use mk_island::{DomainLevel, IslandDomain, IslandScenario, LocalPatchSpec};
use serde::{Deserialize, Serialize};

use super::commands::{
    as_control, validate, CommandError, CommandState, ControlCommand, CreateLocation,
    IslandCommand, IslandCreateHumanRequest, IslandReplayEntry, QueuedCommandId,
};
use super::ecology::{RegionalEcologyError, RegionalEcologyState};
use super::energy::{solar_output_kw, EstateEnergy};
use super::estate_layout::{EstateLayout, Space};
use super::humans::{
    bootstrap_regional_humans, step_regional_humans, HumanEstatePositions, RegionalHumanContext,
    RegionalHumanError,
};
use super::hydrology::discharge_m3_s;
use super::labour::{LabourBody, LabourError, LabourTable};
use super::local_vegetation::{seed_local_vegetation, LocalVegetationError, LocalVegetationPatch};
use super::materials::{cells_around, water_search_cells, Material, MaterialError, MaterialLedger};
use super::physical::{PhysicalSnapshot, RegionalPhysicalError, RegionalPhysicalState};
use super::property::{place_regional_estate, PlaceEstateError, PlacedEstate};
use super::scheduler::IslandScheduler;
use crate::humans::HumanSystem;
use crate::organisms::property::PropertySystem;
use crate::resource_economy::ResourceEconomyState;
use crate::topology::GridTopology;

/// Light daily activity while the founders live at home (MET).
const HOME_MET: f64 = 1.3;
/// Food and water each founder takes per 6 hours: ~2.4 kg of plant food
/// (~2,000 kcal at the packs' energy density is not claimed; carbon is
/// what is tracked) and 2.6 L of drinking water a day.
const FOOD_KG_PER_MEAL: f64 = 0.6;
const WATER_KG_PER_DRINK: f64 = 0.65;
/// Body weight from which a person eats and drinks the full ration; lighter
/// people (children) take a share that scales with metabolic rate, weight^0.75.
const FULL_RATION_WEIGHT_KG: f64 = 55.0;
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
    /// Children following mothers and newborn adoption (every substep).
    pub households: std::time::Duration,
    /// Weather, ocean, tides, insolation, hydrology step and the estate energy and respiration (hourly).
    pub physics_hourly: std::time::Duration,
    /// Ecology, vegetation, food and water (six-hourly).
    pub ecology_six_hourly: std::time::Duration,
    /// Per-human folder writes (store cadence).
    pub human_store: std::time::Duration,
}

/// The share of a full meal or drink a person of this weight takes. Exactly
/// 1 from `FULL_RATION_WEIGHT_KG` up, so adults are unaffected.
fn ration_factor(weight_kg: f64) -> f64 {
    (weight_kg / FULL_RATION_WEIGHT_KG)
        .powf(0.75)
        .clamp(0.05, 1.0)
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

/// Everything about the island that is not re-derived from the scenario and
/// the terrain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IslandLifeSnapshot {
    pub physical: PhysicalSnapshot,
    pub ecology: RegionalEcologyState,
    pub location: (usize, usize),
    pub patch: LocalPatchSpec,
    pub property: PropertySystem,
    pub layout: EstateLayout,
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
    pub scheduler: IslandScheduler,
    pub commands: CommandState,
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
    pub scheduler: IslandScheduler,
    pub commands: CommandState,
    /// Per-human folders for this run (runtime-only; not hashed or saved).
    pub store: Option<super::store::HumanStoreStatus>,
    /// Where the step time goes (runtime-only; not hashed or saved).
    pub timings: SubsystemTimings,
    labour: LabourTable,
    rng: RngRegistry,
    food_cell: (usize, usize),
    pub water_cell: Option<(usize, usize)>,
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
        Ok(Self {
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
            scheduler: IslandScheduler::new(scenario_cadences),
            commands: CommandState::default(),
            store: None,
            timings: SubsystemTimings::default(),
            labour: LabourTable::load_default().map_err(IslandLifeError::Labour)?,
            food_cell,
            water_cell,
            topology,
        })
    }

    /// The state to save.
    pub fn to_snapshot(&self) -> IslandLifeSnapshot {
        IslandLifeSnapshot {
            physical: self.physical.to_snapshot(),
            ecology: self.ecology.clone(),
            location: self.placed.location,
            patch: self.placed.patch.clone(),
            property: self.placed.property.clone(),
            layout: self.placed.layout.clone(),
            vegetation: self.vegetation.clone(),
            humans: self.humans.clone(),
            positions: self.positions.clone(),
            energy: self.energy.clone(),
            materials: self.materials.clone(),
            economy: self.economy.clone(),
            sim_time_s: self.sim_time_s,
            tick: self.tick,
            audits_closed: self.audits_closed,
            shortfalls: self.shortfalls,
            scheduler: self.scheduler.clone(),
            commands: self.commands.clone(),
            food_cell: self.food_cell,
            water_cell: self.water_cell,
        }
    }

    /// Rebuild the island from a scenario, its canon and a snapshot,
    /// re-deriving the domain, topology, labour table, random registry and
    /// the terrain-derived caches.
    pub fn from_snapshot(
        scenario: IslandScenario,
        canon: Arc<CanonLocked>,
        snap: IslandLifeSnapshot,
    ) -> Result<Self, IslandLifeError> {
        let domain =
            IslandDomain::from_profile(scenario.profile.clone()).expect("a validated scenario");
        let physical = RegionalPhysicalState::from_snapshot(snap.physical, &domain);
        let topology = GridTopology::regional(&domain, DomainLevel::Medium);
        Ok(Self {
            rng: RngRegistry::new(scenario.seed),
            canon,
            domain,
            physical,
            ecology: snap.ecology,
            placed: PlacedEstate {
                location: snap.location,
                patch: snap.patch,
                property: snap.property,
                layout: snap.layout,
            },
            vegetation: snap.vegetation,
            humans: snap.humans,
            positions: snap.positions,
            energy: snap.energy,
            materials: snap.materials,
            economy: snap.economy,
            sim_time_s: snap.sim_time_s,
            tick: snap.tick,
            audits_closed: snap.audits_closed,
            shortfalls: snap.shortfalls,
            scheduler: snap.scheduler,
            commands: snap.commands,
            store: None,
            timings: SubsystemTimings::default(),
            labour: LabourTable::load_default().map_err(IslandLifeError::Labour)?,
            food_cell: snap.food_cell,
            water_cell: snap.water_cell,
            topology,
            scenario,
        })
    }

    /// Run `f` as one audited household step: the ledger's net flows must
    /// equal the change in each material stock.
    fn audited(&mut self, f: impl FnOnce(&mut Self, &mut Ledger)) -> Result<(), IslandLifeError> {
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

    fn founders(&self) -> Vec<(String, LabourBody)> {
        self.humans
            .registry
            .iter()
            .filter(|h| matches!(h.profile.status, mk_core::human::HumanStatus::Alive))
            .map(|h| (h.agent_id().to_string(), LabourBody::from_human(h)))
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
        let labour = self.labour.clone();
        let dt_physical = self.scenario.cadences.weather_ocean_seconds;
        self.audited(|life, ledger| {
            life.energy.step(dt_physical as f64, solar_kw, ledger);
            for (id, body) in &founders {
                let kcal = labour
                    .energy_kcal(body, HOME_MET, dt_physical as f64)
                    .unwrap_or(0.0);
                let _ = life.materials.respire(id, kcal, ledger);
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
            let (rows, cols) = (
                life.domain.rows(DomainLevel::Medium),
                life.domain.cols(DomainLevel::Medium),
            );
            for (id, _) in &founders {
                // Food and water are taken where the person stands.
                let (ration, stands) =
                    life.humans
                        .registry
                        .get_human(id)
                        .map_or((1.0, life.food_cell), |h| {
                            (
                                ration_factor(h.body.weight_kg),
                                (
                                    h.position.row.clamp(0, rows as i32 - 1) as usize,
                                    h.position.col.clamp(0, cols as i32 - 1) as usize,
                                ),
                            )
                        });
                // Food: the cell they stand on, else the nearest one around
                // it that has plants.
                let mut ate = Err(MaterialError::NothingToHarvest);
                for cell in cells_around(stands, FORAGE_RADIUS_CELLS, rows, cols) {
                    ate = life.materials.gather_biotic(
                        Material::PlantFood,
                        FOOD_KG_PER_MEAL * ration,
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
                        let _ = life.materials.eat(id, Material::PlantFood, kg, ledger);
                    }
                    Err(_) => life.shortfalls.food += 1,
                }
                // Water: the nearest wet cell within reach; near home, also
                // the household's stream (or the wet cell beside it).
                let mut candidates = cells_around(stands, WATER_RADIUS_CELLS, rows, cols);
                if let Some(home) = life.water_cell {
                    if stands.0.abs_diff(home.0).max(stands.1.abs_diff(home.1)) <= HOME_RANGE_CELLS
                    {
                        candidates.extend(water_search_cells(home, rows, cols));
                    }
                }
                let mut drawn = Err(MaterialError::DryCell);
                for cell in candidates {
                    drawn = life.materials.gather_water(
                        WATER_KG_PER_DRINK * ration,
                        &mut life.physical.hydrology,
                        cell,
                        area,
                        ledger,
                    );
                    if drawn.is_ok() {
                        break;
                    }
                }
                match drawn {
                    Ok(kg) => {
                        let _ = life.materials.drink(id, kg);
                        let _ =
                            life.materials
                                .lose_water(id, kg, EXCRETION_TO_SOIL_FRACTION, ledger);
                    }
                    Err(_) => life.shortfalls.water += 1,
                }
            }
        })
    }

    /// Queue an external command for the next human substep; it applies at
    /// that substep's start, in `(tick, sequence)` order.
    pub fn queue_command(
        &mut self,
        command: IslandCommand,
    ) -> Result<QueuedCommandId, CommandError> {
        validate(&command)?;
        if let IslandCommand::CreateHuman(r) = &command {
            self.resolve_location(r.location.as_ref())?;
            let queued = self.commands.pending.iter().any(
                |e| matches!(&e.command, IslandCommand::CreateHuman(q) if q.agent_id == r.agent_id),
            );
            if queued || self.humans.registry.get_human(&r.agent_id).is_some() {
                return Err(CommandError::DuplicateHuman(r.agent_id.clone()));
            }
        }
        let sequence = self.commands.next_sequence;
        self.commands.next_sequence += 1;
        self.commands.pending.push(IslandReplayEntry {
            tick: self.tick + 1,
            sequence,
            command,
        });
        Ok(QueuedCommandId(sequence))
    }

    /// Queue a logged command at its recorded tick and sequence (replay).
    pub fn queue_recorded(&mut self, entry: IslandReplayEntry) {
        self.commands.next_sequence = self.commands.next_sequence.max(entry.sequence + 1);
        self.commands.pending.push(entry);
    }

    fn apply_control(&mut self, c: ControlCommand) {
        match c {
            ControlCommand::Pause => self.commands.paused = true,
            ControlCommand::Resume => {
                self.commands.paused = false;
                self.commands.step_credit = 0;
            }
            ControlCommand::Step(n) => self.commands.step_credit += n,
            ControlCommand::Snapshot => self.commands.snapshots_requested += 1,
            ControlCommand::SetSpeed(speed) => self.commands.speed = speed,
        }
    }

    /// Where an arrival appears, in metres from the island's SW corner.
    /// Refused if the place does not exist or is not land.
    fn resolve_location(&self, at: Option<&CreateLocation>) -> Result<(f64, f64), CommandError> {
        let layout = &self.placed.layout;
        let bad = |why: String| Err(CommandError::InvalidRequest(why));
        match at {
            None => {
                let yard = layout.yard;
                Ok((0.5 * (yard.x0 + yard.x1), 0.5 * (yard.y0 + yard.y1)))
            }
            Some(CreateLocation::EstateSpace { label }) => {
                match layout
                    .spaces
                    .iter()
                    .find(|s| s.label.eq_ignore_ascii_case(label.trim()))
                {
                    Some(s) => Ok((
                        0.5 * (s.rect_m.x0 + s.rect_m.x1),
                        0.5 * (s.rect_m.y0 + s.rect_m.y1),
                    )),
                    None => bad(format!("the estate has no space named {label:?}")),
                }
            }
            Some(CreateLocation::EstatePoint { position_m }) => {
                let p = &layout.patch;
                let inside = (p.origin_x_m..p.origin_x_m + p.width_m).contains(&position_m.0)
                    && (p.origin_y_m..p.origin_y_m + p.height_m).contains(&position_m.1);
                if inside {
                    Ok(*position_m)
                } else {
                    bad("that point is outside the estate".into())
                }
            }
            Some(CreateLocation::IslandCell { row, col }) => {
                let level = DomainLevel::Medium;
                if *row >= self.domain.rows(level) || *col >= self.domain.cols(level) {
                    return bad(format!("cell ({row}, {col}) is off the map"));
                }
                if *self.physical.geophysics.elevation_m.get(*row, *col) <= 0.0 {
                    return bad(format!("cell ({row}, {col}) is not land"));
                }
                Ok(self.domain.cell_center_m(level, *row, *col))
            }
        }
    }

    fn create_human(&mut self, r: &IslandCreateHumanRequest) {
        use mk_core::human::astrology::GeoCoordinates;
        /// 2000-01-01T00:00:00Z: the island calendar's epoch.
        const EPOCH_S: i64 = 946_684_800;
        if self.humans.registry.get_human(&r.agent_id).is_some() {
            return;
        }
        let Ok(at) = self.resolve_location(r.location.as_ref()) else {
            return;
        };
        let birth_s = EPOCH_S + self.sim_time_s as i64 - (r.age_years * 365.25 * 86_400.0) as i64;
        let Some(birth) = chrono::DateTime::from_timestamp(birth_s, 0) else {
            return;
        };
        let (lat, lon) = self.domain.lat_lon_at_m(at.0, at.1);
        let here = (lat.to_degrees(), lon.to_degrees());
        let profile = r.spawn_profile(&birth.to_rfc3339(), here);
        // An authored birth instant and birthplace win over the defaults.
        let (birth, birthplace) = match &profile {
            Some(p) => (
                chrono::DateTime::parse_from_rfc3339(&p.birth_timestamp)
                    .map_or(birth, |t| t.with_timezone(&chrono::Utc)),
                (p.birth_latitude, p.birth_longitude),
            ),
            None => (birth, here),
        };
        let built = crate::humans::spawn::build_spawned_human(
            &self.rng,
            self.tick,
            r.agent_id.clone(),
            r.sex,
            birth,
            GeoCoordinates {
                latitude: birthplace.0,
                longitude: birthplace.1,
            },
            r.age_years,
            profile.as_ref(),
        );
        if let Ok(mut human) = built {
            let size = self.domain.cell_size_m(DomainLevel::Medium);
            let cell = ((at.1 / size).floor() as i32, (at.0 / size).floor() as i32);
            human.set_runtime_position(crate::humans::GridPosition::new(cell.0, cell.1));
            self.materials
                .register_human(&r.agent_id, human.body.weight_kg);
            // Added first, folder second: a failed write is counted and the
            // human still exists.
            if let Err(e) = self.humans.registry.add_human(human) {
                if let Some(store) = self.store.as_mut() {
                    store.record(vec![e]);
                }
            }
            let event = serde_json::json!({
                "kind": "created",
                "tick": self.tick,
                "authored": r.appearance.is_some(),
                "position_m": [at.0, at.1],
            });
            if let Err(e) = self.humans.registry.record_event(&r.agent_id, &event) {
                if let Some(store) = self.store.as_mut() {
                    store.record(vec![e]);
                }
            }
            // Inside the estate patch the person has a metric position.
            let p = &self.placed.layout.patch;
            let in_patch = (p.origin_x_m..p.origin_x_m + p.width_m).contains(&at.0)
                && (p.origin_y_m..p.origin_y_m + p.height_m).contains(&at.1);
            if in_patch {
                self.positions.0.insert(
                    r.agent_id.clone(),
                    super::humans::EstatePosition {
                        space: self.placed.layout.space_at(at),
                        position_m: at,
                    },
                );
            }
        }
    }

    /// Apply every queued command due by `tick`, in `(tick, sequence)`
    /// order, logging each.
    fn apply_commands(&mut self, tick: u64) {
        let mut due: Vec<IslandReplayEntry> = Vec::new();
        self.commands.pending.retain(|e| {
            if e.tick <= tick {
                due.push(e.clone());
                false
            } else {
                true
            }
        });
        due.sort_by_key(|e| (e.tick, e.sequence));
        for entry in due {
            match &entry.command {
                IslandCommand::Control(c) => self.apply_control(*c),
                IslandCommand::Intervention(a) => {
                    if let Some(c) = as_control(a) {
                        self.apply_control(c);
                    }
                }
                IslandCommand::CreateHuman(r) => self.create_human(r),
            }
            self.commands.log.push(entry);
        }
    }

    /// Advance by `seconds` of simulated time in human substeps; a
    /// remainder shorter than a substep is carried to the next call. While
    /// paused no time passes (only `Step` credits run); queued commands
    /// apply at the start of the substep they are due for.
    pub fn advance(&mut self, seconds: u64) -> Result<(), IslandLifeError> {
        let mut remaining = if self.commands.paused {
            0
        } else {
            self.scheduler.take_substeps(seconds)
        };
        if remaining == 0 && !self.commands.paused {
            // No substep runs now, but a command (a pause, a speed change)
            // must not wait for one: nothing happens between substeps, so
            // applying it here is the same as at the start of the next.
            self.apply_commands(self.tick + 1);
        }
        loop {
            if !self.commands.paused && remaining == 0 {
                break;
            }
            self.apply_commands(self.tick + 1);
            if self.commands.paused {
                if self.commands.step_credit == 0 {
                    break;
                }
                self.commands.step_credit -= 1;
            } else if remaining > 0 {
                remaining -= 1;
            } else {
                break;
            }
            self.substep()?;
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
            .filter(|h| !self.materials.is_registered(h.agent_id()))
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

    /// One human substep and whatever fires at its end.
    fn substep(&mut self) -> Result<(), IslandLifeError> {
        let human_s = self.scenario.cadences.human_seconds;
        self.sim_time_s += human_s;
        self.tick += 1;
        let started = std::time::Instant::now();
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
        let humans_done = std::time::Instant::now();
        self.timings.humans += humans_done - started;
        self.keep_children_with_mothers();
        self.adopt_newborns();
        self.timings.households += humans_done.elapsed();
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
            self.sync_store_shard();
            self.timings.human_store += t.elapsed();
        }
        self.timings.substeps += 1;
        self.scheduler.record(due);
        Ok(())
    }

    /// The cadence write: one rotating shard of at most `max_per_sync`
    /// humans, so a large population never costs one huge pause.
    fn sync_store_shard(&mut self) {
        let Some(store) = self.store.as_mut() else {
            return;
        };
        let n = self.humans.registry.iter().count();
        let shards = n.div_ceil(store.max_per_sync.max(1)).max(1);
        let shard = (self.scheduler.counters.human_store as usize) % shards;
        store.record(self.humans.sync_store_shard(shard, shards));
    }

    /// Write every human's folder; failures are counted in the store status.
    pub fn sync_store(&mut self) {
        if let Some(store) = self.store.as_mut() {
            store.record(self.humans.sync_store());
        }
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
        self.humans.canonical_chunks(|bytes| {
            h.update(bytes);
        });
        h.update(canonical(&self.positions).as_bytes());
        h.update(canonical(&self.energy).as_bytes());
        h.update(canonical(&self.economy).as_bytes());
        h.update(format!("{:?}", self.materials).as_bytes());
        h.update(canonical(&self.scheduler).as_bytes());
        h.update(canonical(&self.commands).as_bytes());
        h.update(&self.sim_time_s.to_le_bytes());
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
mod ration_tests {
    use super::ration_factor;

    #[test]
    fn adults_take_the_full_ration_and_children_a_metabolic_share() {
        for adult_kg in [55.0, 58.0, 72.0, 110.0] {
            assert_eq!(ration_factor(adult_kg), 1.0, "{adult_kg} kg");
        }
        let infant = ration_factor(5.1);
        assert!(
            (0.1..0.2).contains(&infant),
            "a 5 kg infant takes {infant:.3} of a ration"
        );
        let child = ration_factor(20.0);
        assert!(
            (0.4..0.6).contains(&child),
            "a 20 kg child takes {child:.3}"
        );
        assert!(
            ration_factor(5.0) < ration_factor(20.0) && ration_factor(20.0) < ration_factor(40.0)
        );
        assert!(ration_factor(0.0) >= 0.05, "never zero");
    }
}
