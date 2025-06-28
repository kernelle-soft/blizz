use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

use aes_gcm::{
  aead::{Aead, KeyInit, OsRng},
  Aes256Gcm, Key, Nonce,
};
use rand::RngCore;

pub mod encryption;

/// Trait interface for credential providers
pub trait CredentialProvider {
  fn get_credential(&self, service: &str, key: &str) -> Result<String>;
  fn store_credential(&self, service: &str, key: &str, value: &str) -> Result<()>;
}

/// Mock credential provider for testing
pub struct MockCredentialProvider {
  credentials: HashMap<String, (String, String)>,
}

impl MockCredentialProvider {
  pub fn new() -> Self {
    Self { credentials: HashMap::new() }
  }

  pub fn with_credential(mut self, service: &str, username: &str, secret: &str) -> Self {
    self.credentials.insert(service.to_string(), (username.to_string(), secret.to_string()));
    self
  }
}

impl Default for MockCredentialProvider {
  fn default() -> Self {
    Self::new()
  }
}

impl CredentialProvider for MockCredentialProvider {
  fn get_credential(&self, service: &str, key: &str) -> Result<String> {
    self
      .credentials
      .get(service)
      .and_then(|(username, secret)| {
        if key == "username" {
          Some(username.clone())
        } else if key == "token" || key == "password" || key == "secret" {
          Some(secret.clone())
        } else {
          None
        }
      })
      .ok_or_else(|| anyhow!("Credential not found: {}/{}", service, key))
  }

  fn store_credential(&self, _service: &str, _key: &str, _value: &str) -> Result<()> {
    // Mock implementation - could store in memory if needed
    Ok(())
  }
}

/// Encrypted credential store using file-based storage instead of keychain
#[derive(Debug, Serialize, Deserialize)]
struct EncryptedCredentialStore {
  credentials: HashMap<String, HashMap<String, String>>, // service -> key -> encrypted_value
}

impl EncryptedCredentialStore {
  fn new() -> Self {
    Self { credentials: HashMap::new() }
  }

  fn get_encrypted(&self, service: &str, key: &str) -> Option<&String> {
    self.credentials.get(service)?.get(key)
  }

  fn set_encrypted(&mut self, service: &str, key: &str, encrypted_value: String) {
    self
      .credentials
      .entry(service.to_string())
      .or_default()
      .insert(key.to_string(), encrypted_value);
  }

  fn load_from_file(path: &PathBuf) -> Result<Self> {
    if path.exists() {
      let content = fs::read_to_string(path)?;
      let store: EncryptedCredentialStore = serde_json::from_str(content.trim())?;
      Ok(store)
    } else {
      Ok(Self::new())
    }
  }

  fn save_to_file(&self, path: &PathBuf) -> Result<()> {
    if let Some(parent) = path.parent() {
      fs::create_dir_all(parent)?;
    }
    let content = serde_json::to_string_pretty(self)?;
    fs::write(path, content)?;
    Ok(())
  }
}

/// Crypto manager for encryption/decryption
struct CryptoManager {
  key_path: PathBuf,
}

impl CryptoManager {
  fn new() -> Self {
    let base_path = if let Ok(kernelle_dir) = std::env::var("KERNELLE_DIR") {
      std::path::PathBuf::from(kernelle_dir)
    } else {
      dirs::home_dir().unwrap_or_else(|| std::env::current_dir().unwrap()).join(".kernelle")
    };

    let mut key_path = base_path;
    key_path.push("sentinel");
    key_path.push("master.key");

    Self { key_path }
  }

  fn key_exists(&self) -> bool {
    self.key_path.exists()
  }

  fn generate_key(&self) -> Result<()> {
    bentley::info("🔐 Generating AES encryption key for secure credential storage...");

    let mut key = [0u8; 32]; // 256-bit key for AES-256
    OsRng.fill_bytes(&mut key);

    // Create directory if it doesn't exist
    if let Some(parent) = self.key_path.parent() {
      fs::create_dir_all(parent)?;
    }

    // Save key as base64
    let key_b64 = base64::encode(key);
    fs::write(&self.key_path, key_b64)?;

    // Set restrictive permissions on key file
    #[cfg(unix)]
    {
      use std::os::unix::fs::PermissionsExt;
      let mut perms = fs::metadata(&self.key_path)?.permissions();
      perms.set_mode(0o600); // Owner read/write only
      fs::set_permissions(&self.key_path, perms)?;
    }

    bentley::success("🔑 AES encryption key generated and stored securely");
    Ok(())
  }

