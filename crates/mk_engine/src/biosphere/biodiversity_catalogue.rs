//! Biodiversity Catalogue for Maer'Ken Biosphere
//!
//! Implements ingestion and validation of the 20,000-entry starting biodiversity catalogue.
//! This remains a population-field species/clade catalogue at load/validation
//! time — every entry's `forbidden_state_flags` still documents the original
//! `no_individuals` constraint, and `validate()` still enforces that string's
//! presence, so loading/parsing behavior here is unchanged.
//!
//! **Superseded for the ~3,500 `major_group == "animal"` entries only**, per
//! explicit user direction: those now get a small, hard-capped set of
//! individually-tracked male/female pairs with real sexual reproduction —
//! see [`crate::biosphere::catalogue_individuals`]. That module reads this
//! catalogue's real `id`/`major_group` fields but does not modify or bypass
//! this file's loading/validation; the two coexist deliberately. The
//! ~6,500 plant/fungal/microbial entries are unaffected and remain strictly
//! population-field-only, reproducing via their real documented
//! `reproduction_strategy` (spores, seeds, division, etc.), not individually
//! tracked, and not sexually paired: no memory, no goals, no planning, no
//! symbols, no teaching, no tools, no culture, no sapience leakage.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

/// Single biodiversity catalogue entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiodiversityEntry {
    /// Deterministic catalogue id
    pub id: String,
    /// Major group: animal, plant, fungi, microbial
    pub major_group: String,
    /// High-level bucket category
    pub category: String,
    /// Unique constructed name
    pub common_name: String,
    /// Mutated biology-root taxon style
    pub canonical_taxon: String,
    /// Lineage stem
    pub lineage_stem: String,
    /// Clade classification
    pub clade: String,
    /// Trophic role: producer, consumer, apex, decomposer, base layer
    pub trophic_role: String,
    /// Diet/resource intake for simulation
    pub diet_or_resource_use: String,
    /// Edible or resource detail
    pub edible_or_resource_detail: String,
    /// Anatomy/growth morphology
    pub body_or_growth_form: String,
    /// Size classification
    pub size_class: String,
    /// Length or height range
    pub length_or_height_range: String,
    /// Mass or biomass range
    pub mass_or_biomass_range: String,
    /// Habitat type
    pub habitat: String,
    /// Biome classification
    pub biome: String,
    /// Moisture band
    pub moisture_band: String,
    /// Temperature band
    pub temperature_band: String,
    /// Salinity band
    pub salinity_band: String,
    /// Movement or growth strategy
    pub movement_or_growth_strategy: String,
    /// Reproduction strategy
    pub reproduction_strategy: String,
    /// Reproduction rate
    pub reproduction_rate: String,
    /// Maturation time
    pub maturation_time: String,
    /// Lifespan or turnover
    pub lifespan_or_turnover: String,
    /// Population pattern
    pub population_pattern: String,
    /// Defense or resilience
    pub defense_or_resilience: String,
    /// Attack or competition
    pub attack_or_competition: String,
    /// Sensory or response mode
    pub sensory_or_response_mode: String,
    /// Symbiosis relationships
    pub symbiosis: String,
    /// Ecological function
    pub ecological_function: String,
    /// Nutrient cycle role
    pub nutrient_cycle_role: String,
    /// Seasonality
    pub seasonality: String,
    /// Climate sensitivity
    pub climate_sensitivity: String,
    /// Disease sensitivity
    pub disease_sensitivity: String,
    /// Extinction sensitivity
    pub extinction_sensitivity: String,
    /// Capped animal intelligence ceiling (0 for non-animal)
    pub intelligence_index_ceiling: f64,
    /// MK phase scope
    pub mk_phase_scope: String,
    /// Ledger links for conservation integration
    pub ledger_links: String,
    /// UI render hint
    pub ui_render_hint: String,
    /// Fields that must not be implemented in MK-I
    pub forbidden_state_flags: String,
    /// Quick filtering tags for code/UI
    pub implementation_tags: String,
    /// Description
    pub description: String,
}

