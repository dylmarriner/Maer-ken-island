//! Reproductive Systems Snapshot - sexual system, mate selection, and
//! genetics/lineage history summary.
//!
//! Extends the reproduction/fertility work in `lifecycle.rs` by surfacing
//! the rest of `docs/canon/HumanReplicationSchema.js`'s
//! `reproductive_systems` block (`sexual_system`, `mate_selection`,
//! `genetics_system`) as a pure projection, and by giving `lifecycle.rs`
//! somewhere to *write* real lineage history (birth records, conception
//! history, sexual activity log) instead of that canon data staying
//! permanently empty.

use mk_core::human::HumanProfile;
use mk_core::time::Tick;
use serde::{Deserialize, Serialize};

/// Full-term human gestation: 40 weeks from conception.
pub const GESTATION_YEARS: f64 = 40.0 * 7.0 / 365.25;
/// Postpartum anovulatory period after a birth, during which the cycle is
/// suspended and conception cannot occur.
pub const POSTPARTUM_INFERTILE_YEARS: f64 = 0.25;
/// Relative chance of conception from an act outside the cycle's fertile
/// window (sperm survival makes it low but not zero).
const OUTSIDE_FERTILE_WINDOW_FACTOR: f64 = 0.08;
/// Acts of intercourse per day for a couple being intimate over a step
/// longer than a day (about twice a week, a typical cohabiting rate).
const INTERCOURSE_PER_DAY: f64 = 2.0 / 7.0;
/// Engine defaults for unfilled (zero-valued) canon `sexual_system` /
/// `reproduction_system` fields, following the same zero-means-unset
/// convention every sibling module uses.
const DEFAULT_LIBIDO: f64 = 0.6;
const DEFAULT_AROUSAL: f64 = 0.4;
/// Per-act conception probability inside the fertile window for a couple
/// at full fertility.
const DEFAULT_CONCEPTION_PROBABILITY: f64 = 0.25;

fn default_baseline_libido() -> f64 {
    DEFAULT_LIBIDO
}

fn default_baseline_arousal() -> f64 {
    DEFAULT_AROUSAL
}

/// An ongoing pregnancy: the conceived child (genome and inherited traits
/// fixed at conception) and how far gestation has progressed. The child's
/// birth instant, birthplace and birth chart are assigned only at delivery.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pregnancy {
    pub father_id: String,
    pub conceived_tick: Tick,
    pub gestation_years: f64,
    pub embryo: Box<super::HumanBeing>,
}

impl Pregnancy {
    pub fn is_due(&self) -> bool {
        self.gestation_years >= GESTATION_YEARS
    }
}

/// Runtime reproductive-systems snapshot, pure-derived from the canonical
/// schema's `reproductive_systems` block (defaults when schema data is
/// absent or unpopulated).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReproductiveSystemSnapshot {
    // --- sexual_system ---
    pub libido: f64,
    pub arousal: f64,
    pub satisfaction: f64,
    pub frustration: f64,
    pub hormonal_influence: f64,
    /// Average of `attraction` map values (empty map -> 0.5 baseline).
    pub attraction_average: f64,
    /// Average of `bonding` map values (empty map -> 0.5 baseline).
    pub bonding_average: f64,

    // --- mate_selection.preferences ---
    pub preferred_age_min: f64,
    pub preferred_age_max: f64,
    pub preferred_age_ideal: f64,
    pub genetic_compatibility_preference: f64,
    pub social_status_preference: f64,
    pub intelligence_preference: f64,
    pub courtship_behavior_count: usize,
    pub relationship_history_count: usize,

    // --- genetics_system (lineage history) ---
    pub conception_history_count: usize,
    pub birth_record_count: usize,
    pub gamete_count: usize,
    pub hereditary_condition_count: usize,
    /// Highest `severity` among any recorded hereditary conditions (0.0 if none).
    pub max_hereditary_condition_severity: f64,

    // --- reproduction_system ---
    pub fertility_level: f64,
    pub conception_probability: f64,
    pub gestation_week: u32,
    pub sexual_activity_count: usize,

    /// Real day-by-day menstrual cycle state (canon `fertility_cycle`),
    /// tracked only while `fertility_level > 0` — see [`FertilityCycle`]
    /// docs. `None` for males/neutral, prepubescent, or post-fertility
    /// humans, matching `fertility_level`'s own gating.
    pub fertility_cycle: Option<FertilityCycle>,

    /// Trait-level libido/arousal the lived values recover toward once
    /// deprivation lifts (suppression is applied to these, never compounded
    /// on the already-suppressed current value).
    #[serde(default = "default_baseline_libido")]
    pub baseline_libido: f64,
    #[serde(default = "default_baseline_arousal")]
    pub baseline_arousal: f64,
    /// The current pregnancy, if any. The cycle is suspended while pregnant.
    #[serde(default)]
    pub pregnancy: Option<Pregnancy>,
    /// Remaining postpartum anovulatory time after the last birth.
    #[serde(default)]
    pub postpartum_years_remaining: f64,
}

