//! Each person's own choice of what to do next (realism R14).
//!
//! Nobody is told what to do. When a person sets out to work, the island
//! lists what they could do now, each with what it would cost them and what
//! it would answer, and the person chooses by their own learned liking for
//! each kind of work, the needs they feel, what they want to become and how
//! tired they are. What happens afterwards changes them: work that went well
//! is liked more, work that hurt or failed is liked less, and a calling they
//! succeed at grows (owner, 2026-10-23: "they need to simulate the closest
//! thing to conscious beings with free will who can be who they want to be
//! and do what they want to do").

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// The kinds of work a person may choose between.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Work {
    Fell,
    Haul,
    Split,
    Pan,
    Forage,
    FetchWater,
    Build,
    Rest,
}

/// What a person may want to become. Drawn from their personality at
/// admission and changed only by their own successes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Calling {
    Provider,
    Builder,
    Prospector,
    Explorer,
}

impl Work {
    /// The calling each kind of work answers.
    fn calling(self) -> Option<Calling> {
        match self {
            Work::Fell | Work::Haul | Work::Split | Work::Forage | Work::FetchWater => {
                Some(Calling::Provider)
            }
            Work::Build => Some(Calling::Builder),
            Work::Pan => Some(Calling::Prospector),
            Work::Rest => None,
        }
    }
}

/// One thing the person could do now.
#[derive(Clone, Debug, PartialEq)]
pub struct Candidate {
    pub work: Work,
    /// How strongly this answers a need the person feels now, 0 to 1
    /// (cold for firewood, hunger for food, thirst for water, want of
    /// money for gold).
    pub need: f64,
    /// The effort it would cost them, 0 to 1 of what they can give in a day.
    pub effort: f64,
}

/// A person's own mind about work.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Will {
    /// Learned liking for each kind of work, -1 to 1; unknown work starts at 0.
    pub liking: BTreeMap<Work, f64>,
    /// How much each calling matters to them, summing to 1.
    pub callings: BTreeMap<Calling, f64>,
    /// How readily they try something new, 0 to 1 (their openness).
    pub openness: f64,
    /// What each place they have worked gave them the last time (a placer's
    /// gold, kg per day there), by the place's id.
    #[serde(default)]
    pub places: BTreeMap<u32, f64>,
}

/// How fast liking and callings move with each outcome. A modelling
/// choice, not a measured rate (R14 is a structure, not a calibration).
const LEARNING: f64 = 0.2;

impl Will {
    /// A new person's mind: no liking yet, callings from their own draws.
    pub fn new(openness: f64, draws: [f64; 4]) -> Self {
        let total: f64 = draws.iter().map(|d| d.max(1e-6)).sum();
        let callings = [
            Calling::Provider,
            Calling::Builder,
            Calling::Prospector,
            Calling::Explorer,
        ]
        .into_iter()
        .zip(draws)
        .map(|(c, d)| (c, d.max(1e-6) / total))
        .collect();
        Self {
            liking: BTreeMap::new(),
            callings,
            openness: openness.clamp(0.0, 1.0),
            places: BTreeMap::new(),
        }
    }

    /// How much the person wants to do `c` now, `fatigue` 0 (fresh) to 1
    /// (spent), `noise` a draw in [0, 1) of their own.
    pub fn score(&self, c: &Candidate, fatigue: f64, noise: f64) -> f64 {
        let liking = self.liking.get(&c.work).copied().unwrap_or(0.0);
        let calling = c
            .work
            .calling()
            .and_then(|k| self.callings.get(&k))
            .copied()
            .unwrap_or(0.0);
        let effort = if c.work == Work::Rest {
            -fatigue
        } else {
            c.effort * (0.5 + fatigue)
        };
        c.need + liking + calling - effort + self.openness * (noise - 0.5)
    }

    /// The person's choice among `candidates`, or `None` if there are none.
    pub fn choose(&self, candidates: &[Candidate], fatigue: f64, noise: &[f64]) -> Option<Work> {
        candidates
            .iter()
            .enumerate()
            .map(|(i, c)| {
                (
                    self.score(c, fatigue, noise.get(i).copied().unwrap_or(0.5)),
                    c.work,
                )
            })
            .max_by(|a, b| a.0.total_cmp(&b.0).then(b.1.cmp(&a.1)))
            .map(|(_, w)| w)
    }

    /// Where they choose to work among `options` (a place's id and how far
    /// it is, 0 to 1 of the farthest), from what each gave them before.
    /// A place they have not tried is worth their openness: the curious go
    /// and look, the cautious return to what they know.
    pub fn choose_place(&self, options: &[(u32, f64)], noise: &[f64]) -> Option<u32> {
        let best = self
            .places
            .values()
            .copied()
            .fold(0.0_f64, f64::max)
            .max(f64::MIN_POSITIVE);
        options
            .iter()
            .enumerate()
            .map(|(i, &(id, far))| {
                let known = self.places.get(&id).map_or(self.openness, |y| y / best);
                let n = noise.get(i).copied().unwrap_or(0.5);
                (known - far + self.openness * (n - 0.5), id)
            })
            .max_by(|a, b| a.0.total_cmp(&b.0).then(b.1.cmp(&a.1)))
            .map(|(_, id)| id)
    }

    /// Remember what `place` gave them this time.
    pub fn remember(&mut self, place: u32, gave: f64) {
        self.places.insert(place, gave.max(0.0));
    }

