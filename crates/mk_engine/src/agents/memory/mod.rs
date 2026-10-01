use mk_core::time::Tick;
use serde::{Deserialize, Serialize};

/// Single episodic memory — a past event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpisodicMemoryEntry {
    pub tick: Tick,
    pub event_type: String,
    pub context: String,
    pub salience: f64,          // 0.0-1.0, how memorable this was
    pub emotional_valence: f64, // -1.0 to 1.0
}

/// Single semantic fact
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticFact {
    pub subject: String,
    pub relation: String,
    pub object: String,
    pub confidence: f64, // 0.0-1.0
    pub last_reinforced: Tick,
}

/// Single procedural skill
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProceduralSkill {
    pub name: String,
    pub proficiency: f64, // 0.0-1.0
    pub execution_count: u64,
}

/// Complete memory system for one agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySystem {
    pub episodic: Vec<EpisodicMemoryEntry>,
    pub semantic: Vec<SemanticFact>,
    pub procedural: Vec<ProceduralSkill>,
    pub max_episodic: usize,
    pub max_semantic: usize,
}

impl Default for MemorySystem {
    fn default() -> Self {
        Self {
            episodic: Vec::new(),
            semantic: Vec::new(),
            procedural: vec![
                ProceduralSkill {
                    name: "walking".into(),
                    proficiency: 0.9,
                    execution_count: 0,
                },
                ProceduralSkill {
                    name: "resting".into(),
                    proficiency: 0.8,
                    execution_count: 0,
                },
            ],
            max_episodic: 500,
            max_semantic: 200,
        }
    }
}

impl MemorySystem {
    pub fn new() -> Self {
        Self::default()
    }

    /// Remember an episodic event
    pub fn remember_event(&mut self, entry: EpisodicMemoryEntry) {
        self.episodic.push(entry);
        if self.episodic.len() > self.max_episodic {
            // Remove least salient memory
            self.episodic
                .sort_by(|a, b| a.salience.partial_cmp(&b.salience).unwrap());
            self.episodic.remove(0);
        }
    }

    /// Learn or reinforce a semantic fact
    pub fn learn_fact(&mut self, subject: &str, relation: &str, object: &str, tick: Tick) {
        for fact in &mut self.semantic {
            if fact.subject == subject && fact.relation == relation && fact.object == object {
                fact.confidence = (fact.confidence + 0.1).min(1.0);
                fact.last_reinforced = tick;
                return;
            }
        }
        if self.semantic.len() < self.max_semantic {
            self.semantic.push(SemanticFact {
                subject: subject.to_string(),
                relation: relation.to_string(),
                object: object.to_string(),
                confidence: 0.3,
                last_reinforced: tick,
            });
        }
    }

    /// Relation name for "this biome affords <resource>".
    ///
    /// Centralised so the writer ([`MemorySystem::learn_affordance`]) and the
    /// reader ([`MemorySystem::affordance_confidence`]) cannot drift apart —
    /// a mismatch there would silently make learned facts unrecallable.
    pub const AFFORDS: &'static str = "affords";

    /// Record that `biome` was observed to afford `resource`.
    ///
    /// Called from the agent tick with the observation's own real access
    /// values, so confidence accrues only where the agent actually found the
    /// resource. Observations below `threshold` teach nothing rather than
    /// teaching a negative — the fact store holds positive affordances only.
    pub fn learn_affordance(&mut self, biome: &str, resource: &str, observed: f64, tick: Tick) {
        const THRESHOLD: f64 = 0.5;
        if observed > THRESHOLD {
            self.learn_fact(biome, Self::AFFORDS, resource, tick);
        }
    }

    /// Confidence that `biome` affords `resource`, or `0.0` if never learned.
    pub fn affordance_confidence(&self, biome: &str, resource: &str) -> f64 {
        self.semantic
            .iter()
            .find(|fact| {
                fact.subject == biome && fact.relation == Self::AFFORDS && fact.object == resource
            })
            .map(|fact| fact.confidence)
            .unwrap_or(0.0)
    }

    /// Query semantic memory for a fact
    pub fn recall_fact(&self, subject: &str, relation: &str) -> Option<&SemanticFact> {
        self.semantic
            .iter()
            .find(|f| f.subject == subject && f.relation == relation)
    }

