//! Genetics Module
//!
//! Genetic inheritance from parents and astrology-based birth traits

use serde::{Deserialize, Serialize};

/// Sex chromosomes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Chromosome {
    X,
    Y,
}

/// Sex chromosome pair
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SexChromosomePair {
    pub maternal: Chromosome,
    pub paternal: Chromosome,
}

impl SexChromosomePair {
    pub fn from_chromosomes(maternal: Chromosome, paternal: Chromosome) -> Self {
        Self { maternal, paternal }
    }

    pub fn is_male(&self) -> bool {
        (self.maternal == Chromosome::Y) || (self.paternal == Chromosome::Y)
    }

    pub fn is_female(&self) -> bool {
        (self.maternal == Chromosome::X) && (self.paternal == Chromosome::X)
    }
}

/// How genes are expressed
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExpressionMode {
    Weighted,
    Dominant,
    Recessive,
}

/// Expression weights
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ExpressionWeights {
    pub maternal: f32,
    pub paternal: f32,
}

impl ExpressionWeights {
    pub fn normalized(maternal: f32, paternal: f32) -> Self {
        let total = maternal + paternal;
        if total == 0.0 {
            Self {
                maternal: 0.5,
                paternal: 0.5,
            }
        } else {
            Self {
                maternal: maternal / total,
                paternal: paternal / total,
            }
        }
    }
}

/// DNA strand with 19 neurochemical traits
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DnaStrand {
    // Consciousness kernel traits
    pub openness: f32,
    pub extraversion: f32,
    pub plasticity: Option<f32>, // strandB learning trait

    // Neurological base traits
    pub dopamine_base: f32,
    pub serotonin_base: f32,
    pub norepinephrine_base: f32,
    pub cortisol_sensitivity: f32,
    pub novelty_seek: f32,
    pub rumination: f32,
    pub exec_control: f32,
    pub threat_bias: f32,
    pub episodic_gain: f32,
    pub mem_decay: f32,
    pub trauma_sticky: f32,
    pub attachment: f32,
    pub trust_gain: f32,
    pub trust_decay: f32,
    pub jealousy: f32,
    pub fatigue_sensitivity: f32,
    pub pain_sensitivity: f32,
}

impl Default for DnaStrand {
    fn default() -> Self {
        Self {
            openness: 0.5,
            extraversion: 0.5,
            plasticity: None,
            dopamine_base: 0.5,
            serotonin_base: 0.5,
            norepinephrine_base: 0.5,
            cortisol_sensitivity: 0.5,
            novelty_seek: 0.5,
            rumination: 0.5,
            exec_control: 0.5,
            threat_bias: 0.5,
            episodic_gain: 0.5,
            mem_decay: 0.5,
            trauma_sticky: 0.5,
            attachment: 0.5,
            trust_gain: 0.5,
            trust_decay: 0.5,
            jealousy: 0.5,
            fatigue_sensitivity: 0.5,
            pain_sensitivity: 0.5,
        }
    }
}

impl DnaStrand {
    pub fn new() -> Self {
        Self::default()
    }

    /// Get all trait values as array for processing
    pub fn traits_array(&self) -> [f32; 19] {
        [
            self.dopamine_base,
            self.serotonin_base,
            self.norepinephrine_base,
            self.cortisol_sensitivity,
            self.novelty_seek,
            self.rumination,
            self.exec_control,
            self.threat_bias,
            self.episodic_gain,
            self.mem_decay,
            self.trauma_sticky,
            self.attachment,
            self.trust_gain,
            self.trust_decay,
            self.jealousy,
            self.fatigue_sensitivity,
            self.pain_sensitivity,
            self.openness,
            self.extraversion,
        ]
    }

