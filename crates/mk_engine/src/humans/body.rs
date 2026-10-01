//! Body Snapshot - vitals, internal physiology, and appearance.
//!
//! Surfaces `docs/canon/HumanReplicationSchema.js`'s `body` block
//! (`BodyVitalsSchema`, `BodyPhysiologySchema`, `BodyAppearanceSchema`),
//! ported in `mk_core::human::schema` but previously unread by the engine.
//! Vitals/physiology are stepped each tick coupled to [`super::needs`]
//! (the two overlap deliberately: `needs` is the survival-drive layer,
//! `body` is the raw physiological readout that drives it and that other
//! systems read). Injury from harm persists in `injury` and heals over
//! time.

use mk_core::human::HumanProfile;
use serde::{Deserialize, Serialize};

/// Runtime body snapshot: vitals + internal physiology + static appearance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BodySnapshot {
    // --- vitals ---
    pub pulse: f64,
    pub blood_pressure_systolic: f64,
    pub blood_pressure_diastolic: f64,
    pub sp_o2: f64,
    pub body_temperature_c: f64,
    pub vital_glucose: f64,
    pub vital_energy: f64,
    pub vital_fatigue: f64,
    pub arousal: f64,
    pub tension: f64,
    /// Resting blood pressure from canon, which the live readings deviate
    /// from under tension and cardiac load.
    #[serde(default = "default_resting_systolic")]
    pub resting_systolic: f64,
    #[serde(default = "default_resting_diastolic")]
    pub resting_diastolic: f64,

    // --- physiology ---
    pub bladder_pressure: f64,
    pub bowel_pressure: f64,
    pub hygiene: f64,
    pub cortisol: f64,
    pub oxytocin: f64,
    pub dopamine: f64,
    pub melatonin: f64,
    pub testosterone: f64,
    pub estrogen: f64,
    pub atp: f64,
    pub calorie_intake: f64,
    pub calorie_burn: f64,

    // --- appearance (static, schema-authored) ---
    pub height_cm: f64,
    pub weight_kg: f64,
    pub build: String,
    pub hair_color: String,
    pub eye_color: String,
    /// Authored skin tone. The canonical schema has no pigmentation field, so
    /// schema-built humans (and snapshots written before this field existed)
    /// get the neutral default; Foundry-spawned humans carry their own.
    #[serde(default = "default_skin_tone")]
    pub skin_tone: String,

    // --- trauma ---
    /// Accumulated tissue damage, `0..=1` (1 is not survivable). Raised by
    /// harm, it caps vital energy and raises pulse until it heals; healing
    /// needs nourishment and water. Zero for snapshots written before
    /// injuries persisted.
    #[serde(default)]
    pub injury: f64,
}

/// Healing rate of a fully nourished, hydrated body, per year of exponential
/// recovery: a half-life of about ten days for soft-tissue damage.
pub const INJURY_HEALING_PER_YEAR: f64 = 25.0;

impl BodySnapshot {
    /// Add tissue damage from harm of `severity` (`0..=1`).
    pub fn wound(&mut self, severity: f64) {
        self.injury = (self.injury + severity.clamp(0.0, 1.0) * 0.4).clamp(0.0, 1.0);
        self.vital_energy = self.vital_energy.min(1.0 - self.injury);
    }

    /// Whether accumulated injury or exhausted energy is fatal.
    pub fn is_fatally_injured(&self) -> bool {
        self.injury >= 1.0 || self.vital_energy <= 0.0
    }
}

fn default_skin_tone() -> String {
    "medium".to_string()
}

/// What the body is exposed to this tick.
#[derive(Debug, Clone, Copy)]
pub struct BodyEnvironment {
    /// 0..1 quality of the shelter the human has.
    pub shelter_quality: f64,
    /// Air temperature where the human stands (°C).
    pub ambient_temperature_c: f64,
    /// 0..1 systemic inflammation from the immune system.
    pub inflammation: f64,
}

/// Healthy core-temperature set-point (°C).
const CORE_SET_POINT_C: f64 = 37.0;
/// Peak fever at full systemic inflammation (°C above the set-point).
const MAX_FEVER_C: f64 = 2.5;
/// Effective ambient range thermoregulation fully compensates for (°C).
const THERMONEUTRAL_LOW_C: f64 = 10.0;
const THERMONEUTRAL_HIGH_C: f64 = 32.0;
/// Core cooling/warming per °C of effective ambient beyond that range.
const COLD_LOAD_PER_C: f64 = 0.15;
const HEAT_LOAD_PER_C: f64 = 0.12;
/// Indoor temperature a perfect shelter holds its occupant at (°C).
const SHELTERED_TEMPERATURE_C: f64 = 22.0;
/// Time constant of core temperature (hours).
const CORE_TEMPERATURE_TIME_CONSTANT_HOURS: f64 = 6.0;
/// Core temperatures beyond which the body cannot survive (°C).
pub const LETHAL_HYPOTHERMIA_C: f64 = 28.0;
pub const LETHAL_HYPERTHERMIA_C: f64 = 42.0;

