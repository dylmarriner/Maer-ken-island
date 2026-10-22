//! Learning by doing (realism R11): no one is born able to do a task. Each
//! person's skill at a task is the experience they have built up doing it,
//! counted in the task's own units (trees felled, cubic metres split, days
//! panned). A beginner takes longer than the packs' time for the task, a
//! practised hand takes the packs' time, and experience fades with time
//! away from the task.
//!
//! - A beginner works at `novice_performance_fraction_of_mean` (Purfürst
//!   2010: 50-60% of the mean operator), so the first unit takes the packs'
//!   time over that fraction.
//! - Each unit after follows Crawford's unit learning curve
//!   T_n = T_1 n^b, with b = `learning_index_novice_manual_felling`
//!   (Silayo et al. 2014: -0.281 for crews new to the crosscut saw).
//! - Time never falls below the packs' time, which is a practised worker's.
//! - Away from the task, a share of the experience is lost per three months
//!   (Silayo et al. 2014: 19% for start-up crews, 9.4% for experienced
//!   ones), compounding.
//!
//! The numbers come from one manual task, crosscut felling; applying them
//! to splitting and panning is a stand-in (DEVIATIONS D64). Skill only sets
//! how long a task takes: what a person does is their own choice.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::validation::{ReferenceDomain, ReferenceLibrary};

/// Seconds in a mean month (365.2425 / 12 days).
const MONTH_S: f64 = 365.2425 / 12.0 * 86_400.0;

/// A task a person gets better at by doing it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Task {
    /// Felling a tree (one unit per tree).
    Felling,
    /// Splitting firewood (one unit per solid cubic metre).
    Splitting,
    /// Panning a placer (one unit per working day at the pan).
    Panning,
}

/// One person's experience at one task.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct Skill {
    /// Units of the task done, less what has been forgotten.
    pub units: f64,
    /// When the task was last practised (simulation seconds).
    pub last_practised_s: f64,
}

/// The learning and forgetting rates, from `fixtures/reference/humans/
/// learning.json`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LearningRates {
    /// A beginner's performance as a share of a practised worker's.
    pub novice_fraction: f64,
    /// The learning index b of T_n = T_1 n^b.
    pub learning_index: f64,
    /// Share of experience lost per break, while still slower than the
    /// packs' time (start-up) and once at it (experienced).
    pub forgotten_start_up: f64,
    pub forgotten_experienced: f64,
    /// The break those shares are measured over (seconds).
    pub break_s: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SkillsError {
    MissingReference(String),
    Invalid(String),
}

impl std::fmt::Display for SkillsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingReference(k) => write!(f, "learning pack lacks {k}"),
            Self::Invalid(m) => write!(f, "learning pack: {m}"),
        }
    }
}

impl std::error::Error for SkillsError {}

impl LearningRates {
    pub fn from_library(library: &ReferenceLibrary) -> Result<Self, SkillsError> {
        let missing = |k: &str| SkillsError::MissingReference(k.to_string());
        let h = ReferenceDomain::Humans;
        let novice_fraction = library
            .item(h, "novice_performance_fraction_of_mean")
            .and_then(|i| i.central())
            .ok_or_else(|| missing("novice_performance_fraction_of_mean"))?;
        let learning_index = library
            .item(h, "learning_index_novice_manual_felling")
            .and_then(|i| i.point())
            .ok_or_else(|| missing("learning_index_novice_manual_felling"))?;
        let table = library
            .item(h, "experience_forgotten_per_break")
            .and_then(|i| i.table())
            .ok_or_else(|| missing("experience_forgotten_per_break"))?;
        let get = |row: &str, col: &str| {
            table
                .get(row, col)
                .ok_or_else(|| missing(&format!("experience_forgotten_per_break/{row}/{col}")))
        };
        let forgotten_start_up = get("start_up", "forgetting_factor")?;
        let forgotten_experienced = get("experienced", "forgetting_factor")?;
        let months = get("start_up", "break_months")?;
        if get("experienced", "break_months")? != months {
            return Err(SkillsError::Invalid(
                "the two crews' breaks differ in length".to_string(),
            ));
        }
        let rates = Self {
            novice_fraction,
            learning_index,
            forgotten_start_up,
            forgotten_experienced,
            break_s: months * MONTH_S,
        };
        let fraction = |v: f64| v > 0.0 && v < 1.0;
        if !(fraction(rates.novice_fraction)
            && rates.learning_index < 0.0
            && fraction(rates.forgotten_start_up)
            && fraction(rates.forgotten_experienced)
            && rates.break_s > 0.0)
        {
            return Err(SkillsError::Invalid(format!("{rates:?}")));
        }
        Ok(rates)
    }

    /// The experience after which a person works at the packs' time:
    /// T_1 n^b = T_pack with T_1 = T_pack / novice_fraction.
    pub fn practised_units(&self) -> f64 {
        self.novice_fraction.powf(1.0 / self.learning_index)
    }
}