    /// Apply `f` locus by locus to this strand and `other`. Plasticity is
    /// optional: when only one strand carries it, that allele is kept.
    pub fn zip_with(&self, other: &DnaStrand, f: impl Fn(f32, f32) -> f32) -> DnaStrand {
        DnaStrand {
            openness: f(self.openness, other.openness),
            extraversion: f(self.extraversion, other.extraversion),
            plasticity: match (self.plasticity, other.plasticity) {
                (Some(a), Some(b)) => Some(f(a, b)),
                (Some(a), None) => Some(a),
                (None, Some(b)) => Some(b),
                (None, None) => None,
            },
            dopamine_base: f(self.dopamine_base, other.dopamine_base),
            serotonin_base: f(self.serotonin_base, other.serotonin_base),
            norepinephrine_base: f(self.norepinephrine_base, other.norepinephrine_base),
            cortisol_sensitivity: f(self.cortisol_sensitivity, other.cortisol_sensitivity),
            novelty_seek: f(self.novelty_seek, other.novelty_seek),
            rumination: f(self.rumination, other.rumination),
            exec_control: f(self.exec_control, other.exec_control),
            threat_bias: f(self.threat_bias, other.threat_bias),
            episodic_gain: f(self.episodic_gain, other.episodic_gain),
            mem_decay: f(self.mem_decay, other.mem_decay),
            trauma_sticky: f(self.trauma_sticky, other.trauma_sticky),
            attachment: f(self.attachment, other.attachment),
            trust_gain: f(self.trust_gain, other.trust_gain),
            trust_decay: f(self.trust_decay, other.trust_decay),
            jealousy: f(self.jealousy, other.jealousy),
            fatigue_sensitivity: f(self.fatigue_sensitivity, other.fatigue_sensitivity),
            pain_sensitivity: f(self.pain_sensitivity, other.pain_sensitivity),
        }
    }

    /// Combine with another strand based on expression mode, locus by locus.
    ///
    /// - `Weighted`: co-dominance, each allele contributing its weight.
    /// - `Dominant`: at each locus the higher-expressing allele is expressed.
    /// - `Recessive`: at each locus the lower-expressing allele is expressed.
    pub fn combine_with(
        &self,
        other: &DnaStrand,
        mode: ExpressionMode,
        weights: ExpressionWeights,
    ) -> DnaStrand {
        match mode {
            ExpressionMode::Weighted => {
                self.zip_with(other, |a, b| a * weights.maternal + b * weights.paternal)
            }
            ExpressionMode::Dominant => self.zip_with(other, f32::max),
            ExpressionMode::Recessive => self.zip_with(other, f32::min),
        }
    }
}

/// Astrological birth influence
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Zodiac {
    Aries,
    Taurus,
    Gemini,
    Cancer,
    Leo,
    Virgo,
    Libra,
    Scorpio,
    Sagittarius,
    Capricorn,
    Aquarius,
    Pisces,
}

/// Astrological birth traits
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AstrologicalProfile {
    pub sun_sign: Zodiac,
    pub moon_sign: Zodiac,
    pub rising_sign: Zodiac,

    /// Season of birth influence
    pub birth_season: String,

    /// Lunar phase at birth
    pub lunar_phase: String,

    /// Astrological trait modifiers (0.0 - 1.0)
    pub trait_modifiers: std::collections::BTreeMap<String, f32>,
}

impl AstrologicalProfile {
    pub fn new(sun_sign: Zodiac, moon_sign: Zodiac, rising_sign: Zodiac) -> Self {
        Self {
            sun_sign,
            moon_sign,
            rising_sign,
            birth_season: "".to_string(),
            lunar_phase: "".to_string(),
            trait_modifiers: std::collections::BTreeMap::new(),
        }
    }

    /// Get personality influence from sun sign
    pub fn sun_sign_trait_boost(&self) -> f32 {
        match self.sun_sign {
            Zodiac::Aries | Zodiac::Leo | Zodiac::Sagittarius => 0.2, // Fire signs - energetic
            Zodiac::Taurus | Zodiac::Virgo | Zodiac::Capricorn => -0.1, // Earth signs - grounded
            Zodiac::Gemini | Zodiac::Libra | Zodiac::Aquarius => 0.15, // Air signs - mental
            Zodiac::Cancer | Zodiac::Scorpio | Zodiac::Pisces => 0.1, // Water signs - emotional
        }
    }

    /// Get emotional influence from moon sign
    pub fn moon_sign_emotional_impact(&self) -> f32 {
        match self.moon_sign {
            Zodiac::Cancer | Zodiac::Pisces => 0.3, // Emotional signs
            Zodiac::Scorpio => 0.25,
            Zodiac::Taurus => 0.15,
            Zodiac::Leo => 0.1,
            _ => 0.05,
        }
    }

