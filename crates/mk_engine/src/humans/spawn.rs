//! Building a complete human from an authored or default spawn request,
//! independent of any world.
//!
//! Extracted from `interventions::spawn_human` (island divergence, see
//! `UPSTREAM.md`) so the island population can create people through the
//! exact path upstream uses for `InterventionAction::SpawnHuman`, without a
//! planetary `WorldState`. `spawn_human` now calls these functions, so both
//! paths produce byte-identical humans for identical inputs.

use super::{HumanBeing, HumanRegistry};
use mk_core::human::astrology::GeoCoordinates;
use mk_core::human::BiologicalSex;
use mk_core::rng::{RngKey, RngRegistry, SubsystemId};
use mk_interventions::HumanSpawnProfile;

/// Keyed RNG epoch for sampling a spawned human's genome and traits,
/// distinct from every other `SubsystemId::Humans` consumer's epoch.
pub const SPAWN_HUMAN_EPOCH: u32 = 0x5350_574E;

/// Why a spawn request could not be turned into a human.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpawnHumanError {
    /// The authored birth timestamp is not RFC 3339.
    InvalidBirthTimestamp(String),
    /// Sampling the individual failed.
    Sample(String),
}

impl std::fmt::Display for SpawnHumanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidBirthTimestamp(reason) => write!(f, "{reason}"),
            Self::Sample(reason) => write!(f, "{reason}"),
        }
    }
}

impl std::error::Error for SpawnHumanError {}

/// The agent id for a human named `name`: the lower-case alphanumeric slug
/// of the name, or the slug with `-2`, `-3`, … appended when it is taken.
pub fn agent_id_for_name(registry: &HumanRegistry, name: &str) -> String {
    let slug = name
        .split(|c: char| !c.is_alphanumeric())
        .filter(|part| !part.is_empty())
        .map(str::to_lowercase)
        .collect::<Vec<_>>()
        .join("-");
    if registry.get_human(&slug).is_none() {
        return slug;
    }
    (2u64..)
        .map(|n| format!("{slug}-{n}"))
        .find(|candidate| registry.get_human(candidate).is_none())
        .expect("an unbounded suffix range always yields a free id")
}

/// Sample a complete human born at `birth` in `birthplace`, aged
/// `age_years`, from the RNG stream keyed by the agent id and `tick`, and
/// apply the authored body and appearance from `profile` when given. The
/// caller places the human in the world.
#[allow(clippy::too_many_arguments)]
pub fn build_spawned_human(
    rng: &RngRegistry,
    tick: u64,
    agent_id: String,
    sex: BiologicalSex,
    birth: chrono::DateTime<chrono::Utc>,
    birthplace: GeoCoordinates,
    age_years: f64,
    profile: Option<&HumanSpawnProfile>,
) -> Result<HumanBeing, SpawnHumanError> {
    let place_name = format!("{:.4}, {:.4}", birthplace.latitude, birthplace.longitude);
    let mut stream = rng.stream(RngKey::new(
        SubsystemId::Humans,
        (super::deterministic_human_id(&agent_id) & 0xFFFF_FFFF) as u32,
        SPAWN_HUMAN_EPOCH,
        tick,
    ));
    let mut human = HumanBeing::sampled(
        agent_id,
        sex,
        birth,
        birthplace,
        &place_name,
        age_years,
        &mut stream,
    )
    .map_err(|err| SpawnHumanError::Sample(err.to_string()))?;

    human.development.age_years = age_years;
    if let Some(profile) = profile {
        // Keep the authored timestamp as written (its offset included); the
        // sampled chart already used the same instant.
        human.profile.core_identity.birth_timestamp = profile.birth_timestamp.clone();
        human.body.height_cm = profile.height_cm;
        human.body.build = profile.build.trim().to_lowercase();
        human.body.hair_color = profile.hair_color.trim().to_lowercase();
        human.body.eye_color = profile.eye_color.trim().to_lowercase();
        human.body.skin_tone = profile.skin_tone.trim().to_lowercase();
    }
    human.refresh_phase11_layers();
    Ok(human)
}

