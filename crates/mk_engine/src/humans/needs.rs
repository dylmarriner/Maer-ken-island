//! Needs Snapshot - Metabolic survival state (hunger, thirst, fatigue, glucose/ATP)
//!
//! Human bodies require food, water, and sleep. This module turns the
//! long-unused `mk_core::human::schema` bio-systems config (metabolic
//! baselines, drive sensitivities, processing rates) into a stepped runtime
//! snapshot, coupled to the actual world (local biome resource abundance,
//! ambient temperature) the same way `crate::agents::physiology` already
//! couples non-human agents to their surroundings.

use super::rates::DAY_YEARS;
use crate::agents::AgentWorldObservation;
use mk_core::human::HumanProfile;
use serde::{Deserialize, Serialize};

/// Fraction of the hydration reserve lost per day at rest in a temperate
/// climate. The reserve spans full hydration (1.0) to a lethal deficit
/// (0.0) of ~15% of body mass, ~10.5 L for a 70 kg adult (Adolph 1947);
/// obligatory losses at rest are ~2-2.5 L/day (IOM 2005). 2.1 / 10.5 = 0.2,
/// so a person without water dies in 4-5 days
/// (fixtures/reference/humans/physiology.json `survival_without_water`).
pub const HYDRATION_LOSS_PER_DAY: f64 = 0.2;
/// Most water a person can take in per day, as a fraction of the reserve,
/// with full access: drinking ~1 L/h for a waking day is ~10x the resting
/// loss (kidneys can clear up to ~0.7-1 L/h; Hew-Butler et al. 2015).
/// Access scales it, so even modest water access (a stream, rain) keeps
/// a person hydrated, as it does in reality; only near-zero access kills.
pub const MAX_DRINKING_PER_DAY: f64 = 10.0 * HYDRATION_LOSS_PER_DAY;
/// Air temperature above which sweating raises water loss (°C).
const SWEATING_ONSET_C: f64 = 25.0;
/// Extra water loss per °C above `SWEATING_ONSET_C`, as a multiple of the
/// resting loss: at 40 °C losses are ~2.5x resting (Adolph 1947, resting
/// desert subjects losing 5-6 L/day).
const SWEAT_LOSS_PER_C: f64 = 0.1;

/// Days of energy expenditure the glycogen store (`glucose`) holds: ~500 g
/// of liver and muscle glycogen, ~2,000 kcal, about one day's expenditure
/// (Cahill 2006, Annu. Rev. Nutr. 26:1).
pub const GLYCOGEN_DAYS: f64 = 1.0;
/// Days of energy expenditure the fat and mobilisable protein stores
/// (`energy_reserve`) hold for a normal-weight adult: ~12 kg fat at
/// 7,700 kcal/kg plus ~6 kg lean tissue at ~1,000 kcal/kg ≈ 98,000 kcal,
/// spent at a fasting expenditure of ~1,650 kcal/day (BMR falls ~20% in
/// starvation; Keys et al. 1950). Total starvation kills in 45-75 days
/// with water (Leiter & Marliss 1982; reference `survival_without_food`).
pub const STORED_ENERGY_DAYS: f64 = 60.0;
/// Most food energy a person can eat and absorb per day with full access,
/// in days of expenditure: refeeding and high-intake studies reach
/// ~4,000-5,000 kcal/day, about 3x a resting day's expenditure (Keys et
/// al. 1950). Foraging access scales it, so a moderately productive
/// biome feeds a person; only barren ground starves them.
pub const MAX_EATING_DAYS_PER_DAY: f64 = 3.0;
/// Glycogen level that fat and protein mobilisation (gluconeogenesis,
/// ketosis) defends while stores last: fasting humans stay hungry and weak
/// but functional until the stores run out.
const FASTING_GLUCOSE_FLOOR: f64 = 0.15;
/// Fraction of surplus food energy stored as fat (the rest is the
/// metabolic cost of storage; Flatt 1987).
const FAT_STORAGE_EFFICIENCY: f64 = 0.75;

