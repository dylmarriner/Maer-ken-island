//! Pathology Snapshot - real per-named psychiatric/trauma "attractor"
//! state, surfacing `docs/canon/HumanReplicationSchema.js`'s
//! `ExtremeBrainDetailSchema.brain_dynamics_module.pathology_failure_modes`
//! (`PathologyFailureModesSchema`), previously unread by the engine and
//! with zero prior implementation anywhere in `humans/*.rs`.
//!
//! **Not a clinical model.** Psychiatric illness is not reducible to a
//! scalar; this tracks one bounded `risk`
//! value per canon-named pathology (0 = asymptomatic, 1 = fully locked
//! into that attractor) using the same general-purpose bistable/hysteresis
//! dynamical-systems approach [`super::attractor_control`] already applies
//! to overall psychological stability — legitimate applied math, not an
//! assertion this matches any specific clinical model.
//!
//! Each pathology's canon `trap_depth`/`trap_width`/`escape_energy`/
//! `progression_rate`/`chronic_probability`/`treatment_susceptibility`
//! genuinely shape its dynamics: deeper/wider traps resist recovery more,
//! higher escape_energy demands longer sustained stability before risk
//! falls, chronic_probability sets a residual risk floor after an episode
//! rather than full recovery. Progression is driven by
//! [`super::attractor_control`]'s `in_crisis_basin` state (sustained
//! psychological dysregulation is a generic trigger for all of
//! trauma/depression/psychosis) together with the pathology's triggers.
//! Canon `trigger_factors` are read as [`Trigger`]s: each `factor_type`
//! reads a real engine signal ([`TriggerKind`]), acts only above its
//! `threshold_level`, and, when `cumulative_effect` is set, builds a load
//! that sheds `recovery_factor` per week once exposure stops. A pathology
//! canon gives no trigger factors for, such as the spectrum entries and
//! the engine defaults, is triggered by its category instead
//! ([`category_trigger`]). Canon's `symptom_profile` lists are carried as
//! [`Symptoms`]: once a pathology is active (risk above [`ACTIVE_RISK`]) it
//! presents them, deeper risk reaching further down the list
//! ([`PathologyState::presenting_symptom`]), and the inner monologue voices
//! the most active pathology's presenting symptom.

use mk_core::human::schema::{SymptomProfileSchema, TriggerFactorSchema, TriggerFactorTypeSchema};
use mk_core::human::HumanProfile;
use serde::{Deserialize, Serialize};

/// Canon `trigger_factors[].factor_type`, and the engine signal each reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TriggerKind {
    /// Acute threat: the stronger of fear and pain.
    Trauma,
    /// Dysregulation: how far the attractor position has fallen from fully
    /// regulated, `1 − psychological_position`.
    Stress,
    /// A standing predisposition, always fully present.
    Genetic,
    /// Adversity (the strongest of fear, sadness and despair) while the
    /// human is still developing (infant, child or adolescent stage).
    Developmental,
    /// Environmental insult: immune load from illness.
    Environmental,
}

/// Risk above which a pathology is active: clinically relevant, and
/// expressed through its symptoms.
pub const ACTIVE_RISK: f64 = 0.5;

/// Canon `symptom_profile`: the symptoms an active pathology presents.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Symptoms {
    pub cognitive: Vec<String>,
    pub emotional: Vec<String>,
    pub behavioral: Vec<String>,
    pub physiological: Vec<String>,
}

impl Symptoms {
    fn from_schema(schema: &SymptomProfileSchema) -> Self {
        let clean = |list: &[String]| -> Vec<String> {
            list.iter()
                .map(|symptom| symptom.trim().to_string())
                .filter(|symptom| !symptom.is_empty())
                .collect()
        };
        Self {
            cognitive: clean(&schema.cognitive_symptoms),
            emotional: clean(&schema.emotional_symptoms),
            behavioral: clean(&schema.behavioral_symptoms),
            physiological: clean(&schema.physiological_symptoms),
        }
    }

    /// Every symptom, in canon order: cognitive, emotional, behavioral,
    /// physiological.
    pub fn all(&self) -> impl Iterator<Item = &str> {
        self.cognitive
            .iter()
            .chain(&self.emotional)
            .chain(&self.behavioral)
            .chain(&self.physiological)
            .map(String::as_str)
    }
}