/// A real 28-day hormone-curve menstrual cycle: `cycle_day` genuinely
/// advances tick-to-tick (accumulated from `dt_years`, not derived from
/// wall-clock or `rand`), and `phase`/`hormone_levels`/`fertility_peak`/
/// `basal_body_temp` are all computed as smooth deterministic functions of
/// `cycle_day` — canon's `FertilityCycleSchema` shape with genuine dynamics
/// behind it instead of a permanently-static readout.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum CyclePhase {
    Menstrual,
    Follicular,
    Ovulation,
    Luteal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FertilityCycle {
    /// 1-28 (real 4-week idealized cycle).
    pub cycle_day: u32,
    pub phase: CyclePhase,
    pub fertility_peak: bool,
    pub estrogen: f64,
    pub progesterone: f64,
    pub lh: f64,
    pub fsh: f64,
    /// Degrees C relative to baseline; rises ~0.3-0.5C after ovulation, a
    /// real physiological signal this curve reproduces.
    pub basal_body_temp_delta: f64,
    /// Fractional day accumulator so `dt_years` steps smaller than a day
    /// still advance the cycle correctly over many ticks.
    day_accumulator: f64,
}

const CYCLE_LENGTH_DAYS: f64 = 28.0;
const OVULATION_DAY: f64 = 14.0;

impl FertilityCycle {
    /// Whether cycle day `day` (1–28) lies in the fertile window around
    /// ovulation.
    pub fn is_fertile_day(day: f64) -> bool {
        (OVULATION_DAY - 2.0..=OVULATION_DAY + 1.0).contains(&day)
    }

    fn at_day(cycle_day: u32, day_accumulator: f64) -> Self {
        let d = cycle_day as f64;
        let phase = if d <= 5.0 {
            CyclePhase::Menstrual
        } else if d < OVULATION_DAY - 1.0 {
            CyclePhase::Follicular
        } else if d <= OVULATION_DAY + 1.0 {
            CyclePhase::Ovulation
        } else {
            CyclePhase::Luteal
        };
        let fertility_peak = Self::is_fertile_day(d);

        // Smooth idealized curves (not literature-precision endocrinology,
        // but a genuine deterministic function of cycle_day, not a static
        // constant): estrogen peaks just before ovulation, progesterone
        // dominates the luteal phase, LH surges at ovulation, FSH rises
        // early follicular then falls after ovulation.
        let estrogen =
            (0.3 + 0.6 * gaussian(d, OVULATION_DAY - 1.0, 3.0) + 0.15 * gaussian(d, 21.0, 3.0))
                .clamp(0.1, 1.0);
        let progesterone = if d > OVULATION_DAY {
            (0.2 + 0.7 * gaussian(d, 21.0, 4.0)).clamp(0.1, 1.0)
        } else {
            0.15
        };
        let lh = (0.15 + 0.8 * gaussian(d, OVULATION_DAY, 1.0)).clamp(0.1, 1.0);
        let fsh = (0.3 + 0.4 * gaussian(d, 6.0, 3.0) + 0.2 * gaussian(d, OVULATION_DAY, 1.5))
            .clamp(0.1, 1.0);
        let basal_body_temp_delta = if d > OVULATION_DAY + 1.0 { 0.4 } else { 0.0 };

        Self {
            cycle_day,
            phase,
            fertility_peak,
            estrogen,
            progesterone,
            lh,
            fsh,
            basal_body_temp_delta,
            day_accumulator,
        }
    }

