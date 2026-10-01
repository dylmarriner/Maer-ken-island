//! Event Database Module
//!
//! Implements persistent event logging for the Maer'Ken world.
//! Mirrors Earth's fossil record - every birth, death, evolution is recorded.
//!
//! Uses SQLite for 24/7 append-only logging with crash safety.

use rusqlite::{params, Connection, Result as SqliteResult};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Mutex;
use tracing::{info, warn};

/// Event types that can occur in the universe
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EventType {
    // Human events
    HumanBorn,
    HumanDied,
    HumanReproduced,

    // Species/Biosphere events
    SpeciesCreated,
    SpeciesExtinct,
    SpeciesMutated,

    // Animal events
    AnimalBorn,
    AnimalDied,
    AnimalReproduced,

    // World events
    WorldBoot,
    WorldShutdown,
}

/// Entity types that can generate events
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EntityType {
    Human,
    Animal,
    Species,
    World,
}

/// A single event in the universe's history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UniverseEvent {
    pub id: Option<i64>,
    pub tick: u64,
    pub timestamp: String,
    pub event_type: EventType,
    pub entity_type: EntityType,
    pub entity_id: String,
    pub parent_id_1: Option<String>,
    pub parent_id_2: Option<String>,
    pub species_id: Option<u64>,
    pub details: Option<String>,
}

/// Returned by [`EventDatabase::verify_chain`] when the hash chain breaks:
/// the id of the first event whose stored hash doesn't match recomputation
/// from its predecessor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChainBreak {
    pub event_id: i64,
}

impl std::fmt::Display for ChainBreak {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "event hash chain broken at event id {}", self.event_id)
    }
}

impl std::error::Error for ChainBreak {}

/// Event database for persistent storage
pub struct EventDatabase {
    conn: Mutex<Connection>,
}

impl EventDatabase {
    /// Create or open an event database at the given path
    pub fn new<P: AsRef<Path>>(path: P) -> SqliteResult<Self> {
        let conn = Connection::open(path)?;

        // Enable WAL mode for better concurrent access and crash safety
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")?;

        let db = Self {
            conn: Mutex::new(conn),
        };

        db.initialize_schema()?;
        Ok(db)
    }

    /// Initialize the database schema
    fn initialize_schema(&self) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap_or_else(|p| p.into_inner());

        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS events (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                tick INTEGER NOT NULL,
                timestamp TEXT NOT NULL,
                event_type TEXT NOT NULL,
                entity_type TEXT NOT NULL,
                entity_id TEXT NOT NULL,
                parent_id_1 TEXT,
                parent_id_2 TEXT,
                species_id INTEGER,
                details TEXT,
                created_at TEXT DEFAULT CURRENT_TIMESTAMP,
                prev_hash TEXT NOT NULL DEFAULT '',
                hash TEXT NOT NULL DEFAULT ''
            );

            CREATE INDEX IF NOT EXISTS idx_events_tick ON events(tick);
            CREATE INDEX IF NOT EXISTS idx_events_event_type ON events(event_type);
            CREATE INDEX IF NOT EXISTS idx_events_entity_id ON events(entity_id);
            CREATE INDEX IF NOT EXISTS idx_events_entity_type ON events(entity_type);

            -- Metadata table for universe info
            CREATE TABLE IF NOT EXISTS universe_metadata (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            "#,
        )?;

        // Migration for databases created before the hash-chain columns
        // existed: `CREATE TABLE IF NOT EXISTS` above is a no-op against an
        // existing table, so add the columns explicitly, ignoring the
        // "duplicate column" error on a database that already has them.
        for stmt in [
            "ALTER TABLE events ADD COLUMN prev_hash TEXT NOT NULL DEFAULT ''",
            "ALTER TABLE events ADD COLUMN hash TEXT NOT NULL DEFAULT ''",
        ] {
            if let Err(e) = conn.execute(stmt, []) {
                if !e.to_string().contains("duplicate column name") {
                    return Err(e);
                }
            }
        }

