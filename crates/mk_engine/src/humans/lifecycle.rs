//! Human Lifecycle - Aging, Birth, and Death logic
//!
//! This module implements the temporal evolution of human beings.

use super::genetics::GeneticsSnapshot;
use super::registry::HumanRegistry;
use super::reproduction::ReproductiveTimeline;
use super::{BiologicalSex, HumanBeing, HumanStatus};
use chrono::Utc;
use mk_core::rng::{RngExt, RngKey, RngRegistry, SubsystemId};
use serde_json::Value;
use tracing::warn;

pub(crate) const MAX_HUMAN_AGE_YEARS: f64 = 122.0;

/// Population distributions of pubertal and reproductive timing, as
/// (mean, standard deviation) in years.
///
/// - Female puberty onset is breast budding (Tanner B2), 10.0 ± 1.2 y in
///   contemporary cohorts (Eckert-Lind et al. 2020 meta-analysis); menarche
///   follows it by 2.3 y (Marshall & Tanner 1969).
/// - Male puberty onset is testicular enlargement (Tanner G2), 11.6 ± 1.1 y
///   (Marshall & Tanner 1970); spermarche follows it by about 1.8 y
///   (13.4 y — Nielsen et al. 1986).
/// - Female fertility ends about ten years before the 51 ± 4 y menopause,
///   at 41 ± 4 y (te Velde & Pearson 2002). Men have no such end.
const FEMALE_PUBERTY_ONSET: (f64, f64) = (10.0, 1.2);
const MALE_PUBERTY_ONSET: (f64, f64) = (11.6, 1.1);
const FEMALE_STERILITY_AGE: (f64, f64) = (41.0, 4.0);
const MENARCHE_AFTER_ONSET_YEARS: f64 = 2.3;
const SPERMARCHE_AFTER_ONSET_YEARS: f64 = 1.8;

/// After menarche the share of ovulatory cycles rises with a 4.5-year
/// e-folding time (≈20 % in year one, ≈75 % by year six — Apter & Vihko
/// 1983); sperm output reaches adult levels within a few years of
/// spermarche.
const OVULATORY_MATURATION_YEARS: f64 = 4.5;
const SPERMATOGENIC_MATURATION_YEARS: f64 = 1.5;

/// Female fecundability falls from the late twenties and halves by the
/// late thirties (Dunson, Colombo & Baird 2002); the male decline starts
/// later and is shallower (Dunson, Baird & Colombo 2004).
const FEMALE_DECLINE_ONSET_YEARS: f64 = 27.0;
const FEMALE_DECLINE_HALF_LIFE_YEARS: f64 = 10.0;
const MALE_DECLINE_ONSET_YEARS: f64 = 35.0;
const MALE_DECLINE_HALF_LIFE_YEARS: f64 = 20.0;

/// Where this individual falls within a population distribution: the
/// standard-normal quantile of a uniform keyed by their genome and by what
/// is being drawn, so the same genome always matures on the same schedule
/// and different traits are drawn independently.
fn genome_normal_quantile(genetics: &GeneticsSnapshot, trait_name: &str) -> f64 {
    standard_normal_quantile(genome_uniform(genetics, trait_name))
}

/// A uniform draw in the open unit interval keyed by genome and trait.
fn genome_uniform(genetics: &GeneticsSnapshot, trait_name: &str) -> f64 {
    let mut hasher = blake3::Hasher::new();
    hasher.update(trait_name.as_bytes());
    for value in [
        genetics.heritable_stability,
        genetics.openness,
        genetics.extraversion,
        genetics.plasticity,
        genetics.dopamine_base,
        genetics.serotonin_base,
        genetics.norepinephrine_base,
        genetics.cortisol_sensitivity,
        genetics.novelty_seek,
        genetics.rumination,
        genetics.trust_gain,
        genetics.expression_weight_a,
    ] {
        hasher.update(&value.to_bits().to_le_bytes());
    }
    let mut word = [0u8; 8];
    word.copy_from_slice(&hasher.finalize().as_bytes()[..8]);
    // 53 uniform bits, centred in their bucket so the draw is never 0 or 1.
    ((u64::from_le_bytes(word) >> 11) as f64 + 0.5) / (1u64 << 53) as f64
}

/// Inverse of the standard normal CDF (Acklam's rational approximation,
/// relative error below 1.2e-9 over the open unit interval).
fn standard_normal_quantile(p: f64) -> f64 {
    const A: [f64; 6] = [
        -3.969_683_028_665_376e1,
        2.209_460_984_245_205e2,
        -2.759_285_104_469_687e2,
        1.383_577_518_672_69e2,
        -3.066_479_806_614_716e1,
        2.506_628_277_459_239,
    ];
    const B: [f64; 5] = [
        -5.447_609_879_822_406e1,
        1.615_858_368_580_409e2,
        -1.556_989_798_598_866e2,
        6.680_131_188_771_972e1,
        -1.328_068_155_288_572e1,
    ];
    const C: [f64; 6] = [
        -7.784_894_002_430_293e-3,
        -3.223_964_580_411_365e-1,
        -2.400_758_277_161_838,
        -2.549_732_539_343_734,
        4.374_664_141_464_968,
        2.938_163_982_698_783,
    ];
    const D: [f64; 4] = [
        7.784_695_709_041_462e-3,
        3.224_671_290_700_398e-1,
        2.445_134_137_142_996,
        3.754_408_661_907_416,
    ];
    const P_LOW: f64 = 0.024_25;
    let p = p.clamp(f64::MIN_POSITIVE, 1.0 - f64::EPSILON);
    let tail = |q: f64| {
        (((((C[0] * q + C[1]) * q + C[2]) * q + C[3]) * q + C[4]) * q + C[5])
            / ((((D[0] * q + D[1]) * q + D[2]) * q + D[3]) * q + 1.0)
    };
    if p < P_LOW {
        tail((-2.0 * p.ln()).sqrt())
    } else if p > 1.0 - P_LOW {
        -tail((-2.0 * (1.0 - p).ln()).sqrt())
    } else {
        let q = p - 0.5;
        let r = q * q;
        (((((A[0] * r + A[1]) * r + A[2]) * r + A[3]) * r + A[4]) * r + A[5]) * q
            / (((((B[0] * r + B[1]) * r + B[2]) * r + B[3]) * r + B[4]) * r + 1.0)
    }
}

fn population_draw(genetics: &GeneticsSnapshot, trait_name: &str, (mean, sd): (f64, f64)) -> f64 {
    mean + sd * genome_normal_quantile(genetics, trait_name)
}

/// Puberty onset age (years) for a human: the canon's own
/// `pubertal_hormones.timing.onset_age` when it is given, otherwise this
/// individual's place in their sex's population distribution.
pub fn puberty_onset_years(human: &HumanBeing) -> f64 {
    if let Some(onset) = human
        .profile
        .canonical_schema()
        .map(|schema| {
            schema
                .extreme_brain_detail
                .brain_dynamics_module
                .developmental_timeline
                .hormonal_modulation
                .pubertal_hormones
                .timing
                .onset_age
        })
        .filter(|onset| *onset > 0.0)
    {
        return f64::from(onset);
    }
    let female = population_draw(&human.genetics, "puberty-onset", FEMALE_PUBERTY_ONSET);
    let male = population_draw(&human.genetics, "puberty-onset", MALE_PUBERTY_ONSET);
    match human.biological_sex() {
        BiologicalSex::Female => female,
        BiologicalSex::Male => male,
        BiologicalSex::Neutral => (female + male) / 2.0,
    }
}

