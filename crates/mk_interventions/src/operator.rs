//! Operator-console intervention builders shared by `mk_observatory` and
//! `mk_ui`.
//!
//! The consoles edit an [`OperatorControls`] with sliders and pickers and
//! turn it into fully specified [`InterventionAction`]s at the cell the
//! operator picked, so every button reaches the engine through
//! `InterventionExecutor` (validation, permissions, flux ledger) with a real
//! location and magnitude.

use crate::{
    BiomassType, DisturbanceType, EnergyType, InterventionAction, Location, Region, ScenarioValue,
};

/// `ModifyScenario` parameter for atmospheric CO₂, ppm.
pub const CO2_PARAMETER: &str = "climate.co2_concentration_ppm";
/// `ModifyScenario` parameter that scales every resource node's current
/// regeneration rate.
pub const REGENERATION_PARAMETER: &str = "resource_economy.regeneration_multiplier";

/// Range the consoles offer for injected energy, joules. At the default
/// 32×64 grid, 10¹⁸ J warms one cell by about 0.1 K.
pub const ENERGY_RANGE_J: std::ops::RangeInclusive<f32> = 1.0e15..=1.0e21;

impl DisturbanceType {
    pub const ALL: [DisturbanceType; 6] = [
        DisturbanceType::VolcanicEruption,
        DisturbanceType::Fire,
        DisturbanceType::Flood,
        DisturbanceType::Drought,
        DisturbanceType::Disease,
        DisturbanceType::MeteorImpact,
    ];

    pub fn label(self) -> &'static str {
        match self {
            DisturbanceType::Fire => "Fire",
            DisturbanceType::Flood => "Flood",
            DisturbanceType::VolcanicEruption => "Volcanic eruption",
            DisturbanceType::MeteorImpact => "Meteor impact",
            DisturbanceType::Disease => "Disease",
            DisturbanceType::Drought => "Drought",
        }
    }
}

impl EnergyType {
    pub const ALL: [EnergyType; 3] = [EnergyType::Thermal, EnergyType::Solar, EnergyType::Chemical];

    pub fn label(self) -> &'static str {
        match self {
            EnergyType::Solar => "Solar",
            EnergyType::Thermal => "Thermal",
            EnergyType::Chemical => "Chemical",
        }
    }
}

impl BiomassType {
    pub const ALL: [BiomassType; 4] = [
        BiomassType::Producers,
        BiomassType::Consumers,
        BiomassType::Apex,
        BiomassType::Decomposers,
    ];

    pub fn label(self) -> &'static str {
        match self {
            BiomassType::Producers => "Producers",
            BiomassType::Consumers => "Consumers",
            BiomassType::Apex => "Apex predators",
            BiomassType::Decomposers => "Decomposers",
        }
    }
}

/// The parameters an operator console edits before applying an
/// intervention.
#[derive(Debug, Clone, PartialEq)]
pub struct OperatorControls {
    pub disturbance: DisturbanceType,
    /// Disturbance intensity, `0..=1`.
    pub intensity: f32,
    pub energy_type: EnergyType,
    /// Energy to inject at the picked cell, joules.
    pub energy_j: f32,
    pub biomass: BiomassType,
    /// Individuals to introduce, split across eligible species.
    pub individuals: u32,
    /// Radius of the seeded region, km.
    pub radius_km: f32,
    /// Target atmospheric CO₂, ppm.
    pub co2_ppm: f32,
    /// Factor applied to every resource node's current regeneration rate.
    pub regeneration_multiplier: f32,
}

impl Default for OperatorControls {
    fn default() -> Self {
        Self {
            disturbance: DisturbanceType::VolcanicEruption,
            intensity: 0.5,
            energy_type: EnergyType::Thermal,
            energy_j: 1.0e18,
            biomass: BiomassType::Producers,
            individuals: 1_000,
            radius_km: 250.0,
            co2_ppm: 280.0,
            regeneration_multiplier: 1.0,
        }
    }
}

impl OperatorControls {
    /// Register the selected disturbance at a surface point (degrees).
    pub fn disturbance_at(&self, latitude_deg: f32, longitude_deg: f32) -> InterventionAction {
        InterventionAction::TriggerDisturbance {
            disturbance_type: self.disturbance,
            intensity: self.intensity,
            location: Location::new(latitude_deg, longitude_deg),
        }
    }

    /// Inject the selected energy at a surface point (degrees).
    pub fn energy_at(&self, latitude_deg: f32, longitude_deg: f32) -> InterventionAction {
        InterventionAction::InjectEnergy {
            energy_type: self.energy_type,
            amount: self.energy_j,
            location: Location::new(latitude_deg, longitude_deg),
        }
    }

    /// Introduce individuals of the selected trophic group around a surface
    /// point (degrees).
    pub fn biomass_at(&self, latitude_deg: f32, longitude_deg: f32) -> InterventionAction {
        InterventionAction::InjectBiomass {
            biomass_type: self.biomass,
            amount: self.individuals as f32,
            region: Region::new(Location::new(latitude_deg, longitude_deg), self.radius_km),
        }
    }

    /// Set atmospheric CO₂ to [`Self::co2_ppm`].
    pub fn set_co2(&self) -> InterventionAction {
        InterventionAction::ModifyScenario {
            parameter: CO2_PARAMETER.to_string(),
            value: ScenarioValue::Float(self.co2_ppm),
        }
    }

    /// Scale every resource node's regeneration by
    /// [`Self::regeneration_multiplier`].
    pub fn scale_regeneration(&self) -> InterventionAction {
        InterventionAction::ModifyScenario {
            parameter: REGENERATION_PARAMETER.to_string(),
            value: ScenarioValue::Float(self.regeneration_multiplier),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{validate_intervention, InterventionPermissions};

    #[test]
    fn every_default_action_validates_and_is_permitted_by_default() {
        let controls = OperatorControls::default();
        let permissions = InterventionPermissions::default();
        for action in [
            controls.disturbance_at(-12.5, 140.0),
            controls.energy_at(-12.5, 140.0),
            controls.biomass_at(-12.5, 140.0),
            controls.set_co2(),
            controls.scale_regeneration(),
        ] {
            assert!(validate_intervention(&action).valid, "{action:?}");
            assert!(permissions.can_execute(&action), "{action:?}");
        }
    }

    #[test]
    fn located_actions_carry_the_picked_point() {
        let controls = OperatorControls::default();
        match controls.disturbance_at(33.0, -71.0) {
            InterventionAction::TriggerDisturbance { location, .. } => {
                assert_eq!((location.latitude, location.longitude), (33.0, -71.0));
            }
            other => panic!("unexpected {other:?}"),
        }
        match controls.biomass_at(33.0, -71.0) {
            InterventionAction::InjectBiomass { region, .. } => {
                assert_eq!(region.center.latitude, 33.0);
                assert_eq!(region.radius_km, controls.radius_km);
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn default_energy_is_inside_the_offered_range() {
        assert!(ENERGY_RANGE_J.contains(&OperatorControls::default().energy_j));
    }
}
