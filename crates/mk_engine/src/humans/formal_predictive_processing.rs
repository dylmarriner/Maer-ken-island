//! Formal Predictive Processing Snapshot - hierarchical prediction-error
//! minimization with precision weighting and deterministic action selection.
//!
//! Surfaces `docs/canon/HumanReplicationSchema.js`'s
//! `FormalPredictiveProcessingSchema`, and the belief-revision traits of
//! `PredictiveProcessingSchema` (`learning_mechanisms` and
//! `consciousness_indicators.subjective_confidence`). It implements the
//! schema's actual *structure* — a real multi-level hierarchy where each level predicts the level below it,
//! prediction errors propagate both up (surprise) and down (belief
//! revision), and precision (inverse variance / confidence) weights how much
//! each level's error moves its beliefs — rather than a single scalar
//! expectation.
//!
//! **The four canon levels:**
//! - **Level 0 (`sensory`)**: per-channel predictions of the real per-tick
//!   [`super::AgentWorldObservation`] signals (temperature, resource
//!   abundance, hazard), each with its own precision.
//! - **Level 1 (`features`, "survival pressure")** and **level 2
//!   (`concepts`, "environmental favorability")**: each is a linear
//!   generative model ([`CausalModel`], Rao & Ballard 1999). One latent
//!   cause predicts every channel below it through a learned loading. The
//!   level infers the cause that best explains its channels, weighting
//!   each by its precision, and revises its belief toward it. It then
//!   predicts the channels top-down, and each channel's error trains its
//!   loading and its error variance, whose inverse is that channel's
//!   precision. Level 1 explains hunger, thirst, the shelter shortfall and
//!   hazard ([`SURVIVAL_CHANNELS`]); canon
//!   `level_1_features[0].causal_model` seeds its loadings. Level 2
//!   explains social density, daylight and resource abundance
//!   ([`FAVORABILITY_CHANNELS`]); canon
//!   `level_2_concepts[0].semantic_network` seeds its loadings. Canon
//!   `precision_attention.expected_uncertainty` seeds each channel's
//!   variance, and `precision_learning` sets how fast it adapts.
//! - **Level 3 (`goals`)**: the goal deficit, which weights the survival
//!   channels by canon `level_3_goals[0].utility_function`. Its drop after
//!   an action is the reward the critic below learns from, and each
//!   action's immediate utility is the deficit it would relieve.
//! - Canon entries left unset fall back to documented engine defaults.
//! - **`active_inference`**: implements genuine policy selection over a
//!   small, real action space ([`FormalAction`]) grounded in this human's
//!   actual tracked [`super::needs::NeedsSnapshot`] deficits and the real
//!   observation affordances that address them (food/water/shelter/social/
//!   rest — the only need-relevant behaviors this engine tracks). Each
//!   action carries two values: its *immediate* affordance utility this
//!   tick, and a *learned* critic value `Q(a)` — an actor-critic estimate
//!   of the discounted survival-pressure relief that has actually followed
//!   taking it. `action_selection` is a real softmax over a blend of the
//!   two (canon `goal_directed_weight` sets the mix, `habit_bias` adds a
//!   repeat-last-action pull, `softmax_temperature` and
//!   `exploitation_vs_exploration` set how flat the distribution is, and
//!   `decisiveness` sharpens it further), sampled from a keyed per-human,
//!   per-tick RNG stream so the choice is genuinely stochastic yet
//!   reproducible for a given seed.
//! - **`policy_update`**: a temporal-difference critic update after each
//!   tick, with reward = realized drop in survival pressure since the
//!   previous action was chosen. When canon `eligibility_traces` is enabled
//!   the update is TD(λ) with replacing traces: every earlier action keeps
//!   a decaying eligibility (`γλ` per tick), so a delayed payoff (e.g.
//!   seeking shelter, then resting safely several ticks later) credits the
//!   whole preceding action sequence rather than only the last step. With
//!   traces disabled it is one-step TD(0).
//! - **Actor**: each action also carries a softmax preference `h(a)`, which
//!   is added to its learned value when scoring. Canon
//!   `policy_gradient_method` picks the policy-gradient update for `h`, with
//!   `π` the probabilities the previous action was sampled from:
//!   `reinforce` uses the reward, `h += α·r·(1[a=prev] − π(a))`;
//!   `actor_critic` uses the TD error, `h += α·δ·(1[a=prev] − π(a))`; and
//!   `natural_gradient` uses the natural gradient, which for a tabular
//!   softmax policy with a compatible critic is the advantage itself,
//!   `h(prev) += α·δ` (Kakade 2001). Without canon the method is
//!   `actor_critic`.

use crate::humans::needs::NeedsSnapshot;
use crate::humans::rates;
use crate::humans::AgentWorldObservation;
use mk_core::human::schema::PolicyGradientMethodSchema;
use mk_core::human::HumanId;
use mk_core::rng::{RngKey, RngRegistry, SubsystemId};
use serde::{Deserialize, Serialize};

/// Keyed RNG epoch for this module's softmax action sampling, distinct from
/// every other `SubsystemId::Humans` consumer's epoch.
const FORMAL_PP_SELECTION_EPOCH: u32 = 0x4650_5053;
/// Critic discount factor γ: how much the next tick's best learned value
/// counts toward the current action's return.
const DISCOUNT_GAMMA: f64 = 0.9;
/// Trace-decay λ used when canon enables `eligibility_traces`.
const TRACE_LAMBDA: f64 = 0.8;
/// Floor on the effective softmax temperature, keeping the exponentials
/// finite when every sharpening factor is at its maximum.
const MIN_TEMPERATURE: f64 = 0.01;

/// The real, small action repertoire this engine can honestly reason about:
/// one action per tracked need/observation-affordance pair, plus rest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FormalAction {
    SeekFood,
    SeekWater,
    SeekShelter,
    Socialize,
    Rest,
}

impl FormalAction {
    const ALL: [FormalAction; 5] = [
        FormalAction::SeekFood,
        FormalAction::SeekWater,
        FormalAction::SeekShelter,
        FormalAction::Socialize,
        FormalAction::Rest,
    ];

    fn index(self) -> usize {
        match self {
            FormalAction::SeekFood => 0,
            FormalAction::SeekWater => 1,
            FormalAction::SeekShelter => 2,
            FormalAction::Socialize => 3,
            FormalAction::Rest => 4,
        }
    }
}

/// A single hierarchical level's prediction state: what it expects, how
/// wrong it was last tick, and how confident (precision) it is in that
/// expectation.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PredictionLevel {
    pub prediction: f64,
    pub prediction_error: f64,
    pub precision: f64,
    /// Belief-revision rate, per hour of simulated time.
    pub learning_rate: f64,
}

impl PredictionLevel {
    fn new(initial: f64, learning_rate: f64, precision: f64) -> Self {
        Self {
            prediction: initial,
            prediction_error: 0.0,
            precision: precision.clamp(0.05, 0.95),
            learning_rate,
        }
    }

    /// Precision-weighted update toward `actual`: higher precision (more
    /// confident this level's model is right) means a *smaller* single-tick
    /// move, mirroring the real predictive-coding relationship between
    /// precision and update magnitude (a confident prior resists revision).
    /// Confirmation bias damps the move further; error sensitivity sets how
    /// hard a large error knocks precision down. `learning_rate` is per
    /// hour of simulated time, and `dt_years` is the real step length.
    fn update(self, actual: f64, dt_years: f64, traits: BeliefRevisionTraits) -> Self {
        let error = actual - self.prediction;
        let revision_rate = self.learning_rate
            * (1.0 - self.precision * 0.5)
            * (1.0 - traits.confirmation_bias * 0.5);
        let update_gain = rates::relaxation_fraction(revision_rate, dt_years, rates::HOUR_YEARS);
        let prediction = (self.prediction + error * update_gain).clamp(0.0, 1.0);

        // Precision itself adapts: sustained small errors raise confidence,
        // a large error knocks it down (the model was confidently wrong).
        // Both rates are per hour.
        let hours = rates::elapsed(dt_years, rates::HOUR_YEARS);
        let precision_delta = if error.abs() < 0.1 {
            0.05 * hours
        } else {
            -error.abs() * 0.4 * traits.error_sensitivity * hours
        };
        let precision = (self.precision + precision_delta).clamp(0.05, 0.95);

        Self {
            prediction,
            prediction_error: error.abs(),
            precision,
            learning_rate: self.learning_rate,
        }
    }
}

