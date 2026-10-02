use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit},
    Aes256Gcm, Key, Nonce,
};
use rand::RngCore;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub enum EncryptionError {
    Io(std::io::Error),
    Crypto(String),
    InvalidKey(String),
}

impl std::fmt::Display for EncryptionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EncryptionError::Io(e) => write!(f, "I/O error: {}", e),
            EncryptionError::Crypto(msg) => write!(f, "crypto error: {}", msg),
            EncryptionError::InvalidKey(msg) => write!(f, "invalid storage key: {}", msg),
        }
    }
}

impl std::error::Error for EncryptionError {}

impl From<std::io::Error> for EncryptionError {
    fn from(err: std::io::Error) -> Self {
        EncryptionError::Io(err)
    }
}

#[derive(Debug, Clone)]
pub struct EncryptionManager {
    key: Key<Aes256Gcm>,
}

impl EncryptionManager {
    /// Initialize with a key from a file or environment variable.
    /// If none exists, it will create one in the provided base path if it's safe to do so.
    pub fn init(base_path: &PathBuf) -> Result<Self, EncryptionError> {
        if !base_path.exists() {
            fs::create_dir_all(base_path)?;
        }
        let key_path = base_path.join(".secret_storage_key");

        let key_bytes = if let Ok(k) = std::env::var("MK_STORAGE_KEY") {
            parse_storage_key(&k)?
        } else if key_path.exists() {
            Self::read_key_file(&key_path)?
        } else {
            Self::create_key_file(&key_path)?
        };

        Ok(Self {
            key: *Key::<Aes256Gcm>::from_slice(&key_bytes),
        })
    }

    /// Build a manager from an explicit 32-byte key (no environment or key
    /// file involved).
    pub fn from_key(key: [u8; 32]) -> Self {
        Self {
            key: *Key::<Aes256Gcm>::from_slice(&key),
        }
    }

    pub fn encrypt(&self, plaintext: &[u8]) -> Result<Vec<u8>, EncryptionError> {
        let cipher = Aes256Gcm::new(&self.key);
        let nonce = Aes256Gcm::generate_nonce(&mut rand::thread_rng());

        let ciphertext = cipher
            .encrypt(&nonce, plaintext)
            .map_err(|e| EncryptionError::Crypto(e.to_string()))?;

        let mut result = Vec::with_capacity(nonce.len() + ciphertext.len());
        result.extend_from_slice(&nonce);
        result.extend_from_slice(&ciphertext);
        Ok(result)
    }

