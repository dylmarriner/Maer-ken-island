//! Emotion Snapshot - dispositional baselines and real-time levels for the
//! canon's 30-emotion granular vector.
//!
//! Surfaces `docs/canon/HumanReplicationSchema.js`'s `GranularEmotionsSchema`
//! (ported in `mk_core::human::schema`, previously unread by the engine) as
//! the human's dispositional baseline for each emotion, and steps a
//! real-time `current` vector toward that baseline each tick. All 31
//! entries (the 30-entry granular vector plus `contentment`) are perturbed
//! by `needs`/`social_cognition`/`creativity`/`attention`/`sensory` (pain)/
//! `immune` (sickness) before decaying back toward baseline — grouped
//! below by the real signal each one tracks: physiological
//! satisfaction/deprivation (joy/sadness/fear/disgust/contentment/anger/
//! despair/disappointment), social trust and cooperation (love/hate/
//! resentment/jealousy/envy/contempt/schadenfreude/hope/gratitude/
//! admiration/pity), social anxiety and conflict (shame/guilt/
//! embarrassment/humiliation), and attention/creativity flow-state
//! (curiosity/boredom/pride/triumph/awe/nostalgia/surprise). It makes no
//! clinical or neuroscience claim about how emotions arise. The canon's `ComprehensiveEmotionTaxonomySchema` hormonal/social layers are
//! bridged separately in [`super::comprehensive_emotion`].
//!
//! `mk_core`'s `GranularEmotionsSchema` has 30 named f32 fields (no map),
//! so this snapshot mirrors that shape 1:1 rather than using a BTreeMap —
//! keeps trait access allocation-free and matches the schema's intent that
//! every human has all 30 emotions present, just at differing intensities.

use mk_core::human::HumanProfile;
use serde::{Deserialize, Serialize};

