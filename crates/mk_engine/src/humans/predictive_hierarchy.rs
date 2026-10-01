//! Hierarchical predictive-coding network: canon
//! `FormalPredictiveProcessingSchema.hierarchical_models` and
//! `error_minimization` (`docs/canon/HumanReplicationSchema.js`).
//!
//! Four levels, as in the canon schema: `level_0_sensory`,
//! `level_1_features`, `level_2_concepts` and `level_3_goals`. Every level
//! above the first holds a latent state vector and generates a prediction of
//! the level below through learned weights: the features' `causal_model`
//! predicts the senses, the concepts' `semantic_network` predicts the
//! features, and the goals' `goal_model` predicts the concepts. The goal
//! level itself is predicted by a prior, the need-weighted value of each
//! goal, which is where the body's drives enter perception.
//!
//! This is the linear Gaussian predictive-coding model of Rao & Ballard
//! (1999, *Nature Neuroscience* 2:79) in the free-energy form of Friston
//! (2005, *Phil. Trans. R. Soc. B* 360:815). Each step:
//!
//! - **Inference.** The latent states settle by gradient descent on the
//!   variational free energy `F = ½ Σ (Π·ε² + ln σ²)`, where `ε` is each
//!   unit's prediction error, `σ²` its error variance and `Π = 1/σ²` its
//!   precision. Error flows up (a level is pulled toward explaining the
//!   precision-weighted error below it) and down (it is pulled toward what
//!   the level above predicts). The descent step is bounded by the
//!   gradient's Lipschitz constant, so settling is stable for any weights
//!   and precisions.
//! - **Learning.** Generative weights follow the Hebbian rule
//!   `ΔW = η·(Π·ε)·rᵀ`, which descends the same free energy.
//! - **Precision.** Each unit's error variance relaxes toward the squared
//!   error it has actually seen, so a reliably predicted channel earns
//!   precision and a surprising one loses it.
//!
//! Learning and precision rates are per hour of simulated time and are
//! integrated over the real step length, so the model learns the same
//! amount per simulated hour however finely time is stepped.

use crate::humans::rates;
use serde::{Deserialize, Serialize};

/// The level-0 sensory channels, in the order they are fed in: the eight
/// world signals every human observes, then the three interoceptive drives.
pub const SENSORY_CHANNELS: [&str; 11] = [
    "ambient_temperature",
    "resource_abundance",
    "hazard",
    "caloric_access",
    "hydration_access",
    "shelter_quality",
    "social_density",
    "daylight",
    "hunger",
    "thirst",
    "fatigue",
];
/// Latent units at level 1 (features).
pub const FEATURE_UNITS: usize = 4;
/// Latent units at level 2 (concepts).
pub const CONCEPT_UNITS: usize = 3;
/// Latent units at level 3 (goals): one per goal the human can pursue.
pub const GOAL_UNITS: usize = 5;

/// Gradient-descent iterations per step for the latent states to settle.
const INFERENCE_ITERATIONS: usize = 32;
/// Error-variance bounds, so precision stays within `[1/2, 20]`.
const MIN_VARIANCE: f64 = 0.05;
const MAX_VARIANCE: f64 = 2.0;
/// Bounds on latent states and weights, keeping the linear model finite
/// under extreme inputs.
const STATE_BOUND: f64 = 3.0;
const WEIGHT_BOUND: f64 = 3.0;

/// Canon belief-revision traits that shape inference and learning.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RevisionTraits {
    /// 0-1: at 1, sensory evidence moves beliefs half as hard.
    pub confirmation_bias: f64,
    /// 0-1: how fast error variance tracks surprise (a sensitive mind loses
    /// confidence faster after a large error).
    pub error_sensitivity: f64,
}

/// One level of the hierarchy.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PredictiveLevel {
    /// For level 0, the sensed input (centred on zero); above it, the
    /// settled latent state.
    pub state: Vec<f64>,
    /// What the level above (or, for the goals, the prior) predicted this
    /// level's state to be.
    pub prediction: Vec<f64>,
    /// `state − prediction` after settling.
    pub prediction_error: Vec<f64>,
    /// Running estimate of each unit's error variance `σ²`.
    pub variance: Vec<f64>,
    /// Weight- and variance-learning rate, per hour of simulated time.
    pub learning_rate: f64,
}