/// Rate (per year) at which acute tension relaxes back toward calm.
const TENSION_RECOVERY_PER_YEAR: f64 = 26.0;

fn default_resting_systolic() -> f64 {
    120.0
}

fn default_resting_diastolic() -> f64 {
    80.0
}

impl BodySnapshot {
    pub fn from_profile(profile: &HumanProfile) -> Self {
        let Some(schema) = profile.canonical_schema() else {
            return Self::defaults();
        };
        let vitals = &schema.body.vitals;
        let physiology = &schema.body.physiology;
        let appearance = &schema.body.appearance;

        Self {
            pulse: nonzero_or(vitals.pulse as f64, 70.0),
            blood_pressure_systolic: nonzero_or(vitals.blood_pressure.systolic as f64, 120.0),
            blood_pressure_diastolic: nonzero_or(vitals.blood_pressure.diastolic as f64, 80.0),
            sp_o2: nonzero_or(vitals.sp_o2 as f64, 0.98),
            body_temperature_c: nonzero_or(vitals.temperature as f64, 37.0),
            vital_glucose: vitals.glucose as f64,
            vital_energy: nonzero_or(vitals.energy as f64, 0.8),
            vital_fatigue: vitals.fatigue as f64,
            arousal: vitals.arousal as f64,
            tension: vitals.tension as f64,
            resting_systolic: nonzero_or(vitals.blood_pressure.systolic as f64, 120.0),
            resting_diastolic: nonzero_or(vitals.blood_pressure.diastolic as f64, 80.0),

            bladder_pressure: physiology.waste_pressure.bladder as f64,
            bowel_pressure: physiology.waste_pressure.bowel as f64,
            hygiene: nonzero_or(physiology.hygiene as f64, 1.0),
            cortisol: physiology.hormones.cortisol as f64,
            oxytocin: physiology.hormones.oxytocin as f64,
            dopamine: physiology.hormones.dopamine as f64,
            melatonin: physiology.hormones.melatonin as f64,
            testosterone: physiology.hormones.testosterone as f64,
            estrogen: physiology.hormones.estrogen as f64,
            atp: nonzero_or(physiology.metabolism.atp as f64, 0.8),
            calorie_intake: physiology.metabolism.calorie_intake as f64,
            calorie_burn: physiology.metabolism.calorie_burn as f64,

            height_cm: nonzero_or(appearance.height as f64, 170.0),
            weight_kg: nonzero_or(appearance.weight as f64, 70.0),
            build: non_empty_or(&appearance.build, "average"),
            hair_color: non_empty_or(&appearance.hair_color, "brown"),
            eye_color: non_empty_or(&appearance.eye_color, "brown"),
            skin_tone: default_skin_tone(),
            injury: 0.0,
        }
    }

