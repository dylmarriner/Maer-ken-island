//! Canonical humans on the island (Phase 3 Task 8).
//!
//! The human cognition, biology and behaviour code is not forked: this is
//! an adapter. It builds Gem-D and Gem-K through upstream's constructors,
//! starts each in their own bedroom of the laid-out estate, and runs
//! upstream `HumanSystem::step_on` over the island's medium
//! [`GridTopology`] with an observation built from the island's own data
//! (coarse climate and diurnal cycle, medium hydrology, ecology NPP, the
//! estate's buildings) through the shared `humans::observation` helpers.
//!
//! Humans inside the estate patch have a metric position and a space in a
//! side-table; walking out of the patch removes the entry and they move on
//! medium cells (no east/west wrap). Inside the patch, movement between
//! spaces follows [`EstateLayout::route`].
//!
//! **Computer access** is upstream's gate (a ComputerRoom building and a
//! matching `NetworkAccount`) *and* the human standing in the House's
//! computer room *and* the estate able to supply power (Task 4b).

use std::collections::BTreeMap;

use mk_core::grid::Grid2;
use mk_core::rng::RngRegistry;
use serde::{Deserialize, Serialize};

use super::ecology::RegionalEcologyState;
use super::energy::EstateEnergy;
use super::estate_layout::{EstateLayout, Space, SpaceKind};
use super::levels::sample_coarse_at_medium;
use super::physical::RegionalPhysicalState;
use crate::agents::{AgentWorldObservation, GridPosition};
use crate::humans::observation::{
    biome_shelter_quality, build_observation, property_grants_computer, ObservationFacts,
};
use crate::humans::HumanSystem;
use crate::organisms::property::{PropertySystem, StarterProperty};
use crate::perception::{
    daylight_fraction, Occupancy, FORAGE_HALF_SATURATION_NPP_KGC_M2_YR, KELVIN_TO_CELSIUS_OFFSET,
    PRECIPITATION_HALF_SATURATION_MM_DAY, RUNOFF_HALF_SATURATION_MM_DAY,
    SURFACE_WATER_HALF_SATURATION_MM,
};
use crate::resource_economy::ResourceEconomyState;
use crate::topology::GridTopology;
use mk_island::{DomainLevel, IslandDomain};

#[derive(Debug, Clone, PartialEq)]
pub enum RegionalHumanError {
    /// A founder has no bedroom in the layout.
    NoBedroom(String),
    /// The property has no founders' estate.
    NoFoundersEstate,
    NoSuchHuman(String),
    /// The human is not inside the estate patch.
    NotInEstate(String),
    /// No door path to the space.
    Unreachable(String),
}

impl std::fmt::Display for RegionalHumanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoBedroom(who) => write!(f, "{who} has no bedroom in the estate layout"),
            Self::NoFoundersEstate => write!(f, "the property has no founders' estate"),
            Self::NoSuchHuman(who) => write!(f, "no human {who}"),
            Self::NotInEstate(who) => write!(f, "{who} is not in the estate"),
            Self::Unreachable(who) => write!(f, "no route for {who}"),
        }
    }
}

impl std::error::Error for RegionalHumanError {}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EstatePosition {
    pub space: Space,
    pub position_m: (f64, f64),
}

/// Where each human inside the estate patch stands, by agent id. Humans
/// outside the patch have no entry.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct HumanEstatePositions(pub BTreeMap<String, EstatePosition>);

/// What the observation reads about the estate and the island.
pub struct RegionalHumanContext<'a> {
    pub property: &'a PropertySystem,
    pub layout: &'a EstateLayout,
    pub physical: &'a RegionalPhysicalState,
    pub ecology: &'a RegionalEcologyState,
    pub energy: &'a EstateEnergy,
    pub domain: &'a IslandDomain,
    /// Solar output available now (kW), for the power gate.
    pub solar_kw: f64,
    /// A real bridge to the outside, when one is attached.
    ///
    /// `None` is the ordinary case and the only one a replay ever sees.
    /// With one attached, a human at their own machine in a powered
    /// computer room may search the web or send an email for real --
    /// which means the island stops being reproducible, deliberately, and
    /// is why this is opt-in and why `replay`/`inspect` never attach it.
    pub computer_bridge: Option<&'a dyn crate::humans::computer_bridge::ComputerBridge>,
}

fn founders_estate(property: &PropertySystem) -> Option<&StarterProperty> {
    property
        .properties
        .iter()
        .find(|p| p.owner_agent_ids.iter().any(|id| id == "Gem-D"))
}

pub(crate) fn medium_cell_of(domain: &IslandDomain, position_m: (f64, f64)) -> GridPosition {
    let size = domain.cell_size_m(DomainLevel::Medium);
    GridPosition::new(
        (position_m.1 / size).floor() as i32,
        (position_m.0 / size).floor() as i32,
    )
}

