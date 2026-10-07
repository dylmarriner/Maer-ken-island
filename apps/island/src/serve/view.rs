//! What a person looks like on the People page.
//!
//! The stored human is a few thousand fields deep. Reading it raw tells you
//! almost nothing about the person, so this module picks the fields a reader
//! actually asks about — who they are, what their body is doing, how they
//! feel, how they think — labels them in plain words and formats them once,
//! here, where it can be tested. The page draws whatever these sections
//! contain and knows nothing about the schema; the whole stored record is
//! still served alongside for anyone who wants it.

use island_humans::HumanSummary;
use mk_core::human::{BiologicalSex, Generation, Locality};
use mk_engine::humans::needs::SurvivalStatus;
use mk_engine::humans::HumanBeing;
use serde::Serialize;

/// One labelled reading. `meter` is set only for values that genuinely run
/// from 0 to 1, so the page can draw a bar without guessing a scale.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Field {
    pub label: String,
    pub value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meter: Option<f64>,
}

/// A group of readings with a heading and a sentence saying what it is.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Section {
    pub id: &'static str,
    pub title: &'static str,
    pub blurb: &'static str,
    pub fields: Vec<Field>,
}

fn text(label: &str, value: impl Into<String>) -> Field {
    Field {
        label: label.to_string(),
        value: value.into(),
        detail: None,
        meter: None,
    }
}

fn noted(label: &str, value: impl Into<String>, detail: impl Into<String>) -> Field {
    Field {
        label: label.to_string(),
        value: value.into(),
        detail: Some(detail.into()),
        meter: None,
    }
}

/// A 0-1 reading, shown as a percentage with the bar the page draws from it.
fn scale(label: &str, value: f64, detail: &str) -> Field {
    let clamped = value.clamp(0.0, 1.0);
    Field {
        label: label.to_string(),
        value: format!("{:.0}%", clamped * 100.0),
        detail: (!detail.is_empty()).then(|| detail.to_string()),
        meter: Some(clamped),
    }
}

fn measure(label: &str, value: f64, decimals: usize, unit: &str) -> Field {
    text(label, format!("{value:.decimals$} {unit}"))
}

fn section(
    id: &'static str,
    title: &'static str,
    blurb: &'static str,
    fields: Vec<Field>,
) -> Section {
    Section {
        id,
        title,
        blurb,
        fields,
    }
}

fn sex_word(sex: BiologicalSex) -> &'static str {
    match sex {
        BiologicalSex::Male => "Male",
        BiologicalSex::Female => "Female",
        BiologicalSex::Neutral => "Neutral",
    }
}

fn generation_word(generation: Generation) -> String {
    match generation {
        Generation::First => "First — born before the island".to_string(),
        Generation::Second => "Second — the first islanders".to_string(),
        Generation::Third => "Third".to_string(),
        Generation::Later(n) => format!("Generation {n}"),
    }
}

fn locality_word(locality: Locality) -> &'static str {
    match locality {
        Locality::Urban => "urban",
        Locality::Suburban => "suburban",
        Locality::Rural => "rural",
        Locality::SemiRural => "semi-rural",
    }
}

fn survival_word(status: SurvivalStatus) -> &'static str {
    match status {
        SurvivalStatus::Nourished => "Nourished — fed, watered, holding steady",
        SurvivalStatus::Strained => "Strained — running low on something",
        SurvivalStatus::Critical => "Critical — needs food or water now",
    }
}

/// Age as a reader says it: "33 years, 11 months".
pub fn age_in_words(age_years: f64) -> String {
    if !age_years.is_finite() || age_years < 0.0 {
        return "unknown".to_string();
    }
    let years = age_years.trunc() as i64;
    let months = ((age_years - years as f64) * 12.0).round() as i64;
    let (years, months) = if months == 12 {
        (years + 1, 0)
    } else {
        (years, months)
    };
    match (years, months) {
        (0, 0) => "newborn".to_string(),
        (0, 1) => "1 month".to_string(),
        (0, m) => format!("{m} months"),
        (1, 0) => "1 year".to_string(),
        (y, 0) => format!("{y} years"),
        (y, 1) => format!("{y} years, 1 month"),
        (y, m) => format!("{y} years, {m} months"),
    }
}

