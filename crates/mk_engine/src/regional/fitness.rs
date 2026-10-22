//! Fitness that is trained and lost as it is on Earth (realism R12).
//!
//! Every person carries an aerobic capacity (VO2max, ml O2 per kg per
//! minute), a strength (a one-repetition maximum, kg, for a whole-body lift)
//! and a lean mass (kg). None of them is fixed: each day's work trains or
//! detrains them.
//!
//! - **Aerobic:** the hours a person spent at or above the training
//!   threshold MET move VO2max toward a trained level, in proportion to how
//!   much of the reference training week they did; fewer move it back
//!   toward their own untrained baseline.
//! - **Strength:** a day on which they lifted at least the training share
//!   of their present 1RM counts as a session; the share of the reference
//!   sessions a week they managed moves their 1RM toward a trained level.
//! - **Lean mass:** grows with strength training and wastes with disuse, at
//!   the pack's rates.
//!
//! Each quantity relaxes toward its target first-order: the time constant
//! is fitted to the cited observation (a change of `f` of the full response
//! after `n` days gives `tau = -n / ln(1 - f / full)`). The shape is the
//! model's (D65); the rates and plateaus are cited in
//! `fixtures/reference/humans/training.json`.
//!
//! Fat is not modelled here: a body's mass is its carbon in the ledger
//! (`regional::materials`), which eating raises and respiring lowers, so a
//! person who eats more than they burn already gets heavier. Fat is that
//! mass less the lean.

use crate::validation::{ReferenceDomain, ReferenceLibrary, ReferenceTable};
use serde::{Deserialize, Serialize};

/// One first-order response: a plateau share above baseline and how fast
/// it is approached (when training) and lost (when not).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Response {
    /// The trained plateau as a share above the untrained baseline.
    pub plateau_fraction: f64,
    /// Days to approach the plateau's time constant when trained fully.
    pub train_tau_days: f64,
    /// Days of the time constant for losing it when not trained.
    pub detrain_tau_days: f64,
}

impl Response {
    /// The time constant that reaches `observed` of a full change of
    /// `full` after `days`.
    pub fn fitted_tau(days: f64, observed: f64, full: f64) -> Result<f64, String> {
        let share = observed / full;
        if !(days > 0.0) || !(share > 0.0 && share < 1.0) {
            return Err(format!(
                "cannot fit a time constant to {observed} of {full} in {days} days"
            ));
        }
        Ok(-days / (1.0 - share).ln())
    }

    /// Move `value` one `days` step: toward `baseline * (1 + plateau *
    /// dose)` where `dose` in 0..=1 is how much of the reference training
    /// was done, at the training rate when rising and the detraining rate
    /// when falling.
    pub fn step(&self, value: f64, baseline: f64, dose: f64, days: f64) -> f64 {
        let target = baseline * (1.0 + self.plateau_fraction * dose.clamp(0.0, 1.0));
        let tau = if target >= value {
            self.train_tau_days
        } else {
            self.detrain_tau_days
        };
        target + (value - target) * (-days.max(0.0) / tau).exp()
    }
}

/// The cited rates (`fixtures/reference/humans/training.json`).
#[derive(Clone, Debug, PartialEq)]
pub struct TrainingModels {
    /// MET at or above which an hour trains the aerobic system.
    pub aerobic_threshold_met: f64,
    /// Hours a week at that intensity in the reference training.
    pub aerobic_reference_hours_per_week: f64,
    pub aerobic: Response,
    /// Share of 1RM a lift must reach to train strength.
    pub strength_load_fraction: f64,
    /// Sessions a week in the reference strength training.
    pub strength_reference_sessions_per_week: f64,
    pub strength: Response,
    pub lean: Response,
    /// Untrained adult VO2max (ml/kg/min) for men and women.
    pub baseline_vo2max: [f64; 2],
    /// Untrained 1RM as a share of body mass, men and women.
    pub baseline_strength_per_kg: [f64; 2],
    /// Lean (fat-free) share of an untrained adult's mass, men and women.
    pub baseline_lean_fraction: [f64; 2],
}

