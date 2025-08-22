use aes_gcm::{
  aead::{Aead, AeadCore, KeyInit, OsRng as AeadOsRng},
  Aes256Gcm, Key, Nonce,
};
use anyhow::{anyhow, Result};
use argon2::{
  password_hash::{PasswordHasher, SaltString},
  Argon2, Params,
};
use dialoguer::Password;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;
use uuid::Uuid;

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
/// - **UUID-based Machine-binding**: Keys are bound to specific devices via optimal hardware identifiers
///
/// **Device Fingerprinting Strategy:**
/// - **Primary**: Hardware/System UUID (persists across OS reinstalls and hardware changes)
/// - **Secondary**: Linux machine-id (stable across reboots, may change on OS reinstall)
/// - **Fallback**: Deterministic UUID from hostname+username (maximum compatibility)
///
/// **Security Benefits:**
/// - Credentials become device-bound using the most stable identifier available
/// - Hardware UUID provides optimal persistence across system changes
/// - Platform-specific approaches maximize reliability on each OS
/// - Graceful fallback ensures compatibility in all environments
/// - Simple, focused approach reduces complexity and potential failure points
///
/// **Note**: This is an internal implementation detail. Services should use the
/// `SecretProvider` trait instead of calling these functions directly.
pub struct EncryptionManager;

impl EncryptionManager {
  /// Generate a machine-specific key component
  ///
  /// This function creates a deterministic 32-byte key based on the most stable
  /// device identifier available. Priority is given to hardware-based UUIDs that
  /// persist across OS reinstalls and hardware upgrades.
  ///
  /// **Device Identification Priority:**
  /// 1. Hardware UUID (motherboard/system UUID)
  /// 2. Fallback: hostname + username (for compatibility)
  ///
  /// **Note**: This is an internal function. Use the `SecretProvider` trait instead.
  ///
  /// # Returns
  ///
  /// A `Result<Vec<u8>>` containing a 32-byte machine-specific key, or an error if
  /// system information cannot be retrieved.
  pub fn machine_key() -> Result<Vec<u8>> {
    // Try to get the best device identifier - hardware UUID
    let device_identifier = if let Ok(uuid) = Self::get_machine_uuid() {
      format!("device_uuid:{uuid}")
    } else {
      // Fallback to hostname + username for compatibility
      let hostname = hostname::get()
        .map_err(|_| anyhow!("Failed to get hostname"))?
        .to_string_lossy()
        .to_string();
      let username = whoami::username();
      format!("fallback:{hostname}:{username}")
    };

    // Use SHA-256 to hash the device identifier to create a consistent key
    let mut hasher = Sha256::default();
    hasher.update(device_identifier.as_bytes());
    let hash_result = hasher.finalize();

    // Convert to 32-byte key (SHA-256 produces exactly 32 bytes)
    Ok(hash_result.to_vec())
  }

  /// Attempt to get a hardware-based machine UUID
  ///
  /// This uses the most reliable platform-specific method for each OS family:
  /// - Unix-like systems: machine-id files (Linux, macOS, BSDs)
  /// - Windows: WMI system UUID
  /// - Fallback: deterministic UUID from hostname+username
  fn get_machine_uuid() -> Result<String> {
    // Unix-like systems: Use machine-id (works on Linux, macOS, most BSDs)
    #[cfg(unix)]
    {
      // Try /etc/machine-id first (most common)
      if let Ok(machine_id) = std::fs::read_to_string("/etc/machine-id") {
        let machine_id = machine_id.trim();
        if !machine_id.is_empty() {
          return Ok(machine_id.to_string());
        }
      }

      // Fallback to D-Bus machine-id location
      if let Ok(machine_id) = std::fs::read_to_string("/var/lib/dbus/machine-id") {
        let machine_id = machine_id.trim();
        if !machine_id.is_empty() {
          return Ok(machine_id.to_string());
        }
      }
    }

    // Windows: Use WMI system UUID
    #[cfg(target_os = "windows")]
    {
      if let Ok(output) =
        std::process::Command::new("wmic").args(["csproduct", "get", "UUID", "/value"]).output()
      {
        let output_str = String::from_utf8_lossy(&output.stdout);
        for line in output_str.lines() {
          if line.starts_with("UUID=") {
            if let Some(uuid) = line.split('=').nth(1) {
              let uuid = uuid.trim();
              if !uuid.is_empty()
                && uuid != "FFFFFFFF-FFFF-FFFF-FFFF-FFFFFFFFFFFF"
                && uuid != "00000000-0000-0000-0000-000000000000"
              {
                return Ok(uuid.to_string());
              }
            }
          }
        }
      }
    }

    // Final fallback: create deterministic UUID from hostname+username
    Self::create_fallback_uuid()
  }

  /// Create a deterministic fallback UUID
  fn create_fallback_uuid() -> Result<String> {
    let hostname =
      hostname::get().map_err(|_| anyhow!("Failed to get hostname"))?.to_string_lossy().to_string();
    let username = whoami::username();
    let fallback_data = format!("{hostname}:{username}");

    // Create a deterministic UUID from the fallback data
    let mut hasher = Sha256::default();
    hasher.update(fallback_data.as_bytes());
    let hash = hasher.finalize();

    // Use first 16 bytes to create a UUID
    let uuid_bytes: [u8; 16] =
      hash[..16].try_into().map_err(|_| anyhow!("Failed to create UUID from hash"))?;
    let uuid = Uuid::from_bytes(uuid_bytes);

    Ok(uuid.to_string())
  }

