//! Human Storage - Persistent per-human folder system
//!
//! Each human gets an individual directory with sub-folders for:
//! - profile/ (identity, genome, appearance)
//! - traits/ (personality, temperament, drives)
//! - cognition/ (cognitive state, memory)
//! - social/ (relationships, attachment, culture)
//! - development/ (age, stage, lifecycle)
//! - memories/ (episodic, semantic, procedural)
//! - state/ (current runtime state snapshots)

use crate::humans::HumanBeing;
use crate::io::encryption::EncryptionManager;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Error type for human storage operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HumanStorageError {
    /// IO error during storage operation
    IoError(String),
    /// Human not found in storage
    NotFound(String),
    /// Serialization error
    SerializationError(String),
    /// Human already exists
    AlreadyExists(String),
    /// Invalid human ID (e.g. path traversal attempt)
    InvalidId(String),
    /// A human with this agent id is already in the registry. Unlike every
    /// other variant this means the human was **not** added.
    DuplicateAgent(String),
}

impl std::fmt::Display for HumanStorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HumanStorageError::IoError(msg) => write!(f, "IO error: {}", msg),
            HumanStorageError::NotFound(id) => write!(f, "Human not found: {}", id),
            HumanStorageError::SerializationError(msg) => write!(f, "Serialization error: {}", msg),
            HumanStorageError::AlreadyExists(id) => write!(f, "Human already exists: {}", id),
            HumanStorageError::InvalidId(id) => write!(f, "Invalid human ID: {}", id),
            HumanStorageError::DuplicateAgent(id) => {
                write!(f, "Agent id already in the registry: {}", id)
            }
        }
    }
}

impl std::error::Error for HumanStorageError {}

impl From<std::io::Error> for HumanStorageError {
    fn from(e: std::io::Error) -> Self {
        HumanStorageError::IoError(e.to_string())
    }
}

impl From<serde_json::Error> for HumanStorageError {
    fn from(e: serde_json::Error) -> Self {
        HumanStorageError::SerializationError(e.to_string())
    }
}

/// Persistent storage manager for human beings
///
/// Creates individual folders per human with structured sub-directories
/// for all aspects of their simulated existence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HumanStorage {
    /// Root directory for all human data
    pub base_path: PathBuf,
    /// Encryption manager for sensitive data
    #[serde(skip)]
    pub encryption: Option<EncryptionManager>,
    /// How many snapshots to keep
    pub snapshot_retention: usize,
}

impl HumanStorage {
    /// Create a new human storage at the given path.
    ///
    /// Kept for upstream parity. If the storage key cannot be initialised
    /// this logs the error and continues **without encryption**, so sensitive
    /// files are written in plaintext. Island code must use
    /// [`HumanStorage::try_new`], which refuses instead.
    pub fn new(base_path: impl Into<PathBuf>) -> Self {
        let base_path = base_path.into();
        let encryption = match EncryptionManager::init(&base_path) {
            Ok(manager) => Some(manager),
            Err(e) => {
                tracing::error!(
                    path = %base_path.display(),
                    error = %e,
                    "storage key unavailable; sensitive human data will be stored unencrypted"
                );
                None
            }
        };
        Self {
            base_path,
            encryption,
            snapshot_retention: 5,
        }
    }

    /// Create a new encrypted human storage, propagating any storage-key
    /// error instead of silently downgrading to plaintext.
    pub fn try_new(base_path: impl Into<PathBuf>) -> Result<Self, HumanStorageError> {
        let base_path = base_path.into();
        let encryption = EncryptionManager::init(&base_path)
            .map_err(|e| HumanStorageError::IoError(format!("storage key: {e}")))?;
        Ok(Self {
            base_path,
            encryption: Some(encryption),
            snapshot_retention: 5,
        })
    }

    /// Create an encrypted human storage from an explicit hex key (64 hex
    /// characters). An invalid key is an error; nothing is written.
    pub fn try_with_key_hex(
        base_path: impl Into<PathBuf>,
        key_hex: &str,
    ) -> Result<Self, HumanStorageError> {
        let key = crate::io::encryption::parse_storage_key(key_hex)
            .map_err(|e| HumanStorageError::IoError(format!("storage key: {e}")))?;
        Ok(Self {
            base_path: base_path.into(),
            encryption: Some(EncryptionManager::from_key(key)),
            snapshot_retention: 5,
        })
    }

    /// Create a new human storage without encryption (for testing)
    pub fn new_unencrypted(base_path: impl Into<PathBuf>) -> Self {
        Self {
            base_path: base_path.into(),
            encryption: None,
            snapshot_retention: 5,
        }
    }

    /// Initialize the storage directory structure
    pub fn init(&self) -> Result<(), HumanStorageError> {
        fs::create_dir_all(&self.base_path)?;
        Ok(())
    }

    /// Write sensitive JSON data to an encrypted file
    fn write_encrypted_json(
        &self,
        path: &std::path::Path,
        data: &serde_json::Value,
    ) -> Result<(), HumanStorageError> {
        let json = serde_json::to_string_pretty(data)?;
        if let Some(ref encryption) = self.encryption {
            let encrypted = encryption
                .encrypt(json.as_bytes())
                .map_err(|e| HumanStorageError::IoError(format!("Encryption error: {:?}", e)))?;

            // Write encrypted file
            let mut enc_path = path.to_path_buf();
            enc_path.set_extension("enc.json");
            fs::write(&enc_path, encrypted)?;

            // Securely delete plaintext file if it exists
            if path.exists() {
                let _ = fs::remove_file(path);
            }
        } else {
            // Fallback to plaintext if encryption is not initialized
            fs::write(path, json)?;
        }
        Ok(())
    }

