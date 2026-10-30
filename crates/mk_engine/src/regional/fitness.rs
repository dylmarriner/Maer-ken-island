//! Fitness that is trained and lost as it is on Earth (realism R12).
//!
//! Every person carries an aerobic capacity (VO2max, ml O2 per kg per
//! minute), a strength (grip, kg, the population-normed measure that
//! stands in for whole-body strength) and a lean (fat-free) mass (kg).
//! None of them is fixed: each day's work trains or detrains them.
//!
//! - **Aerobic:** an hour counts when the work's oxygen cost (its MET times
//!   the resting 3.5 ml/kg/min) reaches the share of the person's own VO2max
//!   at which the HERITAGE programme started. The hours of the last seven
//!   days against HERITAGE's week give the dose.
//! - **Strength:** a day on which they lifted or carried at least the
//!   training share of their strength counts as a session; the sessions of
//!   the last seven days against Narici's four a week give the dose.
//! - **Lean mass:** follows the strength dose at Narici's rate for muscle
//!   size.
//!
//! A full dose raises a quantity from the person's untrained baseline at
//! the rate the cited study measured, up to the gain the study ended at;
//! a smaller dose stops it lower. Without training it falls back at the
//! cited detraining rate, never below the baseline. The study's end point
//! is the cap: no gain is extrapolated beyond what was observed (D65).
//!
//! The untrained baselines are the person's own: the population's norms
//! for their sex and age (`fixtures/reference/humans/training.json`),
//! scaled by what their genome made of them (R13, `regional::genome`).
//!
//! Fat is not modelled here: a body's mass is its carbon in the ledger
//! (`regional::materials`), which eating raises and respiring lowers, so a
//! person who eats more than they burn already gets heavier. Fat is that
//! mass less the lean.

use crate::validation::{ReferenceDomain, ReferenceLibrary, ReferenceTable};
use serde::{Deserialize, Serialize};

/// One quantity's response to training: the gain a full dose reaches over
/// `gain_days`, and the share of the trained value lost over `loss_days`
/// without it. Both move at constant rates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Response {
    pub gain_fraction: f64,
    pub gain_days: f64,
    pub loss_fraction: f64,
    pub loss_days: f64,
}

impl Response {
    /// Move `value` one `days` step toward `baseline * (1 + gain * dose)`,
    /// where `dose` in 0..=1 is how much of the reference training was
    /// done: up at the cited training rate, down at the detraining rate,
    /// never past the target.
    pub fn step(&self, value: f64, baseline: f64, dose: f64, days: f64) -> f64 {
        let target = baseline * (1.0 + self.gain_fraction * dose.clamp(0.0, 1.0));
        let days = days.max(0.0);
        if value < target {
            (value + baseline * self.gain_fraction / self.gain_days * days).min(target)
        } else {
            (value - value * self.loss_fraction / self.loss_days * days).max(target)
        }
    }
}

/// The cited rates and norms (`fixtures/reference/humans/training.json`).
#[derive(Clone, Debug, PartialEq)]
pub struct TrainingModels {
    /// Oxygen use at rest, ml/kg/min: one MET.
    pub met_oxygen_ml_kg_min: f64,
    /// Share of a person's VO2max at or above which an hour trains them.
    pub aerobic_intensity_fraction: f64,
    /// Hours a week of the reference aerobic training.
    pub aerobic_reference_hours_per_week: f64,
    pub aerobic: Response,
    /// Share of strength a lift must reach to count as a session.
    pub strength_load_fraction: f64,
    /// Sessions a week of the reference strength training.
    pub strength_reference_sessions_per_week: f64,
    pub strength: Response,
    pub lean: Response,
    /// Untrained VO2max (ml/kg/min) by age, men then women.
    pub vo2max_by_age: [Vec<(f64, f64)>; 2],
    /// Grip strength (kg) by age, men then women.
    pub strength_by_age: [Vec<(f64, f64)>; 2],
    /// Fat-free share of body mass, men then women.
    pub lean_fraction: [f64; 2],
}

fn value(t: &ReferenceTable, row: &str, col: &str) -> Result<f64, String> {
    t.get(row, col)
        .ok_or_else(|| format!("training {row}/{col} is missing"))
}

fn response(t: &ReferenceTable) -> Result<Response, String> {
    Ok(Response {
        gain_fraction: value(t, "gain_fraction", "value")?,
        gain_days: value(t, "gain_days", "value")?,
        loss_fraction: value(t, "loss_fraction", "value")?,
        loss_days: value(t, "loss_days", "value")?,
    })
}