  fn load_key(&self) -> Result<[u8; 32]> {
    let key_b64 = fs::read_to_string(&self.key_path)?;
    let key_bytes = base64::decode(key_b64.trim())?;

    if key_bytes.len() != 32 {
      return Err(anyhow!("Invalid key length"));
    }

    let mut key = [0u8; 32];
    key.copy_from_slice(&key_bytes);
    Ok(key)
  }

  fn encrypt_value(&self, value: &str) -> Result<String> {
    let key_bytes = self.load_key()?;
    let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);

    // Generate random nonce
    let mut nonce_bytes = [0u8; 12]; // 96-bit nonce for GCM
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    // Encrypt the value
    let ciphertext =
      cipher.encrypt(nonce, value.as_bytes()).map_err(|e| anyhow!("Encryption failed: {}", e))?;

    // Combine nonce + ciphertext and encode as base64
    let mut combined = nonce_bytes.to_vec();
    combined.extend_from_slice(&ciphertext);

    Ok(base64::encode(combined))
  }

  fn decrypt_value(&self, encrypted_value: &str) -> Result<String> {
    let key_bytes = self.load_key()?;
    let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);

    // Decode from base64
    let combined = base64::decode(encrypted_value)?;

    if combined.len() < 12 {
      return Err(anyhow!("Invalid encrypted data"));
    }

    // Split nonce and ciphertext
    let (nonce_bytes, ciphertext) = combined.split_at(12);
    let nonce = Nonce::from_slice(nonce_bytes);

    // Decrypt
    let plaintext =
      cipher.decrypt(nonce, ciphertext).map_err(|e| anyhow!("Decryption failed: {}", e))?;

    Ok(String::from_utf8(plaintext)?)
  }
}

/// Sentinel - The watchful guardian of secrets
///
/// Provides secure credential storage using encrypted files instead of OS keychain
pub struct Sentinel {
  #[allow(dead_code)]
  service_name: String,
  crypto: CryptoManager,
  credentials_path_override: Option<PathBuf>,
}

/// Configuration for a service that needs credentials
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceConfig {
  pub name: String,
  pub description: String,
  pub required_credentials: Vec<CredentialSpec>,
}

/// Specification for a required credential
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredentialSpec {
  pub key: String,
  pub description: String,
  pub example: Option<String>,
  pub is_required: bool,
}

/// A stored credential
#[derive(Debug, Clone)]
pub struct Credential {
  pub key: String,
  pub value: String,
}

impl CredentialProvider for Sentinel {
  fn get_credential(&self, service: &str, key: &str) -> Result<String> {
    self.get_credential_raw(service, key)
  }

  fn store_credential(&self, service: &str, key: &str, value: &str) -> Result<()> {
    self.store_credential_raw(service, key, value)
  }
}

impl Sentinel {
  /// Create a new Sentinel instance for the Kernelle toolset
  pub fn new() -> Self {
    Self {
      service_name: "kernelle".to_string(),
      crypto: CryptoManager::new(),
      credentials_path_override: None,
    }
  }

  /// Store a credential securely using encrypted file storage
  pub fn store_credential_raw(&self, service: &str, key: &str, value: &str) -> Result<()> {
    bentley::event_info(&format!("Storing credential for {service}/{key}"));

    // Ensure crypto is set up
    if !self.crypto.key_exists() {
      self.crypto.generate_key()?;
    }

    // Trim the value to remove any trailing newlines (common when copying from password managers)
    let trimmed_value = value.trim();

    // Encrypt the value
    let encrypted_value = self.crypto.encrypt_value(trimmed_value)?;

    // Load, update, and save the credential store
    let credentials_path = self.get_credentials_path();
    let mut store = EncryptedCredentialStore::load_from_file(&credentials_path)?;
    store.set_encrypted(service, key, encrypted_value);
    store.save_to_file(&credentials_path)?;

    bentley::event_success(&format!("Credential stored securely for {service}/{key}"));
    Ok(())
  }

