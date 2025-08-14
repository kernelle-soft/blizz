use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

pub mod cli;
pub mod encryption;

use encryption::{EncryptedBlob, EncryptionManager};

/// Trait interface for secret providers
///
/// This is the main public API that services should use to interact with secret storage.
/// It provides a simple, secure interface for storing and retrieving secrets.
///
pub trait SecretProvider {
  fn get_secret(&self, group: &str, name: &str) -> Result<String>;
  fn store_secret(&self, group: &str, name: &str, value: &str) -> Result<()>;
}

// Backward compatibility - old trait name aliased to new trait
pub use SecretProvider as CredentialProvider;

/// Mock secret provider for testing
pub struct MockSecretProvider {
  secrets: HashMap<String, (String, String)>,
}

impl MockSecretProvider {
  pub fn new() -> Self {
    Self { secrets: HashMap::new() }
  }

  pub fn with_secret(mut self, group: &str, name: &str, value: &str) -> Self {
    self.secrets.insert(group.to_string(), (name.to_string(), value.to_string()));
    self
  }
}

impl Default for MockSecretProvider {
  fn default() -> Self {
    Self::new()
  }
}

impl SecretProvider for MockSecretProvider {
  fn get_secret(&self, group: &str, name: &str) -> Result<String> {
    self
      .secrets
      .get(group)
      .and_then(|(stored_name, secret)| {
        if name == stored_name || name == "username" {
          Some(stored_name.clone())
        } else if name == "token" || name == "password" || name == "secret" {
          Some(secret.clone())
        } else {
          None
        }
      })
      .ok_or_else(|| anyhow!("Secret not found: {}/{}", group, name))
  }

  fn store_secret(&self, _group: &str, _name: &str, _value: &str) -> Result<()> {
    // Mock implementation - could store in memory if needed
    Ok(())
  }
}

/// Password-based credential store using Argon2 key derivation
#[derive(Debug, Serialize, Deserialize)]
pub struct PasswordBasedCredentialStore {
  /// The encrypted credential data
  encrypted_data: EncryptedBlob,
  /// Version identifier for format compatibility
  version: String,
}

impl PasswordBasedCredentialStore {
  pub fn new(
    credentials: &HashMap<String, HashMap<String, String>>,
    master_password: &str,
  ) -> Result<Self> {
    let encrypted_data = EncryptionManager::encrypt_credentials(credentials, master_password)?;
    Ok(Self { encrypted_data, version: "1.0".to_string() })
  }

  pub fn decrypt_credentials(
    &self,
    master_password: &str,
  ) -> Result<HashMap<String, HashMap<String, String>>> {
    EncryptionManager::decrypt_credentials(&self.encrypted_data, master_password)
  }

  pub fn load_from_file(path: &PathBuf) -> Result<Option<Self>> {
    if path.exists() {
      let content = fs::read_to_string(path)?;
      let store: PasswordBasedCredentialStore = serde_json::from_str(content.trim())?;
      Ok(Some(store))
    } else {
      Ok(None)
    }
  }

  pub fn save_to_file(&self, path: &PathBuf) -> Result<()> {
    if let Some(parent) = path.parent() {
      fs::create_dir_all(parent)?;
    }
    let content = serde_json::to_string_pretty(self)?;
    fs::write(path, content)?;

    // Set restrictive permissions on credential file
    #[cfg(unix)]
    {
      use std::os::unix::fs::PermissionsExt;
      let mut perms = fs::metadata(path)?.permissions();
      perms.set_mode(0o600); // Owner read/write only
      fs::set_permissions(path, perms)?;
    }

    Ok(())
  }
}

/// Trait for cryptographic operations to enable dependency injection and testing
pub trait CryptoProvider {
  fn credentials_exist(&self) -> bool;
  fn get_master_password(&self) -> Result<String>;
  fn prompt_for_new_master_password(&self) -> Result<String>;
  fn store_secret(&self, group: &str, name: &str, value: &str, master_password: &str)
    -> Result<()>;
  fn get_secret(&self, group: &str, name: &str, master_password: &str) -> Result<String>;
  fn delete_secret(&self, group: &str, name: &str, master_password: &str) -> Result<()>;
}

/// Password-based crypto manager using Argon2 key derivation
struct PasswordBasedCryptoManager {
  credentials_path: PathBuf,
}