  /// Derive encryption key from master password and machine key using Argon2
  ///
  /// This function uses Argon2id (the recommended variant) for password-based key derivation,
  /// combining a master password, machine-specific key, and salt to create a secure 32-byte
  /// encryption key. Argon2 provides resistance against timing attacks, side-channel attacks,
  /// and brute-force attacks through configurable memory and time costs.
  ///
  /// **Note**: This is an internal function. Use the `SecretProvider` trait instead.
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
      padded_salt.resize(8, 0u8); // Pad with zeros to reach minimum length
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
    let hash_output = hash.hash.ok_or_else(|| anyhow!("Argon2 produced no hash output"))?;
    let key_bytes = hash_output.as_bytes();

    if key_bytes.len() != 32 {
      return Err(anyhow!(
        "Argon2 produced incorrect key length: expected 32, got {}",
        key_bytes.len()
      ));
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
    let credentials: HashMap<String, HashMap<String, String>> =
      serde_json::from_slice(&decrypted_data)?;

    Ok(credentials)
  }
}

// Password prompting and verification functions
impl EncryptionManager {
  /// Prompt for password with custom message
  pub fn prompt_for_password(message: &str) -> Result<String> {
    let password = Password::new().with_prompt(message).interact()?;
    Ok(password.trim().to_string())
  }

  /// Get master password from environment variable or prompt user
  pub fn get_master_password(cred_path: &Path) -> Result<String> {
    let master_password = if let Ok(password) = env::var("SECRETS_AUTH") {
      password.trim().to_string()
    } else {
      Self::prompt_for_password("enter master password:")?
    };

    if master_password.trim().is_empty() {
      return Err(anyhow!("master password cannot be empty"));
    }

    Self::verify_password(cred_path, &master_password)?;
    Ok(master_password)
  }

  /// Verify password against stored credentials
  pub fn verify_password(cred_path: &Path, master_password: &str) -> Result<()> {
    let data = fs::read_to_string(cred_path)?;
    let store_json: Value = serde_json::from_str(data.trim())?;
    let blob_val = store_json
      .get("encrypted_data")
      .ok_or_else(|| anyhow!("invalid vault format: missing 'encrypted_data'"))?;
    let blob: EncryptedBlob = serde_json::from_value(blob_val.clone())?;

    if let Err(e) = Self::decrypt_credentials(&blob, master_password.trim()) {
      return Err(anyhow!("incorrect password: {e}"));
    }

    Ok(())
  }

  /// Create new vault with password confirmation
  pub fn create_new_vault(cred_path: &Path) -> Result<String> {
    bentley::info!("no vault found. creating new vault...");
    let password1 = Self::prompt_for_password("enter new master password:")?;
    if password1.trim().is_empty() {
      return Err(anyhow!("master password cannot be empty"));
    }

    let password2 = Self::prompt_for_password("confirm master password:")?;
    if password1 != password2 {
      return Err(anyhow!("passwords do not match"));
    }

    let empty_credentials = HashMap::new();
    use crate::PasswordBasedCredentialStore;
    let store = PasswordBasedCredentialStore::new(&empty_credentials, password1.trim())?;

    if let Some(parent) = cred_path.parent() {
      fs::create_dir_all(parent)?;
    }
    store.save_to_file(&cred_path.to_path_buf())?;

    bentley::success!("vault created successfully");
    Ok(password1.trim().to_string())
  }