  /// Retrieve a credential from encrypted file storage with automatic setup
  pub fn get_credential_raw(&self, service: &str, key: &str) -> Result<String> {
    // First try to get the credential directly
    if let Ok(value) = self.get_credential_inner(service, key) {
      return Ok(value);
    }

    // If not found, try to get the service config and set it up automatically
    let service_config = match service.to_lowercase().as_str() {
      "github" => Some(services::github()),
      "gitlab" => Some(services::gitlab()),
      "jira" => Some(services::jira()),
      "notion" => Some(services::notion()),
      _ => None,
    };

    if let Some(config) = service_config {
      // Check if this key is part of the service config
      if config.required_credentials.iter().any(|spec| spec.key == key) {
        bentley::info(&format!(
          "Credential {}/{} not found. Setting up {} credentials...",
          service, key, service
        ));
        self.setup_service(&config)?;
        return self.get_credential_raw(service, key);
      }
    }

    // If we can't auto-setup, return the original error
    Err(anyhow!("Credential not found for {}/{}", service, key))
  }

  /// Internal method to get credential without automatic setup
  fn get_credential_inner(&self, service: &str, key: &str) -> Result<String> {
    let credentials_path = self.get_credentials_path();
    let store = EncryptedCredentialStore::load_from_file(&credentials_path)?;

    if let Some(encrypted_value) = store.get_encrypted(service, key) {
      self.crypto.decrypt_value(encrypted_value)
    } else {
      Err(anyhow!("Credential not found for {}/{}", service, key))
    }
  }

  /// Get the path to the credentials file
  fn get_credentials_path(&self) -> PathBuf {
    if let Some(override_path) = &self.credentials_path_override {
      return override_path.clone();
    }

    let base_path = if let Ok(kernelle_dir) = std::env::var("KERNELLE_DIR") {
      std::path::PathBuf::from(kernelle_dir)
    } else {
      dirs::home_dir().unwrap_or_else(|| std::env::current_dir().unwrap()).join(".kernelle")
    };

    let mut path = base_path;
    path.push("sentinel");
    path.push("credentials.json");
    path
  }

  /// Delete a credential from encrypted file storage
  pub fn delete_credential(&self, service: &str, key: &str) -> Result<()> {
    bentley::event_info(&format!("Deleting credential for {}/{}", service, key));

    let credentials_path = self.get_credentials_path();
    let mut store = EncryptedCredentialStore::load_from_file(&credentials_path)?;

    if let Some(service_creds) = store.credentials.get_mut(service) {
      if service_creds.remove(key).is_some() {
        // Remove the service entirely if no credentials left
        if service_creds.is_empty() {
          store.credentials.remove(service);
        }
        store.save_to_file(&credentials_path)?;
        bentley::event_success(&format!("Credential deleted for {}/{}", service, key));
        Ok(())
      } else {
        Err(anyhow!("Credential not found for {}/{}", service, key))
      }
    } else {
      Err(anyhow!("Credential not found for {}/{}", service, key))
    }
  }

  /// Get all credentials for a service as environment variables
  pub fn get_service_env_vars(&self, service: &str) -> Result<HashMap<String, String>> {
    let mut env_vars = HashMap::new();

    // Try to get common credential types for the service
    let common_keys = self.get_common_keys_for_service(service);

    for key in common_keys {
      if let Ok(value) = self.get_credential(service, &key) {
        // Convert to environment variable format (uppercase with underscores)
        let env_key = format!("{}_{}", service.to_uppercase(), key.to_uppercase());
        env_vars.insert(env_key, value);
      }
    }

    Ok(env_vars)
  }

  /// Setup credentials for a service interactively
  pub fn setup_service(&self, config: &ServiceConfig) -> Result<()> {
    bentley::announce(&format!("Setting up credentials for {}", config.name));
    bentley::info(&config.description);

    for cred_spec in &config.required_credentials {
      if cred_spec.is_required || self.prompt_for_optional(&cred_spec.key)? {
        let value = self.prompt_for_credential(cred_spec)?;
        self.store_credential(&config.name, &cred_spec.key, &value)?;
      }
    }

    bentley::flourish(&format!("Credentials setup complete for {}", config.name));
    Ok(())
  }