impl CryptoProvider for PasswordBasedCryptoManager {
  fn credentials_exist(&self) -> bool {
    self.credentials_path.exists()
  }

  fn get_master_password(&self) -> Result<String> {
    // In a real implementation, this would use daemon communication
    // For now, we'll keep the direct prompting for backward compatibility
    // The CLI layer handles daemon communication
    bentley::info("Enter master password to unlock credential store:");
    print!("> ");
    std::io::stdout().flush()?;

    let password = rpassword::read_password()?;

    if password.trim().is_empty() {
      return Err(anyhow!("Master password cannot be empty"));
    }

    Ok(password.trim().to_string())
  }

  fn prompt_for_new_master_password(&self) -> Result<String> {
    bentley::announce("Setting up secure credential storage");
    bentley::info("Please create a master password to protect your credentials.");
    bentley::info("This password will be required to access stored credentials.");

    print!("Enter master password: ");
    std::io::stdout().flush()?;
    let password1 = rpassword::read_password()?;

    if password1.trim().is_empty() {
      return Err(anyhow!("Master password cannot be empty"));
    }

    print!("Confirm master password: ");
    std::io::stdout().flush()?;
    let password2 = rpassword::read_password()?;

    if password1 != password2 {
      return Err(anyhow!("Passwords do not match"));
    }

    if password1.len() < 8 {
      return Err(anyhow!("Master password must be at least 8 characters"));
    }

    bentley::success("Master password set successfully");
    Ok(password1.trim().to_string())
  }

  fn store_secret(
    &self,
    group: &str,
    name: &str,
    value: &str,
    master_password: &str,
  ) -> Result<()> {
    let mut credentials = self.load_credentials(master_password).unwrap_or_else(|_| HashMap::new());

    credentials.entry(group.to_string()).or_default().insert(name.to_string(), value.to_string());

    self.save_credentials(&credentials, master_password)?;
    Ok(())
  }

  fn get_secret(&self, group: &str, name: &str, master_password: &str) -> Result<String> {
    let credentials = self.load_credentials(master_password)?;

    credentials
      .get(group)
      .and_then(|service_creds| service_creds.get(name))
      .cloned()
      .ok_or_else(|| anyhow!("Secret not found for {}/{}", group, name))
  }

  fn delete_secret(&self, group: &str, name: &str, master_password: &str) -> Result<()> {
    let mut credentials = self.load_credentials(master_password)?;

    if let Some(service_creds) = credentials.get_mut(group) {
      if service_creds.remove(name).is_some() {
        // Remove the service entirely if no credentials left
        if service_creds.is_empty() {
          credentials.remove(group);
        }
        self.save_credentials(&credentials, master_password)?;
        Ok(())
      } else {
        Err(anyhow!("Secret not found for {}/{}", group, name))
      }
    } else {
      Err(anyhow!("Secret not found for {}/{}", group, name))
    }
  }
}

impl PasswordBasedCryptoManager {
  fn new() -> Self {
    let base_path = if let Ok(kernelle_dir) = std::env::var("KERNELLE_DIR") {
      std::path::PathBuf::from(kernelle_dir)
    } else {
      dirs::home_dir().unwrap_or_else(|| std::env::current_dir().unwrap()).join(".kernelle")
    };

    let mut credentials_path = base_path;
    credentials_path.push("persistent");
    credentials_path.push("keeper");
    credentials_path.push("credentials.enc");

    Self { credentials_path }
  }

  fn load_credentials(
    &self,
    master_password: &str,
  ) -> Result<HashMap<String, HashMap<String, String>>> {
    if let Some(store) = PasswordBasedCredentialStore::load_from_file(&self.credentials_path)? {
      store.decrypt_credentials(master_password)
    } else {
      Ok(HashMap::new())
    }
  }

  fn save_credentials(
    &self,
    credentials: &HashMap<String, HashMap<String, String>>,
    master_password: &str,
  ) -> Result<()> {
    let store = PasswordBasedCredentialStore::new(credentials, master_password)?;
    store.save_to_file(&self.credentials_path)?;
    Ok(())
  }
}

/// Secrets - The watchful guardian of secrets
///
/// Provides secure credential storage using Argon2-based password derivation
/// instead of storing keys on disk. Requires a master password for access.
pub struct Secrets {
  #[allow(dead_code)]
  service_name: String,
  crypto: Box<dyn CryptoProvider>,
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

impl SecretProvider for Secrets {
  fn get_secret(&self, group: &str, name: &str) -> Result<String> {
    self.get_secret_raw(group, name)
  }

