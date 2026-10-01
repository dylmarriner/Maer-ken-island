//! Sampling a complete, individual [`HumanSchema`].
//!
//! [`HumanSchema::canonical_minimal`] is a structural template: every trait,
//! gene and drive is zero, and its sex chromosomes are the XX default. It is
//! not a person. [`HumanSchema::sample_individual`] builds one:
//!
//! - **Genome.** Two strands with every gene drawn independently. The sex
//!   chromosomes match the biological sex (XY for male, XX for female).
//!   Expression weights are drawn around an even split, so neither parent
//!   strand always dominates.
//! - **Birth chart.** Computed from the real birth instant and birthplace by
//!   [`AstrologyEngine::compute_birth_chart`], not left blank.
//! - **Temperament, neurocognition, drives and hormonal bias.** Derived from
//!   the expressed genome, plus the chart's elemental temperament for the
//!   traits the canon ties to it, plus individual (environmental) variance.
//!
//! Every draw comes from the caller's RNG, so a keyed stream (for example
//! `RngRegistry::stream`) reproduces the same person.

use chrono::{DateTime, Utc};
use rand::Rng;

use super::astrology::{AstrologyEngine, AstrologyTraitMapper, BirthChart, GeoCoordinates};
use super::schema::{
    BiologicalSexSchema, DnaStrandSchema, ExpressionWeightsSchema, GeoCoordinates as SchemaCoords,
    HumanSchema, Pair23Schema, SexChromosomeSchema,
};

/// Why a [`HumanSchema`] could not be sampled.
#[derive(Debug, Clone, PartialEq)]
pub enum SampleIndividualError {
    /// `Neutral` has no single karyotype to draw a genome from. The caller
    /// must pick `Male` or `Female`, or author the genome explicitly.
    NoKaryotypeForSex,
    /// The birthplace coordinates are out of range, or the chart is invalid.
    BirthChart(String),
}

impl std::fmt::Display for SampleIndividualError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoKaryotypeForSex => write!(
                f,
                "biological sex Neutral has no single karyotype; choose Male or Female or author the genome"
            ),
            Self::BirthChart(reason) => write!(f, "birth chart could not be computed: {reason}"),
        }
    }
}

impl std::error::Error for SampleIndividualError {}

/// Population spread of a heritable trait on the 0..1 scale. A standard
/// deviation of 0.15 around 0.5 puts about 99% of people within 0.1..0.9.
const TRAIT_SD: f64 = 0.15;
/// Spread of the non-genetic (environmental) component added to each
/// derived trait.
const ENVIRONMENT_SD: f64 = 0.07;

/// One draw from a normal distribution (Box-Muller transform).
fn normal(rng: &mut impl Rng, mean: f64, sd: f64) -> f64 {
    // `gen::<f64>()` is in [0, 1); shifting into (0, 1] keeps `ln` finite.
    let u1 = 1.0 - rng.gen::<f64>();
    let u2 = rng.gen::<f64>();
    let z = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
    mean + sd * z
}

fn unit(value: f64) -> f32 {
    value.clamp(0.0, 1.0) as f32
}

/// A heritable trait value for one strand.
fn gene(rng: &mut impl Rng) -> f32 {
    unit(normal(rng, 0.5, TRAIT_SD))
}

/// `base` plus individual environmental variance, clamped to 0..1.
fn vary(rng: &mut impl Rng, base: f64) -> f32 {
    unit(normal(rng, base, ENVIRONMENT_SD))
}

fn sample_strand(rng: &mut impl Rng) -> DnaStrandSchema {
    DnaStrandSchema {
        openness: gene(rng),
        extraversion: gene(rng),
        plasticity: Some(gene(rng)),
        dopamine_base: gene(rng),
        serotonin_base: gene(rng),
        norepinephrine_base: gene(rng),
        cortisol_sens: gene(rng),
        novelty_seek: gene(rng),
        rumination: gene(rng),
        exec_control: gene(rng),
        threat_bias: gene(rng),
        episodic_gain: gene(rng),
        mem_decay: gene(rng),
        trauma_sticky: gene(rng),
        attachment: gene(rng),
        trust_gain: gene(rng),
        trust_decay: gene(rng),
        jealousy: gene(rng),
        fatigue_sens: gene(rng),
        pain_sens: gene(rng),
    }
}

