//! Attractor Control Snapshot - a multistable dynamical system over the
//! human's overall psychological stability.
//!
//! Surfaces `docs/canon/HumanReplicationSchema.js`'s
//! `NeurochemicalAttractorControlSchema`. It is a general attractor model
//! (basins of attraction, gradient flow, hysteresis under sustained
//! perturbation). It makes no claim to reproduce a specific
//! neurochemical-attractor theory. Each human's built-in basins are placed
//! from their temperament ([`BaseBasins::from_temperament`]), and canon
//! shifts them.
//!
//! The canon's slow neurochemical variables (`neurochemical_state`) are
//! integrated as first-order kinetics and reshape the landscape through
//! `attractor_modulation` (see [`NeurochemicalDynamics`]). Canon's
//! `receptor_competition`, `enzymatic_interactions` and `hysteresis_effects`
//! maps carry no rate or target a kinetic model could apply, so they stay
//! descriptive.
//!
//! Model: gradient flow of a single scalar `psychological_position`
//! (0 = crisis/dysregulated, 1 = stable/regulated) over a landscape of
//! named basins ([`Landscape`]). Two are built in: a deep `stable` basin
//! near the regulated end, placed and deepened by the human's emotional
//! stability, whose depth grows with time spent stable,
//! and a shallow `crisis` basin near zero, placed by emotional intensity,
//! that sustained
//! deprivation/pain/sickness can pull the position into. Canon can add any
//! number of further basins by name through `basin_position_shifts` and
//! `basin_depth_changes`. Each basin's restoring pull is weighted by how
//! strongly it captures the current position (inverse width-scaled
//! distance, normalized), so the nearest basin dominates. With two basins
//! this is the classic bistable double well.

use mk_core::human::schema::{FeedbackInteractionTypeSchema, NeurochemicalAttractorControlSchema};
use mk_core::human::HumanProfile;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttractorControlSnapshot {
    /// How fast the basin landscape itself reshapes under sustained
    /// perturbation (canon: `attractor_modulation.landscape_deformation_rate`).
    pub landscape_deformation_rate: f64,

    /// Current position in the bistable landscape (0 = crisis basin,
    /// 1 = stable basin).
    pub psychological_position: f64,
    /// Depth of the stable basin at `stable_center` — how strongly the
    /// system resists leaving it once there (grows slightly as the
    /// landscape "sets" under low deformation, mirroring real
    /// habituation/hysteresis).
    pub stable_basin_depth: f64,
    /// Whether the position has crossed into the crisis basin's pull this
    /// tick — a discrete readout of the underlying continuous dynamics,
    /// useful for downstream systems that need a boolean trigger.
    pub in_crisis_basin: bool,
    /// The basin that most strongly captures the position this tick: one
    /// of the built-in `stable`/`crisis` basins or a canon-defined one.
    #[serde(default = "default_current_basin")]
    pub current_basin: String,
    /// Where this human's built-in basins sit before canon shifts them
    /// (see [`BaseBasins::from_temperament`]).
    #[serde(default = "default_stable_center")]
    pub stable_center: f64,
    #[serde(default = "default_crisis_center")]
    pub crisis_center: f64,
    /// Canon slow neurochemicals and the landscape changes they drive.
    #[serde(default)]
    pub neurochemistry: NeurochemicalDynamics,
}

/// The built-in basins before canon modulation: their centers and the
/// stable basin's current depth.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BaseBasins {
    pub stable_center: f64,
    pub crisis_center: f64,
    pub stable_depth: f64,
}

impl Default for BaseBasins {
    fn default() -> Self {
        Self {
            stable_center: DEFAULT_STABLE_CENTER,
            crisis_center: DEFAULT_CRISIS_CENTER,
            stable_depth: 1.0,
        }
    }
}

impl BaseBasins {
    /// Each human's built-in basins, from their temperament:
    /// - emotional stability sets the stable basin's depth (resistance to
    ///   being knocked out of regulation) across its depth range, and
    ///   moves its center within `BASIN_CENTER_SPREAD` of 0.8, so a more
    ///   stable human settles nearer full regulation;
    /// - emotional intensity moves the crisis basin's center within
    ///   `BASIN_CENTER_SPREAD` of 0.15, so a more intense human's crises
    ///   run deeper.
    ///
    /// A median trait (0.5) gives the default basin; an unset (zero) trait
    /// also gives the default.
    pub fn from_temperament(profile: &HumanProfile) -> Self {
        let trait_or_median = |value: f32| {
            let value = value as f64;
            if value > 0.0 && value.is_finite() {
                value.min(1.0)
            } else {
                0.5
            }
        };
        let stability = trait_or_median(profile.temperament_matrix.emotional_stability);
        let intensity = trait_or_median(profile.temperament_matrix.emotional_intensity);
        Self {
            stable_center: DEFAULT_STABLE_CENTER + BASIN_CENTER_SPREAD * (2.0 * stability - 1.0),
            crisis_center: DEFAULT_CRISIS_CENTER - BASIN_CENTER_SPREAD * (2.0 * intensity - 1.0),
            stable_depth: (MAX_STABLE_DEPTH * stability).clamp(MIN_STABLE_DEPTH, MAX_STABLE_DEPTH),
        }
    }
}

