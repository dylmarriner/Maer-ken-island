use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit},
    Aes256Gcm, Key, Nonce,
};
use rand::RngCore;
use std::fs;
use std::path::PathBuf;

#[derive(Debug)]
pub enum EncryptionError {
    Io(std::io::Error),
    Crypto(String),
    InvalidKey(String),
}

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
            // Expecting 64 hex chars for 32 bytes
            let decoded = Self::decode_hex(&k).map_err(EncryptionError::InvalidKey)?;
            if decoded.len() != 32 {
                return Err(EncryptionError::InvalidKey(
                    "Key must be 32 bytes".to_string(),
                ));
            }
            let mut key = [0u8; 32];
            key.copy_from_slice(&decoded);
            key
        } else if key_path.exists() {
            let k = fs::read(&key_path)?;
            if k.len() != 32 {
                return Err(EncryptionError::InvalidKey(
                    "Key file must be 32 bytes".to_string(),
                ));
            }
            let mut key = [0u8; 32];
            key.copy_from_slice(&k);
            key
        } else {
            // Generate a new key and save it
            let mut key = [0u8; 32];
            rand::thread_rng().fill_bytes(&mut key);
            fs::write(&key_path, key)?;
            // Set permissions to 0600 on unix
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(&key_path, fs::Permissions::from_mode(0o600))?;
            }
            key
        };

        Ok(Self {
            key: *Key::<Aes256Gcm>::from_slice(&key_bytes),
        })
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

    fn decode_hex(s: &str) -> Result<Vec<u8>, String> {
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|e| e.to_string()))
            .collect()
    }
}
