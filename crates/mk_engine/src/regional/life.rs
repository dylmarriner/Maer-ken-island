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
use mk_island::{DomainLevel, IslandDomain, IslandScenario};

use super::ecology::{RegionalEcologyError, RegionalEcologyState};
use super::energy::{solar_output_kw, EstateEnergy};
use super::estate_layout::Space;
use super::humans::{
    bootstrap_regional_humans, step_regional_humans, HumanEstatePositions, RegionalHumanContext,
    RegionalHumanError,
};
use super::hydrology::discharge_m3_s;
use super::labour::{LabourBody, LabourError, LabourTable};
use super::local_vegetation::{seed_local_vegetation, LocalVegetationError, LocalVegetationPatch};
use super::materials::{Material, MaterialError, MaterialLedger};
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

/// Which of the two a human at home is doing, from the circadian clock's
/// own sleep state.
fn home_activity(asleep: bool) -> &'static str {
    if asleep {
        ASLEEP_ACTIVITY
    } else {
        AWAKE_AT_HOME_ACTIVITY
    }
}
/// Food and water each founder takes per 6 hours: ~2.4 kg of plant food
/// (~2,000 kcal at the packs' energy density is not claimed; carbon is
/// what is tracked) and 2.6 L of drinking water a day.
const FOOD_KG_PER_MEAL: f64 = 0.6;
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
}

impl std::fmt::Display for IslandLifeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for IslandLifeError {}

/// Shortfalls the households met: nothing to harvest, a dry cell.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Shortfalls {
    pub food: u64,
    pub water: u64,
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
    labour: LabourTable,
    rng: RngRegistry,
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
            labour: LabourTable::load_default().map_err(IslandLifeError::Labour)?,
            food_cell,
            water_cell,
            topology,
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
        let labour = self.labour.clone();
        let dt_physical = self.scenario.cadences.weather_ocean_seconds;
        let asleep_met = labour
            .met(ASLEEP_ACTIVITY)
            .map_err(IslandLifeError::Labour)?;
        let awake_met = labour
            .met(AWAKE_AT_HOME_ACTIVITY)
            .map_err(IslandLifeError::Labour)?;
        self.audited(|life, ledger| {
            life.energy.step(dt_physical as f64, solar_kw, ledger);
            for (id, body, asleep) in &founders {
                let met = if home_activity(*asleep) == ASLEEP_ACTIVITY {
                    asleep_met
                } else {
                    awake_met
                };
                let kcal = labour
                    .energy_kcal(body, met, dt_physical as f64)
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
            for (id, _, _) in &founders {
                // A fixed home routine, deliberately, not each human's own
                // chosen action — see D10, and the measurement in it. Gating
                // the harvest on `economy_action` was tried and reverted: the
                // founders then chose to look for food too rarely to cover
                // what they burn, and lost a tenth of their body carbon in a
                // week. Matching harvest to expenditure is the open economy
                // work, not a condition on this loop.
                {
                    match life.materials.gather_biotic(
                        Material::PlantFood,
                        FOOD_KG_PER_MEAL,
                        &mut life.ecology,
                        life.food_cell,
                        area,
                        ledger,
                    ) {
                        Ok(kg) => {
                            let _ = life.materials.eat(id, Material::PlantFood, kg, ledger);
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
            let due = self.scheduler.due(self.sim_time_s);
            if due.weather_ocean {
                self.hourly()?;
            }
            if due.hydrology_ecology_resource {
                self.six_hourly()?;
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
mod tests {
    use super::*;

    #[test]
    fn a_sleeping_human_at_home_is_costed_as_sleeping() {
        assert_eq!(home_activity(true), ASLEEP_ACTIVITY);
        assert_eq!(home_activity(false), AWAKE_AT_HOME_ACTIVITY);
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
