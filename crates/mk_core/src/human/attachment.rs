//! Attachment Style Module
//!
//! Relationship attachment patterns and bonding characteristics

use serde::{Deserialize, Serialize};

/// Attachment style classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AttachmentPatternType {
    AnxiousPreoccupied,
    AnxiousAvoidantHybrid,
    Secure,
    DismissiveAvoidant,
    FearfulAvoidant,
}

/// Complete attachment style profile
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AttachmentStyle {
    pub primary_pattern: AttachmentPatternType,

    /// How much seeking proximity (0.0 - 1.0)
    pub proximity_seeking_intensity: Option<f32>,

    /// Reactivity to perceived abandonment
    pub abandonment_reactivity: Option<f32>,

    /// Tendency to merge with partner emotionally
    pub emotional_fusion_threshold: Option<f32>,

    /// Time to repair after conflict
    pub repair_after_conflict_latency: Option<f32>,

    /// How much monitoring of partner's availability
    pub closeness_monitoring_intensity: Option<f32>,

    /// Jealousy threshold
    pub jealousy_threshold: Option<f32>,

    /// Sensitivity to abandonment cues
    pub abandonment_sensitivity: Option<f32>,
}

impl Default for AttachmentStyle {
    fn default() -> Self {
        Self {
            primary_pattern: AttachmentPatternType::Secure,
            proximity_seeking_intensity: None,
            abandonment_reactivity: None,
            emotional_fusion_threshold: None,
            repair_after_conflict_latency: None,
            closeness_monitoring_intensity: None,
            jealousy_threshold: None,
            abandonment_sensitivity: None,
        }
    }
}

impl AttachmentStyle {
    pub fn new(pattern: AttachmentPatternType) -> Self {
        Self {
            primary_pattern: pattern,
            ..Default::default()
        }
    }

    /// Check if secure attachment
    pub fn is_secure(&self) -> bool {
        self.primary_pattern == AttachmentPatternType::Secure
    }

    /// Check if anxious attachment
    pub fn is_anxious(&self) -> bool {
        matches!(
            self.primary_pattern,
            AttachmentPatternType::AnxiousPreoccupied
                | AttachmentPatternType::AnxiousAvoidantHybrid
        )
    }

    /// Check if avoidant attachment
    pub fn is_avoidant(&self) -> bool {
        matches!(
            self.primary_pattern,
            AttachmentPatternType::DismissiveAvoidant
                | AttachmentPatternType::FearfulAvoidant
                | AttachmentPatternType::AnxiousAvoidantHybrid
        )
    }

    /// Check if fearful/disorganized attachment
    pub fn is_disorganized(&self) -> bool {
        self.primary_pattern == AttachmentPatternType::FearfulAvoidant
    }

    /// Get security level
    pub fn security_level(&self) -> &'static str {
        match self.primary_pattern {
            AttachmentPatternType::Secure => "Secure",
            AttachmentPatternType::AnxiousPreoccupied => "Anxious",
            AttachmentPatternType::AnxiousAvoidantHybrid => "Ambivalent",
            AttachmentPatternType::DismissiveAvoidant => "Avoidant",
            AttachmentPatternType::FearfulAvoidant => "Disorganized",
        }
    }

    /// Get relationship risk profile
    pub fn relationship_risk(&self) -> &'static str {
        match self.primary_pattern {
            AttachmentPatternType::Secure => "Low risk",
            AttachmentPatternType::AnxiousPreoccupied => "Clingy/dependent",
            AttachmentPatternType::AnxiousAvoidantHybrid => "Ambivalent/inconsistent",
            AttachmentPatternType::DismissiveAvoidant => "Emotionally distant",
            AttachmentPatternType::FearfulAvoidant => "Conflicted/volatile",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attachment_type_detection() {
        let secure = AttachmentStyle::new(AttachmentPatternType::Secure);
        assert!(secure.is_secure());
        assert!(!secure.is_anxious());
        assert!(!secure.is_avoidant());

        let anxious = AttachmentStyle::new(AttachmentPatternType::AnxiousPreoccupied);
        assert!(anxious.is_anxious());
        assert!(!anxious.is_secure());
    }

    #[test]
    fn attachment_security_levels() {
        let disorganized = AttachmentStyle::new(AttachmentPatternType::FearfulAvoidant);
        assert!(disorganized.is_disorganized());
        assert_eq!(disorganized.security_level(), "Disorganized");
    }
}
