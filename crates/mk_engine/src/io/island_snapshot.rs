//! Complete island snapshots (Phase 4 Task 3).
//!
//! File layout: an 8-byte magic, a `u32` format version, a blake3 digest of
//! the payload, then the payload: a JSON `Envelope` holding the scenario,
//! the canon and the whole island state. The digest makes tampering and
//! truncation fail loudly; the canon and scenario are validated before any
//! state is returned. Writing is atomic (temp file, fsync, rename).
//!
//! The payload is JSON, as the plan says (upstream human state embeds
//! `serde_json::Value`s, which compact binary formats cannot read back). It
//! is large: the medium hydrology grids dominate. Compression is a
//! Phase 5 option.

use std::io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use mk_core::canon::CanonLocked;
use mk_island::IslandScenario;
use serde::{Deserialize, Serialize};

use crate::regional::life::{IslandLife, IslandLifeSnapshot};
use crate::regional::world::IslandWorldState;

const MAGIC: &[u8; 8] = b"MKISLSNP";
/// The snapshot format this code reads and writes.
pub const FORMAT_VERSION: u32 = 1;
const HEADER_LEN: usize = 8 + 4 + 32;

#[derive(Debug)]
pub enum IslandSnapshotError {
    Io(PathBuf, std::io::Error),
    /// Not a snapshot, or cut short.
    Truncated,
    BadMagic,
    UnsupportedFormat(u32),
    /// The payload does not match its digest: damaged or tampered.
    DigestMismatch,
    Serialization(String),
    /// The scenario or its profile is not a version this code reads.
    IncompatibleScenario(String),
    /// The canon in the file does not match its recorded digest.
    CanonMismatch,
    /// The canon is not a valid canon (non-finite or non-positive values).
    InvalidCanon(String),
    /// The restored state does not hash to the digest recorded at save.
    StateHashMismatch,
}

impl std::fmt::Display for IslandSnapshotError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for IslandSnapshotError {}

#[derive(Serialize, Deserialize)]
struct Envelope {
    scenario: IslandScenario,
    canon: CanonLocked,
    canon_digest: [u8; 32],
    state_digest: [u8; 32],
    life: IslandLifeSnapshot,
}

struct HashingWriter<'a, W: Write> {
    inner: &'a mut W,
    hasher: blake3::Hasher,
}

impl<W: Write> Write for HashingWriter<'_, W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let n = self.inner.write(buf)?;
        self.hasher.update(&buf[..n]);
        Ok(n)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

fn canon_digest(canon: &CanonLocked) -> [u8; 32] {
    *blake3::hash(&bincode::serialize(canon).expect("canon serialises")).as_bytes()
}

/// A canon is usable only if its physical scalars are finite and positive.
fn check_canon(canon: &CanonLocked) -> Result<(), IslandSnapshotError> {
    let value = serde_json::to_value(canon)
        .map_err(|e| IslandSnapshotError::Serialization(e.to_string()))?;
    let bad = value
        .as_object()
        .into_iter()
        .flatten()
        .find_map(|(k, v)| v.as_f64().filter(|x| !x.is_finite()).map(|_| k.clone()));
    // serde_json maps non-finite floats to null.
    let null = value
        .as_object()
        .into_iter()
        .flatten()
        .find_map(|(k, v)| v.is_null().then(|| k.clone()));
    match bad.or(null) {
        Some(k) => Err(IslandSnapshotError::InvalidCanon(format!(
            "{k} is not a finite number"
        ))),
        None if canon.planet_radius_m <= 0.0 || canon.rotation_period_s <= 0.0 => {
            Err(IslandSnapshotError::InvalidCanon(
                "planet radius and rotation period must be positive".into(),
            ))
        }
        None => Ok(()),
    }
}