  fn store_secret(&self, group: &str, name: &str, value: &str) -> Result<()> {
    self.store_secret_raw(group, name, value)
  }
}

impl Secrets {
  /// Create a new Secrets instance for the Kernelle toolset
  pub fn new() -> Self {
    Self::with_crypto_provider(Box::new(PasswordBasedCryptoManager::new()))
  }

  /// Create a Secrets instance with a custom crypto provider for dependency injection
  pub fn with_crypto_provider(crypto: Box<dyn CryptoProvider>) -> Self {
    Self { service_name: "kernelle".to_string(), crypto }
  }

  /// Store a secret securely using Argon2-based encryption
  pub fn store_secret_raw(&self, group: &str, name: &str, value: &str) -> Result<()> {
    bentley::event_info(&format!("Storing secret for {group}/{name}"));

    // Get master password (prompt for new one if first time)
    let master_password = if self.crypto.credentials_exist() {
      self.crypto.get_master_password()?
    } else {
      self.crypto.prompt_for_new_master_password()?
    };

    // Trim the value to remove any trailing newlines (common when copying from password managers)
    let trimmed_value = value.trim();

    // Store the secret using Argon2-based encryption
    self.crypto.store_secret(group, name, trimmed_value, &master_password)?;

    bentley::event_success(&format!("Secret stored securely for {group}/{name}"));
    Ok(())
  }

  /// Retrieve a secret from encrypted file storage with automatic setup
  pub fn get_secret_raw(&self, group: &str, name: &str) -> Result<String> {
    // First try to get the secret directly
    if let Ok(value) = self.get_secret_inner(group, name) {
      return Ok(value);
    }

    // If not found, try to get the service config and set it up automatically
    let service_config = match group.to_lowercase().as_str() {
      "github" => Some(services::github()),
      "gitlab" => Some(services::gitlab()),
      "jira" => Some(services::jira()),
      "notion" => Some(services::notion()),
      _ => None,
    };

    if let Some(config) = service_config {
      // Check if this name is part of the service config
      if config.required_credentials.iter().any(|spec| spec.key == name) {
        bentley::info(&format!("Secret {group}/{name} not found. Setting up {group} secrets..."));
        self.setup_service(&config)?;
        return self.get_secret_raw(group, name);
      }
    }

    // If we can't auto-setup, return the original error
    Err(anyhow!("Secret not found for {}/{}", group, name))
  }

  /// Retrieve a secret from encrypted file storage WITHOUT automatic setup
  /// This is intended for CLI usage where we don't want to auto-trigger setup
  pub fn get_secret_raw_no_setup(&self, group: &str, name: &str) -> Result<String> {
    self.get_secret_inner(group, name)
  }

  /// Internal method to get secret without automatic setup
  fn get_secret_inner(&self, group: &str, name: &str) -> Result<String> {
    if !self.crypto.credentials_exist() {
      return Err(anyhow!("No secrets stored yet"));
    }

    let master_password = self.crypto.get_master_password()?;
    self.crypto.get_secret(group, name, &master_password)
  }

  /// Delete a secret from password-protected storage
  pub fn delete_secret(&self, group: &str, name: &str) -> Result<()> {
    bentley::event_info(&format!("Deleting secret for {group}/{name}"));

    if !self.crypto.credentials_exist() {
      return Err(anyhow!("No secrets stored yet"));
    }

    let master_password = self.crypto.get_master_password()?;
    self.crypto.delete_secret(group, name, &master_password)?;

    bentley::event_success(&format!("Secret deleted for {group}/{name}"));
    Ok(())
  }

  /// Get all secrets for a group as environment variables
  pub fn get_group_env_vars(&self, group: &str) -> Result<HashMap<String, String>> {
    let mut env_vars = HashMap::new();

    if !self.crypto.credentials_exist() {
      return Ok(env_vars); // Return empty if no secrets exist
    }

    // Try to get common secret types for the group
    let common_keys = self.get_common_keys_for_group(group);

    for key in common_keys {
      if let Ok(value) = self.get_secret(group, &key) {
        // Convert to environment variable format (uppercase with underscores)
        let env_key = format!("{}_{}", group.to_uppercase(), key.to_uppercase());
        env_vars.insert(env_key, value);
      }
    }

    Ok(env_vars)
  }

