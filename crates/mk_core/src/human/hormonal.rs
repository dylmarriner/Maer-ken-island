//! Hormonal Baseline Bias Module
//!
//! Neurochemical baseline tendencies (0.0 - 1.0 scale)

use serde::{Deserialize, Serialize};

/// Hormonal and neurochemical baseline biases
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HormonalBaselineBias {
    /// Sensitivity to oxytocin (bonding hormone)
    pub oxytocin_reactivity: Option<f32>,

    /// Baseline oxytocin levels
    pub oxytocin_bias: Option<f32>,

    /// Variability in dopamine production
    pub dopamine_variability: f32,

    /// Instability in serotonin regulation
    pub serotonin_instability: Option<f32>,

    /// Baseline serotonin levels
    pub serotonin_baseline: Option<f32>,

    /// Sensitivity to cortisol (stress hormone)
    pub cortisol_sensitivity: f32,

    /// Tendency toward adrenaline shutdown (freeze response)
    pub adrenaline_shutdown_bias: Option<f32>,

    /// Reactivity to adrenaline
    pub adrenaline_reactivity: Option<f32>,

    /// Irregularity in melatonin production
    pub melatonin_irregularity: f32,
}

impl Default for HormonalBaselineBias {
    fn default() -> Self {
        Self {
            oxytocin_reactivity: None,
            oxytocin_bias: None,
            dopamine_variability: 0.5,
            serotonin_instability: None,
            serotonin_baseline: None,
            cortisol_sensitivity: 0.5,
            adrenaline_shutdown_bias: None,
            adrenaline_reactivity: None,
            melatonin_irregularity: 0.5,
        }
    }
}

impl HormonalBaselineBias {
    pub fn new() -> Self {
        Self::default()
    }

    /// Create with core hormonal values
    pub fn with_core_hormones(
        dopamine_variability: f32,
        cortisol_sensitivity: f32,
        melatonin_irregularity: f32,
    ) -> Self {
        Self {
            dopamine_variability: dopamine_variability.clamp(0.0, 1.0),
            cortisol_sensitivity: cortisol_sensitivity.clamp(0.0, 1.0),
            melatonin_irregularity: melatonin_irregularity.clamp(0.0, 1.0),
            ..Default::default()
        }
    }

    /// Add oxytocin profile
    pub fn with_oxytocin(mut self, reactivity: f32, bias: f32) -> Self {
        self.oxytocin_reactivity = Some(reactivity.clamp(0.0, 1.0));
        self.oxytocin_bias = Some(bias.clamp(0.0, 1.0));
        self
    }

    /// Add serotonin profile
    pub fn with_serotonin(mut self, instability: f32, baseline: f32) -> Self {
        self.serotonin_instability = Some(instability.clamp(0.0, 1.0));
        self.serotonin_baseline = Some(baseline.clamp(0.0, 1.0));
        self
    }

    /// Add adrenaline profile
    pub fn with_adrenaline(mut self, shutdown_bias: f32, reactivity: f32) -> Self {
        self.adrenaline_shutdown_bias = Some(shutdown_bias.clamp(0.0, 1.0));
        self.adrenaline_reactivity = Some(reactivity.clamp(0.0, 1.0));
        self
    }

    /// Check if human has high dopamine variability (ADHD indicator)
    pub fn is_dopamine_variable(&self) -> bool {
        self.dopamine_variability > 0.6
    }

    /// Check if human has high cortisol sensitivity (anxiety indicator)
    pub fn is_stress_sensitive(&self) -> bool {
        self.cortisol_sensitivity > 0.6
    }

    /// Check if human has sleep irregularity
    pub fn has_sleep_issues(&self) -> bool {
        self.melatonin_irregularity > 0.6
    }

    /// Check if human has adrenaline shutdown tendency (freeze response)
    pub fn has_freeze_response(&self) -> bool {
        self.adrenaline_shutdown_bias
            .map(|v| v > 0.5)
            .unwrap_or(false)
    }

    /// Check if human has low serotonin (depression indicator)
    pub fn has_low_serotonin(&self) -> bool {
        self.serotonin_baseline.map(|v| v < 0.4).unwrap_or(false)
    }

    /// Check if human is bonding-oriented (high oxytocin)
    pub fn is_bonding_oriented(&self) -> bool {
        self.oxytocin_reactivity.map(|v| v > 0.6).unwrap_or(false)
    }

    /// Get neurochemical profile type
    pub fn profile_type(&self) -> &'static str {
        let dopamine_variable = self.dopamine_variability > 0.6;
        let cortisol_sensitive = self.cortisol_sensitivity > 0.6;
        let sleep_issues = self.melatonin_irregularity > 0.6;

        match (dopamine_variable, cortisol_sensitive, sleep_issues) {
            (true, true, true) => "Highly Reactive",
            (true, true, false) => "Anxious-Variable",
            (true, false, true) => "Variable-Dysrhythmic",
            (true, false, false) => "Dopamine-Variable",
            (false, true, true) => "Stress-Dysrhythmic",
            (false, true, false) => "Stress-Sensitive",
            (false, false, true) => "Sleep-Dysrhythmic",
            (false, false, false) => "Neurochemically-Stable",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_hormonal_bias() {
        let bias = HormonalBaselineBias::default();
        assert_eq!(bias.dopamine_variability, 0.5);
        assert_eq!(bias.cortisol_sensitivity, 0.5);
        assert!(bias.oxytocin_reactivity.is_none());
    }

    #[test]
    fn hormonal_trait_detection() {
        let reactive = HormonalBaselineBias::with_core_hormones(0.8, 0.8, 0.8);
        assert!(reactive.is_dopamine_variable());
        assert!(reactive.is_stress_sensitive());
        assert!(reactive.has_sleep_issues());
    }

    #[test]
    fn oxytocin_bonding_detection() {
        let bonding = HormonalBaselineBias::new().with_oxytocin(0.8, 0.7);
        assert!(bonding.is_bonding_oriented());
    }

    #[test]
    fn profile_type_detection() {
        let highly_reactive = HormonalBaselineBias::with_core_hormones(0.75, 0.8, 0.8);
        assert_eq!(highly_reactive.profile_type(), "Highly Reactive");

        let stable = HormonalBaselineBias::with_core_hormones(0.3, 0.3, 0.3);
        assert_eq!(stable.profile_type(), "Neurochemically-Stable");
    }
}