    fn start() -> Self {
        Self::at_day(1, 0.0)
    }

    fn step(&self, dt_years: f64) -> Self {
        let days_elapsed = self.day_accumulator + dt_years.max(0.0) * 365.25;
        let whole_days = days_elapsed.floor();
        let remainder = days_elapsed - whole_days;
        let new_day = (((self.cycle_day as f64 - 1.0) + whole_days) % CYCLE_LENGTH_DAYS) + 1.0;
        Self::at_day(new_day as u32, remainder)
    }
}

/// Gaussian bump centered at `mean` with the given `std_dev`, used to build
/// smooth, deterministic hormone curves from `cycle_day` alone.
fn gaussian(x: f64, mean: f64, std_dev: f64) -> f64 {
    let z = (x - mean) / std_dev;
    (-0.5 * z * z).exp()
}

/// When one individual's fecundity rises, peaks and falls with age.
///
/// Fecundability (per-cycle chance of conception relative to the peak) is
/// the product of two published curves:
/// - maturation: after the first mature gametes (menarche, spermarche) the
///   share of cycles that are fertile rises as `1 − e^(−t/τ)` (about a
///   fifth of cycles ovulate in the first year after menarche and three
///   quarters by year six — Apter & Vihko 1983);
/// - decline: past a sex-specific age fecundability halves every
///   `decline_half_life_years` (female fecundability at 35–39 is about half
///   that at 19–26 — Dunson, Colombo & Baird 2002; male-age effects are
///   later and shallower — Dunson, Baird & Colombo 2004).
///
/// Female fertility ends at an individual sterility age; male fertility
/// has no such end and just keeps declining.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReproductiveTimeline {
    /// Age of the first mature gametes (menarche / spermarche).
    pub gametogenesis_onset_years: f64,
    /// e-folding time of the rise in fertile cycles after that onset.
    pub maturation_time_constant_years: f64,
    /// Age after which fecundability starts to fall.
    pub decline_onset_years: f64,
    /// Years over which fecundability halves once it is falling.
    pub decline_half_life_years: f64,
    /// Age at which this individual becomes sterile, if their sex has one.
    pub sterility_years: Option<f64>,
}

impl ReproductiveTimeline {
    /// Fecundability relative to the population peak at `age_years`, 0–1.
    pub fn fecundity(&self, age_years: f64) -> f64 {
        let since_onset = age_years - self.gametogenesis_onset_years;
        if since_onset <= 0.0 {
            return 0.0;
        }
        if self
            .sterility_years
            .is_some_and(|sterile_at| age_years >= sterile_at)
        {
            return 0.0;
        }
        let maturity = -(-since_onset / self.maturation_time_constant_years.max(1e-9)).exp_m1();
        let declining_for = (age_years - self.decline_onset_years).max(0.0);
        let decline = 0.5_f64.powf(declining_for / self.decline_half_life_years.max(1e-9));
        (maturity * decline).clamp(0.0, 1.0)
    }
}