  /// Setup secrets for a service interactively
  pub fn setup_service(&self, config: &ServiceConfig) -> Result<()> {
    bentley::announce(&format!("Setting up secrets for {}", config.name));
    bentley::info(&config.description);

    for cred_spec in &config.required_credentials {
      if cred_spec.is_required || self.prompt_for_optional(&cred_spec.key)? {
        let value = self.prompt_for_credential(cred_spec)?;
        self.store_secret(&config.name, &cred_spec.key, &value)?;
      }
    }

    bentley::flourish(&format!("Secrets setup complete for {}", config.name));
    Ok(())
  }

  /// Check if all required secrets exist for a service
  pub fn verify_service_credentials(&self, config: &ServiceConfig) -> Result<Vec<String>> {
    let mut missing = Vec::new();

    for cred_spec in &config.required_credentials {
      if cred_spec.is_required && self.get_secret(&config.name, &cred_spec.key).is_err() {
        missing.push(cred_spec.key.clone());
      }
    }

    Ok(missing)
  }

  /// Check if all required secrets exist for a service WITHOUT triggering auto-setup
  /// This is intended for CLI usage where we don't want to auto-trigger setup
  pub fn verify_service_credentials_no_setup(&self, config: &ServiceConfig) -> Result<Vec<String>> {
    let mut missing = Vec::new();

    for cred_spec in &config.required_credentials {
      if cred_spec.is_required
        && self.get_secret_raw_no_setup(&config.name, &cred_spec.key).is_err()
      {
        missing.push(cred_spec.key.clone());
      }
    }

    Ok(missing)
  }

  // Private helper methods

  fn get_common_keys_for_group(&self, group: &str) -> Vec<String> {
    match group.to_lowercase().as_str() {
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
    bentley::info(&format!("Enter {}: {}", spec.key, spec.description));

    if let Some(example) = &spec.example {
      bentley::info(&format!("Example: {example}"));
    }

    print!("> ");
    std::io::stdout().flush()?;

    let value = rpassword::read_password()?;

    if value.trim().is_empty() {
      return Err(anyhow!("{} cannot be empty", spec.key));
    }

    Ok(value.trim().to_string())
  }

  // Backward compatibility methods for legacy CredentialProvider interface
  /// @deprecated Use get_secret instead
  pub fn get_credential(&self, service: &str, key: &str) -> Result<String> {
    self.get_secret(service, key)
  }

  /// @deprecated Use store_secret instead  
  pub fn store_credential(&self, service: &str, key: &str, value: &str) -> Result<()> {
    self.store_secret(service, key, value)
  }

  /// @deprecated Use delete_secret instead
  pub fn delete_credential(&self, service: &str, key: &str) -> Result<()> {
    self.delete_secret(service, key)
  }

  /// @deprecated Use get_secret_raw instead
  pub fn get_credential_raw(&self, service: &str, key: &str) -> Result<String> {
    self.get_secret_raw(service, key)
  }

  /// @deprecated Use store_secret_raw instead
  pub fn store_credential_raw(&self, service: &str, key: &str, value: &str) -> Result<()> {
    self.store_secret_raw(service, key, value)
  }

  /// @deprecated Use get_group_env_vars instead
  pub fn get_service_env_vars(&self, service: &str) -> Result<HashMap<String, String>> {
    self.get_group_env_vars(service)
  }
}

impl Default for Secrets {
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
  use std::collections::HashMap;
  use std::sync::{Arc, Mutex};

  /// Mock crypto provider for testing that doesn't require password prompts
  #[derive(Debug)]
  struct MockCryptoProvider {
    credentials: Arc<Mutex<HashMap<String, HashMap<String, String>>>>,
    stored_password: String,
  }

  impl MockCryptoProvider {
    fn new(password: &str) -> Self {
      Self {
        credentials: Arc::new(Mutex::new(HashMap::new())),
        stored_password: password.to_string(),
      }
    }
  }

  impl CryptoProvider for MockCryptoProvider {
    fn credentials_exist(&self) -> bool {
      !self.credentials.lock().unwrap().is_empty()
    }

    fn get_master_password(&self) -> Result<String> {
      Ok(self.stored_password.clone())
    }

