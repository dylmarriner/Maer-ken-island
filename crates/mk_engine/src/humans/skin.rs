//! Skin Snapshot - temperature, cleanliness, healing, protection, infection risk.
//!
//! Surfaces `docs/canon/HumanReplicationSchema.js`'s `SkinSystemSchema`,
//! ported in `mk_core::human::schema` but previously unread by the engine.
//! Coupled to `body` (hygiene) and `immune` (system_stress) so cleanliness
//! and immune stress actually affect infection risk, instead of the
//! canon field sitting static.

use mk_core::human::HumanProfile;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkinSnapshot {
    pub temperature_c: f64,
    pub cleanliness: f64,
    pub healing_rate: f64,
    pub protection: f64,
    pub integrity: f64,
    pub infection_risk: f64,
    pub healing_event_count: usize,
}

impl SkinSnapshot {
    pub fn from_profile(profile: &HumanProfile) -> Self {
        let Some(schema) = profile.canonical_schema() else {
            return Self::defaults();
        };
        let skin = &schema.skin_system;

        Self {
            temperature_c: nonzero_or(skin.temperature as f64, 33.0),
            cleanliness: nonzero_or(skin.cleanliness as f64, 1.0),
            healing_rate: nonzero_or(skin.healing_rate as f64, 0.5),
            protection: nonzero_or(skin.protection as f64, 0.8),
            integrity: nonzero_or(skin.integrity as f64, 1.0),
            infection_risk: skin.infection_risk as f64,
            healing_event_count: skin.healing_events.len(),
        }
    }

    /// Step skin state forward: cleanliness tracks hygiene access, and
    /// infection risk rises when cleanliness/integrity are low and immune
    /// system_stress is high (rather than an inert schema field).
    pub fn step(&self, hygiene: f64, immune_system_stress: f64, dt_years: f64) -> Self {
        let dt = dt_years.max(0.0);

        let cleanliness = (self.cleanliness
            + (hygiene - self.cleanliness) * (0.5 * dt).clamp(0.0, 1.0))
        .clamp(0.0, 1.0);

        let integrity = (self.integrity + self.healing_rate * dt * 0.1).clamp(0.0, 1.0);

        let risk_pressure =
            (1.0 - cleanliness) * 0.5 + (1.0 - integrity) * 0.3 + immune_system_stress * 0.2;
        let infection_risk = (self.infection_risk
            + (risk_pressure - self.infection_risk) * (0.3 * dt).clamp(0.0, 1.0))
        .clamp(0.0, 1.0);

        Self {
            cleanliness,
            integrity,
            infection_risk,
            ..self.clone()
        }
    }

    fn defaults() -> Self {
        Self {
            temperature_c: 33.0,
            cleanliness: 1.0,
            healing_rate: 0.5,
            protection: 0.8,
            integrity: 1.0,
            infection_risk: 0.0,
            healing_event_count: 0,
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
    use mk_core::human::{HumanId, HumanSchema};

    fn profile() -> HumanProfile {
        let schema = HumanSchema::canonical_minimal("skin_test");
        HumanProfile::from_canonical_schema(HumanId::new(1), schema)
    }

    #[test]
    fn from_profile_uses_sane_defaults() {
        let snapshot = SkinSnapshot::from_profile(&profile());
        assert!((snapshot.cleanliness - 1.0).abs() < 1e-9);
    }

    #[test]
    fn poor_hygiene_and_high_immune_stress_raise_infection_risk() {
        let snapshot = SkinSnapshot::from_profile(&profile());
        let mut neglected = snapshot.clone();
        for _ in 0..20 {
            neglected = neglected.step(0.0, 0.8, 1.0);
        }
        assert!(neglected.infection_risk > snapshot.infection_risk);
    }
}
