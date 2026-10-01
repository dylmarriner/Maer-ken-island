//! Skill Matrix Module
//!
//! Defines competency levels across various domains (0.0 - 1.0 scale)

use serde::{Deserialize, Serialize};

/// Skill competency matrix for a human
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SkillMatrix {
    /// Ability to break down problems analytically
    pub analytical_thinking: f32,

    /// Ability to recognize and understand patterns
    pub pattern_recognition: f32,

    /// Ability to retain and recall information
    pub memory_retention: f32,

    /// Ability to generate novel solutions
    pub creative_problem_solving: f32,

    /// Ability to express ideas with words
    pub verbal_communication: f32,

    /// Ability to communicate through body language, tone, expression
    pub nonverbal_communication: f32,

    /// Physical dexterity and coordination
    pub physical_coordination: f32,

    /// Ability to understand and manage emotions in self and others
    pub emotional_intelligence: f32,

    /// Ability to navigate social situations effectively
    pub social_navigation: f32,

    /// Ability to guide and inspire others
    pub leadership: f32,
}

impl Default for SkillMatrix {
    fn default() -> Self {
        Self {
            analytical_thinking: 0.5,
            pattern_recognition: 0.5,
            memory_retention: 0.5,
            creative_problem_solving: 0.5,
            verbal_communication: 0.5,
            nonverbal_communication: 0.5,
            physical_coordination: 0.5,
            emotional_intelligence: 0.5,
            social_navigation: 0.5,
            leadership: 0.5,
        }
    }
}

impl SkillMatrix {
    pub fn new() -> Self {
        Self::default()
    }

    /// Create with specific values
    // One positional f32 per canon-schema skill field — a params struct
    // would just move the same 10 fields one layer out with no behavior
    // change, so not worth the call-site churn across the engine.
    #[allow(clippy::too_many_arguments)]
    pub fn with_values(
        analytical_thinking: f32,
        pattern_recognition: f32,
        memory_retention: f32,
        creative_problem_solving: f32,
        verbal_communication: f32,
        nonverbal_communication: f32,
        physical_coordination: f32,
        emotional_intelligence: f32,
        social_navigation: f32,
        leadership: f32,
    ) -> Self {
        Self {
            analytical_thinking,
            pattern_recognition,
            memory_retention,
            creative_problem_solving,
            verbal_communication,
            nonverbal_communication,
            physical_coordination,
            emotional_intelligence,
            social_navigation,
            leadership,
        }
    }

    /// Clamp all values to 0.0 - 1.0 range
    pub fn clamp(&mut self) {
        self.analytical_thinking = self.analytical_thinking.clamp(0.0, 1.0);
        self.pattern_recognition = self.pattern_recognition.clamp(0.0, 1.0);
        self.memory_retention = self.memory_retention.clamp(0.0, 1.0);
        self.creative_problem_solving = self.creative_problem_solving.clamp(0.0, 1.0);
        self.verbal_communication = self.verbal_communication.clamp(0.0, 1.0);
        self.nonverbal_communication = self.nonverbal_communication.clamp(0.0, 1.0);
        self.physical_coordination = self.physical_coordination.clamp(0.0, 1.0);
        self.emotional_intelligence = self.emotional_intelligence.clamp(0.0, 1.0);
        self.social_navigation = self.social_navigation.clamp(0.0, 1.0);
        self.leadership = self.leadership.clamp(0.0, 1.0);
    }

    /// Get average cognitive skill level
    pub fn cognitive_average(&self) -> f32 {
        (self.analytical_thinking
            + self.pattern_recognition
            + self.memory_retention
            + self.creative_problem_solving)
            / 4.0
    }

    /// Get average social skill level
    pub fn social_average(&self) -> f32 {
        (self.verbal_communication
            + self.nonverbal_communication
            + self.emotional_intelligence
            + self.social_navigation)
            / 4.0
    }

    /// Check if human is analytical (cognitive > social)
    pub fn is_analytical_type(&self) -> bool {
        self.cognitive_average() > self.social_average()
    }

    /// Check if human is social (social > cognitive)
    pub fn is_social_type(&self) -> bool {
        self.social_average() > self.cognitive_average()
    }

