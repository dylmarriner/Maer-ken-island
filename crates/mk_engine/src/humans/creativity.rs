//! Creativity Snapshot - creative thinking, problem solving, innovation,
//! and aesthetic sensitivity.
//!
//! Surfaces `docs/canon/HumanReplicationSchema.js`'s `CreativeSystemsSchema`,
//! ported in `mk_core::human::schema` but previously unread by the engine.
//! Coupled to `attention`/`learning`: flow state amplifies divergent
//! thinking and insight generation, while accrued pattern recognition
//! feeds creative problem solving, mirroring real creative-cognition
//! effects.

use mk_core::human::HumanProfile;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreativitySnapshot {
    // --- creative thinking (traits) ---
    pub divergent_thinking: f64,
    pub convergent_thinking: f64,
    pub originality: f64,
    pub flexibility: f64,

    // --- problem solving (traits) ---
    pub analytical_solving: f64,
    pub creative_solving: f64,
    pub insight_generation: f64,

    // --- innovation (traits) ---
    pub novelty_seeking: f64,
    pub risk_tolerance: f64,
    pub experimentation: f64,

    // --- aesthetic creativity (traits) ---
    pub artistic_expression: f64,
    pub aesthetic_sensitivity: f64,
    pub symbolic_thinking: f64,

    // --- state (stepped) ---
    /// Effective divergent thinking, amplified by flow (0-1).
    pub effective_divergent_thinking: f64,
    /// Effective insight generation, fed by accrued pattern recognition
    /// (0-1).
    pub effective_insight_generation: f64,
}

impl CreativitySnapshot {
    pub fn from_profile(profile: &HumanProfile) -> Self {
        let Some(schema) = profile.canonical_schema() else {
            return Self::defaults();
        };
        let c = &schema.creative_systems;

        Self {
            divergent_thinking: nonzero_or(c.creative_thinking.divergent_thinking, 0.5),
            convergent_thinking: nonzero_or(c.creative_thinking.convergent_thinking, 0.5),
            originality: nonzero_or(c.creative_thinking.originality, 0.5),
            flexibility: nonzero_or(c.creative_thinking.flexibility, 0.5),

            analytical_solving: nonzero_or(c.problem_solving.analytical_solving, 0.5),
            creative_solving: nonzero_or(c.problem_solving.creative_solving, 0.5),
            insight_generation: nonzero_or(c.problem_solving.insight_generation, 0.4),

            novelty_seeking: nonzero_or(c.innovation.novelty_seeking, 0.5),
            risk_tolerance: nonzero_or(c.innovation.risk_tolerance, 0.5),
            experimentation: nonzero_or(c.innovation.experimentation, 0.5),

            artistic_expression: nonzero_or(c.aesthetic_creativity.artistic_expression, 0.5),
            aesthetic_sensitivity: nonzero_or(c.aesthetic_creativity.aesthetic_sensitivity, 0.5),
            symbolic_thinking: nonzero_or(c.aesthetic_creativity.symbolic_thinking, 0.5),

            effective_divergent_thinking: nonzero_or(c.creative_thinking.divergent_thinking, 0.5),
            effective_insight_generation: nonzero_or(c.problem_solving.insight_generation, 0.4),
        }
    }

    /// Couple divergent thinking to flow state and insight generation to
    /// accrued pattern recognition instead of a static schema readout.
    pub fn step(
        &self,
        attention: &super::attention::AttentionSnapshot,
        learning: &super::learning::LearningSnapshot,
    ) -> Self {
        let effective_divergent_thinking =
            (self.divergent_thinking * (1.0 + attention.flow * 0.5)).clamp(0.0, 1.0);

        let effective_insight_generation =
            (self.insight_generation + learning.pattern_recognition * 0.3).clamp(0.0, 1.0);

        Self {
            effective_divergent_thinking,
            effective_insight_generation,
            ..self.clone()
        }
    }

    fn defaults() -> Self {
        Self {
            divergent_thinking: 0.5,
            convergent_thinking: 0.5,
            originality: 0.5,
            flexibility: 0.5,
            analytical_solving: 0.5,
            creative_solving: 0.5,
            insight_generation: 0.4,
            novelty_seeking: 0.5,
            risk_tolerance: 0.5,
            experimentation: 0.5,
            artistic_expression: 0.5,
            aesthetic_sensitivity: 0.5,
            symbolic_thinking: 0.5,
            effective_divergent_thinking: 0.5,
            effective_insight_generation: 0.4,
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
    use crate::humans::attention::AttentionSnapshot;
    use crate::humans::learning::LearningSnapshot;
    use mk_core::human::{HumanId, HumanSchema};

    fn profile() -> HumanProfile {
        let schema = HumanSchema::canonical_minimal("creativity_test");
        HumanProfile::from_canonical_schema(HumanId::new(1), schema)
    }

    #[test]
    fn from_profile_uses_sane_defaults() {
        let snapshot = CreativitySnapshot::from_profile(&profile());
        assert!(snapshot.divergent_thinking > 0.0);
        assert!(snapshot.artistic_expression > 0.0);
    }

    #[test]
    fn flow_amplifies_divergent_thinking() {
        let snapshot = CreativitySnapshot::from_profile(&profile());
        let learning = LearningSnapshot::from_profile(&profile());

        let mut flowing = AttentionSnapshot::from_profile(&profile());
        flowing.flow = 1.0;
        let mut flat = AttentionSnapshot::from_profile(&profile());
        flat.flow = 0.0;

        let stepped_flowing = snapshot.step(&flowing, &learning);
        let stepped_flat = snapshot.step(&flat, &learning);

        assert!(
            stepped_flowing.effective_divergent_thinking
                >= stepped_flat.effective_divergent_thinking
        );
    }

    #[test]
    fn pattern_recognition_feeds_insight_generation() {
        let snapshot = CreativitySnapshot::from_profile(&profile());
        let attention = AttentionSnapshot::from_profile(&profile());
        let mut skilled_learning = LearningSnapshot::from_profile(&profile());
        skilled_learning.pattern_recognition = 1.0;

        let stepped = snapshot.step(&attention, &skilled_learning);
        assert!(stepped.effective_insight_generation >= snapshot.insight_generation);
    }
}
