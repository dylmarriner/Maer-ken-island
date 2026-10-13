//! The island's human population before the island world exists.
//!
//! People are created through the same path upstream uses for
//! `InterventionAction::SpawnHuman` (`mk_engine::humans::spawn`), validated by
//! upstream's `mk_interventions::validate_intervention`, and — when the
//! population is opened on a data directory — each one gets their own folder
//! in the upstream `HumanStorage` layout. Time does not pass yet: humans are
//! created and stored but not stepped until the island world exists.

use mk_core::human::{BiologicalSex, HumanStatus};
use mk_core::rng::RngRegistry;
use mk_engine::humans::spawn::{agent_id_for_name, build_authored_human};
use mk_engine::humans::{HumanBeing, HumanRegistry, HumanSystem};
use mk_engine::io::HumanStorage;
use mk_interventions::{validate_intervention, HumanSpawnProfile, InterventionAction, Location};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HumanSummary {
    pub agent_id: String,
    /// The name as it was typed at creation (the agent id is its slug).
    pub name: String,
    pub human_id: String,
    pub biological_sex: String,
    pub status: String,
    pub age_years: f64,
}

/// A request to create one human: the upstream `HumanSpawnProfile` fields
/// plus biological sex (`"male"` or `"female"`, the templates upstream's
/// spawn supports).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CreateHumanRequest {
    pub name: String,
    pub biological_sex: String,
    /// RFC 3339 birth instant.
    pub birth_timestamp: String,
    pub birth_latitude: f64,
    pub birth_longitude: f64,
    pub age_years: f64,
    pub height_cm: f64,
    pub build: String,
    pub hair_color: String,
    pub eye_color: String,
    pub skin_tone: String,
}

impl CreateHumanRequest {
    fn profile(&self) -> HumanSpawnProfile {
        HumanSpawnProfile {
            name: self.name.trim().to_string(),
            birth_timestamp: self.birth_timestamp.clone(),
            birth_latitude: self.birth_latitude,
            birth_longitude: self.birth_longitude,
            age_years: self.age_years,
            height_cm: self.height_cm,
            build: self.build.clone(),
            hair_color: self.hair_color.clone(),
            eye_color: self.eye_color.clone(),
            skin_tone: self.skin_tone.clone(),
        }
    }

    fn sex(&self) -> Option<BiologicalSex> {
        match self.biological_sex.trim().to_ascii_lowercase().as_str() {
            "male" => Some(BiologicalSex::Male),
            "female" => Some(BiologicalSex::Female),
            _ => None,
        }
    }

    /// Upstream's validation for the equivalent `SpawnHuman` action, plus a
    /// check that the sex is one upstream supports.
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        if self.sex().is_none() {
            errors.push("Biological sex must be \"male\" or \"female\"".to_string());
        }
        let action = InterventionAction::SpawnHuman {
            template_id: self.biological_sex.trim().to_ascii_lowercase(),
            location: Location::new(self.birth_latitude as f32, self.birth_longitude as f32),
            profile: Some(self.profile()),
        };
        errors.extend(validate_intervention(&action).errors);
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

/// A human that was created. `storage_error` is set when the human exists
/// but their folder or `created` event could not be written.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CreatedHuman {
    pub summary: HumanSummary,
    pub folder: Option<PathBuf>,
    pub storage_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CreateHumanError {
    /// Upstream's validation messages, one per problem.
    Invalid(Vec<String>),
}

#[derive(Debug)]
pub enum PopulationError {
    Io(std::io::Error),
    Storage(String),
    Corrupt(String),
    /// The data directory was created with a different seed.
    SeedMismatch,
}

impl std::fmt::Display for PopulationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(err) => write!(f, "population I/O error: {err}"),
            Self::Storage(err) => write!(f, "human storage error: {err}"),
            Self::Corrupt(err) => write!(f, "population data is corrupt: {err}"),
            Self::SeedMismatch => write!(f, "the data directory was created with a different seed"),
        }
    }
}

impl std::error::Error for PopulationError {}

impl From<std::io::Error> for PopulationError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

