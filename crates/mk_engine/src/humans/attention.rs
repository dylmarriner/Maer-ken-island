//! Attention Snapshot - traits, current state, resources, distraction.
//!
//! Surfaces `docs/canon/HumanReplicationSchema.js`'s `AttentionSystemSchema`,
//! ported in `mk_core::human::schema` but previously unread by the engine.
//! Coupled to `needs`/`body`: fatigue depletes available attention and
//! raises distraction, mirroring real cognitive-fatigue effects.

use mk_core::human::HumanProfile;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttentionSnapshot {
    // --- traits (stable) ---
    pub capacity: f64,
    pub focus_trait: f64,
    pub distractibility: f64,
    pub multitasking: f64,

    // --- state (stepped) ---
    pub focus_level: f64,
    pub cognitive_load: f64,
    pub attention_fatigue: f64,
    pub flow: f64,

    // --- resources (stepped) ---
    pub available: f64,
    pub efficiency: f64,
}

impl AttentionSnapshot {
    pub fn from_profile(profile: &HumanProfile) -> Self {
        let Some(schema) = profile.canonical_schema() else {
            return Self::defaults();
        };
        let a = &schema.attention_system;

        Self {
            capacity: nonzero_or(a.attention_traits.capacity as f64, 0.7),
            focus_trait: nonzero_or(a.attention_traits.focus as f64, 0.6),
            distractibility: a.attention_traits.distractibility as f64,
            multitasking: nonzero_or(a.attention_traits.multitasking as f64, 0.5),

            focus_level: nonzero_or(a.attention_state.focus_level as f64, 0.6),
            cognitive_load: a.attention_state.cognitive_load as f64,
            attention_fatigue: a.attention_state.fatigue as f64,
            flow: a.attention_state.flow as f64,

            available: nonzero_or(a.attention_resources.available as f64, 0.7),
            efficiency: nonzero_or(a.attention_resources.efficiency as f64, 0.7),
        }
    }

    /// Couple attentional resources to actual physiological fatigue/needs
    /// instead of a static schema readout.
    pub fn step(&self, needs: &super::needs::NeedsSnapshot, dt_years: f64) -> Self {
        let dt = dt_years.max(0.0);

        let attention_fatigue = (self.attention_fatigue
            + (needs.fatigue - self.attention_fatigue) * (0.6 * dt).clamp(0.0, 1.0))
        .clamp(0.0, 1.0);

        let available = (self.capacity * (1.0 - attention_fatigue)).clamp(0.0, 1.0);
        let focus_level = (self.focus_trait * (1.0 - attention_fatigue * 0.7)).clamp(0.0, 1.0);
        let cognitive_load = (self.distractibility * 0.3 + attention_fatigue * 0.5).clamp(0.0, 1.0);

        Self {
            attention_fatigue,
            available,
            focus_level,
            cognitive_load,
            ..self.clone()
        }
    }

    fn defaults() -> Self {
        Self {
            capacity: 0.7,
            focus_trait: 0.6,
            distractibility: 0.3,
            multitasking: 0.5,
            focus_level: 0.6,
            cognitive_load: 0.2,
            attention_fatigue: 0.0,
            flow: 0.0,
            available: 0.7,
            efficiency: 0.7,
        }
    }
}

fn nonzero_or(value: f64, default: f64) -> f64 {
    if value == 0.0 {
        default
    } else {
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::AgentWorldObservation;
    use crate::humans::needs::{EffortFocus, NeedsSnapshot};
    use mk_core::human::{HumanId, HumanSchema};

    fn profile() -> HumanProfile {
        let schema = HumanSchema::canonical_minimal("attention_test");
        HumanProfile::from_canonical_schema(HumanId::new(1), schema)
    }

    #[test]
    fn from_profile_uses_sane_defaults() {
        let snapshot = AttentionSnapshot::from_profile(&profile());
        assert!(snapshot.capacity > 0.0);
    }

    #[test]
    fn fatigue_depletes_available_attention() {
        let snapshot = AttentionSnapshot::from_profile(&profile());
        let needs = NeedsSnapshot::from_profile(&profile());
        let starved_observation = AgentWorldObservation {
            caloric_access: 0.0,
            hydration_access: 0.0,
            shelter_quality: 0.0,
            ..AgentWorldObservation::default()
        };
        let mut tired_needs = needs;
        for _ in 0..30 {
            tired_needs = tired_needs.step(&starved_observation, 1.0, EffortFocus::none(), 1.0);
        }

        let rested = snapshot.step(&NeedsSnapshot::from_profile(&profile()), 1.0);
        let tired = snapshot.step(&tired_needs, 1.0);

        assert!(tired.available <= rested.available);
    }
}