fn default_stable_center() -> f64 {
    DEFAULT_STABLE_CENTER
}

fn default_crisis_center() -> f64 {
    DEFAULT_CRISIS_CENTER
}

/// Canon basin names the landscape modulation maps key into.
const STABLE_BASIN: &str = "stable";
const CRISIS_BASIN: &str = "crisis";

/// Seconds in one (Julian) year: canon neurochemical rates and time
/// constants are per second (`τ_dopamine ~200ms`, `τ_cortisol ~20min`).
const SECONDS_PER_YEAR: f64 = 365.25 * 24.0 * 3600.0;

/// Built-in basin centers for a median temperament.
const DEFAULT_STABLE_CENTER: f64 = 0.8;
const DEFAULT_CRISIS_CENTER: f64 = 0.15;
/// How far temperament moves a built-in basin's center either way. Both
/// basins stay on their side of `CRISIS_THRESHOLD`.
const BASIN_CENTER_SPREAD: f64 = 0.1;
const CRISIS_THRESHOLD: f64 = 0.4;
/// Restoring-pull depth of the built-in crisis basin, per year.
const CRISIS_DEPTH: f64 = 0.35;
/// Floor on a width-scaled basin distance, so a position at a basin's
/// center gives that basin a finite capture weight.
const MIN_BASIN_DISTANCE: f64 = 0.05;

/// Bounds on the stable basin's depth.
const MIN_STABLE_DEPTH: f64 = 0.3;
const MAX_STABLE_DEPTH: f64 = 2.0;

fn default_current_basin() -> String {
    STABLE_BASIN.to_string()
}

impl AttractorControlSnapshot {
    pub fn from_profile(profile: &HumanProfile) -> Self {
        let base = BaseBasins::from_temperament(profile);
        let Some(schema) = profile.canonical_schema() else {
            return Self {
                psychological_position: base.stable_center,
                stable_basin_depth: base.stable_depth,
                stable_center: base.stable_center,
                crisis_center: base.crisis_center,
                ..Self::defaults()
            };
        };
        let ac = &schema.extreme_brain_detail.neurochemical_attractor_control;

        Self {
            landscape_deformation_rate: nonzero_or(
                ac.attractor_modulation.landscape_deformation_rate,
                0.1,
            ),
            psychological_position: base.stable_center,
            stable_basin_depth: base.stable_depth,
            in_crisis_basin: false,
            current_basin: default_current_basin(),
            stable_center: base.stable_center,
            crisis_center: base.crisis_center,
            neurochemistry: NeurochemicalDynamics::from_schema(ac),
        }
    }

    /// Advance the landscape dynamics by one tick: gradient flow pulls
    /// `psychological_position` toward whichever basin currently has the
    /// stronger local pull, perturbed by real physiological/social
    /// distress signals already computed elsewhere in the engine.
    pub fn step(
        &self,
        needs: &super::needs::NeedsSnapshot,
        sensory: &super::sensory::SensorySnapshot,
        immune: &super::immune::ImmuneSnapshot,
        emotion: &super::emotion::EmotionSnapshot,
        dt_years: f64,
    ) -> Self {
        let dt = dt_years.clamp(0.0, 1.0);

        // Perturbation: real distress signals push the position down
        // (toward crisis); joy/contentment push it up (toward stability).
        let pain = sensory.overall_pain_level.clamp(0.0, 1.0);
        let sickness = (immune.system_stress + (immune.active_pathogen_count.min(5) as f64) * 0.1)
            .clamp(0.0, 1.0);
        let distress = (needs.fatigue * 0.4 + pain * 0.3 + sickness * 0.3).clamp(0.0, 1.0);
        let uplift = ((emotion.current.joy + emotion.current.contentment) / 2.0).clamp(0.0, 1.0);
        let neurochemistry = self.neurochemistry.step(distress, dt_years);
        let landscape = neurochemistry.landscape(BaseBasins {
            stable_center: self.stable_center,
            crisis_center: self.crisis_center,
            stable_depth: self.stable_basin_depth,
        });

        // Gradient flow over every basin, each weighted by how strongly it
        // captures the current position, plus the distress/uplift push.
        let velocity = landscape.velocity(self.psychological_position) + uplift - distress;
        let psychological_position = (self.psychological_position + velocity * dt).clamp(0.0, 1.0);

        // Hysteresis: a landscape that deforms slowly (low
        // landscape_deformation_rate) lets the stable basin deepen over
        // time spent stable, making the human progressively harder to
        // knock into crisis — real allostatic/habituation-style behavior.
        let deformation_resistance = (1.0 - self.landscape_deformation_rate).clamp(0.0, 1.0);
        let depth_delta = if psychological_position > CRISIS_THRESHOLD {
            deformation_resistance * 0.05 * dt
        } else {
            -0.1 * dt
        };
        let stable_basin_depth =
            (self.stable_basin_depth + depth_delta).clamp(MIN_STABLE_DEPTH, MAX_STABLE_DEPTH);

        Self {
            landscape_deformation_rate: self.landscape_deformation_rate,
            psychological_position,
            stable_basin_depth,
            in_crisis_basin: psychological_position < CRISIS_THRESHOLD,
            current_basin: landscape.dominant_basin(psychological_position),
            stable_center: self.stable_center,
            crisis_center: self.crisis_center,
            neurochemistry,
        }
    }

