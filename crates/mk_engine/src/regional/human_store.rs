//! Phase 4 Task 3b: a folder for every human, under a run of its own.
//!
//! Two runs of the same scenario are two different histories, and the people
//! in them are different people who happen to share a name. Writing both into
//! one folder tree would let the second quietly overwrite the first, so each
//! run gets its own directory under the save root: a counter in `runs.json`
//! keeps a later run off an earlier one's id, and creating the directory —
//! one atomic operation that fails if it already exists — is what settles it
//! when two runs start at the same moment and read the same counter.
//!
//! The store is not part of the island's state. It records what happened; it
//! never decides what happens next. Enabling it, disabling it, or having
//! every write fail leaves the state digest identical — `island_human_store`
//! asserts exactly that, because a world that ran differently when you asked
//! it to keep records would not be worth the records.

use std::collections::BTreeSet;
use std::io::Write;
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

/// The run id for a scenario, seed and run number.
///
/// Derived rather than random or timestamped, so the same scenario and seed
/// give the same sequence of run ids and a run can be named again later.
fn run_id_for(scenario_digest: &[u8; 32], seed: &[u8; 32], counter: u64) -> RunId {
    let mut hasher = blake3::Hasher::new();
    hasher.update(scenario_digest);
    hasher.update(seed);
    hasher.update(&counter.to_le_bytes());
    RunId(
        hasher
            .finalize()
            .to_hex()
            .chars()
            .take(RUN_ID_LEN)
            .collect::<String>(),
    )
}

/// Read the run counter, treating an absent file as a fresh save root.
fn read_runs(runs_path: &Path) -> Result<Runs, HumanStoreError> {
    match std::fs::read(runs_path) {
        Ok(bytes) => serde_json::from_slice(&bytes)
            .map_err(|e| HumanStoreError::Runs(runs_path.into(), e.to_string())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Runs::default()),
        Err(e) => Err(HumanStoreError::Io(runs_path.into(), e)),
    }
}

/// Replace the run counter whole or not at all.
///
/// `fs::write` truncates in place, so a crash part-way through would leave
/// half a JSON document — and every later run would then fail to parse it,
/// making one torn write the permanent end of new runs under this save root.
/// The snapshot path already writes this way; the counter that decides where
/// a snapshot's people go deserves the same.
fn write_runs(runs_path: &Path, runs: &Runs) -> Result<(), HumanStoreError> {
    let text = serde_json::to_vec_pretty(runs)
        .map_err(|e| HumanStoreError::Runs(runs_path.into(), e.to_string()))?;
    let temp = runs_path.with_extension(format!("tmp-{}", std::process::id()));
    let write = (|| -> std::io::Result<()> {
        let mut file = std::fs::File::create(&temp)?;
        file.write_all(&text)?;
        file.sync_all()?;
        std::fs::rename(&temp, runs_path)
    })();
    if let Err(e) = write {
        std::fs::remove_file(&temp).ok();
        return Err(HumanStoreError::Io(runs_path.into(), e));
    }
    Ok(())
}

/// How many ids to try before giving up on finding an unclaimed run.
///
/// Only a concurrent `open_run` under the same save root makes an attempt
/// fail, so reaching the end of this means something is wrong that trying
/// harder will not fix.
const RUN_CLAIM_ATTEMPTS: u64 = 64;

/// Claim the next run under `save_root` and open a store for it.
///
/// The id is `blake3(scenario digest ‖ seed ‖ counter)`, so it is stable for
/// a given scenario, seed and run number rather than a timestamp or a random
/// value that would make two otherwise identical runs unreproducible.
///
/// The run is claimed by creating its directory with `create_dir`, which
/// fails if it already exists — that, not the counter, is what makes the
/// claim exclusive. The counter is a read-modify-write on a shared file and
/// cannot be exclusive by itself: two processes starting at once would read
/// the same `next` and derive the same id. Creating the directory is a
/// single atomic operation, so exactly one of them wins it and the loser
/// takes the following number. The counter's job is to stop a *later* run
/// reusing an earlier one's id, which it still does.
pub fn open_run(
    save_root: &Path,
    scenario_digest: &[u8; 32],
    seed: &[u8; 32],
) -> Result<(HumanStore, HumanStorage), HumanStoreError> {
    std::fs::create_dir_all(save_root).map_err(|e| HumanStoreError::Io(save_root.into(), e))?;
    let runs_path = save_root.join(RUNS_FILE);
    let start = read_runs(&runs_path)?.next;

    for counter in start..start + RUN_CLAIM_ATTEMPTS {
        let run_id = run_id_for(scenario_digest, seed, counter);
        let root = save_root.join(run_id.as_str());
        match std::fs::create_dir(&root) {
            Ok(()) => {}
            // Somebody else holds this one: theirs, and we take the next.
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(HumanStoreError::Io(root, e)),
        }

        // The run is ours now, so the counter is brought up to match.
        // Re-read rather than reuse what we started from: a concurrent run
        // may have moved it on, and the id list should hold both.
        let mut runs = read_runs(&runs_path)?;
        runs.next = runs.next.max(counter + 1);
        runs.ids.push(run_id.clone());
        write_runs(&runs_path, &runs)?;

        let humans_dir = root.join("humans");
        std::fs::create_dir_all(&humans_dir)
            .map_err(|e| HumanStoreError::Io(humans_dir.clone(), e))?;
        // `try_new` rather than `new`: a missing storage key must refuse,
        // not quietly write people's records in plaintext.
        let storage = HumanStorage::try_new(&humans_dir).map_err(HumanStoreError::Storage)?;

        return Ok((
            HumanStore {
                run_id,
                root,
                living: BTreeSet::new(),
                failed_writes: 0,
                last_error: None,
            },
            storage,
        ));
    }

    Err(HumanStoreError::Runs(
        runs_path,
        format!(
            "every run from {start} to {} is already claimed",
            start + RUN_CLAIM_ATTEMPTS
        ),
    ))
}