/// The emotions furthest above this person's own dispositional baseline —
/// what stands out about how they feel right now, rather than whichever
/// emotion happens to have the largest absolute number.
fn standout_emotions(human: &HumanBeing, wanted: usize) -> Vec<(&'static str, f64, f64)> {
    let current = &human.emotion.current;
    let baseline = &human.emotion.baseline;
    let mut named: Vec<(&'static str, f64, f64)> = vec![
        ("Joy", current.joy, baseline.joy),
        ("Sadness", current.sadness, baseline.sadness),
        ("Anger", current.anger, baseline.anger),
        ("Fear", current.fear, baseline.fear),
        ("Love", current.love, baseline.love),
        ("Pride", current.pride, baseline.pride),
        ("Shame", current.shame, baseline.shame),
        ("Guilt", current.guilt, baseline.guilt),
        ("Hope", current.hope, baseline.hope),
        ("Despair", current.despair, baseline.despair),
        ("Curiosity", current.curiosity, baseline.curiosity),
        ("Boredom", current.boredom, baseline.boredom),
        ("Gratitude", current.gratitude, baseline.gratitude),
        ("Contentment", current.contentment, baseline.contentment),
        ("Awe", current.awe, baseline.awe),
        ("Relief", current.relief, baseline.relief),
    ];
    // Sort by distance from baseline, then by level, then by name, so the
    // same human always produces the same list.
    named.sort_by(|a, b| {
        let a_key = ((a.1 - a.2).abs(), a.1);
        let b_key = ((b.1 - b.2).abs(), b.1);
        b_key
            .partial_cmp(&a_key)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.0.cmp(b.0))
    });
    named.truncate(wanted);
    named
}

/// Whether every emotion is sitting exactly where this person's disposition
/// puts it, which is what a human who has never been stepped looks like.
fn emotions_are_at_rest(human: &HumanBeing) -> bool {
    standout_emotions(human, 1)
        .first()
        .is_none_or(|(_, level, baseline)| (level - baseline).abs() < 0.001)
}