/// This human's reproductive timeline: when their gametes mature, when
/// fecundability starts to fall, and — for women — when fertility ends.
pub fn reproductive_timeline(human: &HumanBeing) -> ReproductiveTimeline {
    let onset = puberty_onset_years(human);
    let female_sterility = || {
        // Never before menarche plus the first fertile years.
        population_draw(&human.genetics, "sterility-age", FEMALE_STERILITY_AGE)
            .max(onset + MENARCHE_AFTER_ONSET_YEARS + OVULATORY_MATURATION_YEARS)
    };
    match human.biological_sex() {
        BiologicalSex::Female => ReproductiveTimeline {
            gametogenesis_onset_years: onset + MENARCHE_AFTER_ONSET_YEARS,
            maturation_time_constant_years: OVULATORY_MATURATION_YEARS,
            decline_onset_years: FEMALE_DECLINE_ONSET_YEARS,
            decline_half_life_years: FEMALE_DECLINE_HALF_LIFE_YEARS,
            sterility_years: Some(female_sterility()),
        },
        BiologicalSex::Male => ReproductiveTimeline {
            gametogenesis_onset_years: onset + SPERMARCHE_AFTER_ONSET_YEARS,
            maturation_time_constant_years: SPERMATOGENIC_MATURATION_YEARS,
            decline_onset_years: MALE_DECLINE_ONSET_YEARS,
            decline_half_life_years: MALE_DECLINE_HALF_LIFE_YEARS,
            sterility_years: None,
        },
        // Neither sex's reproductive physiology: the midpoint of both.
        BiologicalSex::Neutral => ReproductiveTimeline {
            gametogenesis_onset_years: onset
                + (MENARCHE_AFTER_ONSET_YEARS + SPERMARCHE_AFTER_ONSET_YEARS) / 2.0,
            maturation_time_constant_years: (OVULATORY_MATURATION_YEARS
                + SPERMATOGENIC_MATURATION_YEARS)
                / 2.0,
            decline_onset_years: (FEMALE_DECLINE_ONSET_YEARS + MALE_DECLINE_ONSET_YEARS) / 2.0,
            decline_half_life_years: (FEMALE_DECLINE_HALF_LIFE_YEARS
                + MALE_DECLINE_HALF_LIFE_YEARS)
                / 2.0,
            sterility_years: None,
        },
    }
}

/// Fraction of the population that is infertile regardless of age — real
/// human fertility is not universal even within the age window; some
/// individuals of either sex are never able to conceive.
const INFERTILITY_RATE: f64 = 0.12;

/// Deterministic, genome-derived fertility flag, independent of age.
///
/// Fallback used when the canonical schema's own
/// `reproductive_systems.reproduction_system` data hasn't been populated
/// (it defaults to `fertility_level: 0.0`, `status: None` for most fixtures
/// today). This is a pure function of already-existing heritable traits
/// (not a new RNG draw), so it is stable under replay: the same genome
/// always yields the same fertility outcome. [`INFERTILITY_RATE`] of genomes
/// fall below the infertility threshold of their keyed genome uniform.
fn is_fertile_from_genetics(genetics: &GeneticsSnapshot) -> bool {
    genome_uniform(genetics, "infertility") >= INFERTILITY_RATE
}

/// Whether a human can conceive at all, independent of age.
///
/// Prefers the canonical `docs/canon/HumanReplicationSchema.js`-derived
/// `reproductive_systems.reproduction_system` data when it carries real
/// signal (an explicit `Infertile` status, or a nonzero `fertility_level`),
/// since that is the actual source of truth for this trait. Falls back to
/// [`is_fertile_from_genetics`] only when that canon data is unpopulated.
pub fn is_fertile(human: &HumanBeing) -> bool {
    if let Some(schema) = human.profile.canonical_schema() {
        let repro = &schema.reproductive_systems.reproduction_system;
        if matches!(
            repro.status,
            Some(mk_core::human::schema::ReproductionStatusSchema::Infertile)
        ) {
            return false;
        }
        if repro.fertility_level > 0.0 {
            return repro.fertility_level > 0.05;
        }
    }
    is_fertile_from_genetics(&human.genetics)
}

/// A newborn's `agent_id`: fixed length whatever the generation, unique per
/// (mother, father, tick), and reproducible. Parentage lives in the birth
/// record, not the id — ids built by concatenating parent ids grew with every
/// generation and would overflow the storage directory-name limit.
pub(crate) fn child_agent_id(mother_id: &str, father_id: &str, tick: u64) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(mother_id.as_bytes());
    hasher.update(&[0]);
    hasher.update(father_id.as_bytes());
    hasher.update(&tick.to_le_bytes());
    let digest = hasher.finalize();
    format!("born_t{tick}_{}", &digest.to_hex()[..12])
}

/// Whether a human has reached sexual maturity, not yet aged out of
/// fertility, and is not one of the individuals who — independent of age —
/// cannot conceive. All three checks are genome/canon/age-derived, never a
/// universal cutoff applied identically to everyone.
pub fn is_reproductively_eligible(human: &HumanBeing) -> bool {
    matches!(human.profile.status, HumanStatus::Alive)
        && is_fertile(human)
        && reproductive_timeline(human).fecundity(human.development.age_years) > 0.0
}

/// Process lifecycle for a human being: aging, needs, and death (Gompertz
/// age mortality up to a hard biological maximum, unmet survival needs —
/// starvation, dehydration — or lethal core temperature from exposure).
pub fn step_lifecycle(
    human: &mut HumanBeing,
    dt_years: f64,
    tick: u64,
    observation: &super::AgentWorldObservation,
    rng_registry: &mk_core::rng::RngRegistry,
    movement_delta: (i32, i32),
) {
    if matches!(human.profile.status, HumanStatus::Dead) {
        return;
    }

    // Age the human and recompute every age-derived development curve.
    human.development.step(dt_years);
    human.refresh_phase11_layers();

    // Physical capacity reads last tick's proprioception/body/immune
    // (before any of them are re-stepped below) — see physical_capacity.rs
    // module docs for why this 1-tick lag is deliberate, not an oversight.
    human.physical_capacity = super::physical_capacity::PhysicalCapacitySnapshot::step(
        &human.proprioception,
        &human.body,
        &human.immune,
    );
    // Active resource-seeking effort reads the autonomous action selected at
    // the start of this tick. The action is selected before needs evolve so
    // its embodied consequence is attributable to this human's choice.
    // A human whose last action was actively seeking a resource converts
    // identical environmental access into more of that specific resource
    // than one who wasn't trying — see needs.rs::EffortFocus docs.
    let effort = match human.economy_action.kind {
        super::ActionKind::SeekFood | super::ActionKind::Gather => super::needs::EffortFocus {
            food: 1.5,
            water: 1.0,
            shelter: 1.0,
        },
        super::ActionKind::SeekWater => super::needs::EffortFocus {
            food: 1.0,
            water: 1.5,
            shelter: 1.0,
        },
        super::ActionKind::SeekShelter | super::ActionKind::Rest | super::ActionKind::Build => {
            super::needs::EffortFocus {
                food: 1.0,
                water: 1.0,
                shelter: 1.5,
            }
        }
        _ => super::needs::EffortFocus::none(),
    };
    human.needs = human.needs.step(
        observation,
        human.physical_capacity.effectiveness,
        effort,
        dt_years,
    );
    consume_carried_rations(human);
    human.body = human.body.step(
        &human.needs,
        dt_years,
        super::body::BodyEnvironment {
            shelter_quality: observation.shelter_quality,
            ambient_temperature_c: observation.ambient_temperature_c,
            inflammation: human.immune.inflammation,
        },
    );
    human.immune = human.immune.step(
        &human.needs,
        human.profile.human_id,
        tick,
        rng_registry,
        dt_years,
    );
    human.proprioception = human.proprioception.step(
        &human.needs,
        &human.immune,
        &human.development,
        dt_years,
        movement_delta,
    );
    human.skin = human
        .skin
        .step(human.body.hygiene, human.immune.system_stress, dt_years);
    human.sensory = human
        .sensory
        .step(&human.body, &human.needs, observation, dt_years);
    human.attention = human.attention.step(&human.needs, dt_years);
    // `neurochemistry` lags one tick here (stepped later this same
    // function) — same one-tick-lag pattern already used elsewhere in this
    // pipeline (e.g. `dark_triad` reading `emotion` before its own step).
    human.memory = human.memory.step(
        &human.attention,
        human.neurochemistry.hormones.melatonin,
        dt_years,
    );
    human.learning = human.learning.step(
        &human.memory,
        &human.development,
        human
            .neurochemistry
            .modulation_effects
            .learning_rate_multiplier,
        dt_years,
    );
    human.decision = human.decision.step(
        &human.cognition,
        &human.learning,
        &human.sensory,
        &human.immune,
        dt_years,
    );
    human.social_cognition = human.social_cognition.step(&human.attention);
    human.creativity = human.creativity.step(&human.attention, &human.learning);
    let timeline = reproductive_timeline(human);
    human.reproduction = human.reproduction.step(
        human.development.age_years,
        &timeline,
        !is_fertile_from_genetics(&human.genetics),
        &human.needs,
        &human.social_cognition,
        dt_years,
    );
    human.emotion = human.emotion.step(
        &human.needs,
        &human.social_cognition,
        &human.creativity,
        &human.attention,
        &human.sensory,
        &human.immune,
        dt_years,
    );
    human.advanced_memory = human.advanced_memory.step(
        &human.memory,
        &human.attention,
        &human.emotion,
        &human.autonomous_mind.experiences,
    );
    human.dark_triad = human.dark_triad.step(&human.emotion);
    human.brain_regions = human.brain_regions.step(
        &human.decision,
        &human.emotion,
        &human.memory,
        &human.learning,
        &human.needs,
        dt_years,
    );
    human.population_dynamics = human.population_dynamics.step(&human.brain_regions);
    human.neurochemistry = human.neurochemistry.step(
        &human.emotion,
        &human.needs,
        &human.immune,
        &human.attention,
        &human.social_cognition,
        &human.dark_triad,
        &human.reproduction,
        observation,
        dt_years,
    );
    human.comprehensive_emotion = human.comprehensive_emotion.step(
        &human.emotion.current,
        &human.neurochemistry,
        &human.reproduction,
        dt_years,
    );
    human.core_systems = human.core_systems.step(
        human.profile.human_id,
        tick,
        &human.needs,
        &human.emotion,
        &human.social_cognition,
        &human.attention,
        rng_registry,
        dt_years,
    );
    human.attractor_control = human.attractor_control.step(
        &human.needs,
        &human.sensory,
        &human.immune,
        &human.emotion,
        dt_years,
    );
    human.pathology = human.pathology.step(
        &human.attractor_control,
        &human.emotion,
        &human.sensory,
        &human.immune,
        &human.development,
        dt_years,
    );
    human.formal_predictive_processing = human.formal_predictive_processing.step(
        human.profile.human_id,
        tick,
        observation,
        &human.needs,
        &human.core_systems,
        rng_registry,
        dt_years,
    );
    human.mesoscale_brain = super::mesoscale_brain::MesoscaleBrainSnapshot::from_layers(
        &human.profile,
        &human.brain_regions,
        &human.population_dynamics,
        &human.neurochemistry,
    );

    // Reverence Veto: creator-designated founder entities cannot be killed
    // by ordinary lifecycle mechanics either — see `crate::governance`. This
    // module previously bypassed the veto entirely because it never called
    // it; the veto's only prior caller, `HumanRegistry::remove_human`, has
    // no callers anywhere in the engine, so natural death was the actual
    // (unguarded) removal path for a protected entity.
    if crate::governance::check(human.agent_id()).is_err() {
        return;
    }

    let age = human.age();
    let age_years = human.development.age_years;
    let reason =
        if age_years > MAX_HUMAN_AGE_YEARS || dies_of_age(human, dt_years, tick, rng_registry) {
            Some("natural_age")
        } else if human.needs.glucose <= 0.0 {
            Some("starvation")
        } else if human.needs.hydration <= 0.0 {
            Some("dehydration")
        } else if human.body.body_temperature_c <= super::body::LETHAL_HYPOTHERMIA_C {
            Some("hypothermia")
        } else if human.body.body_temperature_c >= super::body::LETHAL_HYPERTHERMIA_C {
            Some("hyperthermia")
        } else {
            None
        };
    if let Some(reason) = reason {
        human.profile.status = HumanStatus::Dead;
        human.death_reason = Some(reason.to_string());
        // Emit death event to the universe's fossil record
        crate::io::global_events::log_human_died(tick, human.agent_id(), age, Some(reason));
    }
}