/// One canon trigger factor, with its accumulated load.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Trigger {
    pub kind: TriggerKind,
    /// Signal level (0–1) the factor must exceed to act.
    pub threshold: f64,
    /// Whether exposure accumulates into a lasting load rather than acting
    /// only while present.
    pub cumulative: bool,
    /// For a cumulative factor, the fraction of its load shed per week
    /// once exposure stops.
    pub recovery_factor: f64,
    /// Accumulated load of a cumulative factor (0–1).
    #[serde(default)]
    pub load: f64,
}

impl Trigger {
    fn from_schema(schema: &TriggerFactorSchema) -> Option<Self> {
        let kind = match schema.factor_type.as_ref()? {
            TriggerFactorTypeSchema::Trauma => TriggerKind::Trauma,
            TriggerFactorTypeSchema::Stress => TriggerKind::Stress,
            TriggerFactorTypeSchema::Genetic => TriggerKind::Genetic,
            TriggerFactorTypeSchema::Developmental => TriggerKind::Developmental,
            TriggerFactorTypeSchema::Environmental => TriggerKind::Environmental,
        };
        Some(Self {
            kind,
            threshold: (schema.threshold_level as f64).clamp(0.0, 1.0),
            cumulative: schema.cumulative_effect,
            recovery_factor: (schema.recovery_factor as f64).clamp(0.0, 1.0),
            load: 0.0,
        })
    }

    /// How far the signal exceeds the threshold, rescaled to 0–1.
    fn excess(&self, signal: f64) -> f64 {
        if signal <= self.threshold {
            0.0
        } else {
            ((signal - self.threshold) / (1.0 - self.threshold).max(f64::EPSILON)).min(1.0)
        }
    }

    /// Advance this factor over `weeks` of exposure to `signal`, returning
    /// the updated factor and its strength. A non-cumulative factor acts
    /// only while its signal exceeds the threshold. A cumulative factor's
    /// load grows by the excess per week and, once exposure stops, sheds
    /// `recovery_factor` of itself per week, exactly for any step length.
    fn step(&self, signal: f64, dt_years: f64, weeks: f64) -> (Self, f64) {
        let excess = self.excess(signal);
        if !self.cumulative {
            return (self.clone(), excess);
        }
        let load = if excess > 0.0 {
            (self.load + excess * weeks).min(1.0)
        } else {
            let shed = super::rates::relaxation_fraction(
                self.recovery_factor,
                dt_years,
                super::rates::WEEK_YEARS,
            );
            self.load * (1.0 - shed)
        };
        (
            Self {
                load,
                ..self.clone()
            },
            load,
        )
    }
}

/// The engine signals the canon trigger kinds read, this tick.
struct TriggerSignals {
    trauma: f64,
    stress: f64,
    developmental: f64,
    environmental: f64,
}

