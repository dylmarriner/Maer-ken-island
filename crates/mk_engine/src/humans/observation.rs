//! What a human perceives of a cell (Phase 3 Task 8, extracted from
//! `WorldState::step_humans_period`).
//!
//! The planetary world and the island read different data (a planetary grid
//! and biosphere; coarse climate and a medium hydrology), but turn a cell's
//! facts into the same [`AgentWorldObservation`] with the same formulas.
//! Those formulas live here, unchanged, so both worlds share them:
//!
//! - the biome's shelter and hazard from its terrain properties;
//! - the computer gate: a property with a `ComputerRoom` building and a
//!   `NetworkAccount` for the agent;
//! - [`build_observation`], which assembles the observation.

use mk_core::biomes::{biome_properties, BiomeType};
use mk_core::time::Tick;

use crate::agents::AgentWorldObservation;
use crate::organisms::property::{PropertyBuildingKind, StarterProperty};

/// Shelter a biome's canopy gives, or an unremarkable `0.5` where the
/// biome is unknown.
pub fn biome_shelter_quality(biome: Option<BiomeType>) -> f64 {
    biome
        .map(|b| {
            let props = biome_properties(b);
            props.canopy_cover * 0.6 + 0.2
        })
        .unwrap_or(0.5)
}

/// Hazard of a biome from its roughness and how hard it is to cross, or
/// `0.1` where the biome is unknown.
pub fn biome_hazard_index(biome: Option<BiomeType>) -> f64 {
    biome
        .map(|b| {
            let props = biome_properties(b);
            props.roughness * 0.5 + (1.0 - props.traversability) * 0.5
        })
        .unwrap_or(0.1)
}

/// Whether `property` has a `ComputerRoom` building and a `NetworkAccount`
/// for `agent_id`: upstream's computer gate, before any question of where
/// the agent stands.
pub fn property_grants_computer(property: &StarterProperty, agent_id: &str) -> bool {
    property
        .buildings
        .iter()
        .any(|building| matches!(building.kind, PropertyBuildingKind::ComputerRoom))
        && property
            .network_accounts
            .iter()
            .any(|account| account.agent_id == agent_id)
}

/// Everything an observation reads, already resolved for one cell.
#[derive(Debug, Clone, Copy)]
pub struct ObservationFacts {
    pub tick: Tick,
    pub ambient_temperature_c: f64,
    pub hydration_access: f64,
    /// Forage abundance before clamping.
    pub resource_abundance: f64,
    /// Shelter the cell gives (the biome's, raised by any building).
    pub shelter_quality: f64,
    pub social_density: f64,
    pub daylight_fraction: f64,
    pub biome: Option<BiomeType>,
    pub computer_access: bool,
    pub computer_bridge_available: f64,
    /// Electric light at the eye (lux), zero where there is none.
    pub lamp_lux: f64,
    /// Fraction of outdoor daylight reaching the eye (1 outdoors).
    pub daylight_factor: f64,
    /// Inside a room that can be darkened.
    pub indoors: bool,
}

/// Assemble an observation from a cell's facts.
pub fn build_observation(f: &ObservationFacts) -> AgentWorldObservation {
    AgentWorldObservation {
        tick: f.tick,
        ambient_temperature_c: f.ambient_temperature_c,
        hydration_access: f.hydration_access,
        caloric_access: f.resource_abundance.clamp(0.0, 1.0),
        shelter_quality: f.shelter_quality,
        social_density: f.social_density,
        hazard_index: biome_hazard_index(f.biome),
        daylight_fraction: f.daylight_fraction,
        biome_type: f.biome,
        resource_abundance: f.resource_abundance,
        computer_access: if f.computer_access { 1.0 } else { 0.0 },
        computer_bridge_available: f.computer_bridge_available,
        lamp_lux: f.lamp_lux,
        daylight_factor: f.daylight_factor,
        indoors: f.indoors,
    }
}
