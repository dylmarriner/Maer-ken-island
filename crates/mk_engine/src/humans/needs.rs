//! Needs Snapshot - Metabolic survival state (hunger, thirst, fatigue, glucose/ATP)
//!
//! Human bodies require food, water, and sleep. This module turns the
//! long-unused `mk_core::human::schema` bio-systems config (metabolic
//! baselines, drive sensitivities, processing rates) into a stepped runtime
//! snapshot, coupled to the actual world (local biome resource abundance,
//! ambient temperature) the same way `crate::agents::physiology` already
//! couples non-human agents to their surroundings.

use crate::agents::AgentWorldObservation;
use mk_core::human::HumanProfile;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SurvivalStatus {
    Nourished,
    Strained,
    Critical,
}

/// Per-resource effort multiplier applied to `NeedsSnapshot::step`'s gain
/// terms — the real effect of a human actively focusing on acquiring a
/// resource this tick vs. not trying at all. Each field is a bounded
/// multiplier ([1.0, 1.5] once clamped by `step`); 1.0 means "no extra
/// effort applied," matching prior behavior exactly.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct EffortFocus {
    pub food: f64,
    pub water: f64,
    pub shelter: f64,
}

impl EffortFocus {
    /// No active resource focus — every gain term behaves exactly as
    /// before this parameter existed.
    pub fn none() -> Self {
        Self {
            food: 1.0,
            water: 1.0,
            shelter: 1.0,
        }
    }
}

/// Runtime metabolic/survival snapshot for a human.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeedsSnapshot {
    /// Blood glucose / usable energy reserve (0.0 = depleted, 1.0 = full)
    pub glucose: f64,
    /// Hydration reserve (0.0 = dehydrated, 1.0 = fully hydrated)
    pub hydration: f64,
    /// Accumulated sleep debt / fatigue (0.0 = rested, 1.0 = exhausted)
    pub fatigue: f64,
    /// Subjective hunger drive, rises as glucose depletes
    pub hunger: f64,
    /// Subjective thirst drive, rises as hydration depletes
    pub thirst: f64,
    pub status: SurvivalStatus,

    // Per-individual sensitivities/rates, taken from the canonical schema's
    // `core_systems.biosys` block when present (else sane defaults).
    hunger_sensitivity: f64,
    thirst_sensitivity: f64,
    fatigue_sensitivity: f64,
    somnolence_sensitivity: f64,
    atp_consumption_rate: f64,
    glucose_atp_conversion: f64,
}

impl NeedsSnapshot {
    pub fn from_profile(profile: &HumanProfile) -> Self {
        let (hunger_sensitivity, thirst_sensitivity, fatigue_sensitivity, somnolence_sensitivity) =
            profile
                .canonical_schema()
                .map(|schema| {
                    let d = &schema.core_systems.biosys.drive_sensitivities;
                    (
                        d.hunger as f64,
                        d.thirst as f64,
                        d.fatigue as f64,
                        d.somnolence as f64,
                    )
                })
                .unwrap_or((0.65, 0.70, 0.72, 0.45));

        let (glucose, atp_consumption_rate, glucose_atp_conversion) = profile
            .canonical_schema()
            .map(|schema| {
                let m = &schema.core_systems.biosys.metabolic_baselines;
                let p = &schema.core_systems.biosys.processing_rates;
                let or_baseline = |value: f32, baseline: f64| {
                    if value > 0.0 {
                        value as f64
                    } else {
                        baseline
                    }
                };
                (
                    m.glucose as f64,
                    or_baseline(p.atp_consumption_rate, 0.085),
                    or_baseline(p.glucose_atp_conversion, 0.92),
                )
            })
            .unwrap_or((0.68, 0.085, 0.92));

        Self {
            glucose,
            hydration: 0.8,
            fatigue: 0.0,
            hunger: (1.0 - glucose).clamp(0.0, 1.0),
            thirst: 0.2,
            status: SurvivalStatus::Nourished,
            hunger_sensitivity,
            thirst_sensitivity,
            fatigue_sensitivity,
            somnolence_sensitivity,
            atp_consumption_rate,
            glucose_atp_conversion,
        }
    }

