/// Metabolism Schema
///
/// Pure schema definitions for future organism metabolism.
/// No executable logic, no state, no agency.
use serde::{Deserialize, Serialize};

/// Metabolism schema for organism energy processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetabolismSchema {
    pub metabolic_pathways: Vec<MetabolicPathway>,
    pub energy_budget: EnergyBudget,
    pub thermoregulation: Thermoregulation,
    pub respiratory_system: RespiratorySystem,
    pub waste_management: WasteManagement,
}

/// Metabolic pathway definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetabolicPathway {
    pub pathway_name: String,
    pub pathway_type: PathwayType,
    pub efficiency: f64,   // 0.0 to 1.0
    pub energy_yield: f64, // ATP per substrate
    pub substrate_requirements: Vec<SubstrateRequirement>,
    pub environmental_conditions: EnvironmentalConditions,
}

/// Types of metabolic pathways
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PathwayType {
    /// Aerobic respiration (with oxygen)
    Aerobic,
    /// Anaerobic respiration (without oxygen)
    Anaerobic,
    /// Fermentation
    Fermentation,
    /// Photosynthesis
    Photosynthesis,
    /// Chemosynthesis
    Chemosynthesis,
    /// Specialized pathway
    Specialized(String),
}

/// Substrate requirement for pathway
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubstrateRequirement {
    pub substrate_name: String,
    pub concentration_required: f64,
    pub availability: Availability,
}

/// Availability of substrate
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Availability {
    /// Always available
    Abundant,
    /// Seasonally available
    Seasonal,
    /// Rarely available
    Rare,
    /// Environmentally dependent
    Environmental,
}

/// Environmental conditions for pathway
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentalConditions {
    pub temperature_range: (f64, f64), // Celsius
    pub pressure_range: (f64, f64),    // Atmospheres
    pub ph_range: (f64, f64),          // pH units
    pub oxygen_requirement: OxygenRequirement,
}

/// Oxygen requirement
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OxygenRequirement {
    /// Requires oxygen
    ObligateAerobe,
    /// Requires no oxygen
    ObligateAnaerobe,
    /// Tolerates oxygen but doesn't require it
    FacultativeAnaerobe,
    /// Oxygen is toxic
    AerotolerantAnaerobe,
    /// Requires micro-oxygen levels
    Microaerophile,
}

/// Energy budget for organism
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnergyBudget {
    pub basal_metabolic_rate: f64,  // Energy at rest
    pub active_metabolic_rate: f64, // Energy during activity
    pub energy_storage: EnergyStorage,
    pub energy_allocation: EnergyAllocation,
}

/// Energy storage mechanisms
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnergyStorage {
    pub storage_type: StorageType,
    pub capacity_relative_to_body_mass: f64,
    pub conversion_efficiency: f64, // Storage to usable energy
    pub depletion_rate: f64,        // Rate of energy use
}

/// Types of energy storage
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StorageType {
    /// Fat storage
    Lipid,
    /// Carbohydrate storage
    Carbohydrate,
    /// Protein storage
    Protein,
    /// Specialized storage compound
    Specialized(String),
    /// No storage
    None,
}

/// Energy allocation priorities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnergyAllocation {
    pub maintenance_percentage: f64,  // Basic functions
    pub growth_percentage: f64,       // Growth and repair
    pub reproduction_percentage: f64, // Reproductive processes
    pub activity_percentage: f64,     // Movement and behavior
    pub storage_percentage: f64,      // Energy storage
}

/// Thermoregulation system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Thermoregulation {
    pub regulation_type: RegulationType,
    pub temperature_tolerance: TemperatureTolerance,
    pub heat_exchange_mechanisms: Vec<HeatExchangeMechanism>,
    pub metabolic_heat_production: MetabolicHeatProduction,
}

/// Types of thermoregulation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RegulationType {
    /// Body temperature varies with environment
    Ectothermic,
    /// Body temperature maintained internally
    Endothermic,
    /// Partially endothermic
    Mesothermic,
    /// Regional endothermy
    RegionalEndothermy { regions: Vec<String> },
}

/// Temperature tolerance range
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemperatureTolerance {
    pub critical_minimum: f64,     // Lethal cold temperature
    pub critical_maximum: f64,     // Lethal hot temperature
    pub optimal_range: (f64, f64), // Preferred temperature range
    pub acclimation_capacity: f64, // Ability to adjust tolerance
}

/// Heat exchange mechanisms
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HeatExchangeMechanism {
    /// Radiation heat exchange
    Radiation,
    /// Convection heat exchange
    Convection,
    /// Conduction heat exchange
    Conduction,
    /// Evaporative cooling
    Evaporation,
    /// Counter-current heat exchange
    CounterCurrent,
    /// Behavioral thermoregulation
    Behavioral,
}