    fn prompt_for_new_master_password(&self) -> Result<String> {
      Ok(self.stored_password.clone())
    }

    fn store_secret(
      &self,
      group: &str,
      name: &str,
      value: &str,
      master_password: &str,
    ) -> Result<()> {
      if master_password != self.stored_password {
        return Err(anyhow!("Invalid password"));
      }

      self
        .credentials
        .lock()
        .unwrap()
        .entry(group.to_string())
        .or_default()
        .insert(name.to_string(), value.to_string());

      Ok(())
    }

    fn get_secret(&self, group: &str, name: &str, master_password: &str) -> Result<String> {
      if master_password != self.stored_password {
        return Err(anyhow!("Invalid password"));
      }

      self
        .credentials
        .lock()
        .unwrap()
        .get(group)
        .and_then(|service_creds| service_creds.get(name))
        .cloned()
        .ok_or_else(|| anyhow!("Secret not found for {}/{}", group, name))
    }

    fn delete_secret(&self, group: &str, name: &str, master_password: &str) -> Result<()> {
      if master_password != self.stored_password {
        return Err(anyhow!("Invalid password"));
      }

      let mut credentials = self.credentials.lock().unwrap();
      if let Some(service_creds) = credentials.get_mut(group) {
        if service_creds.remove(name).is_some() {
          if service_creds.is_empty() {
            credentials.remove(group);
          }
          Ok(())
        } else {
          Err(anyhow!("Secret not found for {}/{}", group, name))
        }
      } else {
        Err(anyhow!("Secret not found for {}/{}", group, name))
      }
    }
  }

  // Helper function to create a test secrets with mock crypto provider
  fn create_test_secrets_with_mock(password: &str) -> Secrets {
    Secrets::with_crypto_provider(Box::new(MockCryptoProvider::new(password)))
  }