fn value(t: &ReferenceTable, row: &str, col: &str) -> Result<f64, String> {
    t.get(row, col)
        .ok_or_else(|| format!("training {row}/{col} is missing"))
}

fn response(t: &ReferenceTable) -> Result<Response, String> {
    let plateau = value(t, "plateau_fraction", "value")?;
    Ok(Response {
        plateau_fraction: plateau,
        train_tau_days: Response::fitted_tau(
            value(t, "training_days", "value")?,
            value(t, "training_gain_fraction", "value")?,
            plateau,
        )?,
        detrain_tau_days: Response::fitted_tau(
            value(t, "detraining_days", "value")?,
            value(t, "detraining_loss_fraction", "value")?,
            plateau,
        )?,
    })
}

impl TrainingModels {
    pub fn from_reference(library: &ReferenceLibrary) -> Result<Self, String> {
        let table = |key: &str| {
            library
                .item(ReferenceDomain::Humans, key)
                .and_then(|i| i.table())
                .ok_or_else(|| format!("humans/{key} is missing"))
        };
        let aerobic = table("aerobic_training")?;
        let strength = table("strength_training")?;
        let lean = table("lean_mass_training")?;
        let base = table("untrained_baselines")?;
        let sexed = |row: &str| -> Result<[f64; 2], String> {
            Ok([value(base, row, "male")?, value(base, row, "female")?])
        };
        Ok(Self {
            aerobic_threshold_met: value(aerobic, "threshold_met", "value")?,
            aerobic_reference_hours_per_week: value(aerobic, "reference_hours_per_week", "value")?,
            aerobic: response(aerobic)?,
            strength_load_fraction: value(strength, "load_fraction_of_1rm", "value")?,
            strength_reference_sessions_per_week: value(
                strength,
                "reference_sessions_per_week",
                "value",
            )?,
            strength: response(strength)?,
            lean: response(lean)?,
            baseline_vo2max: sexed("vo2max_ml_kg_min")?,
            baseline_strength_per_kg: sexed("strength_1rm_per_kg")?,
            baseline_lean_fraction: sexed("lean_fraction")?,
        })
    }
}

/// What a person did in a day that trains them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct DayWork {
    /// Hours spent at or above the aerobic threshold MET.
    pub aerobic_hours: f64,
    /// The heaviest load they lifted or carried (kg).
    pub heaviest_lift_kg: f64,
}

impl DayWork {
    /// Book `seconds` of work at `met` lifting `load_kg`.
    pub fn record(&mut self, models: &TrainingModels, met: f64, seconds: f64, load_kg: f64) {
        if met >= models.aerobic_threshold_met {
            self.aerobic_hours += seconds.max(0.0) / 3_600.0;
        }
        self.heaviest_lift_kg = self.heaviest_lift_kg.max(load_kg);
    }
}

/// One person's trainable body.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Fitness {
    /// Untrained levels this person returns to: their own, which their
    /// genes set (R13) and which age moves.
    pub baseline_vo2max: f64,
    pub baseline_strength_kg: f64,
    pub baseline_lean_kg: f64,
    pub vo2max_ml_kg_min: f64,
    pub strength_1rm_kg: f64,
    pub lean_kg: f64,
    /// Training doses over the last seven days, oldest first: aerobic
    /// hours and whether strength was trained.
    pub week_aerobic_hours: Vec<f64>,
    pub week_strength_sessions: Vec<bool>,
    /// Today's work so far.
    pub today: DayWork,
    /// What this person's genome and early environment make of the
    /// population's untrained aerobic capacity, strength and lean mass
    /// (`regional::genome`).
    #[serde(default = "unscaled")]
    pub inherited: [f64; 3],
}

fn unscaled() -> [f64; 3] {
    [1.0; 3]
}