/// The sections the People page shows for one person, in reading order.
pub fn sections(human: &HumanBeing, summary: &HumanSummary, folder: Option<&str>) -> Vec<Section> {
    let identity = &human.profile.core_identity;
    let body = &human.body;
    let needs = &human.needs;
    let development = &human.development;
    let temperament = &human.profile.temperament_matrix;
    let social = &human.social_systems;
    let attention = &human.attention;
    let birthplace = &identity.birthplace;

    let mut sections = Vec::new();

    sections.push(section(
        "identity",
        "Who they are",
        "The parts of this person that never change.",
        vec![
            text("Name", summary.name.clone()),
            noted(
                "Agent id",
                summary.agent_id.clone(),
                "The folder and API name for this person.",
            ),
            noted(
                "Human id",
                summary.human_id.clone(),
                "Assigned in creation order.",
            ),
            text("Biological sex", sex_word(*human.biological_sex())),
            text("Age", age_in_words(development.age_years)),
            text("Life stage", format!("{:?}", development.stage)),
            text("Born", identity.birth_timestamp.clone()),
            noted(
                "Birthplace",
                birthplace.location.clone(),
                format!(
                    "{:.4}, {:.4} — {}",
                    birthplace.coordinates.latitude,
                    birthplace.coordinates.longitude,
                    locality_word(birthplace.locality)
                ),
            ),
            text("Generation", generation_word(identity.generation)),
            text(
                "Status",
                match summary.status.as_str() {
                    "alive" => "Alive".to_string(),
                    "dead" => human
                        .death_reason
                        .clone()
                        .map_or("Dead".to_string(), |reason| format!("Dead — {reason}")),
                    other => {
                        let mut word = other.to_string();
                        word[..1].make_ascii_uppercase();
                        word
                    }
                },
            ),
        ],
    ));

    sections.push(section(
        "body",
        "Body",
        "Height, weight and the appearance they were created with.",
        vec![
            measure("Height", body.height_cm, 1, "cm"),
            measure("Weight", body.weight_kg, 1, "kg"),
            text("Build", body.build.clone()),
            text("Hair", body.hair_color.clone()),
            text("Eyes", body.eye_color.clone()),
            text("Skin", body.skin_tone.clone()),
        ],
    ));

    sections.push(section(
        "vitals",
        "Vital signs",
        "What a nurse would write down. These move once the island clock runs.",
        vec![
            measure("Pulse", body.pulse, 0, "bpm"),
            text(
                "Blood pressure",
                format!(
                    "{:.0}/{:.0} mmHg",
                    body.blood_pressure_systolic, body.blood_pressure_diastolic
                ),
            ),
            noted(
                "Resting pressure",
                format!(
                    "{:.0}/{:.0} mmHg",
                    body.resting_systolic, body.resting_diastolic
                ),
                "What they sit at when nothing is happening.",
            ),
            text(
                "Blood oxygen",
                format!("{:.0}% saturation", body.sp_o2 * 100.0),
            ),
            measure("Temperature", body.body_temperature_c, 1, "°C"),
            scale(
                "Physical tension",
                body.tension,
                "Muscular and autonomic load.",
            ),
            scale("Arousal", body.arousal, "How switched-on the body is."),
        ],
    ));

    sections.push(section(
        "needs",
        "How they are doing",
        "Hunger, thirst and tiredness, as the survival model tracks them.",
        vec![
            text("Overall", survival_word(needs.status)),
            scale("Hunger", needs.hunger, "0% is fed, 100% is starving."),
            scale("Thirst", needs.thirst, "0% is watered, 100% is parched."),
            scale(
                "Sleep pressure",
                needs.fatigue,
                "How badly they need to sleep.",
            ),
            scale(
                "Blood glucose",
                needs.glucose,
                "Usable energy in the blood.",
            ),
            scale("Hydration", needs.hydration, "Water in the body."),
            scale(
                "Energy reserve",
                needs.energy_reserve,
                "Fat and protein stores to fall back on.",
            ),
            noted(
                "Sleep state",
                if human.circadian.asleep {
                    "Asleep"
                } else {
                    "Awake"
                },
                "Their circadian clock decides this, and what their body burns follows it.",
            ),
        ],
    ));

    sections.push(section(
        "temperament",
        "Temperament",
        "The dispositions they were born with. These barely move over a life.",
        vec![
            scale(
                "Extroversion",
                temperament.introversion_extroversion as f64,
                "0% keeps to themselves, 100% seeks people out.",
            ),
            scale(
                "Emotional intensity",
                temperament.emotional_intensity as f64,
                "How hard feelings land.",
            ),
            scale(
                "Emotional stability",
                temperament.emotional_stability as f64,
                "How quickly they settle again.",
            ),
            scale("Empathy", temperament.empathy as f64, ""),
            scale("Assertiveness", temperament.assertiveness as f64, ""),
            scale(
                "Sensitivity to surroundings",
                temperament.sensitivity_to_environment as f64,
                "Noise, light, weather, crowding.",
            ),
            scale("Adaptability", temperament.adaptability as f64, ""),
            scale(
                "Conscientiousness",
                temperament.conscientiousness as f64,
                "",
            ),
            scale(
                "Openness to experience",
                temperament.openness_to_experience as f64,
                "",
            ),
        ],
    ));

    sections.push(section(
        "feeling",
        "Feeling right now",
        "The emotions furthest from this person's own baseline.",
        if emotions_are_at_rest(human) {
            vec![noted(
                "At rest",
                "Nothing stirring",
                "Every emotion sits exactly on this person's baseline. Nothing has happened to them yet, because time is not running.",
            )]
        } else {
        standout_emotions(human, 6)
            .into_iter()
            .map(|(name, level, baseline)| {
                let drift = level - baseline;
                let detail = if drift.abs() < 0.005 {
                    format!("At their usual {:.0}%.", baseline * 100.0)
                } else if drift > 0.0 {
                    format!("{:.0} points above their usual {:.0}%.", drift * 100.0, baseline * 100.0)
                } else {
                    format!("{:.0} points below their usual {:.0}%.", -drift * 100.0, baseline * 100.0)
                };
                scale(name, level, &detail)
            })
            .collect()
        },
    ));

    sections.push(section(
        "mind",
        "Attention and thinking",
        "What their mind has available to work with.",
        vec![
            scale(
                "Attention capacity",
                attention.capacity,
                "The ceiling they were born with.",
            ),
            scale("Focus right now", attention.focus_level, ""),
            scale(
                "Cognitive load",
                attention.cognitive_load,
                "How much is already being carried.",
            ),
            scale("Attention fatigue", attention.attention_fatigue, ""),
            scale("Flow", attention.flow, "Absorbed in what they are doing."),
            scale("Distractibility", attention.distractibility, ""),
        ],
    ));

    sections.push(section(
        "social",
        "With other people",
        "How they meet, trust and fall out with everyone else.",
        vec![
            scale("Attachment security", social.attachment_security, ""),
            scale("Trust in others", social.social_trust, ""),
            scale("Cooperation", social.cooperation, "Against competing."),
            scale(
                "Empathic accuracy",
                social.empathic_accuracy,
                "How well they read a room.",
            ),
            scale(
                "Comfort alone",
                social.loneliness_tolerance,
                "100% is content with their own company.",
            ),
            scale("Conflict resolution", social.conflict_resolution, ""),
            text(
                "Pair bond",
                human
                    .pair_bond
                    .clone()
                    .unwrap_or_else(|| "None".to_string()),
            ),
        ],
    ));

    sections.push(section(
        "development",
        "Growing up",
        "How far along this person is, by the development curves.",
        vec![
            scale("Maturity", development.maturity_index, ""),
            scale(
                "Cognitive development",
                development.cognitive_development,
                "",
            ),
            scale("Physical development", development.physical_development, ""),
            scale("Emotional maturity", development.emotional_maturity, ""),
            scale("Social development", development.social_development, ""),
        ],
    ));

    let mut record = vec![
        noted(
            "Schema version",
            human.profile.schema_version.clone(),
            "The canonical human schema this record follows.",
        ),
        noted(
            "Profile created",
            human.profile.created_at.clone(),
            "A schema-built profile carries its birth instant here.",
        ),
    ];
    if let Some(folder) = folder {
        record.push(noted(
            "Folder",
            folder.to_string(),
            "Encrypted with island-data/humans/.secret_storage_key.",
        ));
    }
    sections.push(section(
        "record",
        "Their record",
        "Where this person is kept and which schema they were built from.",
        record,
    ));

    sections
}

