//! Learning Snapshot - plasticity, learning processes, adaptive traits,
//! experience integration.
//!
//! Surfaces `docs/canon/HumanReplicationSchema.js`'s `LearningAdaptationSchema`,
//! ported in `mk_core::human::schema` but previously unread by the engine.
//! Coupled to `memory`/`development`: sustained memory consolidation drives
//! wisdom accumulation, and neural plasticity declines with age, mirroring
//! real learning/aging effects.

use mk_core::human::HumanProfile;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningSnapshot {
    // --- neural plasticity (traits) ---
    pub synaptic_plasticity: f64,
    pub structural_plasticity: f64,
    pub plasticity_decline_rate: f64,

    // --- learning processes (traits) ---
    pub acquisition_rate: f64,
    pub retention_rate: f64,
    pub transfer_ability: f64,
    pub generalization: f64,

    // --- adaptive traits (traits) ---
    pub logic_weight: f64,
    pub efficiency_weight: f64,
    pub emotion_weight: f64,
    pub creativity_weight: f64,
    pub social_weight: f64,

    // --- state (stepped) ---
    /// Current effective plasticity after age-related decline (0-1).
    pub current_plasticity: f64,
    /// Pattern-recognition skill accrued from experience (0-1).
    pub pattern_recognition: f64,
    /// Wisdom accumulated from sustained, consolidated experience (0-1).
    pub wisdom_accumulation: f64,
}

impl LearningSnapshot {
    pub fn from_profile(profile: &HumanProfile) -> Self {
        let Some(schema) = profile.canonical_schema() else {
            return Self::defaults();
        };
        let l = &schema.learning_adaptation;

        Self {
            synaptic_plasticity: nonzero_or(l.neural_plasticity.synaptic_plasticity, 0.7),
            structural_plasticity: nonzero_or(l.neural_plasticity.structural_plasticity, 0.5),
            plasticity_decline_rate: nonzero_or(l.neural_plasticity.plasticity_decline, 0.1),

            acquisition_rate: nonzero_or(l.learning_processes.acquisition_rate, 0.5),
            retention_rate: nonzero_or(l.learning_processes.retention_rate, 0.5),
            transfer_ability: nonzero_or(l.learning_processes.transfer_ability, 0.4),
            generalization: nonzero_or(l.learning_processes.generalization, 0.4),

            logic_weight: nonzero_or(l.adaptive_traits.logic_weight, 0.5),
            efficiency_weight: nonzero_or(l.adaptive_traits.efficiency_weight, 0.5),
            emotion_weight: nonzero_or(l.adaptive_traits.emotion_weight, 0.5),
            creativity_weight: nonzero_or(l.adaptive_traits.creativity_weight, 0.5),
            social_weight: nonzero_or(l.adaptive_traits.social_weight, 0.5),

            current_plasticity: nonzero_or(l.neural_plasticity.synaptic_plasticity, 0.7),
            pattern_recognition: nonzero_or(l.experience_integration.pattern_recognition, 0.3),
            wisdom_accumulation: nonzero_or(l.experience_integration.wisdom_accumulation, 0.0),
        }
    }

    /// Couple plasticity to age and wisdom/pattern-recognition to sustained
    /// memory consolidation instead of a static schema readout.
    /// `learning_rate_multiplier` is neurochemistry's dopamine-driven
    /// modulation (1 = neutral). The tick pipeline passes the previous
    /// tick's value, since neurochemistry steps after learning.
    pub fn step(
        &self,
        memory: &super::memory::MemorySnapshot,
        development: &super::development::DevelopmentSnapshot,
        learning_rate_multiplier: f64,
        dt_years: f64,
    ) -> Self {
        let dt = dt_years.max(0.0);

        // Plasticity declines with maturity (fastest early, then slower).
        let decline = self.plasticity_decline_rate * development.maturity_index * dt;
        let current_plasticity = (self.current_plasticity - decline).clamp(0.05, 1.0);

        // Pattern recognition and wisdom accrue from consolidated memory,
        // scaled by acquisition rate and current plasticity.
        let learning_signal = memory.consolidation_level
            * self.acquisition_rate
            * current_plasticity
            * learning_rate_multiplier.max(0.0)
            * dt;

        let pattern_recognition =
            (self.pattern_recognition + learning_signal * 0.5).clamp(0.0, 1.0);
        let wisdom_accumulation = (self.wisdom_accumulation
            + learning_signal * self.retention_rate * 0.3)
            .clamp(0.0, 1.0);

        Self {
            current_plasticity,
            pattern_recognition,
            wisdom_accumulation,
            ..self.clone()
        }
    }