fn centre(rect: &super::estate_layout::Rect) -> (f64, f64) {
    (0.5 * (rect.x0 + rect.x1), 0.5 * (rect.y0 + rect.y1))
}

/// Gem-D and Gem-K from upstream's constructors, each in their own bedroom.
pub fn bootstrap_regional_humans(
    property: &PropertySystem,
    layout: &EstateLayout,
    domain: &IslandDomain,
) -> Result<(HumanSystem, HumanEstatePositions), RegionalHumanError> {
    let estate = founders_estate(property).ok_or(RegionalHumanError::NoFoundersEstate)?;
    let mut humans = HumanSystem::new();
    humans
        .registry
        .seed_founders()
        .map_err(|_| RegionalHumanError::NoFoundersEstate)?;
    let mut positions = HumanEstatePositions::default();
    for agent in &estate.owner_agent_ids {
        let label = format!("{agent}'s Bedroom");
        let room = layout
            .spaces
            .iter()
            .find(|s| s.label == label)
            .ok_or_else(|| RegionalHumanError::NoBedroom(agent.clone()))?;
        let at = centre(&room.rect_m);
        if let Some(h) = humans.registry.get_human_mut(agent) {
            h.set_runtime_position(medium_cell_of(domain, at));
        }
        positions.0.insert(
            agent.clone(),
            EstatePosition {
                space: Space::Inside(room.id),
                position_m: at,
            },
        );
    }
    Ok((humans, positions))
}

/// Move a human to a space in the estate, along the door route: returns the
/// route walked. The walk is instantaneous here; Task 3b gives it a time.
pub fn walk_to_space(
    positions: &mut HumanEstatePositions,
    layout: &EstateLayout,
    agent_id: &str,
    target: Space,
) -> Result<Vec<Space>, RegionalHumanError> {
    let from = positions
        .0
        .get(agent_id)
        .ok_or_else(|| RegionalHumanError::NotInEstate(agent_id.into()))?
        .space;
    let route = layout
        .route(from, target)
        .ok_or_else(|| RegionalHumanError::Unreachable(agent_id.into()))?;
    let at = match target {
        Space::Inside(id) => layout
            .spaces
            .iter()
            .find(|s| s.id == id)
            .map(|s| centre(&s.rect_m))
            .ok_or_else(|| RegionalHumanError::Unreachable(agent_id.into()))?,
        Space::Outdoors => {
            let door = layout
                .doors
                .iter()
                .find(|d| d.a == Space::Outdoors || d.b == Space::Outdoors)
                .map_or_else(|| centre(&layout.yard), |d| d.position_m);
            (door.0, door.1 - 1.0)
        }
    };
    positions.0.insert(
        agent_id.into(),
        EstatePosition {
            space: target,
            position_m: at,
        },
    );
    Ok(route)
}

fn saturating(x: f64, half: f64) -> f64 {
    let x = x.max(0.0);
    x / (x + half)
}