  /// Check if all required credentials exist for a service
  pub fn verify_service_credentials(&self, config: &ServiceConfig) -> Result<Vec<String>> {
    let mut missing = Vec::new();

    for cred_spec in &config.required_credentials {
      if cred_spec.is_required && self.get_credential(&config.name, &cred_spec.key).is_err() {
        missing.push(cred_spec.key.clone());
      }
    }

    Ok(missing)
  }

  // Private helper methods

  fn get_common_keys_for_service(&self, service: &str) -> Vec<String> {
    match service.to_lowercase().as_str() {
      "github" => vec!["token".to_string()],
      "gitlab" => vec!["token".to_string()],
      "jira" => vec!["token".to_string(), "email".to_string(), "url".to_string()],
      "notion" => vec!["token".to_string()],
      _ => vec!["token".to_string()], // Default to token
    }
  }

  fn prompt_for_optional(&self, _key: &str) -> Result<bool> {
    // For now, return true - in a real implementation, this would prompt the user
    // TODO: Add interactive prompting
    Ok(true)
  }

  fn prompt_for_credential(&self, spec: &CredentialSpec) -> Result<String> {
    bentley::info(&format!("🔑 Enter {}: {}", spec.key, spec.description));

    if let Some(example) = &spec.example {
      bentley::info(&format!("Example: {}", example));
    }

    print!("> ");
    std::io::stdout().flush()?;

    let value = rpassword::read_password()?;

    if value.trim().is_empty() {
      return Err(anyhow!("{} cannot be empty", spec.key));
    }

    Ok(value.trim().to_string())
  }
}

impl Default for Sentinel {
  fn default() -> Self {
    Self::new()
  }
}

/// Predefined service configurations for common integrations
pub mod services {
  use super::*;

  pub fn github() -> ServiceConfig {
    ServiceConfig {
      name: "github".to_string(),
      description: "GitHub API access for repository and pull request management".to_string(),
      required_credentials: vec![CredentialSpec {
        key: "token".to_string(),
        description: "GitHub Personal Access Token with repo and pull request permissions"
          .to_string(),
        example: Some("ghp_xxxxxxxxxxxxxxxxxxxx".to_string()),
        is_required: true,
      }],
    }
  }

  pub fn gitlab() -> ServiceConfig {
    ServiceConfig {
      name: "gitlab".to_string(),
      description: "GitLab API access for merge request management".to_string(),
      required_credentials: vec![CredentialSpec {
        key: "token".to_string(),
        description: "GitLab Personal Access Token with API and read_repository permissions"
          .to_string(),
        example: Some("glpat-xxxxxxxxxxxxxxxxxxxx".to_string()),
        is_required: true,
      }],
    }
  }

  pub fn jira() -> ServiceConfig {
    ServiceConfig {
      name: "jira".to_string(),
      description: "Jira API access for issue tracking integration".to_string(),
      required_credentials: vec![
        CredentialSpec {
          key: "url".to_string(),
          description: "Jira instance URL".to_string(),
          example: Some("https://yourcompany.atlassian.net".to_string()),
          is_required: true,
        },
        CredentialSpec {
          key: "email".to_string(),
          description: "Your Jira account email".to_string(),
          example: Some("you@yourcompany.com".to_string()),
          is_required: true,
        },
        CredentialSpec {
          key: "token".to_string(),
          description: "Jira API token".to_string(),
          example: Some("ATATT3xFfGF0T...".to_string()),
          is_required: true,
        },
      ],
    }
  }

