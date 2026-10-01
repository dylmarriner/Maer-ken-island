//! Bounded, individually-tracked male/female pairs for the animal-major-group
//! entries in the 10,000-entry biodiversity catalogue.
//!
//! The catalogue's original governance (`biodiversity_catalogue::tests::
//! entry_validation_rejects_missing_no_individuals_flag`) required every
//! entry to stay population-field-only with no individual tracking. Per
//! explicit user direction this is superseded for the catalogue's fauna
//! entries specifically: "everything should have real female and male in
//! the engine for reproduction," clarified to mean the ~3,500
//! `major_group == "animal"` entries (`Insects / small arthropods`,
//! `Birds / flying animals`, `Herbivores`, `Carnivores`, `Omnivores`,
//! `Marine animals`) — explicitly *not* the ~6,500 plant/fungal/microbial
//! entries (trees, vegetables, grasses, flowers, herbs/fungi/microbial base
//! layer), which keep their original population-field-only governance and
//! reproduce via their real documented `reproduction_strategy` (spores,
//! seeds, division, etc.), not sexual pairing.
//!
//! Kept hard-bounded per species (`MAX_TRACKED_PER_SPECIES`) so this can't
//! repeat the earlier, similar animal/plant individual-agent feature
//! (`docs/design-plans/specs/2026-07-04-3d-universe-viewer-design.md`,
//! "Revision: animal/plant individual agents removed") that was reverted
//! after an unbounded per-species cap combined with the founding-species
//! count produced a real FPS/determinism risk (~32,000 simulated
//! organisms). At `MAX_TRACKED_PER_SPECIES = 6` across 3,500 animal
//! species, this caps out at 21,000 individuals — bounded, and far below
//! that prior incident's uncapped growth curve since each species also
//! starts at exactly one pair (2) rather than the full cap.

use crate::agents::GridPosition;
use crate::biosphere::biodiversity_catalogue::BiodiversityCatalogue;
use crate::biosphere::evolution::Sex;
use mk_core::rng::{RngKey, RngRegistry, SubsystemId};
use mk_core::time::Tick;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Hard per-species cap on simultaneously tracked individuals. Species start
/// at exactly one male + one female founding pair and grow toward this cap
/// only via real reproduction (mature opposite-sex pair present).
pub const MAX_TRACKED_PER_SPECIES: usize = 6;

const FAUNA_MAJOR_GROUP: &str = "animal";
/// Individuals reach reproductive maturity after this many simulated years.
const MATURITY_YEARS: f32 = 2.0;
/// Per-step probability a mature pair successfully reproduces, applied once
/// per eligible species per `step()` call.
const REPRODUCTION_PROBABILITY: f64 = 0.05;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogueIndividual {
    pub id: u64,
    pub species_entry_id: String,
    /// Real `BiodiversityEntry.category` this individual belongs to,
    /// carried alongside `species_entry_id` (rather than requiring
    /// downstream consumers to keep the whole catalogue around) so
    /// rendering/inspection can pick a category-appropriate appearance.
    pub species_category: String,
    pub sex: Sex,
    pub alive: bool,
    pub age_years: f32,
    /// Grid cell the individual lives in: its species' habitat cell
    /// ([`crate::biosphere::habitat::habitat_cell`]) for founders, the
    /// mother's cell for offspring. `None` only for snapshots written before
    /// positions existed, until [`CatalogueIndividualRegistry::place_unplaced`]
    /// backfills them.
    #[serde(default)]
    pub position: Option<GridPosition>,
}

impl CatalogueIndividual {
    pub fn is_mature(&self) -> bool {
        self.age_years >= MATURITY_YEARS
    }
}

