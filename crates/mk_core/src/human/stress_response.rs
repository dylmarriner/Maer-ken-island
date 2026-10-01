//! Stress Response Profile Module
//!
//! How the human responds to stress, threats, and overwhelm

use serde::{Deserialize, Serialize};

/// Stress/overwhelm response type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EmotionalResponseType {
    FloodThenShutdown,
    WithdrawalFreeze,
}

/// Fight/flight/freeze bias
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FreezeVsFlightBias {
    WithdrawalFreeze,
}

/// Complete stress response profile
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StressResponseProfile {
    /// How easily threat is detected (lower = more sensitive)
    pub threat_detection_threshold: f32,

    /// Whether stress causes emotional flood or shutdown
    pub emotional_flood_vs_shutdown_bias: Option<EmotionalResponseType>,

    /// Whether stress causes freeze or flight
    pub freeze_vs_flight_bias: Option<FreezeVsFlightBias>,

    /// How easily withdrawal occurs
    pub withdrawal_activation_threshold: f32,

    /// Tendency to become confused under precision/performance pressure
    pub confusion_under_precision_pressure: Option<f32>,

    /// How fast stress cascades (small stress → big stress)
    pub stress_cascade_speed: Option<f32>,

    /// Time to recover from acute stress
    pub recovery_half_life: String,

    /// How much reassurance helps
    pub reassurance_soothing_effectiveness: Option<f32>,

    /// Time needed to restore boundaries after violation
    pub boundary_restoration_latency: Option<f32>,

    /// Negative effect of forced isolation
    pub isolation_penalty: Option<f32>,

    /// How much meaning-reframing helps recovery
    pub meaning_reframe_effectiveness: Option<f32>,
}

impl Default for StressResponseProfile {
    fn default() -> Self {
        Self {
            threat_detection_threshold: 0.5,
            emotional_flood_vs_shutdown_bias: None,
            freeze_vs_flight_bias: None,
            withdrawal_activation_threshold: 0.5,
            confusion_under_precision_pressure: None,
            stress_cascade_speed: None,
            recovery_half_life: "4-6 hours".to_string(),
            reassurance_soothing_effectiveness: None,
            boundary_restoration_latency: None,
            isolation_penalty: None,
            meaning_reframe_effectiveness: None,
        }
    }
}

impl StressResponseProfile {
    pub fn new() -> Self {
        Self::default()
    }

    /// Create with core thresholds
    pub fn with_thresholds(
        threat_detection_threshold: f32,
        withdrawal_activation_threshold: f32,
        recovery_half_life: String,
    ) -> Self {
        Self {
            threat_detection_threshold: threat_detection_threshold.clamp(0.0, 1.0),
            withdrawal_activation_threshold: withdrawal_activation_threshold.clamp(0.0, 1.0),
            recovery_half_life,
            ..Default::default()
        }
    }

    /// Set emotional response type
    pub fn with_emotional_response(mut self, response_type: EmotionalResponseType) -> Self {
        self.emotional_flood_vs_shutdown_bias = Some(response_type);
        self
    }

    /// Set freeze response
    pub fn with_freeze_response(mut self) -> Self {
        self.freeze_vs_flight_bias = Some(FreezeVsFlightBias::WithdrawalFreeze);
        self
    }

    /// Add precision pressure confusion
    pub fn with_precision_pressure(mut self, confusion: f32) -> Self {
        self.confusion_under_precision_pressure = Some(confusion.clamp(0.0, 1.0));
        self
    }

    /// Add stress cascade speed
    pub fn with_stress_cascade(mut self, speed: f32) -> Self {
        self.stress_cascade_speed = Some(speed.clamp(0.0, 1.0));
        self
    }

    /// Add reassurance effectiveness
    pub fn with_reassurance_effectiveness(mut self, effectiveness: f32) -> Self {
        self.reassurance_soothing_effectiveness = Some(effectiveness.clamp(0.0, 1.0));
        self
    }

    /// Add boundary restoration latency
    pub fn with_boundary_restoration(mut self, latency: f32) -> Self {
        self.boundary_restoration_latency = Some(latency.clamp(0.0, 1.0));
        self
    }

    /// Add isolation penalty
    pub fn with_isolation_penalty(mut self, penalty: f32) -> Self {
        self.isolation_penalty = Some(penalty.clamp(0.0, 1.0));
        self
    }

    /// Add meaning reframe effectiveness
    pub fn with_meaning_reframe(mut self, effectiveness: f32) -> Self {
        self.meaning_reframe_effectiveness = Some(effectiveness.clamp(0.0, 1.0));
        self
    }

    /// Check if threat-sensitive (low threshold)
    pub fn is_threat_sensitive(&self) -> bool {
        self.threat_detection_threshold < 0.4
    }

    /// Check if easily withdrawn
    pub fn withdraws_easily(&self) -> bool {
        self.withdrawal_activation_threshold > 0.6
    }

    /// Check if fast stress cascade
    pub fn has_fast_stress_cascade(&self) -> bool {
        self.stress_cascade_speed.map(|v| v > 0.6).unwrap_or(false)
    }

    /// Check if freeze response dominant
    pub fn has_freeze_dominant(&self) -> bool {
        matches!(
            self.freeze_vs_flight_bias,
            Some(FreezeVsFlightBias::WithdrawalFreeze)
        )
    }

    /// Check if responds to reassurance
    pub fn responds_to_reassurance(&self) -> bool {
        self.reassurance_soothing_effectiveness
            .map(|v| v > 0.5)
            .unwrap_or(false)
    }