    /// Read JSON data previously written by [`write_encrypted_json`], from
    /// either its encrypted (`.enc.json`) or plaintext form depending on how
    /// this storage is configured. Returns `Ok(None)` if neither form exists
    /// (the field was never written for this human).
    fn read_encrypted_json(
        &self,
        path: &std::path::Path,
    ) -> Result<Option<serde_json::Value>, HumanStorageError> {
        if let Some(ref encryption) = self.encryption {
            let mut enc_path = path.to_path_buf();
            enc_path.set_extension("enc.json");
            if enc_path.exists() {
                let encrypted = fs::read(&enc_path)?;
                let plaintext = encryption.decrypt(&encrypted).map_err(|e| {
                    HumanStorageError::IoError(format!("Decryption error: {:?}", e))
                })?;
                let value = serde_json::from_slice(&plaintext)?;
                return Ok(Some(value));
            }
        }
        if path.exists() {
            let raw = fs::read_to_string(path)?;
            return Ok(Some(serde_json::from_str(&raw)?));
        }
        Ok(None)
    }

    /// Reconstruct a [`HumanBeing`] previously persisted by [`Self::create_human`]
    /// / [`Self::update_human`], so that in-world human state survives process
    /// restarts and rebuilds. Returns `Ok(None)` if the human isn't in
    /// storage.
    ///
    /// Prefers the full-fidelity `state/full_snapshot.json` (every field of
    /// `HumanBeing`, written for every human regardless of whether it has a
    /// canonical schema). Falls back to reconstructing from
    /// `profile/canonical_schema.json` + the numeric id in
    /// `profile/identity.json` only for data written before the full
    /// snapshot existed; that path cannot recover derived-only humans that
    /// have neither file.
    pub fn load_human(&self, agent_id: &str) -> Result<Option<HumanBeing>, HumanStorageError> {
        let dir = self.human_dir(agent_id)?;
        if !dir.exists() {
            return Ok(None);
        }

        if let Some(snapshot_json) =
            self.read_encrypted_json(&dir.join("state/full_snapshot.json"))?
        {
            let human: HumanBeing = serde_json::from_value(snapshot_json)?;
            return Ok(Some(human));
        }

        let Some(schema_json) =
            self.read_encrypted_json(&dir.join("profile/canonical_schema.json"))?
        else {
            return Ok(None);
        };
        let schema: mk_core::human::schema::HumanSchema = serde_json::from_value(schema_json)?;

        let human_id = match self.read_encrypted_json(&dir.join("profile/identity.json"))? {
            Some(identity_json) => identity_json
                .get("human_id")
                .and_then(|v| v.as_str())
                .and_then(|s| s.trim_start_matches("HUM-").parse::<u64>().ok())
                .map(mk_core::human::HumanId::new)
                .unwrap_or(mk_core::human::HumanId::new(0)),
            None => mk_core::human::HumanId::new(0),
        };

        Ok(Some(HumanBeing::from_schema(human_id, schema)))
    }

    /// Validate a human ID to prevent path traversal
    fn validate_id(&self, agent_id: &str) -> Result<(), HumanStorageError> {
        if agent_id.is_empty() {
            return Err(HumanStorageError::InvalidId(
                "ID cannot be empty".to_string(),
            ));
        }

        // Only allow alphanumeric, underscore, and dash
        for c in agent_id.chars() {
            if !c.is_alphanumeric() && c != '_' && c != '-' {
                return Err(HumanStorageError::InvalidId(agent_id.to_string()));
            }
        }

        // Explicitly check for dots to prevent .. or .
        if agent_id.contains('.') {
            return Err(HumanStorageError::InvalidId(agent_id.to_string()));
        }

        Ok(())
    }

    /// Get the directory path for a specific human
    pub fn human_dir(&self, agent_id: &str) -> Result<PathBuf, HumanStorageError> {
        self.validate_id(agent_id)?;
        Ok(self.base_path.join(agent_id))
    }

    /// Create all sub-directories for a human
    pub fn create_human(&self, human: &HumanBeing) -> Result<(), HumanStorageError> {
        let dir = self.human_dir(human.agent_id())?;

        if dir.exists() {
            return Err(HumanStorageError::AlreadyExists(
                human.agent_id().to_string(),
            ));
        }

        // Create directory structure
        let subdirs = [
            "profile",
            "traits",
            "cognition",
            "social",
            "development",
            "reproduction",
            "memories/episodic",
            "memories/semantic",
            "memories/procedural",
            "state",
            "relationships",
            "events",
        ];

        for subdir in &subdirs {
            fs::create_dir_all(dir.join(subdir))?;
        }

        // Write initial data files
        self.write_profile(human)?;
        self.write_traits(human)?;
        self.write_cognition(human)?;
        self.write_social(human)?;
        self.write_development(human)?;
        self.write_reproduction(human)?;
        self.write_state(human)?;

        Ok(())
    }

    /// Update all storage files for a human
    pub fn update_human(&self, human: &HumanBeing) -> Result<(), HumanStorageError> {
        let dir = self.human_dir(human.agent_id())?;
        if !dir.exists() {
            return self.create_human(human);
        }

        self.write_profile(human)?;
        self.write_traits(human)?;
        self.write_cognition(human)?;
        self.write_social(human)?;
        self.write_development(human)?;
        self.write_reproduction(human)?;
        self.write_state(human)?;

        Ok(())
    }

    /// Delete a human's entire directory
    pub fn delete_human(&self, agent_id: &str) -> Result<(), HumanStorageError> {
        let dir = self.human_dir(agent_id)?;
        if dir.exists() {
            fs::remove_dir_all(&dir)?;
        }
        Ok(())
    }