fn default_energy_reserve() -> f64 {
    1.0
}

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
    /// Energy expenditure of the current activity as a multiple of the
    /// activity level the drain rates are calibrated for (light daily
    /// activity, ~1.5 MET). 1.0, or the zero a `Default` leaves, means
    /// the baseline and changes nothing; heavy labour scales the glucose
    /// drain and, through breathing and sweat, the water loss (see
    /// `regional::labour::effort_for_met`).
    #[serde(default)]
    pub activity: f64,
}

impl EffortFocus {
    /// No active resource focus — every gain term behaves exactly as
    /// before this parameter existed.
    pub fn none() -> Self {
        Self {
            food: 1.0,
            water: 1.0,
            shelter: 1.0,
            activity: 1.0,
        }
    }
}

/// Highest activity multiple the needs model accepts (~12 MET on a 1.5 MET
/// baseline).
pub const MAX_ACTIVITY_MULTIPLE: f64 = 8.0;
/// Extra water loss per unit of activity above the baseline: breathing and
/// sweat rise with metabolic rate, by roughly half as much.
const WATER_LOSS_PER_EXTRA_ACTIVITY: f64 = 0.5;

/// Runtime metabolic/survival snapshot for a human.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeedsSnapshot {
    /// Blood glucose / usable energy reserve (0.0 = depleted, 1.0 = full)
    pub glucose: f64,
    /// Hydration reserve (0.0 = dehydrated, 1.0 = fully hydrated)
    pub hydration: f64,
    /// Homeostatic sleep pressure (process S; 0.0 = rested, 1.0 =
    /// exhausted). Owned by [`super::circadian::CircadianClock::step`],
    /// which raises it while awake and dissipates it in sleep; `step` here
    /// leaves it unchanged.
    pub fatigue: f64,
    /// Fat and mobilisable protein stores (1.0 = normal stores, 0.0 =
    /// exhausted). They hold `STORED_ENERGY_DAYS` of expenditure; once they
    /// are gone the glycogen store empties and the human starves.
    #[serde(default = "default_energy_reserve")]
    pub energy_reserve: f64,
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
            energy_reserve: 1.0,
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
    /// Rates are real physiology (see the constants above): a human with
    /// baseline schema sensitivities, full physical capacity and no water
    /// dies of dehydration in 4-5 days; with water but no food they empty
    /// their glycogen in about a day, live on fat and protein for about 60
    /// days, then starve. Food access above ~0.35 keeps glycogen topped up;
    /// water access above ~0.1 is enough to stay hydrated at rest. Rates are integrated over `dt_years`, so step length
    /// does not change the outcome.
    pub fn step(
        &self,
        observation: &AgentWorldObservation,
        physical_capacity: f64,
        effort: EffortFocus,
        dt_years: f64,
    ) -> Self {
        // Elapsed time in days: every rate below is per day.
        let dt = dt_years.max(0.0) / DAY_YEARS;
        let capacity = physical_capacity.clamp(0.05, 1.0);

        const HUNGER_SENSITIVITY_BASELINE: f64 = 0.65;
        const THIRST_SENSITIVITY_BASELINE: f64 = 0.70;
        // Canon metabolic baselines the drain is calibrated against: a
        // human turning over ATP faster burns glucose faster, one converting
        // glucose to ATP more efficiently burns less for the same work.
        const ATP_CONSUMPTION_BASELINE: f64 = 0.085;
        const GLUCOSE_ATP_CONVERSION_BASELINE: f64 = 0.92;
        let metabolic_demand = (self.atp_consumption_rate / ATP_CONSUMPTION_BASELINE)
            .clamp(0.25, 4.0)
            / (self.glucose_atp_conversion / GLUCOSE_ATP_CONVERSION_BASELINE).clamp(0.25, 4.0);

        // Work above the baseline activity burns proportionally more.
        let activity = effort.activity.clamp(1.0, MAX_ACTIVITY_MULTIPLE);
        let glucose_drain = (self.hunger_sensitivity / HUNGER_SENSITIVITY_BASELINE)
            * metabolic_demand
            * activity
            * dt
            / GLYCOGEN_DAYS;
        let glucose_gain = observation.caloric_access
            * MAX_EATING_DAYS_PER_DAY
            * capacity
            * effort.food.clamp(1.0, 1.5)
            * dt
            / GLYCOGEN_DAYS;
        let mut glucose = self.glucose - glucose_drain + glucose_gain;
        let mut energy_reserve = self.energy_reserve.clamp(0.0, 1.0);
        // One unit of glycogen is this fraction of the fat/protein stores.
        let glycogen_per_reserve = STORED_ENERGY_DAYS / GLYCOGEN_DAYS;
        if glucose > 1.0 {
            // Surplus beyond full glycogen is laid down as fat.
            energy_reserve = (energy_reserve
                + (glucose - 1.0) * FAT_STORAGE_EFFICIENCY / glycogen_per_reserve)
                .min(1.0);
        } else if glucose < FASTING_GLUCOSE_FLOOR && energy_reserve > 0.0 {
            // Fasting: fat and protein are mobilised to hold the floor.
            let wanted = FASTING_GLUCOSE_FLOOR - glucose;
            let drawn = (wanted / glycogen_per_reserve).min(energy_reserve);
            energy_reserve -= drawn;
            glucose += drawn * glycogen_per_reserve;
        }
        let glucose = glucose.clamp(0.0, 1.0);

        let heat = (observation.ambient_temperature_c - SWEATING_ONSET_C).max(0.0);
        let water_loss_per_day = HYDRATION_LOSS_PER_DAY
            * (1.0 + SWEAT_LOSS_PER_C * heat)
            * (1.0 + WATER_LOSS_PER_EXTRA_ACTIVITY * (activity - 1.0));
        let thirst_drain =
            (self.thirst_sensitivity / THIRST_SENSITIVITY_BASELINE) * water_loss_per_day * dt;
        let hydration_gain = observation.hydration_access
            * capacity
            * effort.water.clamp(1.0, 1.5)
            * MAX_DRINKING_PER_DAY
            * dt;
        let hydration = (self.hydration - thirst_drain + hydration_gain).clamp(0.0, 1.0);

        let fatigue = self.fatigue;

        let hunger = (1.0 - glucose).clamp(0.0, 1.0);
        let thirst = (1.0 - hydration).clamp(0.0, 1.0);

        let status = if glucose < 0.1 || hydration < 0.1 || fatigue > 0.95 || energy_reserve < 0.2 {
            SurvivalStatus::Critical
        } else if glucose < 0.35 || hydration < 0.35 || fatigue > 0.75 || energy_reserve < 0.5 {
            SurvivalStatus::Strained
        } else {
            SurvivalStatus::Nourished
        };

        Self {
            glucose,
            hydration,
            fatigue,
            energy_reserve,
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

    /// How much faster than baseline this human's sleep pressure builds:
    /// the canon fatigue and somnolence drive sensitivities relative to
    /// the defaults, bounded to the ±30% spread of human sleep need
    /// (7-9 h around 8 h; Hirshkowitz et al. 2015).
    pub fn sleep_pressure_rate_factor(&self) -> f64 {
        const BASELINE: f64 = 0.72 * 0.45;
        (self.fatigue_sensitivity * self.somnolence_sensitivity / BASELINE).clamp(0.75, 1.3)
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
            activity: 1.0,
        };

        // Hourly steps: over longer ones both humans would fill their
        // glycogen stores and the difference would vanish.
        let hour = crate::humans::rates::HOUR_YEARS;
        let mut unfocused = NeedsSnapshot::from_profile(&profile());
        let mut food_focused = NeedsSnapshot::from_profile(&profile());
        for _ in 0..5 {
            unfocused = unfocused.step(&observation, 1.0, EffortFocus::none(), hour);
            food_focused = food_focused.step(&observation, 1.0, focused, hour);
        }

        assert!(food_focused.glucose > unfocused.glucose);
        // Focus on food specifically shouldn't also boost hydration.
        assert!((food_focused.hydration - unfocused.hydration).abs() < 1e-9);
    }
}