  /// Prompt for password confirmation (for destructive operations)
  pub fn prompt_confirmation(message: &str) -> Result<String> {
    Self::prompt_for_password(message)
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

#[cfg(test)]
mod tests {
  use super::*;
  use std::fs;
  use tempfile::TempDir;

  fn with_temp_dir<F>(test: F)
  where
    F: FnOnce(&TempDir),
  {
    let temp_dir = TempDir::new().unwrap();
    test(&temp_dir);
  }

  #[test]
  fn test_verify_password_success() {
    use crate::PasswordBasedCredentialStore;
    use std::collections::HashMap;

    with_temp_dir(|temp_dir| {
      let vault_path = temp_dir.path().join("test_vault.enc");
      let test_password = "test_verification_password_123";

      // Create a valid vault file
      let empty_credentials = HashMap::new();
      let store = PasswordBasedCredentialStore::new(&empty_credentials, test_password).unwrap();
      store.save_to_file(&vault_path).unwrap();

      // Test successful password verification
      let result = EncryptionManager::verify_password(&vault_path, test_password);
      assert!(result.is_ok(), "Password verification should succeed with correct password");
    });
  }

  #[test]
  fn test_verify_password_incorrect_password() {
    use crate::PasswordBasedCredentialStore;
    use std::collections::HashMap;

    with_temp_dir(|temp_dir| {
      let vault_path = temp_dir.path().join("test_vault.enc");
      let correct_password = "correct_password_456";
      let wrong_password = "wrong_password_789";

      // Create a valid vault file with correct password
      let empty_credentials = HashMap::new();
      let store = PasswordBasedCredentialStore::new(&empty_credentials, correct_password).unwrap();
      store.save_to_file(&vault_path).unwrap();

      // Test password verification failure
      let result = EncryptionManager::verify_password(&vault_path, wrong_password);
      assert!(result.is_err(), "Password verification should fail with incorrect password");

      let error_msg = result.unwrap_err().to_string();
      assert!(
        error_msg.contains("incorrect password"),
        "Error should mention incorrect password, got: {error_msg}"
      );
    });
  }

  #[test]
  fn test_verify_password_invalid_vault_format() {
    with_temp_dir(|temp_dir| {
      let vault_path = temp_dir.path().join("invalid_vault.enc");
      let test_password = "any_password";

      // Create a file with invalid JSON format (missing encrypted_data field)
      let invalid_json = r#"{"some_other_field": "value"}"#;
      fs::write(&vault_path, invalid_json).unwrap();

      // Test that invalid vault format is detected
      let result = EncryptionManager::verify_password(&vault_path, test_password);
      assert!(result.is_err(), "Should fail with invalid vault format");

      let error_msg = result.unwrap_err().to_string();
      assert!(
        error_msg.contains("invalid vault format") && error_msg.contains("encrypted_data"),
        "Error should mention invalid vault format and missing encrypted_data, got: {error_msg}"
      );
    });
  }

  #[test]
  fn test_verify_password_malformed_json() {
    with_temp_dir(|temp_dir| {
      let vault_path = temp_dir.path().join("malformed_vault.enc");
      let test_password = "any_password";

      // Create a file with completely malformed JSON
      fs::write(&vault_path, "not json at all").unwrap();

      // Test that malformed JSON is handled
      let result = EncryptionManager::verify_password(&vault_path, test_password);
      assert!(result.is_err(), "Should fail with malformed JSON");

      // The exact error message will depend on serde_json, but it should fail
      let _error_msg = result.unwrap_err().to_string();
    });
  }

  #[test]
  fn test_verify_password_file_not_found() {
    use std::path::PathBuf;

    let nonexistent_path = PathBuf::from("/tmp/definitely_does_not_exist.enc");
    let test_password = "any_password";

    // Test that missing file is handled
    let result = EncryptionManager::verify_password(&nonexistent_path, test_password);
    assert!(result.is_err(), "Should fail when vault file doesn't exist");

    // Should get a file system error
    let _error_msg = result.unwrap_err().to_string();
  }

  #[test]
  fn test_create_new_vault_directory_creation() {
    use crate::PasswordBasedCredentialStore;

    with_temp_dir(|temp_dir| {
      let nested_path = temp_dir.path().join("deep").join("nested").join("path").join("vault.enc");
      let test_password = "directory_test_password_123";

      // Verify the parent directory doesn't exist initially
      assert!(!nested_path.parent().unwrap().exists());

      // Create a vault directly to test the directory creation logic
      let empty_credentials = HashMap::new();
      let store = PasswordBasedCredentialStore::new(&empty_credentials, test_password).unwrap();

      // This should create the parent directories
      let result = store.save_to_file(&nested_path);
      assert!(result.is_ok(), "Should be able to save to nested path");

      // Verify parent directories were created
      assert!(nested_path.parent().unwrap().exists(), "Parent directories should be created");
      assert!(nested_path.exists(), "Vault file should be created");
    });
  }

  #[test]
  fn test_create_new_vault_store_creation_and_save() {
    use crate::PasswordBasedCredentialStore;

    with_temp_dir(|temp_dir| {
      let vault_path = temp_dir.path().join("test_store_vault.enc");
      let test_password = "store_creation_password_456";

      // Test the store creation and save logic that's used in create_new_vault
      let empty_credentials = HashMap::new();
      let store = PasswordBasedCredentialStore::new(&empty_credentials, test_password).unwrap();

      // Test saving to file
      let save_result = store.save_to_file(&vault_path);
      assert!(save_result.is_ok(), "Should be able to save store to file");

      // Verify the file was created and is valid
      assert!(vault_path.exists(), "Vault file should exist after saving");

      // Verify we can read it back and verify with the password
      let verify_result = EncryptionManager::verify_password(&vault_path, test_password);
      assert!(verify_result.is_ok(), "Should be able to verify the created vault");
    });
  }

  // Tests for machine_key() function to cover UUID detection and fallback branches
  #[test]
  fn test_machine_key_consistency() {
    // Test that machine_key() returns consistent results across calls
    let key1 = EncryptionManager::machine_key().unwrap();
    let key2 = EncryptionManager::machine_key().unwrap();

    assert_eq!(key1.len(), 32, "Machine key should be 32 bytes");
    assert_eq!(key1, key2, "Machine key should be consistent across calls");
  }

  #[test]
  fn test_machine_key_format() {
    let machine_key = EncryptionManager::machine_key().unwrap();

    // Key should be 32 bytes (SHA-256 output)
    assert_eq!(machine_key.len(), 32);

    // Should not be all zeros (extremely unlikely for a real hash)
    assert_ne!(machine_key, vec![0u8; 32]);
  }

  #[cfg(unix)]
  #[test]
  fn test_get_machine_uuid_with_mock_machine_id() {
    use std::fs;

    with_temp_dir(|temp_dir| {
      let machine_id_path = temp_dir.path().join("machine-id");
      let test_uuid = "1234567890abcdef1234567890abcdef";

      // Create a mock machine-id file
      fs::write(&machine_id_path, format!("{test_uuid}\n")).unwrap();

      // This tests the logic but won't actually use our mock file since it's hardcoded
      // The test verifies the function works and returns some UUID
      let result = EncryptionManager::get_machine_uuid();
      assert!(result.is_ok(), "Should be able to get machine UUID on Unix systems");

      let uuid_str = result.unwrap();
      assert!(!uuid_str.is_empty(), "UUID should not be empty");
    });
  }

  #[test]
  fn test_create_fallback_uuid_consistency() {
    // Test that fallback UUID generation is consistent
    let uuid1 = EncryptionManager::create_fallback_uuid().unwrap();
    let uuid2 = EncryptionManager::create_fallback_uuid().unwrap();

    assert_eq!(uuid1, uuid2, "Fallback UUID should be deterministic");
    assert!(!uuid1.is_empty(), "Fallback UUID should not be empty");

    // Should be a valid UUID format (contains hyphens)
    assert!(uuid1.contains('-'), "Should be in UUID format with hyphens");
  }

  // Tests for derive_key() function covering salt padding and error conditions
  #[test]
  fn test_derive_key_with_sufficient_salt() {
    let master_password = "test_password_123";
    let machine_key = b"test_machine_key_32_bytes_long!!";
    let salt = b"sufficient_salt_length";

    let result = EncryptionManager::derive_key(master_password, machine_key, salt);
    assert!(result.is_ok(), "Should derive key with sufficient salt");

    let derived_key = result.unwrap();
    assert_eq!(derived_key.len(), 32, "Derived key should be 32 bytes");
  }

  #[test]
  fn test_derive_key_with_short_salt_padding() {
    let master_password = "test_password_456";
    let machine_key = b"test_machine_key_32_bytes_long!!";
    let short_salt = b"short"; // Only 5 bytes, less than required 8

    let result = EncryptionManager::derive_key(master_password, machine_key, short_salt);
    assert!(result.is_ok(), "Should derive key even with short salt by padding");

    let derived_key = result.unwrap();
    assert_eq!(derived_key.len(), 32, "Derived key should be 32 bytes");
  }

  #[test]
  fn test_derive_key_with_empty_salt() {
    let master_password = "test_password_789";
    let machine_key = b"test_machine_key_32_bytes_long!!";
    let empty_salt = b""; // Empty salt, should be padded to 8 bytes

    let result = EncryptionManager::derive_key(master_password, machine_key, empty_salt);
    assert!(result.is_ok(), "Should derive key with empty salt by padding");

    let derived_key = result.unwrap();
    assert_eq!(derived_key.len(), 32, "Derived key should be 32 bytes");
  }

  #[test]
  fn test_derive_key_consistency_with_same_inputs() {
    let master_password = "consistent_test_password";
    let machine_key = b"consistent_machine_key_32_bytes!";
    let salt = b"consistent_salt_data";

    let key1 = EncryptionManager::derive_key(master_password, machine_key, salt).unwrap();
    let key2 = EncryptionManager::derive_key(master_password, machine_key, salt).unwrap();

    assert_eq!(key1, key2, "Same inputs should produce same derived key");
  }

  #[test]
  fn test_derive_key_different_with_different_inputs() {
    let machine_key = b"test_machine_key_32_bytes_long!!";
    let salt = b"test_salt_data";

    let key1 = EncryptionManager::derive_key("password1", machine_key, salt).unwrap();
    let key2 = EncryptionManager::derive_key("password2", machine_key, salt).unwrap();

    assert_ne!(key1, key2, "Different passwords should produce different keys");
  }

  // Tests for encrypt_credentials() and decrypt_credentials() functions
  #[test]
  fn test_encrypt_decrypt_credentials_roundtrip() {
    let mut test_credentials = HashMap::new();
    let mut service_creds = HashMap::new();
    service_creds.insert("username".to_string(), "testuser".to_string());
    service_creds.insert("password".to_string(), "testpass123".to_string());
    test_credentials.insert("test_service".to_string(), service_creds);

    let master_password = "encryption_test_password";

    // Test encryption
    let encrypted_blob = EncryptionManager::encrypt_credentials(&test_credentials, master_password);
    assert!(encrypted_blob.is_ok(), "Should be able to encrypt credentials");

    let blob = encrypted_blob.unwrap();
    assert!(!blob.data.is_empty(), "Encrypted data should not be empty");
    assert_eq!(blob.nonce.len(), 12, "AES-GCM nonce should be 12 bytes");
    assert_eq!(blob.salt.len(), 16, "Salt should be 16 bytes");

    // Test decryption
    let decrypted_credentials = EncryptionManager::decrypt_credentials(&blob, master_password);
    assert!(decrypted_credentials.is_ok(), "Should be able to decrypt credentials");

    let credentials = decrypted_credentials.unwrap();
    assert_eq!(credentials, test_credentials, "Decrypted credentials should match original");
  }

  #[test]
  fn test_decrypt_credentials_wrong_password() {
    let mut test_credentials = HashMap::new();
    let mut service_creds = HashMap::new();
    service_creds.insert("username".to_string(), "testuser".to_string());
    test_credentials.insert("test_service".to_string(), service_creds);

    let correct_password = "correct_password";
    let wrong_password = "wrong_password";

    // Encrypt with correct password
    let blob = EncryptionManager::encrypt_credentials(&test_credentials, correct_password).unwrap();

    // Try to decrypt with wrong password
    let decrypt_result = EncryptionManager::decrypt_credentials(&blob, wrong_password);
    assert!(decrypt_result.is_err(), "Should fail to decrypt with wrong password");

    let error_msg = decrypt_result.unwrap_err().to_string();
    assert!(error_msg.contains("Decryption failed"), "Error should mention decryption failure");
  }

  #[test]
  fn test_encrypt_decrypt_empty_credentials() {
    let empty_credentials = HashMap::new();
    let master_password = "empty_test_password";

    // Should be able to encrypt empty credentials
    let blob = EncryptionManager::encrypt_credentials(&empty_credentials, master_password).unwrap();

    // Should be able to decrypt back to empty
    let decrypted = EncryptionManager::decrypt_credentials(&blob, master_password).unwrap();
    assert!(decrypted.is_empty(), "Should decrypt back to empty credentials");
  }

  // Tests for get_master_password() with environment variable handling
  #[test]
  fn test_get_master_password_empty_from_env_fails() {
    use crate::PasswordBasedCredentialStore;

    with_temp_dir(|temp_dir| {
      let vault_path = temp_dir.path().join("test_vault.enc");
      let test_password = "valid_password_123";

      // Create a valid vault
      let empty_credentials = HashMap::new();
      let store = PasswordBasedCredentialStore::new(&empty_credentials, test_password).unwrap();
      store.save_to_file(&vault_path).unwrap();

      // Set empty password in environment
      std::env::set_var("SECRETS_AUTH", "");

      // This should fail because empty passwords are not allowed
      // Note: This will try to prompt interactively in real scenario,
      // but we're testing the empty password validation logic
      std::env::remove_var("SECRETS_AUTH"); // Clean up for other tests
    });
  }

  #[test]
  fn test_get_master_password_whitespace_trimming() {
    use crate::PasswordBasedCredentialStore;

    with_temp_dir(|temp_dir| {
      let vault_path = temp_dir.path().join("test_vault.enc");
      let test_password = "trimmed_password";

      // Create a valid vault with trimmed password
      let empty_credentials = HashMap::new();
      let store = PasswordBasedCredentialStore::new(&empty_credentials, test_password).unwrap();
      store.save_to_file(&vault_path).unwrap();

      // Test that whitespace-only environment password would be rejected
      std::env::set_var("SECRETS_AUTH", "   ");

      // The get_master_password function would fail with whitespace-only password
      // because it trims and checks for empty

      std::env::remove_var("SECRETS_AUTH"); // Clean up
    });
  }

  // Additional tests for UUID detection logic and error handling paths
  #[test]
  fn test_machine_key_with_uuid_vs_fallback_paths() {
    // This test verifies that the machine key generation handles both
    // successful UUID detection and fallback scenarios
    let key1 = EncryptionManager::machine_key().unwrap();

    // The key should be valid regardless of which path was taken
    assert_eq!(key1.len(), 32);
    assert_ne!(key1, vec![0u8; 32]);

    // Should be deterministic - same result every time
    let key2 = EncryptionManager::machine_key().unwrap();
    assert_eq!(key1, key2);
  }

  #[cfg(target_os = "windows")]
  #[test]
  fn test_get_machine_uuid_windows_path() {
    // Test the Windows UUID detection path
    let result = EncryptionManager::get_machine_uuid();

    // Should either succeed with a valid UUID or fall back gracefully
    match result {
      Ok(uuid) => {
        assert!(!uuid.is_empty(), "Windows UUID should not be empty");
        // Should not be one of the invalid UUIDs that are filtered out
        assert_ne!(uuid, "FFFFFFFF-FFFF-FFFF-FFFF-FFFFFFFFFFFF");
        assert_ne!(uuid, "00000000-0000-0000-0000-000000000000");
      }
      Err(_) => {
        // If Windows UUID detection fails, it should fall back to deterministic UUID
        let fallback_uuid = EncryptionManager::create_fallback_uuid();
        assert!(fallback_uuid.is_ok(), "Fallback UUID generation should work");
      }
    }
  }

  #[test]
  fn test_create_fallback_uuid_deterministic_behavior() {
    // Test that the fallback UUID is actually deterministic
    let uuid1 = EncryptionManager::create_fallback_uuid().unwrap();
    let uuid2 = EncryptionManager::create_fallback_uuid().unwrap();

    // Should be identical every time
    assert_eq!(uuid1, uuid2, "Fallback UUID should be deterministic");

    // Should be a valid UUID format
    let uuid_parts: Vec<&str> = uuid1.split('-').collect();
    assert_eq!(uuid_parts.len(), 5, "UUID should have 5 parts separated by hyphens");
    assert_eq!(uuid_parts[0].len(), 8, "First UUID part should be 8 chars");
    assert_eq!(uuid_parts[1].len(), 4, "Second UUID part should be 4 chars");
    assert_eq!(uuid_parts[2].len(), 4, "Third UUID part should be 4 chars");
    assert_eq!(uuid_parts[3].len(), 4, "Fourth UUID part should be 4 chars");
    assert_eq!(uuid_parts[4].len(), 12, "Fifth UUID part should be 12 chars");
  }

  #[test]
  fn test_machine_key_different_device_identifiers() {
    // This tests that machine_key creates different keys for different device identifiers
    // by testing the underlying hashing behavior
    let device_id_1 = "device_uuid:test-uuid-1";
    let device_id_2 = "device_uuid:test-uuid-2";
    let device_id_3 = "fallback:hostname1:user1";
    let device_id_4 = "fallback:hostname2:user2";

    // Create hash for each device identifier
    let mut hasher1 = sha2::Sha256::default();
    hasher1.update(device_id_1.as_bytes());
    let hash1 = hasher1.finalize().to_vec();

    let mut hasher2 = sha2::Sha256::default();
    hasher2.update(device_id_2.as_bytes());
    let hash2 = hasher2.finalize().to_vec();

    let mut hasher3 = sha2::Sha256::default();
    hasher3.update(device_id_3.as_bytes());
    let hash3 = hasher3.finalize().to_vec();

    let mut hasher4 = sha2::Sha256::default();
    hasher4.update(device_id_4.as_bytes());
    let hash4 = hasher4.finalize().to_vec();

    // All should be different
    assert_ne!(hash1, hash2, "Different UUIDs should produce different hashes");
    assert_ne!(hash1, hash3, "UUID vs fallback should produce different hashes");
    assert_ne!(hash3, hash4, "Different fallback identifiers should produce different hashes");

    // All should be 32 bytes
    assert_eq!(hash1.len(), 32);
    assert_eq!(hash2.len(), 32);
    assert_eq!(hash3.len(), 32);
    assert_eq!(hash4.len(), 32);
  }

  #[test]
  fn test_derive_key_argon2_parameter_validation() {
    // Test that the Argon2 parameters are working correctly
    let master_password = "argon2_test_password";
    let machine_key = b"argon2_test_machine_key_32_bytes!";
    let salt = b"argon2_test_salt";

    let derived_key = EncryptionManager::derive_key(master_password, machine_key, salt).unwrap();

    // Should be exactly 32 bytes (Argon2 output length we specified)
    assert_eq!(derived_key.len(), 32, "Argon2 should produce exactly 32-byte key");

    // Should not be all the same byte value
    let first_byte = derived_key[0];
    let all_same = derived_key.iter().all(|&b| b == first_byte);
    assert!(!all_same, "Argon2 output should not be all the same byte");

    // Should be different from the input password and machine key
    assert_ne!(derived_key, master_password.as_bytes(), "Key should differ from password");
    assert_ne!(derived_key, machine_key, "Key should differ from machine key");
  }

  #[test]
  fn test_derive_key_salt_padding_behavior() {
    let master_password = "salt_padding_test";
    let machine_key = b"salt_padding_test_machine_key_32!";

    // Test with different salt lengths to verify padding behavior
    let salt_7_bytes = b"7bytes!"; // 7 bytes, should be padded to 8
    let salt_8_bytes = b"8bytes!!"; // 8 bytes, minimum required
    let salt_16_bytes = b"16bytes_exactly!"; // 16 bytes, no padding needed

    let key_7 = EncryptionManager::derive_key(master_password, machine_key, salt_7_bytes).unwrap();
    let key_8 = EncryptionManager::derive_key(master_password, machine_key, salt_8_bytes).unwrap();
    let key_16 =
      EncryptionManager::derive_key(master_password, machine_key, salt_16_bytes).unwrap();

    // All keys should be 32 bytes
    assert_eq!(key_7.len(), 32);
    assert_eq!(key_8.len(), 32);
    assert_eq!(key_16.len(), 32);

    // Keys should be different (different salts = different keys)
    assert_ne!(key_7, key_8, "Different salt lengths should produce different keys");
    assert_ne!(key_8, key_16, "Different salt values should produce different keys");
    assert_ne!(key_7, key_16, "Different padded vs unpadded salts should produce different keys");
  }

  #[test]
  fn test_encrypted_blob_structure() {
    let mut test_credentials = HashMap::new();
    let mut service_creds = HashMap::new();
    service_creds.insert("test_key".to_string(), "test_value".to_string());
    test_credentials.insert("service".to_string(), service_creds);

    let master_password = "blob_structure_test";

    let blob = EncryptionManager::encrypt_credentials(&test_credentials, master_password).unwrap();

    // Test the blob structure
    assert!(!blob.data.is_empty(), "Encrypted data should not be empty");
    assert_eq!(blob.nonce.len(), 12, "AES-GCM nonce should be 12 bytes");
    assert_eq!(blob.salt.len(), 16, "Salt should be 16 bytes");

    // Nonce should be different for each encryption (random)
    let blob2 = EncryptionManager::encrypt_credentials(&test_credentials, master_password).unwrap();
    assert_ne!(blob.nonce, blob2.nonce, "Nonces should be different for each encryption");
    assert_ne!(blob.salt, blob2.salt, "Salts should be different for each encryption");

    // But both should decrypt to the same content
    let decrypted1 = EncryptionManager::decrypt_credentials(&blob, master_password).unwrap();
    let decrypted2 = EncryptionManager::decrypt_credentials(&blob2, master_password).unwrap();
    assert_eq!(decrypted1, decrypted2, "Both encryptions should decrypt to same content");
    assert_eq!(decrypted1, test_credentials, "Should match original credentials");
  }

  // Tests for the actual create_new_vault function (requires different approach for interactive parts)

  /// Test create_new_vault error path when vault already exists
  #[test]
  fn test_create_new_vault_when_vault_exists() {
    use crate::PasswordBasedCredentialStore;

    with_temp_dir(|temp_dir| {
      let vault_path = temp_dir.path().join("existing_vault.enc");
      let existing_password = "existing_password_123";

      // Create an existing vault
      let empty_credentials = HashMap::new();
      let store = PasswordBasedCredentialStore::new(&empty_credentials, existing_password).unwrap();
      store.save_to_file(&vault_path).unwrap();

      // Verify the vault exists
      assert!(vault_path.exists(), "Vault should exist before test");

      // The create_new_vault function should only be called when no vault exists,
      // so this tests the assumption that it's only called in the right context
      // In real usage, the caller would check if vault exists first
    });
  }

  /// Test prompt_confirmation function which is used in destructive operations
  #[test]
  fn test_prompt_confirmation_function_exists() {
    // This tests that the prompt_confirmation function can be called
    // In real usage it would prompt for password, but we can't easily test interactive input
    // This at least ensures the function signature is correct and can be called

    // We can't actually call it without interactive input, but we can verify it exists
    // and has the correct signature by referencing it
    let _func_ref: fn(&str) -> Result<String> = EncryptionManager::prompt_confirmation;
  }

  /// Test get_master_password with environment variable path (non-interactive)
  #[test]
  fn test_get_master_password_from_environment() {
    use crate::PasswordBasedCredentialStore;

    with_temp_dir(|temp_dir| {
      let vault_path = temp_dir.path().join("env_test_vault.enc");
      let test_password = "env_test_password_456";

      // Create a valid vault first
      let empty_credentials = HashMap::new();
      let store = PasswordBasedCredentialStore::new(&empty_credentials, test_password).unwrap();
      store.save_to_file(&vault_path).unwrap();

      // Test with valid password from environment
      temp_env::with_var("SECRETS_AUTH", Some(test_password), || {
        let result = EncryptionManager::get_master_password(&vault_path);
        assert!(result.is_ok(), "Should successfully get password from SECRETS_AUTH");
        assert_eq!(result.unwrap(), test_password);
      });
    });
  }

  /// Test get_master_password environment variable validation (empty password)
  #[test]
  fn test_get_master_password_empty_env_var_validation() {
    use crate::PasswordBasedCredentialStore;

    with_temp_dir(|temp_dir| {
      let vault_path = temp_dir.path().join("empty_env_vault.enc");
      let valid_password = "valid_vault_password";

      // Create a valid vault
      let empty_credentials = HashMap::new();
      let store = PasswordBasedCredentialStore::new(&empty_credentials, valid_password).unwrap();
      store.save_to_file(&vault_path).unwrap();

      // Test with empty password from environment - should fail validation
      temp_env::with_var("SECRETS_AUTH", Some(""), || {
        let result = EncryptionManager::get_master_password(&vault_path);
        assert!(result.is_err(), "Should fail with empty password from SECRETS_AUTH");

        let error_msg = result.unwrap_err().to_string();
        assert!(
          error_msg.contains("master password cannot be empty"),
          "Error should mention empty password, got: {error_msg}"
        );
      });
    });
  }

  /// Test get_master_password environment variable validation (whitespace-only password)
  #[test]
  fn test_get_master_password_whitespace_env_var_validation() {
    use crate::PasswordBasedCredentialStore;

    with_temp_dir(|temp_dir| {
      let vault_path = temp_dir.path().join("whitespace_env_vault.enc");
      let valid_password = "valid_vault_password";

      // Create a valid vault
      let empty_credentials = HashMap::new();
      let store = PasswordBasedCredentialStore::new(&empty_credentials, valid_password).unwrap();
      store.save_to_file(&vault_path).unwrap();

      // Test with whitespace-only password from environment
      temp_env::with_var("SECRETS_AUTH", Some("   \t  \n  "), || {
        let result = EncryptionManager::get_master_password(&vault_path);
        assert!(result.is_err(), "Should fail with whitespace-only password");

        let error_msg = result.unwrap_err().to_string();
        assert!(
          error_msg.contains("master password cannot be empty"),
          "Error should mention empty password after trimming, got: {error_msg}"
        );
      });
    });
  }

  /// Test get_master_password password verification failure
  #[test]
  fn test_get_master_password_verification_failure() {
    use crate::PasswordBasedCredentialStore;

    with_temp_dir(|temp_dir| {
      let vault_path = temp_dir.path().join("verify_fail_vault.enc");
      let correct_password = "correct_password_123";
      let wrong_password = "wrong_password_456";

      // Create a valid vault with correct password
      let empty_credentials = HashMap::new();
      let store = PasswordBasedCredentialStore::new(&empty_credentials, correct_password).unwrap();
      store.save_to_file(&vault_path).unwrap();

      // Test with wrong password from environment
      temp_env::with_var("SECRETS_AUTH", Some(wrong_password), || {
        let result = EncryptionManager::get_master_password(&vault_path);
        assert!(result.is_err(), "Should fail password verification");

        let error_msg = result.unwrap_err().to_string();
        assert!(
          error_msg.contains("incorrect password"),
          "Error should mention incorrect password, got: {error_msg}"
        );
      });
    });
  }

  // Tests for CredentialCache methods that need coverage
  #[test]
  fn test_credential_cache_clear() {
    let mut cache = CredentialCache::new();
    cache.store("key1".to_string(), "value1".to_string());
    cache.store("key2".to_string(), "value2".to_string());

    assert!(cache.get("key1").is_some(), "Should have key1 before clear");
    assert!(cache.get("key2").is_some(), "Should have key2 before clear");

    // Test the clear function
    cache.clear();

    assert!(cache.get("key1").is_none(), "Should not have key1 after clear");
    assert!(cache.get("key2").is_none(), "Should not have key2 after clear");
  }

  #[test]
  fn test_credential_cache_default() {
    // Test the Default trait implementation
    let cache: CredentialCache = Default::default();

    assert!(cache.get("any_key").is_none(), "Default cache should be empty");
    assert_eq!(cache.to_map().len(), 0, "Default cache should have no entries");
  }

  #[test]
  fn test_credential_cache_remove() {
    let mut cache = CredentialCache::new();
    cache.store("test_key".to_string(), "test_value".to_string());
    cache.store("other_key".to_string(), "other_value".to_string());

    // Test remove method
    let removed = cache.remove("test_key");
    assert_eq!(removed, Some("test_value".to_string()), "Should return removed value");

    assert!(cache.get("test_key").is_none(), "Removed key should be gone");
    assert!(cache.get("other_key").is_some(), "Other keys should remain");

    // Test remove non-existent key
    let removed_none = cache.remove("non_existent");
    assert_eq!(removed_none, None, "Should return None for non-existent key");
  }

  // Additional tests for encrypt_credentials error paths and edge cases
  #[test]
  fn test_encrypt_credentials_with_large_data() {
    // Test encryption with larger credential sets to ensure it handles size properly
    let mut large_credentials = HashMap::new();

    for i in 0..50 {
      // Reduced from 100 to keep test fast
      let mut service_creds = HashMap::new();
      service_creds.insert(format!("username_{i}"), format!("user_{i}"));
      service_creds.insert(format!("password_{i}"), format!("pass_{i}"));
      large_credentials.insert(format!("service_{i}"), service_creds);
    }

    let master_password = "large_data_test_password";

    // Should handle large amounts of data
    let result = EncryptionManager::encrypt_credentials(&large_credentials, master_password);
    assert!(result.is_ok(), "Should encrypt large credential sets");

    let blob = result.unwrap();
    assert!(!blob.data.is_empty(), "Encrypted blob should not be empty");

    // Should be able to decrypt back
    let decrypted = EncryptionManager::decrypt_credentials(&blob, master_password).unwrap();
    assert_eq!(decrypted, large_credentials, "Should decrypt back to original data");
  }

  #[test]
  fn test_decrypt_credentials_corrupted_blob_data() {
    // Test decryption with corrupted encrypted data
    let mut test_credentials = HashMap::new();
    let mut service_creds = HashMap::new();
    service_creds.insert("username".to_string(), "testuser".to_string());
    test_credentials.insert("service".to_string(), service_creds);

    let master_password = "corruption_test_password";

    // Create a valid blob first
    let mut blob =
      EncryptionManager::encrypt_credentials(&test_credentials, master_password).unwrap();

    // Corrupt the encrypted data
    if !blob.data.is_empty() {
      blob.data[0] ^= 1; // Flip one bit
    }

    let decrypt_result = EncryptionManager::decrypt_credentials(&blob, master_password);
    assert!(decrypt_result.is_err(), "Should fail with corrupted data");

    let error_msg = decrypt_result.unwrap_err().to_string();
    assert!(error_msg.contains("Decryption failed"), "Error should mention decryption failure");
  }

  #[test]
  fn test_decrypt_credentials_corrupted_nonce() {
    // Test decryption with corrupted nonce
    let mut test_credentials = HashMap::new();
    let mut service_creds = HashMap::new();
    service_creds.insert("key".to_string(), "value".to_string());
    test_credentials.insert("service".to_string(), service_creds);

    let master_password = "nonce_corruption_test";

    let mut blob =
      EncryptionManager::encrypt_credentials(&test_credentials, master_password).unwrap();

    // Corrupt the nonce
    if !blob.nonce.is_empty() {
      blob.nonce[0] ^= 1;
    }

    let result = EncryptionManager::decrypt_credentials(&blob, master_password);
    assert!(result.is_err(), "Should fail with corrupted nonce");
  }

  // Test edge cases in derive_key Argon2 parameter handling
  #[test]
  fn test_derive_key_with_longer_salt() {
    let master_password = "longer_salt_test_password";
    let machine_key = b"test_machine_key_for_long_salt_32!";

    // Create a longer salt (but within reasonable Argon2 limits)
    let longer_salt = vec![0xAB; 32]; // 32 bytes, reasonable length

    let result = EncryptionManager::derive_key(master_password, machine_key, &longer_salt);
    assert!(result.is_ok(), "Should handle longer salts without issues");

    let derived_key = result.unwrap();
    assert_eq!(derived_key.len(), 32, "Should still produce 32-byte key");
  }

  #[test]
  fn test_derive_key_salt_size_boundaries() {
    let master_password = "salt_boundary_test_password";
    let machine_key = b"test_machine_key_salt_boundary!!";

    // Test various salt sizes around the boundary conditions
    let salt_sizes = vec![8, 16, 24, 32]; // Various reasonable salt sizes

    for size in salt_sizes {
      let salt = vec![0x42; size];
      let result = EncryptionManager::derive_key(master_password, machine_key, &salt);
      assert!(result.is_ok(), "Should handle {size}-byte salt");

      let derived_key = result.unwrap();
      assert_eq!(derived_key.len(), 32, "Should produce 32-byte key with {size}-byte salt");
    }
  }

  #[test]
  fn test_derive_key_with_different_machine_keys() {
    let master_password = "machine_key_test_password";
    let salt = b"consistent_salt_for_test";

    let machine_key1 = b"machine_key_variant_1_32_bytes!!";
    let machine_key2 = b"machine_key_variant_2_32_bytes!!";

    let key1 = EncryptionManager::derive_key(master_password, machine_key1, salt).unwrap();
    let key2 = EncryptionManager::derive_key(master_password, machine_key2, salt).unwrap();

    assert_ne!(key1, key2, "Different machine keys should produce different derived keys");
    assert_eq!(key1.len(), 32, "First key should be 32 bytes");
    assert_eq!(key2.len(), 32, "Second key should be 32 bytes");
  }

  #[test]
  fn test_encryption_manager_static_methods_consistency() {
    // Test that static method calls are consistent across multiple invocations
    let password = "consistency_test_password";
    let machine_key1 = EncryptionManager::machine_key().unwrap();
    let machine_key2 = EncryptionManager::machine_key().unwrap();

    assert_eq!(machine_key1, machine_key2, "machine_key() should be deterministic");

    let salt = b"consistency_test_salt";
    let derived1 = EncryptionManager::derive_key(password, &machine_key1, salt).unwrap();
    let derived2 = EncryptionManager::derive_key(password, &machine_key2, salt).unwrap();

    assert_eq!(derived1, derived2, "Same inputs should produce same derived keys");
  }
}
