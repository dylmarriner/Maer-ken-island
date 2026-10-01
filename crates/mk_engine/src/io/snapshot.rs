use crate::world_integration::WorldState;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug)]
pub enum SnapshotError {
    SerializeError(String),
    FileError(String),
    CanonMismatch,
    HashError,
    /// The snapshot's canon fails `mk_core::canon::validator::validate_canon`.
    InvalidCanon(mk_core::canon::validator::CanonError),
}

impl std::fmt::Display for SnapshotError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            SnapshotError::SerializeError(msg) => write!(f, "Serialize error: {}", msg),
            SnapshotError::FileError(msg) => write!(f, "File error: {}", msg),
            SnapshotError::CanonMismatch => write!(f, "Canon digest mismatch"),
            SnapshotError::HashError => write!(f, "Hash chain error"),
            SnapshotError::InvalidCanon(err) => write!(f, "Snapshot canon is invalid: {err}"),
        }
    }
}

impl std::error::Error for SnapshotError {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotHeader {
    pub canon_digest: [u8; 32],
    pub tick: u64,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonSnapshot {
    pub horizon: String,
    pub config: serde_json::Value,
    pub tick: u64,
    pub timestamp: u64,
}

/// Length of the BLAKE3 digest prefix written before the serialized state.
const DIGEST_LEN: usize = 32;

/// Serialize `state` as deterministic JSON, prefix the bytes with a BLAKE3
/// digest of that serialization, and write the result to `path`.
///
/// Previously this wrote raw bincode bytes with no digest at all —
/// `SnapshotError::HashError`/`CanonMismatch` existed as enum variants but
/// nothing ever produced them (see
/// `audit-results/maerken-vs-gemini-markenz-gap-audit.md` finding 7). The
/// error model is now backed by a real check, mirroring the digest scheme
/// `WorldState::create_snapshot`/`load_snapshot` already use correctly
/// elsewhere. JSON is intentional here: live UI worlds contain human schema
/// values represented by `serde_json::Value`, which bincode cannot restore
/// because it does not implement `deserialize_any`.
pub fn save_snapshot(state: &WorldState, path: &Path) -> Result<(), SnapshotError> {
    let state_bytes =
        serde_json::to_vec(state).map_err(|e| SnapshotError::SerializeError(e.to_string()))?;

    let digest = blake3::hash(&state_bytes);

    let mut out = Vec::with_capacity(DIGEST_LEN + state_bytes.len());
    out.extend_from_slice(digest.as_bytes());
    out.extend_from_slice(&state_bytes);

    let temp_path = path.with_file_name(format!(
        ".{}.tmp-{}",
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("snapshot"),
        std::process::id()
    ));
    std::fs::write(&temp_path, out).map_err(|e| SnapshotError::FileError(e.to_string()))?;
    std::fs::rename(&temp_path, path).map_err(|e| SnapshotError::FileError(e.to_string()))?;

    Ok(())
}

pub fn load_snapshot(path: &Path) -> Result<(SnapshotHeader, WorldState), SnapshotError> {
    let data = std::fs::read(path).map_err(|e| SnapshotError::FileError(e.to_string()))?;

    // Try JSON format first (verification snapshots are JSON)
    if let Ok(json_snapshot) = serde_json::from_slice::<JsonSnapshot>(&data) {
        // Convert JSON snapshot to bincode format
        return load_json_snapshot_as_bincode(&json_snapshot);
    }

    // Otherwise try the digest-prefixed JSON format written by `save_snapshot`.
    if data.len() < DIGEST_LEN {
        return Err(SnapshotError::HashError);
    }
    let (stored_digest, state_bytes) = data.split_at(DIGEST_LEN);
    let actual_digest = blake3::hash(state_bytes);
    if actual_digest.as_bytes().as_slice() != stored_digest {
        return Err(SnapshotError::HashError);
    }

    let mut state: WorldState = serde_json::from_slice(state_bytes)
        .map_err(|e| SnapshotError::SerializeError(e.to_string()))?;
    let stored_canon_digest = state.canon_derived.canon_digest;
    mk_core::canon::validator::validate_canon(&state.canon).map_err(SnapshotError::InvalidCanon)?;

    // JSON numeric normalization can produce a byte-level digest difference
    // after a load even when the canon values are semantically unchanged.
    // Recompute all derived values from the loaded canon so the restored
    // state cannot retain stale derived data. The outer BLAKE3 digest above
    // still protects the persisted payload from accidental or unauthorised
    // modification.
    state.canon_derived = std::sync::Arc::new(mk_core::canon::CanonDerived::from(&state.canon));

    let header = SnapshotHeader {
        canon_digest: stored_canon_digest,
        tick: state.tick,
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
    };

    Ok((header, state))
}

fn load_json_snapshot_as_bincode(
    _json_snapshot: &JsonSnapshot,
) -> Result<(SnapshotHeader, WorldState), SnapshotError> {
    Err(SnapshotError::SerializeError(
        "verification JSON snapshots are not supported for world-state restoration; use persisted state snapshots"
            .to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use mk_core::canon::CanonLocked;
    use std::sync::Arc;

    fn test_world() -> WorldState {
        WorldState::new(Arc::new(CanonLocked::default()), [7u8; 32])
    }

    #[test]
    fn save_then_load_round_trips() {
        let dir =
            std::env::temp_dir().join(format!("mk_snapshot_roundtrip_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("state.bin");

        let world = test_world();
        save_snapshot(&world, &path).unwrap();

        let (header, loaded) = load_snapshot(&path).unwrap();
        assert_eq!(header.tick, world.tick);
        assert_eq!(loaded.tick, world.tick);
        assert_eq!(header.canon_digest, world.canon_derived.canon_digest);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn snapshot_with_an_invalid_canon_is_rejected() {
        let dir = std::env::temp_dir().join(format!("mk_snapshot_canon_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("state.bin");

        // A correctly digested payload whose canon was altered before saving.
        let mut world = test_world();
        world.canon = Arc::new(CanonLocked {
            rotation_period_s: CanonLocked::default().rotation_period_s * 2.0,
            ..CanonLocked::default()
        });
        save_snapshot(&world, &path).unwrap();

        assert!(matches!(
            load_snapshot(&path),
            Err(SnapshotError::InvalidCanon(_))
        ));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn tampered_file_is_rejected() {
        let dir = std::env::temp_dir().join(format!("mk_snapshot_tamper_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("state.bin");

        save_snapshot(&test_world(), &path).unwrap();

        // Flip a byte in the state payload (after the digest prefix) —
        // this is exactly the scenario `SnapshotError::HashError` exists to
        // catch, and previously nothing ever did.
        let mut bytes = std::fs::read(&path).unwrap();
        let last = bytes.len() - 1;
        bytes[last] ^= 0xFF;
        std::fs::write(&path, &bytes).unwrap();

        let result = load_snapshot(&path);
        assert!(matches!(result, Err(SnapshotError::HashError)));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn truncated_file_is_rejected() {
        let dir =
            std::env::temp_dir().join(format!("mk_snapshot_truncated_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("state.bin");
        std::fs::write(&path, [0u8; 4]).unwrap();

        let result = load_snapshot(&path);
        assert!(matches!(result, Err(SnapshotError::HashError)));

        std::fs::remove_dir_all(&dir).ok();
    }
}
