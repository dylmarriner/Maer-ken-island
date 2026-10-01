//! Neurochemistry Snapshot - neurotransmitter/hormone levels and their
//! aggregate modulation effects on cognition.
//!
//! Surfaces `docs/canon/HumanReplicationSchema.js`'s
//! `ExtremeBrainDetailSchema.layer_3_neurotransmitter_modulation`
//! (`NeurotransmitterModulationSchema`), ported in `mk_core::human::schema`
//! but previously unread by the engine. Canon defines six neurotransmitters
//! (dopamine, serotonin, norepinephrine, acetylcholine, GABA, glutamate)
//! and — via `BioSysConfigSchema.endocrine_baselines` — all ten canon
//! hormones (cortisol, oxytocin, testosterone, estrogen, progesterone,
//! vasopressin, adrenaline, melatonin, plus dopamine/serotonin which are
//! deliberately tracked only via the neurotransmitter path below to avoid a
//! second competing source of truth for the same substance), each with a
//! `level` plus several derived-effect fields; this snapshot tracks `level`
//! as real per-tick state (driven by the closest already-modeled engine
//! signal for each substance) and computes canon's `modulation_effects`
//! block as an aggregate of those levels, rather than also modeling each
//! substance's individual derived-effect fields (`learning_rate_modulation`,
//! `stress_response`, etc.) as separate state — those are one-line
//! functions of `level` a caller can derive directly if needed, and
//! duplicating them as tracked fields would be redundant state with no
//! additional behavior.
//!
//! This module reads several already-stepped systems and never writes into
//! them. Systems that step before it read its previous-tick state instead:
//! `learning::LearningSnapshot::step` scales its learning signal by
//! `learning_rate_multiplier`, and memory reads melatonin. That keeps the
//! tick pipeline acyclic.

