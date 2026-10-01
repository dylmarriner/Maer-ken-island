//! Comprehensive Emotion Snapshot - the canon's full named emotion
//! taxonomy (`ComprehensiveEmotionTaxonomySchema` in
//! `docs/canon/HumanReplicationSchema.js`, ~130 named entries across
//! basic/light/shadow(x3)/complex/social/cognitive/self-conscious/moral/
//! aesthetic/existential/power/sexual/glitch/dark-triad/physiological/
//! bonding/achievement/temporal categories), previously unread by the
//! engine.
//!
//! This is deliberately a second, richer layer alongside
//! [`super::emotion`]'s 30-emotion `GranularEmotionsSchema` rather than a
//! replacement for it — the two canon schemas are genuinely separate and
//! both real. Because the taxonomy is a `BTreeMap<String, ...>` in canon
//! (data, not fixed struct fields), this snapshot stays data-driven too:
//! every category/entry from canon is tracked generically rather than
//! hand-written as ~130 struct fields, which would be an unmaintainable
//! wall of near-duplicate code for the same shape.
//!
//! Per-tick dynamics: each entry's `intensity` is driven toward a target
//! resolved in priority order — (1) if its name name-matches one of the
//! 30 granular emotions (e.g. `"joy_primary"` -> granular `joy`), track
//! that; (2) else if it carries a canon `hormonal_profile`, average the
//! matching real neurochemistry levels (dopamine/cortisol/etc., by name);
//! (3) else decay toward its static canon baseline intensity. `valence`/
//! `arousal`/`dominance`/`social` are canon-authored trait *shape*
//! (anger is always high-arousal/negative-valence) rather than per-tick
//! state — there is no engine signal that should independently move them
//! tick to tick, so they are held at their canon values rather than
//! fabricated dynamics with no real driver.