    /// Get social influence from rising sign
    pub fn rising_sign_presentation(&self) -> &'static str {
        match self.rising_sign {
            Zodiac::Aries => "Bold and Direct",
            Zodiac::Taurus => "Calm and Stable",
            Zodiac::Gemini => "Curious and Adaptable",
            Zodiac::Cancer => "Nurturing and Protective",
            Zodiac::Leo => "Charismatic and Confident",
            Zodiac::Virgo => "Analytical and Helpful",
            Zodiac::Libra => "Diplomatic and Graceful",
            Zodiac::Scorpio => "Intense and Mysterious",
            Zodiac::Sagittarius => "Adventurous and Optimistic",
            Zodiac::Capricorn => "Ambitious and Responsible",
            Zodiac::Aquarius => "Intellectual and Progressive",
            Zodiac::Pisces => "Dreamy and Compassionate",
        }
    }

    /// Attach season/lunar-phase metadata derived from a real birth
    /// timestamp, and populate `trait_modifiers` from this profile's own
    /// sun/moon influence formulas.
    ///
    /// `new()` alone leaves `birth_season`/`lunar_phase`/`trait_modifiers`
    /// empty because most callers (the manual creator-UI form, tests) have
    /// no real birth date/location to derive them from. This method is for
    /// the one caller that does: `mk_engine::humans::lifecycle`'s
    /// child-birth path, which computes a real
    /// `astrology::AstrologyEngine` chart. See
    /// `audit-results/hostile-completeness-audit-2026-09-04.md` finding A —
    /// these fields previously existed in the schema but were never
    /// populated or read anywhere.
    pub fn with_derived_metadata(mut self, birth_season: String, lunar_phase: String) -> Self {
        let mut trait_modifiers = std::collections::BTreeMap::new();
        trait_modifiers.insert("sun_trait_boost".to_string(), self.sun_sign_trait_boost());
        trait_modifiers.insert(
            "moon_emotional_impact".to_string(),
            self.moon_sign_emotional_impact(),
        );
        self.birth_season = birth_season;
        self.lunar_phase = lunar_phase;
        self.trait_modifiers = trait_modifiers;
        self
    }
}

/// Complete genetic profile
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Genome {
    pub maternal_dna: DnaStrand,
    pub paternal_dna: DnaStrand,
    pub sex_chromosomes: SexChromosomePair,
    pub expression_mode: ExpressionMode,
    pub expression_weights: ExpressionWeights,
    pub astrology: AstrologicalProfile,
    pub resulting_phenotype: DnaStrand,
}

impl Genome {
    pub fn new(
        maternal_dna: DnaStrand,
        paternal_dna: DnaStrand,
        sex_chromosomes: SexChromosomePair,
        expression_mode: ExpressionMode,
        expression_weights: ExpressionWeights,
        astrology: AstrologicalProfile,
    ) -> Self {
        let resulting_phenotype =
            maternal_dna.combine_with(&paternal_dna, expression_mode, expression_weights);

        Self {
            maternal_dna,
            paternal_dna,
            sex_chromosomes,
            expression_mode,
            expression_weights,
            astrology,
            resulting_phenotype,
        }
    }

    /// Get genetic contribution from each parent
    pub fn parental_contribution(&self) -> (f32, f32) {
        (
            self.expression_weights.maternal,
            self.expression_weights.paternal,
        )
    }

    /// Check if genetically similar to another human
    pub fn genetic_similarity(&self, other: &Genome) -> f32 {
        let self_traits = self.resulting_phenotype.traits_array();
        let other_traits = other.resulting_phenotype.traits_array();

        let mut total_similarity: f32 = self_traits
            .iter()
            .zip(other_traits.iter())
            .map(|(a, b)| 1.0 - (a - b).abs())
            .sum();
        let mut loci = self_traits.len() as f32;
        // Plasticity is optional; compare it only when both carry it.
        if let (Some(a), Some(b)) = (
            self.resulting_phenotype.plasticity,
            other.resulting_phenotype.plasticity,
        ) {
            total_similarity += 1.0 - (a - b).abs();
            loci += 1.0;
        }

        (total_similarity / loci).clamp(0.0, 1.0)
    }
}