/// Gompertz baseline hazard (per year) extrapolated to age 0, and its
/// exponential rate of increase with age: human adult mortality doubles
/// roughly every 7.5 years, giving a realistic spread of lifespans instead
/// of everyone dying at one fixed age.
const GOMPERTZ_BASELINE_HAZARD_PER_YEAR: f64 = 5.0e-5;
const GOMPERTZ_AGING_RATE_PER_YEAR: f64 = 0.093;
/// Keyed RNG epoch for the per-step age-mortality roll.
const AGE_MORTALITY_EPOCH: u32 = 0x4147_4544;

/// Whether this human dies of age-related causes during the `dt_years`
/// just lived: the Gompertz cumulative hazard over the interval, rolled on
/// a keyed per-human stream.
fn dies_of_age(human: &HumanBeing, dt_years: f64, tick: u64, rng_registry: &RngRegistry) -> bool {
    let dt = dt_years.max(0.0);
    if dt == 0.0 {
        return false;
    }
    let end = human.development.age_years;
    let start = (end - dt).max(0.0);
    let (a, b) = (
        GOMPERTZ_BASELINE_HAZARD_PER_YEAR,
        GOMPERTZ_AGING_RATE_PER_YEAR,
    );
    let cumulative_hazard = a / b * ((b * end).exp() - (b * start).exp());
    let probability = 1.0 - (-cumulative_hazard).exp();
    let key = RngKey::new(
        SubsystemId::Humans,
        (human.profile.human_id.0 & 0xFFFF_FFFF) as u32,
        AGE_MORTALITY_EPOCH,
        tick,
    );
    rng_registry.gen_f64_01(key) < probability
}

/// Reserve (0..1 glucose or hydration) one carried `Food` / `Water` item
/// restores when eaten or drunk.
const CARRIED_RATION_RESERVE: f64 = 0.1;
/// Reserve level below which a human eats or drinks from what they carry.
const RATION_CONSUMPTION_THRESHOLD: f64 = 0.6;

/// Eat and drink from carried provisions when reserves run low. Gathered
/// `Food`/`Water` items are otherwise inert inventory: without this a human
/// could starve while carrying food.
fn consume_carried_rations(human: &mut HumanBeing) {
    use crate::agents::ItemKind;
    while human.needs.glucose < RATION_CONSUMPTION_THRESHOLD
        && human.carrying.remove(ItemKind::Food, 1) == 1
    {
        human.needs.glucose = (human.needs.glucose + CARRIED_RATION_RESERVE).min(1.0);
    }
    while human.needs.hydration < RATION_CONSUMPTION_THRESHOLD
        && human.carrying.remove(ItemKind::Water, 1) == 1
    {
        human.needs.hydration = (human.needs.hydration + CARRIED_RATION_RESERVE).min(1.0);
    }
    human.needs.hunger = (1.0 - human.needs.glucose).clamp(0.0, 1.0);
    human.needs.thirst = (1.0 - human.needs.hydration).clamp(0.0, 1.0);
}

/// Conceive a child from two living, fertile, opposite-sex humans (either
/// argument order) using the genetics pipeline: meiosis in each parent,
/// fertilisation, and inheritance of parental traits.
///
/// The returned embryo has its genome, sex, inherited traits and conception
/// record fixed, but no birth instant, birthplace, position or birth chart
/// yet — those belong to the moment and place of delivery
/// ([`deliver_birth`]). Whether conception happens at all is decided by
/// [`attempt_conception`]; this function only performs it.
pub fn conceive_embryo(
    parent_a: &HumanBeing,
    parent_b: &HumanBeing,
    rng_registry: &RngRegistry,
    tick: mk_core::time::Tick,
) -> Option<HumanBeing> {
    if matches!(parent_a.profile.status, HumanStatus::Dead)
        || matches!(parent_b.profile.status, HumanStatus::Dead)
    {
        return None;
    }

    if !is_fertile(parent_a) || !is_fertile(parent_b) {
        return None;
    }

    match (parent_a.biological_sex(), parent_b.biological_sex()) {
        (BiologicalSex::Male, BiologicalSex::Female)
        | (BiologicalSex::Female, BiologicalSex::Male) => {
            let pair_chunk = parent_pair_chunk(parent_a, parent_b);
            let rng_key = RngKey::new(SubsystemId::Humans, pair_chunk, GAMETE_EPOCH, tick);
            let mut rng = rng_registry.stream(rng_key);

            // The egg comes from the female parent and the sperm from the
            // male, whichever order the pair was passed in, so the child's
            // `maternal`/`paternal` DNA and sex chromosomes are labelled by
            // their real origin.
            let (mother, father) = if matches!(parent_a.biological_sex(), BiologicalSex::Female) {
                (parent_a, parent_b)
            } else {
                (parent_b, parent_a)
            };

            // Create an f32 RNG adapter
            let mut rng_f32 = move || rng.gen_f64_01() as f32;

            // Meiosis: create gametes from each parent
            let maternal_gamete =
                mk_core::human::create_gamete(&mother.profile.genome, &mut rng_f32);
            let paternal_gamete =
                mk_core::human::create_gamete(&father.profile.genome, &mut rng_f32);

            // Conception: fuse gametes into new genome
            let child_genome = mk_core::human::conceive(&maternal_gamete, &paternal_gamete);

            // Determine sex from combined sex chromosomes
            let sex = if child_genome.sex_chromosomes.is_male() {
                BiologicalSex::Male
            } else {
                BiologicalSex::Female
            };

            let child_id = child_agent_id(mother.agent_id(), father.agent_id(), tick);

            // Determine mother/father by sex for lineage-record fields.
            let (mother_id, father_id) =
                (mother.agent_id().to_string(), father.agent_id().to_string());
            let total_mutations = maternal_gamete.mutation_count + paternal_gamete.mutation_count;

            // Create child with inherited genome
            let mut child = HumanBeing::new(child_id.clone(), sex);
            child.profile.genome = child_genome;
            // Separate epoch from the gamete stream above so personality
            // mutation draws are independent of genome mutation draws for
            // the same pair and tick, while staying keyed/deterministic.
            let mutation_rng_key =
                RngKey::new(SubsystemId::Humans, pair_chunk, TRAIT_MUTATION_EPOCH, tick);
            let mut mutation_rng = rng_registry.stream(mutation_rng_key);
            inherit_parent_traits(&mut child, parent_a, parent_b, &mut mutation_rng);

            // Write real lineage history into the child's own canonical
            // schema (previously always empty regardless of how many
            // generations of reproduction occurred — see
            // `reproduction.rs`'s ReproductiveSystemSnapshot, which surfaces
            // exactly this data).
            if let Some(child_schema) = child.profile.canonical_schema.as_mut() {
                let genetics_system = &mut child_schema.reproductive_systems.genetics_system;

                genetics_system.conception_history.push(
                    mk_core::human::schema::ConceptionHistorySchema {
                        genotype_id: format!("genotype_{child_id}"),
                        father_id: father_id.clone(),
                        mother_id: mother_id.clone(),
                        conception_timestamp: format!("tick-{tick}"),
                        mutations: vec![serde_json::json!({ "count": total_mutations })],
                    },
                );
            }

            child.refresh_phase11_layers();

            crate::io::global_events::log_human_reproduced(tick, &child_id, &mother_id, &father_id);

            Some(child)
        }
        _ => None,
    }
}