    /// Check if human is balanced
    pub fn is_balanced_type(&self) -> bool {
        (self.cognitive_average() - self.social_average()).abs() < 0.15
    }

    /// Get primary strength domain
    pub fn primary_strength(&self) -> SkillDomain {
        let skills = [
            (self.analytical_thinking, SkillDomain::Analytical),
            (self.pattern_recognition, SkillDomain::Pattern),
            (self.memory_retention, SkillDomain::Memory),
            (self.creative_problem_solving, SkillDomain::Creative),
            (self.verbal_communication, SkillDomain::Verbal),
            (self.nonverbal_communication, SkillDomain::Nonverbal),
            (self.physical_coordination, SkillDomain::Physical),
            (self.emotional_intelligence, SkillDomain::Emotional),
            (self.social_navigation, SkillDomain::Social),
            (self.leadership, SkillDomain::Leadership),
        ];

        skills
            .iter()
            .max_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(_, domain)| *domain)
            .unwrap_or(SkillDomain::Balanced)
    }

    /// Determine skill profile archetype
    pub fn archetype(&self) -> &'static str {
        match (
            self.is_analytical_type(),
            self.leadership > 0.6,
            self.emotional_intelligence > 0.6,
        ) {
            (true, true, _) => "Strategic Analyst",
            (true, false, true) => "Thoughtful Supporter",
            (true, false, false) => "Pure Analyst",
            (false, true, true) => "Natural Leader",
            (false, true, false) => "Commanding Leader",
            (false, false, true) => "Empathetic Guide",
            (false, false, false) => "Independent Operator",
        }
    }
}

/// Skill domains for identification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillDomain {
    Analytical,
    Pattern,
    Memory,
    Creative,
    Verbal,
    Nonverbal,
    Physical,
    Emotional,
    Social,
    Leadership,
    Balanced,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_skills_are_neutral() {
        let skills = SkillMatrix::default();
        assert_eq!(skills.analytical_thinking, 0.5);
    }

    #[test]
    fn cognitive_and_social_averages() {
        let skills = SkillMatrix {
            analytical_thinking: 0.8,
            pattern_recognition: 0.8,
            memory_retention: 0.7,
            creative_problem_solving: 0.7,
            verbal_communication: 0.3,
            nonverbal_communication: 0.3,
            emotional_intelligence: 0.3,
            social_navigation: 0.3,
            physical_coordination: 0.5,
            leadership: 0.5,
        };

        assert!((skills.cognitive_average() - 0.75).abs() < 0.01);
        assert!((skills.social_average() - 0.3).abs() < 0.01);
    }

    #[test]
    fn type_detection() {
        let analytical = SkillMatrix {
            analytical_thinking: 0.9,
            pattern_recognition: 0.8,
            memory_retention: 0.8,
            creative_problem_solving: 0.7,
            verbal_communication: 0.2,
            nonverbal_communication: 0.2,
            emotional_intelligence: 0.2,
            social_navigation: 0.2,
            physical_coordination: 0.5,
            leadership: 0.2,
        };
        assert!(analytical.is_analytical_type());

        let social = SkillMatrix {
            analytical_thinking: 0.2,
            pattern_recognition: 0.2,
            memory_retention: 0.2,
            creative_problem_solving: 0.2,
            verbal_communication: 0.9,
            nonverbal_communication: 0.8,
            emotional_intelligence: 0.8,
            social_navigation: 0.8,
            physical_coordination: 0.5,
            leadership: 0.8,
        };
        assert!(social.is_social_type());
    }

    #[test]
    fn archetype_assignment() {
        let strategic_analyst = SkillMatrix {
            analytical_thinking: 0.85,
            pattern_recognition: 0.8,
            memory_retention: 0.8,
            creative_problem_solving: 0.7,
            verbal_communication: 0.3,
            nonverbal_communication: 0.3,
            emotional_intelligence: 0.3,
            social_navigation: 0.3,
            physical_coordination: 0.5,
            leadership: 0.7,
        };
        assert_eq!(strategic_analyst.archetype(), "Strategic Analyst");
    }
}