impl Fitness {
    /// An untrained adult of `mass_kg`; `female` picks the women's column.
    pub fn untrained(models: &TrainingModels, female: bool, mass_kg: f64) -> Self {
        let s = usize::from(female);
        let vo2 = models.baseline_vo2max[s];
        let strength = models.baseline_strength_per_kg[s] * mass_kg;
        let lean = models.baseline_lean_fraction[s] * mass_kg;
        Self {
            baseline_vo2max: vo2,
            baseline_strength_kg: strength,
            baseline_lean_kg: lean,
            vo2max_ml_kg_min: vo2,
            strength_1rm_kg: strength,
            lean_kg: lean,
            week_aerobic_hours: Vec::new(),
            week_strength_sessions: Vec::new(),
            today: DayWork::default(),
            inherited: unscaled(),
        }
    }

    /// Scale the untrained baselines by what this person inherited, and
    /// start them at their own baseline.
    pub fn scale_baselines(&mut self, aerobic: f64, strength: f64, lean: f64) {
        self.inherited = [aerobic, strength, lean];
        self.baseline_vo2max *= aerobic;
        self.baseline_strength_kg *= strength;
        self.baseline_lean_kg *= lean;
        self.vo2max_ml_kg_min = self.baseline_vo2max;
        self.strength_1rm_kg = self.baseline_strength_kg;
        self.lean_kg = self.baseline_lean_kg;
    }

    /// Close one day for a body now of `mass_kg`: the untrained baselines
    /// follow the body (a child's grow with them), today's work joins the
    /// week, and the week's dose trains or detrains each quantity for the
    /// day. Everyone alive closes the same way, whatever their age (D65:
    /// the per-kg baselines are adults').
    pub fn end_day(&mut self, models: &TrainingModels, female: bool, mass_kg: f64) {
        let s = usize::from(female);
        self.baseline_strength_kg = models.baseline_strength_per_kg[s] * mass_kg * self.inherited[1];
        self.baseline_lean_kg = models.baseline_lean_fraction[s] * mass_kg * self.inherited[2];
        let today = std::mem::take(&mut self.today);
        let trained_strength =
            today.heaviest_lift_kg >= models.strength_load_fraction * self.strength_1rm_kg;
        self.week_aerobic_hours.push(today.aerobic_hours);
        self.week_strength_sessions.push(trained_strength);
        while self.week_aerobic_hours.len() > 7 {
            self.week_aerobic_hours.remove(0);
        }
        while self.week_strength_sessions.len() > 7 {
            self.week_strength_sessions.remove(0);
        }
        let aerobic_dose = self.week_aerobic_hours.iter().sum::<f64>()
            / models.aerobic_reference_hours_per_week;
        let strength_dose = self.week_strength_sessions.iter().filter(|t| **t).count() as f64
            / models.strength_reference_sessions_per_week;
        self.vo2max_ml_kg_min =
            models
                .aerobic
                .step(self.vo2max_ml_kg_min, self.baseline_vo2max, aerobic_dose, 1.0);
        self.strength_1rm_kg = models.strength.step(
            self.strength_1rm_kg,
            self.baseline_strength_kg,
            strength_dose,
            1.0,
        );
        self.lean_kg = models
            .lean
            .step(self.lean_kg, self.baseline_lean_kg, strength_dose, 1.0);
    }

    /// The heaviest load this person can lift once.
    pub fn max_lift_kg(&self) -> f64 {
        self.strength_1rm_kg
    }

