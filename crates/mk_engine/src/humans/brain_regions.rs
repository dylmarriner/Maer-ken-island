//! Brain Region Snapshot - activation/fatigue state for the canon's seven
//! functional brain regions.
//!
//! Surfaces `docs/canon/HumanReplicationSchema.js`'s
//! `ExtremeBrainDetailSchema.layer_1_functional_regions`
//! (`FunctionalBrainRegionsSchema`), ported in `mk_core::human::schema` but
//! previously unread by the engine. This module is Layer 1; the deeper
//! layers each have their own module with its own integration scheme —
//! [`super::population_dynamics`] (layer 2), [`super::neurochemistry`]
//! (layer 3), [`super::mesoscale_brain`], [`super::formal_predictive_processing`]
//! and [`super::attractor_control`] — and read this layer's activations.
//!
//! Each region's `activation` is driven by whichever already-modeled
//! engine system most plausibly drives that region physiologically, and
//! `fatigue` rises with sustained high activation and recovers otherwise —
//! mirroring the `attention`/`cognition` fatigue pattern already in the
//! engine.

use mk_core::human::HumanProfile;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegionState {
    pub activation: f64,
    pub fatigue: f64,
}

impl RegionState {
    fn from_trait(baseline: f64, baseline_fatigue: f64) -> Self {
        Self {
            activation: baseline,
            fatigue: baseline_fatigue,
        }
    }