/// Give a newly conceived child its inherited parental traits: the blend
/// covers personality, cognition, drives, stress, attachment, relationships,
/// canonical systems and generation. The birth chart is assigned separately
/// at delivery ([`assign_birth_chart`]).
fn inherit_parent_traits(
    child: &mut HumanBeing,
    parent_a: &HumanBeing,
    parent_b: &HumanBeing,
    mutation_rng: &mut impl mk_core::rng::RngExt,
) {
    let mut child_value = serde_json::to_value(&child.profile).expect("profile must serialize");
    let parent_a_value = serde_json::to_value(&parent_a.profile).expect("profile must serialize");
    let parent_b_value = serde_json::to_value(&parent_b.profile).expect("profile must serialize");

    for section in [
        "temperament_matrix",
        "neurocognitive_profile",
        "drive_weights",
        "hormonal_baseline_bias",
        "stress_response_profile",
        "attachment_style",
        "relational_defaults",
        "identity",
        "canonical_schema",
    ] {
        if let (Some(a), Some(b)) = (parent_a_value.get(section), parent_b_value.get(section)) {
            let mut inherited = child_value.get(section).cloned().unwrap_or(Value::Null);
            blend_numeric_values(&mut inherited, a, b, mutation_rng);
            child_value[section] = inherited;
        }
    }

    child.profile = serde_json::from_value(child_value).expect("inherited profile must be valid");

    let parent_generation = |generation: &mk_core::human::Generation| -> u8 {
        match generation {
            mk_core::human::Generation::First => 1,
            mk_core::human::Generation::Second => 2,
            mk_core::human::Generation::Third => 3,
            mk_core::human::Generation::Later(value) => *value,
        }
    };
    let generation = parent_generation(&parent_a.profile.core_identity.generation)
        .max(parent_generation(
            &parent_b.profile.core_identity.generation,
        ))
        .saturating_add(1);
    child.profile.core_identity.generation = mk_core::human::Generation::from(generation);
}

/// Compute and store the newborn's birth chart, birth timestamp and
/// birthplace from the real instant and place of delivery.
fn assign_birth_chart(
    child: &mut HumanBeing,
    birth_datetime: chrono::DateTime<Utc>,
    coordinates: mk_core::human::astrology::GeoCoordinates,
    tick: mk_core::time::Tick,
) {
    let child_generation = generation_number(&child.profile.core_identity.generation);
    match mk_core::human::astrology::AstrologyEngine::compute_birth_chart(
        birth_datetime,
        coordinates,
    ) {
        Ok(chart) => {
            let timestamp = chart.birth_timestamp.clone();
            child.profile.core_identity.birth_timestamp = timestamp.clone();
            child.profile.core_identity.birthplace.coordinates = mk_core::human::Coordinates {
                latitude: coordinates.latitude,
                longitude: coordinates.longitude,
            };
            child.profile.created_at = timestamp.clone();
            child.profile.metadata.created_at = timestamp.clone();
            child.profile.metadata.last_updated = timestamp.clone();
            let birth_season = mk_core::human::astrology::AstrologyEngine::calculate_birth_season(
                birth_datetime,
                coordinates.latitude,
            );
            let lunar_phase =
                mk_core::human::astrology::AstrologyEngine::calculate_lunar_phase(birth_datetime);
            child.profile.genome.astrology = mk_core::human::AstrologicalProfile::new(
                zodiac_from_chart(chart.sun),
                zodiac_from_chart(chart.moon),
                zodiac_from_chart(chart.ascendant),
            )
            .with_derived_metadata(birth_season, lunar_phase);

            if let Some(schema) = child.profile.canonical_schema.as_mut() {
                schema.core_identity.generation = child_generation;
                schema.core_identity.birth_timestamp = timestamp.clone();
                schema.core_identity.birthplace.coordinates.latitude = coordinates.latitude;
                schema.core_identity.birthplace.coordinates.longitude = coordinates.longitude;
                schema.birth_chart.sun = format!("{:?}", chart.sun);
                schema.birth_chart.moon = format!("{:?}", chart.moon);
                schema.birth_chart.ascendant = format!("{:?}", chart.ascendant);
                schema.birth_chart.birth_timestamp = timestamp;
                schema.birth_chart.coordinates.latitude = coordinates.latitude;
                schema.birth_chart.coordinates.longitude = coordinates.longitude;
            }
        }
        Err(e) => {
            // `conceive()` already gave the child a deterministic
            // Libra/Libra/Libra scaffold astrology; this branch is reached
            // only if the birth chart itself can't be computed (e.g.
            // averaged parent coordinates fail range validation). Leaving
            // that scaffold in place is the correct deterministic fallback
            // — but it must be observable, not silent, so a real chart
            // failure is never mistaken for a genuinely Libra-born child.
            warn!(
                child_id = %child.agent_id(),
                tick,
                error = %e,
                "birth chart computation failed; child keeps scaffold Libra/Libra/Libra astrology"
            );
        }
    }
}

/// Keyed RNG epochs for reproduction, distinct from every other
/// `SubsystemId::Humans` consumer (the per-urge chaos epochs `0..=4` in
/// `core_systems` would otherwise collide with a fixed chunk/epoch here).
const GAMETE_EPOCH: u32 = 0x4741_4D45;
const TRAIT_MUTATION_EPOCH: u32 = 0x4D55_5441;

/// Order-independent RNG chunk for a parent pair, so two pairs conceiving on
/// the same tick draw independent gametes instead of sharing one stream.
fn parent_pair_chunk(parent_a: &HumanBeing, parent_b: &HumanBeing) -> u32 {
    let (a, b) = (parent_a.profile.human_id.0, parent_b.profile.human_id.0);
    let (low, high) = (a.min(b), a.max(b));
    let mixed = low.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ high.rotate_left(29);
    (mixed ^ (mixed >> 32)) as u32
}

/// Half-width of the additive mutation applied to blended floating-point
/// trait values (personality, drives, etc.), e.g. `0.02` means the
/// parental average is offset by a value drawn uniformly from
/// `[-0.02, 0.02]`. Additive (not multiplicative) so a trait sitting at
/// exactly `0.0` — a legitimate value for these mostly-`[0,1]`-ranged
/// fields — still gets a real mutation draw instead of `0.0 * factor == 0.0`.
/// Mirrors, at trait scale, the real keyed-RNG mutation already applied to
/// gamete-level genome traits a few hundred lines above in this file —
/// without this, personality converges toward the population mean over
/// generations (pure averaging has no diversifying force) while genome
/// traits keep real variance. Integer fields (counts, ids) are left
/// exactly averaged, unmutated, matching prior behavior.
const PERSONALITY_MUTATION_HALF_WIDTH: f64 = 0.02;