impl BiodiversityEntry {
    /// Validate entry meets MK-I constraints
    pub fn validate(&self) -> Result<(), String> {
        // Ensure forbidden state flags are present
        if !self.forbidden_state_flags.contains("no_individuals") {
            return Err(format!(
                "Entry {} missing no_individuals flag: {}",
                self.id, self.forbidden_state_flags
            ));
        }

        // Ensure intelligence ceiling is appropriate for major group
        if self.major_group == "animal" && self.intelligence_index_ceiling < 0.0 {
            return Err(format!(
                "Entry {} has negative intelligence ceiling for animal: {}",
                self.id, self.intelligence_index_ceiling
            ));
        }

        if self.major_group != "animal" && self.intelligence_index_ceiling != 0.0 {
            return Err(format!(
                "Entry {} has non-zero intelligence ceiling for non-animal: {}",
                self.id, self.intelligence_index_ceiling
            ));
        }

        // Ensure trophic role is valid
        let valid_trophic_roles = vec![
            "producer",
            "primary_consumer",
            "secondary_consumer",
            "apex_consumer",
            "decomposer",
            "base_layer",
            "mixed_consumer",
            "marine_consumer",
            "aerial_consumer",
            "producer/resource_pulse",
            "producer/storage_organ",
        ];

        let role_valid = valid_trophic_roles
            .iter()
            .any(|valid| self.trophic_role.contains(valid));

        if !role_valid {
            return Err(format!(
                "Entry {} has invalid trophic role: {}",
                self.id, self.trophic_role
            ));
        }

        Ok(())
    }
}

/// Loaded biodiversity catalogue
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BiodiversityCatalogue {
    /// All catalogue entries
    pub entries: Vec<BiodiversityEntry>,
    /// Index by ID
    pub by_id: HashMap<String, BiodiversityEntry>,
    /// Index by category
    pub by_category: HashMap<String, Vec<BiodiversityEntry>>,
    /// Index by major group
    pub by_major_group: HashMap<String, Vec<BiodiversityEntry>>,
}

impl BiodiversityCatalogue {
    /// Load catalogue from JSONL file
    pub fn load_from_jsonl(path: &Path) -> Result<Self, String> {
        let file = File::open(path).map_err(|e| format!("Failed to open file: {}", e))?;
        let reader = BufReader::new(file);

        let mut entries = Vec::new();
        let mut by_id = HashMap::new();
        let mut by_category = HashMap::new();
        let mut by_major_group = HashMap::new();

        for (line_num, line_result) in reader.lines().enumerate() {
            let line =
                line_result.map_err(|e| format!("Failed to read line {}: {}", line_num + 1, e))?;

            if line.trim().is_empty() {
                continue;
            }

            let entry: BiodiversityEntry = serde_json::from_str(&line)
                .map_err(|e| format!("Failed to parse line {}: {}", line_num + 1, e))?;

            // Validate entry
            entry
                .validate()
                .map_err(|e| format!("Validation error at line {}: {}", line_num + 1, e))?;

            // Build indices
            by_id.insert(entry.id.clone(), entry.clone());

            by_category
                .entry(entry.category.clone())
                .or_insert_with(Vec::new)
                .push(entry.clone());

            by_major_group
                .entry(entry.major_group.clone())
                .or_insert_with(Vec::new)
                .push(entry.clone());

            entries.push(entry);
        }

        Ok(Self {
            entries,
            by_id,
            by_category,
            by_major_group,
        })
    }

    /// Get entry by ID
    pub fn get_by_id(&self, id: &str) -> Option<&BiodiversityEntry> {
        self.by_id.get(id)
    }

    /// Get entries by category
    pub fn get_by_category(&self, category: &str) -> Option<&[BiodiversityEntry]> {
        self.by_category.get(category).map(|v| v.as_slice())
    }

    /// Get entries by major group
    pub fn get_by_major_group(&self, major_group: &str) -> Option<&[BiodiversityEntry]> {
        self.by_major_group.get(major_group).map(|v| v.as_slice())
    }