impl PredictiveLevel {
    fn new(units: usize, variance: f64, learning_rate: f64) -> Self {
        Self {
            state: vec![0.0; units],
            prediction: vec![0.0; units],
            prediction_error: vec![0.0; units],
            variance: vec![variance.clamp(MIN_VARIANCE, MAX_VARIANCE); units],
            learning_rate,
        }
    }

    /// Precision `Π = 1/σ²` of unit `i`.
    pub fn precision(&self, i: usize) -> f64 {
        1.0 / self.variance[i].clamp(MIN_VARIANCE, MAX_VARIANCE)
    }

    /// Mean `Π/(1+Π)` over the level: 0.5 at unit precision, tending to 1
    /// as the level becomes certain.
    pub fn confidence(&self) -> f64 {
        if self.variance.is_empty() {
            return 0.5;
        }
        let n = self.variance.len() as f64;
        (0..self.variance.len())
            .map(|i| {
                let p = self.precision(i);
                p / (1.0 + p)
            })
            .sum::<f64>()
            / n
    }
}

/// Seed values for a new hierarchy, taken from the human's canon entries
/// where they are filled in.
#[derive(Debug, Clone, Default)]
pub struct HierarchySeed {
    /// Initial error variance per level (from canon precision or variance).
    pub variance: [Option<f64>; 4],
    /// Learning rate per level, per hour.
    pub learning_rate: [Option<f64>; 4],
    /// `causal_model[k][channel]`: weight from feature `k` to a sensory
    /// channel, by channel name.
    pub causal_model: Vec<Vec<(String, f64)>>,
    /// `semantic_network[k][feature_j]`: weight from concept `k` to feature
    /// `j`, keyed `feature_j`.
    pub semantic_network: Vec<Vec<(String, f64)>>,
    /// `utility_function[g][goal]`: weight from goal unit `g` to concept
    /// units, keyed `concept_j`.
    pub goal_model: Vec<Vec<(String, f64)>>,
}

/// The four-level hierarchy and its learned generative weights.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PredictiveHierarchy {
    pub sensory: PredictiveLevel,
    pub features: PredictiveLevel,
    pub concepts: PredictiveLevel,
    pub goals: PredictiveLevel,
    /// `[channel][feature]`: how each feature generates each sensory channel.
    pub causal_model: Vec<Vec<f64>>,
    /// `[feature][concept]`: how each concept generates each feature.
    pub semantic_network: Vec<Vec<f64>>,
    /// `[concept][goal]`: how each goal generates each concept.
    pub goal_model: Vec<Vec<f64>>,
    /// Variational free energy after this step's settling.
    pub free_energy: f64,
    /// Mean |precision-weighted sensory error| sent up this step: how hard
    /// the senses pushed on the beliefs above them.
    pub upward_error: f64,
}

/// A deterministic draw in `[-1, 1)` for weight initialisation (SplitMix64).
fn init_draw(seed: u64, index: u64) -> f64 {
    let mut z = seed
        .wrapping_add(0x9E37_79B9_7F4A_7C15)
        .wrapping_add(index.wrapping_mul(0xBF58_476D_1CE4_E5B9));
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^= z >> 31;
    (z >> 11) as f64 / (1u64 << 53) as f64 * 2.0 - 1.0
}

fn init_weights(rows: usize, cols: usize, seed: u64, salt: u64, scale: f64) -> Vec<Vec<f64>> {
    (0..rows)
        .map(|r| {
            (0..cols)
                .map(|c| scale * init_draw(seed ^ salt, (r * cols + c) as u64))
                .collect()
        })
        .collect()
}

/// Overwrite weight column `unit` of `weights` from named canon entries.
fn apply_named(weights: &mut [Vec<f64>], unit: usize, entries: &[(String, f64)], names: &[String]) {
    for (name, value) in entries {
        if let Some(row) = names.iter().position(|n| n == name) {
            if value.is_finite() {
                weights[row][unit] = value.clamp(-WEIGHT_BOUND, WEIGHT_BOUND);
            }
        }
    }
}

fn mat_vec(weights: &[Vec<f64>], v: &[f64]) -> Vec<f64> {
    weights
        .iter()
        .map(|row| row.iter().zip(v).map(|(w, x)| w * x).sum())
        .collect()
}