    /// Improve procedural skill
    pub fn practice_skill(&mut self, name: &str) {
        for skill in &mut self.procedural {
            if skill.name == name {
                skill.proficiency = (skill.proficiency + 0.02).min(1.0);
                skill.execution_count += 1;
                return;
            }
        }
        self.procedural.push(ProceduralSkill {
            name: name.to_string(),
            proficiency: 0.2,
            execution_count: 1,
        });
    }

    /// Get proficiency for a skill (0.0 if unknown)
    pub fn skill_proficiency(&self, name: &str) -> f64 {
        self.procedural
            .iter()
            .find(|s| s.name == name)
            .map(|s| s.proficiency)
            .unwrap_or(0.0)
    }

    /// Decay old memories — called each tick
    pub fn decay(&mut self, tick: Tick) {
        self.semantic.retain(|f| {
            let age = tick.saturating_sub(f.last_reinforced);
            if age > 1000 {
                f.confidence > 0.3 // only retain strong facts past 1000 ticks
            } else {
                true
            }
        });
    }

    /// Count memories
    pub fn memory_count(&self) -> (usize, usize, usize) {
        (
            self.episodic.len(),
            self.semantic.len(),
            self.procedural.len(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn affordance_is_learned_only_above_the_observation_threshold() {
        let mut memory = MemorySystem::new();
        // A barely-present resource teaches nothing.
        memory.learn_affordance("Desert", "water", 0.1, 10);
        assert_eq!(memory.affordance_confidence("Desert", "water"), 0.0);

        // A genuinely observed one does.
        memory.learn_affordance("Wetland", "water", 0.9, 10);
        assert!(memory.affordance_confidence("Wetland", "water") > 0.0);
    }

    #[test]
    fn repeated_observation_reinforces_confidence_up_to_a_ceiling() {
        let mut memory = MemorySystem::new();
        memory.learn_affordance("Wetland", "water", 0.9, 1);
        let first = memory.affordance_confidence("Wetland", "water");

        memory.learn_affordance("Wetland", "water", 0.9, 2);
        let second = memory.affordance_confidence("Wetland", "water");
        assert!(second > first, "reinforcement should raise confidence");

        for tick in 3..100 {
            memory.learn_affordance("Wetland", "water", 0.9, tick);
        }
        assert!(
            memory.affordance_confidence("Wetland", "water") <= 1.0,
            "confidence must stay bounded"
        );
    }

    #[test]
    fn affordances_are_kept_per_biome_and_per_resource() {
        let mut memory = MemorySystem::new();
        memory.learn_affordance("Wetland", "water", 0.9, 1);
        memory.learn_affordance("Grassland", "food", 0.9, 1);

        assert!(memory.affordance_confidence("Wetland", "water") > 0.0);
        // Learning about one biome must not imply anything about another.
        assert_eq!(memory.affordance_confidence("Grassland", "water"), 0.0);
        assert_eq!(memory.affordance_confidence("Wetland", "food"), 0.0);
    }

    #[test]
    fn unlearned_affordance_reports_zero_not_a_default_guess() {
        let memory = MemorySystem::new();
        assert_eq!(memory.affordance_confidence("Alpine", "food"), 0.0);
    }

    #[test]
    fn learned_facts_are_recallable_through_the_generic_api() {
        // The writer and reader must agree on the relation name, or learned
        // facts would be silently unrecallable.
        let mut memory = MemorySystem::new();
        memory.learn_affordance("Wetland", "water", 0.9, 5);
        let fact = memory
            .recall_fact("Wetland", MemorySystem::AFFORDS)
            .expect("the learned fact must be recallable");
        assert_eq!(fact.object, "water");
        assert_eq!(fact.last_reinforced, 5);
    }

    #[test]
    fn semantic_store_respects_its_capacity() {
        let mut memory = MemorySystem::new();
        for i in 0..(memory.max_semantic + 50) {
            memory.learn_fact(&format!("biome_{i}"), MemorySystem::AFFORDS, "food", 1);
        }
        assert_eq!(memory.semantic.len(), memory.max_semantic);
    }
}
