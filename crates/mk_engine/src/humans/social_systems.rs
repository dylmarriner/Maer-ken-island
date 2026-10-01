//! Social Systems Snapshot - Relationship dynamics and social behavior

use mk_core::human::HumanProfile;
use serde::{Deserialize, Serialize};

/// Runtime social systems snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocialSystemsSnapshot {
    /// Attachment security level (0.0 = insecure, 1.0 = fully secure)
    pub attachment_security: f64,

    /// Social trust level (0.0 = distrustful, 1.0 = trusting)
    pub social_trust: f64,

    /// Conflict resolution ability (0.0 to 1.0)
    pub conflict_resolution: f64,

    /// Cooperation tendency (0.0 = competitive, 1.0 = cooperative)
    pub cooperation: f64,

    /// Empathic accuracy (0.0 to 1.0)
    pub empathic_accuracy: f64,

    /// Social boundary detection (0.0 = poor, 1.0 = excellent)
    pub boundary_detection: f64,

    /// Loneliness tolerance (0.0 = needs constant company, 1.0 = fully self-sufficient)
    pub loneliness_tolerance: f64,

    /// Group identification strength (0.0 to 1.0)
    pub group_identity: f64,

    /// Jealousy reactivity (0.0 = none, 1.0 = extreme)
    pub jealousy_reactivity: f64,

    /// Abandonment sensitivity (0.0 = secure, 1.0 = hypervigilant)
    pub abandonment_sensitivity: f64,

    /// Accumulated deviation from disposition caused by real lived events
    /// (grief, betrayal, bonding), in `[-0.4, 0.4]`. `from_profile()`
    /// always resets this to `0.0`; callers that re-derive this snapshot
    /// every tick (`HumanBeing::refresh_phase11_layers`) must preserve it
    /// across that reset the same way `TechnologySnapshot::accumulated_knowledge`
    /// already is (see that call site's own comment). Without this,
    /// `attachment_security`/`social_trust` re-derive identically every
    /// tick from static `temperament`/`attachment_style`/`drive_weights` —
    /// the confirmed gap in
    /// `audit-results/human-consciousness-plan-v1-code-verification-2026-09-11.md`
    /// Phase 12 ("no life event moves this module's real inputs at all").
    #[serde(default)]
    pub experiential_trust: f64,
}

impl SocialSystemsSnapshot {
    /// Nudge `experiential_trust` by a real life event (positive = bonding/
    /// trust-building, negative = betrayal/grief/harm), clamped to
    /// `[-0.4, 0.4]`, and immediately apply the actually-accepted portion
    /// (post-clamp) to `social_trust` (full weight) and
    /// `attachment_security` (half weight — losing someone erodes trust
    /// more directly than it erodes a person's whole attachment style).
    pub fn apply_social_shock(&mut self, delta: f64) {
        let before = self.experiential_trust;
        self.experiential_trust = (self.experiential_trust + delta).clamp(-0.4, 0.4);
        let applied = self.experiential_trust - before;
        self.social_trust = (self.social_trust + applied).clamp(0.0, 1.0);
        self.attachment_security = (self.attachment_security + applied * 0.5).clamp(0.0, 1.0);
    }

    /// Derive social systems snapshot from a HumanProfile
    pub fn from_profile(profile: &HumanProfile) -> Self {
        let temperament = &profile.temperament_matrix;
        let attachment = &profile.attachment_style;
        let drives = &profile.drive_weights;
        let stress = &profile.stress_response_profile;
        let _relational = &profile.relational_defaults;

        let empathy = temperament.empathy as f64;
        let bonding = drives.bonding as f64;

        // Attachment security derived from attachment style
        let attachment_security = match attachment.primary_pattern {
            mk_core::human::attachment::AttachmentPatternType::Secure => 0.85,
            mk_core::human::attachment::AttachmentPatternType::AnxiousPreoccupied => 0.35,
            mk_core::human::attachment::AttachmentPatternType::DismissiveAvoidant => 0.45,
            mk_core::human::attachment::AttachmentPatternType::FearfulAvoidant => 0.25,
            mk_core::human::attachment::AttachmentPatternType::AnxiousAvoidantHybrid => 0.30,
        };

        // Social trust from attachment and bonding
        let social_trust =
            (attachment_security * 0.5 + bonding * 0.3 + empathy * 0.2).clamp(0.0, 1.0);

        Self {
            attachment_security,
            social_trust,
            conflict_resolution: empathy * 0.4
                + temperament.adaptability as f64 * 0.4
                + social_trust * 0.2,
            cooperation: empathy * 0.4 + bonding * 0.4 + temperament.assertiveness as f64 * 0.2,
            empathic_accuracy: empathy,
            boundary_detection: (1.0 - stress.threat_detection_threshold as f64).clamp(0.0, 1.0),
            loneliness_tolerance: temperament.introversion_extroversion as f64 * 0.5
                + (1.0 - bonding) * 0.3
                + drives.autonomy as f64 * 0.2,
            group_identity: bonding * 0.5
                + empathy * 0.3
                + temperament.conscientiousness as f64 * 0.2,
            jealousy_reactivity: attachment.abandonment_reactivity.unwrap_or(0.3) as f64,
            abandonment_sensitivity: attachment.abandonment_sensitivity.unwrap_or(0.3) as f64,
            experiential_trust: 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn social_snapshot_has_attachment_security() {
        let schema = mk_core::human::HumanSchema::canonical_minimal("test");
        let profile = mk_core::human::HumanProfile::from_canonical_schema(
            mk_core::human::HumanId::new(1),
            schema,
        );
        let social = SocialSystemsSnapshot::from_profile(&profile);
        assert!(social.attachment_security >= 0.0 && social.attachment_security <= 1.0);
    }

    #[test]
    fn apply_social_shock_moves_trust_and_attachment_and_clamps() {
        let schema = mk_core::human::HumanSchema::canonical_minimal("test");
        let profile = mk_core::human::HumanProfile::from_canonical_schema(
            mk_core::human::HumanId::new(1),
            schema,
        );
        let mut social = SocialSystemsSnapshot::from_profile(&profile);
        let trust_before = social.social_trust;
        let attachment_before = social.attachment_security;

        social.apply_social_shock(-0.2);
        assert!(social.social_trust < trust_before);
        assert!(social.attachment_security < attachment_before);
        assert_eq!(social.experiential_trust, -0.2);

        // Repeated large negative shocks clamp the accumulator rather than
        // driving trust arbitrarily negative-then-clamped-at-zero forever.
        social.apply_social_shock(-10.0);
        assert_eq!(social.experiential_trust, -0.4);
        assert!(social.social_trust >= 0.0);
        assert!(social.attachment_security >= 0.0);
    }
}