/// `<data-dir>/population.json`: what makes creation deterministic.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct PopulationRecord {
    version: u32,
    seed_hex: String,
    /// Creations so far; the next creation uses this value as its tick.
    counter: u64,
}

/// One line of `<data-dir>/creations.jsonl`, in creation order.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CreationRecord {
    pub counter: u64,
    pub agent_id: String,
    pub by: String,
    pub request: CreateHumanRequest,
}

pub const DEFAULT_SEED: [u8; 32] = [0u8; 32];

pub struct IslandHumanPopulation {
    system: HumanSystem,
    rng: RngRegistry,
    seed: [u8; 32],
    counter: u64,
    data_dir: Option<PathBuf>,
    /// Names as typed at creation, by agent id. Upstream keeps only the slug,
    /// so the island layer keeps the original spelling (from `creations.jsonl`).
    names: HashMap<String, String>,
}

impl Default for IslandHumanPopulation {
    fn default() -> Self {
        Self::with_founders()
    }
}

impl IslandHumanPopulation {
    /// In-memory population of the two canonical founders (nothing written).
    pub fn with_founders() -> Self {
        let mut population = Self::empty();
        population
            .system
            .registry
            .add_human_no_storage(HumanBeing::gem_d_founder());
        population
            .system
            .registry
            .add_human_no_storage(HumanBeing::gem_k_founder());
        population
    }

    /// In-memory population with nobody in it (nothing written).
    pub fn empty() -> Self {
        Self {
            system: HumanSystem::new(),
            rng: RngRegistry::new(DEFAULT_SEED),
            seed: DEFAULT_SEED,
            counter: 0,
            data_dir: None,
            names: HashMap::new(),
        }
    }

    /// Open (or create) a population stored in `data_dir`: every human has a
    /// folder under `<data_dir>/humans/`, everyone stored there is reloaded,
    /// and Gem-D and Gem-K are added (with folders) if absent. The seed is
    /// recorded on first open and must match afterwards. Returns any storage
    /// warnings from seeding the founders alongside the population.
    pub fn open(data_dir: &Path, seed: [u8; 32]) -> Result<(Self, Vec<String>), PopulationError> {
        std::fs::create_dir_all(data_dir)?;
        let record_path = data_dir.join("population.json");
        let record = if record_path.exists() {
            let record: PopulationRecord = serde_json::from_slice(&std::fs::read(&record_path)?)
                .map_err(|err| PopulationError::Corrupt(err.to_string()))?;
            if record.version != 1 {
                return Err(PopulationError::Corrupt(format!(
                    "unknown population version {}",
                    record.version
                )));
            }
            if record.seed_hex != hex::encode(seed) {
                return Err(PopulationError::SeedMismatch);
            }
            record
        } else {
            let record = PopulationRecord {
                version: 1,
                seed_hex: hex::encode(seed),
                counter: 0,
            };
            write_atomically(
                &record_path,
                &serde_json::to_vec_pretty(&record).expect("serializable"),
            )?;
            record
        };

        let storage = HumanStorage::try_new(data_dir.join("humans"))
            .map_err(|err| PopulationError::Storage(err.to_string()))?;
        let mut registry = HumanRegistry::with_storage(storage)
            .map_err(|err| PopulationError::Storage(err.to_string()))?;
        let warnings = match registry.seed_founders() {
            Ok(()) => Vec::new(),
            Err(errors) => errors.iter().map(ToString::to_string).collect(),
        };
        let names = read_creations(data_dir)?
            .into_iter()
            .map(|record| (record.agent_id, record.request.name.trim().to_string()))
            .collect();
        let mut system = HumanSystem::new();
        system.registry = registry;
        Ok((
            Self {
                system,
                rng: RngRegistry::new(seed),
                seed,
                counter: record.counter,
                data_dir: Some(data_dir.to_path_buf()),
                names,
            },
            warnings,
        ))
    }