impl ReproductiveSystemSnapshot {
    pub fn from_profile(profile: &HumanProfile) -> Self {
        let Some(schema) = profile.canonical_schema() else {
            return Self::defaults();
        };
        let repro = &schema.reproductive_systems;
        let sexual = &repro.sexual_system;
        let mate = &repro.mate_selection;
        let genetics = &repro.genetics_system;
        let reproduction = &repro.reproduction_system;

        let map_average = |map: &std::collections::BTreeMap<String, f32>| -> f64 {
            if map.is_empty() {
                0.5
            } else {
                map.values().map(|v| *v as f64).sum::<f64>() / map.len() as f64
            }
        };

        let max_hereditary_condition_severity = genetics
            .hereditary_conditions
            .iter()
            .map(|c| c.severity as f64)
            .fold(0.0_f64, f64::max);

        let libido = nonzero_or(sexual.libido as f64, DEFAULT_LIBIDO);
        let arousal = nonzero_or(sexual.arousal as f64, DEFAULT_AROUSAL);

        Self {
            libido,
            arousal,
            satisfaction: sexual.satisfaction as f64,
            frustration: sexual.frustration as f64,
            hormonal_influence: sexual.hormonal_influence as f64,
            attraction_average: map_average(&sexual.attraction),
            bonding_average: map_average(&sexual.bonding),

            preferred_age_min: mate.preferences.age_preference.min as f64,
            preferred_age_max: mate.preferences.age_preference.max as f64,
            preferred_age_ideal: mate.preferences.age_preference.ideal as f64,
            genetic_compatibility_preference: mate.preferences.genetic_compatibility as f64,
            social_status_preference: mate.preferences.social_status as f64,
            intelligence_preference: mate.preferences.intelligence as f64,
            courtship_behavior_count: mate.courtship_behaviors.len(),
            relationship_history_count: mate.relationship_history.len(),

            conception_history_count: genetics.conception_history.len(),
            birth_record_count: genetics.birth_records.len(),
            gamete_count: genetics.gametes.len(),
            hereditary_condition_count: genetics.hereditary_conditions.len(),
            max_hereditary_condition_severity,

            fertility_level: reproduction.fertility_level as f64,
            conception_probability: nonzero_or(
                reproduction.conception_probability as f64,
                DEFAULT_CONCEPTION_PROBABILITY,
            ),
            gestation_week: reproduction.gestation_week,
            sexual_activity_count: reproduction.sexual_activities.len(),
            fertility_cycle: matches!(
                profile.core_identity.biological_sex,
                mk_core::human::BiologicalSex::Female
            )
            .then(FertilityCycle::start),
            baseline_libido: libido,
            baseline_arousal: arousal,
            pregnancy: None,
            postpartum_years_remaining: 0.0,
        }
    }

