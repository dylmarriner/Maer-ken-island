//! Human Registry - Collection management for human beings
//!
//! This module provides the registry for managing multiple humans
//! without integrating into the main world simulation

use super::{BiologicalSex, HumanBeing};
use crate::io::{HumanStorage, HumanStorageError};
use serde::{Deserialize, Serialize};

/// Registry for managing human beings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HumanRegistry {
    humans: Vec<HumanBeing>,
    next_agent_id: u32,
    /// Optional persistent storage for human folders
    #[serde(skip)]
    storage: Option<HumanStorage>,
}

impl HumanRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            humans: Vec::new(),
            next_agent_id: 1,
            storage: None,
        }
    }

    /// Create a new registry with persistent storage, reloading any humans
    /// already present on disk so in-world state survives process restarts
    /// and rebuilds instead of being re-seeded (and colliding) every time.
    pub fn with_storage(storage: HumanStorage) -> Result<Self, HumanStorageError> {
        let mut registry = Self::new();
        storage.init()?;
        for agent_id in storage.list_humans() {
            if let Some(human) = storage.load_human(&agent_id)? {
                registry.humans.push(human);
            }
        }
        registry.storage = Some(storage);
        Ok(registry)
    }

    /// Seed the canonical first humans exactly once. Their folders are
    /// created through the same path used for every future human or birth.
    pub fn seed_founders(&mut self) -> Result<(), HumanStorageError> {
        if self.get_human("Gem-D").is_none() {
            self.add_human(HumanBeing::gem_d_founder())?;
        }
        if self.get_human("Gem-K").is_none() {
            self.add_human(HumanBeing::gem_k_founder())?;
        }
        Ok(())
    }

    /// Add a human to the registry (and create their folder if storage is enabled)
    pub fn add_human(&mut self, human: HumanBeing) -> Result<(), HumanStorageError> {
        let _agent_id = human.agent_id().to_string();

        // Create human folder if storage is enabled
        if let Some(ref storage) = self.storage {
            storage.create_human(&human)?;
        }

        self.humans.push(human);
        Ok(())
    }

    /// Add a human without creating storage (for in-memory only)
    pub fn add_human_no_storage(&mut self, human: HumanBeing) {
        self.humans.push(human);
    }

    /// Set storage backend
    pub fn set_storage(&mut self, storage: HumanStorage) -> Result<(), HumanStorageError> {
        storage.init()?;
        self.storage = Some(storage);
        // Reconcile snapshot-loaded humans with the persistent store. A
        // resumed snapshot already contains the authoritative state, so an
        // existing profile must be updated rather than created a second time.
        for human in &self.humans {
            let storage = self.storage.as_ref().unwrap();
            if storage.human_exists(human.agent_id()) {
                storage.update_human(human)?;
            } else {
                storage.create_human(human)?;
            }
        }
        Ok(())
    }

    /// Enable/disable storage
    pub fn has_storage(&self) -> bool {
        self.storage.is_some()
    }

    /// Sync human data to storage
    pub fn sync_to_storage(&self) {
        if let Some(ref storage) = self.storage {
            for human in &self.humans {
                let _ = storage.update_human(human);
            }
        }
    }

    /// Sync specific human to storage
    pub fn sync_human_to_storage(&self, agent_id: &str) {
        if let Some(ref storage) = self.storage {
            if let Some(human) = self.get_human(agent_id) {
                let _ = storage.update_human(human);
            }
        }
    }

    /// Record a life event (birth, reproduction, death, ...) to a human's
    /// personal event log, if storage is enabled. No-op otherwise.
    pub fn record_event(&self, agent_id: &str, event: &serde_json::Value) {
        if let Some(ref storage) = self.storage {
            let _ = storage.record_event(agent_id, event);
        }
    }

    /// Delete human from storage
    pub fn delete_from_storage(&self, agent_id: &str) {
        if let Some(ref storage) = self.storage {
            let _ = storage.delete_human(agent_id);
        }
    }

    /// List all humans in storage
    pub fn list_stored_humans(&self) -> Vec<String> {
        if let Some(ref storage) = self.storage {
            storage.list_humans()
        } else {
            Vec::new()
        }
    }

    /// Check if human exists in storage
    pub fn human_in_storage(&self, agent_id: &str) -> bool {
        if let Some(ref storage) = self.storage {
            storage.human_exists(agent_id)
        } else {
            false
        }
    }

    /// Reserve the next auto-generated agent id (`human_<n>`).
    pub fn allocate_agent_id(&mut self) -> String {
        let agent_id = format!("human_{}", self.next_agent_id);
        self.next_agent_id += 1;
        agent_id
    }

    /// Create and add a new human with auto-generated ID (and create folder if storage enabled)
    pub fn create_human(
        &mut self,
        biological_sex: BiologicalSex,
    ) -> Result<&HumanBeing, HumanStorageError> {
        let agent_id = self.allocate_agent_id();
        let human = HumanBeing::new(agent_id.clone(), biological_sex);

        // Create human folder if storage is enabled
        if let Some(ref storage) = self.storage {
            storage.create_human(&human)?;
        }

        self.humans.push(human);
        Ok(self.humans.last().unwrap())
    }

    /// Create and add a human with a specific name (and create folder if storage enabled)
    pub fn create_named_human(
        &mut self,
        agent_id: String,
        biological_sex: BiologicalSex,
    ) -> Result<&HumanBeing, HumanStorageError> {
        let human = HumanBeing::new(agent_id.clone(), biological_sex);

        // Create human folder if storage is enabled
        if let Some(ref storage) = self.storage {
            storage.create_human(&human)?;
        }

        self.humans.push(human);
        Ok(self.humans.last().unwrap())
    }

    /// Get a human by agent ID
    pub fn get_human(&self, agent_id: &str) -> Option<&HumanBeing> {
        self.humans.iter().find(|h| h.agent_id() == agent_id)
    }

    /// Get a mutable reference to a human by agent ID
    pub fn get_human_mut(&mut self, agent_id: &str) -> Option<&mut HumanBeing> {
        self.humans.iter_mut().find(|h| h.agent_id() == agent_id)
    }

    /// Remove a human by agent ID.
    ///
    /// Refuses unconditionally (Reverence Veto — see
    /// `crate::governance`) if `agent_id` names a protected creator
    /// entity (Gem-D/Gem-K), regardless of caller.
    pub fn remove_human(
        &mut self,
        agent_id: &str,
    ) -> Result<Option<HumanBeing>, crate::governance::ReverenceVetoViolation> {
        crate::governance::check(agent_id)?;
        if let Some(pos) = self.humans.iter().position(|h| h.agent_id() == agent_id) {
            // Also delete from storage
            self.delete_from_storage(agent_id);
            Ok(Some(self.humans.remove(pos)))
        } else {
            Ok(None)
        }
    }

    /// Get all humans
    pub fn get_all_humans(&self) -> &[HumanBeing] {
        &self.humans
    }

    /// Get all humans mutably
    pub fn get_all_humans_mut(&mut self) -> &mut Vec<HumanBeing> {
        &mut self.humans
    }

    /// Get count of humans
    pub fn count(&self) -> usize {
        self.humans.len()
    }

    /// Iterate over all humans
    pub fn iter(&self) -> impl Iterator<Item = &HumanBeing> {
        self.humans.iter()
    }

    /// Iterate over all humans (mutable)
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut HumanBeing> {
        self.humans.iter_mut()
    }

    /// Get next human ID
    pub fn next_human_id(&self) -> u64 {
        self.next_agent_id as u64
    }

    /// Get total population count
    pub fn population_count(&self) -> usize {
        self.humans.len()
    }

    /// Get humans by biological sex
    pub fn get_humans_by_sex(&self, sex: BiologicalSex) -> Vec<&HumanBeing> {
        self.humans
            .iter()
            .filter(|h| {
                let human_sex = h.biological_sex();
                matches!(
                    (human_sex, &sex),
                    (BiologicalSex::Male, BiologicalSex::Male)
                        | (BiologicalSex::Female, BiologicalSex::Female)
                        | (BiologicalSex::Neutral, BiologicalSex::Neutral)
                )
            })
            .collect()
    }

    /// Clear all humans
    pub fn clear(&mut self) {
        self.humans.clear();
        self.next_agent_id = 1;
    }
}