    fn defaults() -> Self {
        Self {
            landscape_deformation_rate: 0.1,
            psychological_position: DEFAULT_STABLE_CENTER,
            stable_basin_depth: 1.0,
            in_crisis_basin: false,
            current_basin: default_current_basin(),
            stable_center: DEFAULT_STABLE_CENTER,
            crisis_center: DEFAULT_CRISIS_CENTER,
            neurochemistry: NeurochemicalDynamics::default(),
        }
    }
}

/// One canon slow neurochemical: `dC/dt = production − k·C`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ChemicalKinetics {
    /// Current concentration (canon `concentration`).
    pub concentration: f64,
    /// Production per second (canon `baseline_rate`).
    pub baseline_rate: f64,
    /// First-order clearance per second: canon `clearance_rate`, or
    /// `1 / time_constant` when only the time constant is given.
    pub clearance_per_s: f64,
    /// Basin name → bias per unit concentration (canon
    /// `effect_on_attractors`).
    pub effect_on_attractors: BTreeMap<String, f64>,
    /// Extra production per unit of real distress (canon cortisol
    /// `stress_reactivity`); zero for chemicals canon gives none.
    pub stress_reactivity: f64,
}

/// A canon `feedback_loops` entry: `source` changes `target`'s production,
/// seen through a first-order lag with the canon `delay` as time constant.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FeedbackLoop {
    pub source: String,
    pub target: String,
    pub kind: FeedbackKind,
    pub strength: f64,
    /// Seconds (canon `delay`); zero means the target sees the source now.
    pub delay_s: f64,
    /// The source concentration as the target currently sees it.
    pub lagged_source: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FeedbackKind {
    /// Adds `strength · source` to the target's production.
    Positive,
    /// Removes `strength · source` from the target's production.
    Negative,
    /// Scales the target's production by `1 + strength · source`.
    Modulatory,
}

/// A canon `critical_transitions` entry: when `trigger` reaches
/// `threshold`, the landscape tips toward `after_state`, and stays tipped
/// until the concentration falls below `threshold · (1 − hysteresis)`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CriticalTransition {
    pub trigger: String,
    pub threshold: f64,
    pub after_state: String,
    pub hysteresis: f64,
}

/// Canon `NeurochemicalAttractorControlSchema` as integrated state.
///
/// Each chemical relaxes exactly toward `production / k` over the real
/// elapsed seconds, so no step length overshoots. Canon
/// `synthesis_modulation` and `clearance_modulation` scale a chemical's
/// production and clearance by `1 + m`. The landscape then shifts:
/// `effect_on_attractors` biases the position toward each basin in
/// proportion to concentration, `basin_depth_changes`,
/// `basin_position_shifts` and `basin_width_changes` reshape the basins
/// (and can define new ones, see [`NeurochemicalDynamics::landscape`]), and
/// an active critical transition pulls toward its `after_state` basin.
/// With canon's maps empty every term is zero and the landscape is the
/// unmodulated double well.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct NeurochemicalDynamics {
    pub chemicals: BTreeMap<String, ChemicalKinetics>,
    pub feedback_loops: Vec<FeedbackLoop>,
    pub synthesis_modulation: BTreeMap<String, f64>,
    pub clearance_modulation: BTreeMap<String, f64>,
    pub basin_depth_changes: BTreeMap<String, f64>,
    pub basin_position_shifts: BTreeMap<String, f64>,
    pub basin_width_changes: BTreeMap<String, f64>,
    pub critical_transitions: Vec<CriticalTransition>,
    /// Indices into `critical_transitions` that are currently tipped.
    pub active_transitions: BTreeSet<usize>,
}

/// One basin of the psychological landscape.
#[derive(Debug, Clone, PartialEq)]
pub struct Basin {
    pub name: String,
    /// Position of the basin's floor on the 0–1 axis.
    pub center: f64,
    /// Restoring-pull strength toward `center`, per year.
    pub depth: f64,
    /// Capture-width multiplier (≥ 0.1): a wider basin captures from
    /// further away.
    pub width: f64,
    /// Net neurochemical push toward this basin, per year: Σ over
    /// chemicals of concentration × `effect_on_attractors[name]`.
    pub chemical_pull: f64,
}