    /// Step needs forward by `dt_years` using local environmental access
    /// (biome resource abundance, ambient temperature) sampled from the
    /// world at the human's position — the same observation type already
    /// used to couple agents to the world.
    ///
    /// `physical_capacity` (see [`super::physical_capacity`]) multiplies
    /// the gain terms only — a worn-down body still benefits from abundant
    /// access, just converts it into real nourishment less effectively.
    /// Pass `1.0` for full effectiveness (the default/no-op case, and what
    /// every pre-existing caller/test in this module still uses).
    ///
    /// `effort` bounds a per-resource focus multiplier on the three gain
    /// terms (glucose/hydration/fatigue-recovery respectively) — see
    /// [`EffortFocus`]. Passing `EffortFocus::none()` reproduces prior
    /// behavior exactly (every pre-existing caller/test uses this). This
    /// module intentionally has no knowledge of *why* a human is focused on
    /// a resource (that's `super::autonomy::AutonomousMind`'s job, whose
    /// chosen action `lifecycle::step_lifecycle` turns into this focus, and
    /// which reads this same `hunger`/`thirst`/`fatigue` state when
    /// choosing) — it only applies the bounded
    /// effect of active effort once decided, keeping this module's
    /// dependency direction one-way.
    ///
    /// Rates are calibrated per-year: a human with baseline schema
    /// sensitivities (`hunger`/`thirst` ~0.65-0.70), full physical capacity,
    /// no active resource focus, and no caloric/hydration access at all
    /// fully depletes glucose/hydration within about a year, while
    /// "abundant" access (>~0.8) keeps them topped up indefinitely. This
    /// holds regardless of how large or small a single `dt_years` step is,
    /// since it's a linear rate integrated over time, not a per-tick fixed
    /// decrement.
    pub fn step(
        &self,
        observation: &AgentWorldObservation,
        physical_capacity: f64,
        effort: EffortFocus,
        dt_years: f64,
    ) -> Self {
        let dt = dt_years.max(0.0);
        let capacity = physical_capacity.clamp(0.05, 1.0);

        const HUNGER_SENSITIVITY_BASELINE: f64 = 0.65;
        const THIRST_SENSITIVITY_BASELINE: f64 = 0.70;
        const GAIN_MULTIPLIER: f64 = 1.3;
        // Canon metabolic baselines the drain is calibrated against: a
        // human turning over ATP faster burns glucose faster, one converting
        // glucose to ATP more efficiently burns less for the same work.
        const ATP_CONSUMPTION_BASELINE: f64 = 0.085;
        const GLUCOSE_ATP_CONVERSION_BASELINE: f64 = 0.92;
        let metabolic_demand = (self.atp_consumption_rate / ATP_CONSUMPTION_BASELINE)
            .clamp(0.25, 4.0)
            / (self.glucose_atp_conversion / GLUCOSE_ATP_CONVERSION_BASELINE).clamp(0.25, 4.0);

        let glucose_drain =
            (self.hunger_sensitivity / HUNGER_SENSITIVITY_BASELINE) * metabolic_demand * dt;
        let glucose_gain = observation.caloric_access
            * GAIN_MULTIPLIER
            * capacity
            * effort.food.clamp(1.0, 1.5)
            * dt;
        let glucose = (self.glucose - glucose_drain + glucose_gain).clamp(0.0, 1.0);

        let thirst_drain = (self.thirst_sensitivity / THIRST_SENSITIVITY_BASELINE) * dt;
        let hydration_gain = observation.hydration_access
            * GAIN_MULTIPLIER
            * capacity
            * effort.water.clamp(1.0, 1.5)
            * dt;
        let hydration = (self.hydration - thirst_drain + hydration_gain).clamp(0.0, 1.0);

        let fatigue_accum = self.fatigue_sensitivity * self.somnolence_sensitivity * dt;
        let fatigue_recovery = observation.shelter_quality * effort.shelter.clamp(1.0, 1.5) * dt;
        let fatigue = (self.fatigue + fatigue_accum - fatigue_recovery).clamp(0.0, 1.0);

        let hunger = (1.0 - glucose).clamp(0.0, 1.0);
        let thirst = (1.0 - hydration).clamp(0.0, 1.0);

        let status = if glucose < 0.1 || hydration < 0.1 || fatigue > 0.95 {
            SurvivalStatus::Critical
        } else if glucose < 0.35 || hydration < 0.35 || fatigue > 0.75 {
            SurvivalStatus::Strained
        } else {
            SurvivalStatus::Nourished
        };

        Self {
            glucose,
            hydration,
            fatigue,
            hunger,
            thirst,
            status,
            hunger_sensitivity: self.hunger_sensitivity,
            thirst_sensitivity: self.thirst_sensitivity,
            fatigue_sensitivity: self.fatigue_sensitivity,
            somnolence_sensitivity: self.somnolence_sensitivity,
            atp_consumption_rate: self.atp_consumption_rate,
            glucose_atp_conversion: self.glucose_atp_conversion,
        }
    }

