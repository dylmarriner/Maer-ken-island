//! Culture Snapshot - Cultural identity and norm adherence

use mk_core::human::HumanProfile;
use serde::{Deserialize, Serialize};

/// Runtime culture snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CultureSnapshot {
    /// Capacity for symbolic/abstract thought (0.0 to 1.0)
    pub symbolic_capacity: f64,

    /// Adherence to cultural norms (0.0 = rebel, 1.0 = conformist)
    pub norm_retention: f64,

    /// Cultural identity strength (0.0 to 1.0)
    pub identity_strength: f64,

    /// Tradition vs innovation orientation (0.0 = traditionalist, 1.0 = innovator)
    pub innovation_orientation: f64,

    /// Collectivism vs individualism (0.0 = collectivist, 1.0 = individualist)
    pub individualism: f64,

    /// Ritual participation tendency (0.0 to 1.0)
    pub ritual_tendency: f64,

    /// Cultural transmission ability (0.0 to 1.0)
    pub transmission_ability: f64,
}

impl CultureSnapshot {
    /// Derive culture snapshot from a HumanProfile
    pub fn from_profile(profile: &HumanProfile) -> Self {
        let temperament = &profile.temperament_matrix;
        let drives = &profile.drive_weights;
        let _attachment = &profile.attachment_style;

        // Openness drives innovation orientation and symbolic capacity
        let openness = temperament.openness_to_experience as f64;

        // Conscientiousness drives norm retention and ritual tendency
        let conscientiousness = temperament.conscientiousness as f64;

        // Autonomy drive vs harmony drive shapes individualism
        let autonomy = drives.autonomy as f64;
        let harmony = drives.harmony.unwrap_or(0.5) as f64;
        let individualism = (autonomy / (autonomy + harmony + 0.001)).clamp(0.0, 1.0);

        // Bonding strength affects cultural transmission
        let bonding = drives.bonding as f64;

        Self {
            symbolic_capacity: openness,
            norm_retention: conscientiousness,
            identity_strength: (bonding * 0.5 + conscientiousness * 0.3 + openness * 0.2)
                .clamp(0.0, 1.0),
            innovation_orientation: openness,
            individualism,
            ritual_tendency: conscientiousness * 0.7 + bonding * 0.3,
            transmission_ability: bonding * 0.5 + temperament.empathy as f64 * 0.5,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn culture_snapshot_from_profile() {
        let schema = mk_core::human::HumanSchema::canonical_minimal("test");
        let profile = mk_core::human::HumanProfile::from_canonical_schema(
            mk_core::human::HumanId::new(1),
            schema,
        );
        let culture = CultureSnapshot::from_profile(&profile);
        assert!(culture.symbolic_capacity >= 0.0);
        assert!(culture.norm_retention >= 0.0);
    }
}