  pub fn notion() -> ServiceConfig {
    ServiceConfig {
      name: "notion".to_string(),
      description: "Notion API access for documentation and knowledge management".to_string(),
      required_credentials: vec![CredentialSpec {
        key: "token".to_string(),
        description: "Notion Integration Token".to_string(),
        example: Some("secret_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx".to_string()),
        is_required: true,
      }],
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  // Helper function to create a test sentinel with a unique service name
  fn create_test_sentinel() -> Sentinel {
    // Create isolated test environment for each test
    use std::time::{SystemTime, UNIX_EPOCH};
    let unique_id = format!(
      "{}_{}",
      std::process::id(),
      SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()
    );
    let temp_dir = std::env::temp_dir().join("kernelle_test").join(&unique_id);

    // Create a custom CryptoManager with isolated path instead of using env var
    let mut key_path = temp_dir.clone();
    key_path.push("sentinel");
    key_path.push("master.key");
    let crypto = CryptoManager { key_path };

    // Set up custom credentials path for isolation
    let mut credentials_path = temp_dir;
    credentials_path.push("sentinel");
    credentials_path.push("credentials.json");

    Sentinel {
      service_name: format!("test_kernelle_{}", unique_id),
      crypto,
      credentials_path_override: Some(credentials_path),
    }
  }

  #[test]
  fn test_service_configs() {
    let github_config = services::github();
    assert_eq!(github_config.name, "github");
    assert_eq!(github_config.required_credentials.len(), 1);
    assert_eq!(github_config.required_credentials[0].key, "token");
    assert!(github_config.required_credentials[0].is_required);
    assert!(github_config.required_credentials[0].example.is_some());

    let gitlab_config = services::gitlab();
    assert_eq!(gitlab_config.name, "gitlab");
    assert_eq!(gitlab_config.required_credentials.len(), 1);
    assert_eq!(gitlab_config.required_credentials[0].key, "token");

    let jira_config = services::jira();
    assert_eq!(jira_config.name, "jira");
    assert_eq!(jira_config.required_credentials.len(), 3);

    let notion_config = services::notion();
    assert_eq!(notion_config.name, "notion");
    assert_eq!(notion_config.required_credentials.len(), 1);
  }

  #[test]
  fn test_sentinel_creation() {
    let sentinel = Sentinel::new();
    assert_eq!(sentinel.service_name, "kernelle");

    let default_sentinel = Sentinel::default();
    assert_eq!(default_sentinel.service_name, "kernelle");
  }

  #[test]
  fn test_common_keys_for_service() {
    let sentinel = Sentinel::new();

    assert_eq!(sentinel.get_common_keys_for_service("github"), vec!["token"]);
    assert_eq!(sentinel.get_common_keys_for_service("gitlab"), vec!["token"]);
    assert_eq!(sentinel.get_common_keys_for_service("jira"), vec!["token", "email", "url"]);
    assert_eq!(sentinel.get_common_keys_for_service("notion"), vec!["token"]);
    assert_eq!(sentinel.get_common_keys_for_service("unknown"), vec!["token"]);
    assert_eq!(sentinel.get_common_keys_for_service("GITHUB"), vec!["token"]);
  }

  #[test]
  fn test_credential_storage_and_retrieval() {
    let sentinel = create_test_sentinel();
    let service = "test_service";
    let key = "test_key";
    let value = "test_secret_value";

    // Store credential
    let store_result = sentinel.store_credential(service, key, value);
    assert!(store_result.is_ok(), "Failed to store credential: {:?}", store_result.err());

    // Retrieve credential
    let retrieved = sentinel.get_credential(service, key);
    assert!(retrieved.is_ok(), "Failed to retrieve credential: {:?}", retrieved.err());
    assert_eq!(retrieved.unwrap(), value);

    // Clean up
    let _ = sentinel.delete_credential(service, key);
  }

  #[test]
  fn test_credential_retrieval_nonexistent() {
    let sentinel = create_test_sentinel();
    let result = sentinel.get_credential("nonexistent_service", "nonexistent_key");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not found"));
  }

  #[test]
  fn test_credential_deletion() {
    let sentinel = create_test_sentinel();
    let service = "test_delete_service";
    let key = "test_delete_key";
    let value = "test_delete_value";

    // Store then delete
    sentinel.store_credential(service, key, value).unwrap();
    let delete_result = sentinel.delete_credential(service, key);
    assert!(delete_result.is_ok());

    // Verify deletion
    let retrieve_result = sentinel.get_credential(service, key);
    assert!(retrieve_result.is_err());
  }

  #[test]
  fn test_credential_overwrite() {
    let sentinel = create_test_sentinel();
    let service = "test_overwrite_service";
    let key = "test_overwrite_key";
    let value1 = "original_value";
    let value2 = "updated_value";

    // Store original value
    sentinel.store_credential(service, key, value1).unwrap();
    assert_eq!(sentinel.get_credential(service, key).unwrap(), value1);

    // Overwrite with new value
    sentinel.store_credential(service, key, value2).unwrap();
    assert_eq!(sentinel.get_credential(service, key).unwrap(), value2);

    // Clean up
    let _ = sentinel.delete_credential(service, key);
  }

  #[test]
  fn test_multiple_credentials_same_service() {
    let sentinel = create_test_sentinel();
    let service = "multi_cred_service";

    // Store multiple credentials for same service
    sentinel.store_credential(service, "key1", "value1").unwrap();
    sentinel.store_credential(service, "key2", "value2").unwrap();
    sentinel.store_credential(service, "key3", "value3").unwrap();

    // Verify all can be retrieved independently
    assert_eq!(sentinel.get_credential(service, "key1").unwrap(), "value1");
    assert_eq!(sentinel.get_credential(service, "key2").unwrap(), "value2");
    assert_eq!(sentinel.get_credential(service, "key3").unwrap(), "value3");

    // Clean up
    let _ = sentinel.delete_credential(service, "key1");
    let _ = sentinel.delete_credential(service, "key2");
    let _ = sentinel.delete_credential(service, "key3");
  }

  #[test]
  fn test_get_service_env_vars() {
    let sentinel = create_test_sentinel();
    let service = "env_test_service";

    // Test with no credentials stored
    let env_vars = sentinel.get_service_env_vars(service).unwrap();
    assert!(env_vars.is_empty());

    // Store a credential and test env var generation
    sentinel.store_credential(service, "token", "test_token_123").unwrap();
    let env_vars = sentinel.get_service_env_vars(service).unwrap();

    let expected_key = format!("{}_TOKEN", service.to_uppercase());
    assert!(env_vars.contains_key(&expected_key));
    assert_eq!(env_vars.get(&expected_key).unwrap(), "test_token_123");

    // Clean up
    let _ = sentinel.delete_credential(service, "token");
  }

  #[test]
  fn test_get_service_env_vars_github() {
    let sentinel = create_test_sentinel();
    let service = "github";

    // Store GitHub token
    sentinel.store_credential(service, "token", "ghp_test_token").unwrap();
    let env_vars = sentinel.get_service_env_vars(service).unwrap();

    assert_eq!(env_vars.get("GITHUB_TOKEN").unwrap(), "ghp_test_token");

    // Clean up
    let _ = sentinel.delete_credential(service, "token");
  }

  #[test]
  fn test_get_service_env_vars_jira() {
    let sentinel = create_test_sentinel();
    let service = "jira";

    // Store Jira credentials
    sentinel.store_credential(service, "token", "jira_token").unwrap();
    sentinel.store_credential(service, "email", "test@example.com").unwrap();
    sentinel.store_credential(service, "url", "https://test.atlassian.net").unwrap();

    let env_vars = sentinel.get_service_env_vars(service).unwrap();

    assert_eq!(env_vars.get("JIRA_TOKEN").unwrap(), "jira_token");
    assert_eq!(env_vars.get("JIRA_EMAIL").unwrap(), "test@example.com");
    assert_eq!(env_vars.get("JIRA_URL").unwrap(), "https://test.atlassian.net");

    // Clean up
    let _ = sentinel.delete_credential(service, "token");
    let _ = sentinel.delete_credential(service, "email");
    let _ = sentinel.delete_credential(service, "url");
  }

  #[test]
  #[ignore = "Prompts for user input - hangs in test environment"]
  fn test_setup_service() {
    let sentinel = create_test_sentinel();
    let config = ServiceConfig {
      name: "test_setup_service".to_string(),
      description: "Test service for setup".to_string(),
      required_credentials: vec![CredentialSpec {
        key: "test_key".to_string(),
        description: "Test credential".to_string(),
        example: Some("test_example".to_string()),
        is_required: true,
      }],
    };

    // Note: This test will use placeholder values from prompt_for_credential
    let result = sentinel.setup_service(&config);
    assert!(result.is_ok());

    // Verify credential was stored (with placeholder value)
    let stored = sentinel.get_credential("test_setup_service", "test_key");
    assert!(stored.is_ok());
    assert_eq!(stored.unwrap(), "placeholder_credential");

    // Clean up
    let _ = sentinel.delete_credential("test_setup_service", "test_key");
  }

  #[test]
  fn test_prompt_for_optional() {
    let sentinel = create_test_sentinel();
    // Test the current implementation which always returns true
    let result = sentinel.prompt_for_optional("test_key");
    assert!(result.is_ok());
    assert!(result.unwrap());
  }

  #[test]
  #[ignore = "Prompts for user input - hangs in test environment"]
  fn test_prompt_for_credential() {
    let sentinel = create_test_sentinel();
    let spec = CredentialSpec {
      key: "test_key".to_string(),
      description: "Test description".to_string(),
      example: Some("test_example".to_string()),
      is_required: true,
    };

    let result = sentinel.prompt_for_credential(&spec);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "placeholder_credential");

    // Test without example
    let spec_no_example = CredentialSpec {
      key: "test_key_no_example".to_string(),
      description: "Test description".to_string(),
      example: None,
      is_required: true,
    };

    let result = sentinel.prompt_for_credential(&spec_no_example);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "placeholder_credential");
  }

