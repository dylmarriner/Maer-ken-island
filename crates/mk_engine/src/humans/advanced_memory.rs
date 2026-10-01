//! Advanced Memory Systems Snapshot - Layer 4 of the canon's memory model.
//!
//! Surfaces `docs/canon/HumanReplicationSchema.js`'s
//! `AdvancedMemorySystemsSchema`. This is the canon's more granular successor
//! to `LegacyMemorySystemsSchema` (already implemented in [`super::memory`]):
//! it separates working/short-term/long-term-semantic/episodic/procedural
//! memory into distinct sub-stores and adds cross-cutting integration
//! metrics (consolidation activity, sleep-dependent consolidation,
//! emotional-memory enhancement, forgetting-curve rate, false-memory
//! susceptibility).
//!
//! Following the precedent set by `population_dynamics.rs` and
//! `mesoscale_brain.rs`: the canon's content-bearing sub-lists
//! (`working_memory.current_items`, `short_term_memory.items`,
//! `long_term_semantic.concepts`, `procedural_memory.skills`,
//! `procedural_memory.habit_strengths`) are still **not fabricated** — no
//! synthetic memory content exists for those in this engine, so those
//! containers stay empty. `episodic_memory.episodes` and
//! `.autobiographical_timeline` are the one exception: real recorded data
//! already exists in [`super::autonomy::AutonomousMind::experiences`] (each
//! tick's actually-taken action, its actually-received reward, and the
//! actually-observed environment access at that moment), it was just never
//! projected into this canon-shaped slot. `step()` now surfaces the most
//! recent experiences as `episodes`, and the subset with reward magnitude
//! above [`AUTOBIOGRAPHICAL_REWARD_THRESHOLD`] (life events salient enough
//! to remember distinctly) as `autobiographical_timeline`. The scalar
//! aggregates that summarize sub-store *state* are computed the same way as
//! before, each coupled to the real drivers already stepped this tick: base
//! `MemorySnapshot` traits, `AttentionSnapshot` (encoding pressure), and
//! `EmotionSnapshot` (emotional-memory enhancement, since real memory
//! research ties affect to consolidation strength).

use std::collections::{HashMap, VecDeque};

use mk_core::human::HumanProfile;
use serde::{Deserialize, Serialize};

use super::autonomy::AutonomousExperience;

/// Bound on how many recent [`AutonomousExperience`]s project into
/// `episodic_memory.episodes` each tick — real capacity is not unbounded
/// human recall, and re-serializing all 2,000 retained experiences every
/// tick would be wasted work for a value nothing reads beyond the most
/// recent slice.
const MAX_PROJECTED_EPISODES: usize = 50;