/// Haploid gamete produced by meiosis from a parent's genome.
/// Carries one copy of each gene (recombined) plus a single sex chromosome.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Gamete {
    pub dna: DnaStrand,
    pub sex_chromosome: Chromosome,
    pub mutation_count: u32,
}

impl Gamete {
    pub fn new(dna: DnaStrand, sex_chromosome: Chromosome, mutation_count: u32) -> Self {
        Self {
            dna,
            sex_chromosome,
            mutation_count,
        }
    }
}

/// Cross-over rate per trait for meiosis (0.0–1.0).
const CROSSOVER_RATE: f32 = 0.35;
/// Base mutation probability per trait during gamete formation.
const MUTATION_RATE: f32 = 0.005;
/// Maximum mutation magnitude as fraction of trait range.
const MUTATION_MAGNITUDE: f32 = 0.08;

/// Perform meiosis on a parent genome, producing a haploid gamete.
///
/// Each trait independently crosses over between maternal/paternal strands
/// with probability CROSSOVER_RATE, then each trait mutates with probability
/// MUTATION_RATE by up to ±MUTATION_MAGNITUDE.
pub fn create_gamete(parent: &Genome, rng: &mut impl FnMut() -> f32) -> Gamete {
    let maternal = &parent.maternal_dna;
    let paternal = &parent.paternal_dna;

    // Create recombined DNA strand via crossover
    let recombined = recombine(maternal, paternal, rng);

    // Apply point mutations
    let (mutated, mutation_count) = apply_mutations(recombined, rng);

    // Select sex chromosome (randomly pick X or Y from parent's pair)
    let sex_chromosome = if rng() < 0.5 {
        parent.sex_chromosomes.maternal
    } else {
        parent.sex_chromosomes.paternal
    };

    Gamete::new(mutated, sex_chromosome, mutation_count)
}

/// Recombine maternal and paternal strands via independent crossover per trait.
fn recombine(
    maternal: &DnaStrand,
    paternal: &DnaStrand,
    rng: &mut impl FnMut() -> f32,
) -> DnaStrand {
    let mut cross = |a: f32, b: f32| -> f32 {
        if rng() < CROSSOVER_RATE {
            b
        } else {
            a
        }
    };

    DnaStrand {
        openness: cross(maternal.openness, paternal.openness),
        extraversion: cross(maternal.extraversion, paternal.extraversion),
        plasticity: match (maternal.plasticity, paternal.plasticity) {
            (Some(a), Some(b)) => Some(cross(a, b)),
            (Some(a), None) => Some(a),
            (None, Some(b)) => Some(b),
            (None, None) => None,
        },
        dopamine_base: cross(maternal.dopamine_base, paternal.dopamine_base),
        serotonin_base: cross(maternal.serotonin_base, paternal.serotonin_base),
        norepinephrine_base: cross(maternal.norepinephrine_base, paternal.norepinephrine_base),
        cortisol_sensitivity: cross(maternal.cortisol_sensitivity, paternal.cortisol_sensitivity),
        novelty_seek: cross(maternal.novelty_seek, paternal.novelty_seek),
        rumination: cross(maternal.rumination, paternal.rumination),
        exec_control: cross(maternal.exec_control, paternal.exec_control),
        threat_bias: cross(maternal.threat_bias, paternal.threat_bias),
        episodic_gain: cross(maternal.episodic_gain, paternal.episodic_gain),
        mem_decay: cross(maternal.mem_decay, paternal.mem_decay),
        trauma_sticky: cross(maternal.trauma_sticky, paternal.trauma_sticky),
        attachment: cross(maternal.attachment, paternal.attachment),
        trust_gain: cross(maternal.trust_gain, paternal.trust_gain),
        trust_decay: cross(maternal.trust_decay, paternal.trust_decay),
        jealousy: cross(maternal.jealousy, paternal.jealousy),
        fatigue_sensitivity: cross(maternal.fatigue_sensitivity, paternal.fatigue_sensitivity),
        pain_sensitivity: cross(maternal.pain_sensitivity, paternal.pain_sensitivity),
    }
}