fn blend_numeric_values(
    target: &mut Value,
    a: &Value,
    b: &Value,
    rng: &mut impl mk_core::rng::RngExt,
) {
    if let (Value::Object(target_object), Value::Object(a_object), Value::Object(b_object)) =
        (&mut *target, a, b)
    {
        for (key, value) in target_object.iter_mut() {
            if let (Some(a_value), Some(b_value)) = (a_object.get(key), b_object.get(key)) {
                blend_numeric_values(value, a_value, b_value, rng);
            }
        }
    } else if let (Value::Number(a), Value::Number(b)) = (a, b) {
        if let (Some(a), Some(b)) = (a.as_u64(), b.as_u64()) {
            *target = serde_json::json!((a + b) / 2);
            return;
        }
        if let (Some(a), Some(b)) = (a.as_f64(), b.as_f64()) {
            let average = (a + b) / 2.0;
            let noise = (rng.gen_f64_01() * 2.0 - 1.0) * PERSONALITY_MUTATION_HALF_WIDTH;
            *target = serde_json::json!(average + noise);
        }
    }
}

fn zodiac_from_chart(sign: mk_core::human::astrology::ZodiacSign) -> mk_core::human::Zodiac {
    match sign {
        mk_core::human::astrology::ZodiacSign::Aries => mk_core::human::Zodiac::Aries,
        mk_core::human::astrology::ZodiacSign::Taurus => mk_core::human::Zodiac::Taurus,
        mk_core::human::astrology::ZodiacSign::Gemini => mk_core::human::Zodiac::Gemini,
        mk_core::human::astrology::ZodiacSign::Cancer => mk_core::human::Zodiac::Cancer,
        mk_core::human::astrology::ZodiacSign::Leo => mk_core::human::Zodiac::Leo,
        mk_core::human::astrology::ZodiacSign::Virgo => mk_core::human::Zodiac::Virgo,
        mk_core::human::astrology::ZodiacSign::Libra => mk_core::human::Zodiac::Libra,
        mk_core::human::astrology::ZodiacSign::Scorpio => mk_core::human::Zodiac::Scorpio,
        mk_core::human::astrology::ZodiacSign::Sagittarius => mk_core::human::Zodiac::Sagittarius,
        mk_core::human::astrology::ZodiacSign::Capricorn => mk_core::human::Zodiac::Capricorn,
        mk_core::human::astrology::ZodiacSign::Aquarius => mk_core::human::Zodiac::Aquarius,
        mk_core::human::astrology::ZodiacSign::Pisces => mk_core::human::Zodiac::Pisces,
    }
}

fn generation_number(generation: &mk_core::human::Generation) -> u32 {
    match generation {
        mk_core::human::Generation::First => 1,
        mk_core::human::Generation::Second => 2,
        mk_core::human::Generation::Third => 3,
        mk_core::human::Generation::Later(value) => u32::from(*value),
    }
}

/// Keyed RNG epoch for the per-act conception roll.
const CONCEPTION_EPOCH: u32 = 0x434F_4E43;

/// Resolve whether one consensual act of intercourse between `mother` and
/// `father` conceives. Both must be living, reproductively eligible
/// (fertile, within their own genome-derived fertile age window) and of
/// opposite sex; the chance is the mother's cycle-aware
/// [`super::reproduction::ReproductiveSystemSnapshot::conception_chance`]
/// against the father's fertility, rolled on a keyed per-pair stream. On
/// success the conceived embryo starts gestating in `mother`.
pub fn attempt_conception(
    mother: &mut HumanBeing,
    father: &HumanBeing,
    dt_years: f64,
    rng_registry: &RngRegistry,
    tick: mk_core::time::Tick,
) -> bool {
    if !matches!(mother.biological_sex(), BiologicalSex::Female)
        || !matches!(father.biological_sex(), BiologicalSex::Male)
        || !is_reproductively_eligible(mother)
        || !is_reproductively_eligible(father)
    {
        return false;
    }
    let chance = mother
        .reproduction
        .conception_chance_over(father.reproduction.fertility_level, dt_years * 365.25);
    if chance <= 0.0 {
        return false;
    }
    let key = RngKey::new(
        SubsystemId::Humans,
        parent_pair_chunk(mother, father),
        CONCEPTION_EPOCH,
        tick,
    );
    if rng_registry.gen_f64_01(key) >= chance {
        return false;
    }
    let Some(embryo) = conceive_embryo(mother, father, rng_registry, tick) else {
        return false;
    };
    mother.reproduction.pregnancy = Some(super::reproduction::Pregnancy {
        father_id: father.agent_id().to_string(),
        conceived_tick: tick,
        gestation_years: 0.0,
        embryo: Box::new(embryo),
    });
    true
}

/// The in-world instant `mother` is living at right now: her own birth
/// instant plus her simulated age. Every human's age advances by exactly the
/// simulated `dt_years` each tick, so this is the world's real elapsed time
/// on the calendar the founders' birth timestamps anchor.
pub(crate) fn current_instant_of(mother: &HumanBeing) -> chrono::DateTime<Utc> {
    let born = chrono::DateTime::parse_from_rfc3339(&mother.profile.core_identity.birth_timestamp)
        .map(|t| t.with_timezone(&Utc))
        .unwrap_or_else(|error| {
            warn!(
                mother_id = %mother.agent_id(),
                timestamp = %mother.profile.core_identity.birth_timestamp,
                %error,
                "unparseable birth timestamp; anchoring the child's birth to the Unix epoch"
            );
            chrono::DateTime::<Utc>::UNIX_EPOCH
        });
    let lived_seconds = (mother.development.age_years.max(0.0) * 365.25 * 24.0 * 3600.0).round();
    born + chrono::Duration::seconds(lived_seconds.min(i64::MAX as f64) as i64)
}

/// Deliver a full-term pregnancy: the child is born now, where the mother
/// stands, with its birth chart computed from that real instant and place.
pub fn deliver_birth(
    mother: &HumanBeing,
    pregnancy: super::reproduction::Pregnancy,
    tick: mk_core::time::Tick,
    grid_spec: &mk_core::grid::GridSpec,
) -> HumanBeing {
    let mut child = *pregnancy.embryo;
    let (latitude, longitude) = super::grid_position_to_birthplace(mother.position, grid_spec);
    let coordinates = mk_core::human::astrology::GeoCoordinates {
        latitude,
        longitude,
    };
    assign_birth_chart(&mut child, current_instant_of(mother), coordinates, tick);
    child.set_runtime_position(mother.position);

    let child_id = child.agent_id().to_string();
    let birth_timestamp = child.profile.core_identity.birth_timestamp.clone();
    if let Some(child_schema) = child.profile.canonical_schema.as_mut() {
        let genetics_system = &mut child_schema.reproductive_systems.genetics_system;
        let mutations = genetics_system
            .conception_history
            .last()
            .map(|conception| conception.mutations.clone())
            .unwrap_or_default();
        genetics_system
            .birth_records
            .push(mk_core::human::schema::BirthRecordSchema {
                birth_id: format!("birth_{child_id}_tick{tick}"),
                genotype_id: format!("genotype_{child_id}"),
                father_id: pregnancy.father_id.clone(),
                mother_id: mother.agent_id().to_string(),
                birth_timestamp,
                agent_id: child_id.clone(),
                mutations,
            });
    }
    child.refresh_phase11_layers();
    // A newborn has no lived reproductive state yet, so its snapshot is
    // re-derived to surface the conception and birth records just written.
    child.reproduction =
        super::reproduction::ReproductiveSystemSnapshot::from_profile(&child.profile);

    let sex_str = match child.biological_sex() {
        BiologicalSex::Male => "male",
        BiologicalSex::Female => "female",
        BiologicalSex::Neutral => "neutral",
    };
    let generation = generation_number(&child.profile.core_identity.generation);
    crate::io::global_events::log_human_born(tick, &child_id, sex_str, generation);
    child
}

