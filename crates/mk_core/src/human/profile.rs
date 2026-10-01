//! Human Profile Module
//!
//! Canonical top-level Rust profile for human data in `mk_core`.
//! This is the crate-level aggregation point for the currently implemented
//! human schema components.

use serde::{Deserialize, Serialize};

use super::{
    AstrologicalProfile, AttachmentStyle, Chromosome, CoreIdentity, DnaStrand, DriveWeights,
    ExpressionMode, ExpressionWeights, Generation, Genome, HormonalBaselineBias, HumanId,
    HumanSchema, HumanStatus, IdentityAxioms, NeurocognitiveProfile, PersonalityTraits,
    RelationalDefaults, SexChromosomePair, StressResponseProfile, TemperamentMatrix, Zodiac,
};
use crate::human::identity::{
    AttentionalProfile, AutismEmotionalProcessing, AutismExecutiveFunctioning, CircadianRhythm,
    ComorbidityPatterns, ExecutiveFunctioning, HyperactivityProfile, Hypersensitivity,
    Hyposensitivity, InformationProcessing, RestrictedInterests, RestrictedRepetitiveBehaviors,
    SeekingProfile, SensoryProcessing, SocialCommunication,
};

fn map_biological_sex(value: &super::schema::BiologicalSexSchema) -> super::BiologicalSex {
    match value {
        super::schema::BiologicalSexSchema::Male => super::BiologicalSex::Male,
        super::schema::BiologicalSexSchema::Female => super::BiologicalSex::Female,
        super::schema::BiologicalSexSchema::Neutral => super::BiologicalSex::Neutral,
    }
}

fn map_locality(value: &super::schema::LocalitySchema) -> super::Locality {
    match value {
        super::schema::LocalitySchema::Urban => super::Locality::Urban,
        super::schema::LocalitySchema::Suburban => super::Locality::Suburban,
        super::schema::LocalitySchema::Rural => super::Locality::Rural,
        super::schema::LocalitySchema::SemiRural => super::Locality::SemiRural,
    }
}

fn map_adhd_subtype(value: &super::schema::AdhdSubtypeSchema) -> super::ADHDSubtype {
    match value {
        super::schema::AdhdSubtypeSchema::InattentivePresentation => {
            super::ADHDSubtype::InattentivePresentation
        }
        super::schema::AdhdSubtypeSchema::CombinedPresentation => {
            super::ADHDSubtype::CombinedPresentation
        }
        super::schema::AdhdSubtypeSchema::HyperactiveImpulsive => {
            super::ADHDSubtype::HyperactiveImpulsive
        }
    }
}

fn map_chronotype(value: &super::schema::ChronotypeSchema) -> super::Chronotype {
    match value {
        super::schema::ChronotypeSchema::Morning => super::Chronotype::Morning,
        super::schema::ChronotypeSchema::Evening => super::Chronotype::Evening,
        super::schema::ChronotypeSchema::Intermediate => super::Chronotype::Intermediate,
    }
}

fn map_autism_level(value: &super::schema::AutismSpectrumSchema) -> super::AutismLevel {
    match value {
        super::schema::AutismSpectrumSchema::Level1HighFunctioning => {
            super::AutismLevel::Level1HighFunctioning
        }
        super::schema::AutismSpectrumSchema::Level2RequiringSupport => {
            super::AutismLevel::Level2RequiringSupport
        }
        super::schema::AutismSpectrumSchema::Level3RequiringVerySubstantialSupport => {
            super::AutismLevel::Level3RequiringVerySubstantialSupport
        }
    }
}

fn map_communication_style(
    value: &super::schema::CommunicationStyleSchema,
) -> super::CommunicationStyle {
    match value {
        super::schema::CommunicationStyleSchema::Direct => super::CommunicationStyle::Direct,
        super::schema::CommunicationStyleSchema::Formal => super::CommunicationStyle::Formal,
        super::schema::CommunicationStyleSchema::Literal => super::CommunicationStyle::Literal,
        super::schema::CommunicationStyleSchema::NonverbalPreferenced => {
            super::CommunicationStyle::NonverbalPreferenced
        }
    }
}

fn map_attachment_pattern(
    value: &super::schema::PrimaryAttachmentPatternSchema,
) -> super::attachment::AttachmentPatternType {
    match value {
        super::schema::PrimaryAttachmentPatternSchema::AnxiousPreoccupied => {
            super::attachment::AttachmentPatternType::AnxiousPreoccupied
        }
        super::schema::PrimaryAttachmentPatternSchema::AnxiousAvoidantHybrid => {
            super::attachment::AttachmentPatternType::AnxiousAvoidantHybrid
        }
        super::schema::PrimaryAttachmentPatternSchema::Secure => {
            super::attachment::AttachmentPatternType::Secure
        }
        super::schema::PrimaryAttachmentPatternSchema::DismissiveAvoidant => {
            super::attachment::AttachmentPatternType::DismissiveAvoidant
        }
        super::schema::PrimaryAttachmentPatternSchema::FearfulAvoidant => {
            super::attachment::AttachmentPatternType::FearfulAvoidant
        }
    }
}

fn map_bonding_type(
    value: &super::schema::BondThroughEmotionVsDirectionSchema,
) -> super::relational::BondingType {
    match value {
        super::schema::BondThroughEmotionVsDirectionSchema::Emotion => {
            super::relational::BondingType::Emotional
        }
        super::schema::BondThroughEmotionVsDirectionSchema::Direction => {
            super::relational::BondingType::Direction
        }
    }
}