/// Save the island to `path`, atomically.
pub fn save_island_snapshot(
    state: &IslandWorldState,
    path: &Path,
) -> Result<(), IslandSnapshotError> {
    let canon = (*state.life.canon).clone();
    let envelope = Envelope {
        scenario: state.life.scenario.clone(),
        canon_digest: canon_digest(&canon),
        canon,
        state_digest: state.life.state_digest(),
        life: state.life.to_snapshot(),
    };
    let tmp = path.with_extension("tmp");
    let io = |e| IslandSnapshotError::Io(path.to_path_buf(), e);
    {
        // Stream the payload to disk through the hasher: the whole JSON is
        // never held in memory. The digest is patched into the header after.
        let file = std::fs::File::create(&tmp).map_err(io)?;
        let mut out = BufWriter::new(file);
        out.write_all(MAGIC).map_err(io)?;
        out.write_all(&FORMAT_VERSION.to_le_bytes()).map_err(io)?;
        out.write_all(&[0u8; 32]).map_err(io)?;
        let mut hashing = HashingWriter {
            inner: &mut out,
            hasher: blake3::Hasher::new(),
        };
        serde_json::to_writer(&mut hashing, &envelope)
            .map_err(|e| IslandSnapshotError::Serialization(e.to_string()))?;
        let digest = hashing.hasher.finalize();
        out.flush().map_err(io)?;
        let mut file = out.into_inner().map_err(|e| io(e.into_error()))?;
        file.seek(SeekFrom::Start(12)).map_err(io)?;
        file.write_all(digest.as_bytes()).map_err(io)?;
        file.sync_all().map_err(io)?;
    }
    std::fs::rename(&tmp, path).map_err(io)
}

/// Load an island snapshot, validating the digest, the format, the
/// scenario and canon before returning any state.
pub fn load_island_snapshot(path: &Path) -> Result<IslandWorldState, IslandSnapshotError> {
    let io = |e| IslandSnapshotError::Io(path.to_path_buf(), e);
    let mut file = std::fs::File::open(path).map_err(io)?;
    let mut header = [0u8; HEADER_LEN];
    file.read_exact(&mut header).map_err(|e| {
        if e.kind() == std::io::ErrorKind::UnexpectedEof {
            IslandSnapshotError::Truncated
        } else {
            IslandSnapshotError::Io(path.to_path_buf(), e)
        }
    })?;
    if &header[..8] != MAGIC {
        return Err(IslandSnapshotError::BadMagic);
    }
    let version = u32::from_le_bytes(header[8..12].try_into().expect("4 bytes"));
    if version != FORMAT_VERSION {
        return Err(IslandSnapshotError::UnsupportedFormat(version));
    }
    // Verify the digest in chunks, then parse from the same file: the
    // payload is never held in memory as one buffer.
    let mut hasher = blake3::Hasher::new();
    let mut chunk = vec![0u8; 1 << 20];
    loop {
        let n = file.read(&mut chunk).map_err(io)?;
        if n == 0 {
            break;
        }
        hasher.update(&chunk[..n]);
    }
    drop(chunk);
    if hasher.finalize().as_bytes() != &header[12..HEADER_LEN] {
        return Err(IslandSnapshotError::DigestMismatch);
    }
    file.seek(SeekFrom::Start(HEADER_LEN as u64)).map_err(io)?;
    let envelope: Envelope = serde_json::from_reader(BufReader::with_capacity(1 << 20, file))
        .map_err(|e| IslandSnapshotError::Serialization(e.to_string()))?;
    envelope
        .scenario
        .validate()
        .map_err(|e| IslandSnapshotError::IncompatibleScenario(e.to_string()))?;
    check_canon(&envelope.canon)?;
    if canon_digest(&envelope.canon) != envelope.canon_digest {
        return Err(IslandSnapshotError::CanonMismatch);
    }
    let life =
        IslandLife::from_snapshot(envelope.scenario, Arc::new(envelope.canon), envelope.life)
            .map_err(|e| IslandSnapshotError::Serialization(e.to_string()))?;
    if life.state_digest() != envelope.state_digest {
        return Err(IslandSnapshotError::StateHashMismatch);
    }
    Ok(IslandWorldState { life })
}
