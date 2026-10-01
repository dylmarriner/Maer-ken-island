//! Decision Snapshot - decision weights, processes, choice architecture,
//! and context sensitivity.
//!
//! Surfaces `docs/canon/HumanReplicationSchema.js`'s `DecisionMakingSchema`,
//! ported in `mk_core::human::schema` but previously unread by the engine.
//! Coupled to `learning`/`cognition`/`sensory`/`immune`: adaptive trait
//! weights learned over time bias decision weights, and current cognitive
//! load/fatigue — plus physical pain and active sickness — degrade rational
//! analysis in favor of intuitive judgment, mirroring real decision-fatigue
//! and "sick and in pain, can't think straight" effects.

use mk_core::human::HumanProfile;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionSnapshot {
    // --- decision weights (traits) ---
    pub logic_weight: f64,
    pub efficiency_weight: f64,
    pub emotion_weight: f64,
    pub creativity_weight: f64,
    pub social_weight: f64,

    // --- decision processes (traits) ---
    pub rational_analysis: f64,
    pub intuitive_judgment: f64,
    pub ethical_reasoning: f64,

    // --- choice architecture (traits) ---
    pub option_generation: f64,
    pub consequence_analysis: f64,
    pub commitment_level: f64,

    // --- state (stepped) ---
    /// Effective rational-analysis capacity after cognitive load (0-1).
    pub effective_rationality: f64,
    /// Effective intuitive-judgment weight, rises as rationality is
    /// squeezed by load (0-1).
    pub effective_intuition: f64,
    /// Effective decision commitment, strengthened by accumulated wisdom
    /// (0-1).
    pub effective_commitment: f64,
}

impl DecisionSnapshot {
    pub fn from_profile(profile: &HumanProfile) -> Self {
        let Some(schema) = profile.canonical_schema() else {
            return Self::defaults();
        };
        let d = &schema.decision_making;

        Self {
            logic_weight: nonzero_or(d.decision_weights.logic_weight, 0.5),
            efficiency_weight: nonzero_or(d.decision_weights.efficiency_weight, 0.5),
            emotion_weight: nonzero_or(d.decision_weights.emotion_weight, 0.5),
            creativity_weight: nonzero_or(d.decision_weights.creativity_weight, 0.5),
            social_weight: nonzero_or(d.decision_weights.social_weight, 0.5),

            rational_analysis: nonzero_or(d.decision_processes.rational_analysis, 0.5),
            intuitive_judgment: nonzero_or(d.decision_processes.intuitive_judgment, 0.4),
            ethical_reasoning: nonzero_or(d.decision_processes.ethical_reasoning, 0.5),

            option_generation: nonzero_or(d.choice_architecture.option_generation, 0.5),
            consequence_analysis: nonzero_or(d.choice_architecture.consequence_analysis, 0.5),
            commitment_level: nonzero_or(d.choice_architecture.commitment_level, 0.5),

            effective_rationality: nonzero_or(d.decision_processes.rational_analysis, 0.5),
            effective_intuition: nonzero_or(d.decision_processes.intuitive_judgment, 0.4),
            effective_commitment: nonzero_or(d.choice_architecture.commitment_level, 0.5),
        }
    }

    /// Couple decision weights/processes to the human's learned adaptive
    /// traits and current cognitive load instead of a static schema
    /// readout.
    pub fn step(
        &self,
        cognition: &super::cognition::HumanCognitionSnapshot,
        learning: &super::learning::LearningSnapshot,
        sensory: &super::sensory::SensorySnapshot,
        immune: &super::immune::ImmuneSnapshot,
        dt_years: f64,
    ) -> Self {
        let dt = dt_years.max(0.0);

        // Learned adaptive weights slowly pull the innate decision weights
        // toward what experience has reinforced.
        let blend = (0.2 * dt).clamp(0.0, 1.0);
        let logic_weight = lerp(self.logic_weight, learning.logic_weight, blend);
        let efficiency_weight = lerp(self.efficiency_weight, learning.efficiency_weight, blend);
        let emotion_weight = lerp(self.emotion_weight, learning.emotion_weight, blend);
        let creativity_weight = lerp(self.creativity_weight, learning.creativity_weight, blend);
        let social_weight = lerp(self.social_weight, learning.social_weight, blend);

        // Cognitive load/fatigue squeezes deliberate rational analysis;
        // intuition compensates. Physical pain and active sickness impose
        // the same squeeze — a hurting or sick body degrades deliberate
        // reasoning independently of mental cognitive load.
        let load = cognition.cognitive_load.clamp(0.0, 1.0);
        let pain = sensory.overall_pain_level.clamp(0.0, 1.0);
        let sickness = (immune.system_stress + (immune.active_pathogen_count.min(5) as f64) * 0.1)
            .clamp(0.0, 1.0);
        let physical_squeeze = (pain * 0.5 + sickness * 0.3).clamp(0.0, 1.0);
        let effective_rationality =
            (self.rational_analysis * (1.0 - load * 0.6) * (1.0 - physical_squeeze))
                .clamp(0.0, 1.0);
        let effective_intuition =
            (self.intuitive_judgment + load * 0.4 + physical_squeeze * 0.3).clamp(0.0, 1.0);

        // Commitment strengthens with accumulated wisdom.
        let effective_commitment =
            (self.commitment_level + learning.wisdom_accumulation * 0.3).clamp(0.0, 1.0);

        Self {
            logic_weight,
            efficiency_weight,
            emotion_weight,
            creativity_weight,
            social_weight,
            effective_rationality,
            effective_intuition,
            effective_commitment,
            ..self.clone()
        }
    }