impl TriggerSignals {
    fn get(&self, kind: TriggerKind) -> f64 {
        match kind {
            TriggerKind::Trauma => self.trauma,
            TriggerKind::Stress => self.stress,
            TriggerKind::Genetic => 1.0,
            TriggerKind::Developmental => self.developmental,
            TriggerKind::Environmental => self.environmental,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathologyState {
    pub id: String,
    pub name: String,
    /// 0 = asymptomatic, 1 = fully locked into this attractor.
    pub risk: f64,
    /// Rises as `risk` deepens without recovering — a human who has spent
    /// a long time symptomatic resists recovery more (real trap depth
    /// growth, not just a static canon constant).
    trap_depth: f64,
    trap_width: f64,
    escape_energy: f64,
    progression_rate: f64,
    chronic_probability: f64,
    treatment_susceptibility: f64,
    /// Consecutive stable time outside the crisis basin, in weeks,
    /// accumulated toward this pathology's `escape_energy` recovery
    /// threshold (also in weeks). Snapshots from before this unit existed
    /// stored a per-step counter under `stable_ticks`; that field is not
    /// read, so a resumed human re-earns stability from zero rather than
    /// carrying a count in the wrong unit.
    #[serde(default)]
    stable_weeks: f64,
    /// Canon `trigger_factors`. Empty when canon lists none; the
    /// pathology's category then sets its trigger ([`category_trigger`]).
    #[serde(default)]
    pub triggers: Vec<Trigger>,
    /// Canon `symptom_profile`; empty for pathologies canon does not list.
    #[serde(default)]
    pub symptoms: Symptoms,
}

impl PathologyState {
    /// The symptom this pathology presents most at its current risk: the
    /// symptom list read in order, with deeper risk reaching further into
    /// it. `None` while inactive or when canon lists no symptoms.
    pub fn presenting_symptom(&self) -> Option<&str> {
        if self.risk <= ACTIVE_RISK {
            return None;
        }
        let count = self.symptoms.all().count();
        let depth = (self.risk - ACTIVE_RISK) / (1.0 - ACTIVE_RISK);
        let index = ((depth * count as f64) as usize).min(count.checked_sub(1)?);
        self.symptoms.all().nth(index)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathologySnapshot {
    pub pathologies: Vec<PathologyState>,
}

impl PathologySnapshot {
    pub fn from_profile(profile: &HumanProfile) -> Self {
        let Some(schema) = profile.canonical_schema() else {
            return Self::defaults();
        };
        let p = &schema
            .extreme_brain_detail
            .brain_dynamics_module
            .pathology_failure_modes;

        let mut pathologies: Vec<PathologyState> = p
            .pathological_attractors
            .iter()
            .map(|a| PathologyState {
                id: a.pathology_id.clone(),
                name: a.pathology_name.clone(),
                risk: 0.0,
                trap_depth: nz(a.trap_depth, 0.4),
                trap_width: nz(a.trap_width, 0.3),
                escape_energy: nz(a.escape_energy, 5.0),
                progression_rate: nz(a.progression_rate, 0.15),
                chronic_probability: nz(a.chronic_probability, 0.2),
                treatment_susceptibility: nz(a.treatment_susceptibility, 0.5),
                stable_weeks: 0.0,
                triggers: a
                    .trigger_factors
                    .iter()
                    .filter_map(Trigger::from_schema)
                    .collect(),
                symptoms: Symptoms::from_schema(&a.symptom_profile),
            })
            .collect();

        // Canon always defines the trauma/depression/psychosis spectrum
        // BTreeMaps even when `pathological_attractors` (the free-form
        // list) is empty, so this snapshot never sits inert on the
        // canonical-minimal fixture — synthesize one tracked pathology per
        // named spectrum entry from those parameter maps.
        // Each spectrum field is a single flat parameter map describing
        // one pathology (not a map of multiple named pathologies).
        for (id, params) in [
            ("ptsd_attractor", &p.trauma_spectrum.ptsd_attractor),
            (
                "acute_stress_reaction",
                &p.trauma_spectrum.acute_stress_reaction,
            ),
            ("complex_trauma", &p.trauma_spectrum.complex_trauma),
        ] {
            if !params.is_empty() {
                pathologies.push(synthesize("trauma", id, params));
            }
        }
        for (id, params) in [
            (
                "major_depression_attractor",
                &p.depression_spectrum.major_depression_attractor,
            ),
            (
                "dysthymia_attractor",
                &p.depression_spectrum.dysthymia_attractor,
            ),
            (
                "seasonal_affective_pattern",
                &p.depression_spectrum.seasonal_affective_pattern,
            ),
        ] {
            if !params.is_empty() {
                pathologies.push(synthesize("depression", id, params));
            }
        }
        if !p.psychosis_spectrum.schizophrenia_attractor.is_empty() {
            pathologies.push(synthesize(
                "psychosis",
                "schizophrenia_attractor",
                &p.psychosis_spectrum.schizophrenia_attractor,
            ));
        }

        if pathologies.is_empty() {
            pathologies = default_pathologies();
        }

        Self { pathologies }
    }

    /// Advance every pathology's risk toward its bistable attractor,
    /// driven by real sustained-dysregulation state
    /// ([`super::attractor_control`]'s `in_crisis_basin`) and per-pathology
    /// distress triggers.
    pub fn step(
        &self,
        attractor_control: &super::attractor_control::AttractorControlSnapshot,
        emotion: &super::emotion::EmotionSnapshot,
        sensory: &super::sensory::SensorySnapshot,
        immune: &super::immune::ImmuneSnapshot,
        development: &super::development::DevelopmentSnapshot,
        dt_years: f64,
    ) -> Self {
        // Progression and recovery rates are per week, and stability is
        // counted in weeks: psychiatric states shift over weeks, not ticks.
        let weeks = super::rates::elapsed(dt_years, super::rates::WEEK_YEARS);

        let fear = emotion.current.fear.clamp(0.0, 1.0);
        let sadness = emotion.current.sadness.clamp(0.0, 1.0);
        let despair = emotion.current.despair.clamp(0.0, 1.0);
        let pain = sensory.overall_pain_level.clamp(0.0, 1.0);
        let sickness = (immune.system_stress + (immune.active_pathogen_count.min(5) as f64) * 0.1)
            .clamp(0.0, 1.0);

        let developing = matches!(
            development.stage,
            super::development::DevelopmentStage::Infant
                | super::development::DevelopmentStage::Child
                | super::development::DevelopmentStage::Adolescent
        );
        let signals = TriggerSignals {
            trauma: fear.max(pain),
            stress: (1.0 - attractor_control.psychological_position).clamp(0.0, 1.0),
            developmental: if developing {
                fear.max(sadness).max(despair)
            } else {
                0.0
            },
            environmental: sickness,
        };

        let pathologies = self
            .pathologies
            .iter()
            .map(|p| {
                // Canon trigger factors each apply their own threshold, and
                // the strongest sets the trigger. Without canon factors the
                // category trigger applies, gated at 0.3.
                let stepped: Vec<(Trigger, f64)> = p
                    .triggers
                    .iter()
                    .map(|t| t.step(signals.get(t.kind), dt_years, weeks))
                    .collect();
                let (trigger, gate) = if stepped.is_empty() {
                    (
                        category_trigger(&p.id, fear, sadness, despair, pain, sickness),
                        CATEGORY_TRIGGER_GATE,
                    )
                } else {
                    let strongest = stepped
                        .iter()
                        .map(|(_, strength)| *strength)
                        .fold(0.0, f64::max);
                    (strongest, 0.0)
                };
                let triggers: Vec<Trigger> = stepped.into_iter().map(|(t, _)| t).collect();

                let in_crisis = attractor_control.in_crisis_basin;
                let deepening = trap_pull(p.trap_depth, p.trap_width, p.risk);

                let (risk, stable_weeks) = if in_crisis && trigger > gate {
                    let growth = p.progression_rate * trigger * deepening * weeks;
                    ((p.risk + growth).clamp(0.0, 1.0), 0.0)
                } else {
                    let stable_weeks = p.stable_weeks + weeks;
                    if stable_weeks >= p.escape_energy {
                        // Enough sustained stability to genuinely recover,
                        // gated by treatment_susceptibility; chronic_
                        // probability sets a residual floor rather than
                        // full recovery — real illness/trauma can leave a
                        // lasting baseline shift.
                        let floor = p.risk * p.chronic_probability;
                        // Risk decays at treatment_susceptibility × 0.1 per
                        // week, exactly for any step length.
                        let decay = super::rates::relaxation_fraction(
                            p.treatment_susceptibility * 0.1,
                            dt_years,
                            super::rates::WEEK_YEARS,
                        );
                        let recovered = (p.risk * (1.0 - decay)).max(floor);
                        (recovered.clamp(0.0, 1.0), stable_weeks)
                    } else {
                        (p.risk, stable_weeks)
                    }
                };

                PathologyState {
                    risk,
                    stable_weeks,
                    triggers,
                    ..p.clone()
                }
            })
            .collect();

        Self { pathologies }
    }

    /// Whether any tracked pathology has crossed into clinically-relevant
    /// territory (risk > 0.5) — a single boolean readout for downstream
    /// systems that need one without inspecting the full list.
    pub fn any_active(&self) -> bool {
        self.pathologies.iter().any(|p| p.risk > ACTIVE_RISK)
    }

    /// The active pathology with the highest risk, if any.
    pub fn most_active(&self) -> Option<&PathologyState> {
        self.pathologies
            .iter()
            .filter(|p| p.risk > ACTIVE_RISK)
            .max_by(|a, b| a.risk.total_cmp(&b.risk))
    }

    fn defaults() -> Self {
        Self {
            pathologies: default_pathologies(),
        }
    }
}

/// Trigger level below which a category trigger does not progress a
/// pathology.
const CATEGORY_TRIGGER_GATE: f64 = 0.3;

/// The trigger for a pathology canon gives no `trigger_factors` for, by
/// its category: threat for trauma, low mood for depression, illness with
/// fear and despair for psychosis, and a general distress mix otherwise.
fn category_trigger(
    id: &str,
    fear: f64,
    sadness: f64,
    despair: f64,
    pain: f64,
    sickness: f64,
) -> f64 {
    let trigger = if id.contains("trauma") || id.contains("ptsd") || id.contains("stress") {
        fear * 0.6 + pain * 0.4
    } else if id.contains("depress") || id.contains("dysthymia") {
        sadness * 0.5 + despair * 0.5
    } else if id.contains("psychosis") || id.contains("schizo") {
        sickness * 0.5 + fear * 0.3 + despair * 0.2
    } else {
        fear * 0.3 + sadness * 0.3 + sickness * 0.4
    };
    trigger.clamp(0.0, 1.0)
}

/// A wide/deep trap resists both entry and exit; this returns how strongly
/// the current risk level is "pulled" further into the trap once triggered
/// — a human already partway in sinks faster than one just crossing the
/// threshold (real hysteresis, not a fixed rate).
fn trap_pull(trap_depth: f64, trap_width: f64, risk: f64) -> f64 {
    (trap_depth * (0.3 + risk * 0.7) * (1.0 + trap_width * 0.5)).clamp(0.1, 3.0)
}

fn synthesize(
    category: &str,
    id: &str,
    params: &std::collections::BTreeMap<String, f32>,
) -> PathologyState {
    let get = |key: &str, default: f64| {
        params
            .get(key)
            .copied()
            .map(|v| v as f64)
            .filter(|v| *v != 0.0)
            .unwrap_or(default)
    };
    PathologyState {
        id: format!("{category}_{id}"),
        name: id.replace('_', " "),
        risk: 0.0,
        trap_depth: get("trap_depth", 0.4),
        trap_width: get("trap_width", 0.3),
        escape_energy: get("escape_energy", 5.0),
        progression_rate: get("progression_rate", 0.15),
        chronic_probability: get("chronic_probability", 0.2),
        treatment_susceptibility: get("treatment_susceptibility", 0.5),
        stable_weeks: 0.0,
        triggers: Vec::new(),
        symptoms: Symptoms::default(),
    }
}

fn default_pathologies() -> Vec<PathologyState> {
    [
        ("trauma_ptsd", "PTSD", 0.4, 0.3, 6.0, 0.15, 0.25),
        (
            "depression_major",
            "Major Depression",
            0.35,
            0.35,
            5.0,
            0.15,
            0.2,
        ),
        (
            "psychosis_schizophrenia",
            "Schizophrenia Spectrum",
            0.5,
            0.4,
            8.0,
            0.1,
            0.35,
        ),
    ]
    .into_iter()
    .map(
        |(
            id,
            name,
            trap_depth,
            trap_width,
            escape_energy,
            progression_rate,
            chronic_probability,
        )| {
            PathologyState {
                id: id.to_string(),
                name: name.to_string(),
                risk: 0.0,
                trap_depth,
                trap_width,
                escape_energy,
                progression_rate,
                chronic_probability,
                treatment_susceptibility: 0.5,
                stable_weeks: 0.0,
                triggers: Vec::new(),
                symptoms: Symptoms::default(),
            }
        },
    )
    .collect()
}

fn nz(value: f32, default: f64) -> f64 {
    if value == 0.0 {
        default
    } else {
        value as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::humans::attractor_control::AttractorControlSnapshot;
    use crate::humans::emotion::EmotionSnapshot;
    use crate::humans::immune::ImmuneSnapshot;
    use crate::humans::sensory::SensorySnapshot;
    use mk_core::human::{HumanId, HumanSchema};

    fn profile() -> HumanProfile {
        let schema = HumanSchema::canonical_minimal("pathology_test");
        HumanProfile::from_canonical_schema(HumanId::new(1), schema)
    }

    fn development() -> crate::humans::development::DevelopmentSnapshot {
        crate::humans::development::DevelopmentSnapshot::new(30.0)
    }

    fn trigger(kind: TriggerKind, threshold: f64, cumulative: bool) -> Trigger {
        Trigger {
            kind,
            threshold,
            cumulative,
            recovery_factor: 0.5,
            load: 0.0,
        }
    }

    /// One pathology driven only by `triggers`, stepped for `weeks` with
    /// the attractor in crisis and the given fear.
    fn run(triggers: Vec<Trigger>, fear: f64, age: f64, weeks: usize) -> PathologySnapshot {
        let mut snapshot = PathologySnapshot::from_profile(&profile());
        snapshot.pathologies.truncate(1);
        snapshot.pathologies[0].triggers = triggers;
        let mut attractor = AttractorControlSnapshot::from_profile(&profile());
        attractor.in_crisis_basin = true;
        let mut emotion = EmotionSnapshot::from_profile(&profile());
        emotion.current.fear = fear;
        emotion.current.sadness = 0.0;
        emotion.current.despair = 0.0;
        let sensory = SensorySnapshot::from_profile(&profile());
        let immune = ImmuneSnapshot::from_profile(&profile());
        let development = crate::humans::development::DevelopmentSnapshot::new(age);
        let week = crate::humans::rates::WEEK_YEARS;
        for _ in 0..weeks {
            snapshot = snapshot.step(&attractor, &emotion, &sensory, &immune, &development, week);
        }
        snapshot
    }

    #[test]
    fn an_active_pathology_presents_its_canon_symptoms() {
        let mut state = PathologySnapshot::from_profile(&profile()).pathologies[0].clone();
        state.symptoms = Symptoms {
            cognitive: vec!["intrusive memories".into()],
            emotional: vec!["numbness".into()],
            ..Default::default()
        };
        state.risk = 0.4;
        assert_eq!(state.presenting_symptom(), None);
        state.risk = 0.6;
        assert_eq!(state.presenting_symptom(), Some("intrusive memories"));
        state.risk = 0.95;
        assert_eq!(state.presenting_symptom(), Some("numbness"));
        state.symptoms = Symptoms::default();
        assert_eq!(state.presenting_symptom(), None);
    }

    #[test]
    fn canon_trigger_factors_are_read() {
        use mk_core::human::schema::PathologicalAttractorSchema;
        let mut schema = HumanSchema::canonical_minimal("pathology_triggers");
        schema
            .extreme_brain_detail
            .brain_dynamics_module
            .pathology_failure_modes
            .pathological_attractors
            .push(PathologicalAttractorSchema {
                pathology_id: "panic".into(),
                pathology_name: "Panic".into(),
                symptom_profile: SymptomProfileSchema {
                    behavioral_symptoms: vec!["avoidance".into(), " ".into()],
                    ..Default::default()
                },
                trigger_factors: vec![TriggerFactorSchema {
                    factor_type: Some(TriggerFactorTypeSchema::Trauma),
                    threshold_level: 0.6,
                    cumulative_effect: true,
                    recovery_factor: 0.25,
                }],
                ..Default::default()
            });
        let profile = HumanProfile::from_canonical_schema(HumanId::new(2), schema);

        let snapshot = PathologySnapshot::from_profile(&profile);

        let panic = snapshot
            .pathologies
            .iter()
            .find(|p| p.id == "panic")
            .unwrap();
        assert_eq!(panic.symptoms.behavioral, vec!["avoidance".to_string()]);
        assert_eq!(
            panic.triggers,
            vec![Trigger {
                kind: TriggerKind::Trauma,
                threshold: 0.6_f32 as f64,
                cumulative: true,
                recovery_factor: 0.25,
                load: 0.0,
            }]
        );
    }

    #[test]
    fn a_trigger_below_its_threshold_does_not_progress_risk() {
        let below = run(
            vec![trigger(TriggerKind::Trauma, 0.8, false)],
            0.7,
            30.0,
            20,
        );
        let above = run(
            vec![trigger(TriggerKind::Trauma, 0.5, false)],
            0.7,
            30.0,
            20,
        );
        assert_eq!(below.pathologies[0].risk, 0.0);
        assert!(above.pathologies[0].risk > 0.0);
    }

    #[test]
    fn a_cumulative_trigger_builds_load_and_sheds_it_after_exposure() {
        let exposed = run(vec![trigger(TriggerKind::Trauma, 0.5, true)], 1.0, 30.0, 3);
        let load = exposed.pathologies[0].triggers[0].load;
        assert!((load - 1.0).abs() < 1e-12, "3 weeks at full excess: {load}");

        // A week without exposure sheds recovery_factor (0.5 per week).
        let mut calm = exposed.clone();
        let attractor = AttractorControlSnapshot::from_profile(&profile());
        let mut emotion = EmotionSnapshot::from_profile(&profile());
        emotion.current.fear = 0.0;
        let sensory = SensorySnapshot::from_profile(&profile());
        let immune = ImmuneSnapshot::from_profile(&profile());
        calm = calm.step(
            &attractor,
            &emotion,
            &sensory,
            &immune,
            &development(),
            crate::humans::rates::WEEK_YEARS,
        );
        let shed = calm.pathologies[0].triggers[0].load;
        assert!((shed - (-0.5f64).exp()).abs() < 1e-9, "{shed}");
    }

    #[test]
    fn a_developmental_trigger_acts_only_while_developing() {
        let child = run(
            vec![trigger(TriggerKind::Developmental, 0.2, false)],
            0.9,
            8.0,
            10,
        );
        let adult = run(
            vec![trigger(TriggerKind::Developmental, 0.2, false)],
            0.9,
            30.0,
            10,
        );
        assert!(child.pathologies[0].risk > 0.0);
        assert_eq!(adult.pathologies[0].risk, 0.0);
    }

    #[test]
    fn a_genetic_predisposition_progresses_risk_in_any_crisis() {
        let snapshot = run(
            vec![trigger(TriggerKind::Genetic, 0.5, false)],
            0.0,
            30.0,
            10,
        );
        assert!(snapshot.pathologies[0].risk > 0.0);
    }

    #[test]
    fn from_profile_starts_asymptomatic() {
        let snapshot = PathologySnapshot::from_profile(&profile());
        assert!(!snapshot.pathologies.is_empty());
        assert!(!snapshot.any_active());
    }

    #[test]
    fn sustained_crisis_and_fear_raise_trauma_risk() {
        let snapshot = PathologySnapshot::from_profile(&profile());
        let mut attractor = AttractorControlSnapshot::from_profile(&profile());
        attractor.in_crisis_basin = true;
        let mut emotion = EmotionSnapshot::from_profile(&profile());
        emotion.current.fear = 1.0;
        let sensory = SensorySnapshot::from_profile(&profile());
        let immune = ImmuneSnapshot::from_profile(&profile());

        let mut stepped = snapshot.clone();
        for _ in 0..50 {
            stepped = stepped.step(&attractor, &emotion, &sensory, &immune, &development(), 1.0);
        }

        let trauma = stepped
            .pathologies
            .iter()
            .find(|p| p.id.contains("trauma"))
            .expect("trauma pathology tracked");
        let baseline = snapshot
            .pathologies
            .iter()
            .find(|p| p.id.contains("trauma"))
            .unwrap();
        assert!(trauma.risk > baseline.risk);
    }

    #[test]
    fn sustained_stability_recovers_risk_toward_chronic_floor() {
        let mut snapshot = PathologySnapshot::from_profile(&profile());
        for p in &mut snapshot.pathologies {
            p.risk = 0.8;
        }
        let mut attractor = AttractorControlSnapshot::from_profile(&profile());
        attractor.in_crisis_basin = false;
        let mut emotion = EmotionSnapshot::from_profile(&profile());
        emotion.current.fear = 0.0;
        emotion.current.sadness = 0.0;
        emotion.current.despair = 0.0;
        let sensory = SensorySnapshot::from_profile(&profile());
        let immune = ImmuneSnapshot::from_profile(&profile());

        let mut stepped = snapshot.clone();
        for _ in 0..200 {
            stepped = stepped.step(&attractor, &emotion, &sensory, &immune, &development(), 1.0);
        }

        for (before, after) in snapshot.pathologies.iter().zip(stepped.pathologies.iter()) {
            assert!(
                after.risk <= before.risk,
                "{} should not worsen while stable",
                before.id
            );
        }
    }

    #[test]
    fn a_legacy_stability_counter_is_not_read_as_weeks() {
        let snapshot = PathologySnapshot::from_profile(&HumanProfile::from_canonical_schema(
            mk_core::human::HumanId::new(4),
            mk_core::human::HumanSchema::canonical_minimal("legacy_pathology"),
        ));
        let mut value = serde_json::to_value(&snapshot).unwrap();
        for p in value["pathologies"].as_array_mut().unwrap() {
            let object = p.as_object_mut().unwrap();
            object.remove("stable_weeks");
            object.insert("stable_ticks".to_string(), serde_json::json!(1.0e6));
        }
        let restored: PathologySnapshot = serde_json::from_value(value).unwrap();
        assert!(!restored.pathologies.is_empty());
        assert!(restored.pathologies.iter().all(|p| p.stable_weeks == 0.0));
    }
}