/// The basins the current neurochemistry shapes this step.
#[derive(Debug, Clone, PartialEq)]
pub struct Landscape {
    /// The built-in `stable` and `crisis` basins first, then every
    /// canon-defined basin.
    pub basins: Vec<Basin>,
    /// Basins that an active critical transition is tipping toward.
    pub tipped_toward: Vec<String>,
}

impl Landscape {
    pub fn basin(&self, name: &str) -> Option<&Basin> {
        self.basins.iter().find(|basin| basin.name == name)
    }

    /// Each basin's share of the position: inverse width-scaled distance,
    /// normalized, so the nearest basin dominates (the system is genuinely
    /// multistable rather than averaging its basins).
    pub fn capture_weights(&self, position: f64) -> Vec<f64> {
        let inverse: Vec<f64> = self
            .basins
            .iter()
            .map(|basin| {
                1.0 / ((basin.center - position).abs() / basin.width).max(MIN_BASIN_DISTANCE)
            })
            .collect();
        let total: f64 = inverse.iter().sum();
        inverse.iter().map(|weight| weight / total).collect()
    }

    /// Pull toward each basin an active critical transition names, with the
    /// same unit strength as the stable basin's own restoring pull.
    pub fn transition_pull(&self, position: f64) -> f64 {
        self.tipped_toward
            .iter()
            .filter_map(|name| self.basin(name))
            .map(|basin| basin.center - position)
            .sum()
    }

    /// Rate of change of the position, per year, from the landscape alone:
    /// each basin's capture-weighted restoring pull, each chemical push
    /// toward its basin, and the pull of any tipped transition.
    pub fn velocity(&self, position: f64) -> f64 {
        let weights = self.capture_weights(position);
        let restoring: f64 = self
            .basins
            .iter()
            .zip(&weights)
            .map(|(basin, weight)| weight * basin.depth * (basin.center - position))
            .sum();
        let chemical: f64 = self
            .basins
            .iter()
            .map(|basin| {
                let offset = basin.center - position;
                if offset == 0.0 {
                    0.0
                } else {
                    basin.chemical_pull * offset.signum()
                }
            })
            .sum();
        restoring + chemical + self.transition_pull(position)
    }

    /// The basin with the largest capture weight at `position`.
    pub fn dominant_basin(&self, position: f64) -> String {
        self.basins
            .iter()
            .zip(self.capture_weights(position))
            .max_by(|(_, a), (_, b)| a.total_cmp(b))
            .map(|(basin, _)| basin.name.clone())
            .unwrap_or_else(default_current_basin)
    }
}

fn map_f64(map: &BTreeMap<String, f32>) -> BTreeMap<String, f64> {
    map.iter().map(|(k, v)| (k.clone(), *v as f64)).collect()
}

impl NeurochemicalDynamics {
    pub fn from_schema(schema: &NeurochemicalAttractorControlSchema) -> Self {
        let chemicals = schema
            .neurochemical_state
            .iter()
            .map(|(name, entry)| {
                let clearance_per_s = if entry.clearance_rate > 0.0 {
                    entry.clearance_rate as f64
                } else if entry.time_constant > 0.0 {
                    1.0 / entry.time_constant as f64
                } else {
                    0.0
                };
                (
                    name.clone(),
                    ChemicalKinetics {
                        concentration: entry.concentration.max(0.0) as f64,
                        baseline_rate: entry.baseline_rate.max(0.0) as f64,
                        clearance_per_s,
                        effect_on_attractors: map_f64(&entry.effect_on_attractors),
                        stress_reactivity: entry.stress_reactivity.unwrap_or(0.0).max(0.0) as f64,
                    },
                )
            })
            .collect::<BTreeMap<_, _>>();
        let feedback_loops = schema
            .chemical_interactions
            .feedback_loops
            .iter()
            .map(|lp| FeedbackLoop {
                source: lp.source_chemical.clone(),
                target: lp.target_chemical.clone(),
                kind: match lp.interaction_type {
                    Some(FeedbackInteractionTypeSchema::Negative) => FeedbackKind::Negative,
                    Some(FeedbackInteractionTypeSchema::Modulatory) => FeedbackKind::Modulatory,
                    Some(FeedbackInteractionTypeSchema::Positive) | None => FeedbackKind::Positive,
                },
                strength: lp.strength as f64,
                delay_s: lp.delay.max(0.0) as f64,
                lagged_source: chemicals
                    .get(&lp.source_chemical)
                    .map_or(0.0, |c| c.concentration),
            })
            .collect();
        let modulation = &schema.attractor_modulation;
        Self {
            chemicals,
            feedback_loops,
            synthesis_modulation: map_f64(&schema.chemical_interactions.synthesis_modulation),
            clearance_modulation: map_f64(&schema.chemical_interactions.clearance_modulation),
            basin_depth_changes: map_f64(&modulation.basin_depth_changes),
            basin_position_shifts: map_f64(&modulation.basin_position_shifts),
            basin_width_changes: map_f64(&modulation.basin_width_changes),
            critical_transitions: modulation
                .critical_transitions
                .iter()
                .map(|t| CriticalTransition {
                    trigger: t.trigger_chemical.clone(),
                    threshold: t.threshold_concentration as f64,
                    after_state: t.after_state.clone(),
                    hysteresis: (t.hysteresis_strength as f64).clamp(0.0, 1.0),
                })
                .collect(),
            active_transitions: BTreeSet::new(),
        }
    }

