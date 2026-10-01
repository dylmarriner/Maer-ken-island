//! Development Snapshot - Age, maturity, and lifecycle stage tracking
//!
//! When a [`HumanProfile`](mk_core::human::HumanProfile) carries a canonical
//! [`HumanSchema`](mk_core::human::HumanSchema), initial age and maturity can be
//! taken from `extreme_brain_detail.brain_dynamics_module.developmental_timeline`
//! so simulation layers stay aligned with schema-authored lifecycle data.

use mk_core::human::HumanProfile;
use serde::{Deserialize, Serialize};

/// Developmental stage of a human
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DevelopmentStage {
    Infant,
    Child,
    Adolescent,
    Adult,
    Elder,
}

impl DevelopmentStage {
    pub fn from_age(age_years: f64) -> Self {
        if age_years < 2.0 {
            DevelopmentStage::Infant
        } else if age_years < 13.0 {
            DevelopmentStage::Child
        } else if age_years < 20.0 {
            DevelopmentStage::Adolescent
        } else if age_years < 65.0 {
            DevelopmentStage::Adult
        } else {
            DevelopmentStage::Elder
        }
    }
}

/// Runtime development snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DevelopmentSnapshot {
    /// Current age in years (fractional)
    pub age_years: f64,

    /// Maturity index (0.0 = newborn, 1.0 = fully mature)
    pub maturity_index: f64,

    /// Current developmental stage
    pub stage: DevelopmentStage,

    /// Cognitive development factor (0.0 to 1.0)
    pub cognitive_development: f64,

    /// Physical development factor (0.0 to 1.0)
    pub physical_development: f64,

    /// Emotional regulation maturity (0.0 to 1.0)
    pub emotional_maturity: f64,

    /// Social competence development (0.0 to 1.0)
    pub social_development: f64,
}

impl DevelopmentSnapshot {
    /// Create a new development snapshot at a given age
    pub fn new(age_years: f64) -> Self {
        Self::from_age_years(age_years)
    }

    /// Build a snapshot from explicit age (deterministic curves).
    fn from_age_years(age_years: f64) -> Self {
        let stage = DevelopmentStage::from_age(age_years);

        let maturity_index = Self::compute_maturity(age_years);
        let cognitive = Self::compute_cognitive_development(age_years);
        let physical = Self::compute_physical_development(age_years);
        let emotional = Self::compute_emotional_maturity(age_years);
        let social = Self::compute_social_development(age_years);

        Self {
            age_years,
            maturity_index,
            stage,
            cognitive_development: cognitive,
            physical_development: physical,
            emotional_maturity: emotional,
            social_development: social,
        }
    }

    /// Initial development state from a profile, consuming schema timeline when present.
    ///
    /// Uses `developmental_timeline.current_age` when it is **> 0**; otherwise falls back
    /// to **25.0** years (historical `HumanBeing::from_profile` default). When
    /// `developmental_progress` is **> 0**, it overrides `maturity_index` (clamped to 0..=1).
    pub fn from_profile(profile: &HumanProfile) -> Self {
        let mut timeline_age: Option<f64> = None;
        let mut progress_override: Option<f32> = None;

        if let Some(schema) = profile.canonical_schema() {
            let timeline = &schema
                .extreme_brain_detail
                .brain_dynamics_module
                .developmental_timeline;
            if timeline.current_age > 0.0 {
                timeline_age = Some(timeline.current_age as f64);
            }
            if timeline.developmental_progress > 0.0 {
                progress_override = Some(timeline.developmental_progress);
            }
        }

        let age_years = timeline_age.unwrap_or(25.0);
        let mut snap = Self::from_age_years(age_years);
        if let Some(p) = progress_override {
            snap.maturity_index = (p as f64).clamp(0.0, 1.0);
        }
        snap
    }

    /// Step development forward by dt_years
    pub fn step(&mut self, dt_years: f64) {
        self.age_years += dt_years;
        self.recompute_from_age();
    }

    /// Recompute every derived field from the current `age_years`, without
    /// changing the age itself. Used by [`super::HumanBeing::refresh_phase11_layers`]
    /// so `development` stays consistent with itself after a profile mutation
    /// that does not itself change age (e.g. a neurotype field flip), keeping
    /// this snapshot in line with the other six pure-projection snapshots
    /// instead of silently going stale.
    pub fn recompute_from_age(&mut self) {
        self.stage = DevelopmentStage::from_age(self.age_years);
        self.maturity_index = Self::compute_maturity(self.age_years);
        self.cognitive_development = Self::compute_cognitive_development(self.age_years);
        self.physical_development = Self::compute_physical_development(self.age_years);
        self.emotional_maturity = Self::compute_emotional_maturity(self.age_years);
        self.social_development = Self::compute_social_development(self.age_years);
    }