/// The expressed value of each gene under weighted co-dominance.
fn express(a: &DnaStrandSchema, b: &DnaStrandSchema, w: &ExpressionWeightsSchema) -> Expressed {
    let (wa, wb) = (w.A as f64, w.B as f64);
    let total = (wa + wb).max(f64::EPSILON);
    let mix = |x: f32, y: f32| (x as f64 * wa + y as f64 * wb) / total;
    Expressed {
        openness: mix(a.openness, b.openness),
        extraversion: mix(a.extraversion, b.extraversion),
        dopamine: mix(a.dopamine_base, b.dopamine_base),
        serotonin: mix(a.serotonin_base, b.serotonin_base),
        cortisol_sens: mix(a.cortisol_sens, b.cortisol_sens),
        novelty_seek: mix(a.novelty_seek, b.novelty_seek),
        rumination: mix(a.rumination, b.rumination),
        exec_control: mix(a.exec_control, b.exec_control),
        threat_bias: mix(a.threat_bias, b.threat_bias),
        mem_decay: mix(a.mem_decay, b.mem_decay),
        attachment: mix(a.attachment, b.attachment),
        trust_gain: mix(a.trust_gain, b.trust_gain),
        fatigue_sens: mix(a.fatigue_sens, b.fatigue_sens),
        pain_sens: mix(a.pain_sens, b.pain_sens),
    }
}

struct Expressed {
    openness: f64,
    extraversion: f64,
    dopamine: f64,
    serotonin: f64,
    cortisol_sens: f64,
    novelty_seek: f64,
    rumination: f64,
    exec_control: f64,
    threat_bias: f64,
    mem_decay: f64,
    attachment: f64,
    trust_gain: f64,
    fatigue_sens: f64,
    pain_sens: f64,
}

/// Hair colours in rough order of global prevalence.
const HAIR_COLORS: [(&str, f64); 5] = [
    ("black", 0.75),
    ("brown", 0.13),
    ("blonde", 0.06),
    ("auburn", 0.04),
    ("red", 0.02),
];

/// Eye colours in rough order of global prevalence.
const EYE_COLORS: [(&str, f64); 5] = [
    ("brown", 0.70),
    ("blue", 0.09),
    ("hazel", 0.08),
    ("amber", 0.07),
    ("green", 0.06),
];

