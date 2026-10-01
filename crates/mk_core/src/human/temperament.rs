//! Temperament Matrix Module
//!
//! Core personality dimensions (0.0 - 1.0 scale)

use serde::{Deserialize, Serialize};

/// Core personality dimensions across multiple axes
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TemperamentMatrix {
    /// 0.0 = introverted, 1.0 = extroverted
    pub introversion_extroversion: f32,

    /// 0.0 = low emotional intensity, 1.0 = high emotional intensity
    pub emotional_intensity: f32,

    /// 0.0 = emotionally volatile, 1.0 = emotionally stable
    pub emotional_stability: f32,

    /// 0.0 = low empathy, 1.0 = high empathy
    pub empathy: f32,

    /// 0.0 = passive, 1.0 = assertive
    pub assertiveness: f32,

    /// 0.0 = resilient to environment, 1.0 = highly sensitive to environment
    pub sensitivity_to_environment: f32,

    /// 0.0 = rigid, 1.0 = highly adaptable
    pub adaptability: f32,

    /// 0.0 = careless, 1.0 = highly conscientious
    pub conscientiousness: f32,

    /// 0.0 = conventional, 1.0 = open to experience
    pub openness_to_experience: f32,
}

impl Default for TemperamentMatrix {
    fn default() -> Self {
        Self {
            introversion_extroversion: 0.5,
            emotional_intensity: 0.5,
            emotional_stability: 0.5,
            empathy: 0.5,
            assertiveness: 0.5,
            sensitivity_to_environment: 0.5,
            adaptability: 0.5,
            conscientiousness: 0.5,
            openness_to_experience: 0.5,
        }
    }
}

impl TemperamentMatrix {
    pub fn new() -> Self {
        Self::default()
    }

    /// Create with all values
    // One positional f32 per canon-schema temperament field — see
    // `human::skills::SkillMatrix::with_values` for the same tradeoff.
    #[allow(clippy::too_many_arguments)]
    pub fn with_values(
        introversion_extroversion: f32,
        emotional_intensity: f32,
        emotional_stability: f32,
        empathy: f32,
        assertiveness: f32,
        sensitivity_to_environment: f32,
        adaptability: f32,
        conscientiousness: f32,
        openness_to_experience: f32,
    ) -> Self {
        Self {
            introversion_extroversion,
            emotional_intensity,
            emotional_stability,
            empathy,
            assertiveness,
            sensitivity_to_environment,
            adaptability,
            conscientiousness,
            openness_to_experience,
        }
    }

    /// Clamp all values to 0.0 - 1.0 range
    pub fn clamp(&mut self) {
        self.introversion_extroversion = self.introversion_extroversion.clamp(0.0, 1.0);
        self.emotional_intensity = self.emotional_intensity.clamp(0.0, 1.0);
        self.emotional_stability = self.emotional_stability.clamp(0.0, 1.0);
        self.empathy = self.empathy.clamp(0.0, 1.0);
        self.assertiveness = self.assertiveness.clamp(0.0, 1.0);
        self.sensitivity_to_environment = self.sensitivity_to_environment.clamp(0.0, 1.0);
        self.adaptability = self.adaptability.clamp(0.0, 1.0);
        self.conscientiousness = self.conscientiousness.clamp(0.0, 1.0);
        self.openness_to_experience = self.openness_to_experience.clamp(0.0, 1.0);
    }

    /// Check if human is introverted (< 0.5)
    pub fn is_introverted(&self) -> bool {
        self.introversion_extroversion < 0.5
    }

    /// Check if human is extroverted (>= 0.5)
    pub fn is_extroverted(&self) -> bool {
        self.introversion_extroversion >= 0.5
    }

    /// Get archetype from temperament profile
    pub fn archetype(&self) -> &'static str {
        match (
            self.is_introverted(),
            self.emotional_intensity > 0.6,
            self.empathy > 0.6,
            self.conscientiousness > 0.6,
        ) {
            (true, true, true, _) => "Introspective Empath",
            (true, false, true, _) => "Quiet Supporter",
            (true, _, _, true) => "Focused Analyst",
            (false, true, true, _) => "Passionate Leader",
            (false, true, false, _) => "Energetic Adventurer",
            (false, _, true, _) => "Natural Connector",
            _ => "Balanced Personality",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_temperament_is_neutral() {
        let temp = TemperamentMatrix::default();
        assert_eq!(temp.introversion_extroversion, 0.5);
        assert_eq!(temp.empathy, 0.5);
    }

    #[test]
    fn temperament_values_clamp_correctly() {
        let mut temp = TemperamentMatrix {
            introversion_extroversion: 1.5,
            emotional_intensity: -0.5,
            emotional_stability: 0.5,
            empathy: 0.5,
            assertiveness: 0.5,
            sensitivity_to_environment: 0.5,
            adaptability: 0.5,
            conscientiousness: 0.5,
            openness_to_experience: 0.5,
        };

        temp.clamp();
        assert_eq!(temp.introversion_extroversion, 1.0);
        assert_eq!(temp.emotional_intensity, 0.0);
    }

    #[test]
    fn introversion_extroversion_detection() {
        let introverted = TemperamentMatrix {
            introversion_extroversion: 0.3,
            ..Default::default()
        };
        assert!(introverted.is_introverted());

        let extroverted = TemperamentMatrix {
            introversion_extroversion: 0.7,
            ..Default::default()
        };
        assert!(extroverted.is_extroverted());
    }

    #[test]
    fn archetype_assignment() {
        let introspective_empath = TemperamentMatrix {
            introversion_extroversion: 0.3,
            emotional_intensity: 0.7,
            empathy: 0.8,
            conscientiousness: 0.5,
            ..Default::default()
        };
        assert_eq!(introspective_empath.archetype(), "Introspective Empath");

        let passionate_leader = TemperamentMatrix {
            introversion_extroversion: 0.8,
            emotional_intensity: 0.7,
            empathy: 0.8,
            conscientiousness: 0.5,
            ..Default::default()
        };
        assert_eq!(passionate_leader.archetype(), "Passionate Leader");
    }
}
