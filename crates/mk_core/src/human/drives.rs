//! Drive Weights Module
//!
//! Core motivational drives that determine behavior (0.0 - 1.0 scale)

use serde::{Deserialize, Serialize};

/// Core motivational drives
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DriveWeights {
    /// Drive to maintain life and safety
    pub survival: f32,

    /// Drive to form and maintain bonds
    pub bonding: f32,

    /// Need for reassurance and comfort
    pub reassurance: Option<f32>,

    /// Drive for independence and self-direction
    pub autonomy: f32,

    /// Drive to explore and understand
    pub curiosity: f32,

    /// Drive to find purpose and significance
    pub meaning: f32,

    /// Need for emotional safety and protection
    pub emotional_safety: Option<f32>,

    /// Drive to avoid structure and constraints
    pub structure_avoidance: Option<f32>,

    /// Need for safety and predictability
    pub security: Option<f32>,

    /// Drive toward peaceful coexistence
    pub harmony: Option<f32>,

    /// Drive to minimize control and dominance
    pub control_minimization: Option<f32>,
}

impl Default for DriveWeights {
    fn default() -> Self {
        Self {
            survival: 0.7,
            bonding: 0.6,
            reassurance: None,
            autonomy: 0.5,
            curiosity: 0.5,
            meaning: 0.5,
            emotional_safety: None,
            structure_avoidance: None,
            security: None,
            harmony: None,
            control_minimization: None,
        }
    }
}

impl DriveWeights {
    pub fn new() -> Self {
        Self::default()
    }

    /// Create with basic drives
    pub fn with_basic_drives(
        survival: f32,
        bonding: f32,
        autonomy: f32,
        curiosity: f32,
        meaning: f32,
    ) -> Self {
        Self {
            survival: survival.clamp(0.0, 1.0),
            bonding: bonding.clamp(0.0, 1.0),
            autonomy: autonomy.clamp(0.0, 1.0),
            curiosity: curiosity.clamp(0.0, 1.0),
            meaning: meaning.clamp(0.0, 1.0),
            ..Default::default()
        }
    }

    /// Add reassurance drive
    pub fn with_reassurance(mut self, value: f32) -> Self {
        self.reassurance = Some(value.clamp(0.0, 1.0));
        self
    }

    /// Add emotional safety drive
    pub fn with_emotional_safety(mut self, value: f32) -> Self {
        self.emotional_safety = Some(value.clamp(0.0, 1.0));
        self
    }

    /// Add structure avoidance
    pub fn with_structure_avoidance(mut self, value: f32) -> Self {
        self.structure_avoidance = Some(value.clamp(0.0, 1.0));
        self
    }

    /// Add security drive
    pub fn with_security(mut self, value: f32) -> Self {
        self.security = Some(value.clamp(0.0, 1.0));
        self
    }

    /// Add harmony drive
    pub fn with_harmony(mut self, value: f32) -> Self {
        self.harmony = Some(value.clamp(0.0, 1.0));
        self
    }

    /// Add control minimization
    pub fn with_control_minimization(mut self, value: f32) -> Self {
        self.control_minimization = Some(value.clamp(0.0, 1.0));
        self
    }

    /// Get primary drive (highest weight)
    pub fn primary_drive(&self) -> &'static str {
        let basic_drives = [
            ("survival", self.survival),
            ("bonding", self.bonding),
            ("autonomy", self.autonomy),
            ("curiosity", self.curiosity),
            ("meaning", self.meaning),
        ];

        basic_drives
            .iter()
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(name, _)| *name)
            .unwrap_or("balanced")
    }

    /// Check if drive-driven profile (high drive values)
    pub fn is_drive_heavy(&self) -> bool {
        self.survival > 0.7 || self.bonding > 0.7 || self.curiosity > 0.7 || self.meaning > 0.7
    }

    /// Check for conflict between autonomy and bonding
    pub fn has_autonomy_bonding_conflict(&self) -> bool {
        (self.autonomy - self.bonding).abs() > 0.4
    }

    /// Get drive summary profile
    pub fn profile_type(&self) -> &'static str {
        match (
            self.survival > 0.6,
            self.bonding > 0.6,
            self.autonomy > 0.6,
            self.curiosity > 0.6,
            self.meaning > 0.6,
        ) {
            (true, true, _, _, _) => "Security-Bonded",
            (true, _, true, _, _) => "Survival-Autonomous",
            (_, true, true, true, _) => "Explorer-Connected",
            (_, _, true, true, true) => "Autonomous-Seeker",
            (_, true, _, _, true) => "Bonded-Meaning",
            (_, _, _, true, true) => "Curious-Meaningful",
            _ => "Balanced",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_drives() {
        let drives = DriveWeights::default();
        assert_eq!(drives.survival, 0.7);
        assert_eq!(drives.bonding, 0.6);
        assert!(drives.reassurance.is_none());
    }

    #[test]
    fn primary_drive_detection() {
        let drives = DriveWeights::with_basic_drives(0.9, 0.3, 0.4, 0.5, 0.5);
        assert_eq!(drives.primary_drive(), "survival");
    }

    #[test]
    fn optional_drives() {
        let drives = DriveWeights::new()
            .with_reassurance(0.8)
            .with_emotional_safety(0.7);

        assert!(drives.reassurance.is_some());
        assert_eq!(drives.reassurance, Some(0.8));
        assert_eq!(drives.emotional_safety, Some(0.7));
    }

    #[test]
    fn autonomy_bonding_conflict() {
        let conflict = DriveWeights::with_basic_drives(0.5, 0.9, 0.2, 0.5, 0.5);
        assert!(conflict.has_autonomy_bonding_conflict());

        let harmonious = DriveWeights::with_basic_drives(0.5, 0.6, 0.65, 0.5, 0.5);
        assert!(!harmonious.has_autonomy_bonding_conflict());
    }

    #[test]
    fn profile_type_detection() {
        let explorer = DriveWeights::with_basic_drives(0.5, 0.7, 0.8, 0.8, 0.4);
        assert_eq!(explorer.profile_type(), "Explorer-Connected");
    }
}