    /// Create one human. Validation uses upstream's rules; the name becomes
    /// the agent id (duplicates get `-2`, `-3`, …); the human is built
    /// through upstream's spawn path from this population's seed and
    /// creation counter, so the same seed and the same ordered requests
    /// produce the same people. A storage failure never removes the human:
    /// it is reported in `storage_error`.
    pub fn create_human(
        &mut self,
        request: CreateHumanRequest,
        by: &str,
    ) -> Result<CreatedHuman, CreateHumanError> {
        request.validate().map_err(CreateHumanError::Invalid)?;
        let sex = request.sex().expect("validated");
        let agent_id = agent_id_for_name(&self.system.registry, &request.name);
        let human = build_authored_human(
            &self.rng,
            self.counter,
            agent_id.clone(),
            sex,
            &request.profile(),
        )
        .map_err(|err| CreateHumanError::Invalid(vec![err.to_string()]))?;
        let mut summary = summarize(&human);
        summary.name = request.name.trim().to_string();
        self.names.insert(agent_id.clone(), summary.name.clone());
        let counter = self.counter;

        let mut storage_errors = Vec::new();
        if let Err(err) = self.system.registry.add_human(human) {
            storage_errors.push(err.to_string());
        }
        let event = serde_json::json!({ "kind": "created", "by": by, "counter": counter });
        if let Err(err) = self.system.registry.record_event(&agent_id, &event) {
            storage_errors.push(err.to_string());
        }
        self.counter += 1;
        if let Some(dir) = &self.data_dir {
            let line = CreationRecord {
                counter,
                agent_id: agent_id.clone(),
                by: by.to_string(),
                request,
            };
            if let Err(err) = append_line(&dir.join("creations.jsonl"), &line)
                .and_then(|()| self.persist_record())
            {
                storage_errors.push(err.to_string());
            }
        }

        Ok(CreatedHuman {
            folder: self.folder_of(&agent_id),
            summary,
            storage_error: (!storage_errors.is_empty()).then(|| storage_errors.join("; ")),
        })
    }

    fn persist_record(&self) -> std::io::Result<()> {
        let Some(dir) = &self.data_dir else {
            return Ok(());
        };
        let record = PopulationRecord {
            version: 1,
            seed_hex: hex::encode(self.seed),
            counter: self.counter,
        };
        write_atomically(
            &dir.join("population.json"),
            &serde_json::to_vec_pretty(&record).expect("serializable"),
        )
    }

    /// Where this human's folder is, when the population is stored.
    pub fn folder_of(&self, agent_id: &str) -> Option<PathBuf> {
        self.data_dir
            .as_ref()
            .map(|dir| dir.join("humans").join(agent_id))
    }

    pub fn data_dir(&self) -> Option<&Path> {
        self.data_dir.as_deref()
    }

    pub fn seed(&self) -> [u8; 32] {
        self.seed
    }

    pub fn get(&self, agent_id: &str) -> Option<&HumanBeing> {
        self.system.registry.get_human(agent_id)
    }

    pub fn len(&self) -> usize {
        self.system.registry.count()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn summaries(&self) -> Vec<HumanSummary> {
        self.system
            .registry
            .iter()
            .map(|human| self.with_name(summarize(human)))
            .collect()
    }

    /// The summary of one human, with their name as typed at creation.
    pub fn summary(&self, agent_id: &str) -> Option<HumanSummary> {
        self.get(agent_id)
            .map(|human| self.with_name(summarize(human)))
    }

    fn with_name(&self, mut summary: HumanSummary) -> HumanSummary {
        if let Some(name) = self.names.get(&summary.agent_id) {
            summary.name = name.clone();
        }
        summary
    }

    pub fn system(&self) -> &HumanSystem {
        &self.system
    }

    pub fn system_mut(&mut self) -> &mut HumanSystem {
        &mut self.system
    }
}

fn write_atomically(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let tmp = path.with_extension("tmp");
    {
        let mut file = std::fs::File::create(&tmp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
    }
    std::fs::rename(tmp, path)
}

fn append_line(path: &Path, record: &CreationRecord) -> std::io::Result<()> {
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    let mut line = serde_json::to_vec(record).expect("serializable");
    line.push(b'\n');
    file.write_all(&line)?;
    file.sync_all()
}

/// Read `<data-dir>/creations.jsonl` back, in creation order.
pub fn read_creations(data_dir: &Path) -> Result<Vec<CreationRecord>, PopulationError> {
    let path = data_dir.join("creations.jsonl");
    if !path.exists() {
        return Ok(Vec::new());
    }
    std::fs::read_to_string(path)?
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            serde_json::from_str(line).map_err(|err| PopulationError::Corrupt(err.to_string()))
        })
        .collect()
}

