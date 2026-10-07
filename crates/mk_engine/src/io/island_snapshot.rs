//! Phase 4 Task 3: writing a whole island to a file and reading it back.
//!
//! The island's state is large and almost all of it is irreplaceable: a
//! quarter of a million individual stems with their own positions and
//! diameters, two people with their memories, and the keyed RNG streams that
//! decide what happens next. A save that silently dropped or rounded any of
//! it would restore a different island, so this module is built around one
//! rule: a file either restores the exact state that was written, digest for
//! digest, or it refuses to load at all.
//!
//! The file is framed so that each way it can be wrong has its own answer:
//!
//! ```text
//! [0 .. 8)    magic "MKISLND\0"      - this is not some other file
//! [8 .. 40)   blake3(rest of file)   - nothing was altered in transit
//! [40 .. 72)  canon digest           - the physics it was written under
//! [72 ..  )   the state, deflated    - the island itself
//! ```
//!
//! The state is JSON rather than bincode for the same reason
//! [`crate::io::snapshot`] uses it: human schema values are
//! `serde_json::Value`, and bincode 1.x cannot deserialize a self-describing
//! value because it never implements `deserialize_any`. A snapshot that
//! cannot restore a person's genome is not a snapshot.
//!
//! That JSON is then deflated, which is not a nicety. An island is a dozen
//! grids of 1,152,000 cells each, and written out plainly the full island
//! comes to 338 MB — most of it the decimal expansions of numbers that
//! barely differ from their neighbours. Deflate takes the same island to
//! 44 MB, and the digest covers the compressed bytes, so what is checked is
//! exactly what is on the disk.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use mk_core::canon::{CanonDerived, CanonLocked};

use crate::regional::life::{IslandLife, IslandLifeError, IslandLifeSnapshot};

/// Marks the file as an island snapshot before anything else is read.
const MAGIC: &[u8; 8] = b"MKISLND\0";
/// Length of a BLAKE3 digest.
const DIGEST_LEN: usize = 32;
/// Where the digested region starts: everything from the canon digest on.
const DIGESTED_FROM: usize = MAGIC.len() + DIGEST_LEN;
/// Where the serialized state starts.
const PAYLOAD_FROM: usize = DIGESTED_FROM + DIGEST_LEN;
/// Deflate level: the library's default, in the middle of the range. Level 9
/// has not been measured against it on an island.
const DEFLATE_LEVEL: u32 = 6;
/// The largest state this build will inflate from a file. A deflate stream
/// a few hundred bytes long can claim to expand to gigabytes, and a loader
/// that believed it would exhaust memory before it ever checked the state.
/// A gigabyte is three times the largest state observed (338 MB before
/// compression), so it leaves room to grow.
const MAX_STATE_BYTES: u64 = 1 << 30;

/// Every way loading an island can fail, kept apart so a caller can tell a
/// corrupted file from one written under different physics.
#[derive(Debug)]
pub enum IslandSnapshotError {
    /// The file could not be read, written, or renamed into place.
    Io(std::io::Error),
    /// The state could not be serialized, or the file's payload is not the
    /// state this build expects.
    Serialize(serde_json::Error),
    /// The file does not begin with the island snapshot magic.
    NotASnapshot,
    /// The file is shorter than its own header.
    Truncated { bytes: usize },
    /// The file's digest does not match its contents.
    Tampered,
    /// The file is whole and correctly digested, but its payload is not a
    /// deflate stream this build can read. Only a deliberately built file
    /// reaches this: a save always writes a stream it can read back.
    Compression(std::io::Error),
    /// The payload claims to expand to more than this build will hold. A
    /// short deflate stream can name a size large enough to exhaust memory,
    /// so the size is refused rather than allocated.
    TooLarge,
    /// The snapshot was written under a different canon. Restoring it would
    /// run a saved world under physics it never experienced.
    CanonMismatch {
        stored: [u8; DIGEST_LEN],
        canon: [u8; DIGEST_LEN],
    },
    /// The state deserialized but does not describe an island this build can
    /// rebuild — a future format version, or a profile that no longer yields
    /// a valid domain.
    Restore(IslandLifeError),
}

impl std::fmt::Display for IslandSnapshotError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "island snapshot file error: {e}"),
            Self::Serialize(e) => write!(f, "island snapshot state error: {e}"),
            Self::NotASnapshot => write!(f, "not an island snapshot"),
            Self::Truncated { bytes } => {
                write!(f, "island snapshot is truncated: {bytes} bytes")
            }
            Self::Tampered => write!(f, "island snapshot does not match its own digest"),
            Self::Compression(e) => write!(f, "island snapshot payload will not inflate: {e}"),
            Self::TooLarge => write!(
                f,
                "island snapshot claims to hold more than {} MB of state",
                MAX_STATE_BYTES / 1_048_576
            ),
            Self::CanonMismatch { .. } => {
                write!(f, "island snapshot was written under a different canon")
            }
            Self::Restore(e) => write!(f, "island snapshot will not restore: {e}"),
        }
    }
}

impl std::error::Error for IslandSnapshotError {}

impl From<std::io::Error> for IslandSnapshotError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

/// The canon digest a snapshot is written under and checked against.
fn canon_digest(canon: &CanonLocked) -> [u8; DIGEST_LEN] {
    CanonDerived::from(canon).canon_digest
}