    /// Check if a human exists in storage
    pub fn human_exists(&self, agent_id: &str) -> bool {
        match self.human_dir(agent_id) {
            Ok(dir) => dir.exists(),
            Err(_) => false,
        }
    }

    /// List all human agent IDs in storage
    pub fn list_humans(&self) -> Vec<String> {
        let mut humans = Vec::new();
        if let Ok(entries) = fs::read_dir(&self.base_path) {
            for entry in entries.flatten() {
                if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                    if let Some(name) = entry.file_name().to_str() {
                        humans.push(name.to_string());
                    }
                }
            }
        }
        humans.sort();
        humans
    }

    /// Write profile data (identity, genome, appearance)
    fn write_profile(&self, human: &HumanBeing) -> Result<(), HumanStorageError> {
        let dir = self.human_dir(human.agent_id())?.join("profile");

        // Identity
        let identity = serde_json::json!({
            "agent_id": human.agent_id(),
            "human_id": human.profile.human_id.to_string(),
            "biological_sex": format!("{:?}", human.biological_sex()),
            "birth_timestamp": human.profile.core_identity.birth_timestamp,
            "birthplace": {
                "location": human.profile.core_identity.birthplace.location,
                "latitude": human.profile.core_identity.birthplace.coordinates.latitude,
                "longitude": human.profile.core_identity.birthplace.coordinates.longitude,
                "locality": format!("{:?}", human.profile.core_identity.birthplace.locality),
            },
            "generation": format!("{:?}", human.profile.core_identity.generation),
            "neurotype": {
                "adhd": human.profile.core_identity.neurotype.adhd.is_some(),
                "autism": human.profile.core_identity.neurotype.autism.is_some(),
                "sensory_processing_sensitivity": human.profile.core_identity.neurotype.sensory_processing_sensitivity,
                "executive_dysfunction_bias": human.profile.core_identity.neurotype.executive_dysfunction_bias,
            },
            "status": format!("{:?}", human.profile.status),
        });
        self.write_encrypted_json(&dir.join("identity.json"), &identity)?;

        // Genome summary
        let genome = serde_json::json!({
            "heritable_stability": human.genetics.heritable_stability,
            "openness": human.genetics.openness,
            "extraversion": human.genetics.extraversion,
            "dopamine_base": human.genetics.dopamine_base,
            "serotonin_base": human.genetics.serotonin_base,
            "is_male": human.genetics.is_male,
            "expression_weight_a": human.genetics.expression_weight_a,
            "expression_weight_b": human.genetics.expression_weight_b,
        });
        self.write_encrypted_json(&dir.join("genome.json"), &genome)?;

        // Astrology
        let astrology = serde_json::json!({
            "sun_sign": format!("{:?}", human.profile.genome.astrology.sun_sign),
            "moon_sign": format!("{:?}", human.profile.genome.astrology.moon_sign),
            "ascendant": format!("{:?}", human.profile.genome.astrology.rising_sign),
        });
        self.write_encrypted_json(&dir.join("astrology.json"), &astrology)?;

        // Preserve the complete canonical source profile alongside the
        // derived projections above. This is the authoritative record of
        // everything unique about the human and lets future schema versions
        // rebuild derived runtime layers without losing original detail.
        if let Some(canonical_schema) = human.profile.canonical_schema() {
            let canonical = serde_json::to_value(canonical_schema)?;
            self.write_encrypted_json(&dir.join("canonical_schema.json"), &canonical)?;
        }

        if let Some(gemini_identity) = human.gemini_identity_json() {
            let identity: serde_json::Value = serde_json::from_str(gemini_identity)?;
            self.write_encrypted_json(&dir.join("gemini_identity.json"), &identity)?;
        }

        Ok(())
    }

    /// Write traits data (personality, temperament, drives)
    fn write_traits(&self, human: &HumanBeing) -> Result<(), HumanStorageError> {
        let dir = self.human_dir(human.agent_id())?.join("traits");

        // Temperament
        let temperament = serde_json::json!({
            "introversion_extroversion": human.profile.temperament_matrix.introversion_extroversion,
            "emotional_intensity": human.profile.temperament_matrix.emotional_intensity,
            "emotional_stability": human.profile.temperament_matrix.emotional_stability,
            "empathy": human.profile.temperament_matrix.empathy,
            "assertiveness": human.profile.temperament_matrix.assertiveness,
            "sensitivity_to_environment": human.profile.temperament_matrix.sensitivity_to_environment,
            "adaptability": human.profile.temperament_matrix.adaptability,
            "conscientiousness": human.profile.temperament_matrix.conscientiousness,
            "openness_to_experience": human.profile.temperament_matrix.openness_to_experience,
            "archetype": human.profile.temperament_matrix.archetype(),
        });
        self.write_encrypted_json(&dir.join("temperament.json"), &temperament)?;

        // Personality traits
        let traits = serde_json::json!({
            "emotional_traits": human.profile.personality_traits.emotional.traits.iter().map(|t| {
                serde_json::json!({
                    "name": t.trait_name,
                    "polarity": format!("{:?}", t.polarity),
                    "baseline": t.baseline_value,
                    "behavioral_expression": t.behavioral_expression,
                    "stress_expression": t.stress_expression,
                    "withdrawal_expression": t.withdrawal_expression,
                })
            }).collect::<Vec<_>>(),
            "cognitive_traits": human.profile.personality_traits.cognitive.traits.iter().map(|t| {
                serde_json::json!({
                    "name": t.trait_name,
                    "polarity": format!("{:?}", t.polarity),
                    "baseline": t.baseline_value,
                })
            }).collect::<Vec<_>>(),
            "motivational_traits": human.profile.personality_traits.motivational.traits.iter().map(|t| {
                serde_json::json!({
                    "name": t.trait_name,
                    "polarity": format!("{:?}", t.polarity),
                    "baseline": t.baseline_value,
                })
            }).collect::<Vec<_>>(),
        });
        self.write_encrypted_json(&dir.join("personality.json"), &traits)?;

        // Drives
        let drives = serde_json::json!({
            "primary_drive": human.profile.drive_weights.primary_drive(),
            "survival": human.profile.drive_weights.survival,
            "bonding": human.profile.drive_weights.bonding,
            "autonomy": human.profile.drive_weights.autonomy,
            "curiosity": human.profile.drive_weights.curiosity,
            "meaning": human.profile.drive_weights.meaning,
        });
        self.write_encrypted_json(&dir.join("drives.json"), &drives)?;

        // Stress and attachment
        let stress_attachment = serde_json::json!({
            "stress_response": {
                "threat_detection_threshold": human.profile.stress_response_profile.threat_detection_threshold,
                "withdrawal_activation_threshold": human.profile.stress_response_profile.withdrawal_activation_threshold,
                "recovery_half_life": human.profile.stress_response_profile.recovery_half_life,
            },
            "attachment": {
                "primary_pattern": format!("{:?}", human.profile.attachment_style.primary_pattern),
                "proximity_seeking_intensity": human.profile.attachment_style.proximity_seeking_intensity,
                "abandonment_sensitivity": human.profile.attachment_style.abandonment_sensitivity,
            },
            "hormonal_baseline": {
                "dopamine_variability": human.profile.hormonal_baseline_bias.dopamine_variability,
                "cortisol_sensitivity": human.profile.hormonal_baseline_bias.cortisol_sensitivity,
                "melatonin_irregularity": human.profile.hormonal_baseline_bias.melatonin_irregularity,
            },
        });
        self.write_encrypted_json(&dir.join("stress_attachment.json"), &stress_attachment)?;

        Ok(())
    }

    /// Write cognition data
    fn write_cognition(&self, human: &HumanBeing) -> Result<(), HumanStorageError> {
        let dir = self.human_dir(human.agent_id())?.join("cognition");

        let cognition = serde_json::json!({
            "mode": format!("{:?}", human.cognition.mode),
            "attention_capacity": human.cognition.attention_capacity,
            "hyperfocus_tendency": human.cognition.hyperfocus_tendency,
            "working_memory": human.cognition.working_memory,
            "task_initiation": human.cognition.task_initiation,
            "task_persistence": human.cognition.task_persistence,
            "associative_thinking": human.cognition.associative_thinking,
            "emotional_permeability": human.cognition.emotional_permeability,
            "exec_fatigue_rate": human.cognition.exec_fatigue_rate,
            "overload_threshold": human.cognition.overload_threshold,
            "cognitive_load": human.cognition.cognitive_load,
            "fatigue": human.cognition.fatigue,
        });
        self.write_encrypted_json(&dir.join("cognition.json"), &cognition)?;

        // Language
        let language = serde_json::json!({
            "mode": format!("{:?}", human.language.mode),
            "expressivity": human.language.expressivity,
            "comprehension": human.language.comprehension,
            "pragmatic_skill": human.language.pragmatic_skill,
            "vocabulary_richness": human.language.vocabulary_richness,
            "narrative_ability": human.language.narrative_ability,
        });
        self.write_encrypted_json(&dir.join("language.json"), &language)?;

        Ok(())
    }

    /// Write social data
    fn write_social(&self, human: &HumanBeing) -> Result<(), HumanStorageError> {
        let dir = self.human_dir(human.agent_id())?.join("social");

        let social = serde_json::json!({
            "attachment_security": human.social_systems.attachment_security,
            "social_trust": human.social_systems.social_trust,
            "conflict_resolution": human.social_systems.conflict_resolution,
            "cooperation": human.social_systems.cooperation,
            "empathic_accuracy": human.social_systems.empathic_accuracy,
            "boundary_detection": human.social_systems.boundary_detection,
            "loneliness_tolerance": human.social_systems.loneliness_tolerance,
            "group_identity": human.social_systems.group_identity,
            "jealousy_reactivity": human.social_systems.jealousy_reactivity,
            "abandonment_sensitivity": human.social_systems.abandonment_sensitivity,
        });
        self.write_encrypted_json(&dir.join("social_systems.json"), &social)?;

        // Culture
        let culture = serde_json::json!({
            "symbolic_capacity": human.culture.symbolic_capacity,
            "norm_retention": human.culture.norm_retention,
            "identity_strength": human.culture.identity_strength,
            "innovation_orientation": human.culture.innovation_orientation,
            "individualism": human.culture.individualism,
        });
        self.write_encrypted_json(&dir.join("culture.json"), &culture)?;

        Ok(())
    }

    /// Write development data
    fn write_development(&self, human: &HumanBeing) -> Result<(), HumanStorageError> {
        let dir = self.human_dir(human.agent_id())?.join("development");

        let development = serde_json::json!({
            "age_years": human.development.age_years,
            "maturity_index": human.development.maturity_index,
            "stage": format!("{:?}", human.development.stage),
            "cognitive_development": human.development.cognitive_development,
            "physical_development": human.development.physical_development,
            "emotional_maturity": human.development.emotional_maturity,
            "social_development": human.development.social_development,
        });
        self.write_encrypted_json(&dir.join("development.json"), &development)?;

        // Technology
        let technology = serde_json::json!({
            "innovation_bias": human.technology.innovation_bias,
            "tooling_aptitude": human.technology.tooling_aptitude,
            "abstraction_to_application": human.technology.abstraction_to_application,
            "systematic_thinking": human.technology.systematic_thinking,
            "risk_tolerance": human.technology.risk_tolerance,
            "learning_speed": human.technology.learning_speed,
        });
        self.write_encrypted_json(&dir.join("technology.json"), &technology)?;

        Ok(())
    }

    /// Write reproductive-systems data (sexual system, mate selection,
    /// lineage history: conception/birth records, sexual activity log)
    fn write_reproduction(&self, human: &HumanBeing) -> Result<(), HumanStorageError> {
        let dir = self.human_dir(human.agent_id())?.join("reproduction");
        fs::create_dir_all(&dir)?;

        let repro = &human.reproduction;
        let reproduction = serde_json::json!({
            "sexual_system": {
                "libido": repro.libido,
                "arousal": repro.arousal,
                "satisfaction": repro.satisfaction,
                "frustration": repro.frustration,
                "hormonal_influence": repro.hormonal_influence,
                "attraction_average": repro.attraction_average,
                "bonding_average": repro.bonding_average,
            },
            "mate_selection": {
                "preferred_age_min": repro.preferred_age_min,
                "preferred_age_max": repro.preferred_age_max,
                "preferred_age_ideal": repro.preferred_age_ideal,
                "genetic_compatibility_preference": repro.genetic_compatibility_preference,
                "social_status_preference": repro.social_status_preference,
                "intelligence_preference": repro.intelligence_preference,
                "courtship_behavior_count": repro.courtship_behavior_count,
                "relationship_history_count": repro.relationship_history_count,
            },
            "lineage": {
                "conception_history_count": repro.conception_history_count,
                "birth_record_count": repro.birth_record_count,
                "gamete_count": repro.gamete_count,
                "hereditary_condition_count": repro.hereditary_condition_count,
                "max_hereditary_condition_severity": repro.max_hereditary_condition_severity,
            },
            "fertility": {
                "fertility_level": repro.fertility_level,
                "conception_probability": repro.conception_probability,
                "gestation_week": repro.gestation_week,
                "sexual_activity_count": repro.sexual_activity_count,
            },
        });
        self.write_encrypted_json(&dir.join("reproduction.json"), &reproduction)?;

        Ok(())
    }

    /// Write current runtime state snapshot
    fn write_state(&self, human: &HumanBeing) -> Result<(), HumanStorageError> {
        let dir = self.human_dir(human.agent_id())?.join("state");
        let ts = lifecycle_timestamp(human);

        let state = serde_json::json!({
            "agent_id": human.agent_id(),
            "status": format!("{:?}", human.profile.status),
            "age": human.age(),
            "is_sapient": human.is_sapient(),
            "sapience_level": human.sapience_level(),
            "personality_archetype": human.profile.personality_archetype(),
            "primary_drive": human.profile.primary_drive(),
            "is_likely_neurodivergent": human.profile.is_likely_neurodivergent(),
            "position": {
                "row": human.position.row,
                "col": human.position.col,
            },
            "needs": {
                "glucose": human.needs.glucose,
                "hydration": human.needs.hydration,
                "fatigue": human.needs.fatigue,
                "hunger": human.needs.hunger,
                "thirst": human.needs.thirst,
                "status": format!("{:?}", human.needs.status),
            },
            "body": {
                "pulse": human.body.pulse,
                "blood_pressure_systolic": human.body.blood_pressure_systolic,
                "blood_pressure_diastolic": human.body.blood_pressure_diastolic,
                "sp_o2": human.body.sp_o2,
                "body_temperature_c": human.body.body_temperature_c,
                "bladder_pressure": human.body.bladder_pressure,
                "bowel_pressure": human.body.bowel_pressure,
                "hygiene": human.body.hygiene,
                "height_cm": human.body.height_cm,
                "weight_kg": human.body.weight_kg,
                "build": human.body.build,
                "hair_color": human.body.hair_color,
                "eye_color": human.body.eye_color,
            },
            "immune": {
                "system_stress": human.immune.system_stress,
                "autoimmunity_risk": human.immune.autoimmunity_risk,
                "active_pathogen_count": human.immune.active_pathogen_count,
            },
            "skin": {
                "cleanliness": human.skin.cleanliness,
                "integrity": human.skin.integrity,
                "infection_risk": human.skin.infection_risk,
            },
            "sensory": {
                "overall_pain_level": human.sensory.overall_pain_level,
                "visual_acuity": human.sensory.visual_acuity,
                "hearing_sensitivity": human.sensory.hearing_sensitivity,
                "body_awareness": human.sensory.body_awareness,
            },
            "attention": {
                "available": human.attention.available,
                "focus_level": human.attention.focus_level,
                "cognitive_load": human.attention.cognitive_load,
                "attention_fatigue": human.attention.attention_fatigue,
            },
            "consciousness": {
                "meta_cognition": human.consciousness.meta_cognition,
                "identity_continuity": human.consciousness.identity_continuity,
                "anxiety_baseline": human.consciousness.anxiety_baseline,
                "self_value": human.consciousness.self_value,
            },
            "timestamp": ts,
        });
        // Write the latest state
        self.write_encrypted_json(&dir.join("current_state.json"), &state)?;

        // Also write a timestamped snapshot
        let snapshot_path = dir.join(format!("state_{}.json", ts));
        self.write_encrypted_json(&snapshot_path, &state)?;

        // Cleanup old snapshots
        self.cleanup_snapshots(&dir, "state_", self.snapshot_retention)?;

        // If encryption is enabled, remove the plaintext current_state.json to avoid leaving
        // an unencrypted copy alongside encrypted snapshots. If encryption is not enabled,
        // keep the plaintext current_state.json for test and debugging convenience.
        if self.encryption.is_some() {
            let current_plain = dir.join("current_state.json");
            let _ = fs::remove_file(current_plain);
        }

        // Full-fidelity round-trip snapshot: every field of `HumanBeing`,
        // not just the derived summaries written above. This is what
        // `load_human` reconstructs from, so persistence survives process
        // restarts and rebuilds for every human (founders and offspring
        // alike), not only humans that happen to carry a `canonical_schema`.
        let full_snapshot = serde_json::to_value(human)?;
        self.write_encrypted_json(&dir.join("full_snapshot.json"), &full_snapshot)?;

        Ok(())
    }

    /// Clean up old snapshots in a directory
    fn cleanup_snapshots(
        &self,
        dir: &std::path::Path,
        prefix: &str,
        keep: usize,
    ) -> Result<(), HumanStorageError> {
        if !dir.exists() {
            return Ok(());
        }

        let mut snapshots = Vec::new();
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    if name.starts_with(prefix)
                        && (name.ends_with(".json") || name.ends_with(".enc.json"))
                    {
                        if let Ok(metadata) = entry.metadata() {
                            if let Ok(modified) = metadata.modified() {
                                snapshots.push((entry.path(), modified));
                            }
                        }
                    }
                }
            }
        }

        // Sort by modification time (oldest first)
        snapshots.sort_by_key(|&(_, modified)| modified);

        // Remove oldest if we have more than 'keep'
        if snapshots.len() > keep {
            let to_remove = snapshots.len() - keep;
            for snapshot in snapshots.iter().take(to_remove) {
                let _ = fs::remove_file(&snapshot.0);
                if let Some(stem) = snapshot.0.file_stem().and_then(|s| s.to_str()) {
                    let enc = snapshot.0.with_file_name(format!("{}.enc.json", stem));
                    let _ = fs::remove_file(enc);
                }
            }
        }

        Ok(())
    }

    /// Append a memory entry to a human's episodic memory
    pub fn append_episodic_memory(
        &self,
        agent_id: &str,
        event: &serde_json::Value,
    ) -> Result<(), HumanStorageError> {
        let path = self
            .human_dir(agent_id)?
            .join("memories")
            .join("episodic")
            .join("events.jsonl");

        let line = serde_json::to_string(event)?;
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)?;
        writeln!(file, "{}", line)?;
        Ok(())
    }

    /// Append a semantic memory entry
    pub fn append_semantic_memory(
        &self,
        agent_id: &str,
        knowledge: &serde_json::Value,
    ) -> Result<(), HumanStorageError> {
        let path = self
            .human_dir(agent_id)?
            .join("memories")
            .join("semantic")
            .join("knowledge.jsonl");

        let line = serde_json::to_string(knowledge)?;
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)?;
        writeln!(file, "{}", line)?;
        Ok(())
    }

    /// Append a procedural memory entry (learned skills)
    pub fn append_procedural_memory(
        &self,
        agent_id: &str,
        skill: &serde_json::Value,
    ) -> Result<(), HumanStorageError> {
        let path = self
            .human_dir(agent_id)?
            .join("memories")
            .join("procedural")
            .join("skills.jsonl");

        let line = serde_json::to_string(skill)?;
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)?;
        writeln!(file, "{}", line)?;
        Ok(())
    }

    /// Record a relationship with another human
    pub fn record_relationship(
        &self,
        agent_id: &str,
        other_id: &str,
        relationship: &serde_json::Value,
    ) -> Result<(), HumanStorageError> {
        self.validate_id(other_id)?;
        let path = self
            .human_dir(agent_id)?
            .join("relationships")
            .join(format!("{}.json", other_id));

        fs::write(&path, serde_json::to_string_pretty(relationship)?)?;
        Ok(())
    }

    /// Record an event in a human's event log
    pub fn record_event(
        &self,
        agent_id: &str,
        event: &serde_json::Value,
    ) -> Result<(), HumanStorageError> {
        let path = self
            .human_dir(agent_id)?
            .join("events")
            .join("event_log.jsonl");

        let line = serde_json::to_string(event)?;
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)?;
        writeln!(file, "{}", line)?;
        Ok(())
    }
}

