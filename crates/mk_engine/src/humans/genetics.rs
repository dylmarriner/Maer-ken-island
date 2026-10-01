//! Genetics Snapshot - Heritable trait summary for engine simulation
//!
//! Distilled from the full Genome in mk_core for efficient runtime use.

use mk_core::human::HumanProfile;
use serde::{Deserialize, Serialize};

/// Runtime genetics snapshot derived from the canonical genome
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneticsSnapshot {
    /// Stability of genetic expression (0.0 = highly mutable, 1.0 = locked)
    pub heritable_stability: f64,

    /// Combined openness from both DNA strands
    pub openness: f64,

    /// Combined extraversion
    pub extraversion: f64,

    /// Neuroplasticity factor
    pub plasticity: f64,

    /// Baseline dopamine availability
    pub dopamine_base: f64,

    /// Baseline serotonin availability
    pub serotonin_base: f64,

    /// Norepinephrine baseline
    pub norepinephrine_base: f64,

    /// Cortisol sensitivity
    pub cortisol_sensitivity: f64,

    /// Novelty-seeking drive
    pub novelty_seek: f64,

    /// Rumination tendency
    pub rumination: f64,

    /// Executive control baseline
    pub exec_control: f64,

    /// Threat detection bias
    pub threat_bias: f64,

    /// Memory consolidation gain
    pub episodic_gain: f64,

    /// Memory decay rate
    pub mem_decay: f64,

    /// Trauma persistence factor
    pub trauma_sticky: f64,

    /// Attachment drive strength
    pub attachment_drive: f64,

    /// Trust formation rate
    pub trust_gain: f64,

    /// Trust decay rate
    pub trust_decay: f64,

    /// Jealousy reactivity
    pub jealousy: f64,

    /// Fatigue sensitivity
    pub fatigue_sensitivity: f64,

    /// Pain sensitivity
    pub pain_sensitivity: f64,

    /// Sex chromosome pair (true = XY/male, false = XX/female)
    pub is_male: bool,

    /// Expression weights (A vs B strand dominance)
    pub expression_weight_a: f64,
    pub expression_weight_b: f64,
}

impl GeneticsSnapshot {
    /// Derive a genetics snapshot from a full HumanProfile
    pub fn from_profile(profile: &HumanProfile) -> Self {
        let genome = &profile.genome;
        let maternal = &genome.maternal_dna;
        let paternal = &genome.paternal_dna;
        let weights = &genome.expression_weights;

        // Weighted average of both strands based on expression weights
        let w_m = weights.maternal as f64;
        let w_p = weights.paternal as f64;

        let blend = |a: f32, b: f32| -> f64 { a as f64 * w_m + b as f64 * w_p };

        // Heritable stability is derived from expression control and genetic variance
        let variance = ((maternal.openness - paternal.openness).abs() as f64
            + (maternal.extraversion - paternal.extraversion).abs() as f64
            + (maternal.dopamine_base - paternal.dopamine_base).abs() as f64)
            / 3.0;
        let heritable_stability = (1.0 - variance).clamp(0.0, 1.0);

        Self {
            heritable_stability,
            openness: blend(maternal.openness, paternal.openness),
            extraversion: blend(maternal.extraversion, paternal.extraversion),
            plasticity: blend(
                maternal.plasticity.unwrap_or(0.5),
                paternal.plasticity.unwrap_or(0.5),
            ),
            dopamine_base: blend(maternal.dopamine_base, paternal.dopamine_base),
            serotonin_base: blend(maternal.serotonin_base, paternal.serotonin_base),
            norepinephrine_base: blend(maternal.norepinephrine_base, paternal.norepinephrine_base),
            cortisol_sensitivity: blend(
                maternal.cortisol_sensitivity,
                paternal.cortisol_sensitivity,
            ),
            novelty_seek: blend(maternal.novelty_seek, paternal.novelty_seek),
            rumination: blend(maternal.rumination, paternal.rumination),
            exec_control: blend(maternal.exec_control, paternal.exec_control),
            threat_bias: blend(maternal.threat_bias, paternal.threat_bias),
            episodic_gain: blend(maternal.episodic_gain, paternal.episodic_gain),
            mem_decay: blend(maternal.mem_decay, paternal.mem_decay),
            trauma_sticky: blend(maternal.trauma_sticky, paternal.trauma_sticky),
            attachment_drive: blend(maternal.attachment, paternal.attachment),
            trust_gain: blend(maternal.trust_gain, paternal.trust_gain),
            trust_decay: blend(maternal.trust_decay, paternal.trust_decay),
            jealousy: blend(maternal.jealousy, paternal.jealousy),
            fatigue_sensitivity: blend(maternal.fatigue_sensitivity, paternal.fatigue_sensitivity),
            pain_sensitivity: blend(maternal.pain_sensitivity, paternal.pain_sensitivity),
            is_male: genome.sex_chromosomes.is_male(),
            expression_weight_a: w_m,
            expression_weight_b: w_p,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn genetics_snapshot_has_heritable_stability() {
        let profile = mk_core::human::HumanSchema::canonical_minimal("test");
        let profile = mk_core::human::HumanProfile::from_canonical_schema(
            mk_core::human::HumanId::new(1),
            profile,
        );
        let snapshot = GeneticsSnapshot::from_profile(&profile);
        assert!(snapshot.heritable_stability >= 0.0 && snapshot.heritable_stability <= 1.0);
    }
}