/// Reward magnitude above which an experience counts as salient enough to
/// surface in `episodic_memory.autobiographical_timeline` rather than just
/// the general `episodes` list — mirrors how humans remember exceptional
/// (very good or very bad) events more distinctly than routine ones.
const AUTOBIOGRAPHICAL_REWARD_THRESHOLD: f64 = 0.5;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkingMemoryLayer {
    pub capacity: f64,
    pub current_items: Vec<serde_json::Value>,
    pub rehearsal_active: bool,
    pub interference_level: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShortTermMemoryLayer {
    pub items: Vec<serde_json::Value>,
    pub consolidation_threshold: f64,
    pub max_duration: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LongTermSemanticLayer {
    pub concepts: HashMap<String, serde_json::Value>,
    pub semantic_network_density: f64,
    pub knowledge_integration_level: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpisodicMemoryLayer {
    pub episodes: Vec<serde_json::Value>,
    pub autobiographical_timeline: Vec<serde_json::Value>,
    pub memory_distortion_level: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProceduralMemoryLayer {
    pub skills: HashMap<String, serde_json::Value>,
    pub habit_strengths: HashMap<String, f64>,
    pub interference_vulnerability: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryIntegration {
    pub consolidation_active: bool,
    pub sleep_dependent_consolidation: f64,
    pub emotional_memory_enhancement: f64,
    pub forgetting_curve_rate: f64,
    pub false_memory_susceptibility: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvancedMemorySnapshot {
    pub working_memory: WorkingMemoryLayer,
    pub short_term_memory: ShortTermMemoryLayer,
    pub long_term_semantic: LongTermSemanticLayer,
    pub episodic_memory: EpisodicMemoryLayer,
    pub procedural_memory: ProceduralMemoryLayer,
    pub memory_integration: MemoryIntegration,
}

impl AdvancedMemorySnapshot {
    pub fn from_profile(profile: &HumanProfile) -> Self {
        let base = super::memory::MemorySnapshot::from_profile(profile);
        Self::from_base(&base)
    }

    fn from_base(base: &super::memory::MemorySnapshot) -> Self {
        Self {
            working_memory: WorkingMemoryLayer {
                capacity: base.working_memory_capacity,
                current_items: Vec::new(),
                rehearsal_active: false,
                interference_level: base.working_memory_interference,
            },
            short_term_memory: ShortTermMemoryLayer {
                items: Vec::new(),
                consolidation_threshold: base.episodic_consolidation_threshold,
                max_duration: base.working_memory_duration,
            },
            long_term_semantic: LongTermSemanticLayer {
                concepts: HashMap::new(),
                semantic_network_density: base.semantic_association_strength,
                knowledge_integration_level: base.semantic_knowledge_integration,
            },
            episodic_memory: EpisodicMemoryLayer {
                episodes: Vec::new(),
                autobiographical_timeline: Vec::new(),
                memory_distortion_level: (1.0 - base.retrieval_strength).clamp(0.0, 1.0),
            },
            procedural_memory: ProceduralMemoryLayer {
                skills: HashMap::new(),
                habit_strengths: HashMap::new(),
                interference_vulnerability: base.working_memory_interference,
            },
            memory_integration: MemoryIntegration {
                consolidation_active: base.consolidation_level
                    > base.episodic_consolidation_threshold,
                sleep_dependent_consolidation: base.consolidation_level,
                emotional_memory_enhancement: 0.0,
                forgetting_curve_rate: base.semantic_forgetting_curve,
                false_memory_susceptibility: (base.working_memory_interference * 0.5)
                    .clamp(0.0, 1.0),
            },
        }
    }

    /// Recompute the Layer-4 aggregates from the already-stepped base
    /// `MemorySnapshot`, `AttentionSnapshot`, and `EmotionSnapshot` — no
    /// independent state of its own, mirroring `MesoscaleBrainSnapshot`'s
    /// pure-aggregation design.
    pub fn step(
        &self,
        base: &super::memory::MemorySnapshot,
        attention: &super::attention::AttentionSnapshot,
        emotion: &super::emotion::EmotionSnapshot,
        experiences: &VecDeque<AutonomousExperience>,
    ) -> Self {
        let mut next = Self::from_base(base);

        // Rehearsal is active while attentional resources are still
        // available and working memory is under load.
        next.working_memory.rehearsal_active =
            attention.available > 0.3 && base.encoding_strength > 0.4;

        // Real memory research: emotionally salient material consolidates
        // more readily. Use the emotion snapshot's overall intensity as the
        // enhancement driver.
        let emotional_intensity = emotion.overall_intensity();
        next.memory_integration.emotional_memory_enhancement = emotional_intensity.clamp(0.0, 1.0);
        next.episodic_memory.memory_distortion_level =
            (next.episodic_memory.memory_distortion_level + emotional_intensity * 0.2)
                .clamp(0.0, 1.0);

        // Project real recorded experiences into the canon-shaped episodic
        // slots — see module docs. `serde_json::to_value` on a plain struct
        // of primitives cannot fail, but skip a failure gracefully rather
        // than panicking on this non-critical projection.
        let recent = experiences.iter().rev().take(MAX_PROJECTED_EPISODES);
        for experience in recent {
            let Ok(value) = serde_json::to_value(experience) else {
                continue;
            };
            if experience.reward.abs() >= AUTOBIOGRAPHICAL_REWARD_THRESHOLD {
                next.episodic_memory
                    .autobiographical_timeline
                    .push(value.clone());
            }
            next.episodic_memory.episodes.push(value);
        }

        next
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::humans::attention::AttentionSnapshot;
    use crate::humans::emotion::EmotionSnapshot;
    use crate::humans::memory::MemorySnapshot;
    use mk_core::human::{HumanId, HumanSchema};

    fn profile() -> HumanProfile {
        let schema = HumanSchema::canonical_minimal("advanced_memory_test");
        HumanProfile::from_canonical_schema(HumanId::new(1), schema)
    }

    #[test]
    fn from_profile_uses_sane_defaults() {
        let snapshot = AdvancedMemorySnapshot::from_profile(&profile());
        assert!(snapshot.working_memory.capacity > 0.0);
        assert!(snapshot.working_memory.current_items.is_empty());
        assert!(snapshot.episodic_memory.episodes.is_empty());
        assert!(snapshot.procedural_memory.skills.is_empty());
    }

    #[test]
    fn consolidation_active_tracks_threshold() {
        let base = MemorySnapshot::from_profile(&profile());
        let attention = AttentionSnapshot::from_profile(&profile());
        let emotion = EmotionSnapshot::from_profile(&profile());

        let mut stepped_base = base.clone();
        for _ in 0..20 {
            stepped_base = stepped_base.step(&attention, 0.0, 1.0);
        }

        let snapshot = AdvancedMemorySnapshot::from_profile(&profile()).step(
            &stepped_base,
            &attention,
            &emotion,
            &VecDeque::new(),
        );

        assert_eq!(
            snapshot.memory_integration.consolidation_active,
            stepped_base.consolidation_level > stepped_base.episodic_consolidation_threshold
        );
    }

    #[test]
    fn no_fabricated_memory_content_with_no_real_experiences() {
        let base = MemorySnapshot::from_profile(&profile());
        let attention = AttentionSnapshot::from_profile(&profile());
        let emotion = EmotionSnapshot::from_profile(&profile());

        let snapshot = AdvancedMemorySnapshot::from_profile(&profile()).step(
            &base,
            &attention,
            &emotion,
            &VecDeque::new(),
        );

        assert!(snapshot.short_term_memory.items.is_empty());
        assert!(snapshot.long_term_semantic.concepts.is_empty());
        assert!(snapshot.episodic_memory.episodes.is_empty());
        assert!(snapshot
            .episodic_memory
            .autobiographical_timeline
            .is_empty());
        assert!(snapshot.procedural_memory.habit_strengths.is_empty());
    }

    #[test]
    fn episodic_memory_projects_real_autonomous_experiences() {
        use crate::agents::behavior::ActionKind;

        let base = MemorySnapshot::from_profile(&profile());
        let attention = AttentionSnapshot::from_profile(&profile());
        let emotion = EmotionSnapshot::from_profile(&profile());
        let mut experiences = VecDeque::new();
        experiences.push_back(AutonomousExperience {
            tick: 1,
            action: ActionKind::Gather,
            reward: 0.9,
            fitness_after: 0.5,
            action_success: true,
            caloric_access: 0.5,
            hydration_access: 0.5,
            shelter_quality: 0.5,
            hazard_index: 0.1,
            resource_abundance: 0.5,
        });
        experiences.push_back(AutonomousExperience {
            tick: 2,
            action: ActionKind::Idle,
            reward: 0.05,
            fitness_after: 0.51,
            action_success: true,
            caloric_access: 0.5,
            hydration_access: 0.5,
            shelter_quality: 0.5,
            hazard_index: 0.1,
            resource_abundance: 0.5,
        });

        let snapshot = AdvancedMemorySnapshot::from_profile(&profile()).step(
            &base,
            &attention,
            &emotion,
            &experiences,
        );

        // Both real experiences project into `episodes`; only the
        // high-reward one clears the autobiographical-salience threshold.
        assert_eq!(snapshot.episodic_memory.episodes.len(), 2);
        assert_eq!(snapshot.episodic_memory.autobiographical_timeline.len(), 1);
    }
}