impl EmotionLevels {
    /// All 31 named emotions (the 30-entry granular vector plus
    /// `contentment`) as `(name, value)` pairs, in declaration order. Single
    /// source of truth backing [`Self::named`], [`Self::dominant`], and
    /// [`Self::top_n`] so the name list is written exactly once.
    fn all_pairs(&self) -> [(&'static str, f64); 31] {
        [
            ("joy", self.joy),
            ("sadness", self.sadness),
            ("anger", self.anger),
            ("fear", self.fear),
            ("disgust", self.disgust),
            ("surprise", self.surprise),
            ("love", self.love),
            ("hate", self.hate),
            ("pride", self.pride),
            ("shame", self.shame),
            ("guilt", self.guilt),
            ("jealousy", self.jealousy),
            ("envy", self.envy),
            ("contempt", self.contempt),
            ("awe", self.awe),
            ("nostalgia", self.nostalgia),
            ("hope", self.hope),
            ("despair", self.despair),
            ("curiosity", self.curiosity),
            ("boredom", self.boredom),
            ("relief", self.relief),
            ("disappointment", self.disappointment),
            ("gratitude", self.gratitude),
            ("resentment", self.resentment),
            ("admiration", self.admiration),
            ("pity", self.pity),
            ("schadenfreude", self.schadenfreude),
            ("embarrassment", self.embarrassment),
            ("triumph", self.triumph),
            ("humiliation", self.humiliation),
            ("contentment", self.contentment),
        ]
    }

    /// Look up a granular emotion by name (case-insensitive substring
    /// match), for [`super::comprehensive_emotion`]'s name-based bridging
    /// between the flat 30-emotion vector and the canon's much larger
    /// named taxonomy (e.g. `"joy_primary"` -> [`Self::joy`]).
    pub fn named(&self, name: &str) -> Option<f64> {
        let n = name.to_ascii_lowercase();
        self.all_pairs()
            .iter()
            .filter(|(key, _)| n.contains(key) && *key != "contentment")
            .max_by_key(|(key, _)| key.len())
            .map(|(_, v)| *v)
            .or_else(|| n.contains("content").then_some(self.contentment))
    }

    /// The single highest-magnitude emotion right now (by absolute value,
    /// so a strongly negative emotion like despair or humiliation can be
    /// "dominant" too), for [`super::thought`]/[`super::dialogue`]'s
    /// template selection. Ties resolve to the last tied entry in
    /// declaration order (`Iterator::max_by`'s documented tie-break), so
    /// the result is deterministic.
    pub fn dominant(&self) -> (&'static str, f64) {
        self.all_pairs()
            .into_iter()
            .max_by(|(_, a), (_, b)| {
                a.abs()
                    .partial_cmp(&b.abs())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .expect("all_pairs is a non-empty fixed-size array")
    }

    /// The `n` highest-magnitude emotions right now (by absolute value),
    /// most intense first, for dashboard projections that want more than
    /// just [`Self::dominant`]'s single top entry. Ties break by
    /// declaration order (stable sort). `n` is clamped to the number of
    /// tracked emotions (31).
    pub fn top_n(&self, n: usize) -> Vec<(&'static str, f64)> {
        let mut pairs = self.all_pairs().to_vec();
        pairs.sort_by(|(_, a), (_, b)| {
            b.abs()
                .partial_cmp(&a.abs())
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        pairs.truncate(n);
        pairs
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmotionLevels {
    pub joy: f64,
    pub sadness: f64,
    pub anger: f64,
    pub fear: f64,
    pub disgust: f64,
    pub surprise: f64,
    pub love: f64,
    pub hate: f64,
    pub pride: f64,
    pub shame: f64,
    pub guilt: f64,
    pub jealousy: f64,
    pub envy: f64,
    pub contempt: f64,
    pub awe: f64,
    pub nostalgia: f64,
    pub hope: f64,
    pub despair: f64,
    pub curiosity: f64,
    pub boredom: f64,
    pub relief: f64,
    pub disappointment: f64,
    pub gratitude: f64,
    pub resentment: f64,
    pub admiration: f64,
    pub pity: f64,
    pub schadenfreude: f64,
    pub embarrassment: f64,
    pub triumph: f64,
    pub humiliation: f64,
    pub contentment: f64,
}

impl EmotionLevels {
    fn zero() -> Self {
        Self {
            joy: 0.0,
            sadness: 0.0,
            anger: 0.0,
            fear: 0.0,
            disgust: 0.0,
            surprise: 0.0,
            love: 0.0,
            hate: 0.0,
            pride: 0.0,
            shame: 0.0,
            guilt: 0.0,
            jealousy: 0.0,
            envy: 0.0,
            contempt: 0.0,
            awe: 0.0,
            nostalgia: 0.0,
            hope: 0.0,
            despair: 0.0,
            curiosity: 0.0,
            boredom: 0.0,
            relief: 0.0,
            disappointment: 0.0,
            gratitude: 0.0,
            resentment: 0.0,
            admiration: 0.0,
            pity: 0.0,
            schadenfreude: 0.0,
            embarrassment: 0.0,
            triumph: 0.0,
            humiliation: 0.0,
            contentment: 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmotionSnapshot {
    /// Dispositional baseline every emotion decays toward (from canon
    /// schema, static per human).
    pub baseline: EmotionLevels,
    /// Real-time levels, stepped each tick.
    pub current: EmotionLevels,
}

impl EmotionSnapshot {
    /// Apply a single life event (e.g. the death of a close relative) as an
    /// instantaneous shift in `current`, independent of the tick's regular
    /// physiological/social stepping. Fades back toward `baseline` on the
    /// next `step()` call the same way any other perturbation does — this
    /// adds an input, not a new decay mechanism. See `super::mod::HumanSystem`'s
    /// grief-propagation call site.
    pub fn apply_grief_shock(&mut self, intensity: f64) {
        let intensity = intensity.clamp(0.0, 1.0);
        self.current.sadness = (self.current.sadness + intensity).clamp(0.0, 1.0);
        self.current.despair = (self.current.despair + intensity * 0.5).clamp(0.0, 1.0);
    }

    /// Overall emotional activation right now (0-1): mean absolute deviation
    /// of `current` from `baseline` across all 30 emotions. Used by systems
    /// that need a single "how activated is this human emotionally" scalar
    /// (e.g. [`super::advanced_memory`]'s emotional-memory-enhancement
    /// coupling) without picking any one emotion as privileged.
    pub fn overall_intensity(&self) -> f64 {
        let b = &self.baseline;
        let c = &self.current;
        let deviations = [
            (c.joy - b.joy).abs(),
            (c.sadness - b.sadness).abs(),
            (c.anger - b.anger).abs(),
            (c.fear - b.fear).abs(),
            (c.disgust - b.disgust).abs(),
            (c.surprise - b.surprise).abs(),
            (c.love - b.love).abs(),
            (c.hate - b.hate).abs(),
            (c.pride - b.pride).abs(),
            (c.shame - b.shame).abs(),
            (c.guilt - b.guilt).abs(),
            (c.jealousy - b.jealousy).abs(),
            (c.envy - b.envy).abs(),
            (c.contempt - b.contempt).abs(),
            (c.awe - b.awe).abs(),
            (c.nostalgia - b.nostalgia).abs(),
            (c.hope - b.hope).abs(),
            (c.despair - b.despair).abs(),
            (c.curiosity - b.curiosity).abs(),
            (c.boredom - b.boredom).abs(),
            (c.relief - b.relief).abs(),
            (c.disappointment - b.disappointment).abs(),
            (c.gratitude - b.gratitude).abs(),
            (c.resentment - b.resentment).abs(),
            (c.admiration - b.admiration).abs(),
            (c.pity - b.pity).abs(),
            (c.schadenfreude - b.schadenfreude).abs(),
            (c.embarrassment - b.embarrassment).abs(),
            (c.triumph - b.triumph).abs(),
            (c.humiliation - b.humiliation).abs(),
            (c.contentment - b.contentment).abs(),
        ];
        (deviations.iter().sum::<f64>() / deviations.len() as f64).clamp(0.0, 1.0)
    }

    pub fn from_profile(profile: &HumanProfile) -> Self {
        let Some(schema) = profile.canonical_schema() else {
            return Self::defaults();
        };
        let g = &schema.granular_emotions;

        let baseline = EmotionLevels {
            joy: g.joy as f64,
            sadness: g.sadness as f64,
            anger: g.anger as f64,
            fear: g.fear as f64,
            disgust: g.disgust as f64,
            surprise: g.surprise as f64,
            love: g.love as f64,
            hate: g.hate as f64,
            pride: g.pride as f64,
            shame: g.shame as f64,
            guilt: g.guilt as f64,
            jealousy: g.jealousy as f64,
            envy: g.envy as f64,
            contempt: g.contempt as f64,
            awe: g.awe as f64,
            nostalgia: g.nostalgia as f64,
            hope: g.hope as f64,
            despair: g.despair as f64,
            curiosity: g.curiosity as f64,
            boredom: g.boredom as f64,
            relief: g.relief as f64,
            disappointment: g.disappointment as f64,
            gratitude: g.gratitude as f64,
            resentment: g.resentment as f64,
            admiration: g.admiration as f64,
            pity: g.pity as f64,
            schadenfreude: g.schadenfreude as f64,
            embarrassment: g.embarrassment as f64,
            triumph: g.triumph as f64,
            humiliation: g.humiliation as f64,
            contentment: g.contentment as f64,
        };

        Self {
            current: baseline.clone(),
            baseline,
        }
    }

    /// Perturb the physiologically/socially-grounded emotions from real
    /// engine state, then decay every emotion back toward its baseline.
    #[allow(clippy::too_many_arguments)]
    pub fn step(
        &self,
        needs: &super::needs::NeedsSnapshot,
        social_cognition: &super::social_cognition::SocialCognitionSnapshot,
        creativity: &super::creativity::CreativitySnapshot,
        attention: &super::attention::AttentionSnapshot,
        sensory: &super::sensory::SensorySnapshot,
        immune: &super::immune::ImmuneSnapshot,
        dt_years: f64,
    ) -> Self {
        let dt = dt_years.max(0.0);
        let decay = (0.5 * dt).clamp(0.0, 1.0);

        // Needs satisfaction drives joy/contentment up and sadness/despair
        // down; deprivation does the reverse. Physical pain and active
        // sickness compound deprivation — a hurting, sick body feels worse
        // regardless of whether hunger/thirst/rest are otherwise met.
        let pain = sensory.overall_pain_level.clamp(0.0, 1.0);
        let sickness = (immune.system_stress + (immune.active_pathogen_count.min(5) as f64) * 0.1)
            .clamp(0.0, 1.0);
        let satisfaction =
            ((1.0 - needs.fatigue) * (1.0 - pain * 0.5) * (1.0 - sickness * 0.3)).clamp(0.0, 1.0);
        let deprivation = (needs.fatigue + pain * 0.4 + sickness * 0.3).clamp(0.0, 1.0);

        let joy_target = (self.baseline.joy + satisfaction * 0.3).clamp(0.0, 1.0);
        let sadness_target = (self.baseline.sadness + deprivation * 0.3).clamp(0.0, 1.0);
        let fear_target = (self.baseline.fear + deprivation * 0.2 + pain * 0.2).clamp(0.0, 1.0);
        let disgust_target = (self.baseline.disgust + sickness * 0.3).clamp(0.0, 1.0);
        let contentment_target = (self.baseline.contentment + satisfaction * 0.3).clamp(0.0, 1.0);

        // Social trust/cooperation feeds love; low trust feeds jealousy.
        let love_target =
            (self.baseline.love + social_cognition.trust_assessment * 0.2).clamp(0.0, 1.0);

        // Novelty-seeking/insight generation feeds curiosity; attention
        // fatigue and lack of flow feed boredom.
        let curiosity_target = (self.baseline.curiosity
            + creativity.effective_insight_generation * 0.2)
            .clamp(0.0, 1.0);
        let boredom_target = (self.baseline.boredom
            + (1.0 - attention.flow) * attention.attention_fatigue * 0.2)
            .clamp(0.0, 1.0);

        // Frustration-linked: unmet needs and physical suffering read as
        // anger/despair/disappointment, same deprivation signal that
        // already drives sadness/fear above.
        let anger_target = (self.baseline.anger + deprivation * 0.25 + pain * 0.15).clamp(0.0, 1.0);
        let despair_target =
            (self.baseline.despair + deprivation * 0.25 + sickness * 0.15).clamp(0.0, 1.0);
        let disappointment_target = (self.baseline.disappointment
            + deprivation * 0.15
            + (1.0 - social_cognition.trust_assessment) * 0.1)
            .clamp(0.0, 1.0);

        // Social-trust-linked: distrust/low cooperation read as
        // hate/resentment/jealousy/envy/contempt/schadenfreude; trust and
        // cooperation read as the corresponding warm emotions. `hate` and
        // `resentment` are load-bearing for `dark_triad.rs`'s
        // `active_malice`/`vengeance_drive`.
        let hate_target = (self.baseline.hate
            + (1.0 - social_cognition.trust_assessment) * 0.2
            + (1.0 - social_cognition.cooperation_tendency) * 0.1)
            .clamp(0.0, 1.0);
        let resentment_target = (self.baseline.resentment
            + deprivation * 0.15
            + (1.0 - social_cognition.trust_assessment) * 0.15)
            .clamp(0.0, 1.0);
        let jealousy_target = (self.baseline.jealousy
            + deprivation * 0.15
            + (1.0 - social_cognition.trust_assessment) * 0.1)
            .clamp(0.0, 1.0);
        let envy_target = (self.baseline.envy + deprivation * 0.2).clamp(0.0, 1.0);
        let contempt_target = (self.baseline.contempt
            + (1.0 - social_cognition.cooperation_tendency) * 0.15
            + (1.0 - social_cognition.empathy_level) * 0.15)
            .clamp(0.0, 1.0);
        let schadenfreude_target = (self.baseline.schadenfreude
            + (1.0 - social_cognition.empathy_level) * 0.15
            + (1.0 - social_cognition.cooperation_tendency) * 0.1)
            .clamp(0.0, 1.0);
        let hope_target =
            (self.baseline.hope + satisfaction * 0.2 + social_cognition.trust_assessment * 0.1)
                .clamp(0.0, 1.0);
        let relief_target = (self.baseline.relief + satisfaction * 0.15).clamp(0.0, 1.0);
        let gratitude_target = (self.baseline.gratitude
            + social_cognition.trust_assessment * 0.15
            + social_cognition.cooperation_tendency * 0.1)
            .clamp(0.0, 1.0);
        let admiration_target = (self.baseline.admiration
            + social_cognition.empathy_level * 0.15
            + social_cognition.emotion_recognition * 0.1)
            .clamp(0.0, 1.0);
        let pity_target = (self.baseline.pity
            + social_cognition.empathy_level * 0.2 * (pain + sickness).min(1.0))
        .clamp(0.0, 1.0);

        // Social-anxiety/conflict-linked self-conscious emotions.
        let shame_target =
            (self.baseline.shame + social_cognition.social_anxiety * 0.2).clamp(0.0, 1.0);
        let guilt_target = (self.baseline.guilt
            + (1.0 - social_cognition.conflict_resolution) * 0.15)
            .clamp(0.0, 1.0);
        let embarrassment_target =
            (self.baseline.embarrassment + social_cognition.social_anxiety * 0.3).clamp(0.0, 1.0);
        let humiliation_target =
            (self.baseline.humiliation + social_cognition.social_anxiety * 0.2 + pain * 0.1)
                .clamp(0.0, 1.0);

        // Attention/creativity flow-state-linked: mastery and engaged
        // focus read as pride/triumph/awe; low cognitive load reads as
        // reflective nostalgia, high load as surprise.
        let pride_target = (self.baseline.pride
            + creativity.effective_divergent_thinking * 0.1
            + attention.focus_level * 0.1)
            .clamp(0.0, 1.0);
        let triumph_target = (self.baseline.triumph
            + attention.flow * 0.15
            + creativity.effective_insight_generation * 0.1)
            .clamp(0.0, 1.0);
        let awe_target =
            (self.baseline.awe + creativity.aesthetic_sensitivity * 0.2 + attention.flow * 0.1)
                .clamp(0.0, 1.0);
        let nostalgia_target =
            (self.baseline.nostalgia + (1.0 - attention.cognitive_load) * 0.1).clamp(0.0, 1.0);
        let surprise_target =
            (self.baseline.surprise + attention.cognitive_load * 0.15).clamp(0.0, 1.0);

        let current = EmotionLevels {
            joy: lerp(self.current.joy, joy_target, decay),
            sadness: lerp(self.current.sadness, sadness_target, decay),
            fear: lerp(self.current.fear, fear_target, decay),
            contentment: lerp(self.current.contentment, contentment_target, decay),
            love: lerp(self.current.love, love_target, decay),
            curiosity: lerp(self.current.curiosity, curiosity_target, decay),
            boredom: lerp(self.current.boredom, boredom_target, decay),
            anger: lerp(self.current.anger, anger_target, decay),
            disgust: lerp(self.current.disgust, disgust_target, decay),
            surprise: lerp(self.current.surprise, surprise_target, decay),
            hate: lerp(self.current.hate, hate_target, decay),
            pride: lerp(self.current.pride, pride_target, decay),
            shame: lerp(self.current.shame, shame_target, decay),
            guilt: lerp(self.current.guilt, guilt_target, decay),
            jealousy: lerp(self.current.jealousy, jealousy_target, decay),
            envy: lerp(self.current.envy, envy_target, decay),
            contempt: lerp(self.current.contempt, contempt_target, decay),
            awe: lerp(self.current.awe, awe_target, decay),
            nostalgia: lerp(self.current.nostalgia, nostalgia_target, decay),
            hope: lerp(self.current.hope, hope_target, decay),
            despair: lerp(self.current.despair, despair_target, decay),
            relief: lerp(self.current.relief, relief_target, decay),
            disappointment: lerp(self.current.disappointment, disappointment_target, decay),
            gratitude: lerp(self.current.gratitude, gratitude_target, decay),
            resentment: lerp(self.current.resentment, resentment_target, decay),
            admiration: lerp(self.current.admiration, admiration_target, decay),
            pity: lerp(self.current.pity, pity_target, decay),
            schadenfreude: lerp(self.current.schadenfreude, schadenfreude_target, decay),
            embarrassment: lerp(self.current.embarrassment, embarrassment_target, decay),
            triumph: lerp(self.current.triumph, triumph_target, decay),
            humiliation: lerp(self.current.humiliation, humiliation_target, decay),
        };

        Self {
            baseline: self.baseline.clone(),
            current,
        }
    }

    fn defaults() -> Self {
        Self {
            baseline: EmotionLevels::zero(),
            current: EmotionLevels::zero(),
        }
    }
}

fn lerp(a: f64, b: f64, t: f64) -> f64 {
    (a + (b - a) * t.clamp(0.0, 1.0)).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::humans::attention::AttentionSnapshot;
    use crate::humans::creativity::CreativitySnapshot;
    use crate::humans::immune::ImmuneSnapshot;
    use crate::humans::needs::NeedsSnapshot;
    use crate::humans::sensory::SensorySnapshot;
    use crate::humans::social_cognition::SocialCognitionSnapshot;
    use mk_core::human::{HumanId, HumanSchema};

    fn profile() -> HumanProfile {
        let schema = HumanSchema::canonical_minimal("emotion_test");
        HumanProfile::from_canonical_schema(HumanId::new(1), schema)
    }

    #[test]
    fn from_profile_starts_at_baseline() {
        let snapshot = EmotionSnapshot::from_profile(&profile());
        assert_eq!(snapshot.current.joy, snapshot.baseline.joy);
    }

    #[test]
    fn need_deprivation_raises_sadness_and_lowers_joy() {
        let snapshot = EmotionSnapshot::from_profile(&profile());
        let social = SocialCognitionSnapshot::from_profile(&profile());
        let creativity = CreativitySnapshot::from_profile(&profile());
        let attention = AttentionSnapshot::from_profile(&profile());
        let sensory = SensorySnapshot::from_profile(&profile());
        let immune = ImmuneSnapshot::from_profile(&profile());

        let mut deprived_needs = NeedsSnapshot::from_profile(&profile());
        deprived_needs.fatigue = 1.0;
        let satisfied_needs = NeedsSnapshot::from_profile(&profile());

        let mut deprived = snapshot.clone();
        let mut satisfied = snapshot.clone();
        for _ in 0..10 {
            deprived = deprived.step(
                &deprived_needs,
                &social,
                &creativity,
                &attention,
                &sensory,
                &immune,
                1.0,
            );
            satisfied = satisfied.step(
                &satisfied_needs,
                &social,
                &creativity,
                &attention,
                &sensory,
                &immune,
                1.0,
            );
        }

        assert!(deprived.current.sadness >= satisfied.current.sadness);
        assert!(deprived.current.joy <= satisfied.current.joy);
    }

    #[test]
    fn pain_and_sickness_raise_fear_and_disgust() {
        let snapshot = EmotionSnapshot::from_profile(&profile());
        let social = SocialCognitionSnapshot::from_profile(&profile());
        let creativity = CreativitySnapshot::from_profile(&profile());
        let attention = AttentionSnapshot::from_profile(&profile());
        let needs = NeedsSnapshot::from_profile(&profile());

        let mut hurting_sensory = SensorySnapshot::from_profile(&profile());
        hurting_sensory.overall_pain_level = 1.0;
        let healthy_sensory = SensorySnapshot::from_profile(&profile());

        let mut sick_immune = ImmuneSnapshot::from_profile(&profile());
        sick_immune.system_stress = 1.0;
        sick_immune.active_pathogen_count = 5;
        let healthy_immune = ImmuneSnapshot::from_profile(&profile());

        let mut suffering = snapshot.clone();
        let mut healthy = snapshot.clone();
        for _ in 0..10 {
            suffering = suffering.step(
                &needs,
                &social,
                &creativity,
                &attention,
                &hurting_sensory,
                &sick_immune,
                1.0,
            );
            healthy = healthy.step(
                &needs,
                &social,
                &creativity,
                &attention,
                &healthy_sensory,
                &healthy_immune,
                1.0,
            );
        }

        assert!(suffering.current.fear >= healthy.current.fear);
        assert!(suffering.current.disgust >= healthy.current.disgust);
    }

    #[test]
    fn driven_emotions_settle_at_their_real_target_under_neutral_inputs() {
        let mut snapshot = EmotionSnapshot::from_profile(&profile());
        snapshot.current.anger = 1.0;
        let needs = NeedsSnapshot::from_profile(&profile());
        let social = SocialCognitionSnapshot::from_profile(&profile());
        let creativity = CreativitySnapshot::from_profile(&profile());
        let attention = AttentionSnapshot::from_profile(&profile());
        let sensory = SensorySnapshot::from_profile(&profile());
        let immune = ImmuneSnapshot::from_profile(&profile());

        let mut stepped = snapshot.clone();
        for _ in 0..20 {
            stepped = stepped.step(
                &needs,
                &social,
                &creativity,
                &attention,
                &sensory,
                &immune,
                1.0,
            );
        }

        // `anger` is now driven by deprivation/pain (see `step()`), not a
        // pure baseline decay — under default (near-neutral) profile inputs
        // its target sits close to baseline, so 20 ticks should still
        // settle it near baseline within a small tolerance.
        assert!((stepped.current.anger - stepped.baseline.anger).abs() < 0.05);
    }

    #[test]
    fn hostile_social_inputs_raise_hate_and_resentment_above_baseline() {
        let snapshot = EmotionSnapshot::from_profile(&profile());
        let needs = NeedsSnapshot::from_profile(&profile());
        let mut social = SocialCognitionSnapshot::from_profile(&profile());
        social.trust_assessment = 0.0;
        social.cooperation_tendency = 0.0;
        let creativity = CreativitySnapshot::from_profile(&profile());
        let attention = AttentionSnapshot::from_profile(&profile());
        let sensory = SensorySnapshot::from_profile(&profile());
        let immune = ImmuneSnapshot::from_profile(&profile());

        let mut stepped = snapshot.clone();
        for _ in 0..20 {
            stepped = stepped.step(
                &needs,
                &social,
                &creativity,
                &attention,
                &sensory,
                &immune,
                1.0,
            );
        }

        // `dark_triad.rs`'s active_malice/vengeance_drive read hate and
        // resentment as live signals; zero trust/cooperation must move them
        // off baseline for that gating to ever fire.
        assert!(stepped.current.hate > stepped.baseline.hate + 0.05);
        assert!(stepped.current.resentment > stepped.baseline.resentment + 0.05);
    }
}