  #[test]
  fn test_credential_spec_creation() {
    let spec = CredentialSpec {
      key: "test_key".to_string(),
      description: "Test description".to_string(),
      example: Some("example_value".to_string()),
      is_required: true,
    };

    assert_eq!(spec.key, "test_key");
    assert_eq!(spec.description, "Test description");
    assert_eq!(spec.example, Some("example_value".to_string()));
    assert!(spec.is_required);
  }

  #[test]
  fn test_credential_creation() {
    let cred = Credential { key: "test_key".to_string(), value: "test_value".to_string() };

    assert_eq!(cred.key, "test_key");
    assert_eq!(cred.value, "test_value");
  }

  #[test]
  fn test_service_config_creation() {
    let config = ServiceConfig {
      name: "test_service".to_string(),
      description: "Test service description".to_string(),
      required_credentials: vec![CredentialSpec {
        key: "key1".to_string(),
        description: "Key 1".to_string(),
        example: None,
        is_required: true,
      }],
    };

    assert_eq!(config.name, "test_service");
    assert_eq!(config.description, "Test service description");
    assert_eq!(config.required_credentials.len(), 1);
  }

  #[test]
  fn test_edge_cases() {
    let sentinel = create_test_sentinel();

    // Test with empty strings
    let result = sentinel.store_credential("", "", "");
    assert!(result.is_ok());

    let retrieved = sentinel.get_credential("", "");
    assert!(retrieved.is_ok());
    assert_eq!(retrieved.unwrap(), "");

    // Clean up
    let _ = sentinel.delete_credential("", "");

    // Test with special characters
    let service = "test@service#with$special%chars";
    let key = "key&with*special(chars)";
    let value = "value with spaces and symbols!@#$%^&*()";

    sentinel.store_credential(service, key, value).unwrap();
    let retrieved = sentinel.get_credential(service, key).unwrap();
    assert_eq!(retrieved, value);

    // Clean up
    let _ = sentinel.delete_credential(service, key);
  }