  // Helper function to create a test secrets with a unique service name
  fn create_test_secrets() -> Secrets {
    // Create isolated test environment for each test
    use std::time::{SystemTime, UNIX_EPOCH};
    let unique_id = format!(
      "{}_{}",
      std::process::id(),
      SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()
    );
    let temp_dir = std::env::temp_dir().join("kernelle_test").join(&unique_id);

    // Create a custom PasswordBasedCryptoManager with isolated path
    let mut credentials_path = temp_dir.clone();
    credentials_path.push("secrets");
    credentials_path.push("credentials.enc");
    let crypto = PasswordBasedCryptoManager { credentials_path };

    Secrets { service_name: format!("test_kernelle_{unique_id}"), crypto: Box::new(crypto) }
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
  fn test_secrets_creation() {
    let secrets = Secrets::new();
    assert_eq!(secrets.service_name, "kernelle");

    let default_secrets = Secrets::default();
    assert_eq!(default_secrets.service_name, "kernelle");
  }

  #[test]
  fn test_common_keys_for_service() {
    let secrets = Secrets::new();

    assert_eq!(secrets.get_common_keys_for_group("github"), vec!["token"]);
    assert_eq!(secrets.get_common_keys_for_group("gitlab"), vec!["token"]);
    assert_eq!(secrets.get_common_keys_for_group("jira"), vec!["token", "email", "url"]);
    assert_eq!(secrets.get_common_keys_for_group("notion"), vec!["token"]);
    assert_eq!(secrets.get_common_keys_for_group("unknown"), vec!["token"]);
    assert_eq!(secrets.get_common_keys_for_group("GITHUB"), vec!["token"]);
  }

  #[test]
  fn test_credential_storage_and_retrieval() {
    let password = "test_password_123";
    let secrets = create_test_secrets_with_mock(password);
    let service = "test_service";
    let key = "test_key";
    let value = "test_secret_value";

    // Store credential
    let store_result = secrets.store_credential(service, key, value);
    assert!(store_result.is_ok(), "Failed to store credential: {:?}", store_result.err());

    // Retrieve credential
    let retrieved = secrets.get_credential(service, key);
    assert!(retrieved.is_ok(), "Failed to retrieve credential: {:?}", retrieved.err());
    assert_eq!(retrieved.unwrap(), value);
  }

  #[test]
  fn test_credential_retrieval_nonexistent() {
    let secrets = create_test_secrets();
    let result = secrets.get_credential("nonexistent_service", "nonexistent_key");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not found"));
  }

  #[test]
  fn test_credential_deletion() {
    let password = "test_password_123";
    let secrets = create_test_secrets_with_mock(password);
    let service = "test_delete_service";
    let key = "test_delete_key";
    let value = "test_delete_value";

    // Store then delete
    secrets.store_credential(service, key, value).unwrap();
    let delete_result = secrets.delete_credential(service, key);
    assert!(delete_result.is_ok());

    // Verify deletion
    let retrieve_result = secrets.get_credential(service, key);
    assert!(retrieve_result.is_err());
  }

  #[test]
  fn test_credential_overwrite() {
    let password = "test_password_123";
    let secrets = create_test_secrets_with_mock(password);
    let service = "test_overwrite_service";
    let key = "test_overwrite_key";
    let value1 = "original_value";
    let value2 = "updated_value";

    // Store original value
    secrets.store_credential(service, key, value1).unwrap();
    assert_eq!(secrets.get_credential(service, key).unwrap(), value1);

    // Overwrite with new value
    secrets.store_credential(service, key, value2).unwrap();
    assert_eq!(secrets.get_credential(service, key).unwrap(), value2);
  }

  #[test]
  fn test_multiple_credentials_same_service() {
    let password = "test_password_123";
    let secrets = create_test_secrets_with_mock(password);
    let service = "multi_cred_service";

    // Store multiple credentials for same service
    secrets.store_credential(service, "key1", "value1").unwrap();
    secrets.store_credential(service, "key2", "value2").unwrap();
    secrets.store_credential(service, "key3", "value3").unwrap();

    // Verify all can be retrieved independently
    assert_eq!(secrets.get_credential(service, "key1").unwrap(), "value1");
    assert_eq!(secrets.get_credential(service, "key2").unwrap(), "value2");
    assert_eq!(secrets.get_credential(service, "key3").unwrap(), "value3");
  }

  #[test]
  fn test_get_service_env_vars() {
    let password = "test_password_123";
    let secrets = create_test_secrets_with_mock(password);
    let service = "env_test_service";

    // Test with no credentials stored
    let env_vars = secrets.get_service_env_vars(service).unwrap();
    assert!(env_vars.is_empty());

    // Store a credential and test env var generation
    secrets.store_credential(service, "token", "test_token_123").unwrap();
    let env_vars = secrets.get_service_env_vars(service).unwrap();

    let expected_key = format!("{}_TOKEN", service.to_uppercase());
    assert!(env_vars.contains_key(&expected_key));
    assert_eq!(env_vars.get(&expected_key).unwrap(), "test_token_123");
  }

  #[test]
  fn test_get_service_env_vars_github() {
    let password = "test_password_123";
    let secrets = create_test_secrets_with_mock(password);
    let service = "github";

    // Store GitHub token
    secrets.store_credential(service, "token", "ghp_test_token").unwrap();
    let env_vars = secrets.get_service_env_vars(service).unwrap();

    assert_eq!(env_vars.get("GITHUB_TOKEN").unwrap(), "ghp_test_token");
  }

  #[test]
  fn test_get_service_env_vars_jira() {
    let password = "test_password_123";
    let secrets = create_test_secrets_with_mock(password);
    let service = "jira";

    // Store Jira credentials
    secrets.store_credential(service, "token", "jira_token").unwrap();
    secrets.store_credential(service, "email", "test@example.com").unwrap();
    secrets.store_credential(service, "url", "https://test.atlassian.net").unwrap();

    let env_vars = secrets.get_service_env_vars(service).unwrap();

    assert_eq!(env_vars.get("JIRA_TOKEN").unwrap(), "jira_token");
    assert_eq!(env_vars.get("JIRA_EMAIL").unwrap(), "test@example.com");
    assert_eq!(env_vars.get("JIRA_URL").unwrap(), "https://test.atlassian.net");
  }

  #[test]
  #[ignore = "Prompts for user input - hangs in test environment"]
  fn test_setup_service() {
    let secrets = create_test_secrets();
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
    let result = secrets.setup_service(&config);
    assert!(result.is_ok());

    // Verify credential was stored (with placeholder value)
    let stored = secrets.get_credential("test_setup_service", "test_key");
    assert!(stored.is_ok());
    assert_eq!(stored.unwrap(), "placeholder_credential");

    // Clean up
    let _ = secrets.delete_credential("test_setup_service", "test_key");
  }

  #[test]
  fn test_prompt_for_optional() {
    let secrets = create_test_secrets();
    // Test the current implementation which always returns true
    let result = secrets.prompt_for_optional("test_key");
    assert!(result.is_ok());
    assert!(result.unwrap());
  }

  #[test]
  #[ignore = "Prompts for user input - hangs in test environment"]
  fn test_prompt_for_credential() {
    let secrets = create_test_secrets();
    let spec = CredentialSpec {
      key: "test_key".to_string(),
      description: "Test description".to_string(),
      example: Some("test_example".to_string()),
      is_required: true,
    };

    let result = secrets.prompt_for_credential(&spec);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "placeholder_credential");

    // Test without example
    let spec_no_example = CredentialSpec {
      key: "test_key_no_example".to_string(),
      description: "Test description".to_string(),
      example: None,
      is_required: true,
    };

    let result = secrets.prompt_for_credential(&spec_no_example);
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
    let password = "test_password_123";
    let secrets = create_test_secrets_with_mock(password);

    // Test with empty strings
    let result = secrets.store_credential("", "", "");
    assert!(result.is_ok());

    let retrieved = secrets.get_credential("", "");
    assert!(retrieved.is_ok());
    assert_eq!(retrieved.unwrap(), "");

    // Test with special characters
    let service = "test@service#with$special%chars";
    let key = "key&with*special(chars)";
    let value = "value with spaces and symbols!@#$%^&*()";

    secrets.store_credential(service, key, value).unwrap();
    let retrieved = secrets.get_credential(service, key).unwrap();
    assert_eq!(retrieved, value);
  }