    fn defaults() -> Self {
        Self {
            logic_weight: 0.5,
            efficiency_weight: 0.5,
            emotion_weight: 0.5,
            creativity_weight: 0.5,
            social_weight: 0.5,
            rational_analysis: 0.5,
            intuitive_judgment: 0.4,
            ethical_reasoning: 0.5,
            option_generation: 0.5,
            consequence_analysis: 0.5,
            commitment_level: 0.5,
            effective_rationality: 0.5,
            effective_intuition: 0.4,
            effective_commitment: 0.5,
        }
    }
}

fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t.clamp(0.0, 1.0)
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
    use crate::humans::cognition::HumanCognitionSnapshot;
    use crate::humans::immune::ImmuneSnapshot;
    use crate::humans::learning::LearningSnapshot;
    use crate::humans::sensory::SensorySnapshot;
    use mk_core::human::{HumanId, HumanSchema};

    fn profile() -> HumanProfile {
        let schema = HumanSchema::canonical_minimal("decision_test");
        HumanProfile::from_canonical_schema(HumanId::new(1), schema)
    }

    #[test]
    fn from_profile_uses_sane_defaults() {
        let snapshot = DecisionSnapshot::from_profile(&profile());
        assert!(snapshot.logic_weight > 0.0);
        assert!(snapshot.rational_analysis > 0.0);
    }

    #[test]
    fn high_cognitive_load_shifts_toward_intuition() {
        let snapshot = DecisionSnapshot::from_profile(&profile());
        let learning = LearningSnapshot::from_profile(&profile());
        let sensory = SensorySnapshot::from_profile(&profile());
        let immune = ImmuneSnapshot::from_profile(&profile());

        let mut loaded_cognition = HumanCognitionSnapshot::from_profile(&profile());
        loaded_cognition.cognitive_load = 1.0;
        let mut calm_cognition = HumanCognitionSnapshot::from_profile(&profile());
        calm_cognition.cognitive_load = 0.0;

        let loaded = snapshot.step(&loaded_cognition, &learning, &sensory, &immune, 1.0);
        let calm = snapshot.step(&calm_cognition, &learning, &sensory, &immune, 1.0);

        assert!(loaded.effective_rationality <= calm.effective_rationality);
        assert!(loaded.effective_intuition >= calm.effective_intuition);
    }

    #[test]
    fn pain_and_sickness_shift_toward_intuition() {
        let snapshot = DecisionSnapshot::from_profile(&profile());
        let learning = LearningSnapshot::from_profile(&profile());
        let cognition = HumanCognitionSnapshot::from_profile(&profile());

        let mut hurting_sensory = SensorySnapshot::from_profile(&profile());
        hurting_sensory.overall_pain_level = 1.0;
        let healthy_sensory = SensorySnapshot::from_profile(&profile());

        let mut sick_immune = ImmuneSnapshot::from_profile(&profile());
        sick_immune.system_stress = 1.0;
        sick_immune.active_pathogen_count = 5;
        let healthy_immune = ImmuneSnapshot::from_profile(&profile());

        let suffering = snapshot.step(&cognition, &learning, &hurting_sensory, &sick_immune, 1.0);
        let healthy = snapshot.step(
            &cognition,
            &learning,
            &healthy_sensory,
            &healthy_immune,
            1.0,
        );

        assert!(suffering.effective_rationality <= healthy.effective_rationality);
        assert!(suffering.effective_intuition >= healthy.effective_intuition);
    }

    #[test]
    fn wisdom_strengthens_commitment() {
        let snapshot = DecisionSnapshot::from_profile(&profile());
        let cognition = HumanCognitionSnapshot::from_profile(&profile());
        let sensory = SensorySnapshot::from_profile(&profile());
        let immune = ImmuneSnapshot::from_profile(&profile());
        let mut wise_learning = LearningSnapshot::from_profile(&profile());
        wise_learning.wisdom_accumulation = 1.0;

        let stepped = snapshot.step(&cognition, &wise_learning, &sensory, &immune, 1.0);
        assert!(stepped.effective_commitment >= snapshot.commitment_level);
    }
}