fn mat_t_vec(weights: &[Vec<f64>], v: &[f64], cols: usize) -> Vec<f64> {
    let mut out = vec![0.0; cols];
    for (row, e) in weights.iter().zip(v) {
        for (o, w) in out.iter_mut().zip(row) {
            *o += w * e;
        }
    }
    out
}

fn frobenius_sq(weights: &[Vec<f64>]) -> f64 {
    weights.iter().flatten().map(|w| w * w).sum()
}

fn max_precision(level: &PredictiveLevel) -> f64 {
    (0..level.variance.len())
        .map(|i| level.precision(i))
        .fold(0.0, f64::max)
}

impl PredictiveHierarchy {
    /// A new hierarchy for the human with `seed` (its id), from canon seed
    /// values where given and small deterministic random weights otherwise.
    pub fn new(
        seed: u64,
        initial_variance: f64,
        learning_rate: f64,
        canon: &HierarchySeed,
    ) -> Self {
        let level = |index: usize, units: usize, rate_scale: f64| {
            PredictiveLevel::new(
                units,
                canon.variance[index].unwrap_or(initial_variance),
                canon.learning_rate[index].unwrap_or(learning_rate * rate_scale),
            )
        };
        let n0 = SENSORY_CHANNELS.len();
        let mut causal_model = init_weights(n0, FEATURE_UNITS, seed, 0x0101, 0.3);
        let mut semantic_network = init_weights(FEATURE_UNITS, CONCEPT_UNITS, seed, 0x0202, 0.3);
        let mut goal_model = init_weights(CONCEPT_UNITS, GOAL_UNITS, seed, 0x0303, 0.3);

        let channels: Vec<String> = SENSORY_CHANNELS.iter().map(|s| s.to_string()).collect();
        let features: Vec<String> = (0..FEATURE_UNITS).map(|j| format!("feature_{j}")).collect();
        let concepts: Vec<String> = (0..CONCEPT_UNITS).map(|j| format!("concept_{j}")).collect();
        for (unit, entries) in canon.causal_model.iter().enumerate().take(FEATURE_UNITS) {
            apply_named(&mut causal_model, unit, entries, &channels);
        }
        for (unit, entries) in canon
            .semantic_network
            .iter()
            .enumerate()
            .take(CONCEPT_UNITS)
        {
            apply_named(&mut semantic_network, unit, entries, &features);
        }
        for (unit, entries) in canon.goal_model.iter().enumerate().take(GOAL_UNITS) {
            apply_named(&mut goal_model, unit, entries, &concepts);
        }

        Self {
            sensory: level(0, n0, 1.0),
            features: level(1, FEATURE_UNITS, 0.7),
            concepts: level(2, CONCEPT_UNITS, 0.5),
            goals: level(3, GOAL_UNITS, 0.4),
            causal_model,
            semantic_network,
            goal_model,
            free_energy: 0.0,
            upward_error: 0.0,
        }
    }

    /// Whether this hierarchy has been built (a snapshot saved before the
    /// hierarchy existed deserializes to an empty one).
    pub fn is_built(&self) -> bool {
        self.sensory.state.len() == SENSORY_CHANNELS.len()
            && self.features.state.len() == FEATURE_UNITS
            && self.concepts.state.len() == CONCEPT_UNITS
            && self.goals.state.len() == GOAL_UNITS
            && self.causal_model.len() == SENSORY_CHANNELS.len()
            && self.semantic_network.len() == FEATURE_UNITS
            && self.goal_model.len() == CONCEPT_UNITS
    }

    /// Variational free energy of the current states: `½ Σ (Π·ε² + ln σ²)`
    /// over every unit of every level.
    fn free_energy_of(&self, errors: [&[f64]; 4]) -> f64 {
        let levels = [&self.sensory, &self.features, &self.concepts, &self.goals];
        let mut f = 0.0;
        for (level, error) in levels.iter().zip(errors) {
            for (i, e) in error.iter().enumerate() {
                f += 0.5 * (level.precision(i) * e * e + level.variance[i].ln());
            }
        }
        f
    }