    fn step(&self, drive: f64, dt: f64) -> Self {
        let activation = (self.activation + (drive - self.activation) * (0.5 * dt).clamp(0.0, 1.0))
            .clamp(0.0, 1.0);
        let fatigue_delta = if activation > 0.6 {
            activation * 0.3 * dt
        } else {
            -0.4 * dt
        };
        let fatigue = (self.fatigue + fatigue_delta).clamp(0.0, 1.0);
        Self {
            activation,
            fatigue,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrainRegionsSnapshot {
    pub prefrontal_cortex: RegionState,
    pub limbic_system: RegionState,
    pub amygdala: RegionState,
    pub hippocampus: RegionState,
    pub basal_ganglia: RegionState,
    pub hypothalamus: RegionState,
    pub brainstem: RegionState,
}

impl BrainRegionsSnapshot {
    pub fn from_profile(profile: &HumanProfile) -> Self {
        let Some(schema) = profile.canonical_schema() else {
            return Self::defaults();
        };
        let r = &schema.extreme_brain_detail.layer_1_functional_regions;

        let region = |field: &mk_core::human::schema::FunctionalRegionSchema, default: f64| {
            let activation = if field.activation_level != 0.0 {
                field.activation_level as f64
            } else {
                default
            };
            RegionState::from_trait(activation, field.fatigue as f64)
        };

        Self {
            prefrontal_cortex: region(&r.prefrontal_cortex, 0.5),
            limbic_system: region(&r.limbic_system, 0.4),
            amygdala: region(&r.amygdala, 0.2),
            hippocampus: region(&r.hippocampus, 0.4),
            basal_ganglia: region(&r.basal_ganglia, 0.4),
            hypothalamus: region(&r.hypothalamus, 0.3),
            brainstem: region(&r.brainstem, 0.6),
        }
    }

    /// Drive each region's activation from the already-modeled engine
    /// system most plausibly responsible for it, then step
    /// activation/fatigue.
    #[allow(clippy::too_many_arguments)]
    pub fn step(
        &self,
        decision: &super::decision::DecisionSnapshot,
        emotion: &super::emotion::EmotionSnapshot,
        memory: &super::memory::MemorySnapshot,
        learning: &super::learning::LearningSnapshot,
        needs: &super::needs::NeedsSnapshot,
        dt_years: f64,
    ) -> Self {
        let dt = dt_years.max(0.0);

        // Prefrontal cortex: deliberate/rational decision-making load.
        let prefrontal_drive = decision.effective_rationality;

        // Limbic system: overall emotional arousal (mean of the strongest
        // real-time emotion signals).
        let limbic_drive = (emotion.current.joy
            + emotion.current.sadness
            + emotion.current.anger
            + emotion.current.fear)
            / 4.0;

        // Amygdala: threat/fear response.
        let amygdala_drive = emotion.current.fear.max(emotion.current.anger * 0.5);

        // Hippocampus: memory encoding/consolidation load.
        let hippocampus_drive = memory.encoding_strength;

        // Basal ganglia: habit/skill execution, tracks learned plasticity
        // use via the learning system's current plasticity utilization.
        let basal_ganglia_drive = learning.current_plasticity * learning.acquisition_rate;

        // Hypothalamus: homeostatic drive regulation (hunger/thirst/fatigue).
        let hypothalamus_drive = needs.fatigue.max(1.0 - needs.glucose.clamp(0.0, 1.0));

        // Brainstem: baseline survival arousal, inversely tracks how close
        // to lethal the needs state is.
        let brainstem_drive = if needs.is_lethal() { 1.0 } else { 0.6 };

        Self {
            prefrontal_cortex: self.prefrontal_cortex.step(prefrontal_drive, dt),
            limbic_system: self.limbic_system.step(limbic_drive, dt),
            amygdala: self.amygdala.step(amygdala_drive, dt),
            hippocampus: self.hippocampus.step(hippocampus_drive, dt),
            basal_ganglia: self.basal_ganglia.step(basal_ganglia_drive, dt),
            hypothalamus: self.hypothalamus.step(hypothalamus_drive, dt),
            brainstem: self.brainstem.step(brainstem_drive, dt),
        }
    }

    /// Mean regional activation of the resting defaults, the reference the
    /// mesoscale layer scales metabolic power against.
    pub(super) fn resting_mean_activation() -> f64 {
        let d = Self::defaults();
        [
            &d.prefrontal_cortex,
            &d.limbic_system,
            &d.amygdala,
            &d.hippocampus,
            &d.basal_ganglia,
            &d.hypothalamus,
            &d.brainstem,
        ]
        .iter()
        .map(|r| r.activation)
        .sum::<f64>()
            / 7.0
    }

    pub(super) fn defaults() -> Self {
        Self {
            prefrontal_cortex: RegionState::from_trait(0.5, 0.0),
            limbic_system: RegionState::from_trait(0.4, 0.0),
            amygdala: RegionState::from_trait(0.2, 0.0),
            hippocampus: RegionState::from_trait(0.4, 0.0),
            basal_ganglia: RegionState::from_trait(0.4, 0.0),
            hypothalamus: RegionState::from_trait(0.3, 0.0),
            brainstem: RegionState::from_trait(0.6, 0.0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::humans::decision::DecisionSnapshot;
    use crate::humans::emotion::EmotionSnapshot;
    use crate::humans::learning::LearningSnapshot;
    use crate::humans::memory::MemorySnapshot;
    use crate::humans::needs::NeedsSnapshot;
    use mk_core::human::{HumanId, HumanSchema};

    fn profile() -> HumanProfile {
        let schema = HumanSchema::canonical_minimal("brain_regions_test");
        HumanProfile::from_canonical_schema(HumanId::new(1), schema)
    }

    #[test]
    fn from_profile_uses_sane_defaults() {
        let snapshot = BrainRegionsSnapshot::from_profile(&profile());
        assert!(snapshot.prefrontal_cortex.activation > 0.0);
        assert!(snapshot.brainstem.activation > 0.0);
    }

    #[test]
    fn fear_drives_amygdala_activation() {
        let snapshot = BrainRegionsSnapshot::from_profile(&profile());
        let decision = DecisionSnapshot::from_profile(&profile());
        let memory = MemorySnapshot::from_profile(&profile());
        let learning = LearningSnapshot::from_profile(&profile());
        let needs = NeedsSnapshot::from_profile(&profile());

        let mut fearful_emotion = EmotionSnapshot::from_profile(&profile());
        fearful_emotion.current.fear = 1.0;
        let calm_emotion = EmotionSnapshot::from_profile(&profile());

        let mut fearful = snapshot.clone();
        let mut calm = snapshot.clone();
        for _ in 0..5 {
            fearful = fearful.step(&decision, &fearful_emotion, &memory, &learning, &needs, 1.0);
            calm = calm.step(&decision, &calm_emotion, &memory, &learning, &needs, 1.0);
        }

        assert!(fearful.amygdala.activation >= calm.amygdala.activation);
    }

    #[test]
    fn sustained_high_activation_raises_fatigue() {
        let snapshot = BrainRegionsSnapshot::from_profile(&profile());
        let mut decision = DecisionSnapshot::from_profile(&profile());
        decision.effective_rationality = 1.0;
        let emotion = EmotionSnapshot::from_profile(&profile());
        let memory = MemorySnapshot::from_profile(&profile());
        let learning = LearningSnapshot::from_profile(&profile());
        let needs = NeedsSnapshot::from_profile(&profile());

        let mut stepped = snapshot.clone();
        for _ in 0..10 {
            stepped = stepped.step(&decision, &emotion, &memory, &learning, &needs, 1.0);
        }

        assert!(stepped.prefrontal_cortex.fatigue > 0.0);
    }
}