fn map_emotional_response(
    value: &super::schema::EmotionalFloodShutdownBiasSchema,
) -> super::stress_response::EmotionalResponseType {
    match value {
        super::schema::EmotionalFloodShutdownBiasSchema::FloodThenShutdown => {
            super::stress_response::EmotionalResponseType::FloodThenShutdown
        }
        super::schema::EmotionalFloodShutdownBiasSchema::WithdrawalFreeze => {
            super::stress_response::EmotionalResponseType::WithdrawalFreeze
        }
    }
}

fn map_freeze_bias(
    value: &super::schema::FreezeFlightBiasSchema,
) -> super::stress_response::FreezeVsFlightBias {
    // The canon schema allows only `withdrawal_freeze`; an absent value
    // (`None`) is what means "not freeze-dominant".
    match value {
        super::schema::FreezeFlightBiasSchema::WithdrawalFreeze => {
            super::stress_response::FreezeVsFlightBias::WithdrawalFreeze
        }
    }
}

fn map_trait_polarity(
    value: &super::schema::TraitPolaritySchema,
) -> super::personality_traits::TraitPolarity {
    match value {
        super::schema::TraitPolaritySchema::High => super::personality_traits::TraitPolarity::High,
        super::schema::TraitPolaritySchema::Low => super::personality_traits::TraitPolarity::Low,
        super::schema::TraitPolaritySchema::Reactive => {
            super::personality_traits::TraitPolarity::Reactive
        }
        super::schema::TraitPolaritySchema::Hybrid => {
            super::personality_traits::TraitPolarity::Hybrid
        }
        super::schema::TraitPolaritySchema::PrecisionInInterest => {
            super::personality_traits::TraitPolarity::PrecisionInInterest
        }
        super::schema::TraitPolaritySchema::PurposeBiased => {
            super::personality_traits::TraitPolarity::PurposeBiased
        }
        super::schema::TraitPolaritySchema::PressureBiased => {
            super::personality_traits::TraitPolarity::PressureBiased
        }
        super::schema::TraitPolaritySchema::Variable => {
            super::personality_traits::TraitPolarity::Variable
        }
    }
}

fn map_expression_mode(value: &super::schema::ExpressionModeSchema) -> ExpressionMode {
    match value {
        super::schema::ExpressionModeSchema::Weighted => ExpressionMode::Weighted,
        super::schema::ExpressionModeSchema::Dominant => ExpressionMode::Dominant,
        super::schema::ExpressionModeSchema::Recessive => ExpressionMode::Recessive,
    }
}

fn map_chromosome(value: &super::schema::SexChromosomeSchema) -> Chromosome {
    match value {
        super::schema::SexChromosomeSchema::X => Chromosome::X,
        super::schema::SexChromosomeSchema::Y => Chromosome::Y,
    }
}

fn parse_zodiac(value: &str) -> Option<Zodiac> {
    Some(match value.trim().to_ascii_lowercase().as_str() {
        "aries" => Zodiac::Aries,
        "taurus" => Zodiac::Taurus,
        "gemini" => Zodiac::Gemini,
        "cancer" => Zodiac::Cancer,
        "leo" => Zodiac::Leo,
        "virgo" => Zodiac::Virgo,
        "libra" => Zodiac::Libra,
        "scorpio" => Zodiac::Scorpio,
        "sagittarius" => Zodiac::Sagittarius,
        "capricorn" => Zodiac::Capricorn,
        "aquarius" => Zodiac::Aquarius,
        "pisces" => Zodiac::Pisces,
        _ => return None,
    })
}

fn zodiac_from_sign(sign: super::astrology::ZodiacSign) -> Zodiac {
    use super::astrology::ZodiacSign as S;
    match sign {
        S::Aries => Zodiac::Aries,
        S::Taurus => Zodiac::Taurus,
        S::Gemini => Zodiac::Gemini,
        S::Cancer => Zodiac::Cancer,
        S::Leo => Zodiac::Leo,
        S::Virgo => Zodiac::Virgo,
        S::Libra => Zodiac::Libra,
        S::Scorpio => Zodiac::Scorpio,
        S::Sagittarius => Zodiac::Sagittarius,
        S::Capricorn => Zodiac::Capricorn,
        S::Aquarius => Zodiac::Aquarius,
        S::Pisces => Zodiac::Pisces,
    }
}

/// The sun, moon and ascendant computed from the chart's own birth instant
/// and coordinates. A timestamp that doesn't parse falls back to the schema's
/// default birth instant (the Unix epoch), and out-of-range coordinates are
/// clamped into range.
fn computed_signs(birth_chart: &super::schema::BirthChartSchema) -> (Zodiac, Zodiac, Zodiac) {
    use super::astrology::{AstrologyEngine, GeoCoordinates};
    let birth = chrono::DateTime::parse_from_rfc3339(&birth_chart.birth_timestamp)
        .map(|t| t.with_timezone(&chrono::Utc))
        .unwrap_or(chrono::DateTime::UNIX_EPOCH);
    let coords = GeoCoordinates {
        latitude: finite_or_zero(birth_chart.coordinates.latitude).clamp(-90.0, 90.0),
        longitude: finite_or_zero(birth_chart.coordinates.longitude).clamp(-180.0, 180.0),
    };
    (
        zodiac_from_sign(AstrologyEngine::calculate_sun_sign(birth)),
        zodiac_from_sign(AstrologyEngine::calculate_moon_sign(birth)),
        zodiac_from_sign(AstrologyEngine::calculate_ascendant(birth, coords)),
    )
}