/// Apply random point mutations to a DNA strand.
fn apply_mutations(mut dna: DnaStrand, rng: &mut impl FnMut() -> f32) -> (DnaStrand, u32) {
    let mut count = 0u32;

    let mut maybe_mutate = |val: &mut f32| {
        if rng() < MUTATION_RATE {
            count += 1;
            *val = (*val + (rng() - 0.5) * 2.0 * MUTATION_MAGNITUDE).clamp(0.0, 1.0);
        }
    };

    maybe_mutate(&mut dna.openness);
    maybe_mutate(&mut dna.extraversion);
    if let Some(ref mut p) = dna.plasticity {
        maybe_mutate(p);
    }
    maybe_mutate(&mut dna.dopamine_base);
    maybe_mutate(&mut dna.serotonin_base);
    maybe_mutate(&mut dna.norepinephrine_base);
    maybe_mutate(&mut dna.cortisol_sensitivity);
    maybe_mutate(&mut dna.novelty_seek);
    maybe_mutate(&mut dna.rumination);
    maybe_mutate(&mut dna.exec_control);
    maybe_mutate(&mut dna.threat_bias);
    maybe_mutate(&mut dna.episodic_gain);
    maybe_mutate(&mut dna.mem_decay);
    maybe_mutate(&mut dna.trauma_sticky);
    maybe_mutate(&mut dna.attachment);
    maybe_mutate(&mut dna.trust_gain);
    maybe_mutate(&mut dna.trust_decay);
    maybe_mutate(&mut dna.jealousy);
    maybe_mutate(&mut dna.fatigue_sensitivity);
    maybe_mutate(&mut dna.pain_sensitivity);

    (dna, count)
}

/// Relative expression strength of a gamete: `(1 - MUTATION_MAGNITUDE)^n`
/// for `n` point mutations, so a clean gamete scores 1.
fn gamete_health(gamete: &Gamete) -> f32 {
    (1.0 - MUTATION_MAGNITUDE).powi(gamete.mutation_count.min(i32::MAX as u32) as i32)
}