    /// Integrate every chemical over `dt_years`, with `distress` (0–1) the
    /// real stress signal canon `stress_reactivity` responds to.
    pub fn step(&self, distress: f64, dt_years: f64) -> Self {
        let dt_s = dt_years.max(0.0) * SECONDS_PER_YEAR;
        let mut next = self.clone();

        // Feedback loops see their source through a first-order lag.
        for lp in &mut next.feedback_loops {
            let source = self
                .chemicals
                .get(&lp.source)
                .map_or(0.0, |c| c.concentration);
            lp.lagged_source = if lp.delay_s > 0.0 {
                source + (lp.lagged_source - source) * (-dt_s / lp.delay_s).exp()
            } else {
                source
            };
        }

        for (name, chemical) in &mut next.chemicals {
            let mut production = chemical.baseline_rate + chemical.stress_reactivity * distress;
            production *= 1.0 + self.synthesis_modulation.get(name).copied().unwrap_or(0.0);
            for lp in next.feedback_loops.iter().filter(|lp| &lp.target == name) {
                match lp.kind {
                    FeedbackKind::Positive => production += lp.strength * lp.lagged_source,
                    FeedbackKind::Negative => production -= lp.strength * lp.lagged_source,
                    FeedbackKind::Modulatory => production *= 1.0 + lp.strength * lp.lagged_source,
                }
            }
            let production = production.max(0.0);
            let k = chemical.clearance_per_s
                * (1.0 + self.clearance_modulation.get(name).copied().unwrap_or(0.0)).max(0.0);
            chemical.concentration = if k > 0.0 {
                let equilibrium = production / k;
                equilibrium + (chemical.concentration - equilibrium) * (-k * dt_s).exp()
            } else {
                chemical.concentration + production * dt_s
            }
            .max(0.0);
        }

        for (index, transition) in self.critical_transitions.iter().enumerate() {
            let level = next
                .chemicals
                .get(&transition.trigger)
                .map_or(0.0, |c| c.concentration);
            if level >= transition.threshold {
                next.active_transitions.insert(index);
            } else if level < transition.threshold * (1.0 - transition.hysteresis) {
                next.active_transitions.remove(&index);
            }
        }
        next
    }

    /// The landscape the current concentrations imply. The built-in
    /// basins start from `base` (the human's temperament-placed centers
    /// and the stable basin's learned depth; the crisis basin has a fixed
    /// depth), and canon shifts, deepens and widens them. Canon can also define further basins by
    /// name: one is placed at its `basin_position_shifts` value and takes
    /// its depth from `basin_depth_changes`, and it takes part only when
    /// canon gives it both a position and a positive depth.
    pub fn landscape(&self, base: BaseBasins) -> Landscape {
        let get = |map: &BTreeMap<String, f64>, name: &str| map.get(name).copied().unwrap_or(0.0);
        let chemical_pull = |name: &str| -> f64 {
            self.chemicals
                .values()
                .map(|c| c.concentration * c.effect_on_attractors.get(name).copied().unwrap_or(0.0))
                .sum()
        };
        let basin = |name: &str, base_center: f64, base_depth: f64| Basin {
            name: name.to_string(),
            center: (base_center + get(&self.basin_position_shifts, name)).clamp(0.0, 1.0),
            depth: (base_depth + get(&self.basin_depth_changes, name)).max(0.0),
            width: (1.0 + get(&self.basin_width_changes, name)).max(0.1),
            chemical_pull: chemical_pull(name),
        };
        let mut basins = vec![
            basin(STABLE_BASIN, base.stable_center, base.stable_depth),
            basin(CRISIS_BASIN, base.crisis_center, CRISIS_DEPTH),
        ];
        basins.extend(
            self.basin_position_shifts
                .keys()
                .filter(|name| name.as_str() != STABLE_BASIN && name.as_str() != CRISIS_BASIN)
                .map(|name| basin(name, 0.0, 0.0))
                .filter(|basin| basin.depth > 0.0),
        );
        Landscape {
            basins,
            tipped_toward: self
                .active_transitions
                .iter()
                .filter_map(|i| self.critical_transitions.get(*i))
                .map(|t| t.after_state.clone())
                .collect(),
        }
    }
}