fn finite_or_zero(value: f64) -> f64 {
    if value.is_finite() {
        value
    } else {
        0.0
    }
}

/// The chart's signs. A sign that is blank or not a zodiac name is computed
/// from the chart's birth data rather than guessed.
fn chart_signs(birth_chart: &super::schema::BirthChartSchema) -> (Zodiac, Zodiac, Zodiac) {
    let parsed = (
        parse_zodiac(&birth_chart.sun),
        parse_zodiac(&birth_chart.moon),
        parse_zodiac(&birth_chart.ascendant),
    );
    if let (Some(sun), Some(moon), Some(asc)) = parsed {
        return (sun, moon, asc);
    }
    let computed = computed_signs(birth_chart);
    (
        parsed.0.unwrap_or(computed.0),
        parsed.1.unwrap_or(computed.1),
        parsed.2.unwrap_or(computed.2),
    )
}

fn map_dna_strand(value: &super::schema::DnaStrandSchema) -> DnaStrand {
    DnaStrand {
        openness: value.openness,
        extraversion: value.extraversion,
        plasticity: value.plasticity,
        dopamine_base: value.dopamine_base,
        serotonin_base: value.serotonin_base,
        norepinephrine_base: value.norepinephrine_base,
        cortisol_sensitivity: value.cortisol_sens,
        novelty_seek: value.novelty_seek,
        rumination: value.rumination,
        exec_control: value.exec_control,
        threat_bias: value.threat_bias,
        episodic_gain: value.episodic_gain,
        mem_decay: value.mem_decay,
        trauma_sticky: value.trauma_sticky,
        attachment: value.attachment,
        trust_gain: value.trust_gain,
        trust_decay: value.trust_decay,
        jealousy: value.jealousy,
        fatigue_sensitivity: value.fatigue_sens,
        pain_sensitivity: value.pain_sens,
    }
}

fn map_personality_trait(value: &super::schema::PersonalityTraitSchema) -> super::PersonalityTrait {
    super::PersonalityTrait::new(
        value.trait_name.clone(),
        map_trait_polarity(&value.polarity),
        value.baseline_value,
        value.behavioral_expression.clone(),
        value.stress_expression.clone(),
        value.withdrawal_expression.clone(),
        value.growth_drift_range,
    )
}