/// The lower-level channels level 1 ("survival pressure") explains, as
/// canon `causal_model`/`utility_function` keys: hunger, thirst, the
/// shelter shortfall (1 − shelter quality) and hazard.
pub const SURVIVAL_CHANNELS: [&str; 4] = ["hunger", "thirst", "shelter_deficit", "hazard"];
/// The lower-level channels level 2 ("environmental favorability")
/// explains, as canon `semantic_network` keys.
pub const FAVORABILITY_CHANNELS: [&str; 3] = ["social_density", "daylight", "resource_abundance"];
/// Engine default relative weights of the survival channels, used for the
/// level-1 causal model and the level-3 goal utility when canon leaves
/// them unset.
const DEFAULT_SURVIVAL_WEIGHTS: [f64; 4] = [0.3, 0.3, 0.2, 0.2];
/// Engine default prior error variance of a channel when canon
/// `precision_attention.expected_uncertainty` leaves it unset.
const DEFAULT_CHANNEL_VARIANCE: f64 = 0.05;
/// Floor on a channel's learned error variance, so its precision stays
/// finite when the model explains it exactly.
const MIN_CHANNEL_VARIANCE: f64 = 1.0e-3;
/// Engine default for canon `precision_attention.precision_learning`: how
/// fast a channel's error-variance estimate tracks its squared errors, per
/// hour.
const DEFAULT_PRECISION_LEARNING: f64 = 0.1;

/// A level's linear generative model of the level below (Rao & Ballard
/// 1999): one latent cause `r` predicts each lower channel as
/// `û_i = L_i · r`. Inference finds the `r` that best explains the inputs,
/// weighting each channel by its precision; learning moves each loading
/// along its precision-weighted prediction error (`ΔL_i ∝ π_i · ε_i · r`)
/// and tracks each channel's error variance, whose inverse is its
/// precision. Loadings are kept non-negative with mean 1, so the latent
/// stays on the channels' 0–1 scale.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CausalModel {
    /// Loading of each lower channel on the latent cause (mean 1).
    pub loadings: Vec<f64>,
    /// Learned error variance of each channel.
    pub variance: Vec<f64>,
    /// This tick's top-down prediction error per channel, `u_i − L_i · r`.
    pub errors: Vec<f64>,
}

impl CausalModel {
    fn new(weights: &[f64], variance: &[f64]) -> Self {
        let mut model = Self {
            loadings: weights.to_vec(),
            variance: variance
                .iter()
                .map(|v| v.max(MIN_CHANNEL_VARIANCE))
                .collect(),
            errors: vec![0.0; weights.len()],
        };
        model.normalize();
        model
    }

    /// Scale the loadings to mean 1. All-zero loadings explain nothing, so
    /// they reset to equal weight.
    fn normalize(&mut self) {
        let n = self.loadings.len() as f64;
        let sum: f64 = self.loadings.iter().sum();
        if sum > 0.0 {
            for loading in &mut self.loadings {
                *loading *= n / sum;
            }
        } else {
            self.loadings.fill(1.0);
        }
    }

    fn precision(&self, i: usize) -> f64 {
        1.0 / self.variance[i].max(MIN_CHANNEL_VARIANCE)
    }

    /// Bottom-up inference: the latent minimizing the precision-weighted
    /// squared error `Σ π_i (u_i − L_i r)²`, which is
    /// `r = Σ π_i L_i u_i / Σ π_i L_i²`.
    fn infer(&self, inputs: &[f64]) -> f64 {
        let (mut numerator, mut denominator) = (0.0, 0.0);
        for (i, (&loading, &input)) in self.loadings.iter().zip(inputs).enumerate() {
            let precision = self.precision(i);
            numerator += precision * loading * input;
            denominator += precision * loading * loading;
        }
        if denominator > 0.0 {
            (numerator / denominator).clamp(0.0, 1.0)
        } else {
            inputs.iter().sum::<f64>() / inputs.len().max(1) as f64
        }
    }

    /// Learn from this tick: predict each channel top-down from `latent`
    /// (the level's belief after revision), update each channel's error
    /// variance at `precision_gain`, and move each loading along its
    /// precision-weighted error at `weight_gain`. Precision enters relative
    /// to the channels' mean, so it sets how the step is shared between
    /// channels, not its overall size.
    fn learn(&self, inputs: &[f64], latent: f64, weight_gain: f64, precision_gain: f64) -> Self {
        let n = self.loadings.len();
        let errors: Vec<f64> = self
            .loadings
            .iter()
            .zip(inputs)
            .map(|(loading, input)| input - loading * latent)
            .collect();
        let precisions: Vec<f64> = (0..n).map(|i| self.precision(i)).collect();
        let mean_precision = precisions.iter().sum::<f64>() / n.max(1) as f64;
        let mut model = Self {
            loadings: self
                .loadings
                .iter()
                .zip(&errors)
                .zip(&precisions)
                .map(|((loading, error), precision)| {
                    let relative = precision / mean_precision.max(f64::MIN_POSITIVE);
                    (loading + weight_gain * relative * error * latent).clamp(0.0, n as f64)
                })
                .collect(),
            variance: self
                .variance
                .iter()
                .zip(&errors)
                .map(|(variance, error)| {
                    (variance + precision_gain * (error * error - variance))
                        .max(MIN_CHANNEL_VARIANCE)
                })
                .collect(),
            errors,
        };
        model.normalize();
        model
    }

    /// This model's Laplace free energy, `½ Σ_i (π_i ε_i² + ln σ_i²)`,
    /// dropping the constant `½ n ln 2π`.
    fn free_energy(&self) -> f64 {
        self.errors
            .iter()
            .enumerate()
            .map(|(i, error)| {
                let variance = self.variance[i].max(MIN_CHANNEL_VARIANCE);
                0.5 * (error * error / variance + variance.ln())
            })
            .sum()
    }
}

/// Read one weight per channel from a canon map. Channels the map leaves
/// unset (or at zero) keep their default; an absent or empty map keeps
/// every default.
fn channel_weights(
    map: Option<&std::collections::BTreeMap<String, f32>>,
    channels: &[&str],
    defaults: &[f64],
) -> Vec<f64> {
    channels
        .iter()
        .zip(defaults)
        .map(|(channel, &default)| {
            map.and_then(|map| map.get(*channel))
                .map(|&value| value as f64)
                .filter(|value| value.is_finite() && *value > 0.0)
                .unwrap_or(default)
        })
        .collect()
}

fn default_survival_model() -> CausalModel {
    CausalModel::new(
        &DEFAULT_SURVIVAL_WEIGHTS,
        &[DEFAULT_CHANNEL_VARIANCE; SURVIVAL_CHANNELS.len()],
    )
}

fn default_favorability_model() -> CausalModel {
    CausalModel::new(
        &[1.0; FAVORABILITY_CHANNELS.len()],
        &[DEFAULT_CHANNEL_VARIANCE; FAVORABILITY_CHANNELS.len()],
    )
}

fn default_goal_utility() -> Vec<f64> {
    normalized_to_unit_sum(DEFAULT_SURVIVAL_WEIGHTS.to_vec())
}

/// Scale weights to sum 1; all-zero weights become equal.
fn normalized_to_unit_sum(mut weights: Vec<f64>) -> Vec<f64> {
    let sum: f64 = weights.iter().sum();
    if sum > 0.0 {
        for weight in &mut weights {
            *weight /= sum;
        }
    } else {
        let equal = 1.0 / weights.len().max(1) as f64;
        weights.fill(equal);
    }
    weights
}

fn default_precision_learning() -> f64 {
    DEFAULT_PRECISION_LEARNING
}