/// Per-human storage profiles for organized access
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HumanProfileStorage {
    pub agent_id: String,
    pub identity_path: PathBuf,
    pub genome_path: PathBuf,
    pub astrology_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalityStorage {
    pub agent_id: String,
    pub temperament_path: PathBuf,
    pub personality_path: PathBuf,
    pub drives_path: PathBuf,
    pub stress_attachment_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CognitionStorage {
    pub agent_id: String,
    pub cognition_path: PathBuf,
    pub language_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DevelopmentStorage {
    pub agent_id: String,
    pub development_path: PathBuf,
    pub technology_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneticsStorage {
    pub agent_id: String,
    pub genome_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocialStorage {
    pub agent_id: String,
    pub social_systems_path: PathBuf,
    pub culture_path: PathBuf,
    pub relationships_dir: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CultureStorage {
    pub agent_id: String,
    pub culture_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageStorage {
    pub agent_id: String,
    pub language_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TechnologyStorage {
    pub agent_id: String,
    pub technology_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifecycleStorage {
    pub agent_id: String,
    pub development_path: PathBuf,
    pub state_path: PathBuf,
    pub events_dir: PathBuf,
}

impl HumanStorage {
    /// Get profile storage paths for a human
    pub fn profile_storage(
        &self,
        agent_id: &str,
    ) -> Result<HumanProfileStorage, HumanStorageError> {
        let dir = self.human_dir(agent_id)?.join("profile");
        Ok(HumanProfileStorage {
            agent_id: agent_id.to_string(),
            identity_path: dir.join("identity.json"),
            genome_path: dir.join("genome.json"),
            astrology_path: dir.join("astrology.json"),
        })
    }

    /// Get personality storage paths for a human
    pub fn personality_storage(
        &self,
        agent_id: &str,
    ) -> Result<PersonalityStorage, HumanStorageError> {
        let dir = self.human_dir(agent_id)?.join("traits");
        Ok(PersonalityStorage {
            agent_id: agent_id.to_string(),
            temperament_path: dir.join("temperament.json"),
            personality_path: dir.join("personality.json"),
            drives_path: dir.join("drives.json"),
            stress_attachment_path: dir.join("stress_attachment.json"),
        })
    }

    /// Get cognition storage paths for a human
    pub fn cognition_storage(&self, agent_id: &str) -> Result<CognitionStorage, HumanStorageError> {
        let dir = self.human_dir(agent_id)?.join("cognition");
        Ok(CognitionStorage {
            agent_id: agent_id.to_string(),
            cognition_path: dir.join("cognition.json"),
            language_path: dir.join("language.json"),
        })
    }

    /// Get development storage paths for a human
    pub fn development_storage(
        &self,
        agent_id: &str,
    ) -> Result<DevelopmentStorage, HumanStorageError> {
        let dir = self.human_dir(agent_id)?.join("development");
        Ok(DevelopmentStorage {
            agent_id: agent_id.to_string(),
            development_path: dir.join("development.json"),
            technology_path: dir.join("technology.json"),
        })
    }

    /// Get genetics storage paths for a human
    pub fn genetics_storage(&self, agent_id: &str) -> Result<GeneticsStorage, HumanStorageError> {
        let dir = self.human_dir(agent_id)?.join("profile");
        Ok(GeneticsStorage {
            agent_id: agent_id.to_string(),
            genome_path: dir.join("genome.json"),
        })
    }

    /// Get social storage paths for a human
    pub fn social_storage(&self, agent_id: &str) -> Result<SocialStorage, HumanStorageError> {
        let dir = self.human_dir(agent_id)?;
        Ok(SocialStorage {
            agent_id: agent_id.to_string(),
            social_systems_path: dir.join("social").join("social_systems.json"),
            culture_path: dir.join("social").join("culture.json"),
            relationships_dir: dir.join("relationships"),
        })
    }

    /// Get culture storage paths for a human
    pub fn culture_storage(&self, agent_id: &str) -> Result<CultureStorage, HumanStorageError> {
        let dir = self.human_dir(agent_id)?.join("social");
        Ok(CultureStorage {
            agent_id: agent_id.to_string(),
            culture_path: dir.join("culture.json"),
        })
    }

    /// Get language storage paths for a human
    pub fn language_storage(&self, agent_id: &str) -> Result<LanguageStorage, HumanStorageError> {
        let dir = self.human_dir(agent_id)?.join("cognition");
        Ok(LanguageStorage {
            agent_id: agent_id.to_string(),
            language_path: dir.join("language.json"),
        })
    }

    /// Get technology storage paths for a human
    pub fn technology_storage(
        &self,
        agent_id: &str,
    ) -> Result<TechnologyStorage, HumanStorageError> {
        let dir = self.human_dir(agent_id)?.join("development");
        Ok(TechnologyStorage {
            agent_id: agent_id.to_string(),
            technology_path: dir.join("technology.json"),
        })
    }

    /// Get lifecycle storage paths for a human
    pub fn lifecycle_storage(&self, agent_id: &str) -> Result<LifecycleStorage, HumanStorageError> {
        let dir = self.human_dir(agent_id)?;
        Ok(LifecycleStorage {
            agent_id: agent_id.to_string(),
            development_path: dir.join("development").join("development.json"),
            state_path: dir.join("state").join("current_state.json"),
            events_dir: dir.join("events"),
        })
    }
}

/// Build a deterministic storage timestamp from human state.
fn lifecycle_timestamp(human: &HumanBeing) -> String {
    let now_millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);

    format!(
        "birth-{}-age-{}-ts-{}",
        human.profile.core_identity.birth_timestamp,
        human.age(),
        now_millis,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::humans::{BiologicalSex, HumanBeing};

    #[test]
    fn test_storage_creates_directory_structure() {
        let temp_dir = std::env::temp_dir().join("mk_test_storage");
        let _ = fs::remove_dir_all(&temp_dir); // Clean up first

        let storage = HumanStorage::new_unencrypted(&temp_dir);
        storage.init().unwrap();

        let human = HumanBeing::new("test_human".to_string(), BiologicalSex::Male);
        storage.create_human(&human).unwrap();

        // Verify directory structure
        let human_dir = temp_dir.join("test_human");
        assert!(human_dir.exists());
        assert!(human_dir.join("profile").exists());
        assert!(human_dir.join("traits").exists());
        assert!(human_dir.join("cognition").exists());
        assert!(human_dir.join("social").exists());
        assert!(human_dir.join("development").exists());
        assert!(human_dir.join("memories/episodic").exists());
        assert!(human_dir.join("memories/semantic").exists());
        assert!(human_dir.join("memories/procedural").exists());
        assert!(human_dir.join("state").exists());
        assert!(human_dir.join("relationships").exists());
        assert!(human_dir.join("events").exists());

        // Verify files exist
        assert!(human_dir.join("profile/identity.json").exists());
        assert!(human_dir.join("profile/canonical_schema.json").exists());
        assert!(human_dir.join("traits/temperament.json").exists());
        assert!(human_dir.join("cognition/cognition.json").exists());
        assert!(human_dir.join("development/development.json").exists());
        assert!(human_dir.join("state/current_state.json").exists());

        // Clean up
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn invalid_explicit_key_is_refused_and_writes_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path().join("store");
        for bad in ["", "abc", &"z".repeat(64), &"ab".repeat(31)] {
            let result = HumanStorage::try_with_key_hex(&base, bad);
            assert!(
                matches!(result, Err(HumanStorageError::IoError(ref m)) if m.contains("storage key")),
                "key {bad:?} must be refused"
            );
        }
        assert!(!base.exists(), "a refused key must not touch the disk");
    }

    #[test]
    fn explicit_key_storage_encrypts_sensitive_files() {
        let dir = tempfile::tempdir().unwrap();
        let storage = HumanStorage::try_with_key_hex(dir.path(), &"ab".repeat(32)).unwrap();
        assert!(storage.encryption.is_some());
        storage.init().unwrap();

        let human = HumanBeing::new("keyed_human".to_string(), BiologicalSex::Female);
        storage.create_human(&human).unwrap();
        let human_dir = dir.path().join("keyed_human");
        assert!(human_dir.join("profile/identity.enc.json").exists());
        assert!(!human_dir.join("profile/identity.json").exists());
    }

    #[test]
    fn test_storage_encryption() {
        let temp_dir = std::env::temp_dir().join("mk_test_encryption");
        let _ = fs::remove_dir_all(&temp_dir);

        let storage = HumanStorage::new(&temp_dir);
        storage.init().unwrap();

        let human = HumanBeing::new("enc_human".to_string(), BiologicalSex::Female);
        storage.create_human(&human).unwrap();

        let human_dir = temp_dir.join("enc_human");

        // Identity should be encrypted
        assert!(!human_dir.join("profile/identity.json").exists());
        assert!(human_dir.join("profile/identity.enc.json").exists());

        // Note: Encrypted read functionality removed in refactoring
        // This test verifies encryption is applied at write time
        // Full encrypted read/write cycle is covered by integration tests

        // Temperament should be encrypted
        assert!(!human_dir.join("traits/temperament.json").exists());
        assert!(human_dir.join("traits/temperament.enc.json").exists());

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_storage_snapshot_retention() {
        let temp_dir = std::env::temp_dir().join("mk_test_retention");
        let _ = fs::remove_dir_all(&temp_dir);

        let mut storage = HumanStorage::new_unencrypted(&temp_dir);
        storage.snapshot_retention = 2; // Only keep 2 snapshots
        storage.init().unwrap();

        let human = HumanBeing::new("ret_human".to_string(), BiologicalSex::Male);
        storage.create_human(&human).unwrap();

        // Write state multiple times
        storage.write_state(&human).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(1100)); // Ensure different timestamp
        storage.write_state(&human).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(1100));
        storage.write_state(&human).unwrap();

        let state_dir = temp_dir.join("ret_human/state");
        let entries = fs::read_dir(state_dir).unwrap();
        let files: Vec<_> = entries
            .flatten()
            .map(|e| e.file_name().to_str().unwrap().to_string())
            .filter(|n| n.starts_with("state_"))
            .collect();

        // Should have exactly 2 timestamped state files
        assert_eq!(files.len(), 2);

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_storage_round_trips_private_autonomy_state() {
        let temp_dir = std::env::temp_dir().join("mk_test_autonomy_storage");
        let _ = fs::remove_dir_all(&temp_dir);
        let storage = HumanStorage::new_unencrypted(&temp_dir);
        storage.init().unwrap();

        let mut human = HumanBeing::new("autonomy_storage".to_string(), BiologicalSex::Female);
        human.set_runtime_position(crate::humans::GridPosition::new(7, 11));
        human.autonomous_mind.total_reward = 1.25;
        human.autonomous_mind.experiences.push_back(
            crate::humans::autonomy::AutonomousExperience {
                tick: 4,
                action: crate::humans::ActionKind::Explore,
                reward: 0.25,
                fitness_after: 0.7,
                action_success: true,
                caloric_access: 0.8,
                hydration_access: 0.6,
                shelter_quality: 0.4,
                hazard_index: 0.1,
                resource_abundance: 0.9,
            },
        );
        storage.create_human(&human).unwrap();

        let restored = storage.load_human("autonomy_storage").unwrap().unwrap();
        assert_eq!(restored.position, human.position);
        assert_eq!(restored.autonomous_mind.total_reward, 1.25);
        assert_eq!(
            restored.autonomous_mind.experiences,
            human.autonomous_mind.experiences
        );

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_storage_lists_humans() {
        let temp_dir = std::env::temp_dir().join("mk_test_list");
        let _ = fs::remove_dir_all(&temp_dir);

        let storage = HumanStorage::new(&temp_dir);
        storage.init().unwrap();

        let h1 = HumanBeing::new("alice".to_string(), BiologicalSex::Female);
        let h2 = HumanBeing::new("bob".to_string(), BiologicalSex::Male);
        storage.create_human(&h1).unwrap();
        storage.create_human(&h2).unwrap();

        let humans = storage.list_humans();
        assert_eq!(humans.len(), 2);
        assert!(humans.contains(&"alice".to_string()));
        assert!(humans.contains(&"bob".to_string()));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_path_traversal_prevention() {
        let temp_dir = std::env::temp_dir().join("mk_test_traversal");
        let _ = fs::remove_dir_all(&temp_dir);
        let storage = HumanStorage::new(&temp_dir);
        storage.init().unwrap();

        // Test invalid IDs
        assert!(storage.human_dir("..").is_err());
        assert!(storage.human_dir("../evil").is_err());
        assert!(storage.human_dir("folder/sub").is_err());
        assert!(storage.human_dir("human.dir").is_err());
        assert!(storage.human_dir("").is_err());

        // Test relationship traversal
        assert!(storage
            .record_relationship("alice", "../../evil", &serde_json::json!({}))
            .is_err());

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