impl Default for HumanRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_creation() {
        let registry = HumanRegistry::new();
        assert_eq!(registry.population_count(), 0);
    }

    #[test]
    fn test_add_human() {
        let mut registry = HumanRegistry::new();
        let human = HumanBeing::new("test_human".to_string(), BiologicalSex::Male);
        registry.add_human(human).unwrap();
        assert_eq!(registry.population_count(), 1);
    }

    #[test]
    fn persistent_registry_seeds_gem_founders() {
        let temp_dir = std::env::temp_dir().join("mk_test_founders");
        let _ = std::fs::remove_dir_all(&temp_dir);
        let storage = HumanStorage::new_unencrypted(&temp_dir);
        let mut registry = HumanRegistry::with_storage(storage).unwrap();

        registry.seed_founders().unwrap();

        assert_eq!(registry.population_count(), 2);
        assert!(registry.human_in_storage("Gem-D"));
        assert!(registry.human_in_storage("Gem-K"));
        let gem_d = registry.get_human("Gem-D").unwrap();
        let gem_k = registry.get_human("Gem-K").unwrap();
        assert_ne!(
            gem_d.profile.core_identity.birth_timestamp,
            gem_k.profile.core_identity.birth_timestamp
        );
        assert_ne!(
            gem_d.profile.temperament_matrix,
            gem_k.profile.temperament_matrix
        );
        assert_ne!(gem_d.profile.drive_weights, gem_k.profile.drive_weights);
        assert!(temp_dir
            .join("Gem-D/profile/canonical_schema.json")
            .exists());
        assert!(temp_dir
            .join("Gem-K/profile/canonical_schema.json")
            .exists());
        assert!(temp_dir.join("Gem-D/profile/gemini_identity.json").exists());
        assert!(temp_dir.join("Gem-K/profile/gemini_identity.json").exists());

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_create_human() {
        let mut registry = HumanRegistry::new();
        registry.create_human(BiologicalSex::Female).unwrap();
        assert_eq!(registry.population_count(), 1);
        assert!(registry.get_human("human_1").is_some());
    }

    #[test]
    fn test_create_named_human() {
        let mut registry = HumanRegistry::new();
        registry
            .create_named_human("alice".to_string(), BiologicalSex::Female)
            .unwrap();
        let human = registry.get_human("alice").unwrap();
        assert_eq!(human.agent_id(), "alice");
    }

    #[test]
    fn test_remove_human() {
        let mut registry = HumanRegistry::new();
        registry
            .create_named_human("bob".to_string(), BiologicalSex::Male)
            .unwrap();
        assert_eq!(registry.population_count(), 1);

        let removed = registry.remove_human("bob").unwrap();
        assert!(removed.is_some());
        assert_eq!(registry.population_count(), 0);
    }

    #[test]
    fn reverence_veto_refuses_to_remove_creator_entities() {
        let mut registry = HumanRegistry::new();
        registry
            .create_named_human("Gem-D".to_string(), BiologicalSex::Female)
            .unwrap();
        assert_eq!(registry.population_count(), 1);

        let result = registry.remove_human("Gem-D");
        assert!(result.is_err());
        // Unaffected: the veto refused before touching the registry.
        assert_eq!(registry.population_count(), 1);
        assert!(registry.get_human("Gem-D").is_some());

        // Case-insensitive, per crate::governance::is_protected_creator_entity.
        assert!(registry.remove_human("gem-d").is_err());
    }

    #[test]
    fn test_get_humans_by_sex() {
        let mut registry = HumanRegistry::new();
        registry
            .create_named_human("male1".to_string(), BiologicalSex::Male)
            .unwrap();
        registry
            .create_named_human("female1".to_string(), BiologicalSex::Female)
            .unwrap();
        registry
            .create_named_human("male2".to_string(), BiologicalSex::Male)
            .unwrap();

        let males = registry.get_humans_by_sex(BiologicalSex::Male);
        let females = registry.get_humans_by_sex(BiologicalSex::Female);

        assert_eq!(males.len(), 2, "Expected 2 males, got {}", males.len());
        assert_eq!(females.len(), 1, "Expected 1 female, got {}", females.len());
    }

    #[test]
    fn test_iter() {
        let mut registry = HumanRegistry::new();
        registry
            .create_named_human("human1".to_string(), BiologicalSex::Male)
            .unwrap();
        registry
            .create_named_human("human2".to_string(), BiologicalSex::Female)
            .unwrap();
        let ids: Vec<_> = registry.iter().map(|h| h.agent_id().to_string()).collect();
        assert_eq!(ids, vec!["human1", "human2"]);
    }

    #[test]
    fn test_clear() {
        let mut registry = HumanRegistry::new();
        registry.create_human(BiologicalSex::Male).unwrap();
        registry.create_human(BiologicalSex::Female).unwrap();
        assert_eq!(registry.population_count(), 2);

        registry.clear();
        assert_eq!(registry.population_count(), 0);
    }
}