fn nonzero_or(value: f32, default: f64) -> f64 {
    if value == 0.0 {
        default
    } else {
        value as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::humans::emotion::EmotionSnapshot;
    use crate::humans::immune::ImmuneSnapshot;
    use crate::humans::needs::NeedsSnapshot;
    use crate::humans::sensory::SensorySnapshot;
    use mk_core::human::{HumanId, HumanSchema};

    fn profile() -> HumanProfile {
        let schema = HumanSchema::canonical_minimal("attractor_test");
        HumanProfile::from_canonical_schema(HumanId::new(1), schema)
    }

    #[test]
    fn from_profile_starts_in_the_stable_basin() {
        let snapshot = AttractorControlSnapshot::from_profile(&profile());
        assert!(!snapshot.in_crisis_basin);
        assert!(snapshot.psychological_position > CRISIS_THRESHOLD);
    }

    #[test]
    fn sustained_severe_distress_can_pull_into_crisis_basin() {
        let snapshot = AttractorControlSnapshot::from_profile(&profile());
        let mut needs = NeedsSnapshot::from_profile(&profile());
        needs.fatigue = 1.0;
        let mut sensory = SensorySnapshot::from_profile(&profile());
        sensory.overall_pain_level = 1.0;
        let mut immune = ImmuneSnapshot::from_profile(&profile());
        immune.system_stress = 1.0;
        immune.active_pathogen_count = 5;
        let mut emotion = EmotionSnapshot::from_profile(&profile());
        emotion.current.joy = 0.0;
        emotion.current.contentment = 0.0;

        let mut stepped = snapshot.clone();
        for _ in 0..30 {
            stepped = stepped.step(&needs, &sensory, &immune, &emotion, 1.0);
        }

        assert!(stepped.psychological_position < snapshot.psychological_position);
    }

    #[test]
    fn healthy_state_stays_in_the_stable_basin() {
        let snapshot = AttractorControlSnapshot::from_profile(&profile());
        let needs = NeedsSnapshot::from_profile(&profile());
        let sensory = SensorySnapshot::from_profile(&profile());
        let immune = ImmuneSnapshot::from_profile(&profile());
        let mut emotion = EmotionSnapshot::from_profile(&profile());
        emotion.current.joy = 0.7;
        emotion.current.contentment = 0.7;

        let mut stepped = snapshot.clone();
        for _ in 0..20 {
            stepped = stepped.step(&needs, &sensory, &immune, &emotion, 1.0);
        }

        assert!(!stepped.in_crisis_basin);
    }

    use mk_core::human::schema::{
        CriticalTransitionSchema, FeedbackLoopSchema, NeurochemicalStateEntrySchema,
    };

    const SECOND_YEARS: f64 = 1.0 / SECONDS_PER_YEAR;

    fn chemical(
        concentration: f32,
        baseline_rate: f32,
        clearance_rate: f32,
    ) -> NeurochemicalStateEntrySchema {
        NeurochemicalStateEntrySchema {
            concentration,
            baseline_rate,
            clearance_rate,
            ..Default::default()
        }
    }

    fn schema_with(
        chemicals: Vec<(&str, NeurochemicalStateEntrySchema)>,
    ) -> NeurochemicalAttractorControlSchema {
        let mut schema = NeurochemicalAttractorControlSchema::default();
        for (name, entry) in chemicals {
            schema.neurochemical_state.insert(name.to_string(), entry);
        }
        schema
    }

    fn concentration(dynamics: &NeurochemicalDynamics, name: &str) -> f64 {
        dynamics.chemicals[name].concentration
    }

    #[test]
    fn empty_canon_neurochemistry_leaves_the_double_well_unmodulated() {
        let landscape = NeurochemicalDynamics::default().landscape(BaseBasins::default());
        let stable = landscape.basin(STABLE_BASIN).unwrap();
        let crisis = landscape.basin(CRISIS_BASIN).unwrap();
        assert_eq!(landscape.basins.len(), 2);
        assert_eq!(
            (stable.center, stable.depth, stable.width),
            (DEFAULT_STABLE_CENTER, 1.0, 1.0)
        );
        assert_eq!(
            (crisis.center, crisis.depth, crisis.width),
            (DEFAULT_CRISIS_CENTER, CRISIS_DEPTH, 1.0)
        );
        assert_eq!((stable.chemical_pull, crisis.chemical_pull), (0.0, 0.0));
        assert!(landscape.tipped_toward.is_empty());
    }

    #[test]
    fn temperament_places_and_deepens_the_built_in_basins() {
        let with = |stability: f32, intensity: f32| {
            let mut profile = profile();
            profile.temperament_matrix.emotional_stability = stability;
            profile.temperament_matrix.emotional_intensity = intensity;
            AttractorControlSnapshot::from_profile(&profile)
        };
        let median = with(0.5, 0.5);
        assert!((median.stable_center - DEFAULT_STABLE_CENTER).abs() < 1e-6);
        assert!((median.crisis_center - DEFAULT_CRISIS_CENTER).abs() < 1e-6);
        assert!((median.stable_basin_depth - 1.0).abs() < 1e-6);
        assert_eq!(median.psychological_position, median.stable_center);

        let steady = with(1.0, 0.01);
        assert!((steady.stable_center - 0.9).abs() < 1e-6);
        assert!((steady.crisis_center - 0.248).abs() < 1e-6);
        assert!((steady.stable_basin_depth - MAX_STABLE_DEPTH).abs() < 1e-6);

        let volatile = with(0.05, 1.0);
        assert!((volatile.stable_center - 0.71).abs() < 1e-6);
        assert!((volatile.crisis_center - 0.05).abs() < 1e-6);
        assert_eq!(volatile.stable_basin_depth, MIN_STABLE_DEPTH);
        for snapshot in [&steady, &volatile] {
            assert!(snapshot.stable_center > CRISIS_THRESHOLD);
            assert!(snapshot.crisis_center < CRISIS_THRESHOLD);
        }

        // Unset traits give the default basins.
        let unset = with(0.0, 0.0);
        assert_eq!(unset.stable_center, DEFAULT_STABLE_CENTER);
        assert_eq!(unset.crisis_center, DEFAULT_CRISIS_CENTER);
        assert_eq!(unset.stable_basin_depth, 1.0);
    }

    #[test]
    fn two_basin_capture_weights_match_the_double_well_formula() {
        let landscape = NeurochemicalDynamics::default().landscape(BaseBasins::default());
        let position = 0.5;
        let to_stable = (DEFAULT_STABLE_CENTER - position).abs();
        let to_crisis = (DEFAULT_CRISIS_CENTER - position).abs();
        let weights = landscape.capture_weights(position);
        assert!((weights[0] - to_crisis / (to_stable + to_crisis)).abs() < 1e-12);
        assert!((weights.iter().sum::<f64>() - 1.0).abs() < 1e-12);
    }

    #[test]
    fn a_canon_basin_joins_the_landscape_and_can_capture_the_position() {
        let mut schema = schema_with(vec![]);
        let modulation = &mut schema.attractor_modulation;
        modulation
            .basin_position_shifts
            .insert("rumination".into(), 0.5);
        modulation
            .basin_depth_changes
            .insert("rumination".into(), 1.5);
        // Named but never placed: not a basin.
        modulation
            .basin_depth_changes
            .insert("unplaced".into(), 1.0);
        let dynamics = NeurochemicalDynamics::from_schema(&schema);

        let landscape = dynamics.landscape(BaseBasins::default());
        let names: Vec<&str> = landscape.basins.iter().map(|b| b.name.as_str()).collect();
        assert_eq!(names, [STABLE_BASIN, CRISIS_BASIN, "rumination"]);
        let rumination = landscape.basin("rumination").unwrap();
        assert_eq!((rumination.center, rumination.depth), (0.5, 1.5));
        assert_eq!(landscape.dominant_basin(0.52), "rumination");
        // Just above the new basin, its pull wins over the stable basin's.
        assert!(landscape.velocity(0.55) < 0.0);

        let profile = profile();
        let mut snapshot = AttractorControlSnapshot::from_profile(&profile);
        snapshot.neurochemistry = dynamics;
        snapshot.psychological_position = 0.52;
        let needs = NeedsSnapshot::from_profile(&profile);
        let sensory = SensorySnapshot::from_profile(&profile);
        let immune = ImmuneSnapshot::from_profile(&profile);
        let emotion = EmotionSnapshot::from_profile(&profile);
        let stepped = snapshot.step(&needs, &sensory, &immune, &emotion, 0.01);
        assert_eq!(stepped.current_basin, "rumination");
    }

    #[test]
    fn a_chemical_pulls_toward_the_canon_basin_it_names() {
        let mania = |effect: f32| {
            let mut entry = chemical(1.0, 1.0, 1.0);
            entry.effect_on_attractors.insert("mania".into(), effect);
            let mut schema = schema_with(vec![("dopamine", entry)]);
            let modulation = &mut schema.attractor_modulation;
            modulation
                .basin_position_shifts
                .insert("mania".into(), 0.95);
            modulation.basin_depth_changes.insert("mania".into(), 0.5);
            NeurochemicalDynamics::from_schema(&schema).landscape(BaseBasins::default())
        };

        let plain = mania(0.0);
        let pulled = mania(1.0);

        assert!(pulled.basin("mania").unwrap().chemical_pull > 0.0);
        assert!(pulled.velocity(0.8) > plain.velocity(0.8));
    }

    #[test]
    fn kinetics_relax_exactly_to_production_over_clearance() {
        // dC/dt = 2 − 0.5·C: equilibrium 4, C(t) = 4 − 4·e^(−0.5 t) from 0.
        let dynamics = NeurochemicalDynamics::from_schema(&schema_with(vec![(
            "dopamine",
            chemical(0.0, 2.0, 0.5),
        )]));
        let after = dynamics.step(0.0, 3.0 * SECOND_YEARS);
        let expected = 4.0 - 4.0 * (-1.5f64).exp();
        assert!((concentration(&after, "dopamine") - expected).abs() < 1e-9);
        // One long step lands on the equilibrium without overshooting it.
        let long = dynamics.step(0.0, 1.0);
        assert!((concentration(&long, "dopamine") - 4.0).abs() < 1e-9);
    }

    #[test]
    fn time_constant_sets_clearance_when_no_rate_is_given() {
        let mut entry = chemical(10.0, 0.0, 0.0);
        entry.time_constant = 1200.0; // canon τ_cortisol ~20 min
        let dynamics = NeurochemicalDynamics::from_schema(&schema_with(vec![("cortisol", entry)]));
        let after = dynamics.step(0.0, 1200.0 * SECOND_YEARS);
        assert!((concentration(&after, "cortisol") - 10.0 * (-1.0f64).exp()).abs() < 1e-9);
    }

    #[test]
    fn stress_reactivity_raises_cortisol_under_real_distress() {
        let mut entry = chemical(1.0, 1.0, 1.0);
        entry.stress_reactivity = Some(3.0);
        let dynamics = NeurochemicalDynamics::from_schema(&schema_with(vec![("cortisol", entry)]));
        let calm = dynamics.step(0.0, 60.0 * SECOND_YEARS);
        let stressed = dynamics.step(1.0, 60.0 * SECOND_YEARS);
        assert!((concentration(&calm, "cortisol") - 1.0).abs() < 1e-9);
        assert!((concentration(&stressed, "cortisol") - 4.0).abs() < 1e-6);
    }

    #[test]
    fn negative_feedback_lowers_the_targets_equilibrium() {
        let mut schema = schema_with(vec![
            ("cortisol", chemical(2.0, 2.0, 1.0)),
            ("serotonin", chemical(3.0, 3.0, 1.0)),
        ]);
        schema
            .chemical_interactions
            .feedback_loops
            .push(FeedbackLoopSchema {
                source_chemical: "cortisol".into(),
                target_chemical: "serotonin".into(),
                interaction_type: Some(FeedbackInteractionTypeSchema::Negative),
                strength: 0.5,
                delay: 0.0,
            });
        let after = NeurochemicalDynamics::from_schema(&schema).step(0.0, 1.0);
        // Serotonin production 3 − 0.5·2 = 2 at clearance 1.
        assert!((concentration(&after, "serotonin") - 2.0).abs() < 1e-9);
        assert!((concentration(&after, "cortisol") - 2.0).abs() < 1e-9);
    }

    #[test]
    fn a_crisis_biasing_chemical_pulls_the_position_down() {
        let profile = profile();
        let needs = NeedsSnapshot::from_profile(&profile);
        let sensory = SensorySnapshot::from_profile(&profile);
        let immune = ImmuneSnapshot::from_profile(&profile);
        let emotion = EmotionSnapshot::from_profile(&profile);

        let plain = AttractorControlSnapshot::from_profile(&profile);
        let mut entry = chemical(1.0, 1.0, 1.0);
        entry.effect_on_attractors.insert(CRISIS_BASIN.into(), 2.0);
        let mut biased = plain.clone();
        biased.neurochemistry =
            NeurochemicalDynamics::from_schema(&schema_with(vec![("cortisol", entry)]));

        let dt = 0.05;
        let plain = plain.step(&needs, &sensory, &immune, &emotion, dt);
        let biased = biased.step(&needs, &sensory, &immune, &emotion, dt);
        assert!(biased.psychological_position < plain.psychological_position);
    }

    #[test]
    fn a_critical_transition_latches_with_hysteresis() {
        let mut schema = schema_with(vec![("cortisol", chemical(0.0, 0.0, 1.0))]);
        schema
            .attractor_modulation
            .critical_transitions
            .push(CriticalTransitionSchema {
                trigger_chemical: "cortisol".into(),
                threshold_concentration: 1.0,
                before_state: STABLE_BASIN.into(),
                after_state: CRISIS_BASIN.into(),
                hysteresis_strength: 0.5,
            });
        let mut dynamics = NeurochemicalDynamics::from_schema(&schema);
        let set = |d: &mut NeurochemicalDynamics, level: f64| {
            let c = d.chemicals.get_mut("cortisol").unwrap();
            c.concentration = level;
            c.baseline_rate = level; // hold the level: equilibrium = rate / 1
        };
        set(&mut dynamics, 1.2);
        let tipped = dynamics.step(0.0, SECOND_YEARS);
        assert_eq!(
            tipped.landscape(BaseBasins::default()).tipped_toward,
            vec![CRISIS_BASIN.to_string()]
        );
        assert!(tipped.landscape(BaseBasins::default()).transition_pull(0.8) < 0.0);

        let mut held = tipped.clone();
        set(&mut held, 0.7); // above 1.0 · (1 − 0.5): still tipped
        assert!(!held
            .step(0.0, SECOND_YEARS)
            .landscape(BaseBasins::default())
            .tipped_toward
            .is_empty());

        let mut released = tipped;
        set(&mut released, 0.4); // below the hysteresis band
        assert!(released
            .step(0.0, SECOND_YEARS)
            .landscape(BaseBasins::default())
            .tipped_toward
            .is_empty());
    }
}