/// Metabolic heat production
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetabolicHeatProduction {
    pub basal_heat_production: f64,        // Heat at rest
    pub activity_heat_multiplier: f64,     // Heat increase during activity
    pub non_shivering_thermogenesis: bool, // Specialized heat production
    pub brown_fat_present: bool,           // Specialized heat-producing tissue
}

/// Respiratory system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RespiratorySystem {
    pub respiratory_type: RespiratoryType,
    pub gas_exchange_efficiency: f64,      // 0.0 to 1.0
    pub oxygen_extraction_efficiency: f64, // 0.0 to 1.0
    pub breathing_mechanism: BreathingMechanism,
}

/// Types of respiratory systems
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RespiratoryType {
    /// Diffusion across body surface
    Cutaneous,
    /// Gills for aquatic respiration
    Gill,
    /// Tracheal system
    Tracheal,
    /// Book lungs
    BookLung,
    /// Alveolar lungs
    Alveolar,
    /// Air sac system
    AirSac,
    /// Specialized respiratory system
    Specialized(String),
}

/// Breathing mechanism
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BreathingMechanism {
    /// No active breathing (diffusion only)
    Passive,
    /// Buccal pumping (mouth movements)
    Buccal,
    /// Costal breathing (rib movements)
    Costal,
    /// Diaphragmatic breathing
    Diaphragmatic,
    /// Dual pumping (buccal and opercular)
    Dual,
}

/// Waste management system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasteManagement {
    pub nitrogen_waste_type: NitrogenWasteType,
    pub water_conservation_level: WaterConservationLevel,
    pub excretion_mechanisms: Vec<ExcretionMechanism>,
    pub osmoregulation: Osmoregulation,
}

/// Types of nitrogen waste
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NitrogenWasteType {
    /// Ammonia (toxic, requires water)
    Ammonia,
    /// Urea (less toxic, moderate water)
    Urea,
    /// Uric acid (least toxic, minimal water)
    UricAcid,
    /// Specialized waste product
    Specialized(String),
}

/// Water conservation level
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum WaterConservationLevel {
    /// No water conservation
    None,
    /// Minimal conservation
    Minimal,
    /// Moderate conservation
    Moderate,
    /// High conservation
    High,
    /// Extreme conservation
    Extreme,
}

/// Excretion mechanisms
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExcretionMechanism {
    /// Diffusion across body surface
    Diffusion,
    /// Flame cells (flatworms)
    FlameCell,
    /// Nephridia (annelids, mollusks)
    Nephridia,
    /// Malpighian tubules (insects)
    MalpighianTubule,
    /// Kidneys (vertebrates)
    Kidney,
    /// Specialized excretion
    Specialized(String),
}

/// Osmoregulation strategy
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Osmoregulation {
    /// Osmoconformer (matches environment)
    Osmoconformer,
    /// Osmoregulator (maintains internal balance)
    Osmoregulator { strategy: RegulatoryStrategy },
}

/// Regulatory strategies for osmoregulation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RegulatoryStrategy {
    /// Ion pumps and transporters
    ActiveTransport,
    /// Water channels
    Aquaporins,
    /// Salt glands
    SaltGland,
    /// Behavioral adaptations
    Behavioral,
}

impl MetabolismSchema {
    /// Create a new metabolism schema
    pub fn new(
        metabolic_pathways: Vec<MetabolicPathway>,
        energy_budget: EnergyBudget,
        thermoregulation: Thermoregulation,
        respiratory_system: RespiratorySystem,
        waste_management: WasteManagement,
    ) -> Self {
        Self {
            metabolic_pathways,
            energy_budget,
            thermoregulation,
            respiratory_system,
            waste_management,
        }
    }

    /// Validate energy consistency
    ///
    /// Ensures energy budget matches metabolic capabilities
    pub fn validate_energy_consistency(
        &self,
    ) -> Result<(), crate::organisms::SchemaValidationError> {
        // Check if energy allocation sums to 100%
        let total_allocation = self.energy_budget.energy_allocation.maintenance_percentage
            + self.energy_budget.energy_allocation.growth_percentage
            + self.energy_budget.energy_allocation.reproduction_percentage
            + self.energy_budget.energy_allocation.activity_percentage
            + self.energy_budget.energy_allocation.storage_percentage;

        if (total_allocation - 100.0).abs() > 0.1 {
            return Err(
                crate::organisms::SchemaValidationError::MetabolismInconsistent(format!(
                    "Energy allocation sums to {:.1}%, should be 100%",
                    total_allocation
                )),
            );
        }

        // Check if metabolic pathways support energy requirements
        let max_efficiency = self
            .metabolic_pathways
            .iter()
            .map(|p| p.efficiency)
            .fold(0.0, f64::max);

        if max_efficiency < 0.3
            && self.energy_budget.active_metabolic_rate
                > self.energy_budget.basal_metabolic_rate * 2.0
        {
            return Err(
                crate::organisms::SchemaValidationError::MetabolismInconsistent(
                    "Low metabolic efficiency cannot support high energy demands".to_string(),
                ),
            );
        }

        // Check respiratory system compatibility with metabolic demands
        if self.respiratory_system.gas_exchange_efficiency < 0.5
            && self.energy_budget.active_metabolic_rate
                > self.energy_budget.basal_metabolic_rate * 3.0
        {
            return Err(
                crate::organisms::SchemaValidationError::MetabolismInconsistent(
                    "Respiratory efficiency insufficient for metabolic demands".to_string(),
                ),
            );
        }

        Ok(())
    }