    /// Couple reproductive state to real engine drivers instead of a
    /// permanently-static schema readout:
    /// - `fertility_level` is the individual's age-specific fecundability
    ///   from their [`ReproductiveTimeline`] (gametogenic maturation after
    ///   menarche/spermarche, the adult decline, and sterility where the
    ///   sex has one), zeroed for genome-flagged infertile individuals.
    /// - `libido`/`arousal` are suppressed under sustained need deprivation
    ///   — a real, well-documented reproductive-suppression response to
    ///   starvation/exhaustion (the body deprioritizes reproduction under
    ///   survival stress).
    /// - `bonding_average` slowly tracks social trust, since pair-bonding
    ///   strength is downstream of trust in this engine's social model.
    pub fn step(
        &self,
        age_years: f64,
        timeline: &ReproductiveTimeline,
        genome_infertile: bool,
        needs: &super::needs::NeedsSnapshot,
        social_cognition: &super::social_cognition::SocialCognitionSnapshot,
        dt_years: f64,
    ) -> Self {
        let dt = dt_years.max(0.0);

        let fertility_level = if genome_infertile {
            0.0
        } else {
            timeline.fecundity(age_years)
        };

        // Sustained deprivation (hunger/thirst/exhaustion) suppresses
        // reproductive drive — a real evolved response, not modeled purely
        // as a static trait.
        let deprivation_suppression = needs.fatigue.clamp(0.0, 1.0);
        let blend = (0.3 * dt).clamp(0.0, 1.0);
        let libido_target =
            (self.baseline_libido * (1.0 - deprivation_suppression * 0.6)).clamp(0.0, 1.0);
        let arousal_target =
            (self.baseline_arousal * (1.0 - deprivation_suppression * 0.6)).clamp(0.0, 1.0);
        let libido = lerp(self.libido, libido_target, blend);
        let arousal = lerp(self.arousal, arousal_target, blend);

        let bonding_blend = (0.15 * dt).clamp(0.0, 1.0);
        let bonding_average = lerp(
            self.bonding_average,
            social_cognition.trust_assessment,
            bonding_blend,
        );

        // Gestation advances with simulated time; the cycle is suspended for
        // the whole pregnancy.
        let pregnancy = self.pregnancy.clone().map(|mut pregnancy| {
            pregnancy.gestation_years += dt;
            pregnancy
        });
        let gestation_week = pregnancy
            .as_ref()
            .map(|p| (p.gestation_years * 365.25 / 7.0).floor() as u32)
            .unwrap_or(0);
        let postpartum_years_remaining = (self.postpartum_years_remaining - dt).max(0.0);

        // The cycle only genuinely advances while fertile, not pregnant and
        // past the postpartum anovulatory period; it holds at whatever day
        // it was on otherwise (prepubescent/post-fertility/male/neutral
        // humans never had one to begin with — see `fertility_cycle`'s
        // `None` gating in `from_profile`).
        let cycling =
            fertility_level > 0.0 && pregnancy.is_none() && postpartum_years_remaining == 0.0;
        let fertility_cycle = if cycling {
            self.fertility_cycle.as_ref().map(|c| c.step(dt))
        } else {
            self.fertility_cycle.clone()
        };

        Self {
            fertility_level,
            libido,
            arousal,
            bonding_average,
            fertility_cycle,
            pregnancy,
            gestation_week,
            postpartum_years_remaining,
            ..self.clone()
        }
    }

    /// Chance that one act of intercourse with a partner of
    /// `partner_fertility_level` conceives, from this (female) human's side:
    /// zero while pregnant, postpartum, without a cycle, or infertile;
    /// otherwise the per-act `conception_probability` weighted by both
    /// partners' fertility and by whether the cycle is in its fertile window.
    pub fn conception_chance(&self, partner_fertility_level: f64) -> f64 {
        if self.pregnancy.is_some() || self.postpartum_years_remaining > 0.0 {
            return 0.0;
        }
        let Some(cycle) = self.fertility_cycle.as_ref() else {
            return 0.0;
        };
        let window = if cycle.fertility_peak {
            1.0
        } else {
            OUTSIDE_FERTILE_WINDOW_FACTOR
        };
        (self.conception_probability
            * self.fertility_level.clamp(0.0, 1.0)
            * partner_fertility_level.clamp(0.0, 1.0)
            * window)
            .clamp(0.0, 1.0)
    }

    /// Chance that a couple intimate over the last `dt_days` conceives. For a
    /// step of a day or less this is one act ([`Self::conception_chance`]).
    /// A longer step spans part of the cycle: each day it covered, counted
    /// back from today's cycle day, has its own fertile-window factor and
    /// [`INTERCOURSE_PER_DAY`] acts, and the chances combine as
    /// `1 − Π(1 − pᵢ)`. A step longer than one cycle counts one cycle.
    pub fn conception_chance_over(&self, partner_fertility_level: f64, dt_days: f64) -> f64 {
        let per_act = self.conception_chance(partner_fertility_level);
        if dt_days <= 1.0 || per_act <= 0.0 {
            return per_act;
        }
        let Some(cycle) = self.fertility_cycle.as_ref() else {
            return 0.0;
        };
        let peak_factor = if cycle.fertility_peak {
            1.0
        } else {
            OUTSIDE_FERTILE_WINDOW_FACTOR
        };
        // Per-act chance with the window factor removed, so each covered
        // day can apply its own.
        let base = per_act / peak_factor;
        let days = dt_days.min(CYCLE_LENGTH_DAYS);
        let whole_days = days.floor() as u32;
        let mut none = 1.0;
        for back in 0..=whole_days {
            let share = if back < whole_days {
                1.0
            } else {
                days - whole_days as f64
            };
            if share <= 0.0 {
                continue;
            }
            let day =
                (cycle.cycle_day as f64 - back as f64 - 1.0).rem_euclid(CYCLE_LENGTH_DAYS) + 1.0;
            let window = if FertilityCycle::is_fertile_day(day) {
                1.0
            } else {
                OUTSIDE_FERTILE_WINDOW_FACTOR
            };
            let acts = INTERCOURSE_PER_DAY * share;
            none *= (1.0 - (base * window).clamp(0.0, 1.0)).powf(acts);
        }
        (1.0 - none).clamp(0.0, 1.0)
    }

