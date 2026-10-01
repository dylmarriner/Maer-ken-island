//! Technology Snapshot - Tool use, innovation, and technological aptitude

use mk_core::human::HumanProfile;
use serde::{Deserialize, Serialize};

/// Runtime technology snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TechnologySnapshot {
    /// Innovation bias - tendency to create new solutions (0.0 to 1.0)
    pub innovation_bias: f64,

    /// Tooling aptitude - ability to learn and use tools (0.0 to 1.0)
    pub tooling_aptitude: f64,

    /// Abstraction to application - converting ideas to practical use (0.0 to 1.0)
    pub abstraction_to_application: f64,

    /// Systematic thinking capacity (0.0 to 1.0)
    pub systematic_thinking: f64,

    /// Risk tolerance in experimentation (0.0 = cautious, 1.0 = reckless)
    pub risk_tolerance: f64,

    /// Learning speed for new technologies (0.0 to 1.0)
    pub learning_speed: f64,

    /// Precision vs exploration tendency (0.0 = precise, 1.0 = exploratory)
    pub exploration_tendency: f64,

    /// Collaboration in technical work (0.0 = solo, 1.0 = team-oriented)
    pub technical_collaboration: f64,

    /// Accumulated invention/knowledge progress — a persistent, monotonic
    /// counter (not a 0..1 trait like the others above) that grows every
    /// tick this human is alive, at a rate driven by their own real
    /// `learning_speed`. Gates which `resource_economy::RecipeId` recipes
    /// this human can craft/build — see
    /// `resource_economy::recipe_unlocked`. Starts at zero for every new
    /// human; nothing is pre-unlocked.
    ///
    /// `#[serde(default)]`: this field was added after persistent human
    /// profiles already existed on disk (`enable_persistent_humans`) —
    /// without a default, loading any profile saved before this field
    /// existed fails deserialization outright (`missing field
    /// accumulated_knowledge`), which is exactly what happened the first
    /// time `mk serve` ran against real saved data after this change.
    #[serde(default)]
    pub accumulated_knowledge: f64,
}

impl TechnologySnapshot {
    /// Derive technology snapshot from a HumanProfile
    pub fn from_profile(profile: &HumanProfile) -> Self {
        let temperament = &profile.temperament_matrix;
        let drives = &profile.drive_weights;
        let neuro = &profile.neurocognitive_profile;

        let openness = temperament.openness_to_experience as f64;
        let curiosity = drives.curiosity as f64;
        let conscientiousness = temperament.conscientiousness as f64;
        let extraversion = temperament.introversion_extroversion as f64;

        // Innovation driven by openness, curiosity, and associative thinking
        let innovation =
            (openness * 0.4 + curiosity * 0.3 + neuro.associative_thinking_bias as f64 * 0.3)
                .clamp(0.0, 1.0);

        // Tooling aptitude driven by conscientiousness and executive function
        let exec_capacity = (1.0 - neuro.executive_function_fatigue_rate as f64).clamp(0.0, 1.0);
        let tooling =
            (conscientiousness * 0.4 + exec_capacity * 0.4 + curiosity * 0.2).clamp(0.0, 1.0);

        // Abstraction to application: meaning drive maps directly to practical implementation
        let abstraction = drives.meaning as f64;

        Self {
            innovation_bias: innovation,
            tooling_aptitude: tooling,
            abstraction_to_application: abstraction,
            systematic_thinking: conscientiousness * 0.6
                + (1.0 - neuro.attention_regulation_variability as f64) * 0.4,
            risk_tolerance: curiosity * 0.5 + openness * 0.3 + (1.0 - conscientiousness) * 0.2,
            learning_speed: openness * 0.4 + curiosity * 0.4 + exec_capacity * 0.2,
            exploration_tendency: openness * 0.5
                + (1.0 - conscientiousness) * 0.3
                + curiosity * 0.2,
            technical_collaboration: extraversion * 0.4
                + temperament.empathy as f64 * 0.3
                + drives.bonding as f64 * 0.3,
            accumulated_knowledge: 0.0,
        }
    }

    /// Grows `accumulated_knowledge` for one tick of `dt_years`, at a rate
    /// proportional to this human's own real `learning_speed` (already
    /// derived from their openness/curiosity/executive-capacity traits, not
    /// a new invented input). The `INVENTION_RATE_SCALE` constant is the one
    /// tuned knob: at `learning_speed == 0.5` (a middling human) it takes
    /// ~10 in-sim years to reach the `IronAxe`/`IronPickaxe` unlock
    /// threshold and ~60 years to reach `StoneHouse` — a slow, generational
    /// pace of technological progress, not an instant unlock.
    pub fn step_invention(&mut self, dt_years: f64) {
        const INVENTION_RATE_SCALE: f64 = 1.0;
        self.accumulated_knowledge += self.learning_speed * dt_years * INVENTION_RATE_SCALE;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn technology_snapshot_from_profile() {
        let schema = mk_core::human::HumanSchema::canonical_minimal("test");
        let profile = mk_core::human::HumanProfile::from_canonical_schema(
            mk_core::human::HumanId::new(1),
            schema,
        );
        let tech = TechnologySnapshot::from_profile(&profile);
        assert!(tech.tooling_aptitude >= 0.0 && tech.tooling_aptitude <= 1.0);
        assert!(tech.innovation_bias >= 0.0);
        assert_eq!(tech.accumulated_knowledge, 0.0);
    }

    #[test]
    fn step_invention_grows_monotonically_with_learning_speed() {
        let schema = mk_core::human::HumanSchema::canonical_minimal("test");
        let profile = mk_core::human::HumanProfile::from_canonical_schema(
            mk_core::human::HumanId::new(1),
            schema,
        );
        let mut tech = TechnologySnapshot::from_profile(&profile);
        let before = tech.accumulated_knowledge;
        tech.step_invention(5.0);
        assert!(tech.accumulated_knowledge >= before);
        if tech.learning_speed > 0.0 {
            assert!(tech.accumulated_knowledge > before);
        }
    }
}
