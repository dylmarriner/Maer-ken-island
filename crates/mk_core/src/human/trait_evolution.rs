//! Trait Evolution Module
//!
//! Tracks how personality traits and characteristics change over time
//! based on experiences, trauma, learning, and growth

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Type of event that modifies traits
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModificationEventType {
    Trauma,
    Growth,
    Learning,
    Relationship,
    Achievement,
    Loss,
    Revelation,
    Healing,
    Custom(String),
}

/// Single trait modification event
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TraitModification {
    /// Type of event causing change
    pub event_type: ModificationEventType,

    /// Description of the event
    pub event_description: String,

    /// Trait name being modified
    pub trait_name: String,

    /// Change amount (-1.0 to 1.0, relative to current value)
    pub delta: f32,

    /// When this modification occurred (age in years)
    pub age_at_event: f32,

    /// Whether change is permanent or temporary
    pub is_permanent: bool,

    /// Duration of effect (in years, if temporary)
    pub duration_years: Option<f32>,

    /// Recovery trajectory (how fast effect fades if temporary)
    pub recovery_rate: Option<f32>,
}

impl TraitModification {
    pub fn new(
        event_type: ModificationEventType,
        event_description: String,
        trait_name: String,
        delta: f32,
        age_at_event: f32,
    ) -> Self {
        Self {
            event_type,
            event_description,
            trait_name,
            delta: delta.clamp(-1.0, 1.0),
            age_at_event,
            is_permanent: true,
            duration_years: None,
            recovery_rate: None,
        }
    }

    /// Create temporary modification
    pub fn temporary(
        event_type: ModificationEventType,
        event_description: String,
        trait_name: String,
        delta: f32,
        age_at_event: f32,
        duration_years: f32,
        recovery_rate: f32,
    ) -> Self {
        Self {
            event_type,
            event_description,
            trait_name,
            delta: delta.clamp(-1.0, 1.0),
            age_at_event,
            is_permanent: false,
            duration_years: Some(duration_years),
            recovery_rate: Some(recovery_rate),
        }
    }

    /// Calculate remaining effect at given age
    pub fn effect_at_age(&self, current_age: f32) -> f32 {
        if self.is_permanent {
            return self.delta;
        }

        let years_elapsed = current_age - self.age_at_event;
        if years_elapsed < 0.0 {
            return 0.0;
        }

        let duration = self.duration_years.unwrap_or(1.0);
        if years_elapsed > duration {
            return 0.0;
        }

        let recovery_rate = self.recovery_rate.unwrap_or(1.0);
        let remaining_fraction = 1.0 - (years_elapsed / duration) * recovery_rate;
        self.delta * remaining_fraction.max(0.0)
    }
}

/// Trauma profile tracking lasting effects
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TraumaProfile {
    /// Trauma events experienced
    pub traumatic_events: Vec<TraitModification>,

    /// Overall trauma severity (0.0 - 1.0)
    pub severity: f32,

    /// Triggers for trauma responses
    pub triggers: Vec<String>,

    /// Healing progress (0.0 - 1.0)
    pub healing_progress: f32,

    /// PTSD-like response intensity
    pub hypervigilance_level: f32,

    /// Dissociation tendency from trauma
    pub dissociation_tendency: f32,
}

impl Default for TraumaProfile {
    fn default() -> Self {
        Self {
            traumatic_events: Vec::new(),
            severity: 0.0,
            triggers: Vec::new(),
            healing_progress: 0.0,
            hypervigilance_level: 0.0,
            dissociation_tendency: 0.0,
        }
    }
}

impl TraumaProfile {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add traumatic event
    pub fn add_event(&mut self, event: TraitModification) {
        self.traumatic_events.push(event);
        self.recalculate_severity();
    }

    /// Add trigger for trauma response
    pub fn add_trigger(&mut self, trigger: String) {
        if !self.triggers.contains(&trigger) {
            self.triggers.push(trigger);
        }
    }

    /// Recalculate overall severity
    fn recalculate_severity(&mut self) {
        if self.traumatic_events.is_empty() {
            self.severity = 0.0;
            return;
        }

        let avg_delta: f32 = self
            .traumatic_events
            .iter()
            .map(|e| e.delta.abs())
            .sum::<f32>()
            / self.traumatic_events.len() as f32;

        self.severity = avg_delta.clamp(0.0, 1.0);
    }