  #[test]
  fn test_delete_nonexistent_credential() {
    let sentinel = create_test_sentinel();

    // Try to delete a credential that doesn't exist
    let result = sentinel.delete_credential("nonexistent", "key");
    assert!(result.is_err());
  }

  #[test]
  #[ignore = "Prompts for user input - hangs in test environment"]
  fn test_setup_service_with_optional_credentials() {
    let sentinel = Sentinel::new();

    // Create a config with optional credentials
    let config = ServiceConfig {
      name: "test_service".to_string(),
      description: "Test service with optional credentials".to_string(),
      required_credentials: vec![
        CredentialSpec {
          key: "required_token".to_string(),
          description: "Required token".to_string(),
          example: Some("req_token_123".to_string()),
          is_required: true,
        },
        CredentialSpec {
          key: "optional_key".to_string(),
          description: "Optional key".to_string(),
          example: Some("opt_key_456".to_string()),
          is_required: false,
        },
      ],
    };

    // Clean up any existing credentials
    let _ = sentinel.delete_credential(&config.name, "required_token");
    let _ = sentinel.delete_credential(&config.name, "optional_key");

    // Test setup service - this will call prompt_for_optional and store_credential
    let result = sentinel.setup_service(&config);
    assert!(result.is_ok());

    // Verify the required credential was stored
    let required_cred = sentinel.get_credential(&config.name, "required_token");
    assert!(required_cred.is_ok());
    assert_eq!(required_cred.unwrap(), "placeholder_credential");

    // Verify the optional credential was also stored (since prompt_for_optional returns true)
    let optional_cred = sentinel.get_credential(&config.name, "optional_key");
    assert!(optional_cred.is_ok());
    assert_eq!(optional_cred.unwrap(), "placeholder_credential");

    // Clean up
    let _ = sentinel.delete_credential(&config.name, "required_token");
    let _ = sentinel.delete_credential(&config.name, "optional_key");
  }