use mk_core::human::HumanProfile;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BrainState {
    Resting,
    Focused,
    Stressed,
    Relaxed,
    Excited,
    Fatigued,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeurotransmitterLevels {
    pub dopamine: f64,
    pub serotonin: f64,
    pub norepinephrine: f64,
    pub acetylcholine: f64,
    pub gaba: f64,
    pub glutamate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HormoneLevels {
    pub cortisol: f64,
    pub oxytocin: f64,
    pub testosterone: f64,
    pub estrogen: f64,
    /// Luteal-phase reproductive hormone; here a coarse proxy driven by
    /// this human's fertility state rather than a modeled day-by-day
    /// menstrual cycle (no cycle-phase system exists yet to drive it
    /// honestly at finer grain).
    pub progesterone: f64,
    /// Social-bonding/fluid-regulation hormone; driven by trust-based
    /// bonding (its social role) and hydration deficit (its renal role) —
    /// the two already-modeled engine signals closest to its two canon
    /// functions.
    pub vasopressin: f64,
    /// Acute fight-or-flight hormone, distinct from the chronic-stress
    /// `cortisol` above; driven by immediate threat/fear rather than
    /// sustained fatigue/immune stress.
    pub adrenaline: f64,
    /// Circadian hormone; driven by the real `daylight_fraction` signal
    /// from this human's local `AgentWorldObservation` (low daylight ->
    /// high melatonin).
    pub melatonin: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModulationEffects {
    pub current_brain_state: BrainState,
    pub learning_rate_multiplier: f64,
    pub emotional_bias: f64,
    pub cognitive_load_capacity: f64,
    pub decision_threshold: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeurochemistrySnapshot {
    pub neurotransmitters: NeurotransmitterLevels,
    pub hormones: HormoneLevels,
    pub modulation_effects: ModulationEffects,
}

impl NeurochemistrySnapshot {
    pub fn from_profile(profile: &HumanProfile) -> Self {
        let Some(schema) = profile.canonical_schema() else {
            return Self::defaults();
        };
        let n = &schema
            .extreme_brain_detail
            .layer_3_neurotransmitter_modulation;

        let level = |map: &std::collections::BTreeMap<
            String,
            mk_core::human::schema::NeurotransmitterDetailSchema,
        >,
                     key: &str,
                     default: f64| {
            map.get(key)
                .and_then(|d| d.values.get("level"))
                .copied()
                .map(|v| v as f64)
                .filter(|v| *v != 0.0)
                .unwrap_or(default)
        };

        let neurotransmitters = NeurotransmitterLevels {
            dopamine: level(&n.neurotransmitters, "dopamine", 0.5),
            serotonin: level(&n.neurotransmitters, "serotonin", 0.5),
            norepinephrine: level(&n.neurotransmitters, "norepinephrine", 0.3),
            acetylcholine: level(&n.neurotransmitters, "acetylcholine", 0.5),
            gaba: level(&n.neurotransmitters, "gaba", 0.5),
            glutamate: level(&n.neurotransmitters, "glutamate", 0.4),
        };

        // Starting hormone levels come from `core_systems.biosys.endocrine_
        // baselines` (this human's actual canon-specified resting endocrine
        // state) rather than fixed constants. Of the ten `endocrine_
        // baselines` fields, eight are tracked here as real `HormoneLevels`
        // state; dopamine and serotonin are deliberately tracked only via
        // the neurotransmitter path above — using `endocrine_baselines` for
        // those two as well would be a second, competing source of truth
        // for the same substance.
        let eb = &schema.core_systems.biosys.endocrine_baselines;
        let hormones = HormoneLevels {
            cortisol: level(&n.hormones, "cortisol", nz(eb.cortisol, 0.3)),
            oxytocin: level(&n.hormones, "oxytocin", nz(eb.oxytocin, 0.3)),
            testosterone: level(&n.hormones, "testosterone", nz(eb.testosterone, 0.4)),
            estrogen: level(&n.hormones, "estrogen", nz(eb.estrogen, 0.4)),
            progesterone: nz(eb.progesterone, 0.4),
            vasopressin: nz(eb.vasopressin, 0.4),
            adrenaline: nz(eb.adrenaline, 0.3),
            melatonin: nz(eb.melatonin, 0.3),
        };

        let modulation_effects = Self::compute_modulation(&neurotransmitters, &hormones);

        Self {
            neurotransmitters,
            hormones,
            modulation_effects,
        }
    }

    /// Drive each substance's level from the already-modeled engine signal
    /// most plausibly responsible for it, then recompute the aggregate
    /// modulation effects.
    #[allow(clippy::too_many_arguments)]
    pub fn step(
        &self,
        emotion: &super::emotion::EmotionSnapshot,
        needs: &super::needs::NeedsSnapshot,
        immune: &super::immune::ImmuneSnapshot,
        attention: &super::attention::AttentionSnapshot,
        social_cognition: &super::social_cognition::SocialCognitionSnapshot,
        dark_triad: &super::dark_triad::DarkTriadSnapshot,
        reproduction: &super::reproduction::ReproductiveSystemSnapshot,
        observation: &super::AgentWorldObservation,
        dt_years: f64,
    ) -> Self {
        let blend = (0.5 * dt_years.max(0.0)).clamp(0.0, 1.0);

        let dopamine_target =
            (emotion.current.joy * 0.5 + emotion.current.triumph * 0.5).clamp(0.0, 1.0);
        let serotonin_target = emotion.current.contentment.clamp(0.0, 1.0);
        let norepinephrine_target =
            (emotion.current.fear * 0.6 + needs.fatigue * 0.4).clamp(0.0, 1.0);
        let acetylcholine_target = attention.available.clamp(0.0, 1.0);
        let gaba_target = (1.0 - emotion.current.anger.max(emotion.current.fear)).clamp(0.0, 1.0);
        let glutamate_target = attention.cognitive_load.clamp(0.0, 1.0);

        let cortisol_target = (needs.fatigue * 0.5 + immune.system_stress * 0.5).clamp(0.0, 1.0);
        let oxytocin_target =
            (social_cognition.trust_assessment * 0.6 + emotion.current.love * 0.4).clamp(0.0, 1.0);
        let testosterone_target =
            (dark_triad.active_malice * 0.5 + emotion.current.anger * 0.5).clamp(0.0, 1.0);
        let estrogen_target = self.hormones.estrogen; // no engine-modeled driver yet; holds baseline
        let progesterone_target = reproduction.fertility_level.clamp(0.0, 1.0);
        let vasopressin_target = (social_cognition.trust_assessment * 0.5
            + (1.0 - needs.hydration) * 0.5)
            .clamp(0.0, 1.0);
        let adrenaline_target =
            (emotion.current.fear * 0.6 + observation.hazard_index * 0.4).clamp(0.0, 1.0);
        let melatonin_target = (1.0 - observation.daylight_fraction).clamp(0.0, 1.0);

        let neurotransmitters = NeurotransmitterLevels {
            dopamine: lerp(self.neurotransmitters.dopamine, dopamine_target, blend),
            serotonin: lerp(self.neurotransmitters.serotonin, serotonin_target, blend),
            norepinephrine: lerp(
                self.neurotransmitters.norepinephrine,
                norepinephrine_target,
                blend,
            ),
            acetylcholine: lerp(
                self.neurotransmitters.acetylcholine,
                acetylcholine_target,
                blend,
            ),
            gaba: lerp(self.neurotransmitters.gaba, gaba_target, blend),
            glutamate: lerp(self.neurotransmitters.glutamate, glutamate_target, blend),
        };

        let hormones = HormoneLevels {
            cortisol: lerp(self.hormones.cortisol, cortisol_target, blend),
            oxytocin: lerp(self.hormones.oxytocin, oxytocin_target, blend),
            testosterone: lerp(self.hormones.testosterone, testosterone_target, blend),
            estrogen: lerp(self.hormones.estrogen, estrogen_target, blend),
            progesterone: lerp(self.hormones.progesterone, progesterone_target, blend),
            vasopressin: lerp(self.hormones.vasopressin, vasopressin_target, blend),
            adrenaline: lerp(self.hormones.adrenaline, adrenaline_target, blend),
            melatonin: lerp(self.hormones.melatonin, melatonin_target, blend),
        };

        let modulation_effects = Self::compute_modulation(&neurotransmitters, &hormones);

        Self {
            neurotransmitters,
            hormones,
            modulation_effects,
        }
    }

    fn compute_modulation(n: &NeurotransmitterLevels, h: &HormoneLevels) -> ModulationEffects {
        let current_brain_state = if h.adrenaline > 0.7 || h.cortisol > 0.7 {
            BrainState::Stressed
        } else if n.norepinephrine + n.dopamine > 1.3 {
            BrainState::Excited
        } else if n.acetylcholine > 0.6 && n.norepinephrine > 0.3 {
            BrainState::Focused
        } else if n.gaba > 0.6 && h.cortisol < 0.3 {
            BrainState::Relaxed
        } else if (n.norepinephrine < 0.2 && n.acetylcholine < 0.3) || h.melatonin > 0.7 {
            BrainState::Fatigued
        } else {
            BrainState::Resting
        };

        ModulationEffects {
            current_brain_state,
            learning_rate_multiplier: (n.dopamine * 2.0).clamp(0.0, 3.0),
            emotional_bias: (n.dopamine - h.cortisol).clamp(-1.0, 1.0),
            cognitive_load_capacity: (n.acetylcholine * (1.0 - h.cortisol) * (1.0 - h.melatonin))
                .clamp(0.0, 1.0),
            decision_threshold: n.serotonin.clamp(0.0, 1.0),
        }
    }

    fn defaults() -> Self {
        let neurotransmitters = NeurotransmitterLevels {
            dopamine: 0.5,
            serotonin: 0.5,
            norepinephrine: 0.3,
            acetylcholine: 0.5,
            gaba: 0.5,
            glutamate: 0.4,
        };
        let hormones = HormoneLevels {
            cortisol: 0.3,
            oxytocin: 0.3,
            testosterone: 0.4,
            estrogen: 0.4,
            progesterone: 0.4,
            vasopressin: 0.4,
            adrenaline: 0.3,
            melatonin: 0.3,
        };
        let modulation_effects = Self::compute_modulation(&neurotransmitters, &hormones);
        Self {
            neurotransmitters,
            hormones,
            modulation_effects,
        }
    }
}

fn lerp(a: f64, b: f64, t: f64) -> f64 {
    (a + (b - a) * t.clamp(0.0, 1.0)).clamp(0.0, 1.0)
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
    use crate::humans::attention::AttentionSnapshot;
    use crate::humans::dark_triad::DarkTriadSnapshot;
    use crate::humans::emotion::EmotionSnapshot;
    use crate::humans::immune::ImmuneSnapshot;
    use crate::humans::needs::NeedsSnapshot;
    use crate::humans::social_cognition::SocialCognitionSnapshot;
    use mk_core::human::{HumanId, HumanSchema};

    fn profile() -> HumanProfile {
        let schema = HumanSchema::canonical_minimal("neurochem_test");
        HumanProfile::from_canonical_schema(HumanId::new(1), schema)
    }

    #[test]
    fn from_profile_uses_sane_defaults() {
        let snapshot = NeurochemistrySnapshot::from_profile(&profile());
        assert!(snapshot.neurotransmitters.dopamine > 0.0);
        assert!(snapshot.hormones.cortisol > 0.0);
    }

    #[test]
    fn high_stress_needs_raise_cortisol_and_trigger_stressed_state() {
        let snapshot = NeurochemistrySnapshot::from_profile(&profile());
        let emotion = EmotionSnapshot::from_profile(&profile());
        let attention = AttentionSnapshot::from_profile(&profile());
        let social = SocialCognitionSnapshot::from_profile(&profile());
        let dark_triad = DarkTriadSnapshot::from_profile(&profile());

        let mut stressed_needs = NeedsSnapshot::from_profile(&profile());
        stressed_needs.fatigue = 1.0;
        let mut stressed_immune = ImmuneSnapshot::from_profile(&profile());
        stressed_immune.system_stress = 1.0;

        let reproduction =
            crate::humans::reproduction::ReproductiveSystemSnapshot::from_profile(&profile());
        let observation = crate::agents::AgentWorldObservation::default();

        let mut stepped = snapshot.clone();
        for _ in 0..10 {
            stepped = stepped.step(
                &emotion,
                &stressed_needs,
                &stressed_immune,
                &attention,
                &social,
                &dark_triad,
                &reproduction,
                &observation,
                1.0,
            );
        }

        assert!(stepped.hormones.cortisol > snapshot.hormones.cortisol);
        assert!(matches!(
            stepped.modulation_effects.current_brain_state,
            BrainState::Stressed
        ));
    }

    #[test]
    fn dopamine_drives_learning_rate_multiplier() {
        let snapshot = NeurochemistrySnapshot::from_profile(&profile());
        let needs = NeedsSnapshot::from_profile(&profile());
        let immune = ImmuneSnapshot::from_profile(&profile());
        let attention = AttentionSnapshot::from_profile(&profile());
        let social = SocialCognitionSnapshot::from_profile(&profile());
        let dark_triad = DarkTriadSnapshot::from_profile(&profile());

        let mut joyful_emotion = EmotionSnapshot::from_profile(&profile());
        joyful_emotion.current.joy = 1.0;
        joyful_emotion.current.triumph = 1.0;

        let reproduction =
            crate::humans::reproduction::ReproductiveSystemSnapshot::from_profile(&profile());
        let observation = crate::agents::AgentWorldObservation::default();

        let mut stepped = snapshot.clone();
        for _ in 0..10 {
            stepped = stepped.step(
                &joyful_emotion,
                &needs,
                &immune,
                &attention,
                &social,
                &dark_triad,
                &reproduction,
                &observation,
                1.0,
            );
        }

        assert!(
            stepped.modulation_effects.learning_rate_multiplier
                > snapshot.modulation_effects.learning_rate_multiplier
        );
    }
}
