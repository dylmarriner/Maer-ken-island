//! Personality Traits Module
//!
//! Organized personality trait definitions by domain
//! Each trait has baseline value, polarity, behavioral expressions, and growth range

use serde::{Deserialize, Serialize};

/// Polarity of a personality trait
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TraitPolarity {
    High,
    Low,
    Reactive,
    Hybrid,
    PrecisionInInterest,
    PurposeBiased,
    PressureBiased,
    Variable,
}

/// Single personality trait definition
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PersonalityTrait {
    pub trait_name: String,
    pub polarity: TraitPolarity,
    pub baseline_value: f32,
    pub behavioral_expression: String,
    pub stress_expression: String,
    pub withdrawal_expression: String,
    pub growth_drift_range: (f32, f32),
}

impl PersonalityTrait {
    pub fn new(
        trait_name: String,
        polarity: TraitPolarity,
        baseline_value: f32,
        behavioral_expression: String,
        stress_expression: String,
        withdrawal_expression: String,
        growth_drift_range: (f32, f32),
    ) -> Self {
        Self {
            trait_name,
            polarity,
            baseline_value: baseline_value.clamp(0.0, 1.0),
            behavioral_expression,
            stress_expression,
            withdrawal_expression,
            growth_drift_range,
        }
    }

    /// Get current expression based on state
    pub fn get_expression(&self, state: TraitState) -> &str {
        match state {
            TraitState::Normal => &self.behavioral_expression,
            TraitState::Stressed => &self.stress_expression,
            TraitState::Withdrawn => &self.withdrawal_expression,
        }
    }
}

/// Trait expression state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraitState {
    Normal,
    Stressed,
    Withdrawn,
}

/// Emotional domain traits
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct EmotionalTraits {
    pub traits: Vec<PersonalityTrait>,
}

impl EmotionalTraits {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_trait(&mut self, trait_def: PersonalityTrait) {
        self.traits.push(trait_def);
    }

    pub fn find_trait(&self, name: &str) -> Option<&PersonalityTrait> {
        self.traits.iter().find(|t| t.trait_name == name)
    }
}

/// Social attachment domain traits
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct SocialAttachmentTraits {
    pub traits: Vec<PersonalityTrait>,
}

impl SocialAttachmentTraits {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_trait(&mut self, trait_def: PersonalityTrait) {
        self.traits.push(trait_def);
    }

    pub fn find_trait(&self, name: &str) -> Option<&PersonalityTrait> {
        self.traits.iter().find(|t| t.trait_name == name)
    }
}

/// Cognitive domain traits
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct CognitiveTraits {
    pub traits: Vec<PersonalityTrait>,
}

impl CognitiveTraits {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_trait(&mut self, trait_def: PersonalityTrait) {
        self.traits.push(trait_def);
    }

    pub fn find_trait(&self, name: &str) -> Option<&PersonalityTrait> {
        self.traits.iter().find(|t| t.trait_name == name)
    }
}

/// Motivational domain traits
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct MotivationalTraits {
    pub traits: Vec<PersonalityTrait>,
}

impl MotivationalTraits {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_trait(&mut self, trait_def: PersonalityTrait) {
        self.traits.push(trait_def);
    }

    pub fn find_trait(&self, name: &str) -> Option<&PersonalityTrait> {
        self.traits.iter().find(|t| t.trait_name == name)
    }
}

/// Control/Agency domain traits (gem-d uses this)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ControlAgencyTraits {
    pub traits: Vec<PersonalityTrait>,
}

impl ControlAgencyTraits {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_trait(&mut self, trait_def: PersonalityTrait) {
        self.traits.push(trait_def);
    }

    pub fn find_trait(&self, name: &str) -> Option<&PersonalityTrait> {
        self.traits.iter().find(|t| t.trait_name == name)
    }
}

/// Control/Power domain traits (gem-k uses this)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ControlPowerTraits {
    pub traits: Vec<PersonalityTrait>,
}

impl ControlPowerTraits {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_trait(&mut self, trait_def: PersonalityTrait) {
        self.traits.push(trait_def);
    }

    pub fn find_trait(&self, name: &str) -> Option<&PersonalityTrait> {
        self.traits.iter().find(|t| t.trait_name == name)
    }
}

/// Complete personality traits organized by domain
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PersonalityTraits {
    pub emotional: EmotionalTraits,
    pub social_attachment: SocialAttachmentTraits,
    pub cognitive: CognitiveTraits,
    pub motivational: MotivationalTraits,
    pub control_agency: Option<ControlAgencyTraits>,
    pub control_power: Option<ControlPowerTraits>,
}

