//! Dark Triad Snapshot - narcissism, Machiavellianism, psychopathy facets
//! and their real-time malice/vengeance expression.
//!
//! Surfaces `docs/canon/HumanReplicationSchema.js`'s `DarkTriadSchema`,
//! ported in `mk_core::human::schema` (facets as flattened `BTreeMap<String,
//! f32>`) but previously unread by the engine. Coupled to `emotion`:
//! active malice and vengeance drive track real-time hate/resentment/
//! humiliation rather than a static schema readout, gated by the
//! psychopathy `lack_of_remorse` trait (low-remorse individuals convert
//! negative emotion into malice more readily).

use mk_core::human::HumanProfile;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DarkTriadSnapshot {
    // --- narcissism facets (traits) ---
    pub narcissism_self_importance: f64,
    pub narcissism_entitlement: f64,
    pub narcissism_empathy_deficit: f64,

    // --- machiavellianism facets (traits) ---
    pub machiavellianism_strategic_thinking: f64,
    pub machiavellianism_manipulation_skill: f64,
    pub machiavellianism_emotional_detachment: f64,

    // --- psychopathy facets (traits) ---
    pub psychopathy_lack_of_remorse: f64,
    pub psychopathy_impulsivity: f64,
    pub psychopathy_callousness: f64,

    // --- combined (traits) ---
    pub overall_darkness: f64,

    // --- state (stepped) ---
    /// Real-time malicious intent, driven by current hate/anger and gated
    /// by remorse deficit (0-1).
    pub active_malice: f64,
    /// Real-time desire for revenge, driven by resentment/humiliation
    /// (0-1).
    pub vengeance_drive: f64,
}

impl DarkTriadSnapshot {
    pub fn from_profile(profile: &HumanProfile) -> Self {
        let Some(schema) = profile.canonical_schema() else {
            return Self::defaults();
        };
        let d = &schema.dark_triad;

        let facet = |map: &std::collections::BTreeMap<String, f32>, key: &str, default: f64| {
            map.get(key)
                .copied()
                .map(|v| v as f64)
                .filter(|v| *v != 0.0)
                .unwrap_or(default)
        };

        Self {
            narcissism_self_importance: facet(&d.narcissism.values, "self_importance", 0.3),
            narcissism_entitlement: facet(&d.narcissism.values, "entitlement", 0.3),
            narcissism_empathy_deficit: facet(&d.narcissism.values, "empathy_deficit", 0.2),

            machiavellianism_strategic_thinking: facet(
                &d.machiavellianism.values,
                "strategic_thinking",
                0.4,
            ),
            machiavellianism_manipulation_skill: facet(
                &d.machiavellianism.values,
                "manipulation_skill",
                0.2,
            ),
            machiavellianism_emotional_detachment: facet(
                &d.machiavellianism.values,
                "emotional_detachment",
                0.3,
            ),

            psychopathy_lack_of_remorse: facet(&d.psychopathy.values, "lack_of_remorse", 0.2),
            psychopathy_impulsivity: facet(&d.psychopathy.values, "impulsivity", 0.3),
            psychopathy_callousness: facet(&d.psychopathy.values, "callousness", 0.2),

            overall_darkness: nonzero_or(d.overall_darkness, 0.25),

            active_malice: nonzero_or(d.active_malice, 0.0),
            vengeance_drive: nonzero_or(d.vengeance_drive, 0.0),
        }
    }

    /// Couple malice/vengeance expression to real-time emotional state
    /// instead of a static schema readout.
    pub fn step(&self, emotion: &super::emotion::EmotionSnapshot) -> Self {
        let hostile_signal =
            (emotion.current.hate * 0.5 + emotion.current.anger * 0.5).clamp(0.0, 1.0);
        let active_malice = (hostile_signal * self.psychopathy_lack_of_remorse).clamp(0.0, 1.0);

        let grievance_signal =
            (emotion.current.resentment * 0.6 + emotion.current.humiliation * 0.4).clamp(0.0, 1.0);
        let vengeance_drive =
            (grievance_signal * (0.3 + self.overall_darkness * 0.7)).clamp(0.0, 1.0);

        Self {
            active_malice,
            vengeance_drive,
            ..self.clone()
        }
    }

    fn defaults() -> Self {
        Self {
            narcissism_self_importance: 0.3,
            narcissism_entitlement: 0.3,
            narcissism_empathy_deficit: 0.2,
            machiavellianism_strategic_thinking: 0.4,
            machiavellianism_manipulation_skill: 0.2,
            machiavellianism_emotional_detachment: 0.3,
            psychopathy_lack_of_remorse: 0.2,
            psychopathy_impulsivity: 0.3,
            psychopathy_callousness: 0.2,
            overall_darkness: 0.25,
            active_malice: 0.0,
            vengeance_drive: 0.0,
        }
    }
}

fn nonzero_or(value: f32, default: f64) -> f64 {
    if value == 0.0 {
        default
    } else {
        value as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::humans::emotion::EmotionSnapshot;
    use mk_core::human::{HumanId, HumanSchema};

    fn profile() -> HumanProfile {
        let schema = HumanSchema::canonical_minimal("dark_triad_test");
        HumanProfile::from_canonical_schema(HumanId::new(1), schema)
    }

    #[test]
    fn from_profile_uses_sane_defaults() {
        let snapshot = DarkTriadSnapshot::from_profile(&profile());
        assert!(snapshot.overall_darkness > 0.0);
        assert!(snapshot.psychopathy_lack_of_remorse >= 0.0);
    }

    #[test]
    fn hostile_emotion_raises_active_malice_when_remorse_deficient() {
        let mut snapshot = DarkTriadSnapshot::from_profile(&profile());
        snapshot.psychopathy_lack_of_remorse = 1.0;

        let mut hostile_emotion = EmotionSnapshot::from_profile(&profile());
        hostile_emotion.current.hate = 1.0;
        hostile_emotion.current.anger = 1.0;

        let calm_emotion = EmotionSnapshot::from_profile(&profile());

        let stepped_hostile = snapshot.step(&hostile_emotion);
        let stepped_calm = snapshot.step(&calm_emotion);

        assert!(stepped_hostile.active_malice >= stepped_calm.active_malice);
    }

    #[test]
    fn remorse_gates_malice_expression() {
        let mut remorseful = DarkTriadSnapshot::from_profile(&profile());
        remorseful.psychopathy_lack_of_remorse = 0.0;
        let mut remorseless = DarkTriadSnapshot::from_profile(&profile());
        remorseless.psychopathy_lack_of_remorse = 1.0;

        let mut hostile_emotion = EmotionSnapshot::from_profile(&profile());
        hostile_emotion.current.hate = 1.0;
        hostile_emotion.current.anger = 1.0;

        let stepped_remorseful = remorseful.step(&hostile_emotion);
        let stepped_remorseless = remorseless.step(&hostile_emotion);

        assert!(stepped_remorseful.active_malice <= stepped_remorseless.active_malice);
    }
}