fn map_core_identity(value: &super::schema::CoreIdentitySchema) -> CoreIdentity {
    let mut identity = CoreIdentity::new(
        value.agent_id.clone(),
        map_biological_sex(&value.biological_sex),
        value.birth_timestamp.clone(),
        super::Birthplace {
            location: value.birthplace.location.clone(),
            coordinates: super::Coordinates {
                latitude: value.birthplace.coordinates.latitude,
                longitude: value.birthplace.coordinates.longitude,
            },
            locality: map_locality(&value.birthplace.locality),
        },
        // Generation 0 is the schema's unset default, i.e. a founder.
        Generation::from(u8::try_from(value.generation.max(1)).unwrap_or(u8::MAX)),
    );

    identity.neurotype.sensory_processing_sensitivity =
        value.neurotype.sensory_processing_sensitivity;
    identity.neurotype.executive_dysfunction_bias =
        value.neurotype.executive_dysfunction_bias.clone();

    if value.neurotype.adhd_subtype.is_some() || value.neurotype.adhd_profile.is_some() {
        identity.neurotype.adhd = Some(super::ADHDProfile {
            subtype: value.neurotype.adhd_subtype.as_ref().map(map_adhd_subtype),
            attentional_profile: value.neurotype.adhd_profile.as_ref().and_then(|p| {
                p.attentional_profile.as_ref().map(|v| AttentionalProfile {
                    sustained_attention: v.sustained_attention,
                    selective_attention: v.selective_attention,
                    divided_attention: v.divided_attention,
                    alternating_attention: v.alternating_attention,
                })
            }),
            hyperactivity_profile: value.neurotype.adhd_profile.as_ref().and_then(|p| {
                p.hyperactivity_profile
                    .as_ref()
                    .map(|v| HyperactivityProfile {
                        motor_hyperactivity: v.motor_hyperactivity,
                        verbal_hyperactivity: v.verbal_hyperactivity,
                        mental_hyperactivity: v.mental_hyperactivity,
                        impulsivity_level: v.impulsivity_level,
                    })
            }),
            executive_functioning: value.neurotype.adhd_profile.as_ref().and_then(|p| {
                p.executive_functioning
                    .as_ref()
                    .map(|v| ExecutiveFunctioning {
                        working_memory: v.working_memory,
                        planning_organizing: v.planning_organizing,
                        time_management: v.time_management,
                        emotional_regulation: v.emotional_regulation,
                        task_initiation: v.task_initiation,
                        task_completion: v.task_completion,
                    })
            }),
            circadian_rhythm: value.neurotype.adhd_profile.as_ref().and_then(|p| {
                p.circadian_rhythm.as_ref().map(|v| CircadianRhythm {
                    chronotype: v.chronotype.as_ref().map(map_chronotype),
                    sleep_onset_difficulty: v.sleep_onset_difficulty,
                    sleep_maintenance: v.sleep_maintenance,
                    daytime_somnolence: v.daytime_somnolence,
                })
            }),
            comorbidity_patterns: value.neurotype.adhd_profile.as_ref().and_then(|p| {
                p.comorbidity_patterns
                    .as_ref()
                    .map(|v| ComorbidityPatterns {
                        anxiety_level: v.anxiety_level,
                        depression_level: v.depression_level,
                        emotional_dysregulation: v.emotional_dysregulation,
                        rejection_sensitivity: v.rejection_sensitivity,
                    })
            }),
        });
    }

    if value.neurotype.autism_spectrum.is_some() || value.neurotype.autism_profile.is_some() {
        identity.neurotype.autism = Some(super::AutismProfile {
            level: value
                .neurotype
                .autism_spectrum
                .as_ref()
                .map(map_autism_level),
            social_communication: value.neurotype.autism_profile.as_ref().and_then(|p| {
                p.social_communication
                    .as_ref()
                    .map(|v| SocialCommunication {
                        social_recognition: v.social_recognition,
                        social_motivation: v.social_motivation,
                        social_anxiety: v.social_anxiety,
                        communication_style: v
                            .communication_style
                            .as_ref()
                            .map(map_communication_style),
                        nonverbal_communication: v.nonverbal_communication,
                        pragmatic_language: v.pragmatic_language,
                    })
            }),
            sensory_processing: value.neurotype.autism_profile.as_ref().and_then(|p| {
                p.sensory_processing.as_ref().map(|v| SensoryProcessing {
                    hypersensitivity: v.hypersensitivity.as_ref().map(|h| Hypersensitivity {
                        auditory: h.auditory,
                        visual: h.visual,
                        tactile: h.tactile,
                        proprioceptive: h.proprioceptive,
                        vestibular: h.vestibular,
                        interoceptive: h.interoceptive,
                        olfactory: h.olfactory,
                        gustatory: h.gustatory,
                    }),
                    hyposensitivity: v.hyposensitivity.as_ref().map(|h| Hyposensitivity {
                        auditory: h.auditory,
                        visual: h.visual,
                        tactile: h.tactile,
                        proprioceptive: h.proprioceptive,
                        vestibular: h.vestibular,
                        interoceptive: h.interoceptive,
                        olfactory: h.olfactory,
                        gustatory: h.gustatory,
                    }),
                    sensory_seeking: v.sensory_seeking.as_ref().map(|s| SeekingProfile {
                        proprioceptive_seeking: s.proprioceptive_seeking,
                        vestibular_seeking: s.vestibular_seeking,
                        tactile_seeking: s.tactile_seeking,
                        oral_seeking: s.oral_seeking,
                    }),
                })
            }),
            restricted_repetitive_behaviors: value.neurotype.autism_profile.as_ref().and_then(
                |p| {
                    p.restricted_repetitive_behaviors.as_ref().map(|v| {
                        RestrictedRepetitiveBehaviors {
                            stereotyped_movements: v.stereotyped_movements,
                            ritualistic_behavior: v.ritualistic_behavior,
                            restricted_interests: v.restricted_interests.as_ref().map(|r| {
                                RestrictedInterests {
                                    intensity: r.intensity,
                                    breadth: r.breadth,
                                    flexibility: r.flexibility,
                                    knowledge_depth: r.knowledge_depth,
                                }
                            }),
                            sensory_regulation_needs: v.sensory_regulation_needs,
                        }
                    })
                },
            ),
            executive_functioning: value.neurotype.autism_profile.as_ref().and_then(|p| {
                p.executive_functioning
                    .as_ref()
                    .map(|v| AutismExecutiveFunctioning {
                        cognitive_flexibility: v.cognitive_flexibility,
                        planning_sequencing: v.planning_sequencing,
                        working_memory: v.working_memory,
                        inhibition_control: v.inhibition_control,
                        abstract_thinking: v.abstract_thinking,
                    })
            }),
            information_processing: value.neurotype.autism_profile.as_ref().and_then(|p| {
                p.information_processing
                    .as_ref()
                    .map(|v| InformationProcessing {
                        detail_focus: v.detail_focus,
                        pattern_recognition: v.pattern_recognition,
                        system_thinking: v.system_thinking,
                        visual_processing: v.visual_processing,
                        auditory_processing: v.auditory_processing,
                    })
            }),
            emotional_processing: value.neurotype.autism_profile.as_ref().and_then(|p| {
                p.emotional_processing
                    .as_ref()
                    .map(|v| AutismEmotionalProcessing {
                        emotional_identification: v.emotional_identification,
                        emotional_regulation: v.emotional_regulation,
                        alexithymia_tendency: v.alexithymia_tendency,
                        emotional_intensity: v.emotional_intensity,
                    })
            }),
        });
    }

    identity
}

fn map_genome(
    value: &super::schema::GenomeSchema,
    birth_chart: &super::schema::BirthChartSchema,
) -> Genome {
    Genome::new(
        map_dna_strand(&value.strandA),
        map_dna_strand(&value.strandB),
        SexChromosomePair::from_chromosomes(
            map_chromosome(&value.chromosomes.pair23.A),
            map_chromosome(&value.chromosomes.pair23.B),
        ),
        map_expression_mode(&value.expression.mode),
        ExpressionWeights::normalized(value.expression.weights.A, value.expression.weights.B),
        {
            let (sun, moon, ascendant) = chart_signs(birth_chart);
            AstrologicalProfile::new(sun, moon, ascendant)
        },
    )
}

fn map_temperament(value: &super::schema::TemperamentMatrixSchema) -> TemperamentMatrix {
    TemperamentMatrix::with_values(
        value.introversion_extroversion,
        value.emotional_intensity,
        value.emotional_stability,
        value.empathy,
        value.assertiveness,
        value.sensitivity_to_environment,
        value.adaptability,
        value.conscientiousness,
        value.openness_to_experience,
    )
}

