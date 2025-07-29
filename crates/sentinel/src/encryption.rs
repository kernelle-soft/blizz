use aes_gcm::{
  aead::{Aead, AeadCore, KeyInit, OsRng as AeadOsRng},
  Aes256Gcm, Key, Nonce,
};
use anyhow::{anyhow, Result};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

/// Encrypted credential blob stored on disk
#[derive(Debug, Serialize, Deserialize)]
pub struct EncryptedBlob {
  pub data: Vec<u8>,
  pub nonce: Vec<u8>,
  pub salt: Vec<u8>,
}

/// In-memory credential cache
#[derive(Debug, Clone)]
pub struct CredentialCache {
  credentials: HashMap<String, String>,
}

impl CredentialCache {
  pub fn new() -> Self {
    Self { credentials: HashMap::new() }
  }

  pub fn store(&mut self, key: String, value: String) {
    self.credentials.insert(key, value);
  }

  pub fn get(&self, key: &str) -> Option<&String> {
    self.credentials.get(key)
  }

  pub fn clear(&mut self) {
    self.credentials.clear();
  }

  pub fn remove(&mut self, key: &str) -> Option<String> {
    self.credentials.remove(key)
  }

  pub fn from_map(credentials: HashMap<String, String>) -> Self {
    Self { credentials }
  }

  pub fn to_map(&self) -> &HashMap<String, String> {
    &self.credentials
  }
}

impl Default for CredentialCache {
  fn default() -> Self {
    Self::new()
  }
}

/// Encryption manager for double-encrypted credentials
///
/// This manager provides secure key derivation and encryption operations using SHA-256
/// for key derivation and AES-256-GCM for symmetric encryption.
///
/// # Example
///
/// ```rust
/// use sentinel::encryption::EncryptionManager;
/// use std::collections::HashMap;
///
/// // Generate a machine-specific key component
/// let machine_key = EncryptionManager::machine_key().unwrap();
/// assert_eq!(machine_key.len(), 32);
///
/// // Derive an encryption key
/// let password = "my_secure_password";
/// let salt = b"random_salt_data";
/// let derived_key = EncryptionManager::derive_key(password, &machine_key, salt).unwrap();
/// assert_eq!(derived_key.len(), 32);
/// ```
pub struct EncryptionManager;

impl EncryptionManager {
  /// Generate a machine-specific key component
  ///
  /// This function creates a deterministic 32-byte key based on the hostname and username
  /// of the current system. The key is generated using SHA-256, ensuring cryptographic
  /// security while remaining consistent across program runs on the same machine.
  ///
  /// # Returns
  ///
  /// A `Result<Vec<u8>>` containing a 32-byte machine-specific key, or an error if
  /// system information cannot be retrieved.
  ///
  /// # Example
  ///
  /// ```rust
  /// use sentinel::encryption::EncryptionManager;
  ///
  /// let machine_key = EncryptionManager::machine_key().unwrap();
  /// assert_eq!(machine_key.len(), 32);
  ///
  /// // The key should be deterministic
  /// let machine_key2 = EncryptionManager::machine_key().unwrap();
  /// assert_eq!(machine_key, machine_key2);
  /// ```
  pub fn machine_key() -> Result<Vec<u8>> {
    // Use hostname and username as machine-specific data
    let hostname =
      hostname::get().map_err(|_| anyhow!("Failed to get hostname"))?.to_string_lossy().to_string();

    let username = whoami::username();
    let machine_data = format!("{hostname}:{username}");

    // Use SHA-256 to hash the machine data to create a consistent key
    let mut hasher = Sha256::new();
    hasher.update(machine_data.as_bytes());
    let hash_result = hasher.finalize();

    // Convert to 32-byte key (SHA-256 produces exactly 32 bytes)
    Ok(hash_result.to_vec())
  }

