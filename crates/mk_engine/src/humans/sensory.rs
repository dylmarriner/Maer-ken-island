//! Sensory Snapshot - compact aggregate over tactile, visual, auditory,
//! vestibular, interoception, proprioception, and multimodal integration.
//!
//! Surfaces `docs/canon/HumanReplicationSchema.js`'s `SensorySystemsSchema`,
//! ported in `mk_core::human::schema`. Each sense is modelled as a capacity
//! seeded from the person's schema and stepped every tick against their
//! body (pain, fatigue, hydration) and their real surroundings: daylight
//! from the live insolation field, crowding from the people around them,
//! and terrain hazard (see [`SensorySnapshot::step`]).

use mk_core::human::HumanProfile;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensorySnapshot {
    // --- tactile ---
    pub overall_pain_level: f64,
    pub tactile_event_count: usize,

    // --- proprioception ---
    pub balance: f64,
    pub coordination: f64,
    pub phantom_limb_risk: f64,

    // --- visual ---
    pub visual_acuity: f64,
    pub color_perception: f64,
    pub depth_perception: f64,
    pub motion_detection: f64,

    // --- auditory ---
    pub hearing_sensitivity: f64,
    pub sound_localization: f64,
    pub speech_recognition: f64,

    // --- vestibular ---
    pub balance_sensitivity: f64,
    pub motion_sickness: f64,
    pub spatial_orientation: f64,

    // --- interoception ---
    pub body_awareness: f64,
    pub internal_state_monitoring: f64,
    pub homeostatic_regulation: f64,

    // --- integration ---
    pub multimodal_processing: f64,
    pub sensory_filtering: f64,
    pub attention_modulation: f64,

    // --- world-coupling baselines ---
    // Schema-derived ceilings the fields above are stepped toward, scaled by
    // real `AgentWorldObservation` conditions (light/noise/threat). Without
    // these the `step()` target would have to be computed relative to the
    // *previous* tick's already-scaled value, which compounds toward zero
    // under a sustained condition (e.g. several dark ticks in a row) instead
    // of settling at a stable floor.
    #[serde(default)]
    pub baseline_visual_acuity: f64,
    #[serde(default)]
    pub baseline_color_perception: f64,
    #[serde(default)]
    pub baseline_motion_detection: f64,
    #[serde(default)]
    pub baseline_sound_localization: f64,
    #[serde(default)]
    pub baseline_speech_recognition: f64,
    #[serde(default)]
    pub baseline_sensory_filtering: f64,
    #[serde(default)]
    pub baseline_attention_modulation: f64,
}

impl SensorySnapshot {
    pub fn from_profile(profile: &HumanProfile) -> Self {
        let Some(schema) = profile.canonical_schema() else {
            return Self::defaults();
        };
        let s = &schema.sensory_systems;

        let visual_acuity = nonzero_or(s.visual_system.visual_acuity as f64, 0.9);
        let color_perception = nonzero_or(s.visual_system.color_perception as f64, 0.9);
        let motion_detection = nonzero_or(s.visual_system.motion_detection as f64, 0.8);
        let sound_localization = nonzero_or(s.auditory_system.sound_localization as f64, 0.7);
        let speech_recognition = nonzero_or(s.auditory_system.speech_recognition as f64, 0.9);
        let sensory_filtering = nonzero_or(s.sensory_integration.sensory_filtering as f64, 0.6);
        let attention_modulation =
            nonzero_or(s.sensory_integration.attention_modulation as f64, 0.6);

        Self {
            overall_pain_level: s.tactile_system.overall_pain_level as f64,
            tactile_event_count: s.tactile_system.tactile_events.len(),

            balance: nonzero_or(
                s.proprioception_system.spatial_awareness.balance as f64,
                0.7,
            ),
            coordination: nonzero_or(
                s.proprioception_system.spatial_awareness.coordination as f64,
                0.7,
            ),
            phantom_limb_risk: s.proprioception_system.spatial_awareness.phantom_limb_risk as f64,

            visual_acuity,
            color_perception,
            depth_perception: nonzero_or(s.visual_system.depth_perception as f64, 0.8),
            motion_detection,

            hearing_sensitivity: nonzero_or(s.auditory_system.hearing_sensitivity as f64, 0.8),
            sound_localization,
            speech_recognition,

            balance_sensitivity: nonzero_or(s.vestibular_system.balance_sensitivity as f64, 0.7),
            motion_sickness: s.vestibular_system.motion_sickness as f64,
            spatial_orientation: nonzero_or(s.vestibular_system.spatial_orientation as f64, 0.7),

            body_awareness: nonzero_or(s.interoception_system.body_awareness as f64, 0.6),
            internal_state_monitoring: nonzero_or(
                s.interoception_system.internal_state_monitoring as f64,
                0.6,
            ),
            homeostatic_regulation: nonzero_or(
                s.interoception_system.homeostatic_regulation as f64,
                0.6,
            ),

            multimodal_processing: nonzero_or(
                s.sensory_integration.multimodal_processing as f64,
                0.7,
            ),
            sensory_filtering,
            attention_modulation,

            baseline_visual_acuity: visual_acuity,
            baseline_color_perception: color_perception,
            baseline_motion_detection: motion_detection,
            baseline_sound_localization: sound_localization,
            baseline_speech_recognition: speech_recognition,
            baseline_sensory_filtering: sensory_filtering,
            baseline_attention_modulation: attention_modulation,
        }
    }