    /// Overall maturity: sigmoid curve reaching 0.95 by age 25
    fn compute_maturity(age: f64) -> f64 {
        // Sigmoid centered at age 12, reaching ~0.95 by 25
        let sigmoid = 1.0 / (1.0 + (-0.4 * (age - 12.0)).exp());
        (sigmoid * 1.05).min(1.0)
    }

    /// Cognitive development: rapid early growth, plateau around 20
    fn compute_cognitive_development(age: f64) -> f64 {
        if age <= 0.0 {
            return 0.05;
        }
        // Logarithmic growth: fast early, slow later
        ((age.ln() + 1.0) / 4.0).clamp(0.0, 1.0)
    }

    /// Physical development: peaks around 25, gradual decline after 50
    fn compute_physical_development(age: f64) -> f64 {
        if age <= 0.0 {
            return 0.1;
        }
        if age <= 25.0 {
            age / 25.0
        } else if age <= 50.0 {
            1.0
        } else {
            1.0 - ((age - 50.0) / 100.0).min(0.4)
        }
    }

    /// Emotional maturity: develops slower, continues improving into 40s
    fn compute_emotional_maturity(age: f64) -> f64 {
        if age <= 0.0 {
            return 0.02;
        }
        ((age / 40.0).powf(0.6)).clamp(0.0, 1.0)
    }

    /// Social development: accelerates in childhood/adolescence
    fn compute_social_development(age: f64) -> f64 {
        if age <= 0.0 {
            return 0.0;
        }
        if age <= 5.0 {
            age / 5.0 * 0.3
        } else if age <= 18.0 {
            0.3 + (age - 5.0) / 13.0 * 0.5
        } else {
            (0.8 + (age - 18.0) / 30.0 * 0.2).min(1.0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mk_core::human::{HumanId, HumanSchema};

    #[test]
    fn development_at_25_is_mature() {
        let dev = DevelopmentSnapshot::new(25.0);
        assert!(dev.maturity_index > 0.9);
        assert_eq!(dev.stage, DevelopmentStage::Adult);
    }

    #[test]
    fn development_steps_forward() {
        let mut dev = DevelopmentSnapshot::new(10.0);
        assert_eq!(dev.stage, DevelopmentStage::Child);
        dev.step(5.0);
        assert_eq!(dev.stage, DevelopmentStage::Adolescent);
    }

    #[test]
    fn development_from_profile_uses_schema_timeline_age() {
        let mut schema = HumanSchema::canonical_minimal("timeline_human");
        schema
            .extreme_brain_detail
            .brain_dynamics_module
            .developmental_timeline
            .current_age = 8.0;

        let profile = mk_core::human::HumanProfile::from_canonical_schema(HumanId::new(3), schema);
        let dev = DevelopmentSnapshot::from_profile(&profile);

        assert!((dev.age_years - 8.0).abs() < f64::EPSILON);
        assert_eq!(dev.stage, DevelopmentStage::Child);
    }

    #[test]
    fn development_from_profile_defaults_age_when_timeline_zero() {
        let schema = HumanSchema::canonical_minimal("default_age_human");
        let profile = mk_core::human::HumanProfile::from_canonical_schema(HumanId::new(4), schema);
        let dev = DevelopmentSnapshot::from_profile(&profile);

        assert!((dev.age_years - 25.0).abs() < f64::EPSILON);
        assert_eq!(dev.stage, DevelopmentStage::Adult);
    }

    #[test]
    fn development_from_profile_applies_progress_override() {
        let mut schema = HumanSchema::canonical_minimal("progress_human");
        let timeline = &mut schema
            .extreme_brain_detail
            .brain_dynamics_module
            .developmental_timeline;
        timeline.current_age = 30.0;
        timeline.developmental_progress = 0.55;

        let profile = mk_core::human::HumanProfile::from_canonical_schema(HumanId::new(5), schema);
        let dev = DevelopmentSnapshot::from_profile(&profile);

        assert!((dev.maturity_index - 0.55).abs() < 1e-6);
    }
}