    /// Advance healing
    pub fn progress_healing(&mut self, amount: f32) {
        self.healing_progress = (self.healing_progress + amount).clamp(0.0, 1.0);
        self.severity *= 1.0 - amount * 0.1; // Severity decreases with healing
    }

    /// Check if triggered
    pub fn is_triggered_by(&self, stimulus: &str) -> bool {
        self.triggers.iter().any(|t| t.contains(stimulus))
    }
}

/// Growth and learning profile
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GrowthProfile {
    /// Learning experiences
    pub learning_events: Vec<TraitModification>,

    /// Growth events
    pub growth_events: Vec<TraitModification>,

    /// Cumulative growth (0.0 - 1.0)
    pub total_growth: f32,

    /// Learned skills and knowledge
    pub learned_skills: HashMap<String, f32>,

    /// Belief system evolution
    pub belief_changes: Vec<(f32, String)>, // (age, belief_description)
}

impl Default for GrowthProfile {
    fn default() -> Self {
        Self {
            learning_events: Vec::new(),
            growth_events: Vec::new(),
            total_growth: 0.0,
            learned_skills: HashMap::new(),
            belief_changes: Vec::new(),
        }
    }
}

impl GrowthProfile {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add learning event
    pub fn add_learning(&mut self, event: TraitModification) {
        self.learning_events.push(event);
        self.recalculate_growth();
    }

    /// Add growth event
    pub fn add_growth(&mut self, event: TraitModification) {
        self.growth_events.push(event);
        self.recalculate_growth();
    }

    /// Add learned skill
    pub fn add_skill(&mut self, skill_name: String, level: f32) {
        self.learned_skills
            .insert(skill_name, level.clamp(0.0, 1.0));
        self.recalculate_growth();
    }

    /// Record belief change
    pub fn record_belief_change(&mut self, age: f32, belief: String) {
        self.belief_changes.push((age, belief));
    }

    /// Recalculate total growth
    fn recalculate_growth(&mut self) {
        let learning_growth: f32 = self
            .learning_events
            .iter()
            .map(|e| e.delta.max(0.0))
            .sum::<f32>()
            / (self.learning_events.len() as f32).max(1.0);

        let growth_growth: f32 = self
            .growth_events
            .iter()
            .map(|e| e.delta.max(0.0))
            .sum::<f32>()
            / (self.growth_events.len() as f32).max(1.0);

        let skill_growth: f32 = if self.learned_skills.is_empty() {
            0.0
        } else {
            // Sum in a stable key order rather than raw HashMap::values()
            // iteration order (randomized per-process): float addition is
            // non-associative, so summing the same values in a different
            // order can produce a different bit pattern across otherwise-
            // identical runs.
            let mut keys: Vec<_> = self.learned_skills.keys().collect();
            keys.sort();
            let sum: f32 = keys.iter().map(|k| self.learned_skills[*k]).sum();
            sum / self.learned_skills.len() as f32
        };

        self.total_growth =
            ((learning_growth + growth_growth + skill_growth) / 3.0).clamp(0.0, 1.0);
    }

    /// Get beliefs at specific age
    pub fn beliefs_at_age(&self, age: f32) -> Vec<String> {
        self.belief_changes
            .iter()
            .filter(|(event_age, _)| *event_age <= age)
            .map(|(_, belief)| belief.clone())
            .collect()
    }
}

/// Complete trait evolution system
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TraitEvolution {
    /// Birth traits (immutable baseline)
    pub birth_traits: HashMap<String, f32>,

    /// Current trait values
    pub current_traits: HashMap<String, f32>,

    /// Trauma history
    pub trauma: TraumaProfile,

    /// Growth and learning history
    pub growth: GrowthProfile,

    /// All modifications chronologically
    pub modification_history: Vec<TraitModification>,

    /// Current age
    pub current_age: f32,
}

impl Default for TraitEvolution {
    fn default() -> Self {
        Self {
            birth_traits: HashMap::new(),
            current_traits: HashMap::new(),
            trauma: TraumaProfile::new(),
            growth: GrowthProfile::new(),
            modification_history: Vec::new(),
            current_age: 0.0,
        }
    }
}

impl TraitEvolution {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set birth traits
    pub fn with_birth_traits(mut self, traits: HashMap<String, f32>) -> Self {
        self.birth_traits = traits.clone();
        self.current_traits = traits;
        self
    }