    /// Get metabolic pathways by type
    pub fn get_pathways_by_type(&self, pathway_type: &PathwayType) -> Vec<&MetabolicPathway> {
        self.metabolic_pathways
            .iter()
            .filter(|p| p.pathway_type == *pathway_type)
            .collect()
    }

    /// Check if organism is endothermic
    pub fn is_endothermic(&self) -> bool {
        matches!(
            self.thermoregulation.regulation_type,
            RegulationType::Endothermic
                | RegulationType::Mesothermic
                | RegulationType::RegionalEndothermy { .. }
        )
    }

    /// Calculate total metabolic efficiency
    pub fn overall_efficiency(&self) -> f64 {
        let pathway_efficiency: f64 = self
            .metabolic_pathways
            .iter()
            .map(|p| p.efficiency)
            .sum::<f64>()
            / self.metabolic_pathways.len().max(1) as f64;

        let respiratory_efficiency = self.respiratory_system.gas_exchange_efficiency;
        let storage_efficiency = self.energy_budget.energy_storage.conversion_efficiency;

        (pathway_efficiency + respiratory_efficiency + storage_efficiency) / 3.0
    }

    /// Get water requirements based on waste type
    pub fn water_requirement_level(&self) -> WaterConservationLevel {
        match self.waste_management.nitrogen_waste_type {
            NitrogenWasteType::Ammonia => WaterConservationLevel::None,
            NitrogenWasteType::Urea => WaterConservationLevel::Moderate,
            NitrogenWasteType::UricAcid => WaterConservationLevel::High,
            NitrogenWasteType::Specialized(_) => WaterConservationLevel::Moderate,
        }
    }
}

impl Default for MetabolismSchema {
    fn default() -> Self {
        Self {
            metabolic_pathways: vec![MetabolicPathway {
                pathway_name: "aerobic_respiration".to_string(),
                pathway_type: PathwayType::Aerobic,
                efficiency: 0.4,
                energy_yield: 36.0,
                substrate_requirements: vec![SubstrateRequirement {
                    substrate_name: "glucose".to_string(),
                    concentration_required: 1.0,
                    availability: Availability::Environmental,
                }],
                environmental_conditions: EnvironmentalConditions {
                    temperature_range: (10.0, 40.0),
                    pressure_range: (0.5, 2.0),
                    ph_range: (6.5, 8.0),
                    oxygen_requirement: OxygenRequirement::ObligateAerobe,
                },
            }],
            energy_budget: EnergyBudget {
                basal_metabolic_rate: 1.0,
                active_metabolic_rate: 3.0,
                energy_storage: EnergyStorage {
                    storage_type: StorageType::Lipid,
                    capacity_relative_to_body_mass: 0.2,
                    conversion_efficiency: 0.9,
                    depletion_rate: 0.05,
                },
                energy_allocation: EnergyAllocation {
                    maintenance_percentage: 60.0,
                    growth_percentage: 15.0,
                    reproduction_percentage: 10.0,
                    activity_percentage: 10.0,
                    storage_percentage: 5.0,
                },
            },
            thermoregulation: Thermoregulation {
                regulation_type: RegulationType::Ectothermic,
                temperature_tolerance: TemperatureTolerance {
                    critical_minimum: 0.0,
                    critical_maximum: 45.0,
                    optimal_range: (20.0, 30.0),
                    acclimation_capacity: 5.0,
                },
                heat_exchange_mechanisms: vec![
                    HeatExchangeMechanism::Radiation,
                    HeatExchangeMechanism::Behavioral,
                ],
                metabolic_heat_production: MetabolicHeatProduction {
                    basal_heat_production: 0.1,
                    activity_heat_multiplier: 2.0,
                    non_shivering_thermogenesis: false,
                    brown_fat_present: false,
                },
            },
            respiratory_system: RespiratorySystem {
                respiratory_type: RespiratoryType::Cutaneous,
                gas_exchange_efficiency: 0.3,
                oxygen_extraction_efficiency: 0.4,
                breathing_mechanism: BreathingMechanism::Passive,
            },
            waste_management: WasteManagement {
                nitrogen_waste_type: NitrogenWasteType::Ammonia,
                water_conservation_level: WaterConservationLevel::None,
                excretion_mechanisms: vec![ExcretionMechanism::Diffusion],
                osmoregulation: Osmoregulation::Osmoconformer,
            },
        }
    }
}