/// Conceive a new genome from two gametes (one from each parent).
///
/// Returns the new Genome with recombined DNA and sex determined by the
/// paternal gamete's sex chromosome. Expression weights start at an even
/// split and are modulated by gamete health: each point mutation a gamete
/// carries reduces its relative contribution by `MUTATION_MAGNITUDE`.
pub fn conceive(maternal_gamete: &Gamete, paternal_gamete: &Gamete) -> Genome {
    let sex_chromosomes = SexChromosomePair {
        maternal: maternal_gamete.sex_chromosome,
        paternal: paternal_gamete.sex_chromosome,
    };

    let expression_weights = ExpressionWeights::normalized(
        gamete_health(maternal_gamete),
        gamete_health(paternal_gamete),
    );
    let expression_mode = ExpressionMode::Weighted;

    // Conception has no birth date/location yet (meiosis happens before
    // the child's birth timestamp and coordinates are computed), so a real
    // chart can't be derived here. This scaffold value is always
    // overwritten by a real `astrology::AstrologyEngine` chart in
    // `mk_engine::humans::lifecycle`'s child-birth path once the birth
    // timestamp/coordinates exist — see that module for the real
    // computation and its `with_derived_metadata` follow-up. If that path
    // ever fails to compute a chart, it logs a warning rather than silently
    // keeping this value, so a Libra/Libra/Libra genome downstream is
    // always a known, logged fallback rather than an invisible one.
    let astrology = AstrologicalProfile::new(Zodiac::Libra, Zodiac::Libra, Zodiac::Libra);

    Genome::new(
        maternal_gamete.dna.clone(),
        paternal_gamete.dna.clone(),
        sex_chromosomes,
        expression_mode,
        expression_weights,
        astrology,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dominant_and_recessive_choose_per_locus() {
        let a = DnaStrand {
            openness: 0.9,
            rumination: 0.1,
            ..DnaStrand::default()
        };
        let b = DnaStrand {
            openness: 0.2,
            rumination: 0.8,
            ..DnaStrand::default()
        };
        let w = ExpressionWeights::normalized(0.7, 0.3);
        let dominant = a.combine_with(&b, ExpressionMode::Dominant, w);
        assert_eq!((dominant.openness, dominant.rumination), (0.9, 0.8));
        let recessive = a.combine_with(&b, ExpressionMode::Recessive, w);
        assert_eq!((recessive.openness, recessive.rumination), (0.2, 0.1));
    }

    #[test]
    fn mutated_gametes_contribute_less() {
        let clean = Gamete::new(DnaStrand::default(), Chromosome::X, 0);
        let mutated = Gamete::new(DnaStrand::default(), Chromosome::Y, 3);
        let genome = conceive(&clean, &mutated);
        assert!(genome.expression_weights.maternal > genome.expression_weights.paternal);
        let sum = genome.expression_weights.maternal + genome.expression_weights.paternal;
        assert!((sum - 1.0).abs() < 1e-6);
        let even = conceive(&clean, &Gamete::new(DnaStrand::default(), Chromosome::Y, 0));
        assert_eq!(even.expression_weights.maternal, 0.5);
    }

    #[test]
    fn with_derived_metadata_populates_previously_dead_fields() {
        let profile = AstrologicalProfile::new(Zodiac::Cancer, Zodiac::Scorpio, Zodiac::Leo)
            .with_derived_metadata("Summer".to_string(), "Full Moon".to_string());

        assert_eq!(profile.birth_season, "Summer");
        assert_eq!(profile.lunar_phase, "Full Moon");
        assert_eq!(
            profile.trait_modifiers.get("sun_trait_boost").copied(),
            Some(profile.sun_sign_trait_boost())
        );
        assert_eq!(
            profile
                .trait_modifiers
                .get("moon_emotional_impact")
                .copied(),
            Some(profile.moon_sign_emotional_impact())
        );
    }

    #[test]
    fn new_leaves_derived_metadata_empty_for_callers_without_real_birth_data() {
        let profile = AstrologicalProfile::new(Zodiac::Aries, Zodiac::Aries, Zodiac::Aries);
        assert_eq!(profile.birth_season, "");
        assert_eq!(profile.lunar_phase, "");
        assert!(profile.trait_modifiers.is_empty());
    }

    #[test]
    fn sex_chromosomes_determination() {
        let female = SexChromosomePair::from_chromosomes(Chromosome::X, Chromosome::X);
        assert!(female.is_female());
        assert!(!female.is_male());

        let male = SexChromosomePair::from_chromosomes(Chromosome::X, Chromosome::Y);
        assert!(male.is_male());
        assert!(!male.is_female());
    }

    #[test]
    fn dna_weighted_inheritance() {
        let maternal = DnaStrand {
            dopamine_base: 0.8,
            ..Default::default()
        };
        let paternal = DnaStrand {
            dopamine_base: 0.2,
            ..Default::default()
        };

        let weights = ExpressionWeights::normalized(0.6, 0.4);
        let child = maternal.combine_with(&paternal, ExpressionMode::Weighted, weights);

        assert!((child.dopamine_base - 0.56).abs() < 0.01);
    }

    #[test]
    fn astrology_sun_sign_effects() {
        let astro = AstrologicalProfile::new(Zodiac::Leo, Zodiac::Cancer, Zodiac::Libra);
        let boost = astro.sun_sign_trait_boost();
        assert!(boost > 0.15);
    }

    #[test]
    fn genetic_similarity() {
        let astro = AstrologicalProfile::new(Zodiac::Libra, Zodiac::Libra, Zodiac::Libra);

        let dna1 = DnaStrand::default();
        let dna2 = DnaStrand::default();
        let sex = SexChromosomePair::from_chromosomes(Chromosome::X, Chromosome::Y);

        let genome1 = Genome::new(
            dna1.clone(),
            dna2.clone(),
            sex,
            ExpressionMode::Weighted,
            ExpressionWeights::normalized(0.5, 0.5),
            astro.clone(),
        );

        let genome2 = Genome::new(
            dna1,
            dna2,
            sex,
            ExpressionMode::Weighted,
            ExpressionWeights::normalized(0.5, 0.5),
            astro,
        );

        let similarity = genome1.genetic_similarity(&genome2);
        assert!(similarity > 0.95);
    }
}