impl Default for PersonalityTraits {
    fn default() -> Self {
        Self {
            emotional: EmotionalTraits::new(),
            social_attachment: SocialAttachmentTraits::new(),
            cognitive: CognitiveTraits::new(),
            motivational: MotivationalTraits::new(),
            control_agency: None,
            control_power: None,
        }
    }
}

impl PersonalityTraits {
    pub fn new() -> Self {
        Self::default()
    }

    /// Enable agency control (gem-d type)
    pub fn with_control_agency(mut self) -> Self {
        self.control_agency = Some(ControlAgencyTraits::new());
        self
    }

    /// Enable power control (gem-k type)
    pub fn with_control_power(mut self) -> Self {
        self.control_power = Some(ControlPowerTraits::new());
        self
    }

    /// Count total traits across all domains
    pub fn trait_count(&self) -> usize {
        let mut count = self.emotional.traits.len()
            + self.social_attachment.traits.len()
            + self.cognitive.traits.len()
            + self.motivational.traits.len();

        if let Some(ref agency) = self.control_agency {
            count += agency.traits.len();
        }
        if let Some(ref power) = self.control_power {
            count += power.traits.len();
        }

        count
    }

    /// Get average baseline value across domain
    pub fn emotional_average(&self) -> f32 {
        if self.emotional.traits.is_empty() {
            return 0.5;
        }
        self.emotional
            .traits
            .iter()
            .map(|t| t.baseline_value)
            .sum::<f32>()
            / self.emotional.traits.len() as f32
    }

    pub fn social_average(&self) -> f32 {
        if self.social_attachment.traits.is_empty() {
            return 0.5;
        }
        self.social_attachment
            .traits
            .iter()
            .map(|t| t.baseline_value)
            .sum::<f32>()
            / self.social_attachment.traits.len() as f32
    }

    pub fn cognitive_average(&self) -> f32 {
        if self.cognitive.traits.is_empty() {
            return 0.5;
        }
        self.cognitive
            .traits
            .iter()
            .map(|t| t.baseline_value)
            .sum::<f32>()
            / self.cognitive.traits.len() as f32
    }

    pub fn motivational_average(&self) -> f32 {
        if self.motivational.traits.is_empty() {
            return 0.5;
        }
        self.motivational
            .traits
            .iter()
            .map(|t| t.baseline_value)
            .sum::<f32>()
            / self.motivational.traits.len() as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_personality_trait() {
        let trait_def = PersonalityTrait::new(
            "Openness".to_string(),
            TraitPolarity::High,
            0.8,
            "Explores new ideas".to_string(),
            "Seeks escape through novelty".to_string(),
            "Becomes rigid".to_string(),
            (0.7, 0.9),
        );

        assert_eq!(trait_def.trait_name, "Openness");
        assert_eq!(trait_def.baseline_value, 0.8);
        assert_eq!(
            trait_def.get_expression(TraitState::Normal),
            "Explores new ideas"
        );
    }

    #[test]
    fn personality_traits_domain_tracking() {
        let mut traits = PersonalityTraits::new();
        assert_eq!(traits.trait_count(), 0);

        traits.emotional.add_trait(PersonalityTrait::new(
            "Test".to_string(),
            TraitPolarity::High,
            0.5,
            "test".to_string(),
            "test".to_string(),
            "test".to_string(),
            (0.0, 1.0),
        ));

        assert_eq!(traits.trait_count(), 1);
    }

    #[test]
    fn control_agency_and_power() {
        let traits = PersonalityTraits::new()
            .with_control_agency()
            .with_control_power();

        assert!(traits.control_agency.is_some());
        assert!(traits.control_power.is_some());
    }

    #[test]
    fn domain_averages() {
        let mut traits = PersonalityTraits::new();

        traits.emotional.add_trait(PersonalityTrait::new(
            "Trait1".to_string(),
            TraitPolarity::High,
            0.8,
            "".to_string(),
            "".to_string(),
            "".to_string(),
            (0.0, 1.0),
        ));

        traits.emotional.add_trait(PersonalityTrait::new(
            "Trait2".to_string(),
            TraitPolarity::Low,
            0.2,
            "".to_string(),
            "".to_string(),
            "".to_string(),
            (0.0, 1.0),
        ));

        assert!((traits.emotional_average() - 0.5).abs() < 0.01);
    }
}
