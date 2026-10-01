use super::evolution::{EvolutionaryPressure, Species};
use super::genetics::Genome;
use super::nervous_system::{NeuralState, SensoryInput};
/// Marine vs Terrestrial divergence for MARR'KENA animals (Phase 4 Enhanced)
use mk_core::canon::CanonLocked;
use serde::{Deserialize, Serialize};

pub const MARINE_INTELLIGENCE_MAX: f64 = 0.30;
pub const TERRESTRIAL_INTELLIGENCE_MAX: f64 = 0.35;

/// Phase 4: Divergence pressure coefficients
pub const MARINE_PRESSURE_COEFFICIENT: f64 = 1.2;
pub const TERRESTRIAL_PRESSURE_COEFFICIENT: f64 = 1.0;
pub const AMPHIBIOUS_PRESSURE_COEFFICIENT: f64 = 1.1;

/// Phase 4: Environmental coupling constants
pub const MARINE_ENVIRONMENTAL_COUPLING: f64 = 0.8;
pub const TERRESTRIAL_ENVIRONMENTAL_COUPLING: f64 = 0.6;
pub const AMPHIBIOUS_ENVIRONMENTAL_COUPLING: f64 = 0.7;
pub const GENERALIST_ENVIRONMENTAL_COUPLING: f64 = 0.5;
const MAX_SENSORY_BUFFER_ENTRIES: usize = 64;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EnvironmentalSpecialization {
    Marine,
    Terrestrial,
    Amphibious,
    Generalist,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DivergenceTraits {
    pub specialization: EnvironmentalSpecialization,
    pub sensory_bias: SensoryBias,
    pub neural_adaptation: NeuralAdaptation,
    pub metabolic_adaptation: MetabolicAdaptation,
    pub intelligence_ceiling: f64,
    pub environmental_coupling: f64,

    /// Phase 4: Divergence pressure tracking
    pub divergence_pressure: f64,

    /// Phase 4: Adaptive capacity
    pub adaptive_capacity: f64,

    /// Phase 4: Niche overlap index
    pub niche_overlap: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SensoryBias {
    pub visual_bias: f64,
    pub acoustic_bias: f64,
    pub pressure_bias: f64,
    pub chemical_bias: f64,
    pub temperature_bias: f64,
    pub electromagnetic_bias: f64,

    /// Phase 4: Cross-modal integration
    pub cross_modal_integration: f64,

    /// Phase 4: Environmental filtering efficiency
    pub environmental_filtering: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NeuralAdaptation {
    pub processing_speed_modifier: f64,
    pub memory_duration_modifier: f64,
    pub learning_capacity_modifier: f64,
    pub reflex_complexity_modifier: f64,

    /// Phase 4: Environmental processing bias
    pub environmental_processing_bias: f64,

    /// Phase 4: Multi-tasking efficiency
    pub multi_tasking_efficiency: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MetabolicAdaptation {
    pub thermal_regulation_efficiency: f64,
    pub water_conservation_efficiency: f64,
    pub oxygen_utilization_efficiency: f64,
    pub pressure_tolerance: f64,

    /// Phase 4: Environmental energy efficiency
    pub environmental_energy_efficiency: f64,

    /// Phase 4: Resource utilization strategy
    pub resource_utilization_strategy: f64,
}

/// Phase 4: Divergence metrics for evolutionary tracking
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DivergenceMetrics {
    pub marine_species_count: usize,
    pub terrestrial_species_count: usize,
    pub amphibious_species_count: usize,
    pub generalist_species_count: usize,
    pub average_divergence_pressure: f64,
    pub niche_partitioning_index: f64,
    pub cross_environmental_gene_flow: f64,
}

impl DivergenceTraits {
    pub fn from_genome(genome: &Genome, _canon: &CanonLocked) -> Self {
        let specialization = Self::determine_specialization(genome);
        let sensory_bias = Self::generate_sensory_bias(genome, &specialization);
        let neural_adaptation = Self::generate_neural_adaptation(genome, &specialization);
        let metabolic_adaptation = Self::generate_metabolic_adaptation(genome, &specialization);
        let intelligence_ceiling = Self::calculate_intelligence_ceiling(&specialization);
        let environmental_coupling = Self::calculate_environmental_coupling(&specialization);

        // Phase 4: Calculate divergence pressure
        let divergence_pressure = Self::calculate_divergence_pressure(genome, &specialization);
        let adaptive_capacity = Self::calculate_adaptive_capacity(genome, &specialization);
        let niche_overlap = Self::calculate_niche_overlap(genome, &specialization);

        Self {
            specialization,
            sensory_bias,
            neural_adaptation,
            metabolic_adaptation,
            intelligence_ceiling,
            environmental_coupling,
            divergence_pressure,
            adaptive_capacity,
            niche_overlap,
        }
    }

    /// Phase 4: Calculate divergence pressure based on environmental specialization
    fn calculate_divergence_pressure(
        genome: &Genome,
        specialization: &EnvironmentalSpecialization,
    ) -> f64 {
        let base_pressure = match specialization {
            EnvironmentalSpecialization::Marine => MARINE_PRESSURE_COEFFICIENT,
            EnvironmentalSpecialization::Terrestrial => TERRESTRIAL_PRESSURE_COEFFICIENT,
            EnvironmentalSpecialization::Amphibious => AMPHIBIOUS_PRESSURE_COEFFICIENT,
            EnvironmentalSpecialization::Generalist => 0.8, // Lower pressure for generalists
        };

        // Body mass influences divergence pressure
        let mass_factor = (genome.body_mass_kg().log10() / 3.0).clamp(0.5, 2.0);

        // Neural complexity increases divergence pressure
        let neural_factor = (genome.neural.intel_index as f64 / 15.0).clamp(0.5, 1.5);

        base_pressure * mass_factor * neural_factor
    }

    /// Phase 4: Calculate adaptive capacity
    fn calculate_adaptive_capacity(
        genome: &Genome,
        specialization: &EnvironmentalSpecialization,
    ) -> f64 {
        let sensory_capacity = (genome.sensory.visual_bandwidth as f64
            + genome.sensory.acoustic_bandwidth as f64
            + genome.sensory.pressure_sensitivity as f64
            + genome.sensory.chemosensation_range as f64)
            / 44.0;

        let neural_capacity = (genome.neural.reflex_complexity as f64
            + genome.neural.learning_capacity as f64
            + genome.neural.memory_duration as f64)
            / 21.0;

        let metabolic_capacity = (genome.metabolic.basal_rate as f64
            + genome.metabolic.active_rate as f64
            + genome.metabolic.thermal_tolerance as f64
            + genome.metabolic.water_conservation as f64)
            / 44.0;

        let base_capacity = (sensory_capacity + neural_capacity + metabolic_capacity) / 3.0;

        // Environmental specialization modifies adaptive capacity
        let specialization_modifier = match specialization {
            EnvironmentalSpecialization::Marine => 1.1, // Marine species highly adaptable
            EnvironmentalSpecialization::Terrestrial => 1.0,
            EnvironmentalSpecialization::Amphibious => 1.2, // Amphibious species most adaptable
            EnvironmentalSpecialization::Generalist => 1.3, // Generalists highly adaptable
        };

        base_capacity * specialization_modifier
    }

    /// Phase 4: Calculate niche overlap with other specializations
    fn calculate_niche_overlap(
        genome: &Genome,
        specialization: &EnvironmentalSpecialization,
    ) -> f64 {
        let base_overlap = match specialization {
            EnvironmentalSpecialization::Marine => 0.3, // Marine niches are distinct
            EnvironmentalSpecialization::Terrestrial => 0.4, // Terrestrial niches moderately distinct
            EnvironmentalSpecialization::Amphibious => 0.7,  // Amphibious overlap with both
            EnvironmentalSpecialization::Generalist => 0.9,  // Generalists overlap with all
        };

        // Body mass affects niche overlap (larger animals have broader niches)
        let mass_modifier = (genome.body_mass_kg().log10() / 4.0).min(0.3);

        base_overlap + mass_modifier
    }

    /// Phase 4: Apply divergence pressures to species evolution
    pub fn apply_divergence_pressures(&self, species: &mut Species) {
        // Apply environmental-specific pressures
        match self.specialization {
            EnvironmentalSpecialization::Marine => {
                species
                    .evolutionary_pressures
                    .push(EvolutionaryPressure::Environmental {
                        stress_factor: self.divergence_pressure * 0.1,
                        temperature_change: 0.0,
                        pressure_change: self.divergence_pressure * 0.1,
                    });
                // Marine species face pressure for better pressure tolerance
                species
                    .evolutionary_pressures
                    .push(EvolutionaryPressure::Competition {
                        resource_pressure: self.divergence_pressure * 0.05,
                    });
            }
            EnvironmentalSpecialization::Terrestrial => {
                species
                    .evolutionary_pressures
                    .push(EvolutionaryPressure::Environmental {
                        stress_factor: self.divergence_pressure * 0.05,
                        temperature_change: self.divergence_pressure * 0.05,
                        pressure_change: 0.0,
                    });
                // Terrestrial species face pressure for better thermal regulation
                species
                    .evolutionary_pressures
                    .push(EvolutionaryPressure::Competition {
                        resource_pressure: self.divergence_pressure * 0.08,
                    });
            }
            EnvironmentalSpecialization::Amphibious => {
                // Amphibious species face dual pressures
                species
                    .evolutionary_pressures
                    .push(EvolutionaryPressure::Environmental {
                        stress_factor: self.divergence_pressure * 0.04,
                        temperature_change: self.divergence_pressure * 0.03,
                        pressure_change: self.divergence_pressure * 0.03,
                    });
                species
                    .evolutionary_pressures
                    .push(EvolutionaryPressure::Competition {
                        resource_pressure: self.divergence_pressure * 0.06,
                    });
            }
            EnvironmentalSpecialization::Generalist => {
                // Generalists face lower but broader pressures
                species
                    .evolutionary_pressures
                    .push(EvolutionaryPressure::Environmental {
                        stress_factor: self.divergence_pressure * 0.03,
                        temperature_change: self.divergence_pressure * 0.02,
                        pressure_change: self.divergence_pressure * 0.01,
                    });
                species
                    .evolutionary_pressures
                    .push(EvolutionaryPressure::Competition {
                        resource_pressure: self.divergence_pressure * 0.04,
                    });
            }
        }

        // Apply adaptive capacity pressure
        if self.adaptive_capacity > 0.8 {
            species
                .evolutionary_pressures
                .push(EvolutionaryPressure::Intelligence {
                    selection_for_intelligence: 0.02,
                });
        }

        // Apply niche overlap pressure (high overlap increases competition)
        if self.niche_overlap > 0.7 {
            species
                .evolutionary_pressures
                .push(EvolutionaryPressure::Competition {
                    resource_pressure: self.niche_overlap * 0.1,
                });
        }
    }

    /// Phase 4: Calculate cross-environmental gene flow potential
    pub fn calculate_gene_flow_potential(&self, other: &DivergenceTraits) -> f64 {
        // Different specializations have different gene flow barriers
        let specialization_compatibility = match (&self.specialization, &other.specialization) {
            (EnvironmentalSpecialization::Marine, EnvironmentalSpecialization::Marine) => 0.9,
            (
                EnvironmentalSpecialization::Terrestrial,
                EnvironmentalSpecialization::Terrestrial,
            ) => 0.9,
            (EnvironmentalSpecialization::Amphibious, EnvironmentalSpecialization::Amphibious) => {
                0.85
            }
            (EnvironmentalSpecialization::Generalist, EnvironmentalSpecialization::Generalist) => {
                0.95
            }
            (EnvironmentalSpecialization::Amphibious, EnvironmentalSpecialization::Marine) => 0.4,
            (EnvironmentalSpecialization::Amphibious, EnvironmentalSpecialization::Terrestrial) => {
                0.4
            }
            (EnvironmentalSpecialization::Marine, EnvironmentalSpecialization::Amphibious) => 0.4,
            (EnvironmentalSpecialization::Terrestrial, EnvironmentalSpecialization::Amphibious) => {
                0.4
            }
            (EnvironmentalSpecialization::Generalist, _) => 0.6,
            (_, EnvironmentalSpecialization::Generalist) => 0.6,
            (EnvironmentalSpecialization::Marine, EnvironmentalSpecialization::Terrestrial) => 0.1,
            (EnvironmentalSpecialization::Terrestrial, EnvironmentalSpecialization::Marine) => 0.1,
        };

        // Adaptive capacity similarity increases gene flow
        let adaptive_similarity = 1.0 - (self.adaptive_capacity - other.adaptive_capacity).abs();

        // Niche overlap increases gene flow
        let overlap_factor = (self.niche_overlap + other.niche_overlap) / 2.0;

        specialization_compatibility * adaptive_similarity * overlap_factor
    }

    fn determine_specialization(genome: &Genome) -> EnvironmentalSpecialization {
        match genome.structural.environmental_adaptation {
            0 => EnvironmentalSpecialization::Marine,
            1 => EnvironmentalSpecialization::Terrestrial,
            2 => EnvironmentalSpecialization::Amphibious,
            3 => EnvironmentalSpecialization::Generalist,
            _ => EnvironmentalSpecialization::Terrestrial,
        }
    }

    fn generate_sensory_bias(
        genome: &Genome,
        specialization: &EnvironmentalSpecialization,
    ) -> SensoryBias {
        let sensory = &genome.sensory;

        match specialization {
            EnvironmentalSpecialization::Marine => SensoryBias {
                visual_bias: (sensory.visual_bandwidth as f64 / 15.0) * 0.3,
                acoustic_bias: (sensory.acoustic_bandwidth as f64 / 15.0) * 0.4,
                pressure_bias: (sensory.pressure_sensitivity as f64 / 7.0) * 0.9,
                chemical_bias: (sensory.chemosensation_range as f64 / 7.0) * 0.8,
                temperature_bias: 0.2,
                electromagnetic_bias: (sensory.electrosensitivity as f64 / 3.0) * 0.7,
                cross_modal_integration: 0.3,
                environmental_filtering: 0.8,
            },
            EnvironmentalSpecialization::Terrestrial => SensoryBias {
                visual_bias: (sensory.visual_bandwidth as f64 / 15.0) * 0.8,
                acoustic_bias: (sensory.acoustic_bandwidth as f64 / 15.0) * 0.7,
                pressure_bias: (sensory.pressure_sensitivity as f64 / 7.0) * 0.3,
                chemical_bias: (sensory.chemosensation_range as f64 / 7.0) * 0.6,
                temperature_bias: 0.7,
                electromagnetic_bias: (sensory.electrosensitivity as f64 / 3.0) * 0.2,
                cross_modal_integration: 0.5,
                environmental_filtering: 0.6,
            },
            EnvironmentalSpecialization::Amphibious => SensoryBias {
                visual_bias: 0.6,
                acoustic_bias: 0.6,
                pressure_bias: 0.6,
                chemical_bias: 0.7,
                temperature_bias: 0.5,
                electromagnetic_bias: 0.4,
                cross_modal_integration: 0.4,
                environmental_filtering: 0.7,
            },
            EnvironmentalSpecialization::Generalist => SensoryBias {
                visual_bias: 0.5,
                acoustic_bias: 0.5,
                pressure_bias: 0.5,
                chemical_bias: 0.5,
                temperature_bias: 0.5,
                electromagnetic_bias: 0.3,
                cross_modal_integration: 0.2,
                environmental_filtering: 0.4,
            },
        }
    }

    fn generate_neural_adaptation(
        genome: &Genome,
        specialization: &EnvironmentalSpecialization,
    ) -> NeuralAdaptation {
        let _neural = &genome.neural;

        match specialization {
            EnvironmentalSpecialization::Marine => NeuralAdaptation {
                processing_speed_modifier: 0.8,
                memory_duration_modifier: 0.7,
                learning_capacity_modifier: 0.6,
                reflex_complexity_modifier: 0.9,
                environmental_processing_bias: 0.8,
                multi_tasking_efficiency: 0.4,
            },
            EnvironmentalSpecialization::Terrestrial => NeuralAdaptation {
                processing_speed_modifier: 1.0,
                memory_duration_modifier: 1.0,
                learning_capacity_modifier: 1.0,
                reflex_complexity_modifier: 1.0,
                environmental_processing_bias: 0.6,
                multi_tasking_efficiency: 0.8,
            },
            EnvironmentalSpecialization::Amphibious => NeuralAdaptation {
                processing_speed_modifier: 0.9,
                memory_duration_modifier: 0.9,
                learning_capacity_modifier: 0.8,
                reflex_complexity_modifier: 0.95,
                environmental_processing_bias: 0.7,
                multi_tasking_efficiency: 0.6,
            },
            EnvironmentalSpecialization::Generalist => NeuralAdaptation {
                processing_speed_modifier: 0.85,
                memory_duration_modifier: 0.85,
                learning_capacity_modifier: 0.85,
                reflex_complexity_modifier: 0.85,
                environmental_processing_bias: 0.5,
                multi_tasking_efficiency: 0.5,
            },
        }
    }

    fn generate_metabolic_adaptation(
        genome: &Genome,
        specialization: &EnvironmentalSpecialization,
    ) -> MetabolicAdaptation {
        let _metabolic = &genome.metabolic;

        match specialization {
            EnvironmentalSpecialization::Marine => MetabolicAdaptation {
                thermal_regulation_efficiency: 0.3, // Poor thermoregulation in water
                water_conservation_efficiency: 0.9, // Excellent water conservation
                oxygen_utilization_efficiency: 0.7, // Moderate oxygen use
                pressure_tolerance: 0.8,            // Good pressure tolerance
                environmental_energy_efficiency: 0.9,
                resource_utilization_strategy: 0.3,
            },
            EnvironmentalSpecialization::Terrestrial => MetabolicAdaptation {
                thermal_regulation_efficiency: 0.8, // Good thermoregulation
                water_conservation_efficiency: 0.6, // Poor water conservation
                oxygen_utilization_efficiency: 0.9, // Good oxygen use
                pressure_tolerance: 0.3,            // Poor pressure tolerance
                environmental_energy_efficiency: 0.4,
                resource_utilization_strategy: 0.8,
            },
            EnvironmentalSpecialization::Amphibious => MetabolicAdaptation {
                thermal_regulation_efficiency: 0.6,
                water_conservation_efficiency: 0.7,
                oxygen_utilization_efficiency: 0.8,
                pressure_tolerance: 0.6,
                environmental_energy_efficiency: 0.6,
                resource_utilization_strategy: 0.6,
            },
            EnvironmentalSpecialization::Generalist => MetabolicAdaptation {
                thermal_regulation_efficiency: 0.7,
                water_conservation_efficiency: 0.7,
                oxygen_utilization_efficiency: 0.8,
                pressure_tolerance: 0.5,
                environmental_energy_efficiency: 0.5,
                resource_utilization_strategy: 0.5,
            },
        }
    }

    fn calculate_intelligence_ceiling(specialization: &EnvironmentalSpecialization) -> f64 {
        match specialization {
            EnvironmentalSpecialization::Marine => MARINE_INTELLIGENCE_MAX,
            EnvironmentalSpecialization::Terrestrial => TERRESTRIAL_INTELLIGENCE_MAX,
            EnvironmentalSpecialization::Amphibious => {
                (MARINE_INTELLIGENCE_MAX + TERRESTRIAL_INTELLIGENCE_MAX) / 2.0
            }
            EnvironmentalSpecialization::Generalist => TERRESTRIAL_INTELLIGENCE_MAX * 0.9,
        }
    }

    fn calculate_environmental_coupling(specialization: &EnvironmentalSpecialization) -> f64 {
        match specialization {
            EnvironmentalSpecialization::Marine => 0.9, // Strong coupling to ocean
            EnvironmentalSpecialization::Terrestrial => 0.6, // Moderate coupling to land
            EnvironmentalSpecialization::Amphibious => 0.8, // Strong coupling to both
            EnvironmentalSpecialization::Generalist => 0.4, // Weak coupling to any
        }
    }

    pub fn check_intelligence_ceiling(&self, intelligence_index: f64) -> bool {
        intelligence_index > self.intelligence_ceiling
    }

    pub fn apply_sensory_bias(&self, neural_state: &mut NeuralState, input: SensoryInput) {
        let bias = match input {
            SensoryInput::Visual { .. } => self.sensory_bias.visual_bias,
            SensoryInput::Auditory { .. } => self.sensory_bias.acoustic_bias,
            SensoryInput::Pressure { .. } => self.sensory_bias.pressure_bias,
            SensoryInput::Chemical { .. } => self.sensory_bias.chemical_bias,
            SensoryInput::Temperature { .. } => self.sensory_bias.temperature_bias,
            SensoryInput::Electromagnetic { .. } => self.sensory_bias.electromagnetic_bias,
        };

        // Modify neural processing based on bias
        neural_state.processing_load *= 1.0 + bias * 0.5;
    }

    pub fn apply_neural_adaptation(&self, neural_state: &mut NeuralState) {
        // NeuralState does not retain timestamps, so duration cannot be simulated by
        // selectively expiring entries. Keep the most recent bounded window instead of
        // silently discarding the entire sensory history.
        if neural_state.sensory_buffer.len() > MAX_SENSORY_BUFFER_ENTRIES {
            let first_retained = neural_state.sensory_buffer.len() - MAX_SENSORY_BUFFER_ENTRIES;
            neural_state.sensory_buffer.drain(..first_retained);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::biosphere::genetics::Genome;
    use mk_core::ids::KenzIeSubclass;
    use mk_core::rng::RngRegistry;

    #[test]
    fn test_marine_vs_terrestrial_divergence() {
        let rng = RngRegistry::new([0u8; 32]);
        let marine_genome = Genome::new(1, &rng, KenzIeSubclass::Alpha);
        let terrestrial_genome = Genome::new(2, &rng, KenzIeSubclass::Alpha);
        let canon = CanonLocked::default();

        // Override environmental adaptation for testing
        let mut marine_genome = marine_genome;
        marine_genome.structural.environmental_adaptation = 0; // Marine
        let mut terrestrial_genome = terrestrial_genome;
        terrestrial_genome.structural.environmental_adaptation = 1; // Terrestrial

        let marine_divergence = DivergenceTraits::from_genome(&marine_genome, &canon);
        let terrestrial_divergence = DivergenceTraits::from_genome(&terrestrial_genome, &canon);

        // Marine animals should have higher pressure bias
        assert!(
            marine_divergence.sensory_bias.pressure_bias
                > terrestrial_divergence.sensory_bias.pressure_bias
        );

        // Terrestrial animals should have higher visual bias
        assert!(
            terrestrial_divergence.sensory_bias.visual_bias
                > marine_divergence.sensory_bias.visual_bias
        );

        // Marine animals should have lower intelligence ceiling
        assert!(
            marine_divergence.intelligence_ceiling < terrestrial_divergence.intelligence_ceiling
        );

        // Marine animals should have stronger environmental coupling
        assert!(
            marine_divergence.environmental_coupling
                > terrestrial_divergence.environmental_coupling
        );
    }

    #[test]
    fn test_intelligence_ceiling_enforcement() {
        let rng = RngRegistry::new([0u8; 32]);
        let genome = Genome::new(1, &rng, KenzIeSubclass::Alpha);
        let canon = CanonLocked::default();

        let marine_divergence = DivergenceTraits::from_genome(&genome, &canon);

        // Should enforce marine intelligence ceiling
        assert!(marine_divergence.check_intelligence_ceiling(MARINE_INTELLIGENCE_MAX + 100.0));
        assert!(!marine_divergence.check_intelligence_ceiling(MARINE_INTELLIGENCE_MAX - 100.0));
    }

    #[test]
    fn neural_adaptation_preserves_recent_sensory_history() {
        let rng = RngRegistry::new([0u8; 32]);
        let genome = Genome::new(7, &rng, KenzIeSubclass::Alpha);
        let divergence = DivergenceTraits::from_genome(&genome, &CanonLocked::default());
        let mut neural_state = NeuralState {
            arousal_level: 0.5,
            processing_load: 0.5,
            sensory_buffer: (0..=64)
                .map(|intensity| SensoryInput::Visual {
                    intensity: intensity as f64,
                })
                .collect(),
        };

        divergence.apply_neural_adaptation(&mut neural_state);

        assert_eq!(neural_state.sensory_buffer.len(), 64);
        assert!(matches!(
            neural_state.sensory_buffer.first(),
            Some(SensoryInput::Visual { intensity }) if *intensity == 1.0
        ));
    }
}