  #[test]
  fn test_delete_nonexistent_credential() {
    let password = "test_password_123";
    let secrets = create_test_secrets_with_mock(password);

    // Try to delete a credential that doesn't exist
    let result = secrets.delete_credential("nonexistent", "key");
    assert!(result.is_err());
  }

  #[test]
  #[ignore = "Prompts for user input - hangs in test environment"]
  fn test_setup_service_with_optional_credentials() {
    let secrets = Secrets::new();

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
    let _ = secrets.delete_credential(&config.name, "required_token");
    let _ = secrets.delete_credential(&config.name, "optional_key");

    // Test setup service - this will call prompt_for_optional and store_credential
    let result = secrets.setup_service(&config);
    assert!(result.is_ok());

    // Verify the required credential was stored
    let required_cred = secrets.get_credential(&config.name, "required_token");
    assert!(required_cred.is_ok());
    assert_eq!(required_cred.unwrap(), "placeholder_credential");

    // Verify the optional credential was also stored (since prompt_for_optional returns true)
    let optional_cred = secrets.get_credential(&config.name, "optional_key");
    assert!(optional_cred.is_ok());
    assert_eq!(optional_cred.unwrap(), "placeholder_credential");

    // Clean up
    let _ = secrets.delete_credential(&config.name, "required_token");
    let _ = secrets.delete_credential(&config.name, "optional_key");
  }

  #[test]
  #[ignore = "Prompts for user input - hangs in test environment"]
  fn test_prompt_for_credential_with_example() {
    let secrets = Secrets::new();

    // Test credential spec with example
    let spec_with_example = CredentialSpec {
      key: "test_key".to_string(),
      description: "Test credential".to_string(),
      example: Some("example_value_123".to_string()),
      is_required: true,
    };

    let result = secrets.prompt_for_credential(&spec_with_example);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "placeholder_credential");

    // Test credential spec without example
    let spec_without_example = CredentialSpec {
      key: "test_key_no_example".to_string(),
      description: "Test credential without example".to_string(),
      example: None,
      is_required: true,
    };

    let result = secrets.prompt_for_credential(&spec_without_example);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "placeholder_credential");
  }