  #[test]
  #[ignore = "Prompts for user input - hangs in test environment"]
  fn test_prompt_for_credential_with_example() {
    let sentinel = Sentinel::new();

    // Test credential spec with example
    let spec_with_example = CredentialSpec {
      key: "test_key".to_string(),
      description: "Test credential".to_string(),
      example: Some("example_value_123".to_string()),
      is_required: true,
    };

    let result = sentinel.prompt_for_credential(&spec_with_example);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "placeholder_credential");

    // Test credential spec without example
    let spec_without_example = CredentialSpec {
      key: "test_key_no_example".to_string(),
      description: "Test credential without example".to_string(),
      example: None,
      is_required: true,
    };

    let result = sentinel.prompt_for_credential(&spec_without_example);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "placeholder_credential");
  }

  #[test]
  #[ignore = "Prompts for user input - hangs in test environment"]
  fn test_setup_service_with_mixed_optional_credentials() {
    let sentinel = Sentinel::new();

    // Create a mock config with both required and optional credentials
    let config = ServiceConfig {
      name: "test_mixed_service".to_string(),
      description: "Test service with mixed credentials".to_string(),
      required_credentials: vec![
        CredentialSpec {
          key: "required_token".to_string(),
          description: "Required token".to_string(),
          example: Some("req_token_123".to_string()),
          is_required: true,
        },
        CredentialSpec {
          key: "optional_key".to_string(),
          description: "Optional key".to_string(),
          example: Some("opt_key_456".to_string()),
          is_required: false,
        },
      ],
    };

    // Clean up any existing credentials
    let _ = sentinel.delete_credential(&config.name, "required_token");
    let _ = sentinel.delete_credential(&config.name, "optional_key");

    // Test setup service - this will exercise both branches of the conditional
    let result = sentinel.setup_service(&config);
    assert!(result.is_ok());

    // Verify the required credential was stored (line 109 coverage)
    let required_cred = sentinel.get_credential(&config.name, "required_token");
    assert!(required_cred.is_ok());
    assert_eq!(required_cred.unwrap(), "placeholder_credential");

    // Verify the optional credential was also stored since prompt_for_optional returns true
    let optional_cred = sentinel.get_credential(&config.name, "optional_key");
    assert!(optional_cred.is_ok());
    assert_eq!(optional_cred.unwrap(), "placeholder_credential");

    // Clean up
    let _ = sentinel.delete_credential(&config.name, "required_token");
    let _ = sentinel.delete_credential(&config.name, "optional_key");
  }

  #[test]
  fn test_store_credential_success_logging() {
    let sentinel = create_test_sentinel();

    // Use a unique service and key to avoid conflicts with other tests
    let test_service = format!("test_logging_service_{}", std::process::id());
    let test_key = format!("test_logging_key_{}", std::process::id());

    // Clean up any existing credential first
    let _ = sentinel.delete_credential(&test_service, &test_key);

    // This test specifically targets line 53 - the bentley::event_success call
    let result = sentinel.store_credential(&test_service, &test_key, "test_value");
    assert!(result.is_ok(), "Failed to store credential: {:?}", result.err());

    // Verify the credential was actually stored
    let retrieved = sentinel.get_credential(&test_service, &test_key);
    assert!(retrieved.is_ok(), "Failed to retrieve credential: {:?}", retrieved.err());
    assert_eq!(retrieved.unwrap(), "test_value");

    // Clean up
    let _ = sentinel.delete_credential(&test_service, &test_key);
  }
}