/// Write `life` to `path`, atomically.
///
/// The bytes go to a temporary file beside the target, are flushed to the
/// disk, and only then take the target's name. A crash part-way through
/// leaves the previous snapshot intact rather than a half-written one: the
/// rename is the only moment the file changes, and it either happens or it
/// does not.
///
/// Takes `&mut` because a checkpoint is a checkpoint of everything: if the
/// island is keeping folders for its people, they are brought up to date
/// here, so the snapshot and the folders beside it describe the same moment
/// rather than two moments a cadence apart.
pub fn save_island_snapshot(life: &mut IslandLife, path: &Path) -> Result<(), IslandSnapshotError> {
    life.sync_humans();
    // The state is written straight into the compressor, and the file is
    // assembled from pieces rather than concatenated. A full island is 338 MB
    // of JSON and 44 MB compressed; holding either of them more than once
    // would cost more memory than running the island does.
    let mut deflate =
        flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::new(DEFLATE_LEVEL));
    serde_json::to_writer(&mut deflate, &life.snapshot())
        .map_err(IslandSnapshotError::Serialize)?;
    let payload = deflate.finish()?;
    let canon = canon_digest(&life.canon);

    let mut hasher = blake3::Hasher::new();
    hasher.update(&canon);
    hasher.update(&payload);
    let digest = hasher.finalize();

    write_atomically(path, &[MAGIC, digest.as_bytes(), &canon, &payload])
}

/// Read an island back and rebuild it against `canon`.
///
/// Four checks stand between a file and a running island, in the order that
/// lets each one give a useful answer: that it is a snapshot at all, that it
/// is whole, that it belongs to this canon, and that this build knows its
/// format. Only then is the state handed to [`IslandLife::restore`], which
/// derives the domain, topology and labour table again rather than trusting
/// what the file says about them.
pub fn load_island_snapshot(
    canon: Arc<CanonLocked>,
    path: &Path,
) -> Result<IslandLife, IslandSnapshotError> {
    let bytes = std::fs::read(path)?;
    if bytes.len() < PAYLOAD_FROM {
        return Err(
            if bytes.len() >= MAGIC.len() && &bytes[..MAGIC.len()] != MAGIC {
                IslandSnapshotError::NotASnapshot
            } else {
                IslandSnapshotError::Truncated { bytes: bytes.len() }
            },
        );
    }
    if &bytes[..MAGIC.len()] != MAGIC {
        return Err(IslandSnapshotError::NotASnapshot);
    }
    let stored_digest = &bytes[MAGIC.len()..DIGESTED_FROM];
    if blake3::hash(&bytes[DIGESTED_FROM..]).as_bytes().as_slice() != stored_digest {
        return Err(IslandSnapshotError::Tampered);
    }

    let mut stored = [0u8; DIGEST_LEN];
    stored.copy_from_slice(&bytes[DIGESTED_FROM..PAYLOAD_FROM]);
    let wanted = canon_digest(&canon);
    if stored != wanted {
        return Err(IslandSnapshotError::CanonMismatch {
            stored,
            canon: wanted,
        });
    }

    // Parsed straight out of the decompressor: the state is 338 MB of JSON on
    // a full island, and holding all of it just to parse it would cost more
    // than running the island does. The cap rides on the reader, so a stream
    // that claims to expand forever stops at a gigabyte instead of being
    // allocated.
    let mut reader = std::io::BufReader::new(
        flate2::read::ZlibDecoder::new(&bytes[PAYLOAD_FROM..]).take(MAX_STATE_BYTES + 1),
    );
    let snapshot: IslandLifeSnapshot = match serde_json::from_reader(&mut reader) {
        Ok(snapshot) => snapshot,
        // A parse that stopped at the cap failed because of the cap, whatever
        // serde makes of the half-sentence it was left holding.
        Err(_) if reader.get_ref().limit() == 0 => return Err(IslandSnapshotError::TooLarge),
        Err(e) if e.is_io() => {
            return Err(IslandSnapshotError::Compression(std::io::Error::other(
                e.to_string(),
            )))
        }
        Err(e) => return Err(IslandSnapshotError::Serialize(e)),
    };
    if reader.get_ref().limit() == 0 {
        return Err(IslandSnapshotError::TooLarge);
    }
    IslandLife::restore(canon, snapshot).map_err(IslandSnapshotError::Restore)
}

/// A name beside `path` that no concurrent save will also pick.
fn temp_beside(path: &Path) -> PathBuf {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let serial = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("island");
    path.with_file_name(format!(".{name}.tmp-{}-{serial}", std::process::id()))
}

/// Write `pieces`, in order, so that `path` is either the old file or the
/// whole new one.
fn write_atomically(path: &Path, pieces: &[&[u8]]) -> Result<(), IslandSnapshotError> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    let temp = temp_beside(path);
    let write = (|| -> std::io::Result<()> {
        let mut file = std::fs::File::create(&temp)?;
        // Straight to the file: three small pieces and one large one, so a
        // buffer between them would only add syscalls.
        for piece in pieces {
            file.write_all(piece)?;
        }
        // The rename below is only atomic with respect to a crash if the
        // contents reached the disk first.
        file.sync_all()?;
        std::fs::rename(&temp, path)
    })();
    if let Err(e) = write {
        std::fs::remove_file(&temp).ok();
        return Err(IslandSnapshotError::Io(e));
    }
    // And the directory entry itself needs flushing, or the rename can be
    // lost while the file's contents survive. Opening a directory to fsync
    // it is a Unix idiom: Windows refuses, which is why the result is
    // discarded rather than raised. The snapshot is already whole and
    // renamed by this point, so the worst a refusal costs is that a power
    // cut in the next moment could lose the rename — and NTFS journals
    // metadata, which is the thing this call is standing in for.
    if let Some(parent) = path.parent() {
        let dir = if parent.as_os_str().is_empty() {
            Path::new(".")
        } else {
            parent
        };
        if let Ok(handle) = std::fs::File::open(dir) {
            handle.sync_all().ok();
        }
    }
    Ok(())
}