    /// Fraction of glucose this human converts into usable ATP.
    pub fn glucose_atp_conversion(&self) -> f64 {
        self.glucose_atp_conversion
    }

    /// Whether the human should die of starvation/dehydration this tick.
    pub fn is_lethal(&self) -> bool {
        self.glucose <= 0.0 || self.hydration <= 0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mk_core::human::{HumanId, HumanSchema};

    fn profile() -> HumanProfile {
        let schema = HumanSchema::canonical_minimal("needs_test");
        HumanProfile::from_canonical_schema(HumanId::new(1), schema)
    }

    #[test]
    fn from_profile_uses_schema_metabolic_baseline() {
        let snapshot = NeedsSnapshot::from_profile(&profile());
        assert!(snapshot.glucose > 0.0 && snapshot.glucose <= 1.0);
        assert!(matches!(snapshot.status, SurvivalStatus::Nourished));
    }

    #[test]
    fn starvation_depletes_glucose_without_food_access() {
        let snapshot = NeedsSnapshot::from_profile(&profile());
        let starved_observation = AgentWorldObservation {
            caloric_access: 0.0,
            hydration_access: 0.0,
            shelter_quality: 0.0,
            ..AgentWorldObservation::default()
        };

        let mut current = snapshot;
        for _ in 0..50 {
            current = current.step(&starved_observation, 1.0, EffortFocus::none(), 1.0);
        }

        assert!(current.glucose < 0.3);
        assert!(matches!(
            current.status,
            SurvivalStatus::Strained | SurvivalStatus::Critical
        ));
    }

    #[test]
    fn well_fed_environment_keeps_needs_nourished() {
        let snapshot = NeedsSnapshot::from_profile(&profile());
        let abundant_observation = AgentWorldObservation {
            caloric_access: 1.0,
            hydration_access: 1.0,
            shelter_quality: 1.0,
            ..AgentWorldObservation::default()
        };

        let mut current = snapshot;
        for _ in 0..20 {
            current = current.step(&abundant_observation, 1.0, EffortFocus::none(), 1.0);
        }

        assert!(matches!(current.status, SurvivalStatus::Nourished));
        assert!(!current.is_lethal());
    }

    #[test]
    fn low_physical_capacity_gains_less_from_identical_access() {
        let observation = AgentWorldObservation {
            caloric_access: 1.0,
            hydration_access: 1.0,
            shelter_quality: 1.0,
            ..AgentWorldObservation::default()
        };

        let mut capable = NeedsSnapshot::from_profile(&profile());
        let mut incapacitated = NeedsSnapshot::from_profile(&profile());
        for _ in 0..5 {
            capable = capable.step(&observation, 1.0, EffortFocus::none(), 0.5);
            incapacitated = incapacitated.step(&observation, 0.1, EffortFocus::none(), 0.5);
        }

        assert!(incapacitated.glucose < capable.glucose);
        assert!(incapacitated.hydration < capable.hydration);
    }

    #[test]
    fn active_food_focus_gains_more_glucose_than_no_focus_with_identical_access() {
        let observation = AgentWorldObservation {
            caloric_access: 0.5,
            hydration_access: 0.5,
            shelter_quality: 0.5,
            ..AgentWorldObservation::default()
        };
        let focused = EffortFocus {
            food: 1.5,
            water: 1.0,
            shelter: 1.0,
        };

        let mut unfocused = NeedsSnapshot::from_profile(&profile());
        let mut food_focused = NeedsSnapshot::from_profile(&profile());
        for _ in 0..5 {
            unfocused = unfocused.step(&observation, 1.0, EffortFocus::none(), 0.5);
            food_focused = food_focused.step(&observation, 1.0, focused, 0.5);
        }

        assert!(food_focused.glucose > unfocused.glucose);
        // Focus on food specifically shouldn't also boost hydration.
        assert!((food_focused.hydration - unfocused.hydration).abs() < 1e-9);
    }
}