    /// Step vitals/physiology forward using the same `NeedsSnapshot` this
    /// tick just computed, so the two stay consistent: glucose/hydration
    /// pressure feeds pulse/energy/fatigue readouts, bladder/bowel pressure
    /// accumulates over time, and waste products decay with hygiene access.
    pub fn step(
        &self,
        needs: &super::needs::NeedsSnapshot,
        dt_years: f64,
        environment: BodyEnvironment,
    ) -> Self {
        let dt = dt_years.max(0.0);
        let shelter_quality = environment.shelter_quality;

        // Injuries heal over time, faster when the body has fuel and water
        // to rebuild with; until they do, they cap available energy.
        let nourishment = (needs.glucose.min(needs.hydration)).clamp(0.0, 1.0);
        let injury = self.injury * (-INJURY_HEALING_PER_YEAR * nourishment * dt).exp();
        let tension = (self.tension * (-TENSION_RECOVERY_PER_YEAR * dt).exp()).clamp(0.0, 1.0);
        let vital_energy = needs.glucose.min(1.0 - injury);
        let vital_fatigue = needs.fatigue;
        let vital_glucose = needs.glucose;

        // Pulse rises under low energy/high fatigue (physiological stress
        // response) and with injury (pain, blood loss).
        let pulse = (70.0 + (1.0 - vital_energy) * 30.0 + vital_fatigue * 20.0 + injury * 25.0)
            .clamp(40.0, 180.0);
        // Blood pressure departs from its resting value with sympathetic
        // tension and cardiac load.
        let blood_pressure_systolic =
            (self.resting_systolic + tension * 25.0 + (pulse - 70.0) * 0.3).clamp(70.0, 220.0);
        let blood_pressure_diastolic =
            (self.resting_diastolic + tension * 12.0 + (pulse - 70.0) * 0.15).clamp(40.0, 130.0);

        // SpO2 dips slightly under severe glucose/hydration depletion.
        let sp_o2 = (0.98 - (1.0 - needs.hydration) * 0.05).clamp(0.7, 1.0);

        // Waste pressure accumulates over time, relieved by hygiene access
        // (reusing shelter_quality as a proxy for having somewhere to go).
        let bladder_pressure =
            (self.bladder_pressure + 0.4 * dt - shelter_quality * 0.4 * dt).clamp(0.0, 1.0);
        let bowel_pressure =
            (self.bowel_pressure + 0.15 * dt - shelter_quality * 0.15 * dt).clamp(0.0, 1.0);
        let hygiene = (self.hygiene - 0.2 * dt + shelter_quality * 0.3 * dt).clamp(0.0, 1.0);

        // Thermoregulation: shelter moderates exposure, the body holds its
        // (fever-raised) set-point across the thermoneutral range, and
        // beyond it the core drifts toward the cold or heat load within
        // hours.
        let exposure = environment.ambient_temperature_c
            + (SHELTERED_TEMPERATURE_C - environment.ambient_temperature_c)
                * shelter_quality.clamp(0.0, 1.0);
        let cold_load = (THERMONEUTRAL_LOW_C - exposure).max(0.0) * COLD_LOAD_PER_C;
        let heat_load = (exposure - THERMONEUTRAL_HIGH_C).max(0.0) * HEAT_LOAD_PER_C;
        let target_core = CORE_SET_POINT_C + MAX_FEVER_C * environment.inflammation.clamp(0.0, 1.0)
            - cold_load
            + heat_load;
        let dt_hours = dt * 365.25 * 24.0;
        let core_blend = 1.0 - (-dt_hours / CORE_TEMPERATURE_TIME_CONSTANT_HOURS).exp();
        let body_temperature_c =
            self.body_temperature_c + (target_core - self.body_temperature_c) * core_blend;

        // Cellular energy charge: glucose reserves converted at this
        // human's own glucose→ATP efficiency.
        let atp = (needs.glucose * needs.glucose_atp_conversion()).clamp(0.0, 1.0);

        Self {
            pulse,
            atp,
            body_temperature_c,
            blood_pressure_systolic,
            blood_pressure_diastolic,
            tension,
            sp_o2,
            vital_energy,
            vital_fatigue,
            vital_glucose,
            bladder_pressure,
            bowel_pressure,
            hygiene,
            injury,
            ..self.clone()
        }
    }