/// A by-age table's male and female columns as (age, value) curves.
fn by_age(t: &ReferenceTable) -> Result<[Vec<(f64, f64)>; 2], String> {
    let curve = |col: &str| -> Result<Vec<(f64, f64)>, String> {
        let mut points = t
            .column(col)
            .ok_or_else(|| format!("training by-age column {col} is missing"))?
            .into_iter()
            .map(|(age, v)| {
                age.parse::<f64>()
                    .map(|a| (a, v))
                    .map_err(|_| format!("training age {age} is not a number"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        points.sort_by(|a, b| a.0.total_cmp(&b.0));
        if points.is_empty() {
            return Err(format!("training by-age column {col} is empty"));
        }
        Ok(points)
    };
    Ok([curve("male")?, curve("female")?])
}

/// Linear between the curve's ages, held flat beyond its ends.
fn at_age(curve: &[(f64, f64)], age: f64) -> f64 {
    let (first, last) = (curve[0], curve[curve.len() - 1]);
    if age <= first.0 {
        return first.1;
    }
    if age >= last.0 {
        return last.1;
    }
    let i = curve.partition_point(|p| p.0 <= age);
    let (a, b) = (curve[i - 1], curve[i]);
    a.1 + (b.1 - a.1) * (age - a.0) / (b.0 - a.0)
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
        let fat_free = table("fat_free_fraction")?;
        Ok(Self {
            met_oxygen_ml_kg_min: value(table("met_oxygen_cost")?, "one_met", "value")?,
            aerobic_intensity_fraction: value(aerobic, "intensity_fraction_of_vo2max", "value")?,
            aerobic_reference_hours_per_week: value(aerobic, "reference_hours_per_week", "value")?,
            aerobic: response(aerobic)?,
            strength_load_fraction: value(strength, "load_fraction_of_strength", "value")?,
            strength_reference_sessions_per_week: value(
                strength,
                "reference_sessions_per_week",
                "value",
            )?,
            strength: response(strength)?,
            lean: response(lean)?,
            vo2max_by_age: by_age(table("untrained_vo2max")?)?,
            strength_by_age: by_age(table("grip_strength_by_age")?)?,
            lean_fraction: [
                value(fat_free, "adult", "male")?,
                value(fat_free, "adult", "female")?,
            ],
        })
    }

    /// The population's untrained VO2max, strength (kg) and lean mass
    /// (kg) for a body of this sex, age and mass.
    pub fn norms(&self, female: bool, age_years: f64, mass_kg: f64) -> [f64; 3] {
        let s = usize::from(female);
        [
            at_age(&self.vo2max_by_age[s], age_years),
            at_age(&self.strength_by_age[s], age_years),
            self.lean_fraction[s] * mass_kg,
        ]
    }
}

/// What a person did in a day that trains them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct DayWork {
    /// Hours spent hard enough to train the aerobic system.
    pub aerobic_hours: f64,
    /// The heaviest load they lifted or carried (kg).
    pub heaviest_lift_kg: f64,
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
    pub strength_kg: f64,
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
    pub inherited: [f64; 3],
}

impl Fitness {
    /// An untrained body at the population's norms, scaled by `inherited`.
    pub fn untrained(
        models: &TrainingModels,
        female: bool,
        age_years: f64,
        mass_kg: f64,
        inherited: [f64; 3],
    ) -> Self {
        let n = models.norms(female, age_years, mass_kg);
        let base = [
            n[0] * inherited[0],
            n[1] * inherited[1],
            n[2] * inherited[2],
        ];
        Self {
            baseline_vo2max: base[0],
            baseline_strength_kg: base[1],
            baseline_lean_kg: base[2],
            vo2max_ml_kg_min: base[0],
            strength_kg: base[1],
            lean_kg: base[2],
            week_aerobic_hours: Vec::new(),
            week_strength_sessions: Vec::new(),
            today: DayWork::default(),
            inherited,
        }
    }

    /// Book `seconds` of work at `met` lifting or carrying `load_kg`.
    pub fn record(&mut self, models: &TrainingModels, met: f64, seconds: f64, load_kg: f64) {
        let oxygen = met * models.met_oxygen_ml_kg_min;
        if oxygen >= models.aerobic_intensity_fraction * self.vo2max_ml_kg_min {
            self.today.aerobic_hours += seconds.max(0.0) / 3_600.0;
        }
        self.today.heaviest_lift_kg = self.today.heaviest_lift_kg.max(load_kg);
    }

    /// Close one day for a body now of this age and mass: the untrained
    /// baselines follow the body (a child's grow, an old person's fall),
    /// today's work joins the week, and the week's dose trains or detrains
    /// each quantity for the day.
    pub fn end_day(&mut self, models: &TrainingModels, female: bool, age_years: f64, mass_kg: f64) {
        let n = models.norms(female, age_years, mass_kg);
        self.baseline_vo2max = n[0] * self.inherited[0];
        self.baseline_strength_kg = n[1] * self.inherited[1];
        self.baseline_lean_kg = n[2] * self.inherited[2];
        let today = std::mem::take(&mut self.today);
        let trained_strength =
            today.heaviest_lift_kg >= models.strength_load_fraction * self.strength_kg;
        self.week_aerobic_hours.push(today.aerobic_hours);
        self.week_strength_sessions.push(trained_strength);
        while self.week_aerobic_hours.len() > 7 {
            self.week_aerobic_hours.remove(0);
        }
        while self.week_strength_sessions.len() > 7 {
            self.week_strength_sessions.remove(0);
        }
        let aerobic_dose =
            self.week_aerobic_hours.iter().sum::<f64>() / models.aerobic_reference_hours_per_week;
        let strength_dose = self.week_strength_sessions.iter().filter(|t| **t).count() as f64
            / models.strength_reference_sessions_per_week;
        self.vo2max_ml_kg_min = models.aerobic.step(
            self.vo2max_ml_kg_min,
            self.baseline_vo2max,
            aerobic_dose,
            1.0,
        );
        self.strength_kg = models.strength.step(
            self.strength_kg,
            self.baseline_strength_kg,
            strength_dose,
            1.0,
        );
        self.lean_kg = models
            .lean
            .step(self.lean_kg, self.baseline_lean_kg, strength_dose, 1.0);
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

    const ADULT: f64 = 30.0;

    /// A body of fixed sex, age and mass doing the same each day.
    fn train_days(
        f: &mut Fitness,
        m: &TrainingModels,
        (female, mass): (bool, f64),
        days: usize,
        aerobic_h: f64,
        lift: bool,
    ) {
        for _ in 0..days {
            let lift_kg = if lift {
                f.strength_kg * m.strength_load_fraction
            } else {
                0.0
            };
            f.today = DayWork {
                aerobic_hours: aerobic_h,
                heaviest_lift_kg: lift_kg,
            };
            f.end_day(m, female, ADULT, mass);
        }
    }

    fn reference_daily_hours(m: &TrainingModels) -> f64 {
        m.aerobic_reference_hours_per_week / 7.0
    }

    #[test]
    fn the_reference_programme_reaches_the_cited_gain_in_its_time() {
        let m = models();
        let mut f = Fitness::untrained(&m, false, ADULT, 75.0, [1.0; 3]);
        // A week to build the dose, then the programme's length.
        let days = 7 + m.aerobic.gain_days as usize;
        train_days(
            &mut f,
            &m,
            (false, 75.0),
            days,
            reference_daily_hours(&m),
            true,
        );
        let gain = f.vo2max_ml_kg_min / f.baseline_vo2max - 1.0;
        assert!((gain - m.aerobic.gain_fraction).abs() < 1e-9, "{gain}");
        let strength = f.strength_kg / f.baseline_strength_kg - 1.0;
        assert!(
            (strength - m.strength.gain_fraction).abs() < 1e-9,
            "{strength}"
        );
        // And no further: the study's end point is the cap.
        train_days(
            &mut f,
            &m,
            (false, 75.0),
            365,
            reference_daily_hours(&m),
            true,
        );
        assert!(
            (f.vo2max_ml_kg_min / f.baseline_vo2max - 1.0 - m.aerobic.gain_fraction).abs() < 1e-9
        );
    }

    #[test]
    fn halfway_through_the_programme_is_half_the_gain() {
        let m = models();
        let mut f = Fitness::untrained(&m, true, ADULT, 62.0, [1.0; 3]);
        train_days(
            &mut f,
            &m,
            (true, 62.0),
            7,
            reference_daily_hours(&m),
            false,
        );
        let start = f.vo2max_ml_kg_min;
        train_days(
            &mut f,
            &m,
            (true, 62.0),
            (m.aerobic.gain_days / 2.0) as usize,
            reference_daily_hours(&m),
            false,
        );
        let gain = (f.vo2max_ml_kg_min - start) / f.baseline_vo2max;
        assert!(
            (gain - m.aerobic.gain_fraction * 0.5).abs() < 0.01 * m.aerobic.gain_fraction,
            "{gain}"
        );
    }

    #[test]
    fn idleness_loses_what_training_built_but_not_below_baseline() {
        let m = models();
        let mut f = Fitness::untrained(&m, true, ADULT, 62.0, [1.0; 3]);
        train_days(
            &mut f,
            &m,
            (true, 62.0),
            200,
            reference_daily_hours(&m),
            true,
        );
        let trained = f.clone();
        train_days(&mut f, &m, (true, 62.0), 21, 0.0, false);
        assert!(f.vo2max_ml_kg_min < trained.vo2max_ml_kg_min);
        assert!(f.strength_kg < trained.strength_kg);
        assert!(f.lean_kg < trained.lean_kg);
        train_days(&mut f, &m, (true, 62.0), 3_000, 0.0, false);
        assert!((f.vo2max_ml_kg_min - f.baseline_vo2max).abs() < 1e-9);
        assert!((f.strength_kg - f.baseline_strength_kg).abs() < 1e-9);
    }

    #[test]
    fn half_the_training_buys_half_the_gain() {
        let m = models();
        let mut full = Fitness::untrained(&m, false, ADULT, 80.0, [1.0; 3]);
        let mut half = full.clone();
        let h = reference_daily_hours(&m);
        train_days(&mut full, &m, (false, 80.0), 400, h, false);
        train_days(&mut half, &m, (false, 80.0), 400, h / 2.0, false);
        let g_full = full.vo2max_ml_kg_min / full.baseline_vo2max - 1.0;
        let g_half = half.vo2max_ml_kg_min / half.baseline_vo2max - 1.0;
        assert!((g_half - g_full / 2.0).abs() < 1e-9, "{g_half} {g_full}");
    }

    #[test]
    fn light_lifts_and_easy_work_do_not_train() {
        let m = models();
        let mut f = Fitness::untrained(&m, false, ADULT, 80.0, [1.0; 3]);
        let easy_met =
            0.9 * m.aerobic_intensity_fraction * f.vo2max_ml_kg_min / m.met_oxygen_ml_kg_min;
        for _ in 0..90 {
            let light = 0.3 * m.strength_load_fraction * f.strength_kg;
            f.record(&m, easy_met, 3_600.0, light);
            f.end_day(&m, false, ADULT, 80.0);
        }
        assert!((f.strength_kg - f.baseline_strength_kg).abs() < 1e-9);
        assert!(f.week_aerobic_hours.iter().all(|h| *h == 0.0));
    }

    #[test]
    fn hard_work_counts_against_the_persons_own_capacity() {
        let m = models();
        let fit = Fitness::untrained(&m, false, ADULT, 80.0, [1.3, 1.0, 1.0]);
        let unfit = Fitness::untrained(&m, false, ADULT, 80.0, [0.7, 1.0, 1.0]);
        // The same work: hard for the unfit, easy for the fit.
        let met =
            m.aerobic_intensity_fraction * m.norms(false, ADULT, 80.0)[0] / m.met_oxygen_ml_kg_min;
        let (mut a, mut b) = (fit, unfit);
        a.record(&m, met, 1_800.0, 25.0);
        b.record(&m, met, 1_800.0, 25.0);
        assert_eq!(a.today.aerobic_hours, 0.0);
        assert!((b.today.aerobic_hours - 0.5).abs() < 1e-12);
        assert_eq!(b.today.heaviest_lift_kg, 25.0);
    }

    #[test]
    fn baselines_follow_age_and_sex() {
        let m = models();
        let child = m.norms(true, 10.0, 32.0);
        let woman = m.norms(true, 30.0, 62.0);
        let man = m.norms(false, 30.0, 80.0);
        let old_man = m.norms(false, 75.0, 80.0);
        assert!(child[1] < woman[1] && woman[1] < man[1]);
        assert!(old_man[1] < man[1]);
        assert!(woman[0] < man[0]);
        // Between two tabled ages the norm lies between them.
        let curve = &m.strength_by_age[0];
        let mid = (curve[0].0 + curve[1].0) / 2.0;
        let v = at_age(curve, mid);
        assert!((v - (curve[0].1 + curve[1].1) / 2.0).abs() < 1e-9);
    }
}