/// The island observation of `agent_id` at the medium cell `position`.
pub fn observe(
    ctx: &RegionalHumanContext<'_>,
    positions: &HumanEstatePositions,
    occupancy: &Occupancy,
    tick: u64,
    position: &GridPosition,
    agent_id: &str,
) -> AgentWorldObservation {
    let domain = ctx.domain;
    let medium = DomainLevel::Medium;
    let (rows, cols) = (domain.rows(medium), domain.cols(medium));
    let r = position.row.clamp(0, rows as i32 - 1) as usize;
    let c = position.col.clamp(0, cols as i32 - 1) as usize;
    let physical = ctx.physical;
    let coarse = |grid: &Grid2<f64>| sample_coarse_at_medium(|a, b| *grid.get(a, b), domain, r, c);

    let temperature_k =
        coarse(&physical.climate.surface_temperature) + coarse(&physical.diurnal_offset_k);
    let biome = Some(*ctx.ecology.biome_grid.get(r, c));
    let aquatic = biome.is_some_and(|b| b.is_aquatic());
    let hydration = if aquatic {
        0.0
    } else {
        let hydro = &physical.hydrology;
        let rain = coarse(&physical.weather.precipitation);
        let chances = [
            saturating(
                *hydro.surface_water.get(r, c),
                SURFACE_WATER_HALF_SATURATION_MM,
            ),
            saturating(*hydro.runoff.get(r, c), RUNOFF_HALF_SATURATION_MM_DAY),
            saturating(rain, PRECIPITATION_HALF_SATURATION_MM_DAY),
            hydro.soil_water.get(r, c).moisture_fraction.clamp(0.0, 1.0),
        ];
        (1.0 - chances.iter().map(|p| 1.0 - p).product::<f64>()).clamp(0.0, 1.0)
    };
    let npp = ctx.ecology.npp_kgc_m2_yr.get(r, c).max(0.0);
    let abundance = if npp > 0.0 {
        npp / (npp + FORAGE_HALF_SATURATION_NPP_KGC_M2_YR)
    } else {
        0.0
    };

    // Inside the estate, the building around the human shelters them and
    // the computer room is where the computer can be used.
    let here = positions.0.get(agent_id);
    let building_shelter = here.and_then(|p| match p.space {
        Space::Inside(id) => {
            let space = ctx.layout.spaces.iter().find(|s| s.id == id)?;
            let estate = founders_estate(ctx.property)?;
            estate
                .buildings
                .iter()
                .find(|b| b.id == space.building_id)
                .map(|b| b.kind.shelter_quality())
        }
        Space::Outdoors => None,
    });
    let in_computer_room = here.is_some_and(|p| match p.space {
        Space::Inside(id) => ctx
            .layout
            .spaces
            .iter()
            .any(|s| s.id == id && s.kind == SpaceKind::Room && s.label == "Computer Room"),
        Space::Outdoors => false,
    });
    let gate = founders_estate(ctx.property).is_some_and(|p| property_grants_computer(p, agent_id));
    let powered = ctx.energy.has_stored_supply(ctx.solar_kw);
    let coarse_r =
        ((domain.cell_center_m(medium, r, c).1) / domain.cell_size_m(DomainLevel::Coarse)) as usize;
    let coarse_c =
        ((domain.cell_center_m(medium, r, c).0) / domain.cell_size_m(DomainLevel::Coarse)) as usize;
    build_observation(&ObservationFacts {
        tick,
        ambient_temperature_c: temperature_k - KELVIN_TO_CELSIUS_OFFSET,
        hydration_access: hydration,
        resource_abundance: abundance,
        shelter_quality: biome_shelter_quality(biome).max(building_shelter.unwrap_or(0.0)),
        social_density: occupancy.social_density(*position, true),
        daylight_fraction: daylight_fraction(&physical.insolation, coarse_r, coarse_c),
        biome,
        computer_access: gate && in_computer_room && powered,
        // Two separate affordances, and upstream is explicit about why:
        // `computer_access` is this human being at a machine of their own
        // in a powered room, and this is the world having a way out to
        // the internet at all. `Code` needs only the first; a web search
        // or an email needs both.
        computer_bridge_available: if ctx.computer_bridge.is_some() {
            1.0
        } else {
            0.0
        },
    })
}

/// Step the humans by `dt_seconds` through upstream's `HumanSystem::step_on`
/// on the island's medium topology, then reconcile the estate side-table:
/// a human whose cell is outside the estate block loses their entry; one
/// who moved to another block cell stands outdoors at its centre.
#[allow(clippy::too_many_arguments)]
pub fn step_regional_humans(
    humans: &mut HumanSystem,
    positions: &mut HumanEstatePositions,
    economy: &mut ResourceEconomyState,
    ctx: &RegionalHumanContext<'_>,
    topology: &GridTopology,
    tick: u64,
    dt_seconds: u64,
    rng: &RngRegistry,
) {
    if dt_seconds == 0 {
        return;
    }
    let before: BTreeMap<String, GridPosition> = humans
        .registry
        .iter()
        .map(|h| (h.agent_id().to_string(), h.position))
        .collect();
    let occupancy = Occupancy::new_on(
        topology,
        humans
            .registry
            .iter()
            .filter(|h| matches!(h.profile.status, mk_core::human::HumanStatus::Alive))
            .map(|h| h.position),
    );
    let positions_ro = positions.clone();
    let dt_years = dt_seconds as f64 / (365.25 * 86_400.0);
    humans.step_on(
        dt_years,
        tick,
        rng,
        topology,
        economy,
        &ctx.physical.geophysics.elevation_m,
        |position, agent_id| observe(ctx, &positions_ro, &occupancy, tick, position, agent_id),
        ctx.computer_bridge,
    );

    let patch = &ctx.layout.patch;
    let size = ctx.domain.cell_size_m(DomainLevel::Medium);
    let (row0, col0) = (
        (patch.origin_y_m / size).round() as i32,
        (patch.origin_x_m / size).round() as i32,
    );
    for human in humans.registry.iter() {
        let id = human.agent_id();
        let now = human.position;
        let in_block = (row0..row0 + 2).contains(&now.row) && (col0..col0 + 2).contains(&now.col);
        if !in_block {
            positions.0.remove(id);
        } else if before.get(id) != Some(&now) && positions.0.contains_key(id) {
            let (x, y) =
                ctx.domain
                    .cell_center_m(DomainLevel::Medium, now.row as usize, now.col as usize);
            positions.0.insert(
                id.to_string(),
                EstatePosition {
                    space: Space::Outdoors,
                    position_m: (x, y),
                },
            );
        }
    }
}