/// Per-tick birth pass: every living mother whose pregnancy has reached
/// full term delivers, in stable registry order.
pub fn deliver_due_births(
    registry: &mut HumanRegistry,
    tick: mk_core::time::Tick,
    grid_spec: &mk_core::grid::GridSpec,
) {
    let mut births = Vec::new();
    for mother in registry.get_all_humans_mut() {
        if !matches!(mother.profile.status, HumanStatus::Alive) {
            continue;
        }
        if let Some(pregnancy) = mother.reproduction.take_due_pregnancy() {
            let father_id = pregnancy.father_id.clone();
            let child = deliver_birth(mother, pregnancy, tick, grid_spec);
            births.push((child, mother.agent_id().to_string(), father_id));
        }
    }

    for (child, mother_id, father_id) in births {
        let child_id = child.agent_id().to_string();

        // `add_human` creates the child's personal folder when storage is
        // enabled (falls back to an in-memory push otherwise, same as
        // `add_human_no_storage`), so a birth always leaves a documented
        // record of the new human, matching the retired parents' own folders.
        if registry.add_human(child).is_ok() {
            let birth_event = serde_json::json!({
                "kind": "born",
                "tick": tick,
                "parents": [mother_id, father_id],
            });
            registry.record_event(&child_id, &birth_event);

            let reproduced_event = serde_json::json!({
                "kind": "reproduced",
                "tick": tick,
                "child": child_id,
            });
            registry.record_event(&mother_id, &reproduced_event);
            registry.record_event(&father_id, &reproduced_event);
        }
    }
}