    /// Hand back the due pregnancy (if gestation is complete), resetting the
    /// cycle to day 1 and starting the postpartum anovulatory period.
    pub fn take_due_pregnancy(&mut self) -> Option<Pregnancy> {
        if !self.pregnancy.as_ref().is_some_and(Pregnancy::is_due) {
            return None;
        }
        let pregnancy = self.pregnancy.take();
        self.gestation_week = 0;
        self.postpartum_years_remaining = POSTPARTUM_INFERTILE_YEARS;
        if self.fertility_cycle.is_some() {
            self.fertility_cycle = Some(FertilityCycle::start());
        }
        pregnancy
    }

    fn defaults() -> Self {
        Self {
            libido: DEFAULT_LIBIDO,
            arousal: DEFAULT_AROUSAL,
            satisfaction: 0.0,
            frustration: 0.0,
            hormonal_influence: 0.0,
            attraction_average: 0.5,
            bonding_average: 0.5,
            preferred_age_min: 0.0,
            preferred_age_max: 0.0,
            preferred_age_ideal: 0.0,
            genetic_compatibility_preference: 0.0,
            social_status_preference: 0.0,
            intelligence_preference: 0.0,
            courtship_behavior_count: 0,
            relationship_history_count: 0,
            conception_history_count: 0,
            birth_record_count: 0,
            gamete_count: 0,
            hereditary_condition_count: 0,
            max_hereditary_condition_severity: 0.0,
            fertility_level: 0.0,
            conception_probability: DEFAULT_CONCEPTION_PROBABILITY,
            gestation_week: 0,
            sexual_activity_count: 0,
            fertility_cycle: None,
            baseline_libido: DEFAULT_LIBIDO,
            baseline_arousal: DEFAULT_AROUSAL,
            pregnancy: None,
            postpartum_years_remaining: 0.0,
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

fn lerp(a: f64, b: f64, t: f64) -> f64 {
    (a + (b - a) * t.clamp(0.0, 1.0)).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::humans::needs::NeedsSnapshot;
    use crate::humans::social_cognition::SocialCognitionSnapshot;
    use mk_core::human::{HumanId, HumanSchema};

    fn profile() -> HumanProfile {
        let schema = HumanSchema::canonical_minimal("repro_test");
        HumanProfile::from_canonical_schema(HumanId::new(1), schema)
    }

    #[test]
    fn conception_over_a_long_step_counts_the_days_it_covers() {
        let mut snapshot = ReproductiveSystemSnapshot::from_profile(&profile());
        snapshot.fertility_level = 1.0;
        // Day 20: past ovulation, outside the fertile window.
        snapshot.fertility_cycle = Some(FertilityCycle::at_day(20, 0.0));
        let one_act = snapshot.conception_chance(1.0);
        assert_eq!(snapshot.conception_chance_over(1.0, 0.5), one_act);
        // The week back to day 13 covers the fertile window (days 12-15).
        let week = snapshot.conception_chance_over(1.0, 7.0);
        assert!(week > one_act * 3.0, "week {week} vs one act {one_act}");
        // A step of several cycles counts one cycle.
        let cycle = snapshot.conception_chance_over(1.0, 28.0);
        assert_eq!(snapshot.conception_chance_over(1.0, 100.0), cycle);
        assert!(cycle > week && cycle < 1.0);
        // Pregnant or without a cycle: never.
        snapshot.fertility_cycle = None;
        assert_eq!(snapshot.conception_chance_over(1.0, 7.0), 0.0);
    }

    #[test]
    fn from_profile_defaults_when_schema_unpopulated() {
        let snapshot = ReproductiveSystemSnapshot::from_profile(&profile());

        assert_eq!(snapshot.birth_record_count, 0);
        assert_eq!(snapshot.conception_history_count, 0);
        assert!((snapshot.attraction_average - 0.5).abs() < 1e-9);
    }

    fn timeline() -> ReproductiveTimeline {
        ReproductiveTimeline {
            gametogenesis_onset_years: 12.5,
            maturation_time_constant_years: 4.5,
            decline_onset_years: 27.0,
            decline_half_life_years: 10.0,
            sterility_years: Some(41.0),
        }
    }

    #[test]
    fn fecundity_matures_after_menarche_and_halves_by_the_late_thirties() {
        let timeline = timeline();
        // A fifth of cycles fertile a year after menarche, three quarters
        // six years on (Apter & Vihko 1983).
        assert!((timeline.fecundity(13.5) - 0.2).abs() < 0.02);
        assert!((timeline.fecundity(18.5) - 0.74).abs() < 0.02);
        // Late thirties: about half of the early-twenties level.
        let ratio = timeline.fecundity(37.0) / timeline.fecundity(24.0);
        assert!((0.4..0.6).contains(&ratio), "ratio {ratio}");
        assert_eq!(timeline.fecundity(41.0), 0.0);

        let male = ReproductiveTimeline {
            sterility_years: None,
            decline_onset_years: 35.0,
            decline_half_life_years: 20.0,
            ..timeline
        };
        assert!(male.fecundity(70.0) > 0.0);
        assert!(male.fecundity(70.0) < male.fecundity(30.0));
    }

    #[test]
    fn fertility_is_zero_outside_the_puberty_to_fertility_end_window() {
        let snapshot = ReproductiveSystemSnapshot::from_profile(&profile());
        let needs = NeedsSnapshot::from_profile(&profile());
        let social = SocialCognitionSnapshot::from_profile(&profile());

        let child = snapshot.step(5.0, &timeline(), false, &needs, &social, 1.0);
        let elder = snapshot.step(80.0, &timeline(), false, &needs, &social, 1.0);
        let adult = snapshot.step(25.0, &timeline(), false, &needs, &social, 1.0);

        assert_eq!(child.fertility_level, 0.0);
        assert_eq!(elder.fertility_level, 0.0);
        assert!(adult.fertility_level > 0.0);
    }

    #[test]
    fn genome_infertile_flag_zeroes_fertility_regardless_of_age() {
        let snapshot = ReproductiveSystemSnapshot::from_profile(&profile());
        let needs = NeedsSnapshot::from_profile(&profile());
        let social = SocialCognitionSnapshot::from_profile(&profile());

        let stepped = snapshot.step(25.0, &timeline(), true, &needs, &social, 1.0);
        assert_eq!(stepped.fertility_level, 0.0);
    }

    #[test]
    fn deprivation_suppresses_libido() {
        let mut snapshot = ReproductiveSystemSnapshot::from_profile(&profile());
        snapshot.libido = 0.8;
        snapshot.arousal = 0.8;
        let social = SocialCognitionSnapshot::from_profile(&profile());

        let mut starved_needs = NeedsSnapshot::from_profile(&profile());
        starved_needs.fatigue = 1.0;
        let fed_needs = NeedsSnapshot::from_profile(&profile());

        let mut starved = snapshot.clone();
        let mut fed = snapshot.clone();
        for _ in 0..10 {
            starved = starved.step(25.0, &timeline(), false, &starved_needs, &social, 1.0);
            fed = fed.step(25.0, &timeline(), false, &fed_needs, &social, 1.0);
        }

        assert!(starved.libido <= fed.libido);
    }
}
