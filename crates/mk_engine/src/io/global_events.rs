//! Global Event Emitter
//!
//! Provides a global singleton for event emission across the simulation.
//! This allows any subsystem to emit events without needing database references.

use super::events::EventDatabase;
use std::sync::OnceLock;

/// Global event database - initialized once at startup
static EVENT_DATABASE: OnceLock<EventDatabase> = OnceLock::new();

/// Initialize the global event database
pub fn init_event_database(db: EventDatabase) -> Result<(), String> {
    EVENT_DATABASE
        .set(db)
        .map_err(|_| "Event database already initialized".to_string())
}

/// Get a reference to the global event database
pub fn get_event_database() -> Option<&'static EventDatabase> {
    EVENT_DATABASE.get()
}

/// Check if event database is initialized
pub fn is_event_database_initialized() -> bool {
    EVENT_DATABASE.get().is_some()
}

/// Convenience function to log human birth
pub fn log_human_born(tick: u64, human_id: &str, biological_sex: &str, generation: u32) {
    if let Some(db) = get_event_database() {
        let _ = db.log_human_born(tick, human_id, biological_sex, generation);
    }
}

/// Convenience function to log human death
pub fn log_human_died(tick: u64, human_id: &str, age_years: u32, cause: Option<&str>) {
    if let Some(db) = get_event_database() {
        let _ = db.log_human_died(tick, human_id, age_years, cause);
    }
}

/// Convenience function to log human reproduction
pub fn log_human_reproduced(tick: u64, child_id: &str, parent_id_1: &str, parent_id_2: &str) {
    if let Some(db) = get_event_database() {
        let _ = db.log_human_reproduced(tick, child_id, parent_id_1, parent_id_2);
    }
}

/// Convenience function to log species creation
pub fn log_species_created(tick: u64, species_id: u64, species_name: &str) {
    if let Some(db) = get_event_database() {
        let _ = db.log_species_created(tick, species_id, species_name);
    }
}

/// Convenience function to log species extinction
pub fn log_species_extinct(tick: u64, species_id: u64, species_name: &str, population: u64) {
    if let Some(db) = get_event_database() {
        let _ = db.log_species_extinct(tick, species_id, species_name, population);
    }
}

/// Convenience function to log species mutation
pub fn log_species_mutated(tick: u64, species_id: u64, mutation_type: &str) {
    if let Some(db) = get_event_database() {
        let _ = db.log_species_mutated(tick, species_id, mutation_type);
    }
}