    pub fn decrypt(&self, data: &[u8]) -> Result<Vec<u8>, EncryptionError> {
        if data.len() < 12 {
            return Err(EncryptionError::Crypto("Data too short".to_string()));
        }

        let cipher = Aes256Gcm::new(&self.key);
        let (nonce_bytes, ciphertext) = data.split_at(12);
        let nonce = Nonce::from_slice(nonce_bytes);

        cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| EncryptionError::Crypto(e.to_string()))
    }

    fn read_key_file(key_path: &Path) -> Result<[u8; 32], EncryptionError> {
        let k = fs::read(key_path)?;
        <[u8; 32]>::try_from(k.as_slice())
            .map_err(|_| EncryptionError::InvalidKey("Key file must be 32 bytes".to_string()))
    }

    /// Create the key file atomically with owner-only permissions.
    ///
    /// The key is written to a private temporary file (mode `0600` from the
    /// moment it exists, so it is never world-readable) and then hard-linked
    /// into place. Linking fails with `AlreadyExists` if another process won
    /// the race, in which case that process's key is used. Readers therefore
    /// never observe a partially written key file.
    fn create_key_file(key_path: &Path) -> Result<[u8; 32], EncryptionError> {
        let mut key = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut key);

        let mut suffix = [0u8; 8];
        rand::thread_rng().fill_bytes(&mut suffix);
        let tmp_path = key_path.with_extension(format!(
            "tmp-{}-{}",
            std::process::id(),
            suffix
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
        ));

        let written = Self::write_new_private_file(&tmp_path, &key);
        let linked = written.and_then(|()| fs::hard_link(&tmp_path, key_path));
        let _ = fs::remove_file(&tmp_path);
        match linked {
            Ok(()) => Ok(key),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                Self::read_key_file(key_path)
            }
            // The filesystem cannot hard-link (some network and FAT-style
            // mounts). Create the key file directly, still exclusively and
            // owner-only; a loser of that race waits for the winner's write.
            Err(_) => match Self::write_new_private_file(key_path, &key) {
                Ok(()) => Ok(key),
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                    Self::read_key_file_when_complete(key_path)
                }
                Err(e) => Err(e.into()),
            },
        }
    }

    /// Create `path` (failing if it exists) readable only by the owner, and
    /// write `key` to it durably.
    fn write_new_private_file(path: &Path, key: &[u8; 32]) -> std::io::Result<()> {
        let mut options = fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(path)?;
        file.write_all(key)?;
        file.sync_all()
    }

    /// Read a key file another process may still be writing: wait briefly for
    /// it to reach full length.
    fn read_key_file_when_complete(key_path: &Path) -> Result<[u8; 32], EncryptionError> {
        for _ in 0..50 {
            if fs::metadata(key_path)?.len() >= 32 {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        Self::read_key_file(key_path)
    }
}

/// Parse a 64-character hex string into a 32-byte storage key.
///
/// Rejects wrong lengths, non-ASCII input and non-hex digits with
/// [`EncryptionError::InvalidKey`] instead of panicking.
pub fn parse_storage_key(hex: &str) -> Result<[u8; 32], EncryptionError> {
    let hex = hex.trim();
    if hex.len() != 64 {
        return Err(EncryptionError::InvalidKey(format!(
            "Key must be 64 hex characters (32 bytes), got {}",
            hex.len()
        )));
    }
    if !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(EncryptionError::InvalidKey(
            "Key must contain only ASCII hex digits".to_string(),
        ));
    }
    let mut key = [0u8; 32];
    for (i, byte) in key.iter_mut().enumerate() {
        // Both characters were verified as ASCII hex digits above.
        *byte = u8::from_str_radix(&hex[2 * i..2 * i + 2], 16)
            .map_err(|e| EncryptionError::InvalidKey(e.to_string()))?;
    }
    Ok(key)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn is_invalid_key<T: std::fmt::Debug>(r: Result<T, EncryptionError>) -> bool {
        matches!(r, Err(EncryptionError::InvalidKey(_)))
    }

    #[test]
    fn valid_key_parses() {
        let hex = "00ff".repeat(16);
        let key = parse_storage_key(&hex).unwrap();
        assert_eq!(key[0], 0x00);
        assert_eq!(key[1], 0xff);
        assert_eq!(key.len(), 32);
        assert!(parse_storage_key(&hex.to_uppercase()).is_ok());
    }

    #[test]
    fn odd_length_is_rejected() {
        assert!(is_invalid_key(parse_storage_key(&"a".repeat(63))));
        assert!(is_invalid_key(parse_storage_key(&"a".repeat(65))));
    }

    #[test]
    fn non_ascii_is_rejected_without_panicking() {
        // 'é' is two bytes, so byte-index slicing would split a character.
        let key = format!("{}é", "a".repeat(62));
        assert_eq!(key.len(), 64);
        assert!(is_invalid_key(parse_storage_key(&key)));
    }

    #[test]
    fn non_hex_is_rejected() {
        assert!(is_invalid_key(parse_storage_key(&"g".repeat(64))));
    }

    #[test]
    fn wrong_length_is_rejected() {
        assert!(is_invalid_key(parse_storage_key("")));
        assert!(is_invalid_key(parse_storage_key("abcd")));
        assert!(is_invalid_key(parse_storage_key(&"ab".repeat(33))));
    }

    #[test]
    fn short_key_file_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("key");
        fs::write(&path, [1u8; 16]).unwrap();
        assert!(is_invalid_key(EncryptionManager::read_key_file(&path)));
    }

    #[test]
    fn created_key_file_is_private_and_reused() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(".secret_storage_key");

        let first = EncryptionManager::create_key_file(&path).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode, 0o600);
        }

        // Losing the creation race returns the winner's key and leaves no
        // temporary files behind.
        let second = EncryptionManager::create_key_file(&path).unwrap();
        assert_eq!(first, second);
        assert_eq!(fs::read(&path).unwrap(), first.to_vec());
        let leftovers = fs::read_dir(dir.path()).unwrap().count();
        assert_eq!(leftovers, 1, "temporary key files must be cleaned up");
    }

    #[test]
    fn encrypt_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(".secret_storage_key");
        let key = EncryptionManager::create_key_file(&path).unwrap();
        let manager = EncryptionManager {
            key: *Key::<Aes256Gcm>::from_slice(&key),
        };
        let sealed = manager.encrypt(b"gem-d").unwrap();
        assert_eq!(manager.decrypt(&sealed).unwrap(), b"gem-d");
    }
}