    /// Recovery half-life in hours, parsed from `recovery_half_life`.
    ///
    /// Accepts a range such as `"4-6 hours"` (returns the lower bound, the
    /// fastest recovery the profile allows), a single value such as
    /// `"4 hours"`, `"4h"` or `"90 minutes"`, and ISO 8601 durations such as
    /// `"PT4H"` or `"PT1H30M"`. Returns `None` for anything else.
    pub fn recovery_hours(&self) -> Option<f32> {
        parse_duration_hours(&self.recovery_half_life)
    }

    /// Stress response style, from three independent axes: threat
    /// sensitivity, whether freeze dominates, and how fast stress cascades.
    pub fn response_style(&self) -> &'static str {
        match (
            self.is_threat_sensitive(),
            self.has_freeze_dominant(),
            self.has_fast_stress_cascade(),
        ) {
            (true, true, _) => "Threat-Sensitive Freezer",
            (true, false, _) => "Threat-Sensitive Runner",
            (false, true, _) => "Controlled Freezer",
            (false, false, true) => "Variable Responder",
            (false, false, false) => "Regulated Processor",
        }
    }
}

/// Parse a human or ISO 8601 duration into hours. See
/// [`StressResponseProfile::recovery_hours`].
fn parse_duration_hours(text: &str) -> Option<f32> {
    let text = text.trim();
    if let Some(iso) = text.strip_prefix("PT").or_else(|| text.strip_prefix("pt")) {
        let mut hours = 0.0f32;
        let mut number = String::new();
        let mut any = false;
        for c in iso.chars() {
            if c.is_ascii_digit() || c == '.' {
                number.push(c);
                continue;
            }
            let value: f32 = number.parse().ok()?;
            number.clear();
            hours += match c.to_ascii_uppercase() {
                'H' => value,
                'M' => value / 60.0,
                'S' => value / 3600.0,
                _ => return None,
            };
            any = true;
        }
        return (any && number.is_empty()).then_some(hours);
    }

    let lower = text.to_ascii_lowercase();
    let (number_part, unit_hours) = strip_unit(&lower, &["minutes", "minute", "mins", "min", "m"])
        .map(|rest| (rest, 1.0 / 60.0))
        .or_else(|| {
            strip_unit(&lower, &["hours", "hour", "hrs", "hr", "h"]).map(|rest| (rest, 1.0))
        })?;
    let first = number_part.split('-').next()?.trim();
    let value: f32 = first.parse().ok()?;
    (value.is_finite() && value >= 0.0).then_some(value * unit_hours)
}

fn strip_unit<'a>(text: &'a str, units: &[&str]) -> Option<&'a str> {
    units
        .iter()
        .find_map(|unit| text.strip_suffix(unit))
        .map(str::trim)
        .filter(|rest| !rest.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recovery_hours_parses_common_forms() {
        for (text, hours) in [
            ("4-6 hours", 4.0),
            ("4 hours", 4.0),
            ("2h", 2.0),
            ("90 minutes", 1.5),
            ("PT4H", 4.0),
            ("PT1H30M", 1.5),
        ] {
            let profile = StressResponseProfile {
                recovery_half_life: text.to_string(),
                ..StressResponseProfile::default()
            };
            let parsed = profile.recovery_hours().expect(text);
            assert!((parsed - hours).abs() < 1e-5, "{text}: {parsed}");
        }
        for text in ["", "soon", "PT", "PTxH", "hours"] {
            let profile = StressResponseProfile {
                recovery_half_life: text.to_string(),
                ..StressResponseProfile::default()
            };
            assert_eq!(profile.recovery_hours(), None, "{text}");
        }
    }

    #[test]
    fn every_response_style_is_reachable() {
        let mut seen = std::collections::HashSet::new();
        for threshold in [0.2, 0.8] {
            for freeze in [None, Some(FreezeVsFlightBias::WithdrawalFreeze)] {
                for cascade in [None, Some(0.9)] {
                    let profile = StressResponseProfile {
                        threat_detection_threshold: threshold,
                        freeze_vs_flight_bias: freeze,
                        stress_cascade_speed: cascade,
                        ..StressResponseProfile::default()
                    };
                    seen.insert(profile.response_style());
                }
            }
        }
        assert_eq!(seen.len(), 5);
    }

    #[test]
    fn default_stress_response() {
        let response = StressResponseProfile::default();
        assert_eq!(response.threat_detection_threshold, 0.5);
        assert!(response.emotional_flood_vs_shutdown_bias.is_none());
    }

    #[test]
    fn threat_sensitivity_detection() {
        let sensitive = StressResponseProfile::with_thresholds(0.3, 0.5, "2-4 hours".to_string());
        assert!(sensitive.is_threat_sensitive());

        let resilient = StressResponseProfile::with_thresholds(0.7, 0.5, "2-4 hours".to_string());
        assert!(!resilient.is_threat_sensitive());
    }

    #[test]
    fn freeze_response_detection() {
        let freezer = StressResponseProfile::new().with_freeze_response();
        assert!(freezer.has_freeze_dominant());
    }

    #[test]
    fn recovery_hours_parsing() {
        let response = StressResponseProfile::with_thresholds(0.5, 0.5, "4-6 hours".to_string());
        assert_eq!(response.recovery_hours(), Some(4.0));
    }

    #[test]
    fn response_style_detection() {
        let sensitive_freezer =
            StressResponseProfile::with_thresholds(0.3, 0.5, "4 hours".to_string())
                .with_freeze_response();
        assert_eq!(
            sensitive_freezer.response_style(),
            "Threat-Sensitive Freezer"
        );
    }
}