/// Append one consensual act to `agent_id`'s own
/// `reproduction_system.sexual_activities` log, with its real conception
/// outcome and the grid cell where it happened.
pub fn record_sexual_activity(
    registry: &mut HumanRegistry,
    agent_id: &str,
    partner_id: &str,
    tick: mk_core::time::Tick,
    conception: Option<bool>,
) {
    let Some(human) = registry.get_human_mut(agent_id) else {
        return;
    };
    let location_id = format!("cell_{}_{}", human.position.row, human.position.col);
    let Some(schema) = human.profile.canonical_schema.as_mut() else {
        return;
    };

    use mk_core::human::schema::{ConceptionResultSchema, SexualActivityTypeSchema};
    let (activity_type, conception_result) = match conception {
        Some(true) => (
            SexualActivityTypeSchema::ReproductiveAttempt,
            ConceptionResultSchema::Successful,
        ),
        Some(false) => (
            SexualActivityTypeSchema::ReproductiveAttempt,
            ConceptionResultSchema::Failed,
        ),
        None => (
            SexualActivityTypeSchema::Intimate,
            ConceptionResultSchema::None,
        ),
    };
    schema
        .reproductive_systems
        .reproduction_system
        .sexual_activities
        .push(mk_core::human::schema::SexualActivitySchema {
            activity_id: format!("activity_{agent_id}_{partner_id}_tick{tick}"),
            participant1_id: agent_id.to_string(),
            participant2_id: partner_id.to_string(),
            location_id,
            start_time: format!("tick-{tick}"),
            end_time: Some(format!("tick-{tick}")),
            activity_type: Some(activity_type),
            mutual_consent: true,
            satisfaction: Vec::new(),
            biological_cost: Vec::new(),
            conception_attempted: conception.is_some(),
            conception_result: Some(conception_result),
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    use mk_core::human::schema::HumanSchema;

    #[test]
    fn the_normal_quantile_inverts_the_normal_cdf() {
        assert!(standard_normal_quantile(0.5).abs() < 1e-9);
        assert!((standard_normal_quantile(0.975) - 1.959_963_985).abs() < 1e-6);
        assert!((standard_normal_quantile(0.01) + 2.326_347_874).abs() < 1e-6);
        assert!((standard_normal_quantile(0.841_344_746) - 1.0).abs() < 1e-6);
    }

    fn mean_and_sd(values: &[f64]) -> (f64, f64) {
        let n = values.len() as f64;
        let mean = values.iter().sum::<f64>() / n;
        let var = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (n - 1.0);
        (mean, var.sqrt())
    }

    #[test]
    fn puberty_and_fertility_timing_follow_the_population_distributions() {
        let women: Vec<HumanBeing> = (0..400)
            .map(|i| HumanBeing::new(format!("woman-{i}"), BiologicalSex::Female))
            .collect();
        let men: Vec<HumanBeing> = (0..400)
            .map(|i| HumanBeing::new(format!("man-{i}"), BiologicalSex::Male))
            .collect();

        let (girls, girls_sd) =
            mean_and_sd(&women.iter().map(puberty_onset_years).collect::<Vec<_>>());
        let (boys, boys_sd) = mean_and_sd(&men.iter().map(puberty_onset_years).collect::<Vec<_>>());
        assert!((girls - 10.0).abs() < 0.3 && (girls_sd - 1.2).abs() < 0.2);
        assert!((boys - 11.6).abs() < 0.3 && (boys_sd - 1.1).abs() < 0.2);

        let sterility: Vec<f64> = women
            .iter()
            .map(|w| reproductive_timeline(w).sterility_years.unwrap())
            .collect();
        let (end, end_sd) = mean_and_sd(&sterility);
        assert!((end - 41.0).abs() < 0.8 && (end_sd - 4.0).abs() < 0.6);
        assert!(men
            .iter()
            .all(|m| reproductive_timeline(m).sterility_years.is_none()));

        // The same genome always matures on the same schedule.
        let again = HumanBeing::new("woman-7".to_string(), BiologicalSex::Female);
        assert_eq!(puberty_onset_years(&again), puberty_onset_years(&women[7]));
    }

    #[test]
    fn a_canon_onset_age_overrides_the_population_draw() {
        let mut human = HumanBeing::new("canon-onset".to_string(), BiologicalSex::Female);
        let mut schema = HumanSchema::canonical_minimal(human.agent_id());
        schema
            .extreme_brain_detail
            .brain_dynamics_module
            .developmental_timeline
            .hormonal_modulation
            .pubertal_hormones
            .timing
            .onset_age = 12.5;
        human.profile.canonical_schema = Some(schema);
        assert_eq!(puberty_onset_years(&human), 12.5);
        assert_eq!(
            reproductive_timeline(&human).gametogenesis_onset_years,
            12.5 + MENARCHE_AFTER_ONSET_YEARS
        );
    }

    #[test]
    fn an_older_man_can_still_father_children_but_an_older_woman_cannot_conceive() {
        let mut man = HumanBeing::new("elder-man".to_string(), BiologicalSex::Male);
        let mut woman = HumanBeing::new("elder-woman".to_string(), BiologicalSex::Female);
        man.development = super::super::development::DevelopmentSnapshot::new(62.0);
        woman.development = super::super::development::DevelopmentSnapshot::new(62.0);
        let man_timeline = reproductive_timeline(&man);
        assert!(man_timeline.fecundity(62.0) > 0.0);
        assert!(man_timeline.fecundity(62.0) < man_timeline.fecundity(30.0));
        assert_eq!(reproductive_timeline(&woman).fecundity(62.0), 0.0);
        assert!(!is_reproductively_eligible(&woman));
    }

    #[test]
    fn child_ids_stay_short_and_unique_across_generations() {
        let mut id = "Gem-D".to_string();
        for generation in 0..50u64 {
            id = child_agent_id(&id, "Gem-K", generation);
            assert!(id.len() < 40, "generation {generation} id {id} grew");
        }
        assert_ne!(child_agent_id("a", "b", 1), child_agent_id("a", "b", 2));
        assert_ne!(child_agent_id("a", "b", 1), child_agent_id("b", "a", 1));
        assert_eq!(child_agent_id("a", "b", 1), child_agent_id("a", "b", 1));
    }

    fn rng() -> RngRegistry {
        RngRegistry::new([7u8; 32])
    }

    fn benign_observation() -> super::super::AgentWorldObservation {
        super::super::AgentWorldObservation {
            caloric_access: 0.9,
            hydration_access: 0.9,
            shelter_quality: 0.9,
            ..super::super::AgentWorldObservation::default()
        }
    }

    #[test]
    fn step_lifecycle_ages_a_living_human() {
        let mut human = HumanBeing::new("ager".to_string(), BiologicalSex::Female);
        let start_age = human.development.age_years;

        step_lifecycle(&mut human, 5.0, 0, &benign_observation(), &rng(), (0, 0));

        assert!((human.development.age_years - (start_age + 5.0)).abs() < 1e-9);
        assert!(matches!(human.profile.status, HumanStatus::Alive));
    }

    #[test]
    fn step_lifecycle_kills_humans_past_max_age() {
        let mut human = HumanBeing::new("elder".to_string(), BiologicalSex::Male);

        step_lifecycle(
            &mut human,
            MAX_HUMAN_AGE_YEARS,
            42,
            &benign_observation(),
            &rng(),
            (0, 0),
        );

        assert!(matches!(human.profile.status, HumanStatus::Dead));
    }

    #[test]
    fn step_lifecycle_reverence_veto_prevents_natural_death_of_protected_entities() {
        // Regression test for the hostile-blind-spot-auditor finding
        // (2026-09-03): natural death (old age, starvation) previously
        // bypassed `crate::governance` entirely, so a protected
        // creator-designated entity could die from ordinary lifecycle
        // mechanics even though `remove_human` refuses to remove them.
        for name in ["Gem-D", "gem-d", "Gem-K"] {
            let mut human = HumanBeing::new(name.to_string(), BiologicalSex::Female);

            // Past the max age cap — would normally kill from old age.
            step_lifecycle(
                &mut human,
                MAX_HUMAN_AGE_YEARS,
                42,
                &benign_observation(),
                &rng(),
                (0, 0),
            );
            assert!(
                matches!(human.profile.status, HumanStatus::Alive),
                "{name} must not die of old age under the Reverence Veto"
            );

            // Lethal starvation — would normally kill from unmet needs.
            human.needs.glucose = 0.0;
            step_lifecycle(&mut human, 0.0, 43, &benign_observation(), &rng(), (0, 0));
            assert!(
                matches!(human.profile.status, HumanStatus::Alive),
                "{name} must not die of starvation under the Reverence Veto"
            );
        }
    }

    #[test]
    fn step_lifecycle_is_a_no_op_for_dead_humans() {
        let mut human = HumanBeing::new("already_dead".to_string(), BiologicalSex::Male);
        human.profile.status = HumanStatus::Dead;
        let age_before = human.development.age_years;

        step_lifecycle(&mut human, 10.0, 0, &benign_observation(), &rng(), (0, 0));

        assert!((human.development.age_years - age_before).abs() < 1e-9);
    }

    #[test]
    fn step_lifecycle_kills_from_starvation_without_food_access() {
        let mut human = HumanBeing::new("starving".to_string(), BiologicalSex::Female);
        let starved = super::super::AgentWorldObservation {
            caloric_access: 0.0,
            hydration_access: 0.0,
            shelter_quality: 0.0,
            ..super::super::AgentWorldObservation::default()
        };

        // Enough ticks at zero access for glucose/hydration to hit zero.
        for tick in 0..2000 {
            if matches!(human.profile.status, HumanStatus::Dead) {
                break;
            }
            step_lifecycle(&mut human, 0.05, tick, &starved, &rng(), (0, 0));
        }

        assert!(matches!(human.profile.status, HumanStatus::Dead));
    }

    #[test]
    fn conceive_embryo_requires_opposite_sex_living_parents() {
        let mother = HumanBeing::new("mother".to_string(), BiologicalSex::Female);
        let father = HumanBeing::new("father".to_string(), BiologicalSex::Male);
        let registry_rng = rng();

        let child = conceive_embryo(&mother, &father, &registry_rng, 0);
        assert!(child.is_some());

        let mut dead_father = father;
        dead_father.profile.status = HumanStatus::Dead;
        assert!(conceive_embryo(&mother, &dead_father, &registry_rng, 0).is_none());

        let other_mother = HumanBeing::new("other_mother".to_string(), BiologicalSex::Female);
        assert!(conceive_embryo(&mother, &other_mother, &registry_rng, 0).is_none());
    }

    #[test]
    fn conceive_embryo_derives_real_astrology_metadata_not_the_conceive_scaffold() {
        // Regression test for hostile-completeness-audit-2026-09-04 finding
        // A: `genetics::conceive()` gives every child a scaffold
        // Libra/Libra/Libra astrology with empty birth_season/lunar_phase/
        // trait_modifiers; `assign_birth_chart` must
        // overwrite it at delivery with a real computed chart plus derived
        // metadata on the success path (the only path exercised by default parents
        // with valid birthplace coordinates).
        let mother = HumanBeing::new("mother".to_string(), BiologicalSex::Female);
        let father = HumanBeing::new("father".to_string(), BiologicalSex::Male);
        let registry_rng = rng();

        let child = born(&mother, &father, &registry_rng, 0);
        let astrology = &child.profile.genome.astrology;

        assert!(
            !astrology.birth_season.is_empty(),
            "birth_season must be derived, not left as the conceive() scaffold's empty default"
        );
        assert!(
            !astrology.lunar_phase.is_empty(),
            "lunar_phase must be derived, not left as the conceive() scaffold's empty default"
        );
        assert!(
            !astrology.trait_modifiers.is_empty(),
            "trait_modifiers must be populated from the real computed chart"
        );
    }

    #[test]
    fn conceive_embryo_is_deterministic_for_a_fixed_seed_and_tick() {
        let mother = HumanBeing::new("mother".to_string(), BiologicalSex::Female);
        let father = HumanBeing::new("father".to_string(), BiologicalSex::Male);
        let registry_rng = rng();

        let child_a = conceive_embryo(&mother, &father, &registry_rng, 3).unwrap();
        let child_b = conceive_embryo(&mother, &father, &registry_rng, 3).unwrap();

        assert_eq!(
            child_a.profile.genome.sex_chromosomes.is_male(),
            child_b.profile.genome.sex_chromosomes.is_male()
        );
        assert!(
            (child_a.genetics.heritable_stability - child_b.genetics.heritable_stability).abs()
                < 1e-12
        );
    }

    #[test]
    fn maternal_dna_comes_from_the_mother_whatever_the_argument_order() {
        let mother = HumanBeing::new("order2_mother".to_string(), BiologicalSex::Female);
        let father = HumanBeing::new("order2_father".to_string(), BiologicalSex::Male);
        let registry_rng = rng();

        let mother_first = conceive_embryo(&mother, &father, &registry_rng, 4).unwrap();
        let father_first = conceive_embryo(&father, &mother, &registry_rng, 4).unwrap();

        assert_eq!(
            serde_json::to_value(&mother_first.profile.genome).unwrap(),
            serde_json::to_value(&father_first.profile.genome).unwrap()
        );
    }

    #[test]
    fn different_pairs_on_the_same_tick_draw_independent_gametes() {
        let registry_rng = rng();
        let pair = |tag: &str| {
            (
                HumanBeing::new(format!("{tag}_mother"), BiologicalSex::Female),
                HumanBeing::new(format!("{tag}_father"), BiologicalSex::Male),
            )
        };
        let (mother_a, father_a) = pair("pair_c");
        let (mother_b, father_b) = pair("pair_d");

        let child_a = conceive_embryo(&mother_a, &father_a, &registry_rng, 6).unwrap();
        let child_b = conceive_embryo(&mother_b, &father_b, &registry_rng, 6).unwrap();

        assert_ne!(
            parent_pair_chunk(&mother_a, &father_a),
            parent_pair_chunk(&mother_b, &father_b)
        );
        assert_ne!(
            serde_json::to_value(&child_a.profile.genome).unwrap(),
            serde_json::to_value(&child_b.profile.genome).unwrap()
        );
    }

    #[test]
    fn personality_inheritance_applies_deterministic_mutation_distinct_from_pure_average() {
        // Regression test for the confirmed convergence gap: pure `(a+b)/2`
        // averaging of drive_weights/temperament has no diversifying force,
        // so population personality variance would shrink toward the mean
        // over generations even though gamete-level genome traits keep real
        // mutation-driven variance. `inherit_parent_traits`
        // must apply its own keyed-RNG mutation term on top of the average.
        let mother = HumanBeing::new("mutation_mother".to_string(), BiologicalSex::Female);
        let father = HumanBeing::new("mutation_father".to_string(), BiologicalSex::Male);
        let registry_rng = rng();

        let child_tick_5 = conceive_embryo(&mother, &father, &registry_rng, 5).unwrap();
        let pure_average =
            (mother.profile.drive_weights.survival + father.profile.drive_weights.survival) / 2.0;
        assert!(
            (child_tick_5.profile.drive_weights.survival - pure_average).abs() > 1e-9,
            "mutated drive weight should differ from the unmutated parental average"
        );

        // Same tick/seed stays deterministic (replay-friendly), matching
        // the existing `conceive_embryo_is_deterministic_for_a_fixed_seed_and_tick`
        // contract for genome traits.
        let child_tick_5_again = conceive_embryo(&mother, &father, &registry_rng, 5).unwrap();
        assert_eq!(
            child_tick_5.profile.drive_weights.survival,
            child_tick_5_again.profile.drive_weights.survival
        );

        // A different tick draws a different mutation, giving real
        // generation-to-generation variance instead of monotonic
        // convergence to the population mean.
        let child_tick_9 = conceive_embryo(&mother, &father, &registry_rng, 9).unwrap();
        assert!(
            (child_tick_5.profile.drive_weights.survival
                - child_tick_9.profile.drive_weights.survival)
                .abs()
                > 1e-9
        );
    }

    fn grid() -> mk_core::grid::GridSpec {
        mk_core::grid::GridSpec::new(18, 36)
    }

    /// A fertile adult: reproductive state stepped once at age 25 so its
    /// fertility level reflects its genome-derived fertile window.
    fn adult(name: &str, sex: BiologicalSex) -> HumanBeing {
        let mut human = HumanBeing::new(name.to_string(), sex);
        human.development = super::super::development::DevelopmentSnapshot::new(25.0);
        assert!(
            is_fertile(&human),
            "test fixture {name} must be genome-fertile"
        );
        step_reproductive_days(&mut human, 1.0);
        human
    }

    fn step_reproductive_days(human: &mut HumanBeing, days: f64) {
        let timeline = reproductive_timeline(human);
        human.reproduction = human.reproduction.step(
            human.development.age_years,
            &timeline,
            false,
            &human.needs,
            &human.social_cognition,
            days / 365.25,
        );
    }

    /// Advance a female's cycle a day at a time until its fertile window.
    fn in_fertile_window(mut mother: HumanBeing) -> HumanBeing {
        for _ in 0..28 {
            if mother
                .reproduction
                .fertility_cycle
                .as_ref()
                .is_some_and(|cycle| cycle.fertility_peak)
            {
                return mother;
            }
            step_reproductive_days(&mut mother, 1.0);
        }
        panic!("cycle never reached its fertile window");
    }

    /// Conceive and carry to term immediately, for tests about the newborn.
    fn born(mother: &HumanBeing, father: &HumanBeing, rng: &RngRegistry, tick: u64) -> HumanBeing {
        let embryo = conceive_embryo(mother, father, rng, tick).expect("conception");
        let pregnancy = super::super::reproduction::Pregnancy {
            father_id: father.agent_id().to_string(),
            conceived_tick: tick,
            gestation_years: super::super::reproduction::GESTATION_YEARS,
            embryo: Box::new(embryo),
        };
        deliver_birth(mother, pregnancy, tick, &grid())
    }

    #[test]
    fn conception_requires_an_eligible_opposite_sex_pair() {
        let registry_rng = rng();
        // Sampled genomes make ~12% of people infertile; this id's genome is
        // fertile, which is what the eligibility checks below need.
        let father = adult("elig_father_a", BiologicalSex::Male);
        let mut other_woman = in_fertile_window(adult("elig_other", BiologicalSex::Female));
        let mut mother = in_fertile_window(adult("elig_mother", BiologicalSex::Female));
        let mut child = HumanBeing::new("elig_child".to_string(), BiologicalSex::Female);
        child.development = super::super::development::DevelopmentSnapshot::new(5.0);

        let same_sex = other_woman.clone();
        assert!(!attempt_conception(
            &mut mother,
            &same_sex,
            0.0,
            &registry_rng,
            1
        ));
        assert!(!attempt_conception(
            &mut child,
            &father,
            0.0,
            &registry_rng,
            1
        ));
        let mut dead_father = father.clone();
        dead_father.profile.status = HumanStatus::Dead;
        assert!(!attempt_conception(
            &mut other_woman,
            &dead_father,
            0.0,
            &registry_rng,
            1
        ));
        assert!(mother.reproduction.pregnancy.is_none());
    }

    #[test]
    fn conception_is_a_chance_per_act_not_a_certainty() {
        let registry_rng = rng();
        let father = adult("chance_father", BiologicalSex::Male);
        let mother = in_fertile_window(adult("chance_mother", BiologicalSex::Female));

        let attempts = 400;
        let conceptions = (0..attempts)
            .filter(|tick| {
                attempt_conception(&mut mother.clone(), &father, 0.0, &registry_rng, *tick)
            })
            .count();

        assert!(conceptions > 0, "a fertile couple must sometimes conceive");
        assert!(
            conceptions < attempts as usize,
            "conception must not be certain"
        );
    }

    #[test]
    fn a_pregnant_mother_cannot_conceive_again() {
        let registry_rng = rng();
        let father = adult("again_father", BiologicalSex::Male);
        let mut mother = in_fertile_window(adult("again_mother", BiologicalSex::Female));

        let mut tick = 0;
        while !attempt_conception(&mut mother, &father, 0.0, &registry_rng, tick) {
            tick += 1;
            assert!(tick < 1000, "never conceived");
        }

        assert!(mother.reproduction.pregnancy.is_some());
        assert_eq!(
            mother
                .reproduction
                .conception_chance(father.reproduction.fertility_level),
            0.0
        );
        assert!(!attempt_conception(
            &mut mother,
            &father,
            0.0,
            &registry_rng,
            tick + 1
        ));
    }

    #[test]
    fn a_full_term_pregnancy_delivers_where_the_mother_stands() {
        let registry_rng = rng();
        let father = adult("term_father", BiologicalSex::Male);
        let mut mother = in_fertile_window(adult("term_mother", BiologicalSex::Female));
        mother.set_runtime_position(super::super::GridPosition::new(4, 7));
        let mut tick = 0;
        while !attempt_conception(&mut mother, &father, 0.0, &registry_rng, tick) {
            tick += 1;
        }

        let mut registry = HumanRegistry::new();
        registry.add_human_no_storage(father);
        registry.add_human_no_storage(mother);

        // Not yet due: no birth.
        deliver_due_births(&mut registry, tick + 1, &grid());
        assert_eq!(registry.population_count(), 2);

        // Carry to term through real gestation steps.
        let mother = registry.get_human_mut("term_mother").unwrap();
        for _ in 0..41 {
            step_reproductive_days(mother, 7.0);
        }
        assert!(mother.reproduction.gestation_week >= 40);
        deliver_due_births(&mut registry, tick + 2, &grid());

        assert_eq!(registry.population_count(), 3);
        let child = registry
            .iter()
            .find(|h| h.agent_id() != "term_mother" && h.agent_id() != "term_father")
            .expect("a child must have been born");
        assert_eq!(child.position, super::super::GridPosition::new(4, 7));
        assert_eq!(child.reproduction.birth_record_count, 1);
        assert_eq!(child.reproduction.conception_history_count, 1);
        assert_eq!(
            child.parent_agent_ids(),
            Some(("term_father".to_string(), "term_mother".to_string()))
        );

        let mother = registry.get_human("term_mother").unwrap();
        assert!(mother.reproduction.pregnancy.is_none());
        assert!(mother.reproduction.postpartum_years_remaining > 0.0);
    }

    #[test]
    fn birth_instant_follows_the_mothers_lived_time() {
        let registry_rng = rng();
        let father = adult("birth_father", BiologicalSex::Male);
        let mut mother = adult("birth_mother", BiologicalSex::Female);
        mother.profile.core_identity.birth_timestamp = "1990-06-01T00:00:00Z".to_string();
        mother.development = super::super::development::DevelopmentSnapshot::new(30.0);

        let child = born(&mother, &father, &registry_rng, 3);

        assert!(
            child
                .profile
                .core_identity
                .birth_timestamp
                .starts_with("2020-"),
            "expected a 2020 birth, got {}",
            child.profile.core_identity.birth_timestamp
        );
    }

    #[test]
    fn hungry_humans_eat_the_food_they_carry() {
        let mut human = HumanBeing::new("ration_eater".to_string(), BiologicalSex::Female);
        human.needs.glucose = 0.3;
        human.needs.hydration = 0.9;
        // Bare hands hold one item each: one ration of food, one of water.
        assert_eq!(human.carrying.add(crate::agents::ItemKind::Food, 1), 1);
        assert_eq!(human.carrying.add(crate::agents::ItemKind::Water, 1), 1);

        consume_carried_rations(&mut human);

        assert!((human.needs.glucose - 0.4).abs() < 1e-12);
        assert_eq!(human.carrying.count(crate::agents::ItemKind::Food), 0);
        // Not thirsty: water is kept for later.
        assert_eq!(human.carrying.count(crate::agents::ItemKind::Water), 1);
        assert!((human.needs.hunger - 0.6).abs() < 1e-12);
    }

    #[test]
    fn age_mortality_rises_steeply_with_age() {
        let registry_rng = rng();
        let deaths_at = |age: f64| {
            (0..400)
                .filter(|tick| {
                    let mut human =
                        HumanBeing::new(format!("gompertz_{age}_{tick}"), BiologicalSex::Male);
                    human.development = super::super::development::DevelopmentSnapshot::new(age);
                    dies_of_age(&human, 1.0, *tick, &registry_rng)
                })
                .count()
        };
        let young = deaths_at(30.0);
        let old = deaths_at(90.0);
        assert!(
            young < 5,
            "30-year-olds rarely die of age in a year ({young}/400)"
        );
        assert!(old > 40, "90-year-olds often do ({old}/400)");
    }
}