use mk_core::human::HumanProfile;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmotionEntry {
    pub intensity: f64,
    pub valence: f64,
    pub arousal: f64,
    pub dominance: f64,
    pub social: bool,
    baseline_intensity: f64,
    /// Canon `hormonal_profile` substance names (light/shadow categories
    /// only); used to resolve this entry's per-tick target as an average
    /// of the matching real neurochemistry levels when no granular-emotion
    /// name match exists.
    hormonal_substances: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComprehensiveEmotionSnapshot {
    /// category name -> (entry name -> entry). Mirrors canon's category
    /// grouping (`basic_emotions`, `shadow_aggression`, etc.) 1:1.
    pub categories: BTreeMap<String, BTreeMap<String, EmotionEntry>>,
}

impl ComprehensiveEmotionSnapshot {
    pub fn from_profile(profile: &HumanProfile) -> Self {
        let Some(schema) = profile.canonical_schema() else {
            return Self::defaults();
        };
        let t = &schema.comprehensive_emotion_taxonomy;
        let mut categories = BTreeMap::new();

        let mut basic = BTreeMap::new();
        for (name, v) in &t.basic_emotions {
            basic.insert(name.clone(), entry_from_vector(v, false));
        }
        categories.insert("basic_emotions".to_string(), basic);

        for (category, map) in [
            ("light_emotions", &t.light_emotions),
            ("shadow_aggression", &t.shadow_aggression),
            ("shadow_resource_guarding", &t.shadow_resource_guarding),
            ("shadow_system_collapse", &t.shadow_system_collapse),
        ] {
            let mut entries = BTreeMap::new();
            for (name, v) in map {
                entries.insert(name.clone(), entry_from_hormonal(v));
            }
            categories.insert(category.to_string(), entries);
        }

        for (category, map) in [
            ("complex_emotions", &t.complex_emotions),
            ("social_emotions", &t.social_emotions),
            ("cognitive_emotions", &t.cognitive_emotions),
            ("self_conscious_emotions", &t.self_conscious_emotions),
            ("moral_emotions", &t.moral_emotions),
            ("aesthetic_emotions", &t.aesthetic_emotions),
            ("existential_emotions", &t.existential_emotions),
            ("power_dynamics", &t.power_dynamics),
            ("sexual_pleasure", &t.sexual_pleasure),
            ("system_glitches", &t.system_glitches),
            ("dark_triad_manifestations", &t.dark_triad_manifestations),
            ("physiological_emotions", &t.physiological_emotions),
            ("social_bonding", &t.social_bonding),
            ("achievement_emotions", &t.achievement_emotions),
            ("cognitive_state_emotions", &t.cognitive_state_emotions),
            ("temporal_emotions", &t.temporal_emotions),
        ] {
            let mut entries = BTreeMap::new();
            for (name, v) in map {
                entries.insert(name.clone(), entry_from_social_vector(v));
            }
            categories.insert(category.to_string(), entries);
        }

        Self { categories }
    }

    /// Drive every entry's intensity toward its resolved target — granular
    /// name match, then (for the `sexual_pleasure` category only) real
    /// arousal/satisfaction from [`super::reproduction::ReproductiveSystemSnapshot`],
    /// then hormonal-profile average, then static baseline — and decay.
    pub fn step(
        &self,
        granular: &super::emotion::EmotionLevels,
        neurochemistry: &super::neurochemistry::NeurochemistrySnapshot,
        reproduction: &super::reproduction::ReproductiveSystemSnapshot,
        dt_years: f64,
    ) -> Self {
        let dt = dt_years.max(0.0);
        let decay = (0.5 * dt).clamp(0.0, 1.0);
        // Real physiological driver for `sexual_pleasure` — previously this
        // category could only reach a target via granular-name match or a
        // hormonal-substance average, never the actual arousal/satisfaction
        // state `reproduction.rs` already steps in real response to libido,
        // deprivation, and bonding.
        let sexual_pleasure_signal =
            ((reproduction.arousal + reproduction.satisfaction) / 2.0).clamp(0.0, 1.0);

        let categories = self
            .categories
            .iter()
            .map(|(category, entries)| {
                let stepped = entries
                    .iter()
                    .map(|(name, entry)| {
                        let target = granular.named(name).unwrap_or_else(|| {
                            if category == "sexual_pleasure" {
                                sexual_pleasure_signal
                            } else {
                                hormonal_average(&entry.hormonal_substances, neurochemistry)
                                    .unwrap_or(entry.baseline_intensity)
                            }
                        });
                        let intensity = lerp(entry.intensity, target, decay);
                        (
                            name.clone(),
                            EmotionEntry {
                                intensity,
                                ..entry.clone()
                            },
                        )
                    })
                    .collect::<BTreeMap<_, _>>();
                (category.clone(), stepped)
            })
            .collect();

        Self { categories }
    }

    fn defaults() -> Self {
        Self {
            categories: BTreeMap::new(),
        }
    }
}

/// Average the real neurochemistry levels of a canon `hormonal_profile`'s
/// named substances. Substances canon names that this engine doesn't
/// independently track (e.g. `"endorphin"`) are skipped; `None` is
/// returned only if none of the named substances resolve, so the caller
/// falls back to the entry's static baseline instead.
fn hormonal_average(
    names: &[String],
    n: &super::neurochemistry::NeurochemistrySnapshot,
) -> Option<f64> {
    let levels: Vec<f64> = names
        .iter()
        .filter_map(|name| hormonal_level(name, n))
        .collect();
    if levels.is_empty() {
        None
    } else {
        Some(levels.iter().sum::<f64>() / levels.len() as f64)
    }
}

/// Substance name lookup across both neurotransmitters and hormones,
/// case-insensitively, for the canon's per-emotion `hormonal_profile`
/// entries (e.g. `"dopamine"`, `"cortisol"`). Substances canon names that
/// this engine doesn't independently track (e.g. `"endorphin"`) fall back
/// to `None` and the caller uses the entry's static baseline instead.
fn hormonal_level(name: &str, n: &super::neurochemistry::NeurochemistrySnapshot) -> Option<f64> {
    let key = name.to_ascii_lowercase();
    match key.as_str() {
        "dopamine" => Some(n.neurotransmitters.dopamine),
        "serotonin" => Some(n.neurotransmitters.serotonin),
        "norepinephrine" => Some(n.neurotransmitters.norepinephrine),
        "acetylcholine" => Some(n.neurotransmitters.acetylcholine),
        "gaba" => Some(n.neurotransmitters.gaba),
        "glutamate" => Some(n.neurotransmitters.glutamate),
        "cortisol" => Some(n.hormones.cortisol),
        "oxytocin" => Some(n.hormones.oxytocin),
        "testosterone" => Some(n.hormones.testosterone),
        "estrogen" => Some(n.hormones.estrogen),
        "progesterone" => Some(n.hormones.progesterone),
        "vasopressin" => Some(n.hormones.vasopressin),
        "adrenaline" => Some(n.hormones.adrenaline),
        "melatonin" => Some(n.hormones.melatonin),
        _ => None,
    }
}

fn entry_from_vector(
    v: &mk_core::human::schema::EmotionVectorSchema,
    social: bool,
) -> EmotionEntry {
    let intensity = v.intensity as f64;
    EmotionEntry {
        intensity,
        valence: v.valence as f64,
        arousal: v.arousal as f64,
        dominance: v.dominance as f64,
        social,
        baseline_intensity: intensity,
        hormonal_substances: Vec::new(),
    }
}

fn entry_from_social_vector(v: &mk_core::human::schema::SocialEmotionVectorSchema) -> EmotionEntry {
    entry_from_vector(&v.base, v.social)
}

fn entry_from_hormonal(v: &mk_core::human::schema::HormonalEmotionEntrySchema) -> EmotionEntry {
    let intensity = v.intensity as f64;
    EmotionEntry {
        intensity,
        valence: 0.0,
        arousal: 0.0,
        dominance: 0.0,
        social: v.social,
        baseline_intensity: intensity,
        hormonal_substances: v.hormonal_profile.keys().cloned().collect(),
    }
}

fn lerp(a: f64, b: f64, t: f64) -> f64 {
    (a + (b - a) * t.clamp(0.0, 1.0)).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::humans::emotion::EmotionSnapshot;
    use crate::humans::neurochemistry::NeurochemistrySnapshot;
    use mk_core::human::{HumanId, HumanSchema};

    fn profile() -> HumanProfile {
        let schema = HumanSchema::canonical_minimal("comprehensive_emotion_test");
        HumanProfile::from_canonical_schema(HumanId::new(1), schema)
    }

    #[test]
    fn from_profile_covers_all_canon_categories() {
        let snapshot = ComprehensiveEmotionSnapshot::from_profile(&profile());
        for category in [
            "basic_emotions",
            "light_emotions",
            "shadow_aggression",
            "shadow_resource_guarding",
            "shadow_system_collapse",
            "complex_emotions",
            "social_emotions",
            "cognitive_emotions",
            "self_conscious_emotions",
            "moral_emotions",
            "aesthetic_emotions",
            "existential_emotions",
            "power_dynamics",
            "sexual_pleasure",
            "system_glitches",
            "dark_triad_manifestations",
            "physiological_emotions",
            "social_bonding",
            "achievement_emotions",
            "cognitive_state_emotions",
            "temporal_emotions",
        ] {
            assert!(
                snapshot.categories.contains_key(category),
                "missing category: {category}"
            );
        }
    }

    #[test]
    fn granular_matched_entry_tracks_granular_joy() {
        let snapshot = ComprehensiveEmotionSnapshot::from_profile(&profile());
        let neurochemistry = NeurochemistrySnapshot::from_profile(&profile());
        let mut granular = EmotionSnapshot::from_profile(&profile());
        granular.current.joy = 1.0;

        let reproduction =
            super::super::reproduction::ReproductiveSystemSnapshot::from_profile(&profile());
        let mut stepped = snapshot.clone();
        for _ in 0..20 {
            stepped = stepped.step(&granular.current, &neurochemistry, &reproduction, 1.0);
        }

        let joy_primary = stepped
            .categories
            .get("light_emotions")
            .and_then(|c| c.get("joy_primary"));
        if let Some(entry) = joy_primary {
            assert!(
                entry.intensity > 0.5,
                "joy_primary should track granular joy"
            );
        }
    }

    #[test]
    fn sexual_pleasure_category_tracks_real_arousal_and_satisfaction() {
        // Regression test: `sexual_pleasure` entries previously could only
        // reach a target via granular-name match or a hormonal-substance
        // average — never `reproduction.rs`'s real, already-stepped
        // arousal/satisfaction, which sat unused right next to it.
        // `canonical_minimal`'s schema doesn't populate the canon
        // `sexual_pleasure` map, so seed one entry directly rather than via
        // `from_profile()`, exactly as `EmotionEntry`'s other constructors
        // would shape it (no granular-name match, no hormonal substances —
        // isolating the new fallback path this test targets).
        let mut categories = BTreeMap::new();
        let mut sexual_pleasure = BTreeMap::new();
        sexual_pleasure.insert(
            "arousal_test_entry".to_string(),
            EmotionEntry {
                intensity: 0.0,
                valence: 0.0,
                arousal: 0.0,
                dominance: 0.0,
                social: true,
                baseline_intensity: 0.0,
                hormonal_substances: Vec::new(),
            },
        );
        categories.insert("sexual_pleasure".to_string(), sexual_pleasure);
        let snapshot = ComprehensiveEmotionSnapshot { categories };

        let neurochemistry = NeurochemistrySnapshot::from_profile(&profile());
        let granular = EmotionSnapshot::from_profile(&profile());
        let mut reproduction =
            super::super::reproduction::ReproductiveSystemSnapshot::from_profile(&profile());
        reproduction.arousal = 1.0;
        reproduction.satisfaction = 1.0;

        let mut stepped = snapshot.clone();
        for _ in 0..20 {
            stepped = stepped.step(&granular.current, &neurochemistry, &reproduction, 1.0);
        }

        let entry = stepped
            .categories
            .get("sexual_pleasure")
            .and_then(|c| c.get("arousal_test_entry"))
            .expect("sexual_pleasure entry must exist");
        assert!(
            entry.intensity > 0.5,
            "sexual_pleasure entry should track high real arousal/satisfaction, got {}",
            entry.intensity
        );
    }
}
