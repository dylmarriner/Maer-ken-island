//! Phase 4 Task 3b: a folder for every human, under a run of its own.
//!
//! Two runs of the same scenario are two different histories, and the people
//! in them are different people who happen to share a name. Writing both into
//! one folder tree would let the second quietly overwrite the first, so each
//! run gets its own directory under the save root and a counter in
//! `runs.json` makes sure a fresh run never lands on an earlier run's id.
//!
//! The store is not part of the island's state. It records what happened; it
//! never decides what happens next. Enabling it, disabling it, or having
//! every write fail leaves the state digest identical — `island_human_store`
//! asserts exactly that, because a world that ran differently when you asked
//! it to keep records would not be worth the records.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use crate::io::human_storage::{HumanStorage, HumanStorageError};

/// Twelve hex characters: enough that two runs colliding is not a thing that
/// happens, short enough to type and to read in a path.
const RUN_ID_LEN: usize = 12;
/// Where the run counter lives, relative to the save root.
const RUNS_FILE: &str = "runs.json";

/// Names one run's folder tree.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub struct RunId(String);

impl RunId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for RunId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Everything that can go wrong attaching a store, kept apart from the
/// island's own errors: none of these stop the island running.
#[derive(Debug)]
pub enum HumanStoreError {
    /// The save root, or the run counter inside it, could not be read or
    /// written.
    Io(PathBuf, std::io::Error),
    /// `runs.json` exists but is not a run counter.
    Runs(PathBuf, String),
    /// The folder tree itself refused: a missing storage key, an unwritable
    /// directory, a human who would not serialize.
    Storage(HumanStorageError),
}

impl std::fmt::Display for HumanStoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(path, e) => write!(f, "{}: {e}", path.display()),
            Self::Runs(path, why) => write!(f, "{}: {why}", path.display()),
            Self::Storage(e) => write!(f, "human storage: {e}"),
        }
    }
}

impl std::error::Error for HumanStoreError {}

/// The run counter, as it sits in `runs.json`.
#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
struct Runs {
    /// How many runs have been started under this save root. The next run
    /// takes this value and leaves it one higher.
    next: u64,
    /// The ids handed out so far, newest last. Kept so a person looking at a
    /// save root can tell which folder belongs to which run without having
    /// to recompute a hash.
    #[serde(default)]
    ids: Vec<RunId>,
}

/// An island's open connection to its own records.
///
/// Holds no human data: the folders are the record, and this only knows
/// where they are, who it last saw alive, and what has gone wrong.
#[derive(Debug, Clone)]
pub struct HumanStore {
    run_id: RunId,
    root: PathBuf,
    /// Everyone the store last saw alive, so a death can be written the
    /// moment it happens rather than at the next cadence.
    living: BTreeSet<String>,
    /// Writes that failed since the store was enabled, and the most recent
    /// reason. Counted rather than discarded: a human whose folder cannot be
    /// written still exists, and the island carries on, but nobody should
    /// have to guess whether the records are complete.
    failed_writes: u64,
    last_error: Option<String>,
}

impl HumanStore {
    /// This run's id, which is also the name of its directory.
    pub fn run_id(&self) -> &RunId {
        &self.run_id
    }

    /// The run's directory: `<save root>/<run id>`.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Where this run's people live: `<save root>/<run id>/humans`.
    pub fn humans_dir(&self) -> PathBuf {
        self.root.join("humans")
    }

    /// How many writes have failed, and the most recent reason.
    pub fn failures(&self) -> (u64, Option<&str>) {
        (self.failed_writes, self.last_error.as_deref())
    }

    /// Record failures from a sync. Never returns them: the caller is the
    /// island tick, which must not stop because a disk did.
    pub(crate) fn note_failures(&mut self, errors: &[HumanStorageError]) {
        if let Some(first) = errors.first() {
            self.failed_writes += errors.len() as u64;
            self.last_error = Some(first.to_string());
            tracing::warn!(
                failed = errors.len(),
                error = %first,
                run = %self.run_id,
                "human folder writes failed; the island carries on"
            );
        }
    }

    pub(crate) fn note_failure(&mut self, error: &HumanStorageError) {
        self.note_failures(std::slice::from_ref(error));
    }

    /// Follow one human's status, collecting the ids of anyone who has died
    /// since the last substep so their final state can be written at once.
    pub(crate) fn note_status(&mut self, agent_id: &str, alive: bool, died: &mut Vec<String>) {
        if alive {
            if !self.living.contains(agent_id) {
                self.living.insert(agent_id.to_string());
            }
        } else if self.living.remove(agent_id) {
            died.push(agent_id.to_string());
        }
    }
}

/// Claim the next run under `save_root` and open a store for it.
///
/// The id is `blake3(scenario digest ‖ seed ‖ counter)`, so it is stable for
/// a given scenario, seed and run number rather than a timestamp or a random
/// value that would make two otherwise identical runs unreproducible. The
/// counter is what keeps a second run of the same scenario off the first
/// one's folders, and it is written back before any human is, so a crash
/// mid-run burns an id rather than reusing one.
pub fn open_run(
    save_root: &Path,
    scenario_digest: &[u8; 32],
    seed: &[u8; 32],
) -> Result<(HumanStore, HumanStorage), HumanStoreError> {
    std::fs::create_dir_all(save_root).map_err(|e| HumanStoreError::Io(save_root.into(), e))?;
    let runs_path = save_root.join(RUNS_FILE);

    let mut runs: Runs = match std::fs::read(&runs_path) {
        Ok(bytes) => serde_json::from_slice(&bytes)
            .map_err(|e| HumanStoreError::Runs(runs_path.clone(), e.to_string()))?,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Runs::default(),
        Err(e) => return Err(HumanStoreError::Io(runs_path, e)),
    };

    let counter = runs.next;
    let mut hasher = blake3::Hasher::new();
    hasher.update(scenario_digest);
    hasher.update(seed);
    hasher.update(&counter.to_le_bytes());
    let run_id = RunId(
        hasher
            .finalize()
            .to_hex()
            .chars()
            .take(RUN_ID_LEN)
            .collect::<String>(),
    );

    runs.next = counter + 1;
    runs.ids.push(run_id.clone());
    let text = serde_json::to_vec_pretty(&runs)
        .map_err(|e| HumanStoreError::Runs(runs_path.clone(), e.to_string()))?;
    std::fs::write(&runs_path, text).map_err(|e| HumanStoreError::Io(runs_path, e))?;

    let root = save_root.join(run_id.as_str());
    let humans_dir = root.join("humans");
    std::fs::create_dir_all(&humans_dir).map_err(|e| HumanStoreError::Io(humans_dir.clone(), e))?;
    // `try_new` rather than `new`: a missing storage key must refuse, not
    // quietly write people's records in plaintext.
    let storage = HumanStorage::try_new(&humans_dir).map_err(HumanStoreError::Storage)?;

    Ok((
        HumanStore {
            run_id,
            root,
            living: BTreeSet::new(),
            failed_writes: 0,
            last_error: None,
        },
        storage,
    ))
}