fn weighted_pick(rng: &mut impl Rng, table: &[(&'static str, f64)]) -> &'static str {
    let total: f64 = table.iter().map(|(_, w)| w).sum();
    let mut roll = rng.gen::<f64>() * total;
    for (name, weight) in table {
        if roll < *weight {
            return name;
        }
        roll -= weight;
    }
    table[table.len() - 1].0
}

fn sign_name(sign: super::astrology::ZodiacSign) -> String {
    format!("{sign:?}")
}

fn write_birth_chart(schema: &mut HumanSchema, chart: &BirthChart, timestamp: &str) {
    let bc = &mut schema.birth_chart;
    bc.sun = sign_name(chart.sun);
    bc.moon = sign_name(chart.moon);
    bc.ascendant = sign_name(chart.ascendant);
    bc.element_balance = [
        ("fire", chart.element_balance.fire),
        ("earth", chart.element_balance.earth),
        ("air", chart.element_balance.air),
        ("water", chart.element_balance.water),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_string(), v as f32))
    .collect();
    bc.modality_balance = [
        ("cardinal", chart.modality_balance.cardinal),
        ("fixed", chart.modality_balance.fixed),
        ("mutable", chart.modality_balance.mutable),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_string(), v as f32))
    .collect();
    bc.coordinates = SchemaCoords {
        latitude: chart.coordinates.latitude,
        longitude: chart.coordinates.longitude,
    };
    bc.birth_timestamp = timestamp.to_string();
}

impl HumanSchema {
    /// Set the biological sex and the matching sex-chromosome pair together,
    /// so the genome can never contradict the declared sex. `Neutral` leaves
    /// the chromosomes as authored.
    pub fn set_biological_sex(&mut self, sex: BiologicalSexSchema) {
        let pair = match sex {
            BiologicalSexSchema::Male => Some((SexChromosomeSchema::X, SexChromosomeSchema::Y)),
            BiologicalSexSchema::Female => Some((SexChromosomeSchema::X, SexChromosomeSchema::X)),
            BiologicalSexSchema::Neutral => None,
        };
        if let Some((a, b)) = pair {
            self.genome.chromosomes.pair23 = Pair23Schema {
                A: a.clone(),
                B: b.clone(),
            };
            self.body.dna.chromosomes.pair23 = Pair23Schema { A: a, B: b };
        }
        self.core_identity.biological_sex = sex;
    }

    /// Sample a complete, first-generation individual.
    ///
    /// # Arguments
    ///
    /// * `agent_id` - Stable agent identifier; also used as the name.
    /// * `sex` - `Male` or `Female`. `Neutral` is rejected, because it has no
    ///   single karyotype to draw a genome from.
    /// * `birth` - Birth instant, used for the birth chart.
    /// * `birthplace` - Birth location, used for the ascendant.
    /// * `location` - Human-readable birthplace name.
    /// * `rng` - Source of every draw. A keyed stream reproduces the person.
    pub fn sample_individual(
        agent_id: impl Into<String>,
        sex: BiologicalSexSchema,
        birth: DateTime<Utc>,
        birthplace: GeoCoordinates,
        location: impl Into<String>,
        rng: &mut impl Rng,
    ) -> Result<Self, SampleIndividualError> {
        if sex == BiologicalSexSchema::Neutral {
            return Err(SampleIndividualError::NoKaryotypeForSex);
        }
        let chart = AstrologyEngine::compute_birth_chart(birth, birthplace)
            .map_err(SampleIndividualError::BirthChart)?;

        let mut schema = Self::canonical_minimal(agent_id);
        schema.set_biological_sex(sex.clone());

        // RFC 3339 with a `Z` suffix for UTC, the form authored timestamps use.
        let timestamp = birth.to_rfc3339_opts(chrono::SecondsFormat::AutoSi, true);
        schema.core_identity.birth_timestamp = timestamp.clone();
        schema.core_identity.birthplace.location = location.into();
        schema.core_identity.birthplace.coordinates = SchemaCoords {
            latitude: birthplace.latitude,
            longitude: birthplace.longitude,
        };
        schema.core_identity.generation = 1;
        write_birth_chart(&mut schema, &chart, &timestamp);

        // Genome.
        let strand_a = sample_strand(rng);
        let strand_b = sample_strand(rng);
        let weight_a = unit(normal(rng, 0.5, 0.08)).clamp(0.2, 0.8);
        schema.genome.expression.weights = ExpressionWeightsSchema {
            A: weight_a,
            B: 1.0 - weight_a,
        };
        let g = express(&strand_a, &strand_b, &schema.genome.expression.weights);
        schema.genome.strandA = strand_a;
        schema.genome.strandB = strand_b;
        // The body carries its own copy of the genome; keep them identical.
        schema.body.dna = schema.genome.clone();

        // Temperament: expressed genes blended with the chart's elemental
        // temperament (the canon ties these traits to the chart).
        let astro = AstrologyTraitMapper::map_elemental_to_temperament(&chart.element_balance);
        let behavior = AstrologyTraitMapper::map_modality_to_behavior(&chart.modality_balance);
        let blend = |genetic: f64, chart_value: f32| 0.7 * genetic + 0.3 * chart_value as f64;
        let t = &mut schema.temperament_matrix;
        t.introversion_extroversion = vary(rng, blend(g.extraversion, astro.extroversion));
        t.emotional_intensity = vary(
            rng,
            blend(
                (g.threat_bias + g.rumination) / 2.0,
                astro.emotional_intensity,
            ),
        );
        t.emotional_stability = vary(
            rng,
            blend(
                (g.serotonin + (1.0 - g.cortisol_sens)) / 2.0,
                astro.emotional_stability,
            ),
        );
        t.empathy = vary(
            rng,
            blend((g.attachment + g.trust_gain) / 2.0, astro.empathy),
        );
        t.assertiveness = vary(rng, blend(g.dopamine, behavior.initiative));
        t.sensitivity_to_environment = vary(rng, (g.pain_sens + g.threat_bias) / 2.0);
        t.adaptability = vary(rng, blend(g.novelty_seek, behavior.adaptability));
        t.conscientiousness = vary(rng, blend(g.exec_control, astro.conscientiousness));
        t.openness_to_experience = vary(rng, blend(g.openness, astro.openness_to_experience));

        // Neurocognition.
        let n = &mut schema.neurocognitive_profile;
        n.attention_regulation_variability = vary(rng, 1.0 - g.exec_control);
        n.hyperfocus_probability = vary(rng, (g.novelty_seek + g.dopamine) / 2.0);
        n.task_initiation_cost = vary(rng, 1.0 - (g.exec_control + g.dopamine) / 2.0);
        n.task_completion_decay = vary(rng, (g.mem_decay + g.novelty_seek) / 2.0);
        n.task_switching_cost = Some(vary(rng, g.exec_control * 0.5 + g.rumination * 0.5));
        n.associative_thinking_bias = vary(rng, g.openness);
        n.sensory_emotional_permeability = vary(rng, g.pain_sens);
        n.social_boundary_detection_latency = vary(rng, 1.0 - g.trust_gain);
        n.executive_function_fatigue_rate = vary(rng, g.fatigue_sens);
        n.emotional_overload_threshold = vary(rng, 1.0 - g.threat_bias);
        // Recovery time scales with rumination: roughly 1 to 8 hours.
        let recovery_hours = (1.0 + 7.0 * g.rumination).round() as u32;
        n.recovery_time_after_fusion_or_conflict = format!("PT{recovery_hours}H");

        // Drives.
        let d = &mut schema.drive_weights;
        d.survival = vary(rng, (g.threat_bias + g.cortisol_sens) / 2.0);
        d.bonding = vary(rng, g.attachment);
        d.autonomy = vary(rng, 1.0 - g.attachment * 0.5 - g.threat_bias * 0.2);
        d.curiosity = vary(rng, (g.novelty_seek + g.openness) / 2.0);
        d.meaning = vary(rng, (g.openness + g.rumination) / 2.0);

        // Hormonal bias.
        let h = &mut schema.hormonal_baseline_bias;
        h.dopamine_variability = vary(rng, g.novelty_seek);
        h.cortisol_sensitivity = vary(rng, g.cortisol_sens);
        h.serotonin_baseline = Some(vary(rng, g.serotonin));

        // Appearance. Adult height by sex (global means, NCD-RisC 2016) and
        // body-mass index drawn around the adult mean, which gives weight
        // and build.
        let (height_mean_cm, height_sd_cm) = match sex {
            BiologicalSexSchema::Male => (171.0, 7.4),
            _ => (159.0, 6.8),
        };
        let height_cm = normal(rng, height_mean_cm, height_sd_cm).clamp(135.0, 215.0);
        let bmi = normal(rng, 24.0, 3.5).clamp(16.0, 40.0);
        let height_m = height_cm / 100.0;
        let a = &mut schema.body.appearance;
        a.height = height_cm as f32;
        a.weight = (bmi * height_m * height_m) as f32;
        a.build = match bmi {
            b if b < 18.5 => "slim",
            b if b < 25.0 => "average",
            b if b < 30.0 => "stocky",
            _ => "heavy",
        }
        .to_string();
        a.hair_color = weighted_pick(rng, &HAIR_COLORS).to_string();
        a.eye_color = weighted_pick(rng, &EYE_COLORS).to_string();

        schema.created_at = timestamp.clone();
        schema.metadata.created_at = timestamp.clone();
        schema.metadata.last_updated = timestamp;
        Ok(schema)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::human::HumanId;
    use crate::human::HumanProfile;
    use chrono::TimeZone;
    use rand::SeedableRng;
    use rand_chacha::ChaCha20Rng;

    fn birth() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(1995, 6, 14, 9, 30, 0).unwrap()
    }

    fn auckland() -> GeoCoordinates {
        GeoCoordinates {
            latitude: -36.85,
            longitude: 174.76,
        }
    }

    fn sample(seed: u64, sex: BiologicalSexSchema) -> HumanSchema {
        let mut rng = ChaCha20Rng::seed_from_u64(seed);
        HumanSchema::sample_individual("h", sex, birth(), auckland(), "Auckland", &mut rng)
            .expect("valid inputs")
    }

    #[test]
    fn chromosomes_match_sex() {
        let male = sample(1, BiologicalSexSchema::Male);
        assert_eq!(male.genome.chromosomes.pair23.B, SexChromosomeSchema::Y);
        let female = sample(1, BiologicalSexSchema::Female);
        assert_eq!(female.genome.chromosomes.pair23.A, SexChromosomeSchema::X);
        assert_eq!(female.genome.chromosomes.pair23.B, SexChromosomeSchema::X);
    }

    #[test]
    fn neutral_is_rejected() {
        let mut rng = ChaCha20Rng::seed_from_u64(1);
        let result = HumanSchema::sample_individual(
            "h",
            BiologicalSexSchema::Neutral,
            birth(),
            auckland(),
            "Auckland",
            &mut rng,
        );
        assert_eq!(
            result.unwrap_err(),
            SampleIndividualError::NoKaryotypeForSex
        );
    }

    #[test]
    fn invalid_birthplace_is_rejected() {
        let mut rng = ChaCha20Rng::seed_from_u64(1);
        let bad = GeoCoordinates {
            latitude: 120.0,
            longitude: 0.0,
        };
        let result = HumanSchema::sample_individual(
            "h",
            BiologicalSexSchema::Female,
            birth(),
            bad,
            "Nowhere",
            &mut rng,
        );
        assert!(matches!(result, Err(SampleIndividualError::BirthChart(_))));
    }

    #[test]
    fn birth_chart_is_computed_not_blank() {
        let s = sample(2, BiologicalSexSchema::Female);
        let chart = AstrologyEngine::compute_birth_chart(birth(), auckland()).unwrap();
        assert_eq!(s.birth_chart.sun, format!("{:?}", chart.sun));
        assert_eq!(s.birth_chart.moon, format!("{:?}", chart.moon));
        assert_eq!(s.birth_chart.ascendant, format!("{:?}", chart.ascendant));
        let total: f32 = s.birth_chart.element_balance.values().sum();
        assert!((total - 1.0).abs() < 1e-4);
    }

    #[test]
    fn same_seed_same_person_different_seed_different_person() {
        let a = sample(7, BiologicalSexSchema::Male);
        let b = sample(7, BiologicalSexSchema::Male);
        let c = sample(8, BiologicalSexSchema::Male);
        assert_eq!(a, b);
        assert_ne!(a.genome, c.genome);
        assert_ne!(a.temperament_matrix, c.temperament_matrix);
    }

    #[test]
    fn traits_are_populated_and_in_range() {
        for seed in 0..50 {
            let s = sample(seed, BiologicalSexSchema::Female);
            let t = &s.temperament_matrix;
            for v in [
                t.introversion_extroversion,
                t.emotional_intensity,
                t.emotional_stability,
                t.empathy,
                t.assertiveness,
                t.sensitivity_to_environment,
                t.adaptability,
                t.conscientiousness,
                t.openness_to_experience,
                s.drive_weights.survival,
                s.drive_weights.curiosity,
                s.genome.strandA.openness,
                s.genome.strandB.pain_sens,
            ] {
                assert!((0.0..=1.0).contains(&v));
            }
            let a = &s.body.appearance;
            assert!(!a.hair_color.is_empty() && !a.eye_color.is_empty());
            assert!((135.0..=215.0).contains(&a.height));
            assert!(a.weight > 25.0 && !a.build.is_empty());
            assert_eq!(s.core_identity.generation, 1);
            let w = &s.genome.expression.weights;
            assert!((w.A + w.B - 1.0).abs() < 1e-6);
        }
    }

    #[test]
    fn population_is_varied_around_the_middle() {
        let n = 400;
        let values: Vec<f64> = (0..n)
            .map(|seed| {
                sample(seed, BiologicalSexSchema::Male)
                    .temperament_matrix
                    .empathy as f64
            })
            .collect();
        let mean = values.iter().sum::<f64>() / n as f64;
        let var = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / n as f64;
        assert!((0.35..0.65).contains(&mean), "mean {mean}");
        assert!(var.sqrt() > 0.05, "sd {}", var.sqrt());
    }

    #[test]
    fn profile_conversion_keeps_signs_and_sex() {
        let s = sample(3, BiologicalSexSchema::Male);
        let sun = s.birth_chart.sun.clone();
        let profile = HumanProfile::from_canonical_schema(HumanId::new(1), s);
        assert_eq!(
            profile.core_identity.biological_sex,
            crate::human::BiologicalSex::Male
        );
        assert_eq!(
            format!("{:?}", profile.genome.astrology.sun_sign).to_lowercase(),
            sun.to_lowercase()
        );
    }

    #[test]
    fn strict_conversion_accepts_a_sampled_person() {
        let s = sample(4, BiologicalSexSchema::Female);
        assert!(HumanProfile::try_from_canonical_schema(HumanId::new(1), s).is_ok());
    }

    #[test]
    fn strict_conversion_rejects_contradictions() {
        let mut blank_sign = sample(5, BiologicalSexSchema::Male);
        blank_sign.birth_chart.moon.clear();
        assert!(HumanProfile::try_from_canonical_schema(HumanId::new(1), blank_sign).is_err());

        let mut wrong_karyotype = sample(5, BiologicalSexSchema::Male);
        wrong_karyotype.core_identity.biological_sex = BiologicalSexSchema::Female;
        assert!(HumanProfile::try_from_canonical_schema(HumanId::new(1), wrong_karyotype).is_err());

        let mut generation_zero = sample(5, BiologicalSexSchema::Male);
        generation_zero.core_identity.generation = 0;
        assert!(HumanProfile::try_from_canonical_schema(HumanId::new(1), generation_zero).is_err());
    }

    #[test]
    fn blank_signs_are_computed_from_birth_data() {
        let mut s = sample(6, BiologicalSexSchema::Female);
        let expected = AstrologyEngine::compute_birth_chart(birth(), auckland()).unwrap();
        s.birth_chart.sun.clear();
        s.birth_chart.ascendant = "not a sign".into();
        let profile = HumanProfile::from_canonical_schema(HumanId::new(1), s);
        assert_eq!(
            format!("{:?}", profile.genome.astrology.sun_sign),
            format!("{:?}", expected.sun)
        );
        assert_eq!(
            format!("{:?}", profile.genome.astrology.rising_sign),
            format!("{:?}", expected.ascendant)
        );
    }

    #[test]
    fn set_biological_sex_keeps_genome_consistent() {
        let mut s = HumanSchema::canonical_minimal("t");
        s.set_biological_sex(BiologicalSexSchema::Male);
        assert_eq!(s.genome.chromosomes.pair23.B, SexChromosomeSchema::Y);
        assert_eq!(s.body.biological_sex(), "male");
        s.set_biological_sex(BiologicalSexSchema::Female);
        assert_eq!(s.body.biological_sex(), "female");
    }
}
