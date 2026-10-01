//! Memory Snapshot - episodic/semantic/working memory traits and state.
//!
//! Surfaces `docs/canon/HumanReplicationSchema.js`'s `LegacyMemorySystemsSchema`
//! (the shape actually bound to `HumanSchema.memory_systems` in the canon
//! aggregator; ported 1:1 to `mk_core::human::schema::LegacyMemorySystemsSchema`),
//! previously unread by the engine. Coupled to `attention`/`needs`:
//! encoding strength depends on available attention, and consolidation
//! backslides under fatigue, mirroring real memory-formation effects.

use mk_core::human::HumanProfile;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySnapshot {
    // --- episodic memory (traits) ---
    pub episodic_capacity: f64,
    pub episodic_decay_rate: f64,
    pub episodic_consolidation_threshold: f64,

    // --- semantic memory (traits) ---
    pub semantic_association_strength: f64,
    pub semantic_knowledge_integration: f64,
    pub semantic_forgetting_curve: f64,

    // --- working memory (traits) ---
    pub working_memory_capacity: f64,
    pub working_memory_duration: f64,
    pub working_memory_interference: f64,

    // --- state (stepped) ---
    /// How strongly new experience is being encoded right now (0-1).
    pub encoding_strength: f64,
    /// How much consolidated long-term memory has accumulated (0-1).
    pub consolidation_level: f64,
    /// Current retrieval ease/confidence (0-1).
    pub retrieval_strength: f64,
}

impl MemorySnapshot {
    pub fn from_profile(profile: &HumanProfile) -> Self {
        let Some(schema) = profile.canonical_schema() else {
            return Self::defaults();
        };
        let m = &schema.memory_systems;

        Self {
            episodic_capacity: nonzero_or(m.episodic_memory.capacity, 0.6),
            episodic_decay_rate: nonzero_or(m.episodic_memory.trace_decay_rate, 0.2),
            episodic_consolidation_threshold: nonzero_or(
                m.episodic_memory.consolidation_threshold,
                0.5,
            ),

            semantic_association_strength: nonzero_or(m.semantic_memory.association_strength, 0.5),
            semantic_knowledge_integration: nonzero_or(
                m.semantic_memory.knowledge_integration,
                0.5,
            ),
            semantic_forgetting_curve: nonzero_or(m.semantic_memory.forgetting_curve, 0.3),

            working_memory_capacity: nonzero_or(m.working_memory.capacity, 0.5),
            working_memory_duration: nonzero_or(m.working_memory.duration, 0.5),
            working_memory_interference: m.working_memory.interference_susceptibility.unwrap_or(0.3)
                as f64,

            encoding_strength: 0.5,
            consolidation_level: 0.0,
            retrieval_strength: nonzero_or(m.episodic_memory.retrieval_strength, 0.5),
        }
    }

    /// Couple memory formation to actual attentional resources and fatigue
    /// instead of a static schema readout.
    pub fn step(
        &self,
        attention: &super::attention::AttentionSnapshot,
        melatonin: f64,
        dt_years: f64,
    ) -> Self {
        let dt = dt_years.max(0.0);

        // Encoding requires attentional resources; fatigue impairs it.
        let encoding_strength = (self.encoding_strength
            + (attention.available - self.encoding_strength) * (0.5 * dt).clamp(0.0, 1.0))
        .clamp(0.0, 1.0);

        // Consolidation accrues from sustained encoding, decayed by the
        // episodic decay rate over elapsed time. Real memory research:
        // consolidation into long-term storage is sleep-dependent — high
        // melatonin (the body's real sleep-onset signal, from
        // `neurochemistry.rs`) boosts the gain on top of active encoding,
        // rather than requiring wakeful attention to be the only path to
        // consolidation.
        let sleep_consolidation_bonus = 1.0 + melatonin.clamp(0.0, 1.0) * 0.5;
        let consolidation_gain =
            encoding_strength * self.episodic_capacity * dt * sleep_consolidation_bonus;
        let consolidation_decay = self.consolidation_level * self.episodic_decay_rate * dt;
        let consolidation_level =
            (self.consolidation_level + consolidation_gain - consolidation_decay).clamp(0.0, 1.0);

        // Retrieval strength tracks consolidation, degraded by working-memory
        // interference susceptibility.
        let retrieval_strength =
            (consolidation_level * (1.0 - self.working_memory_interference * 0.3)).clamp(0.0, 1.0);

        Self {
            encoding_strength,
            consolidation_level,
            retrieval_strength,
            ..self.clone()
        }
    }

    fn defaults() -> Self {
        Self {
            episodic_capacity: 0.6,
            episodic_decay_rate: 0.2,
            episodic_consolidation_threshold: 0.5,
            semantic_association_strength: 0.5,
            semantic_knowledge_integration: 0.5,
            semantic_forgetting_curve: 0.3,
            working_memory_capacity: 0.5,
            working_memory_duration: 0.5,
            working_memory_interference: 0.3,
            encoding_strength: 0.5,
            consolidation_level: 0.0,
            retrieval_strength: 0.5,
        }
    }
}

fn nonzero_or(value: Option<f32>, default: f64) -> f64 {
    match value {
        Some(v) if v != 0.0 => v as f64,
        _ => default,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::humans::attention::AttentionSnapshot;
    use mk_core::human::{HumanId, HumanSchema};

    fn profile() -> HumanProfile {
        let schema = HumanSchema::canonical_minimal("memory_test");
        HumanProfile::from_canonical_schema(HumanId::new(1), schema)
    }

    #[test]
    fn from_profile_uses_sane_defaults() {
        let snapshot = MemorySnapshot::from_profile(&profile());
        assert!(snapshot.episodic_capacity > 0.0);
        assert!(snapshot.working_memory_capacity > 0.0);
    }

    #[test]
    fn consolidation_accumulates_with_sustained_attention() {
        let snapshot = MemorySnapshot::from_profile(&profile());
        let attention = AttentionSnapshot::from_profile(&profile());

        let mut stepped = snapshot.clone();
        for _ in 0..10 {
            stepped = stepped.step(&attention, 0.0, 1.0);
        }

        assert!(stepped.consolidation_level > 0.0);
        assert!(stepped.consolidation_level <= 1.0);
    }

    #[test]
    fn low_attention_slows_encoding() {
        let snapshot = MemorySnapshot::from_profile(&profile());
        let mut depleted_attention = AttentionSnapshot::from_profile(&profile());
        depleted_attention.available = 0.0;

        let full_attention = AttentionSnapshot::from_profile(&profile());

        let stepped_low = snapshot.step(&depleted_attention, 0.0, 1.0);
        let stepped_high = snapshot.step(&full_attention, 0.0, 1.0);

        assert!(stepped_low.encoding_strength <= stepped_high.encoding_strength);
    }

    #[test]
    fn high_melatonin_boosts_consolidation_beyond_wakeful_encoding_alone() {
        let snapshot = MemorySnapshot::from_profile(&profile());
        let attention = AttentionSnapshot::from_profile(&profile());

        let awake = snapshot.step(&attention, 0.0, 1.0);
        let asleep = snapshot.step(&attention, 0.9, 1.0);

        assert!(asleep.consolidation_level > awake.consolidation_level);
    }
}