    /// Settle, learn and adapt precision for one step.
    ///
    /// `sensed` is the raw level-0 input in `[0, 1]` per channel of
    /// [`SENSORY_CHANNELS`]; `goal_prior` is the need-weighted value of each
    /// goal, the prior the goal level is pulled toward.
    pub fn step(
        &self,
        sensed: &[f64; SENSORY_CHANNELS.len()],
        goal_prior: &[f64; GOAL_UNITS],
        dt_years: f64,
        traits: RevisionTraits,
    ) -> Self {
        let mut next = self.clone();
        // Inputs centred on zero: the linear generative model has no bias
        // term, and a mid-range signal is then "nothing to explain".
        let x: Vec<f64> = sensed.iter().map(|s| s.clamp(0.0, 1.0) - 0.5).collect();
        let prior: Vec<f64> = goal_prior.iter().map(|g| g.clamp(0.0, 1.0) - 0.5).collect();
        let bottom_up_gain = 1.0 - 0.5 * traits.confirmation_bias.clamp(0.0, 1.0);

        let pi = |level: &PredictiveLevel| -> Vec<f64> {
            (0..level.variance.len())
                .map(|i| level.precision(i))
                .collect()
        };
        let (pi0, pi1, pi2, pi3) = (
            pi(&self.sensory),
            pi(&self.features),
            pi(&self.concepts),
            pi(&self.goals),
        );

        // Descent step bounded by the Lipschitz constant of ∂F/∂r for each
        // latent level: its own precision plus the precision-weighted
        // squared norm of the weights predicting the level below.
        let lipschitz = [
            max_precision(&self.features)
                + bottom_up_gain * max_precision(&self.sensory) * frobenius_sq(&self.causal_model),
            max_precision(&self.concepts)
                + max_precision(&self.features) * frobenius_sq(&self.semantic_network),
            max_precision(&self.goals)
                + max_precision(&self.concepts) * frobenius_sq(&self.goal_model),
        ];
        let rate: Vec<f64> = lipschitz.iter().map(|l| 1.0 / l.max(1e-6)).collect();

        let (mut r1, mut r2, mut r3) = (
            self.features.state.clone(),
            self.concepts.state.clone(),
            self.goals.state.clone(),
        );
        let weighted = |error: &[f64], precision: &[f64]| -> Vec<f64> {
            error.iter().zip(precision).map(|(e, p)| e * p).collect()
        };
        let diff =
            |a: &[f64], b: &[f64]| -> Vec<f64> { a.iter().zip(b).map(|(x, y)| x - y).collect() };

        for _ in 0..INFERENCE_ITERATIONS {
            let e0 = weighted(&diff(&x, &mat_vec(&self.causal_model, &r1)), &pi0);
            let e1 = weighted(&diff(&r1, &mat_vec(&self.semantic_network, &r2)), &pi1);
            let e2 = weighted(&diff(&r2, &mat_vec(&self.goal_model, &r3)), &pi2);
            let e3 = weighted(&diff(&r3, &prior), &pi3);
            // Each latent level moves to explain the error below it and
            // toward what the level above predicts.
            let up1 = mat_t_vec(&self.causal_model, &e0, FEATURE_UNITS);
            let up2 = mat_t_vec(&self.semantic_network, &e1, CONCEPT_UNITS);
            let up3 = mat_t_vec(&self.goal_model, &e2, GOAL_UNITS);
            for (j, r) in r1.iter_mut().enumerate() {
                *r = (*r + rate[0] * (bottom_up_gain * up1[j] - e1[j]))
                    .clamp(-STATE_BOUND, STATE_BOUND);
            }
            for (j, r) in r2.iter_mut().enumerate() {
                *r = (*r + rate[1] * (up2[j] - e2[j])).clamp(-STATE_BOUND, STATE_BOUND);
            }
            for (j, r) in r3.iter_mut().enumerate() {
                *r = (*r + rate[2] * (up3[j] - e3[j])).clamp(-STATE_BOUND, STATE_BOUND);
            }
        }

        // Settled predictions and raw errors.
        let p0 = mat_vec(&self.causal_model, &r1);
        let p1 = mat_vec(&self.semantic_network, &r2);
        let p2 = mat_vec(&self.goal_model, &r3);
        let eps0 = diff(&x, &p0);
        let eps1 = diff(&r1, &p1);
        let eps2 = diff(&r2, &p2);
        let eps3 = diff(&r3, &prior);

        // Hebbian learning of the generative weights, ΔW = η·(Π·ε)·rᵀ.
        let learn = |weights: &mut Vec<Vec<f64>>,
                     error: &[f64],
                     precision: &[f64],
                     cause: &[f64],
                     level: &PredictiveLevel| {
            let gain = rates::relaxation_fraction(level.learning_rate, dt_years, rates::HOUR_YEARS);
            if gain <= 0.0 {
                return;
            }
            for (row, (e, p)) in weights.iter_mut().zip(error.iter().zip(precision)) {
                for (w, r) in row.iter_mut().zip(cause) {
                    *w = (*w + gain * p * e * r).clamp(-WEIGHT_BOUND, WEIGHT_BOUND);
                }
            }
        };
        learn(&mut next.causal_model, &eps0, &pi0, &r1, &self.sensory);
        learn(&mut next.semantic_network, &eps1, &pi1, &r2, &self.features);
        learn(&mut next.goal_model, &eps2, &pi2, &r3, &self.concepts);

        // Precision: each unit's error variance relaxes toward the squared
        // error it just produced.
        let adapt = |level: &mut PredictiveLevel, error: &[f64]| {
            let rate = level.learning_rate * (0.5 + traits.error_sensitivity.clamp(0.0, 1.0));
            let gain = rates::relaxation_fraction(rate, dt_years, rates::HOUR_YEARS);
            for (v, e) in level.variance.iter_mut().zip(error) {
                *v = (*v + gain * (e * e - *v)).clamp(MIN_VARIANCE, MAX_VARIANCE);
            }
        };
        adapt(&mut next.sensory, &eps0);
        adapt(&mut next.features, &eps1);
        adapt(&mut next.concepts, &eps2);
        adapt(&mut next.goals, &eps3);

        next.upward_error = eps0
            .iter()
            .zip(&pi0)
            .map(|(e, p)| (e * p).abs())
            .sum::<f64>()
            / eps0.len() as f64;
        for (level, state, prediction, error) in [
            (&mut next.sensory, x, p0, eps0),
            (&mut next.features, r1, p1, eps1),
            (&mut next.concepts, r2, p2, eps2),
            (&mut next.goals, r3, prior, eps3),
        ] {
            level.state = state;
            level.prediction = prediction;
            level.prediction_error = error;
        }
        next.free_energy = next.free_energy_of([
            &next.sensory.prediction_error,
            &next.features.prediction_error,
            &next.concepts.prediction_error,
            &next.goals.prediction_error,
        ]);
        next
    }