    /// Fat (kg) in a body of `mass_kg`: what is not lean.
    pub fn fat_kg(&self, mass_kg: f64) -> f64 {
        (mass_kg - self.lean_kg).max(0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn models() -> TrainingModels {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/reference");
        let library = ReferenceLibrary::load(&root).unwrap();
        TrainingModels::from_reference(&library).unwrap()
    }

    /// A body of fixed sex and mass doing the same each day.
    fn train_days(
        f: &mut Fitness,
        m: &TrainingModels,
        (female, mass): (bool, f64),
        days: usize,
        aerobic_h: f64,
        lift: bool,
    ) {
        for _ in 0..days {
            let lift_kg = if lift { f.strength_1rm_kg * m.strength_load_fraction } else { 0.0 };
            f.today = DayWork {
                aerobic_hours: aerobic_h,
                heaviest_lift_kg: lift_kg,
            };
            f.end_day(m, female, mass);
        }
    }

    #[test]
    fn fitted_tau_reproduces_the_observation() {
        let tau = Response::fitted_tau(56.0, 0.1, 0.2).unwrap();
        let r = Response {
            plateau_fraction: 0.2,
            train_tau_days: tau,
            detrain_tau_days: tau,
        };
        let mut v = 1.0;
        for _ in 0..56 {
            v = r.step(v, 1.0, 1.0, 1.0);
        }
        assert!((v - 1.1).abs() < 1e-9, "{v}");
        assert!(Response::fitted_tau(10.0, 0.3, 0.2).is_err());
    }

    #[test]
    fn the_reference_training_reaches_the_cited_gain() {
        let m = models();
        let mut f = Fitness::untrained(&m, false, 75.0);
        let daily_h = m.aerobic_reference_hours_per_week / 7.0;
        // Steps are daily; the dose reaches full only once a week is in.
        train_days(&mut f, &m, (false, 75.0), 365, daily_h, true);
        let gain = f.vo2max_ml_kg_min / f.baseline_vo2max - 1.0;
        assert!((gain - m.aerobic.plateau_fraction).abs() < 0.01, "{gain}");
        assert!(f.strength_1rm_kg > f.baseline_strength_kg);
        assert!(f.lean_kg > f.baseline_lean_kg);
    }

    #[test]
    fn idleness_loses_what_training_built_but_not_below_baseline() {
        let m = models();
        let mut f = Fitness::untrained(&m, true, 62.0);
        train_days(&mut f, &m, (true, 62.0), 180, m.aerobic_reference_hours_per_week / 7.0, true);
        let trained = f.clone();
        train_days(&mut f, &m, (true, 62.0), 60, 0.0, false);
        assert!(f.vo2max_ml_kg_min < trained.vo2max_ml_kg_min);
        assert!(f.strength_1rm_kg < trained.strength_1rm_kg);
        assert!(f.lean_kg < trained.lean_kg);
        train_days(&mut f, &m, (true, 62.0), 3_000, 0.0, false);
        assert!(f.vo2max_ml_kg_min >= f.baseline_vo2max - 1e-9);
        assert!((f.vo2max_ml_kg_min - f.baseline_vo2max).abs() < 0.01);
    }

    #[test]
    fn half_the_training_buys_half_the_gain() {
        let m = models();
        let mut full = Fitness::untrained(&m, false, 80.0);
        let mut half = full.clone();
        let h = m.aerobic_reference_hours_per_week / 7.0;
        train_days(&mut full, &m, (false, 80.0), 400, h, false);
        train_days(&mut half, &m, (false, 80.0), 400, h / 2.0, false);
        let g_full = full.vo2max_ml_kg_min / full.baseline_vo2max - 1.0;
        let g_half = half.vo2max_ml_kg_min / half.baseline_vo2max - 1.0;
        assert!((g_half - g_full / 2.0).abs() < 1e-3, "{g_half} {g_full}");
    }

    #[test]
    fn light_lifts_do_not_train_strength() {
        let m = models();
        let mut f = Fitness::untrained(&m, false, 80.0);
        for _ in 0..90 {
            f.today.record(&m, 1.5, 3_600.0, 0.3 * m.strength_load_fraction * f.strength_1rm_kg);
            f.end_day(&m, false, 80.0);
        }
        assert!((f.strength_1rm_kg - f.baseline_strength_kg).abs() < 1e-9);
        assert!(f.week_aerobic_hours.iter().all(|h| *h == 0.0));
    }

    #[test]
    fn work_records_hours_only_above_threshold() {
        let m = models();
        let mut d = DayWork::default();
        d.record(&m, m.aerobic_threshold_met, 1_800.0, 10.0);
        d.record(&m, m.aerobic_threshold_met - 0.5, 3_600.0, 25.0);
        assert!((d.aerobic_hours - 0.5).abs() < 1e-12);
        assert_eq!(d.heaviest_lift_kg, 25.0);
    }
}