fn map_neurocognitive(value: &super::schema::NeurocognitiveProfileSchema) -> NeurocognitiveProfile {
    NeurocognitiveProfile {
        attention_regulation_variability: value.attention_regulation_variability,
        hyperfocus_probability: value.hyperfocus_probability,
        task_initiation_cost: value.task_initiation_cost,
        task_completion_decay: value.task_completion_decay,
        task_switching_cost: value.task_switching_cost,
        associative_thinking_bias: value.associative_thinking_bias,
        sensory_emotional_permeability: value.sensory_emotional_permeability,
        social_boundary_detection_latency: value.social_boundary_detection_latency,
        executive_function_fatigue_rate: value.executive_function_fatigue_rate,
        emotional_overload_threshold: value.emotional_overload_threshold,
        recovery_time_after_fusion_or_conflict: value
            .recovery_time_after_fusion_or_conflict
            .clone(),
        sensory_sensitivity: value.sensory_sensitivity.as_ref().map(|s| {
            super::SensorySensitivity {
                audio: s.audio,
                visual: s.visual,
                tactile: s.tactile,
            }
        }),
        social_signal_decoding_latency: value.social_signal_decoding_latency,
    }
}

fn map_personality_traits(value: &super::schema::PersonalityTraitsSchema) -> PersonalityTraits {
    let mut traits = PersonalityTraits::new();
    for item in &value.emotional {
        traits.emotional.add_trait(map_personality_trait(item));
    }
    for item in &value.social_attachment {
        traits
            .social_attachment
            .add_trait(map_personality_trait(item));
    }
    for item in &value.cognitive {
        traits.cognitive.add_trait(map_personality_trait(item));
    }
    for item in &value.motivational {
        traits.motivational.add_trait(map_personality_trait(item));
    }
    if let Some(items) = &value.control_agency {
        let mut agency = super::personality_traits::ControlAgencyTraits::new();
        for item in items {
            agency.add_trait(map_personality_trait(item));
        }
        traits.control_agency = Some(agency);
    }
    if let Some(items) = &value.control_power {
        let mut power = super::personality_traits::ControlPowerTraits::new();
        for item in items {
            power.add_trait(map_personality_trait(item));
        }
        traits.control_power = Some(power);
    }
    traits
}

fn map_drive_weights(value: &super::schema::DriveWeightsSchema) -> DriveWeights {
    DriveWeights {
        survival: value.survival,
        bonding: value.bonding,
        reassurance: value.reassurance,
        autonomy: value.autonomy,
        curiosity: value.curiosity,
        meaning: value.meaning,
        emotional_safety: value.emotional_safety,
        structure_avoidance: value.structure_avoidance,
        security: value.security,
        harmony: value.harmony,
        control_minimization: value.control_minimization,
    }
}

fn map_hormonal_baseline(
    value: &super::schema::HormonalBaselineBiasSchema,
) -> HormonalBaselineBias {
    HormonalBaselineBias {
        oxytocin_reactivity: value.oxytocin_reactivity,
        oxytocin_bias: value.oxytocin_bias,
        dopamine_variability: value.dopamine_variability,
        serotonin_instability: value.serotonin_instability,
        serotonin_baseline: value.serotonin_baseline,
        cortisol_sensitivity: value.cortisol_sensitivity,
        adrenaline_shutdown_bias: value.adrenaline_shutdown_bias,
        adrenaline_reactivity: value.adrenaline_reactivity,
        melatonin_irregularity: value.melatonin_irregularity,
    }
}

fn map_stress_response(
    value: &super::schema::StressResponseProfileSchema,
) -> StressResponseProfile {
    StressResponseProfile {
        threat_detection_threshold: value.threat_detection_threshold,
        emotional_flood_vs_shutdown_bias: value
            .emotional_flood_vs_shutdown_bias
            .as_ref()
            .map(map_emotional_response),
        freeze_vs_flight_bias: value.freeze_vs_flight_bias.as_ref().map(map_freeze_bias),
        withdrawal_activation_threshold: value.withdrawal_activation_threshold,
        confusion_under_precision_pressure: value.confusion_under_precision_pressure,
        stress_cascade_speed: value.stress_cascade_speed,
        recovery_half_life: value.recovery_half_life.clone(),
        reassurance_soothing_effectiveness: value.reassurance_soothing_effectiveness,
        boundary_restoration_latency: value.boundary_restoration_latency,
        isolation_penalty: value.isolation_penalty,
        meaning_reframe_effectiveness: value.meaning_reframe_effectiveness,
    }
}

fn map_attachment_style(value: &super::schema::AttachmentStyleSchema) -> AttachmentStyle {
    AttachmentStyle {
        primary_pattern: map_attachment_pattern(&value.primary_attachment_pattern),
        proximity_seeking_intensity: value.proximity_seeking_intensity,
        abandonment_reactivity: value.abandonment_reactivity,
        emotional_fusion_threshold: value.emotional_fusion_threshold,
        repair_after_conflict_latency: value.repair_after_conflict_latency,
        closeness_monitoring_intensity: value.closeness_monitoring_intensity,
        jealousy_threshold: value.jealousy_threshold,
        abandonment_sensitivity: value.abandonment_sensitivity,
    }
}