    /// Couple pain/interoceptive readouts to the body's actual physical
    /// state, and couple perception-of-the-world fields to real
    /// `AgentWorldObservation` conditions instead of leaving them frozen at
    /// their schema baseline forever (previously confirmed gap #1 in
    /// `audit-results/human-consciousness-plan-v1-code-verification-2026-09-11.md`
    /// Phase 2: "no terrain, weather, nearby-entity, or event data feeds
    /// into sensory state"). Each world-coupled field is lerped toward a
    /// target derived from its own `baseline_*` ceiling times a real
    /// environmental factor, not toward a fixed constant — a human with
    /// naturally sharper eyes still sees relatively better at night than
    /// one with worse eyes.
    pub fn step(
        &self,
        body: &super::body::BodySnapshot,
        needs: &super::needs::NeedsSnapshot,
        observation: &super::AgentWorldObservation,
        dt_years: f64,
    ) -> Self {
        let overall_pain_level = ((1.0 - body.hygiene).max(0.0) * 0.2
            + needs.fatigue * 0.3
            + (1.0 - needs.hydration).max(0.0) * 0.2)
            .clamp(0.0, 1.0);

        let homeostatic_regulation = ((body.vital_energy + needs.hydration) / 2.0).clamp(0.0, 1.0);

        // Low light degrades vision — color (cone) perception hardest,
        // motion (rod/peripheral) detection least, acuity in between.
        let daylight = observation.daylight_fraction.clamp(0.0, 1.0);
        let visual_acuity_target = self.baseline_visual_acuity * (0.5 + 0.5 * daylight);
        let color_perception_target = self.baseline_color_perception * (0.2 + 0.8 * daylight);
        let motion_detection_target = self.baseline_motion_detection * (0.7 + 0.3 * daylight);

        // Crowd/ambient noise masks localization and speech more than it
        // affects raw hearing sensitivity.
        let social_density = observation.social_density.clamp(0.0, 1.0);
        let sound_localization_target =
            self.baseline_sound_localization * (1.0 - social_density * 0.3);
        let speech_recognition_target =
            self.baseline_speech_recognition * (1.0 - social_density * 0.3);

        // Real threat sharpens vigilance (attention_modulation up) at the
        // cost of broader sensory filtering (tunnel vision).
        let hazard = observation.hazard_index.clamp(0.0, 1.0);
        let attention_modulation_target =
            (self.baseline_attention_modulation + hazard * 0.3).clamp(0.0, 1.0);
        let sensory_filtering_target =
            (self.baseline_sensory_filtering - hazard * 0.2).clamp(0.0, 1.0);

        let blend = (2.0 * dt_years.max(0.0)).clamp(0.0, 1.0);
        let lerp = |current: f64, target: f64| current + (target - current) * blend;

        Self {
            overall_pain_level,
            homeostatic_regulation,
            visual_acuity: lerp(self.visual_acuity, visual_acuity_target),
            color_perception: lerp(self.color_perception, color_perception_target),
            motion_detection: lerp(self.motion_detection, motion_detection_target),
            sound_localization: lerp(self.sound_localization, sound_localization_target),
            speech_recognition: lerp(self.speech_recognition, speech_recognition_target),
            attention_modulation: lerp(self.attention_modulation, attention_modulation_target),
            sensory_filtering: lerp(self.sensory_filtering, sensory_filtering_target),
            ..self.clone()
        }
    }

