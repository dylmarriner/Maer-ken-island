//! Relational Defaults Module
//!
//! Default relationship behaviors and patterns

use serde::{Deserialize, Serialize};

/// How bonding occurs in relationships
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BondingType {
    Emotional,
    Direction,
}

/// Complete relational defaults profile
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct RelationalDefaults {
    /// Bond through emotional connection vs shared direction
    pub bond_through_emotion_vs_direction: Option<BondingType>,

    /// Likelihood of initiating passion/intensity in relationships
    pub passion_initiation_probability: Option<f32>,

    /// Tendency to reassert autonomy after intimacy
    pub autonomy_reassertion_after_intimacy: Option<f32>,

    /// Risk of confusing partner about intentions
    pub partner_confusion_risk: Option<f32>,

    /// Tendency to mask dependency as caregiving
    pub dependency_masked_as_care_bias: Option<f32>,

    /// Depth of bonding preferred (0 = surface, 1 = complete merger)
    pub preferred_bond_depth: Option<f32>,

    /// Prioritize reliability/consistency over affection
    pub reliability_over_affection_bias: Option<f32>,

    /// Risk of stating harsh truths without gentleness
    pub truth_without_tenderness_risk: Option<f32>,

    /// Probability of taking stabilizer role in relationships
    pub stabilizer_role_probability: Option<f32>,

    /// Rate at which resentment accumulates
    pub resentment_accumulation_rate: Option<f32>,
}

impl RelationalDefaults {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set bonding type preference
    pub fn with_bonding_type(mut self, bonding_type: BondingType) -> Self {
        self.bond_through_emotion_vs_direction = Some(bonding_type);
        self
    }

    /// Add passion initiation probability
    pub fn with_passion_probability(mut self, probability: f32) -> Self {
        self.passion_initiation_probability = Some(probability.clamp(0.0, 1.0));
        self
    }

    /// Add autonomy reassertion trait
    pub fn with_autonomy_reassertion(mut self, value: f32) -> Self {
        self.autonomy_reassertion_after_intimacy = Some(value.clamp(0.0, 1.0));
        self
    }

    /// Add preferred bond depth
    pub fn with_bond_depth(mut self, depth: f32) -> Self {
        self.preferred_bond_depth = Some(depth.clamp(0.0, 1.0));
        self
    }

    /// Add reliability bias
    pub fn with_reliability_bias(mut self, bias: f32) -> Self {
        self.reliability_over_affection_bias = Some(bias.clamp(0.0, 1.0));
        self
    }

    /// Add resentment accumulation
    pub fn with_resentment_rate(mut self, rate: f32) -> Self {
        self.resentment_accumulation_rate = Some(rate.clamp(0.0, 1.0));
        self
    }

    /// Check if emotionally bonding type
    pub fn bonds_emotionally(&self) -> bool {
        matches!(
            self.bond_through_emotion_vs_direction,
            Some(BondingType::Emotional)
        )
    }

    /// Check if direction/goal bonding type
    pub fn bonds_through_direction(&self) -> bool {
        matches!(
            self.bond_through_emotion_vs_direction,
            Some(BondingType::Direction)
        )
    }

    /// Check for high passion initiation
    pub fn initiates_passion(&self) -> bool {
        self.passion_initiation_probability
            .map(|p| p > 0.6)
            .unwrap_or(false)
    }

    /// Check for autonomy-seeking behavior
    pub fn reasserts_autonomy(&self) -> bool {
        self.autonomy_reassertion_after_intimacy
            .map(|a| a > 0.5)
            .unwrap_or(false)
    }

    /// Check for partner confusion risk
    pub fn confuses_partners(&self) -> bool {
        self.partner_confusion_risk
            .map(|r| r > 0.5)
            .unwrap_or(false)
    }

    /// Check for stabilizer tendency
    pub fn is_stabilizer(&self) -> bool {
        self.stabilizer_role_probability
            .map(|p| p > 0.6)
            .unwrap_or(false)
    }

    /// Get relational style profile
    pub fn relational_style(&self) -> &'static str {
        match (
            self.bonds_emotionally(),
            self.is_stabilizer(),
            self.initiates_passion(),
        ) {
            (true, true, _) => "Emotionally Steady",
            (true, false, true) => "Emotionally Passionate",
            (true, false, false) => "Emotionally Reserved",
            (_, true, _) => "Supportive Stabilizer",
            (_, _, true) => "Intensity-Seeking",
            _ => "Flexible",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bonding_type_detection() {
        let emotional = RelationalDefaults::new().with_bonding_type(BondingType::Emotional);
        assert!(emotional.bonds_emotionally());
        assert!(!emotional.bonds_through_direction());
    }

    #[test]
    fn relational_style_assessment() {
        let steady = RelationalDefaults::new()
            .with_bonding_type(BondingType::Emotional)
            .with_passion_probability(0.3);

        assert_eq!(steady.relational_style(), "Emotionally Reserved");
    }
}
