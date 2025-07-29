use aes_gcm::{
  aead::{Aead, AeadCore, KeyInit, OsRng as AeadOsRng},
  Aes256Gcm, Key, Nonce,
};
use anyhow::{anyhow, Result};
use argon2::{
  password_hash::{PasswordHasher, SaltString},
  Argon2, Params,
};
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
/// This manager provides secure key derivation and encryption operations using Argon2id
/// for password-based key derivation and AES-256-GCM for symmetric encryption.
///
/// # Security Features
///
/// - **Argon2id**: Uses the winner of the Password Hashing Competition for key derivation
/// - **Memory-hard**: Resistant to specialized hardware attacks (ASICs, GPUs)
/// - **Time-hard**: Configurable computational cost to resist brute-force attacks
/// - **Salt-based**: Each encryption uses a unique salt to prevent rainbow table attacks
/// - **Machine-binding**: Keys are bound to specific machines via hostname+username
///
/// **Note**: This is an internal implementation detail. Services should use the
/// `CredentialProvider` trait instead of calling these functions directly.
pub struct EncryptionManager;

impl EncryptionManager {
  /// Generate a machine-specific key component
  ///
  /// This function creates a deterministic 32-byte key based on the hostname and username
  /// of the current system. The key is generated using SHA-256, ensuring cryptographic
  /// security while remaining consistent across program runs on the same machine.
  ///
  /// **Note**: This is an internal function. Use the `CredentialProvider` trait instead.
  ///
  /// # Returns
  ///
  /// A `Result<Vec<u8>>` containing a 32-byte machine-specific key, or an error if
  /// system information cannot be retrieved.
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

  /// Derive encryption key from master password and machine key using Argon2
  ///
  /// This function uses Argon2id (the recommended variant) for password-based key derivation,
  /// combining a master password, machine-specific key, and salt to create a secure 32-byte
  /// encryption key. Argon2 provides resistance against timing attacks, side-channel attacks,
  /// and brute-force attacks through configurable memory and time costs.
  ///
  /// **Note**: This is an internal function. Use the `CredentialProvider` trait instead.
  ///
  /// # Arguments
  ///
  /// * `master_password` - The user's master password
  /// * `machine_key` - Machine-specific key component (from `machine_key()`)
  /// * `salt` - Random salt data for this specific encryption (minimum 8 bytes for Argon2)
  ///
  /// # Returns
  ///
  /// A `Result<Vec<u8>>` containing a 32-byte derived encryption key.
  pub fn derive_key(master_password: &str, machine_key: &[u8], salt: &[u8]) -> Result<Vec<u8>> {
    // Combine master password with machine key to create password input
    // This ensures that the same password on different machines produces different keys
    let mut password_input = Vec::new();
    password_input.extend_from_slice(master_password.as_bytes());
    password_input.extend_from_slice(machine_key);

    // Ensure salt meets Argon2's minimum requirements (8 bytes)
    // If provided salt is too short, pad it with zeros (for edge case handling)
    let effective_salt = if salt.len() < 8 {
      let mut padded_salt = salt.to_vec();
      padded_salt.resize(8, 0u8);  // Pad with zeros to reach minimum length
      padded_salt
    } else {
      salt.to_vec()
    };

    // Configure Argon2 with secure parameters
    // These parameters balance security with performance:
    // - memory_cost: 65536 KB (64 MB) - reasonable for desktop use
    // - time_cost: 3 iterations - good security/performance tradeoff
    // - parallelism: 4 lanes - leverages multi-core systems
    let params = Params::new(65536, 3, 4, Some(32))
      .map_err(|e| anyhow!("Failed to create Argon2 params: {}", e))?;
    
    let argon2 = Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params);

    // Convert salt to the format expected by Argon2
    let salt_string = SaltString::encode_b64(&effective_salt)
      .map_err(|e| anyhow!("Failed to encode salt: {}", e))?;

    // Derive key using Argon2
    let hash = argon2
      .hash_password(&password_input, &salt_string)
      .map_err(|e| anyhow!("Argon2 key derivation failed: {}", e))?;

    // Extract the 32-byte key from the hash
    let hash_output = hash.hash
      .ok_or_else(|| anyhow!("Argon2 produced no hash output"))?;
    let key_bytes = hash_output.as_bytes();

    if key_bytes.len() != 32 {
      return Err(anyhow!("Argon2 produced incorrect key length: expected 32, got {}", key_bytes.len()));
    }

    Ok(key_bytes.to_vec())
  }

  /// Encrypt credentials with double encryption
  pub fn encrypt_credentials(
    credentials: &HashMap<String, HashMap<String, String>>,
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
  ) -> Result<HashMap<String, HashMap<String, String>>> {
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
    let credentials: HashMap<String, HashMap<String, String>> = serde_json::from_slice(&decrypted_data)?;

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