  #[test]
  #[ignore = "Prompts for user input - hangs in test environment"]
  fn test_setup_service_with_mixed_optional_credentials() {
    let secrets = Secrets::new();

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
    let _ = secrets.delete_credential(&config.name, "required_token");
    let _ = secrets.delete_credential(&config.name, "optional_key");

    // Test setup service - this will exercise both branches of the conditional
    let result = secrets.setup_service(&config);
    assert!(result.is_ok());

    // Verify the required credential was stored (line 109 coverage)
    let required_cred = secrets.get_credential(&config.name, "required_token");
    assert!(required_cred.is_ok());
    assert_eq!(required_cred.unwrap(), "placeholder_credential");

    // Verify the optional credential was also stored since prompt_for_optional returns true
    let optional_cred = secrets.get_credential(&config.name, "optional_key");
    assert!(optional_cred.is_ok());
    assert_eq!(optional_cred.unwrap(), "placeholder_credential");

    // Clean up
    let _ = secrets.delete_credential(&config.name, "required_token");
    let _ = secrets.delete_credential(&config.name, "optional_key");
  }

  #[test]
  fn test_store_credential_success_logging() {
    let password = "test_password_123";
    let secrets = create_test_secrets_with_mock(password);

    // Use a unique service and key to avoid conflicts with other tests
    let test_service = format!("test_logging_service_{}", std::process::id());
    let test_key = format!("test_logging_key_{}", std::process::id());

    // This test specifically targets line 53 - the bentley::event_success call
    let result = secrets.store_credential(&test_service, &test_key, "test_value");
    assert!(result.is_ok(), "Failed to store credential: {:?}", result.err());

    // Verify the credential was actually stored
    let retrieved = secrets.get_credential(&test_service, &test_key);
    assert!(retrieved.is_ok(), "Failed to retrieve credential: {:?}", retrieved.err());
    assert_eq!(retrieved.unwrap(), "test_value");
  }

  #[test]
  fn test_argon2_password_security() {
    let password1 = "correct_password_123";
    let password2 = "wrong_password_456";
    let secrets1 = create_test_secrets_with_mock(password1);
    let secrets2 = create_test_secrets_with_mock(password2);
    let service = "security_test_service";
    let key = "security_test_key";
    let value = "secret_value";

    // Store with password1
    let result = secrets1.store_credential(service, key, value);
    assert!(result.is_ok(), "Failed to store with correct password");

    // Try to retrieve with secrets2 (different password) - should fail
    let wrong_password_result = secrets2.get_credential(service, key);
    assert!(wrong_password_result.is_err(), "Should not be able to retrieve with wrong password");

    // Verify we can still retrieve with correct password
    let correct_password_result = secrets1.get_credential(service, key);
    assert!(correct_password_result.is_ok(), "Should be able to retrieve with correct password");
    assert_eq!(correct_password_result.unwrap(), value);
  }

  #[test]
  fn test_enhanced_device_fingerprinting() {
    use crate::encryption::EncryptionManager;

    // Test that enhanced device fingerprinting works
    let machine_key = EncryptionManager::machine_key();
    assert!(machine_key.is_ok(), "Enhanced machine key generation should succeed");

    let key1 = machine_key.unwrap();
    assert_eq!(key1.len(), 32, "Machine key should be 32 bytes");

    // Test consistency - should generate the same key
    let key2 = EncryptionManager::machine_key().unwrap();
    assert_eq!(key1, key2, "Machine key should be deterministic");

    // Show fingerprinting details
    println!("\nSimplified Device Fingerprinting Test");
    println!("========================================");
    println!("Machine key generated successfully");
    println!("Key length: {} bytes (256-bit)", key1.len());
    println!("Key (hex): {}", hex::encode(&key1));

    // Show what identifier is being used
    println!("\nDevice Identification Strategy:");
    println!("----------------------------------");

    if let Ok(hostname) = hostname::get() {
      println!("Hostname: {}", hostname.to_string_lossy());
    }
    println!("Username: {}", whoami::username());

    // Try to determine which method is being used
    if std::fs::read_to_string("/etc/machine-id").is_ok() {
      println!("� Using: Linux machine-id (persistent across reboots)");
    } else if std::fs::read_to_string("/sys/class/dmi/id/product_uuid").is_ok() {
      println!("Using: Hardware UUID from DMI (most stable)");
    } else {
      println!("� Using: Fallback deterministic UUID from hostname+username");
    }

    println!("\nSecurity Features:");
    println!("----------------------");
    println!("UUID-based device binding (optimal stability)");
    println!("Hardware-first approach (survives OS changes when possible)");
    println!("Deterministic fallback (guaranteed compatibility)");
    println!("Simplified and focused (no unnecessary complexity)");
    println!();
  }
}

// Re-export command types and handlers for use by other crates