/// Deterministic FNV-1a hash truncated to `u32`, used only to derive a
/// stable per-species RNG epoch from a catalogue entry's real `id` string —
/// not a source of randomness itself.
fn species_epoch(species_entry_id: &str) -> u32 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for b in species_entry_id.as_bytes() {
        hash ^= *b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    (hash ^ (hash >> 32)) as u32
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CatalogueIndividualRegistry {
    pub individuals: Vec<CatalogueIndividual>,
    pub next_id: u64,
}

impl CatalogueIndividualRegistry {
    pub fn new() -> Self {
        Self {
            individuals: Vec::new(),
            next_id: 1,
        }
    }

    /// Seeds exactly one male + one female founding pair for every
    /// `major_group == "animal"` catalogue entry. Idempotent: no-ops once
    /// already seeded.
    pub fn seed_from_catalogue(&mut self, catalogue: &BiodiversityCatalogue) {
        if !self.individuals.is_empty() {
            return;
        }
        for entry in &catalogue.entries {
            if entry.major_group != FAUNA_MAJOR_GROUP {
                continue;
            }
            for sex in [Sex::Male, Sex::Female] {
                self.individuals.push(CatalogueIndividual {
                    id: self.next_id,
                    species_entry_id: entry.id.clone(),
                    species_category: entry.category.clone(),
                    sex,
                    alive: true,
                    // Founders start already mature so reproduction can
                    // begin immediately rather than waiting one full
                    // maturity period after world start.
                    age_years: MATURITY_YEARS,
                    position: None,
                });
                self.next_id += 1;
            }
        }
    }

    /// Give every individual without a position its species' habitat cell
    /// in `world`. Founders are placed this way; it also backfills
    /// snapshots written before individuals carried positions.
    pub fn place_unplaced(
        &mut self,
        catalogue: &BiodiversityCatalogue,
        world: &crate::world_integration::WorldState,
    ) {
        let mut cells: HashMap<String, GridPosition> = HashMap::new();
        for individual in self.individuals.iter_mut().filter(|i| i.position.is_none()) {
            if let Some(cell) = cells.get(&individual.species_entry_id) {
                individual.position = Some(*cell);
                continue;
            }
            let Some(entry) = catalogue
                .entries
                .iter()
                .find(|entry| entry.id == individual.species_entry_id)
            else {
                continue;
            };
            let (row, col) = crate::biosphere::habitat::habitat_cell(world, entry);
            let cell = GridPosition::new(row as i32, col as i32);
            cells.insert(individual.species_entry_id.clone(), cell);
            individual.position = Some(cell);
        }
    }

    /// Ages every alive individual by `dt_years`, then rolls deterministic,
    /// per-species-independent reproduction for species under the cap with
    /// at least one mature male and one mature female alive.
    pub fn step(&mut self, dt_years: f32, rng: &RngRegistry, tick: Tick) {
        for individual in &mut self.individuals {
            if individual.alive {
                individual.age_years += dt_years;
            }
        }

        // (count, has mature male, has mature female, category, a mature
        // mother's position — where offspring are born).
        type SpeciesTally<'a> = (usize, bool, bool, &'a str, Option<GridPosition>);
        let mut per_species: HashMap<&str, SpeciesTally<'_>> = HashMap::new();
        for individual in &self.individuals {
            if !individual.alive {
                continue;
            }
            let entry = per_species.entry(&individual.species_entry_id).or_insert((
                0,
                false,
                false,
                individual.species_category.as_str(),
                None,
            ));
            entry.0 += 1;
            let mature = individual.is_mature();
            match individual.sex {
                Sex::Male => entry.1 = entry.1 || mature,
                Sex::Female => {
                    entry.2 = entry.2 || mature;
                    if mature && entry.4.is_none() {
                        entry.4 = individual.position;
                    }
                }
            }
        }

        // Sort by species_id (not HashMap iteration order, which is
        // randomized per-process) since iteration order here drives birth
        // push order, which drives `next_id` assignment below — two runs
        // with identical seed/state could otherwise assign different IDs to
        // different individuals.
        let mut eligible_species: Vec<(String, String, Option<GridPosition>)> = per_species
            .iter()
            .filter(|(_, (count, has_male, has_female, _, _))| {
                *count < MAX_TRACKED_PER_SPECIES && *has_male && *has_female
            })
            .map(|(species_id, (_, _, _, category, birthplace))| {
                (species_id.to_string(), category.to_string(), *birthplace)
            })
            .collect();
        eligible_species.sort_by(|(a, _, _), (b, _, _)| a.cmp(b));

        let mut births = Vec::new();
        for (species_id, category, birthplace) in eligible_species {
            let epoch = species_epoch(&species_id);
            let roll_key = RngKey::new(SubsystemId::Biosphere, 44_001, epoch, tick);
            if rng.gen_f64_01(roll_key) >= REPRODUCTION_PROBABILITY {
                continue;
            }
            let sex_key = RngKey::new(SubsystemId::Biosphere, 44_002, epoch, tick);
            let sex = if rng.gen_f64_01(sex_key) < 0.5 {
                Sex::Male
            } else {
                Sex::Female
            };
            births.push(CatalogueIndividual {
                id: 0, // assigned below
                species_entry_id: species_id,
                species_category: category,
                sex,
                alive: true,
                age_years: 0.0,
                position: birthplace,
            });
        }

        for mut birth in births {
            birth.id = self.next_id;
            self.next_id += 1;
            self.individuals.push(birth);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::biosphere::biodiversity_catalogue::BiodiversityEntry;

    fn animal_entry(id: &str) -> BiodiversityEntry {
        BiodiversityEntry {
            id: id.to_string(),
            major_group: "animal".to_string(),
            intelligence_index_ceiling: 0.0,
            forbidden_state_flags: "no_memory".to_string(),
            trophic_role: "primary_consumer".to_string(),
            category: "Herbivores".to_string(),
            common_name: "Test Fauna".to_string(),
            canonical_taxon: "Testus faunus".to_string(),
            lineage_stem: "Testus".to_string(),
            clade: "test-clade".to_string(),
            diet_or_resource_use: "test".to_string(),
            edible_or_resource_detail: "test".to_string(),
            body_or_growth_form: "test".to_string(),
            size_class: "small".to_string(),
            length_or_height_range: "1-2 cm".to_string(),
            mass_or_biomass_range: "1-2 kg".to_string(),
            habitat: "test".to_string(),
            biome: "test".to_string(),
            moisture_band: "test".to_string(),
            temperature_band: "test".to_string(),
            salinity_band: "test".to_string(),
            movement_or_growth_strategy: "test".to_string(),
            reproduction_strategy: "sexual".to_string(),
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
            implementation_tags: "individuals_allowed".to_string(),
            description: "test".to_string(),
        }
    }

    fn plant_entry(id: &str) -> BiodiversityEntry {
        let mut e = animal_entry(id);
        e.major_group = "plant".to_string();
        e.category = "Trees".to_string();
        e
    }

    #[test]
    fn seed_creates_one_pair_per_animal_entry_only() {
        let catalogue = BiodiversityCatalogue {
            entries: vec![animal_entry("A-1"), plant_entry("P-1")],
            ..Default::default()
        };
        let mut registry = CatalogueIndividualRegistry::new();
        registry.seed_from_catalogue(&catalogue);

        assert_eq!(
            registry.individuals.len(),
            2,
            "only the animal entry gets a pair"
        );
        assert!(registry
            .individuals
            .iter()
            .all(|i| i.species_entry_id == "A-1"));
        assert_eq!(
            registry
                .individuals
                .iter()
                .filter(|i| i.sex == Sex::Male)
                .count(),
            1
        );
        assert_eq!(
            registry
                .individuals
                .iter()
                .filter(|i| i.sex == Sex::Female)
                .count(),
            1
        );
    }

    #[test]
    fn step_is_deterministic_across_identical_runs() {
        let catalogue = BiodiversityCatalogue {
            entries: (0..50).map(|i| animal_entry(&format!("A-{i}"))).collect(),
            ..Default::default()
        };
        let rng = RngRegistry::new([9u8; 32]);

        let run = || {
            let mut registry = CatalogueIndividualRegistry::new();
            registry.seed_from_catalogue(&catalogue);
            for tick in 0..20u64 {
                registry.step(0.5, &rng, tick);
            }
            registry.individuals.len()
        };

        assert_eq!(run(), run());
    }

    #[test]
    fn population_never_exceeds_cap_per_species() {
        let catalogue = BiodiversityCatalogue {
            entries: vec![animal_entry("A-1")],
            ..Default::default()
        };
        let rng = RngRegistry::new([3u8; 32]);
        let mut registry = CatalogueIndividualRegistry::new();
        registry.seed_from_catalogue(&catalogue);

        for tick in 0..500u64 {
            registry.step(1.0, &rng, tick);
        }

        let alive = registry
            .individuals
            .iter()
            .filter(|i| i.alive && i.species_entry_id == "A-1")
            .count();
        assert!(alive <= MAX_TRACKED_PER_SPECIES);
    }

    #[test]
    fn seeded_individuals_live_in_their_habitat_and_offspring_are_born_there() {
        let mut marine = animal_entry("A-MARINE");
        marine.moisture_band = "submerged".to_string();
        let catalogue = BiodiversityCatalogue {
            entries: vec![marine],
            ..Default::default()
        };
        let mut world = crate::world_integration::WorldState::new(
            std::sync::Arc::new(mk_core::canon::CanonLocked::default()),
            [6u8; 32],
        );
        world.seed_catalogue_individuals(&catalogue);

        let founders = &world.catalogue_individuals.individuals;
        assert_eq!(founders.len(), 2);
        let home = founders[0].position.expect("founders are placed");
        assert_eq!(
            founders[1].position,
            Some(home),
            "a founding pair shares a cell"
        );
        assert!(
            *world
                .elevation_grid
                .get(home.row as usize, home.col as usize)
                < 0.0,
            "a submerged species lives in the ocean"
        );

        let rng = RngRegistry::new([3u8; 32]);
        for tick in 0..500u64 {
            world.catalogue_individuals.step(1.0, &rng, tick);
        }
        let individuals = &world.catalogue_individuals.individuals;
        assert!(individuals.len() > 2, "the pair reproduced");
        assert!(individuals.iter().all(|i| i.position == Some(home)));
    }
}