#[cfg(test)]
mod tests {
    use super::*;
    use island_humans::{summarize, IslandHumanPopulation};

    fn founder() -> (IslandHumanPopulation, String) {
        let population = IslandHumanPopulation::with_founders();
        (population, "Gem-D".to_string())
    }

    #[test]
    fn every_section_has_a_heading_a_sentence_and_readings() {
        let (population, id) = founder();
        let human = population.get(&id).unwrap();
        let summary = summarize(human);
        let sections = sections(human, &summary, Some("/tmp/island-data/humans/gem-d"));
        assert!(sections.len() >= 9, "{} sections", sections.len());
        for section in &sections {
            assert!(!section.title.is_empty(), "{} has no title", section.id);
            assert!(
                section.blurb.ends_with('.'),
                "{}'s blurb is not a sentence: {:?}",
                section.id,
                section.blurb
            );
            assert!(!section.fields.is_empty(), "{} has no readings", section.id);
            for field in &section.fields {
                assert!(
                    !field.label.is_empty(),
                    "{} has an unlabelled reading",
                    section.id
                );
                assert!(
                    !field.value.is_empty(),
                    "{}'s {} has no value",
                    section.id,
                    field.label
                );
                if let Some(meter) = field.meter {
                    assert!((0.0..=1.0).contains(&meter), "{} is off scale", field.label);
                }
            }
        }
    }