    /// Apply modification
    pub fn apply_modification(&mut self, modification: TraitModification) {
        let trait_name = modification.trait_name.clone();

        // Get base value
        let base = self.birth_traits.get(&trait_name).copied().unwrap_or(0.5);

        // Apply modification
        let effect = modification.effect_at_age(self.current_age);
        let new_value = (base + effect).clamp(0.0, 1.0);

        self.current_traits.insert(trait_name, new_value);
        self.modification_history.push(modification);
    }

    /// Advance age and recalculate traits
    pub fn advance_age(&mut self, years: f32) {
        self.current_age += years;

        // Recalculate all trait values based on current age
        for (trait_name, base_value) in self.birth_traits.iter() {
            let total_effect: f32 = self
                .modification_history
                .iter()
                .filter(|m| m.trait_name == *trait_name)
                .map(|m| m.effect_at_age(self.current_age))
                .sum();

            let current_value = (base_value + total_effect).clamp(0.0, 1.0);
            self.current_traits
                .insert(trait_name.clone(), current_value);
        }
    }

    /// Get trait change from birth to now
    pub fn trait_delta(&self, trait_name: &str) -> f32 {
        let birth = self.birth_traits.get(trait_name).copied().unwrap_or(0.5);
        let current = self.current_traits.get(trait_name).copied().unwrap_or(0.5);
        current - birth
    }

    /// Check if human has grown overall
    pub fn has_grown(&self) -> bool {
        self.growth.total_growth > 0.1
    }

    /// Check if human is traumatized
    pub fn is_traumatized(&self) -> bool {
        self.trauma.severity > 0.3
    }

    /// Get personality change report
    pub fn change_report(&self) -> String {
        let growth_desc = if self.growth.total_growth > 0.5 {
            "significant growth"
        } else if self.growth.total_growth > 0.2 {
            "modest growth"
        } else {
            "minimal growth"
        };

        let trauma_desc = if self.trauma.severity > 0.7 {
            "severe trauma"
        } else if self.trauma.severity > 0.4 {
            "moderate trauma"
        } else if self.trauma.severity > 0.0 {
            "minor trauma"
        } else {
            "no trauma"
        };

        format!(
            "Age {:.1}: {} experiences, {}, healing at {:.0}%",
            self.current_age,
            growth_desc,
            trauma_desc,
            self.trauma.healing_progress * 100.0
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trait_modification_permanent() {
        let modification = TraitModification::new(
            ModificationEventType::Growth,
            "Learned discipline".to_string(),
            "conscientiousness".to_string(),
            0.2,
            25.0,
        );

        assert_eq!(modification.effect_at_age(30.0), 0.2);
        assert_eq!(modification.effect_at_age(50.0), 0.2);
    }

    #[test]
    fn trait_modification_temporary() {
        let modification = TraitModification::temporary(
            ModificationEventType::Trauma,
            "Brief injury".to_string(),
            "confidence".to_string(),
            -0.3,
            25.0,
            2.0,
            1.0,
        );

        assert!((modification.effect_at_age(25.5) - (-0.3 * 0.75)).abs() < 0.01);
        assert!(modification.effect_at_age(27.5) < 0.01);
    }

    #[test]
    fn trauma_profile_severity() {
        let mut trauma = TraumaProfile::new();
        trauma.add_event(TraitModification::new(
            ModificationEventType::Trauma,
            "Loss".to_string(),
            "openness".to_string(),
            -0.4,
            20.0,
        ));

        assert!(trauma.severity > 0.0);
    }

    #[test]
    fn growth_profile_accumulation() {
        let mut growth = GrowthProfile::new();
        growth.add_learning(TraitModification::new(
            ModificationEventType::Learning,
            "Learned skill".to_string(),
            "competence".to_string(),
            0.3,
            25.0,
        ));

        assert!(growth.total_growth > 0.0);
    }

    #[test]
    fn trait_evolution_aging() {
        let mut traits = HashMap::new();
        traits.insert("openness".to_string(), 0.5);

        let mut evolution = TraitEvolution::new().with_birth_traits(traits);

        evolution.apply_modification(TraitModification::new(
            ModificationEventType::Growth,
            "Explored".to_string(),
            "openness".to_string(),
            0.2,
            20.0,
        ));

        evolution.advance_age(10.0);
        assert_eq!(evolution.current_age, 10.0);
    }
}