pub fn summarize(human: &HumanBeing) -> HumanSummary {
    HumanSummary {
        agent_id: human.agent_id().to_string(),
        name: human.agent_id().to_string(),
        human_id: human.profile.human_id.to_string(),
        biological_sex: match human.biological_sex() {
            BiologicalSex::Male => "male",
            BiologicalSex::Female => "female",
            BiologicalSex::Neutral => "neutral",
        }
        .to_string(),
        status: match human.profile.status {
            HumanStatus::Alive => "alive",
            HumanStatus::Dead => "dead",
            HumanStatus::Dormant => "dormant",
        }
        .to_string(),
        age_years: human.development.age_years,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, Utc};
    use mk_core::human::astrology::GeoCoordinates;

    fn birth() -> DateTime<Utc> {
        "2000-01-02T03:04:05Z".parse().unwrap()
    }

    pub(crate) fn request(name: &str) -> CreateHumanRequest {
        CreateHumanRequest {
            name: name.to_string(),
            biological_sex: "female".to_string(),
            birth_timestamp: "1990-06-15T12:00:00+00:00".to_string(),
            birth_latitude: -41.0,
            birth_longitude: 174.0,
            age_years: 30.0,
            height_cm: 168.0,
            build: "Average".to_string(),
            hair_color: "Brown".to_string(),
            eye_color: "Green".to_string(),
            skin_tone: "Medium".to_string(),
        }
    }

    #[test]
    fn canonical_founders_are_recreated() {
        let population = IslandHumanPopulation::with_founders();
        assert_eq!(population.len(), 2);
        assert!(population.get("Gem-D").is_some());
        assert!(population.get("Gem-K").is_some());
    }

    #[test]
    fn generated_human_is_deterministic_for_same_inputs() {
        let coords = GeoCoordinates {
            latitude: -36.85,
            longitude: 174.76,
        };
        let a = HumanBeing::new_born_at(
            "islander-001".into(),
            BiologicalSex::Female,
            birth(),
            coords,
            "Maer-Ken Island",
        );
        let b = HumanBeing::new_born_at(
            "islander-001".into(),
            BiologicalSex::Female,
            birth(),
            coords,
            "Maer-Ken Island",
        );
        assert_eq!(a.profile.human_id, b.profile.human_id);
        assert_eq!(
            serde_json::to_value(&a.profile).unwrap(),
            serde_json::to_value(&b.profile).unwrap()
        );
    }

    #[test]
    fn duplicate_names_get_numbered_agent_ids() {
        let mut population = IslandHumanPopulation::empty();
        let first = population
            .create_human(request("Same Person"), "test")
            .unwrap();
        let second = population
            .create_human(request("Same Person"), "test")
            .unwrap();
        assert_eq!(first.summary.agent_id, "same-person");
        assert_eq!(second.summary.agent_id, "same-person-2");
        assert_eq!(population.len(), 2);
    }

    #[test]
    fn invalid_requests_return_upstream_validation_messages() {
        let mut population = IslandHumanPopulation::empty();
        let mut bad = request("X");
        bad.age_years = 500.0;
        bad.biological_sex = "robot".into();
        match population.create_human(bad, "test") {
            Err(CreateHumanError::Invalid(errors)) => {
                assert!(errors.iter().any(|e| e.contains("age")), "{errors:?}");
                assert!(errors.iter().any(|e| e.contains("sex")), "{errors:?}");
            }
            other => panic!("expected invalid, got {other:?}"),
        }
        assert!(population.is_empty());
    }
}