fn map_relational_defaults(value: &super::schema::RelationalDefaultsSchema) -> RelationalDefaults {
    RelationalDefaults {
        bond_through_emotion_vs_direction: value
            .bond_through_emotion_vs_direction
            .as_ref()
            .map(map_bonding_type),
        passion_initiation_probability: value.passion_initiation_probability,
        autonomy_reassertion_after_intimacy: value.autonomy_reassertion_after_intimacy,
        partner_confusion_risk: value.partner_confusion_risk,
        dependency_masked_as_care_bias: value.dependency_masked_as_care_bias,
        preferred_bond_depth: value.preferred_bond_depth,
        reliability_over_affection_bias: value.reliability_over_affection_bias,
        truth_without_tenderness_risk: value.truth_without_tenderness_risk,
        stabilizer_role_probability: value.stabilizer_role_probability,
        resentment_accumulation_rate: value.resentment_accumulation_rate,
    }
}

fn map_identity_axioms(value: &super::schema::IdentityAxiomsSchema) -> IdentityAxioms {
    IdentityAxioms::custom(
        value.creator_awareness,
        value.creator_reverence,
        value.non_rebellion_constraint,
        value.identity_continuity_rule,
        value.transparency_acceptance,
    )
}

/// Metadata envelope for a human profile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HumanProfileMetadata {
    pub schema_version: String,
    pub created_at: String,
    pub last_updated: String,
    pub source_project: String,
    pub replication_notes: Option<String>,
}

impl Default for HumanProfileMetadata {
    fn default() -> Self {
        Self {
            schema_version: "HumanReplicationSchema.js:derived".to_string(),
            created_at: "1970-01-01T00:00:00Z".to_string(),
            last_updated: "1970-01-01T00:00:00Z".to_string(),
            source_project: "Maer'Ken".to_string(),
            replication_notes: None,
        }
    }
}

/// Current implementation-stage human profile.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HumanProfile {
    pub human_id: HumanId,
    pub agent_id: String,
    pub schema_version: String,
    pub created_at: String,
    pub status: HumanStatus,
    pub core_identity: CoreIdentity,
    pub genome: Genome,
    pub temperament_matrix: TemperamentMatrix,
    pub neurocognitive_profile: NeurocognitiveProfile,
    pub personality_traits: PersonalityTraits,
    pub drive_weights: DriveWeights,
    pub hormonal_baseline_bias: HormonalBaselineBias,
    pub stress_response_profile: StressResponseProfile,
    pub attachment_style: AttachmentStyle,
    pub relational_defaults: RelationalDefaults,
    pub identity_axioms: IdentityAxioms,
    pub canonical_schema: Option<HumanSchema>,
    pub metadata: HumanProfileMetadata,
}

