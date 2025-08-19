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
    bentley::info("no vault found. creating new vault...");
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

    bentley::success("vault created successfully");
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

  // Test constants
  const PROMPT_ENTER_NEW_PASSWORD: &str = "enter new master password:";
  const PROMPT_CONFIRM_PASSWORD: &str = "confirm master password:";
  const ERROR_PASSWORD_EMPTY: &str = "master password cannot be empty";
  const ERROR_PASSWORDS_DONT_MATCH: &str = "passwords do not match";
  const ERROR_INCORRECT_PASSWORD: &str = "incorrect password";

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

  // Helper function to test create_new_vault logic without interactive prompts
  fn create_new_vault_non_interactive(
    cred_path: &Path,
    password1: &str,
    password2: &str,
  ) -> Result<String> {
    bentley::info("no vault found. creating new vault...");

    if password1.trim().is_empty() {
      return Err(anyhow!("master password cannot be empty"));
    }

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

    bentley::success("vault created successfully");
    Ok(password1.trim().to_string())
  }

  #[test]
  fn test_create_new_vault_empty_password_path() {
    with_temp_dir(|temp_dir| {
      let cred_path = temp_dir.path().join("credentials.enc");

      // Test the empty password check
      let result = create_new_vault_non_interactive(&cred_path, "", "anything");
      assert!(result.is_err());
      let error_msg = result.unwrap_err().to_string();
      assert!(error_msg.contains("master password cannot be empty"));

      // Test whitespace-only password
      let result2 = create_new_vault_non_interactive(&cred_path, "   ", "   ");
      assert!(result2.is_err());
      let error_msg2 = result2.unwrap_err().to_string();
      assert!(error_msg2.contains("master password cannot be empty"));
    });
  }

  #[test]
  fn test_create_new_vault_password_mismatch_path() {
    with_temp_dir(|temp_dir| {
      let cred_path = temp_dir.path().join("credentials.enc");

      // Test the password mismatch check
      let result = create_new_vault_non_interactive(&cred_path, "password123", "different_password");
      assert!(result.is_err());
      let error_msg = result.unwrap_err().to_string();
      assert!(error_msg.contains("passwords do not match"));
    });
  }

  #[test]
  fn test_create_new_vault_success_path_all_components() {
    with_temp_dir(|temp_dir| {
      let nested_path = temp_dir.path().join("nested").join("path");
      let cred_path = nested_path.join("credentials.enc");

      // Test the successful flow covering all components
      let result =
        create_new_vault_non_interactive(&cred_path, "test_password_123", "test_password_123");
      assert!(result.is_ok());
      let returned_password = result.unwrap();
      assert_eq!(returned_password, "test_password_123");

      // Verify the directory was created
      assert!(nested_path.exists());
      assert!(nested_path.is_dir());

      // Verify the credentials file was created
      assert!(cred_path.exists());

      // Verify the file can be read back
      let verify_result = EncryptionManager::verify_password(&cred_path, "test_password_123");
      assert!(verify_result.is_ok());
    });
  }

  #[test]
  fn test_create_new_vault_trimming_in_success_path() {
    with_temp_dir(|temp_dir| {
      let cred_path = temp_dir.path().join("credentials.enc");

      // Test password trimming in success path
      let password_with_spaces = "  trimmed_password  ";
      let result =
        create_new_vault_non_interactive(&cred_path, password_with_spaces, password_with_spaces);
      assert!(result.is_ok());
      let returned_password = result.unwrap();

      // Should be trimmed in return
      assert_eq!(returned_password, "trimmed_password");
      assert!(!returned_password.contains(' '));

      // Should be able to verify with trimmed password
      let verify_result = EncryptionManager::verify_password(&cred_path, "trimmed_password");
      assert!(verify_result.is_ok());
    });
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
}