  /// Derive encryption key from master password and machine key
  ///
  /// This function combines a master password, machine-specific key, and salt to create
  /// a secure 32-byte encryption key using SHA-256. The derivation is deterministic,
  /// meaning the same inputs will always produce the same output.
  ///
  /// # Arguments
  ///
  /// * `master_password` - The user's master password
  /// * `machine_key` - Machine-specific key component (from `machine_key()`)
  /// * `salt` - Random salt data for this specific encryption
  ///
  /// # Returns
  ///
  /// A `Result<Vec<u8>>` containing a 32-byte derived encryption key.
  ///
  /// # Example
  ///
  /// ```rust
  /// use sentinel::encryption::EncryptionManager;
  ///
  /// let machine_key = EncryptionManager::machine_key().unwrap();
  /// let password = "my_secure_password";
  /// let salt = b"unique_salt_data";
  ///
  /// let key = EncryptionManager::derive_key(password, &machine_key, salt).unwrap();
  /// assert_eq!(key.len(), 32);
  ///
  /// // Same inputs produce same key
  /// let key2 = EncryptionManager::derive_key(password, &machine_key, salt).unwrap();
  /// assert_eq!(key, key2);
  /// ```
  pub fn derive_key(master_password: &str, machine_key: &[u8], salt: &[u8]) -> Result<Vec<u8>> {
    // Combine master password, machine key, and salt
    let mut combined = Vec::new();
    combined.extend_from_slice(master_password.as_bytes());
    combined.extend_from_slice(machine_key);
    combined.extend_from_slice(salt);

    // Use SHA-256 to create encryption key
    let mut hasher = Sha256::new();
    hasher.update(&combined);
    let hash_result = hasher.finalize();

    // Return the 32-byte SHA-256 hash as the key
    Ok(hash_result.to_vec())
  }

  /// Encrypt credentials with double encryption
  pub fn encrypt_credentials(
    credentials: &HashMap<String, String>,
    master_password: &str,
  ) -> Result<EncryptedBlob> {
    // Generate salt and machine key
    let mut salt = vec![0u8; 16];
    rand::rng().fill_bytes(&mut salt);

    let machine_key = Self::machine_key()?;
    let encryption_key = Self::derive_key(master_password, &machine_key, &salt)?;

    // Serialize credentials
    let credentials_json = serde_json::to_vec(credentials)?;

    // Encrypt with AES-GCM
    let key = Key::<Aes256Gcm>::from_slice(&encryption_key);
    let cipher = Aes256Gcm::new(key);

    // Use AeadOsRng for nonce generation to avoid trait conflicts
    let nonce = Aes256Gcm::generate_nonce(&mut AeadOsRng);

    let encrypted_data = cipher
      .encrypt(&nonce, credentials_json.as_ref())
      .map_err(|e| anyhow!("Encryption failed: {}", e))?;

    Ok(EncryptedBlob { data: encrypted_data, nonce: nonce.to_vec(), salt })
  }

  /// Decrypt credentials with double decryption
  pub fn decrypt_credentials(
    blob: &EncryptedBlob,
    master_password: &str,
  ) -> Result<HashMap<String, String>> {
    // Derive the same encryption key
    let machine_key = Self::machine_key()?;
    let encryption_key = Self::derive_key(master_password, &machine_key, &blob.salt)?;

    // Decrypt with AES-GCM
    let key = Key::<Aes256Gcm>::from_slice(&encryption_key);
    let cipher = Aes256Gcm::new(key);
    let nonce = Nonce::from_slice(&blob.nonce);

    let decrypted_data =
      cipher.decrypt(nonce, blob.data.as_ref()).map_err(|e| anyhow!("Decryption failed: {}", e))?;

    // Deserialize credentials
    let credentials: HashMap<String, String> = serde_json::from_slice(&decrypted_data)?;

    Ok(credentials)
  }
}

// Add missing dependencies for hostname
use std::process::Command;

fn hostname() -> Result<std::ffi::OsString> {
  let output = Command::new("hostname").output()?;
  if output.status.success() {
    let hostname = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(std::ffi::OsString::from(hostname))
  } else {
    Err(anyhow!("Failed to get hostname"))
  }
}

mod hostname {
  pub fn get() -> Result<std::ffi::OsString, std::io::Error> {
    super::hostname().map_err(std::io::Error::other)
  }
}

mod whoami {
  pub fn username() -> String {
    std::env::var("USER")
      .or_else(|_| std::env::var("USERNAME"))
      .unwrap_or_else(|_| "unknown".to_string())
  }
}