    #[test]
    fn identity_reads_from_the_stored_human() {
        let (population, id) = founder();
        let human = population.get(&id).unwrap();
        let summary = summarize(human);
        let sections = sections(human, &summary, None);
        let identity = &sections[0];
        assert_eq!(identity.id, "identity");
        let value = |label: &str| {
            identity
                .fields
                .iter()
                .find(|f| f.label == label)
                .unwrap_or_else(|| panic!("no {label}"))
                .value
                .clone()
        };
        assert_eq!(value("Agent id"), "Gem-D");
        assert_eq!(value("Human id"), human.profile.human_id.to_string());
        assert_eq!(value("Status"), "Alive");
    }

    #[test]
    fn the_folder_is_only_shown_when_the_population_is_stored() {
        let (population, id) = founder();
        let human = population.get(&id).unwrap();
        let summary = summarize(human);
        let without = sections(human, &summary, None);
        let record = without.last().unwrap();
        assert!(record.fields.iter().all(|f| f.label != "Folder"));
        let with = sections(human, &summary, Some("island-data/humans/gem-d"));
        assert!(with
            .last()
            .unwrap()
            .fields
            .iter()
            .any(|f| f.label == "Folder"));
    }

    #[test]
    fn ages_read_the_way_people_say_them() {
        assert_eq!(age_in_words(0.0), "newborn");
        assert_eq!(age_in_words(0.08), "1 month");
        assert_eq!(age_in_words(0.5), "6 months");
        assert_eq!(age_in_words(1.0), "1 year");
        assert_eq!(age_in_words(33.0), "33 years");
        assert_eq!(age_in_words(33.08), "33 years, 1 month");
        assert_eq!(age_in_words(33.9), "33 years, 11 months");
        // 11.9 months rounds up to a whole year rather than "33 years, 12 months".
        assert_eq!(age_in_words(33.99), "34 years");
        assert_eq!(age_in_words(f64::NAN), "unknown");
    }

    #[test]
    fn a_person_nothing_has_happened_to_says_so_instead_of_listing_zeroes() {
        let (population, id) = founder();
        let human = population.get(&id).unwrap();
        let summary = summarize(human);
        let feeling = sections(human, &summary, None)
            .into_iter()
            .find(|section| section.id == "feeling")
            .unwrap();
        assert_eq!(feeling.fields.len(), 1, "{:?}", feeling.fields);
        assert_eq!(feeling.fields[0].value, "Nothing stirring");

        let mut stirred = human.clone();
        stirred.emotion.current.joy = (stirred.emotion.baseline.joy + 0.4).min(1.0);
        let feeling = sections(&stirred, &summary, None)
            .into_iter()
            .find(|section| section.id == "feeling")
            .unwrap();
        assert_eq!(feeling.fields.len(), 6);
        assert_eq!(feeling.fields[0].label, "Joy");
    }

    #[test]
    fn blood_oxygen_is_shown_as_a_saturation_percentage() {
        let (population, id) = founder();
        let mut human = population.get(&id).unwrap().clone();
        human.body.sp_o2 = 0.97;
        let summary = summarize(&human);
        let vitals = sections(&human, &summary, None)
            .into_iter()
            .find(|section| section.id == "vitals")
            .unwrap();
        let oxygen = vitals
            .fields
            .iter()
            .find(|field| field.label == "Blood oxygen")
            .unwrap();
        assert_eq!(oxygen.value, "97% saturation");
    }

    #[test]
    fn standout_emotions_are_the_ones_furthest_from_baseline() {
        let (population, id) = founder();
        let mut human = population.get(&id).unwrap().clone();
        human.emotion.current.awe = (human.emotion.baseline.awe + 0.6).min(1.0);
        let standouts = standout_emotions(&human, 6);
        assert_eq!(standouts[0].0, "Awe");
        assert_eq!(standouts.len(), 6);
    }
}