/// One action's immediate utility, learned critic value, eligibility trace
/// and current selection probability.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ActionValue {
    pub action: FormalAction,
    /// Immediate affordance utility this tick: how much taking this action
    /// would relieve current need deficits, given what the world offers.
    pub expected_utility: f64,
    /// Learned critic value `Q(a)`: the discounted survival-pressure relief
    /// that has actually followed choosing this action, TD-updated each tick.
    #[serde(default)]
    pub learned_value: f64,
    /// Eligibility trace `e(a)`: how much credit this action still receives
    /// for the current tick's TD error. Always zero with traces disabled
    /// except for the action just taken.
    #[serde(default)]
    pub eligibility: f64,
    /// Softmax probability this action had when this tick's action was
    /// sampled.
    #[serde(default)]
    pub selection_probability: f64,
    /// Actor preference `h(a)`, updated by the canon policy-gradient method.
    #[serde(default)]
    pub preference: f64,
}

/// Canon `PredictiveProcessingSchema.learning_mechanisms` traits that shape
/// how every level revises its beliefs.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BeliefRevisionTraits {
    /// 0-1: how strongly an existing belief resists revision (at 1, each
    /// update moves half as far).
    pub confirmation_bias: f64,
    /// 0-1: how much a large prediction error erodes a level's precision.
    pub error_sensitivity: f64,
}

impl Default for BeliefRevisionTraits {
    fn default() -> Self {
        Self {
            confirmation_bias: 0.3,
            error_sensitivity: 0.5,
        }
    }
}

/// Canon `active_inference.action_selection` / `policy_update` parameters,
/// resolved once from the human's schema. Zero-valued canon fields (an
/// unfilled template) fall back to the engine defaults below, except
/// `habit_bias`, where zero is a legitimate "no habit" setting.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ActiveInferenceParams {
    pub softmax_temperature: f64,
    /// 0 = maximally exploratory (flat distribution), 1 = maximally
    /// exploitative (sharp distribution).
    pub exploitation: f64,
    pub habit_bias: f64,
    /// Weight on immediate affordance utility versus the learned critic
    /// value when scoring actions.
    pub goal_directed_weight: f64,
    /// Critic learning rate, per hour of simulated time.
    pub policy_learning_rate: f64,
    /// Per-tick trace decay `γλ`; zero when canon disables eligibility
    /// traces (one-step TD).
    pub trace_decay: f64,
    /// Canon `policy_gradient_method` for the actor preferences.
    #[serde(default)]
    pub policy_gradient: PolicyGradient,
}

/// Canon `active_inference.policy_update.policy_gradient_method`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum PolicyGradient {
    Reinforce,
    #[default]
    ActorCritic,
    NaturalGradient,
}

impl PolicyGradient {
    /// Change to action `i`'s preference after `previous` was taken with
    /// probability vector `pi`, given reward `reward` and TD error `td`.
    fn preference_step(self, i: usize, previous: usize, pi: f64, reward: f64, td: f64) -> f64 {
        let taken = if i == previous { 1.0 } else { 0.0 };
        match self {
            Self::Reinforce => reward * (taken - pi),
            Self::ActorCritic => td * (taken - pi),
            Self::NaturalGradient => td * taken,
        }
    }
}

impl Default for ActiveInferenceParams {
    /// Engine defaults for a human with no canonical schema at all. Traces
    /// are enabled here because no canon exists to say otherwise.
    fn default() -> Self {
        Self {
            softmax_temperature: 0.15,
            exploitation: 0.7,
            habit_bias: 0.1,
            goal_directed_weight: 0.6,
            policy_learning_rate: 0.2,
            trace_decay: DISCOUNT_GAMMA * TRACE_LAMBDA,
            policy_gradient: PolicyGradient::ActorCritic,
        }
    }
}

