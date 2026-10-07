//! Per-human folders for an island run (Phase 4 Task 3b).
//!
//! A run owns `<save_root>/<run_id>/humans/`, written through upstream
//! `HumanStorage` (no second folder format). Disk success or failure never
//! changes simulation state: the status here is runtime-only and is not in
//! the state digest or the snapshot. Failures are counted and kept, never
//! discarded.

use std::path::{Path, PathBuf};

use crate::io::human_storage::{HumanStorage, HumanStorageError};

use super::replay::scenario_digest;
use super::world::IslandWorldState;

#[derive(Debug)]
pub enum HumanStoreError {
    Io(PathBuf, std::io::Error),
    /// `runs.json` exists but is not a run counter.
    BadRunCounter(PathBuf),
    Storage(HumanStorageError),
}

impl std::fmt::Display for HumanStoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for HumanStoreError {}

/// What the store has done so far in this run.
#[derive(Debug, Clone)]
pub struct HumanStoreStatus {
    pub run_id: String,
    pub humans_dir: PathBuf,
    /// Full syncs performed (cadence and explicit).
    pub syncs: u64,
    /// Failed folder writes, summed over all syncs.
    pub errors: u64,
    pub last_error: Option<String>,
    /// Most humans written by one cadence firing. A larger population is
    /// written in rotating shards (each human at least every
    /// `ceil(n / max_per_sync)` firings); a snapshot save writes everyone.
    pub max_per_sync: usize,
}

/// Default shard size: about 17 s of disk work per firing on the reference
/// machine (`benchmarks/island_population.md`).
pub const DEFAULT_MAX_PER_SYNC: usize = 1_000;

impl HumanStoreStatus {
    pub(crate) fn record(&mut self, errors: Vec<HumanStorageError>) {
        self.syncs += 1;
        self.errors += errors.len() as u64;
        if let Some(first) = errors.first() {
            self.last_error = Some(first.to_string());
        }
    }
}

/// Read, increment and atomically rewrite the run counter in `runs.json`.
fn next_run_number(root: &Path) -> Result<u64, HumanStoreError> {
    let path = root.join("runs.json");
    let current = match std::fs::read(&path) {
        Ok(bytes) => serde_json::from_slice::<serde_json::Value>(&bytes)
            .ok()
            .and_then(|v| v.get("next")?.as_u64())
            .ok_or_else(|| HumanStoreError::BadRunCounter(path.clone()))?,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => 0,
        Err(e) => return Err(HumanStoreError::Io(path, e)),
    };
    let tmp = path.with_extension("tmp");
    let body = serde_json::json!({ "next": current + 1 }).to_string();
    std::fs::write(&tmp, body).map_err(|e| HumanStoreError::Io(tmp.clone(), e))?;
    std::fs::rename(&tmp, &path).map_err(|e| HumanStoreError::Io(path, e))?;
    Ok(current)
}

fn run_id(world: &IslandWorldState, number: u64) -> String {
    let mut h = blake3::Hasher::new();
    h.update(&scenario_digest(&world.life.scenario));
    h.update(&world.life.scenario.seed);
    h.update(&number.to_le_bytes());
    h.finalize().to_hex()[..12].to_string()
}

impl IslandWorldState {
    /// Attach per-human folders under `<save_root>/<run_id>/humans/` and
    /// create one for every human now alive. Later births and arrivals get
    /// their folder when they are added; a death is written that step; every
    /// living human is rewritten on the scenario's store cadence. The run
    /// counter makes a fresh run under the same root use a new `run_id`.
    pub fn enable_human_store(&mut self, save_root: &Path) -> Result<String, HumanStoreError> {
        std::fs::create_dir_all(save_root)
            .map_err(|e| HumanStoreError::Io(save_root.to_path_buf(), e))?;
        let id = run_id(self, next_run_number(save_root)?);
        let humans_dir = save_root.join(&id).join("humans");
        std::fs::create_dir_all(&humans_dir)
            .map_err(|e| HumanStoreError::Io(humans_dir.clone(), e))?;
        let storage = HumanStorage::try_new(&humans_dir).map_err(HumanStoreError::Storage)?;
        self.life
            .humans
            .registry
            .set_storage(storage)
            .map_err(HumanStoreError::Storage)?;
        self.life.humans.set_auto_sync(false);
        self.life.store = Some(HumanStoreStatus {
            run_id: id.clone(),
            humans_dir,
            syncs: 0,
            errors: 0,
            last_error: None,
            max_per_sync: DEFAULT_MAX_PER_SYNC,
        });
        Ok(id)
    }

    /// Write every human's folder now (call before saving a snapshot, so the
    /// folders match it). A no-op without a store; failures are counted.
    pub fn sync_human_store(&mut self) {
        self.life.sync_store();
    }

    /// Change the shard size (at least 1).
    pub fn set_store_max_per_sync(&mut self, n: usize) {
        if let Some(s) = self.life.store.as_mut() {
            s.max_per_sync = n.max(1);
        }
    }

    pub fn human_store_status(&self) -> Option<&HumanStoreStatus> {
        self.life.store.as_ref()
    }
}