    /// Get total entry count
    pub fn count(&self) -> usize {
        self.entries.len()
    }

    /// Get category counts
    pub fn category_counts(&self) -> HashMap<String, usize> {
        self.by_category
            .iter()
            .map(|(cat, entries)| (cat.clone(), entries.len()))
            .collect()
    }

    /// Get major group counts
    pub fn major_group_counts(&self) -> HashMap<String, usize> {
        self.by_major_group
            .iter()
            .map(|(group, entries)| (group.clone(), entries.len()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("crates/")
            .parent()
            .expect("repo root")
            .to_path_buf()
    }

    #[test]
    fn load_catalogue_from_jsonl() {
        let catalogue_path = repo_root().join(
            "assets/biodiversity_catalogue_10000_bundle/maerken_biodiversity_catalogue_10000.jsonl",
        );

        if !catalogue_path.exists() {
            println!(
                "Skipping test: catalogue file not found at {:?}",
                catalogue_path
            );
            return;
        }

        let catalogue = BiodiversityCatalogue::load_from_jsonl(&catalogue_path)
            .expect("Failed to load catalogue");

        assert_eq!(
            catalogue.count(),
            20000,
            "Expected 20000 entries (10000 wildlife + 10000 flora-class)"
        );
    }

    #[test]
    fn catalogue_indices_consistent() {
        let catalogue_path = repo_root().join(
            "assets/biodiversity_catalogue_10000_bundle/maerken_biodiversity_catalogue_10000.jsonl",
        );

        if !catalogue_path.exists() {
            println!(
                "Skipping test: catalogue file not found at {:?}",
                catalogue_path
            );
            return;
        }

        let catalogue = BiodiversityCatalogue::load_from_jsonl(&catalogue_path)
            .expect("Failed to load catalogue");

        // Verify ID index
        for entry in &catalogue.entries {
            assert_eq!(
                catalogue.get_by_id(&entry.id).unwrap().id,
                entry.id,
                "ID index mismatch for {}",
                entry.id
            );
        }

        // Verify category index
        let _category_counts = catalogue.category_counts();
        let mut total_by_category: usize = 0;
        for entries in catalogue.by_category.values() {
            total_by_category += entries.len();
        }
        assert_eq!(
            total_by_category,
            catalogue.count(),
            "Category index total mismatch"
        );

        // Verify major group index
        let _major_group_counts = catalogue.major_group_counts();
        let mut total_by_group: usize = 0;
        for entries in catalogue.by_major_group.values() {
            total_by_group += entries.len();
        }
        assert_eq!(
            total_by_group,
            catalogue.count(),
            "Major group index total mismatch"
        );
    }

    #[test]
    fn entry_validation_rejects_invalid_intelligence_ceiling() {
        let entry = BiodiversityEntry {
            id: "TEST-001".to_string(),
            major_group: "animal".to_string(),
            intelligence_index_ceiling: -1.0,
            forbidden_state_flags: "no_individuals;no_memory".to_string(),
            trophic_role: "primary_consumer".to_string(),
            category: "Test".to_string(),
            common_name: "Test Species".to_string(),
            canonical_taxon: "Testus testus".to_string(),
            lineage_stem: "Testus".to_string(),
            clade: "test".to_string(),
            diet_or_resource_use: "test".to_string(),
            edible_or_resource_detail: "test".to_string(),
            body_or_growth_form: "test".to_string(),
            size_class: "test".to_string(),
            length_or_height_range: "test".to_string(),
            mass_or_biomass_range: "test".to_string(),
            habitat: "test".to_string(),
            biome: "test".to_string(),
            moisture_band: "test".to_string(),
            temperature_band: "test".to_string(),
            salinity_band: "test".to_string(),
            movement_or_growth_strategy: "test".to_string(),
            reproduction_strategy: "test".to_string(),
            reproduction_rate: "test".to_string(),
            maturation_time: "test".to_string(),
            lifespan_or_turnover: "test".to_string(),
            population_pattern: "test".to_string(),
            defense_or_resilience: "test".to_string(),
            attack_or_competition: "test".to_string(),
            sensory_or_response_mode: "test".to_string(),
            symbiosis: "test".to_string(),
            ecological_function: "test".to_string(),
            nutrient_cycle_role: "test".to_string(),
            seasonality: "test".to_string(),
            climate_sensitivity: "test".to_string(),
            disease_sensitivity: "test".to_string(),
            extinction_sensitivity: "test".to_string(),
            mk_phase_scope: "test".to_string(),
            ledger_links: "test".to_string(),
            ui_render_hint: "test".to_string(),
            implementation_tags: "test".to_string(),
            description: "test".to_string(),
        };

        assert!(entry.validate().is_err());
    }

    #[test]
    fn entry_validation_rejects_missing_no_individuals_flag() {
        let entry = BiodiversityEntry {
            id: "TEST-002".to_string(),
            major_group: "plant".to_string(),
            intelligence_index_ceiling: 0.0,
            forbidden_state_flags: "no_memory;no_goals".to_string(),
            trophic_role: "producer".to_string(),
            category: "Test".to_string(),
            common_name: "Test Species".to_string(),
            canonical_taxon: "Testus testus".to_string(),
            lineage_stem: "Testus".to_string(),
            clade: "test".to_string(),
            diet_or_resource_use: "test".to_string(),
            edible_or_resource_detail: "test".to_string(),
            body_or_growth_form: "test".to_string(),
            size_class: "test".to_string(),
            length_or_height_range: "test".to_string(),
            mass_or_biomass_range: "test".to_string(),
            habitat: "test".to_string(),
            biome: "test".to_string(),
            moisture_band: "test".to_string(),
            temperature_band: "test".to_string(),
            salinity_band: "test".to_string(),
            movement_or_growth_strategy: "test".to_string(),
            reproduction_strategy: "test".to_string(),
            reproduction_rate: "test".to_string(),
            maturation_time: "test".to_string(),
            lifespan_or_turnover: "test".to_string(),
            population_pattern: "test".to_string(),
            defense_or_resilience: "test".to_string(),
            attack_or_competition: "test".to_string(),
            sensory_or_response_mode: "test".to_string(),
            symbiosis: "test".to_string(),
            ecological_function: "test".to_string(),
            nutrient_cycle_role: "test".to_string(),
            seasonality: "test".to_string(),
            climate_sensitivity: "test".to_string(),
            disease_sensitivity: "test".to_string(),
            extinction_sensitivity: "test".to_string(),
            mk_phase_scope: "test".to_string(),
            ledger_links: "test".to_string(),
            ui_render_hint: "test".to_string(),
            implementation_tags: "test".to_string(),
            description: "test".to_string(),
        };

        assert!(entry.validate().is_err());
    }

    #[test]
    fn entry_validation_accepts_valid_entry() {
        let entry = BiodiversityEntry {
            id: "TEST-003".to_string(),
            major_group: "plant".to_string(),
            intelligence_index_ceiling: 0.0,
            forbidden_state_flags: "no_individuals;no_memory;no_goals".to_string(),
            trophic_role: "producer".to_string(),
            category: "Test".to_string(),
            common_name: "Test Species".to_string(),
            canonical_taxon: "Testus testus".to_string(),
            lineage_stem: "Testus".to_string(),
            clade: "test".to_string(),
            diet_or_resource_use: "test".to_string(),
            edible_or_resource_detail: "test".to_string(),
            body_or_growth_form: "test".to_string(),
            size_class: "test".to_string(),
            length_or_height_range: "test".to_string(),
            mass_or_biomass_range: "test".to_string(),
            habitat: "test".to_string(),
            biome: "test".to_string(),
            moisture_band: "test".to_string(),
            temperature_band: "test".to_string(),
            salinity_band: "test".to_string(),
            movement_or_growth_strategy: "test".to_string(),
            reproduction_strategy: "test".to_string(),
            reproduction_rate: "test".to_string(),
            maturation_time: "test".to_string(),
            lifespan_or_turnover: "test".to_string(),
            population_pattern: "test".to_string(),
            defense_or_resilience: "test".to_string(),
            attack_or_competition: "test".to_string(),
            sensory_or_response_mode: "test".to_string(),
            symbiosis: "test".to_string(),
            ecological_function: "test".to_string(),
            nutrient_cycle_role: "test".to_string(),
            seasonality: "test".to_string(),
            climate_sensitivity: "test".to_string(),
            disease_sensitivity: "test".to_string(),
            extinction_sensitivity: "test".to_string(),
            mk_phase_scope: "test".to_string(),
            ledger_links: "test".to_string(),
            ui_render_hint: "test".to_string(),
            implementation_tags: "test".to_string(),
            description: "test".to_string(),
        };

        assert!(entry.validate().is_ok());
    }

    #[test]
    fn catalogue_integration_with_biosphere_system() {
        use std::path::PathBuf;

        fn repo_root() -> PathBuf {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .expect("crates/")
                .parent()
                .expect("repo root")
                .to_path_buf()
        }

        let catalogue_path = repo_root().join(
            "assets/biodiversity_catalogue_10000_bundle/maerken_biodiversity_catalogue_10000.jsonl",
        );

        if !catalogue_path.exists() {
            println!(
                "Skipping test: catalogue file not found at {:?}",
                catalogue_path
            );
            return;
        }

        let catalogue = BiodiversityCatalogue::load_from_jsonl(&catalogue_path)
            .expect("Failed to load catalogue");

        // Verify all 14 categories are present
        let expected_categories = vec![
            "Insects / small arthropods",
            "Herbivores",
            "Omnivores",
            "Carnivores",
            "Birds / flying animals",
            "Marine animals",
            "Trees",
            "Grasses",
            "Flowers",
            "Fruit-bearing plants",
            "Vegetable / edible plants",
            "General plants, shrubs, mosses, ferns",
            "Fungi / decomposers",
            "Microbial / algae / lichen base layer",
        ];

        for category in &expected_categories {
            let entries = catalogue.get_by_category(category);
            assert!(
                entries.is_some(),
                "Category '{}' not found in catalogue",
                category
            );
            assert!(
                !entries.unwrap().is_empty(),
                "Category '{}' has no entries",
                category
            );
        }

        // Verify all 4 major groups are present
        let expected_major_groups = vec!["animal", "plant", "fungi", "microbial"];
        for group in &expected_major_groups {
            let entries = catalogue.get_by_major_group(group);
            assert!(
                entries.is_some(),
                "Major group '{}' not found in catalogue",
                group
            );
            assert!(
                !entries.unwrap().is_empty(),
                "Major group '{}' has no entries",
                group
            );
        }

        // Verify trophic roles are distributed correctly
        let trophic_roles: Vec<String> = catalogue
            .entries
            .iter()
            .map(|e| e.trophic_role.clone())
            .collect();

        assert!(
            trophic_roles.iter().any(|r| r.contains("producer")),
            "Catalogue missing producer trophic role"
        );
        assert!(
            trophic_roles.iter().any(|r| r.contains("consumer")),
            "Catalogue missing consumer trophic role"
        );
        assert!(
            trophic_roles.iter().any(|r| r.contains("decomposer")),
            "Catalogue missing decomposer trophic role"
        );

        // Verify intelligence ceilings are within MK-I bounds
        let max_intelligence = catalogue
            .entries
            .iter()
            .map(|e| e.intelligence_index_ceiling)
            .fold(0.0_f64, |a, b| a.max(b));

        assert!(
            max_intelligence <= 1.0,
            "Maximum intelligence ceiling {} exceeds MK-I bound of 1.0",
            max_intelligence
        );

        // Verify all entries have forbidden_state_flags containing no_individuals
        for entry in &catalogue.entries {
            assert!(
                entry.forbidden_state_flags.contains("no_individuals"),
                "Entry {} missing no_individuals flag: {}",
                entry.id,
                entry.forbidden_state_flags
            );
        }
    }
}