/// Build a human from an authored profile: birth instant, birthplace and age
/// come from the profile.
pub fn build_authored_human(
    rng: &RngRegistry,
    tick: u64,
    agent_id: String,
    sex: BiologicalSex,
    profile: &HumanSpawnProfile,
) -> Result<HumanBeing, SpawnHumanError> {
    let birth = chrono::DateTime::parse_from_rfc3339(&profile.birth_timestamp)
        .map_err(|err| {
            SpawnHumanError::InvalidBirthTimestamp(format!(
                "birth timestamp '{}' is not RFC 3339: {err}",
                profile.birth_timestamp
            ))
        })?
        .with_timezone(&chrono::Utc);
    let birthplace = GeoCoordinates {
        latitude: profile.birth_latitude,
        longitude: profile.birth_longitude,
    };
    build_spawned_human(
        rng,
        tick,
        agent_id,
        sex,
        birth,
        birthplace,
        profile.age_years,
        Some(profile),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile(name: &str) -> HumanSpawnProfile {
        HumanSpawnProfile {
            name: name.to_string(),
            birth_timestamp: "1990-06-15T12:00:00+00:00".to_string(),
            birth_latitude: -41.0,
            birth_longitude: 174.0,
            age_years: 30.0,
            height_cm: 172.0,
            build: "Average".to_string(),
            hair_color: "Brown".to_string(),
            eye_color: "Green".to_string(),
            skin_tone: "Medium".to_string(),
        }
    }

    fn json(human: &HumanBeing) -> String {
        serde_json::to_string(&serde_json::to_value(human).unwrap()).unwrap()
    }

    #[test]
    fn identical_requests_build_byte_identical_humans() {
        let rng = RngRegistry::new([9u8; 32]);
        let p = profile("Sam");
        let a = build_authored_human(&rng, 5, "sam".into(), BiologicalSex::Female, &p).unwrap();
        let b = build_authored_human(&rng, 5, "sam".into(), BiologicalSex::Female, &p).unwrap();
        assert_eq!(json(&a), json(&b));
        assert_eq!(a.body.hair_color, "brown");
        assert_eq!(a.development.age_years, 30.0);
    }

    #[test]
    fn a_different_tick_or_agent_id_builds_a_different_human() {
        let rng = RngRegistry::new([9u8; 32]);
        let p = profile("Sam");
        let base = build_authored_human(&rng, 5, "sam".into(), BiologicalSex::Male, &p).unwrap();
        let later = build_authored_human(&rng, 6, "sam".into(), BiologicalSex::Male, &p).unwrap();
        let other = build_authored_human(&rng, 5, "sam-2".into(), BiologicalSex::Male, &p).unwrap();
        assert_ne!(json(&base), json(&later));
        assert_ne!(json(&base), json(&other));
    }

    #[test]
    fn a_bad_birth_timestamp_is_rejected() {
        let rng = RngRegistry::new([9u8; 32]);
        let mut p = profile("Sam");
        p.birth_timestamp = "yesterday".into();
        assert!(matches!(
            build_authored_human(&rng, 0, "sam".into(), BiologicalSex::Male, &p),
            Err(SpawnHumanError::InvalidBirthTimestamp(_))
        ));
    }

    #[test]
    fn duplicate_names_get_numbered_ids() {
        let rng = RngRegistry::new([9u8; 32]);
        let mut registry = HumanRegistry::new();
        assert_eq!(agent_id_for_name(&registry, "Sam O'Neil"), "sam-o-neil");
        for expected in ["sam", "sam-2", "sam-3"] {
            let id = agent_id_for_name(&registry, "Sam");
            assert_eq!(id, expected);
            let human =
                build_authored_human(&rng, 0, id, BiologicalSex::Male, &profile("Sam")).unwrap();
            registry.add_human_no_storage(human);
        }
    }
}
