//! Emotional Baseline Module
//!
//! Defines default emotional state and patterns

use serde::{Deserialize, Serialize};

/// Recovery speed after emotional events
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecoverySpeed {
    Fast,
    Moderate,
    Slow,
}

/// Primary defense mechanism for stress
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DefenseMechanism(pub String);

/// Baseline emotional state and patterns
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EmotionalBaseline {
    /// Description of resting emotional tone
    pub baseline_affect: String,

    /// Resting emotional valence (-1.0 to 1.0)
    /// Negative = sad/withdrawn baseline
    /// Positive = happy/engaged baseline
    /// 0.0 = neutral
    pub resting_emotional_tone: f32,

    /// How readily emotions are triggered (0.0 - 1.0)
    pub emotional_reactivity: f32,

    /// How quickly emotional state returns to baseline
    pub recovery_speed: RecoverySpeed,

    /// How many different emotions person can experience (0.0 - 1.0)
    pub emotional_range_breadth: f32,

    /// Intensity variation of emotions (0.0 - 1.0)
    pub emotional_range_intensity: f32,

    /// Primary way person copes with stress
    pub primary_defense_mechanism: DefenseMechanism,
}

impl Default for EmotionalBaseline {
    fn default() -> Self {
        Self {
            baseline_affect: "neutral".to_string(),
            resting_emotional_tone: 0.0,
            emotional_reactivity: 0.5,
            recovery_speed: RecoverySpeed::Moderate,
            emotional_range_breadth: 0.5,
            emotional_range_intensity: 0.5,
            primary_defense_mechanism: DefenseMechanism("rationalization".to_string()),
        }
    }
}

impl EmotionalBaseline {
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if baseline is positive/happy
    pub fn is_positive_baseline(&self) -> bool {
        self.resting_emotional_tone > 0.1
    }

    /// Check if baseline is negative/sad
    pub fn is_negative_baseline(&self) -> bool {
        self.resting_emotional_tone < -0.1
    }

    /// Check if baseline is neutral
    pub fn is_neutral_baseline(&self) -> bool {
        (self.resting_emotional_tone).abs() <= 0.1
    }

    /// Check if emotionally reactive
    pub fn is_emotionally_reactive(&self) -> bool {
        self.emotional_reactivity > 0.6
    }

    /// Check if emotionally stable
    pub fn is_emotionally_stable(&self) -> bool {
        self.emotional_reactivity < 0.4
    }

    /// Check if emotionally expressive (wide range)
    pub fn has_wide_emotional_range(&self) -> bool {
        self.emotional_range_breadth > 0.6
    }

    /// Check if emotionally intense
    pub fn is_emotionally_intense(&self) -> bool {
        self.emotional_range_intensity > 0.6
    }

    /// Classify emotional recovery capability
    pub fn recovery_capability(&self) -> &'static str {
        match self.recovery_speed {
            RecoverySpeed::Fast => "Quick recovery",
            RecoverySpeed::Moderate => "Balanced recovery",
            RecoverySpeed::Slow => "Extended processing required",
        }
    }

    /// Get overall emotional profile summary
    pub fn profile_summary(&self) -> String {
        let tone = if self.is_positive_baseline() {
            "positive"
        } else if self.is_negative_baseline() {
            "negative"
        } else {
            "neutral"
        };

        let reactivity = if self.is_emotionally_reactive() {
            "reactive"
        } else {
            "stable"
        };

        let range = if self.has_wide_emotional_range() {
            "wide"
        } else {
            "narrow"
        };

        format!(
            "{} baseline, {} temperament, {} emotional range",
            tone, reactivity, range
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_baseline_is_neutral() {
        let baseline = EmotionalBaseline::default();
        assert!(baseline.is_neutral_baseline());
    }

    #[test]
    fn baseline_tone_detection() {
        let positive = EmotionalBaseline {
            resting_emotional_tone: 0.5,
            ..Default::default()
        };
        assert!(positive.is_positive_baseline());

        let negative = EmotionalBaseline {
            resting_emotional_tone: -0.5,
            ..Default::default()
        };
        assert!(negative.is_negative_baseline());
    }

    #[test]
    fn reactivity_detection() {
        let reactive = EmotionalBaseline {
            emotional_reactivity: 0.75,
            ..Default::default()
        };
        assert!(reactive.is_emotionally_reactive());

        let stable = EmotionalBaseline {
            emotional_reactivity: 0.3,
            ..Default::default()
        };
        assert!(stable.is_emotionally_stable());
    }

    #[test]
    fn emotional_range_detection() {
        let wide_range = EmotionalBaseline {
            emotional_range_breadth: 0.8,
            ..Default::default()
        };
        assert!(wide_range.has_wide_emotional_range());

        let intense = EmotionalBaseline {
            emotional_range_intensity: 0.9,
            ..Default::default()
        };
        assert!(intense.is_emotionally_intense());
    }

    #[test]
    fn profile_summary_generation() {
        let profile = EmotionalBaseline {
            resting_emotional_tone: 0.4,
            emotional_reactivity: 0.7,
            emotional_range_breadth: 0.8,
            ..Default::default()
        };
        let summary = profile.profile_summary();
        assert!(summary.contains("positive"));
        assert!(summary.contains("reactive"));
        assert!(summary.contains("wide"));
    }
}