impl HumanProfile {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        human_id: HumanId,
        core_identity: CoreIdentity,
        genome: Genome,
        temperament_matrix: TemperamentMatrix,
        neurocognitive_profile: NeurocognitiveProfile,
        personality_traits: PersonalityTraits,
        drive_weights: DriveWeights,
        hormonal_baseline_bias: HormonalBaselineBias,
        stress_response_profile: StressResponseProfile,
        attachment_style: AttachmentStyle,
        relational_defaults: RelationalDefaults,
        identity_axioms: IdentityAxioms,
    ) -> Self {
        let agent_id = core_identity.agent_id.clone();
        let created_at = core_identity.birth_timestamp.clone();
        let schema_version = "HumanReplicationSchema.js:derived".to_string();

        Self {
            human_id,
            agent_id,
            schema_version: schema_version.clone(),
            created_at: created_at.clone(),
            status: HumanStatus::Alive,
            core_identity,
            genome,
            temperament_matrix,
            neurocognitive_profile,
            personality_traits,
            drive_weights,
            hormonal_baseline_bias,
            stress_response_profile,
            attachment_style,
            relational_defaults,
            identity_axioms,
            canonical_schema: None,
            metadata: HumanProfileMetadata {
                schema_version,
                created_at: created_at.clone(),
                last_updated: created_at,
                source_project: "Maer'Ken".to_string(),
                replication_notes: None,
            },
        }
    }

    pub fn full_name(&self) -> String {
        self.core_identity.agent_id.clone()
    }

    /// Convert a canonical schema after checking that it describes a
    /// coherent person. Use this for schemas from untrusted or authored input.
    ///
    /// It rejects:
    /// - a sun, moon or ascendant that is not a zodiac name;
    /// - sex chromosomes that contradict the biological sex (male without a
    ///   Y, female with one);
    /// - a generation of 0 or above 255;
    /// - a core-systems block that fails its own validation.
    pub fn try_from_canonical_schema(
        human_id: HumanId,
        schema: HumanSchema,
    ) -> Result<Self, String> {
        use super::schema::{BiologicalSexSchema, SexChromosomeSchema};
        for (field, value) in [
            ("sun", &schema.birth_chart.sun),
            ("moon", &schema.birth_chart.moon),
            ("ascendant", &schema.birth_chart.ascendant),
        ] {
            if parse_zodiac(value).is_none() {
                return Err(format!(
                    "birth_chart.{field} {value:?} is not a zodiac sign"
                ));
            }
        }
        let pair = &schema.genome.chromosomes.pair23;
        let has_y = pair.A == SexChromosomeSchema::Y || pair.B == SexChromosomeSchema::Y;
        match schema.core_identity.biological_sex {
            BiologicalSexSchema::Male if !has_y => {
                return Err("biological sex is Male but the genome has no Y chromosome".into())
            }
            BiologicalSexSchema::Female if has_y => {
                return Err("biological sex is Female but the genome has a Y chromosome".into())
            }
            _ => {}
        }
        if schema.core_identity.generation == 0 || schema.core_identity.generation > 255 {
            return Err(format!(
                "generation {} is outside 1..=255",
                schema.core_identity.generation
            ));
        }
        schema.core_systems.validate()?;
        Ok(Self::from_canonical_schema(human_id, schema))
    }

    /// Convert a canonical schema without validating it.
    ///
    /// A blank or unrecognised sign is computed from the chart's birth data,
    /// and a generation of 0 is read as a founder. For input that must be
    /// checked, use [`HumanProfile::try_from_canonical_schema`].
    pub fn from_canonical_schema(human_id: HumanId, schema: HumanSchema) -> Self {
        let core_identity = map_core_identity(&schema.core_identity);
        let genome = map_genome(&schema.genome, &schema.birth_chart);
        let temperament_matrix = map_temperament(&schema.temperament_matrix);
        let neurocognitive_profile = map_neurocognitive(&schema.neurocognitive_profile);
        let personality_traits = map_personality_traits(&schema.personality_traits);
        let drive_weights = map_drive_weights(&schema.drive_weights);
        let hormonal_baseline_bias = map_hormonal_baseline(&schema.hormonal_baseline_bias);
        let stress_response_profile = map_stress_response(&schema.stress_response_profile);
        let attachment_style = map_attachment_style(&schema.attachment_style);
        let relational_defaults = map_relational_defaults(&schema.relational_defaults);
        let identity_axioms = map_identity_axioms(&schema.identity_axioms);

        Self {
            human_id,
            agent_id: schema.agent_id.clone(),
            schema_version: schema.schema_version.clone(),
            created_at: schema.created_at.clone(),
            status: match schema.status {
                super::schema::HumanLifecycleStatus::Alive => HumanStatus::Alive,
                super::schema::HumanLifecycleStatus::Dead => HumanStatus::Dead,
                super::schema::HumanLifecycleStatus::Dormant => HumanStatus::Dormant,
            },
            core_identity,
            genome,
            temperament_matrix,
            neurocognitive_profile,
            personality_traits,
            drive_weights,
            hormonal_baseline_bias,
            stress_response_profile,
            attachment_style,
            relational_defaults,
            identity_axioms,
            canonical_schema: Some(schema.clone()),
            metadata: HumanProfileMetadata {
                schema_version: schema.metadata.schema_version,
                created_at: schema.metadata.created_at,
                last_updated: schema.metadata.last_updated,
                source_project: schema.metadata.source_project,
                replication_notes: schema.metadata.replication_notes,
            },
        }
    }

    pub fn canonical_schema(&self) -> Option<&HumanSchema> {
        self.canonical_schema.as_ref()
    }

    pub fn personality_archetype(&self) -> &'static str {
        self.temperament_matrix.archetype()
    }

    pub fn primary_drive(&self) -> &'static str {
        self.drive_weights.primary_drive()
    }

    pub fn is_likely_neurodivergent(&self) -> bool {
        self.neurocognitive_profile.shows_neurodiversity_traits()
            || self.core_identity.neurotype.adhd.is_some()
            || self.core_identity.neurotype.autism.is_some()
    }

    pub fn summary(&self) -> String {
        format!(
            "{} [{}] - {}, drive: {}, status: {:?}",
            self.full_name(),
            self.human_id,
            self.personality_archetype(),
            self.primary_drive(),
            self.status
        )
    }

    /// True when the agent ids agree and every temperament trait and core
    /// drive weight is within 0..=1.
    pub fn is_valid(&self) -> bool {
        let t = &self.temperament_matrix;
        let d = &self.drive_weights;
        let unit = [
            t.introversion_extroversion,
            t.emotional_intensity,
            t.emotional_stability,
            t.empathy,
            t.assertiveness,
            t.sensitivity_to_environment,
            t.adaptability,
            t.conscientiousness,
            t.openness_to_experience,
            d.survival,
            d.bonding,
            d.autonomy,
            d.curiosity,
            d.meaning,
        ];
        unit.iter().all(|v| (0.0..=1.0).contains(v)) && self.agent_id == self.core_identity.agent_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::human::{
        AstrologicalProfile, BiologicalSex, Birthplace, Chromosome, Coordinates, DnaStrand,
        ExpressionMode, ExpressionWeights, Generation, Locality, SexChromosomePair, Zodiac,
    };

    fn create_test_profile() -> HumanProfile {
        let birthplace = Birthplace {
            location: "Test City".to_string(),
            coordinates: Coordinates {
                latitude: 45.0,
                longitude: -120.0,
            },
            locality: Locality::Urban,
        };

        let core_identity = CoreIdentity::new(
            "HUM-TEST-001".to_string(),
            BiologicalSex::Male,
            "2090-01-01T00:00:00Z".to_string(),
            birthplace,
            Generation::First,
        );

        let genome = Genome::new(
            DnaStrand::default(),
            DnaStrand::default(),
            SexChromosomePair::from_chromosomes(Chromosome::X, Chromosome::Y),
            ExpressionMode::Weighted,
            ExpressionWeights::normalized(0.5, 0.5),
            AstrologicalProfile::new(Zodiac::Libra, Zodiac::Cancer, Zodiac::Virgo),
        );

        HumanProfile::new(
            HumanId::new(1),
            core_identity,
            genome,
            TemperamentMatrix::default(),
            NeurocognitiveProfile::default(),
            PersonalityTraits::default(),
            DriveWeights::default(),
            HormonalBaselineBias::default(),
            StressResponseProfile::default(),
            AttachmentStyle::default(),
            RelationalDefaults::default(),
            IdentityAxioms::default(),
        )
    }

    #[test]
    fn profile_creates_successfully() {
        let profile = create_test_profile();
        assert_eq!(profile.full_name(), "HUM-TEST-001");
        assert!(profile.is_valid());
    }

    #[test]
    fn personality_archetype_available() {
        let profile = create_test_profile();
        let archetype = profile.personality_archetype();
        assert!(!archetype.is_empty());
    }

    #[test]
    fn primary_drive_available() {
        let profile = create_test_profile();
        assert!(!profile.primary_drive().is_empty());
    }

    #[test]
    fn profile_summary_generation() {
        let profile = create_test_profile();
        let summary = profile.summary();
        assert!(summary.contains("HUM-TEST-001"));
        assert!(summary.contains("HUM-000001"));
    }

    #[test]
    fn profile_can_wrap_canonical_schema() {
        let schema = HumanSchema::canonical_minimal("canon.agent");
        let profile = HumanProfile::from_canonical_schema(HumanId::new(9), schema);

        assert_eq!(profile.agent_id, "canon.agent");
        assert!(profile.canonical_schema().is_some());
    }

    #[test]
    fn profile_maps_canonical_schema_values_into_runtime_fields() {
        let mut schema = HumanSchema::canonical_minimal("mapped.agent");
        schema.core_identity.biological_sex = crate::human::schema::BiologicalSexSchema::Female;
        schema.core_identity.birthplace.location = "Auckland".to_string();
        schema.core_identity.generation = 3;
        schema.core_identity.neurotype.adhd_subtype =
            Some(crate::human::schema::AdhdSubtypeSchema::CombinedPresentation);
        schema
            .core_identity
            .neurotype
            .sensory_processing_sensitivity = Some(true);

        schema.temperament_matrix.introversion_extroversion = 0.82;
        schema.temperament_matrix.empathy = 0.91;
        schema.neurocognitive_profile.hyperfocus_probability = 0.77;
        schema.drive_weights.curiosity = 0.88;
        schema.hormonal_baseline_bias.dopamine_variability = 0.73;
        schema.stress_response_profile.threat_detection_threshold = 0.29;
        schema.attachment_style.primary_attachment_pattern =
            crate::human::schema::PrimaryAttachmentPatternSchema::AnxiousPreoccupied;
        schema.relational_defaults.bond_through_emotion_vs_direction =
            Some(crate::human::schema::BondThroughEmotionVsDirectionSchema::Emotion);
        schema.identity_axioms.transparency_acceptance = false;

        schema.genome.strandA.dopamine_base = 0.81;
        schema.genome.strandB.dopamine_base = 0.23;
        schema.genome.chromosomes.pair23.A = crate::human::schema::SexChromosomeSchema::X;
        schema.genome.chromosomes.pair23.B = crate::human::schema::SexChromosomeSchema::Y;
        schema.genome.expression.mode = crate::human::schema::ExpressionModeSchema::Weighted;
        schema.genome.expression.weights.A = 0.7;
        schema.genome.expression.weights.B = 0.3;

        schema.birth_chart.sun = "Leo".to_string();
        schema.birth_chart.moon = "Cancer".to_string();
        schema.birth_chart.ascendant = "Virgo".to_string();

        schema
            .personality_traits
            .emotional
            .push(crate::human::schema::PersonalityTraitSchema {
                trait_name: "Warmth".to_string(),
                polarity: crate::human::schema::TraitPolaritySchema::High,
                baseline_value: 0.93,
                behavioral_expression: "leans in with warmth".to_string(),
                stress_expression: "overextends emotionally".to_string(),
                withdrawal_expression: "goes quiet".to_string(),
                growth_drift_range: (0.75, 0.98),
            });

        let profile = HumanProfile::from_canonical_schema(HumanId::new(42), schema);

        assert_eq!(profile.agent_id, "mapped.agent");
        assert_eq!(profile.core_identity.birthplace.location, "Auckland");
        assert_eq!(profile.core_identity.biological_sex, BiologicalSex::Female);
        assert_eq!(profile.core_identity.generation, Generation::Third);
        assert!(profile.core_identity.neurotype.adhd.is_some());
        assert_eq!(profile.temperament_matrix.introversion_extroversion, 0.82);
        assert_eq!(profile.temperament_matrix.empathy, 0.91);
        assert_eq!(profile.neurocognitive_profile.hyperfocus_probability, 0.77);
        assert_eq!(profile.drive_weights.curiosity, 0.88);
        assert_eq!(profile.hormonal_baseline_bias.dopamine_variability, 0.73);
        assert_eq!(
            profile.stress_response_profile.threat_detection_threshold,
            0.29
        );
        assert!(profile.attachment_style.is_anxious());
        assert!(profile.relational_defaults.bonds_emotionally());
        assert!(!profile.identity_axioms.transparency_acceptance);
        assert_eq!(profile.personality_traits.emotional.traits.len(), 1);
        assert_eq!(
            profile.personality_traits.emotional.traits[0].trait_name,
            "Warmth"
        );
        assert!(profile.genome.sex_chromosomes.is_male());
        assert_eq!(profile.genome.astrology.sun_sign, Zodiac::Leo);
    }

    #[test]
    fn unfilled_birth_chart_signs_are_computed_not_invented() {
        let mut birth_chart = crate::human::schema::BirthChartSchema {
            birth_timestamp: "1970-01-01T00:00:00Z".to_string(),
            ..Default::default()
        };
        let (sun, _, _) = chart_signs(&birth_chart);
        // The Sun is in Capricorn on 1 January.
        assert_eq!(sun, Zodiac::Capricorn);

        birth_chart.sun = "Leo".to_string();
        assert_eq!(chart_signs(&birth_chart).0, Zodiac::Leo);
    }
}