impl Skill {
    /// How many times the packs' time the next unit takes: the unit
    /// learning curve at the unit about to be done, never below one.
    pub fn time_factor(&self, rates: &LearningRates) -> f64 {
        ((self.units + 1.0).powf(rates.learning_index) / rates.novice_fraction).max(1.0)
    }

    /// Experience left after the time since the task was last practised.
    pub fn units_at(&self, rates: &LearningRates, now_s: f64) -> f64 {
        let away_s = (now_s - self.last_practised_s).max(0.0);
        if away_s == 0.0 || self.units == 0.0 {
            return self.units;
        }
        let lost = if self.time_factor(rates) > 1.0 {
            rates.forgotten_start_up
        } else {
            rates.forgotten_experienced
        };
        self.units * (1.0 - lost).powf(away_s / rates.break_s)
    }
}

/// Every person's skills, keyed by agent id: every human on the island,
/// not only the founders.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Skills(pub BTreeMap<String, BTreeMap<Task, Skill>>);

impl Skills {
    /// How many times the packs' time `who`'s next unit of `task` takes at
    /// `now_s`, forgetting included.
    pub fn time_factor(&self, rates: &LearningRates, who: &str, task: Task, now_s: f64) -> f64 {
        let skill = self
            .0
            .get(who)
            .and_then(|s| s.get(&task))
            .copied()
            .unwrap_or_default();
        Skill {
            units: skill.units_at(rates, now_s),
            last_practised_s: now_s,
        }
        .time_factor(rates)
    }

    /// Books `units` of `task` done by `who` at `now_s`, after first taking
    /// off what was forgotten since the last practice.
    pub fn practise(
        &mut self,
        rates: &LearningRates,
        who: &str,
        task: Task,
        units: f64,
        now_s: f64,
    ) {
        let skill = self
            .0
            .entry(who.to_string())
            .or_default()
            .entry(task)
            .or_default();
        skill.units = skill.units_at(rates, now_s) + units.max(0.0);
        skill.last_practised_s = now_s;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rates() -> LearningRates {
        let library = ReferenceLibrary::load(&crate::validation::default_reference_dir()).unwrap();
        LearningRates::from_library(&library).unwrap()
    }

    #[test]
    fn the_pack_gives_silayos_and_purfursts_numbers() {
        let r = rates();
        assert_eq!(r.learning_index, -0.281);
        assert!((r.novice_fraction - 0.55).abs() < 1e-12);
        assert_eq!(r.forgotten_start_up, 0.19);
        assert_eq!(r.forgotten_experienced, 0.094);
        assert!((r.break_s / 86_400.0 - 91.31).abs() < 0.01);
    }

    #[test]
    fn a_beginner_is_slow_and_reaches_the_packs_time_with_practice() {
        let r = rates();
        let mut skills = Skills::default();
        let first = skills.time_factor(&r, "a", Task::Felling, 0.0);
        assert!((first - 1.0 / 0.55).abs() < 1e-12, "{first}");
        let mut last = first;
        for n in 1..=20 {
            skills.practise(&r, "a", Task::Felling, 1.0, 0.0);
            let next = skills.time_factor(&r, "a", Task::Felling, 0.0);
            assert!(next <= last, "unit {n}: {next} after {last}");
            last = next;
        }
        assert_eq!(last, 1.0);
        // 0.55^(1 / -0.281): about eight trees to reach the packs' time.
        let n = r.practised_units();
        assert!((8.0..9.0).contains(&n), "{n}");
        // Skill at one task is not skill at another, nor someone else's.
        assert_eq!(skills.time_factor(&r, "a", Task::Splitting, 0.0), first);
        assert_eq!(skills.time_factor(&r, "b", Task::Felling, 0.0), first);
    }

    #[test]
    fn experience_fades_with_time_away() {
        let r = rates();
        let mut skills = Skills::default();
        skills.practise(&r, "a", Task::Panning, 4.0, 0.0);
        let s = skills.0["a"][&Task::Panning];
        // Still a beginner: a start-up crew's 19% per three months.
        let after = s.units_at(&r, r.break_s);
        assert!((after - 4.0 * 0.81).abs() < 1e-9, "{after}");
        assert!(
            skills.time_factor(&r, "a", Task::Panning, r.break_s)
                > skills.time_factor(&r, "a", Task::Panning, 0.0)
        );
        // Practised: an experienced crew's 9.4%.
        skills.practise(&r, "a", Task::Panning, 96.0, 0.0);
        let s = skills.0["a"][&Task::Panning];
        let after = s.units_at(&r, 2.0 * r.break_s);
        assert!((after - 100.0 * 0.906_f64.powi(2)).abs() < 1e-9, "{after}");
    }
}