    fn defaults() -> Self {
        Self {
            overall_pain_level: 0.0,
            tactile_event_count: 0,
            balance: 0.7,
            coordination: 0.7,
            phantom_limb_risk: 0.0,
            visual_acuity: 0.9,
            color_perception: 0.9,
            depth_perception: 0.8,
            motion_detection: 0.8,
            hearing_sensitivity: 0.8,
            sound_localization: 0.7,
            speech_recognition: 0.9,
            balance_sensitivity: 0.7,
            motion_sickness: 0.0,
            spatial_orientation: 0.7,
            body_awareness: 0.6,
            internal_state_monitoring: 0.6,
            homeostatic_regulation: 0.6,
            multimodal_processing: 0.7,
            sensory_filtering: 0.6,
            attention_modulation: 0.6,
            baseline_visual_acuity: 0.9,
            baseline_color_perception: 0.9,
            baseline_motion_detection: 0.8,
            baseline_sound_localization: 0.7,
            baseline_speech_recognition: 0.9,
            baseline_sensory_filtering: 0.6,
            baseline_attention_modulation: 0.6,
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
        let schema = HumanSchema::canonical_minimal("sensory_test");
        HumanProfile::from_canonical_schema(HumanId::new(1), schema)
    }

    #[test]
    fn from_profile_uses_sane_defaults() {
        let snapshot = SensorySnapshot::from_profile(&profile());
        assert!(snapshot.visual_acuity > 0.0);
        assert_eq!(snapshot.tactile_event_count, 0);
    }

    #[test]
    fn step_degrades_vision_in_low_light_and_recovers_in_daylight() {
        let body = super::super::body::BodySnapshot::from_profile(&profile());
        let needs = super::super::needs::NeedsSnapshot::from_profile(&profile());
        let mut observation = super::super::AgentWorldObservation {
            daylight_fraction: 0.0,
            ..Default::default()
        };
        let mut snapshot = SensorySnapshot::from_profile(&profile());
        let full_daylight_acuity = snapshot.visual_acuity;

        for _ in 0..10 {
            snapshot = snapshot.step(&body, &needs, &observation, 1.0);
        }
        assert!(snapshot.visual_acuity < full_daylight_acuity);
        let night_acuity = snapshot.visual_acuity;

        observation.daylight_fraction = 1.0;
        for _ in 0..10 {
            snapshot = snapshot.step(&body, &needs, &observation, 1.0);
        }
        assert!(snapshot.visual_acuity > night_acuity);
    }

    #[test]
    fn step_narrows_sensory_filtering_under_real_hazard() {
        let body = super::super::body::BodySnapshot::from_profile(&profile());
        let needs = super::super::needs::NeedsSnapshot::from_profile(&profile());
        let observation = super::super::AgentWorldObservation {
            hazard_index: 1.0,
            ..Default::default()
        };
        let mut snapshot = SensorySnapshot::from_profile(&profile());
        let baseline_filtering = snapshot.sensory_filtering;

        for _ in 0..10 {
            snapshot = snapshot.step(&body, &needs, &observation, 1.0);
        }

        assert!(snapshot.sensory_filtering < baseline_filtering);
        assert!(snapshot.attention_modulation > snapshot.baseline_attention_modulation);
    }
}
