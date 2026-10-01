use mk_core::ids::KenzIeSubclass;
/// Genetics system for Phase 3
use mk_core::rng::{RngKey, RngRegistry, SubsystemId};
use serde::{Deserialize, Serialize};

pub const PRE_SAPIENT_CEILING: f64 = 0.4;

/// Genome structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Genome {
    pub structural: StructuralTraits,
    pub metabolic: MetabolicTraits,
    pub sensory: SensoryTraits,
    pub neural: NeuralTraits,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuralTraits {
    pub size_modifier: u8,
    pub environmental_adaptation: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetabolicTraits {
    pub basal_rate: u8,
    pub neural_cost: u8,
    pub active_rate: u8,
    pub thermal_tolerance: u8,
    pub water_conservation: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensoryTraits {
    pub visual_bandwidth: u8,
    pub acoustic_bandwidth: u8,
    pub pressure_sensitivity: u8,
    pub chemosensation_range: u8,
    pub electrosensitivity: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeuralTraits {
    pub processing_capacity: u8,
    pub memory_capacity: u8,
    pub intel_index: u8,
    pub reflex_complexity: u8,
    pub learning_capacity: u8,
    pub memory_duration: u8,
}

impl Genome {
    pub fn new(seed: u64, rng: &RngRegistry, subclass: KenzIeSubclass) -> Self {
        let subclass_key = match subclass {
            KenzIeSubclass::Alpha => 0,
        };
        let next_trait = |index: u32| {
            rng.gen_u32(RngKey::new(
                SubsystemId::Genetics,
                seed as u32,
                (seed >> 32) as u32 ^ subclass_key,
                index as u64,
            )) as u8
                % 7
                + 2
        };

        Self {
            structural: StructuralTraits {
                size_modifier: next_trait(0),
                environmental_adaptation: next_trait(1) % 4,
            },
            metabolic: MetabolicTraits {
                basal_rate: next_trait(2),
                neural_cost: next_trait(3),
                active_rate: next_trait(4),
                thermal_tolerance: next_trait(5),
                water_conservation: next_trait(6),
            },
            sensory: SensoryTraits {
                // Keep the baseline sensory profile stable so environmental
                // divergence comparisons isolate specialization effects.
                visual_bandwidth: 8,
                acoustic_bandwidth: 6,
                pressure_sensitivity: 3,
                chemosensation_range: 5,
                electrosensitivity: 2,
            },
            neural: NeuralTraits {
                processing_capacity: next_trait(7),
                memory_capacity: next_trait(8),
                intel_index: next_trait(9),
                reflex_complexity: next_trait(10),
                learning_capacity: next_trait(11),
                memory_duration: next_trait(12),
            },
        }
    }

    pub fn intelligence_index(&self) -> f64 {
        (self.neural.processing_capacity as f64 / 15.0) * 0.3
    }

    pub fn exceeds_intelligence_ceiling(&self) -> bool {
        self.intelligence_index() > PRE_SAPIENT_CEILING
    }

    pub fn basal_metabolic_rate_w(&self) -> f64 {
        // Basal metabolic rate in watts
        // Scales with mass (size_modifier) and basal_rate trait
        let mass_factor = (self.structural.size_modifier as f64 / 5.0).powf(2.0 / 3.0);
        let rate_factor = self.metabolic.basal_rate as f64 / 5.0;
        10.0 * mass_factor * rate_factor
    }

    pub fn body_mass_kg(&self) -> f64 {
        match self.structural.size_modifier {
            0..=5 => 10.0,
            6..=10 => 100.0,
            _ => 1000.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn genome_seed_changes_generated_traits() {
        let rng = RngRegistry::new([7; 32]);
        let first = Genome::new(1, &rng, KenzIeSubclass::Alpha);
        let second = Genome::new(2, &rng, KenzIeSubclass::Alpha);

        assert_ne!(
            first.structural.size_modifier,
            second.structural.size_modifier
        );
    }

    #[test]
    fn genome_generation_is_repeatable_for_same_inputs() {
        let rng = RngRegistry::new([7; 32]);
        let first = Genome::new(42, &rng, KenzIeSubclass::Alpha);
        let second = Genome::new(42, &rng, KenzIeSubclass::Alpha);

        assert_eq!(
            first.neural.processing_capacity,
            second.neural.processing_capacity
        );
        assert_eq!(
            first.sensory.visual_bandwidth,
            second.sensory.visual_bandwidth
        );
    }
}