    /// The inferred activation of each goal, in `[0, 1]` (the settled goal
    /// state, un-centred).
    pub fn goal_activation(&self) -> [f64; GOAL_UNITS] {
        std::array::from_fn(|g| {
            (self.goals.state.get(g).copied().unwrap_or(0.0) + 0.5).clamp(0.0, 1.0)
        })
    }

    /// Mean confidence over every level.
    pub fn confidence(&self) -> f64 {
        (self.sensory.confidence()
            + self.features.confidence()
            + self.concepts.confidence()
            + self.goals.confidence())
            / 4.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn traits() -> RevisionTraits {
        RevisionTraits {
            confirmation_bias: 0.0,
            error_sensitivity: 0.5,
        }
    }

    fn hierarchy() -> PredictiveHierarchy {
        PredictiveHierarchy::new(42, 1.0, 0.3, &HierarchySeed::default())
    }

    fn scene(value: f64) -> [f64; SENSORY_CHANNELS.len()] {
        std::array::from_fn(|i| if i % 2 == 0 { value } else { 1.0 - value })
    }

    #[test]
    fn settling_lowers_free_energy_below_the_unsettled_state() {
        let h = hierarchy();
        let sensed = scene(0.9);
        let prior = [0.5; GOAL_UNITS];
        // Free energy with every latent state left at zero, against the
        // same input, versus after settling (with learning switched off).
        let x: Vec<f64> = sensed.iter().map(|s| s - 0.5).collect();
        let unsettled = h.free_energy_of([
            &x,
            &[0.0; FEATURE_UNITS],
            &[0.0; CONCEPT_UNITS],
            &[0.0; GOAL_UNITS],
        ]);
        let settled = h.step(&sensed, &prior, 0.0, traits());
        assert!(
            settled.free_energy < unsettled,
            "{} !< {unsettled}",
            settled.free_energy
        );
        assert!(settled.upward_error > 0.0);
    }

    #[test]
    fn repeated_exposure_teaches_the_model_to_predict_its_senses() {
        let mut h = hierarchy();
        let sensed = scene(0.85);
        let prior = [0.5; GOAL_UNITS];
        h = h.step(&sensed, &prior, rates::HOUR_YEARS, traits());
        let first: f64 = h.sensory.prediction_error.iter().map(|e| e * e).sum();
        for _ in 0..200 {
            h = h.step(&sensed, &prior, rates::HOUR_YEARS, traits());
        }
        let learned: f64 = h.sensory.prediction_error.iter().map(|e| e * e).sum();
        assert!(learned < first * 0.5, "{learned} vs {first}");
    }

    #[test]
    fn a_reliable_world_earns_precision_and_a_surprising_one_loses_it() {
        let prior = [0.5; GOAL_UNITS];
        let mut steady = hierarchy();
        for _ in 0..200 {
            steady = steady.step(&scene(0.8), &prior, rates::HOUR_YEARS, traits());
        }
        let confident = steady.sensory.confidence();
        // A structurally new world (every channel at its extreme, not the
        // learned alternating pattern), which the learned model cannot
        // explain at once.
        let mut shocked = steady.clone();
        for _ in 0..3 {
            shocked = shocked.step(
                &[1.0; SENSORY_CHANNELS.len()],
                &prior,
                rates::HOUR_YEARS,
                traits(),
            );
        }
        assert!(shocked.sensory.confidence() < confident);
        assert!(confident > hierarchy().sensory.confidence());
    }

    #[test]
    fn confirmation_bias_discounts_sensory_evidence() {
        let prior = [0.5; GOAL_UNITS];
        let open = hierarchy().step(&scene(1.0), &prior, 0.0, traits());
        let biased = hierarchy().step(
            &scene(1.0),
            &prior,
            0.0,
            RevisionTraits {
                confirmation_bias: 1.0,
                ..traits()
            },
        );
        let norm = |h: &PredictiveHierarchy| h.features.state.iter().map(|r| r * r).sum::<f64>();
        assert!(norm(&biased) < norm(&open));
    }

    #[test]
    fn sensory_surprise_reaches_the_goals() {
        let prior = [0.5; GOAL_UNITS];
        let quiet = hierarchy().step(&[0.5; SENSORY_CHANNELS.len()], &prior, 0.0, traits());
        let loud = hierarchy().step(&scene(1.0), &prior, 0.0, traits());
        let moved: f64 = quiet
            .goals
            .state
            .iter()
            .zip(&loud.goals.state)
            .map(|(a, b)| (a - b).abs())
            .sum();
        assert!(moved > 0.0, "errors must propagate all the way up");
    }

    #[test]
    fn the_goal_prior_shapes_goal_activation() {
        let mut prior = [0.2; GOAL_UNITS];
        prior[3] = 1.0;
        let h = hierarchy().step(&[0.5; SENSORY_CHANNELS.len()], &prior, 0.0, traits());
        let goals = h.goal_activation();
        let best = (0..GOAL_UNITS)
            .max_by(|a, b| goals[*a].total_cmp(&goals[*b]))
            .unwrap();
        assert_eq!(best, 3);
    }

    #[test]
    fn learning_per_hour_does_not_depend_on_the_step_length() {
        let prior = [0.5; GOAL_UNITS];
        let sensed = scene(0.9);
        let hourly = hierarchy().step(&sensed, &prior, rates::HOUR_YEARS, traits());
        let zero = hierarchy().step(&sensed, &prior, 0.0, traits());
        // No time, no learning; an hour, some learning.
        assert_eq!(zero.causal_model, hierarchy().causal_model);
        assert_ne!(hourly.causal_model, hierarchy().causal_model);
    }

    #[test]
    fn canon_causal_weights_seed_the_generative_model() {
        let seed = HierarchySeed {
            causal_model: vec![vec![("hazard".to_string(), 1.25)]],
            ..HierarchySeed::default()
        };
        let h = PredictiveHierarchy::new(42, 1.0, 0.3, &seed);
        let hazard = SENSORY_CHANNELS
            .iter()
            .position(|c| *c == "hazard")
            .unwrap();
        assert_eq!(h.causal_model[hazard][0], 1.25);
        assert!(h.is_built());
        assert!(!PredictiveHierarchy::default().is_built());
    }
}