    /// What came of doing `work`: `outcome` from -1 (hurt, failed,
    /// exhausted) to 1 (it answered the need and went well).
    pub fn learn(&mut self, work: Work, outcome: f64) {
        let outcome = outcome.clamp(-1.0, 1.0);
        let liking = self.liking.entry(work).or_insert(0.0);
        *liking = (*liking + LEARNING * (outcome - *liking)).clamp(-1.0, 1.0);
        if let Some(k) = work.calling() {
            if outcome > 0.0 {
                if let Some(w) = self.callings.get_mut(&k) {
                    *w += LEARNING * outcome * (1.0 - *w);
                }
                let total: f64 = self.callings.values().sum();
                for w in self.callings.values_mut() {
                    *w /= total;
                }
            }
        }
    }
}

/// Every living person's own mind about work, keyed by agent id.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Wills(pub BTreeMap<String, Will>);

/// A uniform draw in [0, 1) from the scenario seed, keyed by `label`.
pub fn draw(seed: &[u8; 32], label: &str) -> f64 {
    let hash = blake3::keyed_hash(seed, label.as_bytes());
    let mut bytes = [0u8; 8];
    bytes.copy_from_slice(&hash.as_bytes()[..8]);
    (u64::from_le_bytes(bytes) >> 11) as f64 / (1u64 << 53) as f64
}

impl Wills {
    /// `id`'s will, made on first use from their openness and their own
    /// draws for what they want to become.
    pub fn of(&mut self, seed: &[u8; 32], id: &str, openness: f64) -> &mut Will {
        self.0.entry(id.to_string()).or_insert_with(|| {
            let draws = [0, 1, 2, 3].map(|k| draw(seed, &format!("calling:{id}:{k}")));
            Will::new(openness, draws)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn people_go_back_to_the_place_that_paid() {
        let mut will = Will::new(0.0, [0.25; 4]);
        will.remember(1, 0.0);
        will.remember(2, 0.004);
        assert_eq!(
            will.choose_place(&[(1, 0.0), (2, 0.5)], &[0.5, 0.5]),
            Some(2)
        );
        // Nothing known pays better than the nearest: they stay close.
        will.remember(2, 0.0);
        assert_eq!(
            will.choose_place(&[(1, 0.0), (2, 0.5)], &[0.5, 0.5]),
            Some(1)
        );
    }

    #[test]
    fn the_curious_try_a_place_they_do_not_know() {
        let mut bold = Will::new(1.0, [0.25; 4]);
        bold.remember(1, 0.001);
        bold.remember(3, 0.004);
        // Place 1 is known to pay a quarter of their best; 2 is unknown.
        assert_eq!(
            bold.choose_place(&[(1, 0.0), (2, 0.2)], &[0.5, 0.5]),
            Some(2)
        );
        let mut wary = Will::new(0.0, [0.25; 4]);
        wary.remember(1, 0.001);
        wary.remember(3, 0.004);
        assert_eq!(
            wary.choose_place(&[(1, 0.0), (2, 0.2)], &[0.5, 0.5]),
            Some(1)
        );
    }

    fn options() -> Vec<Candidate> {
        vec![
            Candidate {
                work: Work::Fell,
                need: 0.3,
                effort: 0.6,
            },
            Candidate {
                work: Work::Pan,
                need: 0.3,
                effort: 0.4,
            },
            Candidate {
                work: Work::Build,
                need: 0.3,
                effort: 0.5,
            },
        ]
    }

    #[test]
    fn two_people_in_the_same_place_choose_by_who_they_are() {
        let prospector = Will::new(0.0, [0.1, 0.1, 0.7, 0.1]);
        let builder = Will::new(0.0, [0.1, 0.7, 0.1, 0.1]);
        assert_eq!(prospector.choose(&options(), 0.0, &[]), Some(Work::Pan));
        assert_eq!(builder.choose(&options(), 0.0, &[]), Some(Work::Build));
    }

    #[test]
    fn work_that_goes_badly_is_chosen_less() {
        let mut will = Will::new(0.0, [0.1, 0.1, 0.7, 0.1]);
        assert_eq!(will.choose(&options(), 0.0, &[]), Some(Work::Pan));
        for _ in 0..10 {
            will.learn(Work::Pan, -1.0);
        }
        assert_ne!(will.choose(&options(), 0.0, &[]), Some(Work::Pan));
    }

    #[test]
    fn success_strengthens_a_calling() {
        let mut will = Will::new(0.0, [0.25; 4]);
        will.learn(Work::Build, 1.0);
        assert!(will.callings[&Calling::Builder] > 0.25);
        let total: f64 = will.callings.values().sum();
        assert!((total - 1.0).abs() < 1e-12);
    }

    #[test]
    fn a_spent_person_rests_and_a_pressing_need_overrides_liking() {
        let will = Will::new(0.0, [0.25; 4]);
        let mut with_rest = options();
        with_rest.push(Candidate {
            work: Work::Rest,
            need: 0.0,
            effort: 0.0,
        });
        assert_eq!(will.choose(&with_rest, 1.0, &[]), Some(Work::Rest));
        let mut cold = options();
        cold[0].need = 1.0;
        assert_eq!(will.choose(&cold, 0.0, &[]), Some(Work::Fell));
    }

    #[test]
    fn an_open_person_sometimes_tries_the_unliked() {
        let mut will = Will::new(1.0, [0.25; 4]);
        will.liking.insert(Work::Pan, -0.3);
        let noise = [0.0, 0.99, 0.0];
        assert_eq!(will.choose(&options(), 0.0, &noise), Some(Work::Pan));
    }
}