        Ok(())
    }

    /// Genesis hash for an empty event log (BLAKE3 hex has no meaningful
    /// "zero" value, so an explicit sentinel is used instead).
    const GENESIS_HASH: &'static str = "genesis";

    /// The hash of the most recently inserted event, or [`Self::GENESIS_HASH`]
    /// if the log is empty. Must be called with `conn` already locked by the
    /// caller (this does not take `&self.conn` itself, to avoid re-locking).
    fn last_hash(conn: &Connection) -> SqliteResult<String> {
        let result = conn.query_row(
            "SELECT hash FROM events ORDER BY id DESC LIMIT 1",
            [],
            |row| row.get::<_, String>(0),
        );
        match result {
            Ok(hash) if !hash.is_empty() => Ok(hash),
            Ok(_) | Err(rusqlite::Error::QueryReturnedNoRows) => Ok(Self::GENESIS_HASH.to_string()),
            Err(e) => Err(e),
        }
    }

    /// Deterministic BLAKE3 hex digest chaining `prev_hash` with this
    /// event's content, so any change to a stored row (or a row deleted
    /// from the middle of the log) breaks the chain at that point — see
    /// [`Self::verify_chain`].
    fn compute_hash(
        prev_hash: &str,
        event: &UniverseEvent,
        event_type_str: &str,
        entity_type_str: &str,
    ) -> String {
        let mut hasher = blake3::Hasher::new();
        hasher.update(prev_hash.as_bytes());
        hasher.update(&event.tick.to_le_bytes());
        hasher.update(event.timestamp.as_bytes());
        hasher.update(event_type_str.as_bytes());
        hasher.update(entity_type_str.as_bytes());
        hasher.update(event.entity_id.as_bytes());
        hasher.update(event.parent_id_1.as_deref().unwrap_or("").as_bytes());
        hasher.update(event.parent_id_2.as_deref().unwrap_or("").as_bytes());
        hasher.update(&event.species_id.unwrap_or(0).to_le_bytes());
        hasher.update(event.details.as_deref().unwrap_or("").as_bytes());
        hasher.finalize().to_hex().to_string()
    }

    /// Log a single event to the database, chaining it onto the previous
    /// event's hash (blake3, per [`Self::compute_hash`]) for tamper
    /// evidence — see [`Self::verify_chain`].
    pub fn log_event(&self, event: &UniverseEvent) -> SqliteResult<i64> {
        let conn = self.conn.lock().unwrap_or_else(|p| p.into_inner());

        let event_type_str = serde_json::to_string(&event.event_type).unwrap_or_default();
        let entity_type_str = serde_json::to_string(&event.entity_type).unwrap_or_default();
        let prev_hash = Self::last_hash(&conn)?;
        let hash = Self::compute_hash(&prev_hash, event, &event_type_str, &entity_type_str);

        conn.execute(
            r#"
            INSERT INTO events (
                tick, timestamp, event_type, entity_type, entity_id,
                parent_id_1, parent_id_2, species_id, details, prev_hash, hash
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
            "#,
            params![
                event.tick as i64,
                event.timestamp,
                event_type_str,
                entity_type_str,
                event.entity_id,
                event.parent_id_1,
                event.parent_id_2,
                event.species_id.map(|id| id as i64),
                event.details,
                prev_hash,
                hash,
            ],
        )?;

        Ok(conn.last_insert_rowid())
    }

    /// Walk every event in insertion order and confirm each row's stored
    /// `hash` matches recomputing [`Self::compute_hash`] from the previous
    /// row's stored hash and that row's own content. Returns the id of the
    /// first row whose hash doesn't match (tampering, a hand-edited row, or
    /// a row deleted from the middle of the log — which shifts every
    /// subsequent link), if any.
    ///
    /// Ported from markenz's `storage.rs::verify_chain()` — Maer-Ken's own
    /// event log previously had no hash/chain/verification equivalent at
    /// all (a plain append-only SQLite log). See
    /// `audit-results/maerken-vs-gemini-markenz-gap-audit.md` finding 7.
    pub fn verify_chain(&self) -> SqliteResult<Result<(), ChainBreak>> {
        let conn = self.conn.lock().unwrap_or_else(|p| p.into_inner());

        let mut stmt = conn.prepare(
            r#"
            SELECT id, tick, timestamp, event_type, entity_type, entity_id,
                   parent_id_1, parent_id_2, species_id, details, prev_hash, hash
            FROM events
            ORDER BY id ASC
            "#,
        )?;

        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                UniverseEvent {
                    id: Some(row.get(0)?),
                    tick: row.get::<_, i64>(1)? as u64,
                    timestamp: row.get(2)?,
                    event_type: serde_json::from_str(&row.get::<_, String>(3)?)
                        .unwrap_or(EventType::WorldBoot),
                    entity_type: serde_json::from_str(&row.get::<_, String>(4)?)
                        .unwrap_or(EntityType::World),
                    entity_id: row.get(5)?,
                    parent_id_1: row.get(6)?,
                    parent_id_2: row.get(7)?,
                    species_id: row.get::<_, Option<i64>>(8)?.map(|id| id as u64),
                    details: row.get(9)?,
                },
                row.get::<_, String>(10)?, // stored prev_hash
                row.get::<_, String>(11)?, // stored hash
            ))
        })?;

        let mut expected_prev_hash = Self::GENESIS_HASH.to_string();
        for row in rows {
            let (id, event, stored_prev_hash, stored_hash) = row?;
            let event_type_str = serde_json::to_string(&event.event_type).unwrap_or_default();
            let entity_type_str = serde_json::to_string(&event.entity_type).unwrap_or_default();

            if stored_prev_hash != expected_prev_hash {
                return Ok(Err(ChainBreak { event_id: id }));
            }
            let recomputed =
                Self::compute_hash(&stored_prev_hash, &event, &event_type_str, &entity_type_str);
            if recomputed != stored_hash {
                return Ok(Err(ChainBreak { event_id: id }));
            }
            expected_prev_hash = stored_hash;
        }

        Ok(Ok(()))
    }

    /// Log a human birth event
    pub fn log_human_born(
        &self,
        tick: u64,
        human_id: &str,
        biological_sex: &str,
        generation: u32,
    ) -> SqliteResult<i64> {
        let timestamp = chrono::Utc::now().to_rfc3339();
        let details = serde_json::json!({
            "biological_sex": biological_sex,
            "generation": generation
        });

        let event = UniverseEvent {
            id: None,
            tick,
            timestamp,
            event_type: EventType::HumanBorn,
            entity_type: EntityType::Human,
            entity_id: human_id.to_string(),
            parent_id_1: None,
            parent_id_2: None,
            species_id: None,
            details: Some(details.to_string()),
        };

        self.log_event(&event)
    }

    /// Log a human death event
    pub fn log_human_died(
        &self,
        tick: u64,
        human_id: &str,
        age_years: u32,
        cause: Option<&str>,
    ) -> SqliteResult<i64> {
        let timestamp = chrono::Utc::now().to_rfc3339();
        let details = serde_json::json!({
            "age_years": age_years,
            "cause": cause.unwrap_or("natural"),
        });

        let event = UniverseEvent {
            id: None,
            tick,
            timestamp,
            event_type: EventType::HumanDied,
            entity_type: EntityType::Human,
            entity_id: human_id.to_string(),
            parent_id_1: None,
            parent_id_2: None,
            species_id: None,
            details: Some(details.to_string()),
        };

        self.log_event(&event)
    }

    /// Log a human reproduction event (child born)
    pub fn log_human_reproduced(
        &self,
        tick: u64,
        child_id: &str,
        parent_id_1: &str,
        parent_id_2: &str,
    ) -> SqliteResult<i64> {
        let timestamp = chrono::Utc::now().to_rfc3339();

        let event = UniverseEvent {
            id: None,
            tick,
            timestamp,
            event_type: EventType::HumanReproduced,
            entity_type: EntityType::Human,
            entity_id: child_id.to_string(),
            parent_id_1: Some(parent_id_1.to_string()),
            parent_id_2: Some(parent_id_2.to_string()),
            species_id: None,
            details: None,
        };

        self.log_event(&event)
    }

    /// Log a species creation event
    pub fn log_species_created(
        &self,
        tick: u64,
        species_id: u64,
        species_name: &str,
    ) -> SqliteResult<i64> {
        let timestamp = chrono::Utc::now().to_rfc3339();
        let details = serde_json::json!({
            "species_name": species_name,
        });

        let event = UniverseEvent {
            id: None,
            tick,
            timestamp,
            event_type: EventType::SpeciesCreated,
            entity_type: EntityType::Species,
            entity_id: format!("species_{}", species_id),
            parent_id_1: None,
            parent_id_2: None,
            species_id: Some(species_id),
            details: Some(details.to_string()),
        };

        self.log_event(&event)
    }

    /// Log a species extinction event
    pub fn log_species_extinct(
        &self,
        tick: u64,
        species_id: u64,
        species_name: &str,
        population_at_death: u64,
    ) -> SqliteResult<i64> {
        let timestamp = chrono::Utc::now().to_rfc3339();
        let details = serde_json::json!({
            "species_name": species_name,
            "population_at_death": population_at_death,
        });

        let event = UniverseEvent {
            id: None,
            tick,
            timestamp,
            event_type: EventType::SpeciesExtinct,
            entity_type: EntityType::Species,
            entity_id: format!("species_{}", species_id),
            parent_id_1: None,
            parent_id_2: None,
            species_id: Some(species_id),
            details: Some(details.to_string()),
        };

        self.log_event(&event)
    }

    /// Log a species mutation event
    pub fn log_species_mutated(
        &self,
        tick: u64,
        species_id: u64,
        mutation_type: &str,
    ) -> SqliteResult<i64> {
        let timestamp = chrono::Utc::now().to_rfc3339();
        let details = serde_json::json!({
            "mutation_type": mutation_type,
        });

        let event = UniverseEvent {
            id: None,
            tick,
            timestamp,
            event_type: EventType::SpeciesMutated,
            entity_type: EntityType::Species,
            entity_id: format!("species_{}", species_id),
            parent_id_1: None,
            parent_id_2: None,
            species_id: Some(species_id),
            details: Some(details.to_string()),
        };

        self.log_event(&event)
    }

    /// Log a world boot event
    pub fn log_world_boot(&self, tick: u64, universe_config: &str) -> SqliteResult<i64> {
        let timestamp = chrono::Utc::now().to_rfc3339();

        let event = UniverseEvent {
            id: None,
            tick,
            timestamp,
            event_type: EventType::WorldBoot,
            entity_type: EntityType::World,
            entity_id: "universe".to_string(),
            parent_id_1: None,
            parent_id_2: None,
            species_id: None,
            details: Some(universe_config.to_string()),
        };

        self.log_event(&event)
    }

    /// Query events by type
    pub fn query_by_type(
        &self,
        event_type: &EventType,
        limit: usize,
    ) -> SqliteResult<Vec<UniverseEvent>> {
        let conn = self.conn.lock().unwrap_or_else(|p| p.into_inner());
        let type_str = serde_json::to_string(event_type).unwrap_or_default();

        let mut stmt = conn.prepare(
            r#"
            SELECT id, tick, timestamp, event_type, entity_type, entity_id,
                   parent_id_1, parent_id_2, species_id, details
            FROM events
            WHERE event_type = ?1
            ORDER BY tick DESC, id DESC
            LIMIT ?2
            "#,
        )?;

        let events = stmt
            .query_map(params![type_str, limit as i64], |row| {
                Ok(UniverseEvent {
                    id: Some(row.get(0)?),
                    tick: row.get::<_, i64>(1)? as u64,
                    timestamp: row.get(2)?,
                    event_type: serde_json::from_str(&row.get::<_, String>(3)?)
                        .unwrap_or(EventType::WorldBoot),
                    entity_type: serde_json::from_str(&row.get::<_, String>(4)?)
                        .unwrap_or(EntityType::World),
                    entity_id: row.get(5)?,
                    parent_id_1: row.get(6)?,
                    parent_id_2: row.get(7)?,
                    species_id: row.get::<_, Option<i64>>(8)?.map(|id| id as u64),
                    details: row.get(9)?,
                })
            })?
            .collect::<SqliteResult<Vec<_>>>()?;

        Ok(events)
    }

    /// Query events by entity ID
    pub fn query_by_entity(
        &self,
        entity_id: &str,
        limit: usize,
    ) -> SqliteResult<Vec<UniverseEvent>> {
        let conn = self.conn.lock().unwrap_or_else(|p| p.into_inner());

        let mut stmt = conn.prepare(
            r#"
            SELECT id, tick, timestamp, event_type, entity_type, entity_id,
                   parent_id_1, parent_id_2, species_id, details
            FROM events
            WHERE entity_id = ?1
            ORDER BY tick DESC
            LIMIT ?2
            "#,
        )?;

        let events = stmt
            .query_map(params![entity_id, limit as i64], |row| {
                Ok(UniverseEvent {
                    id: Some(row.get(0)?),
                    tick: row.get::<_, i64>(1)? as u64,
                    timestamp: row.get(2)?,
                    event_type: serde_json::from_str(&row.get::<_, String>(3)?)
                        .unwrap_or(EventType::WorldBoot),
                    entity_type: serde_json::from_str(&row.get::<_, String>(4)?)
                        .unwrap_or(EntityType::World),
                    entity_id: row.get(5)?,
                    parent_id_1: row.get(6)?,
                    parent_id_2: row.get(7)?,
                    species_id: row.get::<_, Option<i64>>(8)?.map(|id| id as u64),
                    details: row.get(9)?,
                })
            })?
            .collect::<SqliteResult<Vec<_>>>()?;

        Ok(events)
    }

    /// Get event count by type
    pub fn count_by_type(&self, event_type: &EventType) -> SqliteResult<i64> {
        let conn = self.conn.lock().unwrap_or_else(|p| p.into_inner());
        let type_str = serde_json::to_string(event_type).unwrap_or_default();

        conn.query_row(
            "SELECT COUNT(*) FROM events WHERE event_type = ?1",
            params![type_str],
            |row| row.get(0),
        )
    }

    /// Get total event count
    pub fn total_events(&self) -> SqliteResult<i64> {
        let conn = self.conn.lock().unwrap_or_else(|p| p.into_inner());
        conn.query_row("SELECT COUNT(*) FROM events", [], |row| row.get(0))
    }

    /// Get recent events
    pub fn recent_events(&self, limit: usize) -> SqliteResult<Vec<UniverseEvent>> {
        let conn = self.conn.lock().unwrap_or_else(|p| p.into_inner());

        let mut stmt = conn.prepare(
            r#"
            SELECT id, tick, timestamp, event_type, entity_type, entity_id,
                   parent_id_1, parent_id_2, species_id, details
            FROM events
            ORDER BY tick DESC, id DESC
            LIMIT ?1
            "#,
        )?;

        let events = stmt
            .query_map(params![limit as i64], |row| {
                Ok(UniverseEvent {
                    id: Some(row.get(0)?),
                    tick: row.get::<_, i64>(1)? as u64,
                    timestamp: row.get(2)?,
                    event_type: serde_json::from_str(&row.get::<_, String>(3)?)
                        .unwrap_or(EventType::WorldBoot),
                    entity_type: serde_json::from_str(&row.get::<_, String>(4)?)
                        .unwrap_or(EntityType::World),
                    entity_id: row.get(5)?,
                    parent_id_1: row.get(6)?,
                    parent_id_2: row.get(7)?,
                    species_id: row.get::<_, Option<i64>>(8)?.map(|id| id as u64),
                    details: row.get(9)?,
                })
            })?
            .collect::<SqliteResult<Vec<_>>>()?;

        Ok(events)
    }

    /// Store universe metadata
    pub fn set_metadata(&self, key: &str, value: &str) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap_or_else(|p| p.into_inner());
        conn.execute(
            "INSERT OR REPLACE INTO universe_metadata (key, value) VALUES (?1, ?2)",
            params![key, value],
        )?;
        Ok(())
    }

    /// Get universe metadata
    pub fn get_metadata(&self, key: &str) -> SqliteResult<Option<String>> {
        let conn = self.conn.lock().unwrap_or_else(|p| p.into_inner());
        let result = conn.query_row(
            "SELECT value FROM universe_metadata WHERE key = ?1",
            params![key],
            |row| row.get(0),
        );

        match result {
            Ok(value) => Ok(Some(value)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }
}

/// Log a summary of universe history to the tracing output.
pub fn print_universe_summary(db: &EventDatabase) {
    let total = match db.total_events() {
        Ok(t) => t,
        Err(e) => {
            warn!("Could not read event count: {}", e);
            return;
        }
    };
    info!(total_events = total, "Universe history report");

    let event_types = [
        (EventType::HumanBorn, "human_births"),
        (EventType::HumanDied, "human_deaths"),
        (EventType::HumanReproduced, "human_reproductions"),
        (EventType::SpeciesCreated, "species_created"),
        (EventType::SpeciesExtinct, "species_extinct"),
        (EventType::SpeciesMutated, "species_mutations"),
        (EventType::AnimalBorn, "animal_births"),
        (EventType::AnimalDied, "animal_deaths"),
        (EventType::AnimalReproduced, "animal_reproductions"),
    ];

    for (event_type, label) in &event_types {
        if let Ok(count) = db.count_by_type(event_type) {
            if count > 0 {
                info!(count = count, category = label, "Event breakdown");
            }
        }
    }

    match db.recent_events(10) {
        Ok(events) => {
            for event in &events {
                info!(
                    tick = event.tick,
                    event_type = ?event.event_type,
                    entity_id = %event.entity_id,
                    "Recent event"
                );
            }
        }
        Err(e) => warn!("Could not read recent events: {}", e),
    }
}

#[cfg(test)]
mod hash_chain_tests {
    use super::*;

    fn birth_event(tick: u64, entity_id: &str) -> UniverseEvent {
        UniverseEvent {
            id: None,
            tick,
            timestamp: "2026-01-01T00:00:00Z".to_string(),
            event_type: EventType::HumanBorn,
            entity_type: EntityType::Human,
            entity_id: entity_id.to_string(),
            parent_id_1: None,
            parent_id_2: None,
            species_id: None,
            details: None,
        }
    }

    #[test]
    fn empty_log_verifies_clean() {
        let db = EventDatabase::new(":memory:").unwrap();
        assert!(db.verify_chain().unwrap().is_ok());
    }

    #[test]
    fn chained_log_verifies_clean() {
        let db = EventDatabase::new(":memory:").unwrap();
        for i in 0..5 {
            db.log_event(&birth_event(i, &format!("human_{i}")))
                .unwrap();
        }
        assert!(db.verify_chain().unwrap().is_ok());
    }

    #[test]
    fn tampered_row_breaks_the_chain() {
        let db = EventDatabase::new(":memory:").unwrap();
        for i in 0..5 {
            db.log_event(&birth_event(i, &format!("human_{i}")))
                .unwrap();
        }
        assert!(db.verify_chain().unwrap().is_ok());

        // Directly tamper with row 3's content without updating its hash —
        // the exact scenario this chain exists to detect.
        {
            let conn = db.conn.lock().unwrap();
            conn.execute("UPDATE events SET entity_id = 'tampered' WHERE id = 3", [])
                .unwrap();
        }

        let result = db.verify_chain().unwrap();
        assert_eq!(result, Err(ChainBreak { event_id: 3 }));
    }

    #[test]
    fn deleted_row_breaks_the_chain() {
        let db = EventDatabase::new(":memory:").unwrap();
        for i in 0..5 {
            db.log_event(&birth_event(i, &format!("human_{i}")))
                .unwrap();
        }

        {
            let conn = db.conn.lock().unwrap();
            conn.execute("DELETE FROM events WHERE id = 3", []).unwrap();
        }

        let result = db.verify_chain().unwrap();
        // Row 4's stored prev_hash no longer matches row 2's hash, since
        // row 3 (which it was chained onto) is gone.
        assert_eq!(result, Err(ChainBreak { event_id: 4 }));
    }
}