    fn defaults() -> Self {
        Self {
            pulse: 70.0,
            blood_pressure_systolic: 120.0,
            blood_pressure_diastolic: 80.0,
            resting_systolic: 120.0,
            resting_diastolic: 80.0,
            sp_o2: 0.98,
            body_temperature_c: 37.0,
            vital_glucose: 0.0,
            vital_energy: 0.0,
            vital_fatigue: 0.0,
            arousal: 0.0,
            tension: 0.0,
            bladder_pressure: 0.0,
            bowel_pressure: 0.0,
            hygiene: 1.0,
            cortisol: 0.0,
            oxytocin: 0.0,
            dopamine: 0.0,
            melatonin: 0.0,
            testosterone: 0.0,
            estrogen: 0.0,
            atp: 0.8,
            calorie_intake: 0.0,
            calorie_burn: 0.0,
            height_cm: 170.0,
            weight_kg: 70.0,
            build: "average".to_string(),
            hair_color: "brown".to_string(),
            eye_color: "brown".to_string(),
            skin_tone: default_skin_tone(),
            injury: 0.0,
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

fn non_empty_or(value: &str, default: &str) -> String {
    if value.is_empty() {
        default.to_string()
    } else {
        value.to_string()
    }
}

#[cfg(test)]
mod tests {
    fn environment(shelter_quality: f64) -> BodyEnvironment {
        BodyEnvironment {
            shelter_quality,
            ambient_temperature_c: 20.0,
            inflammation: 0.0,
        }
    }

    use super::*;

    #[test]
    fn injuries_persist_cap_energy_and_heal_with_nourishment() {
        let profile = mk_core::human::HumanProfile::from_canonical_schema(
            mk_core::human::HumanId::new(3),
            mk_core::human::HumanSchema::canonical_minimal("injury_test"),
        );
        let mut body = BodySnapshot::from_profile(&profile);
        let mut needs = super::super::needs::NeedsSnapshot::from_profile(&profile);
        needs.glucose = 1.0;
        needs.hydration = 1.0;
        body.wound(1.0);
        assert!((body.injury - 0.4).abs() < 1e-12);

        // One hour later the wound is still there and still caps energy,
        // even though glucose is available.
        let hour = 1.0 / (365.25 * 24.0);
        let later = body.step(&needs, hour, environment(1.0));
        assert!(later.injury > 0.39, "an injury must not vanish in a tick");
        assert!(later.vital_energy <= 1.0 - later.injury + 1e-12);
        let mut unhurt = body.clone();
        unhurt.injury = 0.0;
        assert!(later.pulse > unhurt.step(&needs, hour, environment(1.0)).pulse);

        // Six weeks of nourished recovery heals most of it.
        let healed = body.step(&needs, 42.0 / 365.25, environment(1.0));
        assert!(healed.injury < 0.1, "injury {}", healed.injury);

        // Starved, the same wound barely heals.
        let mut starving = needs.clone();
        starving.glucose = 0.0;
        assert!(body.step(&starving, 42.0 / 365.25, environment(1.0)).injury > 0.39);
    }

    #[test]
    fn accumulated_injury_is_fatal() {
        let mut body = BodySnapshot::defaults();
        for _ in 0..3 {
            body.wound(1.0);
        }
        assert!(body.is_fatally_injured());
    }
    use mk_core::human::{HumanId, HumanSchema};

    #[test]
    fn from_profile_uses_sane_defaults_when_schema_unpopulated() {
        let schema = HumanSchema::canonical_minimal("body_test");
        let profile = HumanProfile::from_canonical_schema(HumanId::new(1), schema);
        let snapshot = BodySnapshot::from_profile(&profile);

        assert!((snapshot.pulse - 70.0).abs() < 1e-9);
        assert_eq!(snapshot.build, "average");
        assert!(snapshot.height_cm > 0.0);
    }

    #[test]
    fn step_couples_waste_pressure_to_shelter_access() {
        let schema = HumanSchema::canonical_minimal("body_step_test");
        let profile = HumanProfile::from_canonical_schema(HumanId::new(2), schema);
        let body = BodySnapshot::from_profile(&profile);
        let needs = super::super::needs::NeedsSnapshot::from_profile(&profile);

        let neglected = body.step(&needs, 1.0, environment(0.0));
        assert!(neglected.bladder_pressure > body.bladder_pressure);

        let cared_for = body.step(&needs, 1.0, environment(1.0));
        assert!(cared_for.bladder_pressure <= body.bladder_pressure);
    }

    #[test]
    fn injuries_persist_and_heal_with_time() {
        let profile = mk_core::human::HumanProfile::from_canonical_schema(
            mk_core::human::HumanId::new(4),
            mk_core::human::HumanSchema::canonical_minimal("injury_test"),
        );
        let mut needs = super::super::needs::NeedsSnapshot::from_profile(&profile);
        needs.glucose = 0.9;
        let mut body = BodySnapshot::from_profile(&profile);
        body.injury = 0.5;

        let next_day = body.step(&needs, 1.0 / 365.25, environment(1.0));
        assert!(next_day.injury > 0.45, "a wound must not vanish in a day");
        assert!(next_day.vital_energy < needs.glucose * 0.6);

        let half_year = body.step(&needs, 0.5, environment(1.0));
        assert!(half_year.injury < 0.05, "a wound heals over months");
    }

    #[test]
    fn exposure_and_fever_move_core_temperature() {
        let profile = mk_core::human::HumanProfile::from_canonical_schema(
            mk_core::human::HumanId::new(5),
            mk_core::human::HumanSchema::canonical_minimal("thermo_test"),
        );
        let needs = super::super::needs::NeedsSnapshot::from_profile(&profile);
        let body = BodySnapshot::from_profile(&profile);
        let day = 1.0 / 365.25;
        let at = |ambient: f64, shelter: f64, inflammation: f64| {
            body.step(
                &needs,
                day,
                BodyEnvironment {
                    shelter_quality: shelter,
                    ambient_temperature_c: ambient,
                    inflammation,
                },
            )
            .body_temperature_c
        };

        assert!((at(20.0, 0.0, 0.0) - 37.0).abs() < 0.01, "thermoneutral");
        assert!(
            at(-30.0, 0.0, 0.0) < LETHAL_HYPOTHERMIA_C + 5.0,
            "exposed in a blizzard"
        );
        assert!(
            (at(-30.0, 1.0, 0.0) - 37.0).abs() < 0.01,
            "sheltered from it"
        );
        assert!(at(20.0, 0.0, 1.0) > 39.0, "fever");
    }
}