    fn defaults() -> Self {
        Self {
            synaptic_plasticity: 0.7,
            structural_plasticity: 0.5,
            plasticity_decline_rate: 0.1,
            acquisition_rate: 0.5,
            retention_rate: 0.5,
            transfer_ability: 0.4,
            generalization: 0.4,
            logic_weight: 0.5,
            efficiency_weight: 0.5,
            emotion_weight: 0.5,
            creativity_weight: 0.5,
            social_weight: 0.5,
            current_plasticity: 0.7,
            pattern_recognition: 0.3,
            wisdom_accumulation: 0.0,
        }
    }
}

fn nonzero_or(value: f32, default: f64) -> f64 {
    if value == 0.0 {
        default
    } else {
        value as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::humans::development::DevelopmentSnapshot;
    use crate::humans::memory::MemorySnapshot;
    use mk_core::human::{HumanId, HumanSchema};

    fn profile() -> HumanProfile {
        let schema = HumanSchema::canonical_minimal("learning_test");
        HumanProfile::from_canonical_schema(HumanId::new(1), schema)
    }

    #[test]
    fn from_profile_uses_sane_defaults() {
        let snapshot = LearningSnapshot::from_profile(&profile());
        assert!(snapshot.synaptic_plasticity > 0.0);
        assert!(snapshot.acquisition_rate > 0.0);
    }

    #[test]
    fn wisdom_accumulates_with_sustained_consolidated_memory() {
        let snapshot = LearningSnapshot::from_profile(&profile());
        let development = DevelopmentSnapshot::from_profile(&profile());
        let mut memory = MemorySnapshot::from_profile(&profile());
        memory.consolidation_level = 1.0;

        let mut stepped = snapshot.clone();
        for _ in 0..10 {
            stepped = stepped.step(&memory, &development, 1.0, 1.0);
        }

        assert!(stepped.wisdom_accumulation > 0.0);
        assert!(stepped.pattern_recognition > snapshot.pattern_recognition);
    }

    #[test]
    fn plasticity_declines_with_maturity() {
        let snapshot = LearningSnapshot::from_profile(&profile());
        let memory = MemorySnapshot::from_profile(&profile());
        let mut mature_development = DevelopmentSnapshot::from_profile(&profile());
        mature_development.maturity_index = 1.0;

        let stepped = snapshot.step(&memory, &mature_development, 1.0, 5.0);
        assert!(stepped.current_plasticity <= snapshot.current_plasticity);
    }

    #[test]
    fn dopamine_learning_rate_multiplier_scales_learning() {
        let snapshot = LearningSnapshot::from_profile(&profile());
        let development = DevelopmentSnapshot::from_profile(&profile());
        let mut memory = MemorySnapshot::from_profile(&profile());
        memory.consolidation_level = 1.0;
        let gain = |multiplier: f64| {
            snapshot
                .step(&memory, &development, multiplier, 0.1)
                .pattern_recognition
                - snapshot.pattern_recognition
        };
        assert!(gain(2.0) > gain(1.0));
        assert!(gain(1.0) > gain(0.5));
        assert_eq!(gain(0.0), 0.0);
    }
}