impl ActiveInferenceParams {
    fn from_profile(profile: &mk_core::human::HumanProfile) -> Self {
        let defaults = Self::default();
        let Some(schema) = profile.canonical_schema() else {
            return defaults;
        };
        let active = &schema
            .extreme_brain_detail
            .formal_predictive_processing
            .active_inference;
        let or_default = |value: f32, fallback: f64| {
            if value > 0.0 {
                value as f64
            } else {
                fallback
            }
        };
        let selection = &active.action_selection;
        let update = &active.policy_update;
        Self {
            softmax_temperature: or_default(
                selection.softmax_temperature,
                defaults.softmax_temperature,
            ),
            exploitation: or_default(selection.exploitation_vs_exploration, defaults.exploitation)
                .clamp(0.0, 1.0),
            habit_bias: (selection.habit_bias as f64).clamp(0.0, 1.0),
            goal_directed_weight: or_default(
                selection.goal_directed_weight,
                defaults.goal_directed_weight,
            )
            .clamp(0.0, 1.0),
            policy_learning_rate: or_default(update.learning_rate, defaults.policy_learning_rate)
                .clamp(0.0, 1.0),
            trace_decay: if update.eligibility_traces {
                DISCOUNT_GAMMA * TRACE_LAMBDA
            } else {
                0.0
            },
            policy_gradient: match update.policy_gradient_method {
                Some(PolicyGradientMethodSchema::Reinforce) => PolicyGradient::Reinforce,
                Some(PolicyGradientMethodSchema::NaturalGradient) => {
                    PolicyGradient::NaturalGradient
                }
                Some(PolicyGradientMethodSchema::ActorCritic) | None => PolicyGradient::ActorCritic,
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormalPredictiveProcessingSnapshot {
    // --- hierarchical_models (state, stepped) ---
    /// Level 0: raw environmental/bodily signal predictions — one per real
    /// `AgentWorldObservation` channel this human tracks expectations for.
    pub level_0_temperature: PredictionLevel,
    pub level_0_resource_abundance: PredictionLevel,
    pub level_0_hazard: PredictionLevel,
    /// Level 1: "survival pressure" — a real aggregate of level-0 signals
    /// (inverse of caloric/hydration/shelter access, plus hazard).
    pub level_1_survival_pressure: PredictionLevel,
    /// Level 2: "environmental favorability" — a real aggregate distinct
    /// from survival pressure (social density + daylight + resource
    /// abundance), the canon's "concept"-level integration.
    pub level_2_favorability: PredictionLevel,
    /// Level 1's learned generative model of the survival channels
    /// ([`SURVIVAL_CHANNELS`]), seeded from canon
    /// `level_1_features[0].causal_model`.
    #[serde(default = "default_survival_model")]
    pub survival_model: CausalModel,
    /// Level 2's learned generative model of the favorability channels
    /// ([`FAVORABILITY_CHANNELS`]), seeded from canon
    /// `level_2_concepts[0].semantic_network`.
    #[serde(default = "default_favorability_model")]
    pub favorability_model: CausalModel,
    /// Level 3 ("goals"): how much each survival channel's deficit costs
    /// this human, from canon `level_3_goals[0].utility_function`,
    /// normalized to sum 1. The drop in this goal deficit is the reward the
    /// critic learns from.
    #[serde(default = "default_goal_utility")]
    pub goal_utility: Vec<f64>,
    /// Canon `precision_attention.precision_learning`: how fast each
    /// channel's error-variance estimate follows its squared errors, per
    /// hour.
    #[serde(default = "default_precision_learning")]
    pub precision_learning: f64,

    // --- error_minimization (derived each tick) ---
    /// Precision-weighted mean |error| across the five level beliefs.
    pub total_weighted_prediction_error: f64,
    /// Variational free energy of levels 1–2's generative models under the
    /// Laplace approximation, which for a linear-Gaussian model is
    /// `½ Σ_i (π_i ε_i² + ln σ_i²)` over every channel, up to a constant
    /// (Friston 2005). Canon `error_minimization.free_energy_principle`.
    #[serde(default)]
    pub variational_free_energy: f64,
    /// How much a level-0 error influenced level-1/2 revision this tick —
    /// evidence hierarchical_error_propagation is genuinely wired, not just
    /// declared.
    pub hierarchical_error_propagation_magnitude: f64,

    // --- active_inference (state, stepped) ---
    pub action_values: [ActionValue; 5],
    /// The action sampled from this tick's softmax distribution.
    pub selected_action: FormalAction,
    /// Extra sharpening of the action-selection distribution on top of the
    /// canon temperature (higher = more decisive/exploitive), driven by
    /// level-2 precision and a cleared willpower gate.
    pub decisiveness: f64,

    // --- consciousness_indicators (derived each tick) ---
    /// Mean precision across the hierarchy: how far this human currently
    /// trusts its own read of the world. Seeded from canon
    /// `subjective_confidence`, then earned or lost as predictions hold or
    /// fail.
    #[serde(default = "default_subjective_confidence")]
    pub subjective_confidence: f64,
    #[serde(default)]
    pub belief_revision: BeliefRevisionTraits,

    // --- policy_update (state, stepped) ---
    #[serde(default)]
    pub params: ActiveInferenceParams,
    /// Whether `selected_action` has actually been taken yet, i.e. whether
    /// there is a previous choice for this tick's TD error to credit.
    #[serde(default)]
    pub has_acted: bool,
    /// Level-3 goal deficit when `selected_action` was chosen; the next
    /// tick's drop from this value is that choice's reward. (The name
    /// predates the goal level and is kept so saved snapshots load.)
    #[serde(default)]
    pub last_survival_pressure: f64,
    /// This tick's temporal-difference error (reward + γ·max Q − Q(prev)).
    #[serde(default)]
    pub last_td_error: f64,
}

fn default_subjective_confidence() -> f64 {
    0.5
}

impl FormalPredictiveProcessingSnapshot {
    pub fn from_profile(profile: &mk_core::human::HumanProfile) -> Self {
        // Zero-valued canon fields (an unfilled template) fall back to the
        // engine defaults.
        let or_default = |value: f32, fallback: f64| {
            if value > 0.0 {
                (value as f64).clamp(0.0, 1.0)
            } else {
                fallback
            }
        };
        let defaults = BeliefRevisionTraits::default();
        let (learning_rate, belief_revision, confidence) = match profile.canonical_schema() {
            Some(schema) => {
                let pp = &schema.extreme_brain_detail.layer_5_predictive_processing;
                let lm = &pp.learning_mechanisms;
                (
                    or_default(lm.learning_rate, 0.3),
                    BeliefRevisionTraits {
                        confirmation_bias: or_default(
                            lm.confirmation_bias,
                            defaults.confirmation_bias,
                        ),
                        error_sensitivity: or_default(
                            lm.prediction_error_sensitivity,
                            defaults.error_sensitivity,
                        ),
                    },
                    or_default(
                        pp.consciousness_indicators.subjective_confidence,
                        default_subjective_confidence(),
                    ),
                )
            }
            None => (0.3, defaults, default_subjective_confidence()),
        };

        let (survival_model, favorability_model, goal_utility, precision_learning) =
            match profile.canonical_schema() {
                Some(schema) => {
                    let fpp = &schema.extreme_brain_detail.formal_predictive_processing;
                    let models = &fpp.hierarchical_models;
                    let uncertainty = &fpp.precision_attention.expected_uncertainty;
                    let variance = |channels: &[&str]| -> Vec<f64> {
                        channels
                            .iter()
                            .map(|channel| {
                                uncertainty
                                    .get(*channel)
                                    .map(|&value| value as f64)
                                    .filter(|value| value.is_finite() && *value > 0.0)
                                    .unwrap_or(DEFAULT_CHANNEL_VARIANCE)
                            })
                            .collect()
                    };
                    let survival = CausalModel::new(
                        &channel_weights(
                            models
                                .level_1_features
                                .first()
                                .and_then(|entry| entry.causal_model.as_ref()),
                            &SURVIVAL_CHANNELS,
                            &DEFAULT_SURVIVAL_WEIGHTS,
                        ),
                        &variance(&SURVIVAL_CHANNELS),
                    );
                    let favorability = CausalModel::new(
                        &channel_weights(
                            models
                                .level_2_concepts
                                .first()
                                .and_then(|entry| entry.semantic_network.as_ref()),
                            &FAVORABILITY_CHANNELS,
                            &[1.0; FAVORABILITY_CHANNELS.len()],
                        ),
                        &variance(&FAVORABILITY_CHANNELS),
                    );
                    let utility = channel_weights(
                        models
                            .level_3_goals
                            .first()
                            .and_then(|entry| entry.utility_function.as_ref()),
                        &SURVIVAL_CHANNELS,
                        &DEFAULT_SURVIVAL_WEIGHTS,
                    );
                    let precision_learning = fpp.precision_attention.precision_learning as f64;
                    (
                        survival,
                        favorability,
                        normalized_to_unit_sum(utility),
                        if precision_learning.is_finite() && precision_learning > 0.0 {
                            precision_learning
                        } else {
                            DEFAULT_PRECISION_LEARNING
                        },
                    )
                }
                None => (
                    default_survival_model(),
                    default_favorability_model(),
                    default_goal_utility(),
                    DEFAULT_PRECISION_LEARNING,
                ),
            };

        let action_values = FormalAction::ALL.map(|action| ActionValue {
            action,
            expected_utility: 0.5,
            learned_value: 0.0,
            eligibility: 0.0,
            selection_probability: 1.0 / FormalAction::ALL.len() as f64,
            preference: 0.0,
        });

        Self {
            level_0_temperature: PredictionLevel::new(0.5, learning_rate, confidence),
            level_0_resource_abundance: PredictionLevel::new(0.5, learning_rate, confidence),
            level_0_hazard: PredictionLevel::new(0.05, learning_rate, confidence),
            level_1_survival_pressure: PredictionLevel::new(0.3, learning_rate * 0.7, confidence),
            level_2_favorability: PredictionLevel::new(0.5, learning_rate * 0.5, confidence),
            survival_model,
            favorability_model,
            goal_utility,
            precision_learning,
            total_weighted_prediction_error: 0.0,
            variational_free_energy: 0.0,
            hierarchical_error_propagation_magnitude: 0.0,
            action_values,
            selected_action: FormalAction::Rest,
            decisiveness: 0.5,
            subjective_confidence: confidence.clamp(0.05, 0.95),
            belief_revision,
            params: ActiveInferenceParams::from_profile(profile),
            has_acted: false,
            last_survival_pressure: 0.0,
            last_td_error: 0.0,
        }
    }

    /// Step the full hierarchy: update level 0 from real observation
    /// signals, propagate error upward into levels 1-2, compute the total
    /// weighted prediction error, TD-update the learned critic from the
    /// survival-pressure relief that followed last tick's choice, then
    /// re-score actions from this human's actual needs against observation
    /// affordances and sample this tick's action from the softmax.
    ///
    /// `core_systems.willpower_available` (see [`super::core_systems`])
    /// feeds real signal into `decisiveness`: canon's willsys is described
    /// as the "ONLY gateway to cognition," and when it has genuinely
    /// cleared this tick (an urge broke through, not just background
    /// pressure) the human commits to whatever action-argmax already
    /// favors more strongly. `UrgeKind` (Survival/Social/Achievement/
    /// Exploration/Reproduction) intentionally does *not* map onto
    /// `FormalAction` (SeekFood/SeekWater/SeekShelter/Socialize/Rest) to
    /// directly pick an action — the categories don't correspond 1:1 (e.g.
    /// `Survival` alone can't distinguish "needs food" from "needs water"),
    /// and forcing a mapping would be an invented correspondence, not a
    /// real one. Amplifying decisiveness is the honest coupling available.
    #[allow(clippy::too_many_arguments)]
    pub fn step(
        &self,
        human_id: HumanId,
        tick: u64,
        observation: &AgentWorldObservation,
        needs: &NeedsSnapshot,
        core_systems: &super::core_systems::CoreSystemsSnapshot,
        rng: &RngRegistry,
        dt_years: f64,
    ) -> Self {
        let dt = dt_years.max(0.0);
        let traits = self.belief_revision;

        // Level 0: predict the raw signals this human directly experiences.
        let temp_signal = (observation.ambient_temperature_c / 40.0).clamp(0.0, 1.0);
        let level_0_temperature = self.level_0_temperature.update(temp_signal, dt, traits);
        let level_0_resource_abundance =
            self.level_0_resource_abundance
                .update(observation.resource_abundance, dt, traits);
        let level_0_hazard = self
            .level_0_hazard
            .update(observation.hazard_index, dt, traits);

        // Level 1 ("survival pressure"): the latent cause that best explains
        // this human's survival channels under its learned causal model.
        // Its belief is revised toward that bottom-up estimate, then
        // predicts each channel top-down, and those errors train the model.
        let survival_inputs = [
            needs.hunger.clamp(0.0, 1.0),
            needs.thirst.clamp(0.0, 1.0),
            (1.0 - observation.shelter_quality).clamp(0.0, 1.0),
            observation.hazard_index.clamp(0.0, 1.0),
        ];
        let precision_gain =
            rates::relaxation_fraction(self.precision_learning, dt, rates::HOUR_YEARS);
        let level_1_survival_pressure = self.level_1_survival_pressure.update(
            self.survival_model.infer(&survival_inputs),
            dt,
            traits,
        );
        let survival_model = self.survival_model.learn(
            &survival_inputs,
            level_1_survival_pressure.prediction,
            rates::relaxation_fraction(
                level_1_survival_pressure.learning_rate,
                dt,
                rates::HOUR_YEARS,
            ),
            precision_gain,
        );

        // Level 2 ("environmental favorability"): the same inference and
        // learning over social, light and resource conditions.
        let favorability_inputs = [
            observation.social_density.clamp(0.0, 1.0),
            observation.daylight_fraction.clamp(0.0, 1.0),
            observation.resource_abundance.clamp(0.0, 1.0),
        ];
        let level_2_favorability = self.level_2_favorability.update(
            self.favorability_model.infer(&favorability_inputs),
            dt,
            traits,
        );
        let favorability_model = self.favorability_model.learn(
            &favorability_inputs,
            level_2_favorability.prediction,
            rates::relaxation_fraction(level_2_favorability.learning_rate, dt, rates::HOUR_YEARS),
            precision_gain,
        );

        // Level 3 ("goals"): the goal deficit the survival channels cost this
        // human under its utility function. Its drop is the reward.
        let goal_deficit = self
            .goal_utility
            .iter()
            .zip(&survival_inputs)
            .map(|(weight, input)| weight * input)
            .sum::<f64>()
            .clamp(0.0, 1.0);

        // Hierarchical error propagation: level-0 surprise nudges level-1's
        // effective precision down (a genuinely wrong sensory prediction
        // should make the human less confident in its higher-level read of
        // survival pressure this tick) — this is the "errors flow up"
        // mechanic actually affecting a different level's state, not just a
        // per-level independent update.
        let level_0_mean_error = (level_0_temperature.prediction_error
            + level_0_resource_abundance.prediction_error
            + level_0_hazard.prediction_error)
            / 3.0;
        let mut level_1_survival_pressure = level_1_survival_pressure;
        level_1_survival_pressure.precision = (level_1_survival_pressure.precision
            - level_0_mean_error * 0.15 * rates::elapsed(dt, rates::HOUR_YEARS))
        .clamp(0.05, 0.95);

        let precisions_sum = level_0_temperature.precision
            + level_0_resource_abundance.precision
            + level_0_hazard.precision
            + level_1_survival_pressure.precision
            + level_2_favorability.precision;
        let total_weighted_prediction_error = (level_0_temperature.precision
            * level_0_temperature.prediction_error
            + level_0_resource_abundance.precision * level_0_resource_abundance.prediction_error
            + level_0_hazard.precision * level_0_hazard.prediction_error
            + level_1_survival_pressure.precision * level_1_survival_pressure.prediction_error
            + level_2_favorability.precision * level_2_favorability.prediction_error)
            / precisions_sum.max(0.01);
        let subjective_confidence = precisions_sum / 5.0;
        let variational_free_energy =
            survival_model.free_energy() + favorability_model.free_energy();

        // active_inference: immediate utility per real action = how much it
        // would relieve this human's actual current need deficits, given
        // real observation affordances (an action to seek food is only
        // useful if there's food to find).
        let immediate_utility = [
            (needs.hunger * observation.caloric_access).clamp(0.0, 1.0),
            (needs.thirst * observation.hydration_access).clamp(0.0, 1.0),
            ((1.0 - observation.shelter_quality) * observation.hazard_index.max(0.1))
                .clamp(0.0, 1.0),
            (observation.social_density * (1.0 - needs.fatigue)).clamp(0.0, 1.0),
            needs.fatigue.clamp(0.0, 1.0),
        ];

        // policy_update: TD(λ) critic over the previously taken action. The
        // reward is the realized drop in survival pressure since that action
        // was chosen; eligibility traces spread the TD error back over the
        // earlier actions that led here.
        let params = self.params;
        let mut action_values = self.action_values;
        let mut last_td_error = 0.0;
        if self.has_acted {
            let reward = self.last_survival_pressure - goal_deficit;
            let previous = self.selected_action.index();
            let best_next = action_values
                .iter()
                .map(|v| v.learned_value)
                .fold(f64::NEG_INFINITY, f64::max);
            last_td_error =
                reward + DISCOUNT_GAMMA * best_next - action_values[previous].learned_value;
            let gain =
                rates::relaxation_fraction(params.policy_learning_rate, dt, rates::HOUR_YEARS);
            for (i, value) in action_values.iter_mut().enumerate() {
                // Replacing traces: the action just taken is fully eligible.
                if i == previous {
                    value.eligibility = 1.0;
                }
                value.learned_value = (value.learned_value
                    + gain * last_td_error * value.eligibility)
                    .clamp(-1.0, 1.0);
                value.eligibility *= params.trace_decay;
                // Actor: `selection_probability` still holds the π the
                // previous action was sampled from.
                let step = params.policy_gradient.preference_step(
                    i,
                    previous,
                    value.selection_probability,
                    reward,
                    last_td_error,
                );
                value.preference = (value.preference + gain * step).clamp(-1.0, 1.0);
            }
        }
        for (value, utility) in action_values.iter_mut().zip(immediate_utility) {
            value.expected_utility = utility;
        }

        // Decisiveness, derived from level-2 precision (a human confident in
        // its read of the environment commits to its best option more
        // strongly), plus real additional commitment when the willpower gate
        // cleared this tick — canon's willsys genuinely reaching cognition.
        let decisiveness = if core_systems.willpower_available {
            (level_2_favorability.precision + 0.2).clamp(0.0, 1.0)
        } else {
            level_2_favorability.precision
        };

        // action_selection: softmax over goal-directed utility blended with
        // the learned critic value, plus a habit pull toward repeating the
        // last action. Exploitation and decisiveness both lower the
        // effective temperature.
        let scores: [f64; 5] = std::array::from_fn(|i| {
            let habit = if self.has_acted && i == self.selected_action.index() {
                params.habit_bias
            } else {
                0.0
            };
            params.goal_directed_weight * action_values[i].expected_utility
                + (1.0 - params.goal_directed_weight)
                    * (action_values[i].learned_value + action_values[i].preference)
                + habit
        });
        let temperature =
            (params.softmax_temperature * (1.5 - params.exploitation) * (1.0 - 0.5 * decisiveness))
                .max(MIN_TEMPERATURE);
        let max_score = scores.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let weights = scores.map(|score| ((score - max_score) / temperature).exp());
        let total_weight: f64 = weights.iter().sum();
        for (value, weight) in action_values.iter_mut().zip(weights) {
            value.selection_probability = weight / total_weight;
        }

        let human_chunk = (human_id.0 & 0xFFFF_FFFF) as u32;
        let draw = rng.gen_f64_01(RngKey::new(
            SubsystemId::Humans,
            human_chunk,
            FORMAL_PP_SELECTION_EPOCH,
            tick,
        ));
        let mut cumulative = 0.0;
        let mut selected_action = FormalAction::ALL[FormalAction::ALL.len() - 1];
        for value in &action_values {
            cumulative += value.selection_probability;
            if draw < cumulative {
                selected_action = value.action;
                break;
            }
        }

        Self {
            level_0_temperature,
            level_0_resource_abundance,
            level_0_hazard,
            level_1_survival_pressure,
            level_2_favorability,
            survival_model,
            favorability_model,
            goal_utility: self.goal_utility.clone(),
            precision_learning: self.precision_learning,
            total_weighted_prediction_error,
            variational_free_energy,
            hierarchical_error_propagation_magnitude: level_0_mean_error,
            action_values,
            selected_action,
            decisiveness,
            subjective_confidence,
            belief_revision: traits,
            params,
            has_acted: true,
            last_survival_pressure: goal_deficit,
            last_td_error,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mk_core::human::{HumanProfile, HumanSchema};

    fn profile() -> HumanProfile {
        let schema = HumanSchema::canonical_minimal("formal_pp_test");
        HumanProfile::from_canonical_schema(HumanId::new(1), schema)
    }

    fn core_systems() -> super::super::core_systems::CoreSystemsSnapshot {
        super::super::core_systems::CoreSystemsSnapshot::from_profile(&profile())
    }

    fn rng() -> RngRegistry {
        RngRegistry::new([7u8; 32])
    }

    fn step(
        snapshot: &FormalPredictiveProcessingSnapshot,
        observation: &AgentWorldObservation,
        needs: &NeedsSnapshot,
        tick: u64,
    ) -> FormalPredictiveProcessingSnapshot {
        snapshot.step(
            HumanId::new(1),
            tick,
            observation,
            needs,
            &core_systems(),
            &rng(),
            // One hour: the belief and critic rates are per hour.
            rates::HOUR_YEARS,
        )
    }

    fn probability(snapshot: &FormalPredictiveProcessingSnapshot, action: FormalAction) -> f64 {
        snapshot.action_values[action.index()].selection_probability
    }

    #[test]
    fn from_profile_uses_sane_defaults() {
        let snapshot = FormalPredictiveProcessingSnapshot::from_profile(&profile());
        assert!(snapshot.level_0_temperature.precision > 0.0);
        assert_eq!(snapshot.total_weighted_prediction_error, 0.0);
        assert!(!snapshot.has_acted);
        // An unfilled canonical template falls back to engine defaults for
        // every zero-valued field, but its explicit `eligibility_traces:
        // false` is honored.
        assert_eq!(
            snapshot.params.softmax_temperature,
            ActiveInferenceParams::default().softmax_temperature
        );
        assert_eq!(snapshot.params.trace_decay, 0.0);
    }

    #[test]
    fn canon_action_selection_fields_are_read() {
        let mut schema = HumanSchema::canonical_minimal("formal_pp_canon");
        let active = &mut schema
            .extreme_brain_detail
            .formal_predictive_processing
            .active_inference;
        active.action_selection.softmax_temperature = 0.4;
        active.action_selection.habit_bias = 0.25;
        active.policy_update.eligibility_traces = true;
        let profile = HumanProfile::from_canonical_schema(HumanId::new(2), schema);

        let params = FormalPredictiveProcessingSnapshot::from_profile(&profile).params;

        assert!((params.softmax_temperature - 0.4).abs() < 1e-6);
        assert!((params.habit_bias - 0.25).abs() < 1e-6);
        assert!((params.trace_decay - DISCOUNT_GAMMA * TRACE_LAMBDA).abs() < 1e-12);
    }

    #[test]
    fn level_0_prediction_moves_toward_observation() {
        let snapshot = FormalPredictiveProcessingSnapshot::from_profile(&profile());
        let observation = AgentWorldObservation {
            resource_abundance: 0.95,
            ..Default::default()
        };
        let needs = NeedsSnapshot::from_profile(&profile());

        let stepped = step(&snapshot, &observation, &needs, 1);

        assert!(
            stepped.level_0_resource_abundance.prediction
                > snapshot.level_0_resource_abundance.prediction
        );
        assert!(stepped.level_0_resource_abundance.prediction_error > 0.0);
    }

    #[test]
    fn canon_belief_revision_traits_are_read() {
        let mut schema = HumanSchema::canonical_minimal("formal_pp_beliefs");
        let pp = &mut schema.extreme_brain_detail.layer_5_predictive_processing;
        pp.learning_mechanisms.confirmation_bias = 0.8;
        pp.learning_mechanisms.prediction_error_sensitivity = 0.9;
        pp.consciousness_indicators.subjective_confidence = 0.7;
        let profile = HumanProfile::from_canonical_schema(HumanId::new(3), schema);

        let snapshot = FormalPredictiveProcessingSnapshot::from_profile(&profile);

        assert!((snapshot.belief_revision.confirmation_bias - 0.8).abs() < 1e-6);
        assert!((snapshot.belief_revision.error_sensitivity - 0.9).abs() < 1e-6);
        assert!((snapshot.subjective_confidence - 0.7).abs() < 1e-6);
        assert!((snapshot.level_0_hazard.precision - 0.7).abs() < 1e-6);
    }

    #[test]
    fn canon_causal_models_goal_utility_and_uncertainty_are_read() {
        use mk_core::human::schema::HierarchicalModelEntrySchema;
        use std::collections::BTreeMap;
        let mut schema = HumanSchema::canonical_minimal("formal_pp_models");
        let fpp = &mut schema.extreme_brain_detail.formal_predictive_processing;
        let map = |pairs: &[(&str, f32)]| -> Option<BTreeMap<String, f32>> {
            Some(pairs.iter().map(|(k, v)| (k.to_string(), *v)).collect())
        };
        fpp.hierarchical_models.level_1_features = vec![HierarchicalModelEntrySchema {
            causal_model: map(&[("hunger", 3.0), ("thirst", 1.0)]),
            ..Default::default()
        }];
        fpp.hierarchical_models.level_2_concepts = vec![HierarchicalModelEntrySchema {
            semantic_network: map(&[("daylight", 4.0)]),
            ..Default::default()
        }];
        fpp.hierarchical_models.level_3_goals = vec![HierarchicalModelEntrySchema {
            utility_function: map(&[("thirst", 2.0)]),
            ..Default::default()
        }];
        fpp.precision_attention.expected_uncertainty = map(&[("hazard", 0.2)]).unwrap();
        fpp.precision_attention.precision_learning = 0.4;
        let profile = HumanProfile::from_canonical_schema(HumanId::new(4), schema);

        let snapshot = FormalPredictiveProcessingSnapshot::from_profile(&profile);

        // Canon hunger 3, thirst 1; unset channels keep their 0.2 defaults.
        // Scaled to mean 1 over the four channels.
        let scale = 4.0 / (3.0 + 1.0 + 0.2 + 0.2);
        let expected = [3.0 * scale, 1.0 * scale, 0.2 * scale, 0.2 * scale];
        for (loading, expected) in snapshot.survival_model.loadings.iter().zip(expected) {
            assert!((loading - expected).abs() < 1e-9);
        }
        assert!(snapshot.favorability_model.loadings[1] > snapshot.favorability_model.loadings[0]);
        let utility_sum = 0.3 + 2.0 + 0.2 + 0.2;
        assert!((snapshot.goal_utility[1] - 2.0 / utility_sum).abs() < 1e-9);
        assert!((snapshot.goal_utility.iter().sum::<f64>() - 1.0).abs() < 1e-12);
        assert!((snapshot.survival_model.variance[3] - 0.2).abs() < 1e-6);
        assert_eq!(
            snapshot.survival_model.variance[0],
            DEFAULT_CHANNEL_VARIANCE
        );
        assert!((snapshot.precision_learning - 0.4).abs() < 1e-6);
    }

    #[test]
    fn inference_recovers_the_latent_cause_the_model_describes() {
        let model = CausalModel::new(&[2.0, 1.0, 1.0], &[0.05; 3]);
        // Loadings scale to mean 1: [1.5, 0.75, 0.75]. Inputs generated by
        // r = 0.4 are explained exactly.
        let inputs = [0.6, 0.3, 0.3];
        assert!((model.infer(&inputs) - 0.4).abs() < 1e-12);
    }

    #[test]
    fn a_channel_that_keeps_running_high_gains_loading() {
        let mut snapshot = FormalPredictiveProcessingSnapshot::from_profile(&profile());
        let observation = AgentWorldObservation {
            shelter_quality: 0.9,
            hazard_index: 0.1,
            ..Default::default()
        };
        let mut needs = NeedsSnapshot::from_profile(&profile());
        needs.hunger = 0.9;
        needs.thirst = 0.1;
        let before = snapshot.survival_model.loadings.clone();

        for tick in 0..48 {
            snapshot = step(&snapshot, &observation, &needs, tick);
        }

        let after = &snapshot.survival_model.loadings;
        assert!(
            after[0] > before[0],
            "hunger loading {before:?} -> {after:?}"
        );
        assert!(
            after[1] < before[1],
            "thirst loading {before:?} -> {after:?}"
        );
        let mean = after.iter().sum::<f64>() / after.len() as f64;
        assert!((mean - 1.0).abs() < 1e-12);
    }

    #[test]
    fn free_energy_falls_as_the_model_learns_a_steady_world() {
        let mut snapshot = FormalPredictiveProcessingSnapshot::from_profile(&profile());
        let observation = AgentWorldObservation {
            shelter_quality: 0.5,
            hazard_index: 0.3,
            social_density: 0.6,
            daylight_fraction: 0.2,
            resource_abundance: 0.7,
            ..Default::default()
        };
        let mut needs = NeedsSnapshot::from_profile(&profile());
        needs.hunger = 0.8;
        needs.thirst = 0.2;

        snapshot = step(&snapshot, &observation, &needs, 0);
        let early = snapshot.variational_free_energy;
        let early_variance = snapshot.survival_model.variance.clone();
        for tick in 1..200 {
            snapshot = step(&snapshot, &observation, &needs, tick);
        }

        assert!(
            snapshot.variational_free_energy < early,
            "free energy {early} -> {}",
            snapshot.variational_free_energy
        );
        // Better-explained channels earn precision: their error variance
        // falls.
        let fell = snapshot
            .survival_model
            .variance
            .iter()
            .zip(&early_variance)
            .filter(|(after, before)| after < before)
            .count();
        assert!(fell > 0);
    }

    #[test]
    fn the_reward_is_the_drop_in_the_goal_deficit() {
        let mut snapshot = FormalPredictiveProcessingSnapshot::from_profile(&profile());
        let observation = AgentWorldObservation {
            shelter_quality: 1.0,
            ..Default::default()
        };
        let mut needs = NeedsSnapshot::from_profile(&profile());
        needs.hunger = 0.5;
        needs.thirst = 0.0;

        snapshot = step(&snapshot, &observation, &needs, 0);

        let channels = [0.5, 0.0, 0.0, observation.hazard_index];
        let expected: f64 = snapshot
            .goal_utility
            .iter()
            .zip(channels)
            .map(|(weight, channel)| weight * channel)
            .sum();
        assert!((snapshot.last_survival_pressure - expected).abs() < 1e-12);
    }

    #[test]
    fn confirmation_bias_slows_belief_revision() {
        let open = FormalPredictiveProcessingSnapshot::from_profile(&profile());
        let mut biased = open.clone();
        biased.belief_revision.confirmation_bias = 1.0;
        let mut open = open;
        open.belief_revision.confirmation_bias = 0.0;
        let observation = AgentWorldObservation {
            resource_abundance: 0.95,
            ..Default::default()
        };
        let needs = NeedsSnapshot::from_profile(&profile());

        let open_move = step(&open, &observation, &needs, 1)
            .level_0_resource_abundance
            .prediction
            - open.level_0_resource_abundance.prediction;
        let biased_move = step(&biased, &observation, &needs, 1)
            .level_0_resource_abundance
            .prediction
            - biased.level_0_resource_abundance.prediction;

        // Full bias halves the revision rate, so the move is the relaxation
        // fraction at half the rate: smaller, but more than half as large.
        assert!(biased_move > 0.0);
        assert!(biased_move < open_move && biased_move > open_move * 0.5);
    }

    #[test]
    fn repeated_surprise_erodes_subjective_confidence() {
        let snapshot = FormalPredictiveProcessingSnapshot::from_profile(&profile());
        let observation = AgentWorldObservation {
            resource_abundance: 0.99,
            hazard_index: 0.95,
            ..Default::default()
        };
        let needs = NeedsSnapshot::from_profile(&profile());

        let mut stepped = snapshot.clone();
        for tick in 0..5 {
            stepped = step(&stepped, &observation, &needs, tick);
        }

        assert!(stepped.subjective_confidence < snapshot.subjective_confidence);
    }

    #[test]
    fn sustained_error_propagates_up_and_lowers_survival_precision() {
        let snapshot = FormalPredictiveProcessingSnapshot::from_profile(&profile());
        let observation = AgentWorldObservation {
            resource_abundance: 0.99,
            hazard_index: 0.95,
            ..Default::default()
        };
        let needs = NeedsSnapshot::from_profile(&profile());

        let mut stepped = snapshot.clone();
        for tick in 0..5 {
            stepped = step(&stepped, &observation, &needs, tick);
        }

        assert!(
            stepped.level_1_survival_pressure.precision
                < snapshot.level_1_survival_pressure.precision
        );
        assert!(stepped.hierarchical_error_propagation_magnitude > 0.0);
    }

    #[test]
    fn dominant_need_gets_the_highest_selection_probability() {
        let snapshot = FormalPredictiveProcessingSnapshot::from_profile(&profile());
        let observation = AgentWorldObservation::default();
        let mut needs = NeedsSnapshot::from_profile(&profile());
        needs.hunger = 0.95;
        needs.thirst = 0.05;
        needs.fatigue = 0.05;

        let stepped = step(&snapshot, &observation, &needs, 1);

        let food = probability(&stepped, FormalAction::SeekFood);
        for action in FormalAction::ALL {
            if action != FormalAction::SeekFood {
                assert!(food > probability(&stepped, action), "{action:?}");
            }
        }
        let total: f64 = stepped
            .action_values
            .iter()
            .map(|v| v.selection_probability)
            .sum();
        assert!((total - 1.0).abs() < 1e-9);
    }

    #[test]
    fn same_seed_and_tick_reproduce_the_same_choice() {
        let snapshot = FormalPredictiveProcessingSnapshot::from_profile(&profile());
        let observation = AgentWorldObservation::default();
        let needs = NeedsSnapshot::from_profile(&profile());

        let a = step(&snapshot, &observation, &needs, 42);
        let b = step(&snapshot, &observation, &needs, 42);

        assert_eq!(a.selected_action, b.selected_action);
        assert_eq!(
            a.total_weighted_prediction_error,
            b.total_weighted_prediction_error
        );
        assert_eq!(a.decisiveness, b.decisiveness);
    }

    #[test]
    fn sampling_explores_non_argmax_actions_across_ticks() {
        let snapshot = FormalPredictiveProcessingSnapshot::from_profile(&profile());
        let observation = AgentWorldObservation::default();
        let needs = NeedsSnapshot::from_profile(&profile());

        let chosen: std::collections::BTreeSet<usize> = (0..200)
            .map(|tick| {
                step(&snapshot, &observation, &needs, tick)
                    .selected_action
                    .index()
            })
            .collect();

        assert!(chosen.len() > 1, "softmax sampling never left the argmax");
    }

    #[test]
    fn cleared_willpower_gate_increases_decisiveness_and_sharpens_choice() {
        let snapshot = FormalPredictiveProcessingSnapshot::from_profile(&profile());
        let observation = AgentWorldObservation::default();
        let mut needs = NeedsSnapshot::from_profile(&profile());
        needs.hunger = 0.9;

        let mut gated_core = core_systems();
        gated_core.willpower_available = true;
        let mut ungated_core = core_systems();
        ungated_core.willpower_available = false;

        let run = |core: &super::super::core_systems::CoreSystemsSnapshot| {
            snapshot.step(HumanId::new(1), 1, &observation, &needs, core, &rng(), 1.0)
        };
        let gated = run(&gated_core);
        let ungated = run(&ungated_core);

        assert!(gated.decisiveness > ungated.decisiveness);
        assert!(
            probability(&gated, FormalAction::SeekFood)
                > probability(&ungated, FormalAction::SeekFood)
        );
    }

    #[test]
    fn relief_after_an_action_raises_its_learned_value() {
        let mut snapshot = FormalPredictiveProcessingSnapshot::from_profile(&profile());
        snapshot.has_acted = true;
        snapshot.selected_action = FormalAction::SeekFood;
        snapshot.last_survival_pressure = 0.9;
        let observation = AgentWorldObservation {
            shelter_quality: 1.0,
            ..Default::default()
        };
        let mut needs = NeedsSnapshot::from_profile(&profile());
        needs.hunger = 0.0;
        needs.thirst = 0.0;

        let stepped = step(&snapshot, &observation, &needs, 1);

        assert!(stepped.last_td_error > 0.0);
        assert!(stepped.action_values[FormalAction::SeekFood.index()].learned_value > 0.0);
        assert_eq!(
            stepped.action_values[FormalAction::Rest.index()].learned_value,
            0.0
        );
    }

    fn relieved_after_seeking_food(method: PolicyGradient) -> FormalPredictiveProcessingSnapshot {
        let mut snapshot = FormalPredictiveProcessingSnapshot::from_profile(&profile());
        snapshot.params.policy_gradient = method;
        snapshot.has_acted = true;
        snapshot.selected_action = FormalAction::SeekFood;
        snapshot.last_survival_pressure = 0.9;
        let observation = AgentWorldObservation {
            shelter_quality: 1.0,
            ..Default::default()
        };
        let mut needs = NeedsSnapshot::from_profile(&profile());
        needs.hunger = 0.0;
        needs.thirst = 0.0;
        step(&snapshot, &observation, &needs, 1)
    }

    fn preference(snapshot: &FormalPredictiveProcessingSnapshot, action: FormalAction) -> f64 {
        snapshot.action_values[action.index()].preference
    }

    #[test]
    fn canon_policy_gradient_method_is_read() {
        for (canon, expected) in [
            (
                Some(PolicyGradientMethodSchema::Reinforce),
                PolicyGradient::Reinforce,
            ),
            (
                Some(PolicyGradientMethodSchema::ActorCritic),
                PolicyGradient::ActorCritic,
            ),
            (
                Some(PolicyGradientMethodSchema::NaturalGradient),
                PolicyGradient::NaturalGradient,
            ),
            (None, PolicyGradient::ActorCritic),
        ] {
            let mut schema = HumanSchema::canonical_minimal("formal_pp_gradient");
            schema
                .extreme_brain_detail
                .formal_predictive_processing
                .active_inference
                .policy_update
                .policy_gradient_method = canon;
            let profile = HumanProfile::from_canonical_schema(HumanId::new(3), schema);
            let params = FormalPredictiveProcessingSnapshot::from_profile(&profile).params;
            assert_eq!(params.policy_gradient, expected);
        }
    }

    #[test]
    fn every_method_reinforces_a_rewarded_action() {
        for method in [
            PolicyGradient::Reinforce,
            PolicyGradient::ActorCritic,
            PolicyGradient::NaturalGradient,
        ] {
            let stepped = relieved_after_seeking_food(method);
            assert!(
                preference(&stepped, FormalAction::SeekFood) > 0.0,
                "{method:?}"
            );
        }
    }

    #[test]
    fn score_function_methods_push_untaken_actions_down_and_natural_gradient_does_not() {
        let reinforce = relieved_after_seeking_food(PolicyGradient::Reinforce);
        let actor_critic = relieved_after_seeking_food(PolicyGradient::ActorCritic);
        let natural = relieved_after_seeking_food(PolicyGradient::NaturalGradient);
        // With a prior critic value the TD error differs from the reward.
        let with_prior = |method| {
            let mut snapshot = FormalPredictiveProcessingSnapshot::from_profile(&profile());
            snapshot.params.policy_gradient = method;
            snapshot.has_acted = true;
            snapshot.selected_action = FormalAction::SeekFood;
            snapshot.last_survival_pressure = 0.9;
            snapshot.action_values[FormalAction::SeekFood.index()].learned_value = 0.5;
            let observation = AgentWorldObservation {
                shelter_quality: 1.0,
                ..Default::default()
            };
            let mut needs = NeedsSnapshot::from_profile(&profile());
            needs.hunger = 0.0;
            needs.thirst = 0.0;
            step(&snapshot, &observation, &needs, 1)
        };
        assert!(preference(&reinforce, FormalAction::Rest) < 0.0);
        assert!(preference(&actor_critic, FormalAction::Rest) < 0.0);
        assert_eq!(preference(&natural, FormalAction::Rest), 0.0);
        // REINFORCE steps on the raw reward, actor-critic on the TD error.
        let reinforce_prior = with_prior(PolicyGradient::Reinforce);
        let actor_critic_prior = with_prior(PolicyGradient::ActorCritic);
        // δ = r + γ·max Q − Q(prev) = r + 0.9·0.5 − 0.5 < r.
        assert!(
            preference(&actor_critic_prior, FormalAction::SeekFood)
                < preference(&reinforce_prior, FormalAction::SeekFood)
        );
    }

    #[test]
    fn a_learned_preference_raises_that_actions_selection_probability() {
        let untrained = relieved_after_seeking_food(PolicyGradient::ActorCritic);
        let mut trained = FormalPredictiveProcessingSnapshot::from_profile(&profile());
        trained.action_values[FormalAction::Rest.index()].preference = 1.0;
        let observation = AgentWorldObservation::default();
        let needs = NeedsSnapshot::from_profile(&profile());
        let baseline = step(
            &FormalPredictiveProcessingSnapshot::from_profile(&profile()),
            &observation,
            &needs,
            1,
        );
        let preferred = step(&trained, &observation, &needs, 1);
        assert!(
            probability(&preferred, FormalAction::Rest)
                > probability(&baseline, FormalAction::Rest)
        );
        assert!(untrained
            .action_values
            .iter()
            .all(|v| v.preference.abs() <= 1.0));
    }

    #[test]
    fn eligibility_traces_credit_earlier_actions_only_when_enabled() {
        let observation = AgentWorldObservation {
            shelter_quality: 1.0,
            ..Default::default()
        };
        let mut needs = NeedsSnapshot::from_profile(&profile());
        needs.hunger = 0.0;
        needs.thirst = 0.0;

        let credit_for_shelter = |trace_decay: f64| {
            let mut snapshot = FormalPredictiveProcessingSnapshot::from_profile(&profile());
            snapshot.params.trace_decay = trace_decay;
            // Tick A: shelter was sought with no payoff yet.
            snapshot.has_acted = true;
            snapshot.selected_action = FormalAction::SeekShelter;
            snapshot.last_survival_pressure = 0.0;
            let mut after_shelter = step(&snapshot, &observation, &needs, 1);
            // Tick B: rest was chosen, and the large relief lands now.
            after_shelter.selected_action = FormalAction::Rest;
            after_shelter.last_survival_pressure = 0.9;
            let after_rest = step(&after_shelter, &observation, &needs, 2);
            after_rest.action_values[FormalAction::SeekShelter.index()].learned_value
                - after_shelter.action_values[FormalAction::SeekShelter.index()].learned_value
        };

        assert!(credit_for_shelter(DISCOUNT_GAMMA * TRACE_LAMBDA) > 0.0);
        assert_eq!(credit_for_shelter(0.0), 0.0);
    }

    #[test]
    fn snapshots_saved_before_policy_learning_still_deserialize() {
        let snapshot = FormalPredictiveProcessingSnapshot::from_profile(&profile());
        let mut value = serde_json::to_value(&snapshot).expect("serialize");
        let object = value.as_object_mut().expect("object");
        for field in [
            "params",
            "has_acted",
            "last_survival_pressure",
            "last_td_error",
            "survival_model",
            "favorability_model",
            "goal_utility",
            "precision_learning",
            "variational_free_energy",
        ] {
            object.remove(field);
        }
        for action in object["action_values"].as_array_mut().expect("array") {
            let action = action.as_object_mut().expect("object");
            action.remove("learned_value");
            action.remove("eligibility");
            action.remove("selection_probability");
        }

        let restored: FormalPredictiveProcessingSnapshot =
            serde_json::from_value(value).expect("legacy snapshot should deserialize");

        assert_eq!(restored.params, ActiveInferenceParams::default());
        assert!(!restored.has_acted);
        assert_eq!(restored.survival_model, default_survival_model());
        assert_eq!(restored.goal_utility, default_goal_utility());
        assert_eq!(restored.precision_learning, DEFAULT_PRECISION_LEARNING);
    }
}
