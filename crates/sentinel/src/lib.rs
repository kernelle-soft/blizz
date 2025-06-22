use anyhow::{anyhow, Result};
use keyring::Entry;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Sentinel - The watchful guardian of secrets
/// 
/// Provides secure credential storage using the OS keychain (macOS Keychain, 
/// Windows Credential Manager, Linux Secret Service).
pub struct Sentinel {
  service_name: String,
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

impl Sentinel {
  /// Create a new Sentinel instance for the Kernelle toolset
  pub fn new() -> Self {
    Self {
      service_name: "kernelle".to_string(),
    }
  }

  /// Store a credential securely in the OS keychain
  pub fn store_credential(&self, service: &str, key: &str, value: &str) -> Result<()> {
    bentley::event_info(&format!("Storing credential for {}/{}", service, key));
    
    let entry_name = format!("{}_{}", service, key);
    let entry = Entry::new(&self.service_name, &entry_name)?;
    
    entry.set_password(value)?;
    
    bentley::event_success(&format!("Credential stored securely for {}/{}", service, key));
    Ok(())
  }

  /// Retrieve a credential from the OS keychain
  pub fn get_credential(&self, service: &str, key: &str) -> Result<String> {
    let entry_name = format!("{}_{}", service, key);
    let entry = Entry::new(&self.service_name, &entry_name)?;
    
    match entry.get_password() {
      Ok(password) => Ok(password),
      Err(_) => Err(anyhow!("Credential not found for {}/{}", service, key)),
    }
  }

  /// Delete a credential from the OS keychain
  pub fn delete_credential(&self, service: &str, key: &str) -> Result<()> {
    bentley::event_info(&format!("Deleting credential for {}/{}", service, key));
    
    let entry_name = format!("{}_{}", service, key);
    let entry = Entry::new(&self.service_name, &entry_name)?;
    
    entry.delete_password()?;
    
    bentley::event_success(&format!("Credential deleted for {}/{}", service, key));
    Ok(())
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
      if cred_spec.is_required {
        if self.get_credential(&config.name, &cred_spec.key).is_err() {
          missing.push(cred_spec.key.clone());
        }
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
    // For now, return a placeholder - in a real implementation, this would prompt securely
    // TODO: Add secure credential prompting (hidden input for tokens)
    bentley::warn(&format!("TODO: Implement secure prompting for {}", spec.key));
    
    if let Some(example) = &spec.example {
      bentley::info(&format!("Example: {}", example));
    }
    
    // Return a placeholder for now
    Ok("placeholder_credential".to_string())
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
      required_credentials: vec![
        CredentialSpec {
          key: "token".to_string(),
          description: "GitHub Personal Access Token with repo and pull request permissions".to_string(),
          example: Some("ghp_xxxxxxxxxxxxxxxxxxxx".to_string()),
          is_required: true,
        },
      ],
    }
  }

  pub fn gitlab() -> ServiceConfig {
    ServiceConfig {
      name: "gitlab".to_string(),
      description: "GitLab API access for merge request management".to_string(),
      required_credentials: vec![
        CredentialSpec {
          key: "token".to_string(),
          description: "GitLab Personal Access Token with API and read_repository permissions".to_string(),
          example: Some("glpat-xxxxxxxxxxxxxxxxxxxx".to_string()),
          is_required: true,
        },
      ],
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
      required_credentials: vec![
        CredentialSpec {
          key: "token".to_string(),
          description: "Notion Integration Token".to_string(),
          example: Some("secret_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx".to_string()),
          is_required: true,
        },
      ],
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_service_configs() {
    let github_config = services::github();
    assert_eq!(github_config.name, "github");
    assert_eq!(github_config.required_credentials.len(), 1);
    assert_eq!(github_config.required_credentials[0].key, "token");
    assert!(github_config.required_credentials[0].is_required);

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
  }

  #[test]
  fn test_common_keys_for_service() {
    let sentinel = Sentinel::new();
    
    assert_eq!(sentinel.get_common_keys_for_service("github"), vec!["token"]);
    assert_eq!(sentinel.get_common_keys_for_service("gitlab"), vec!["token"]);
    assert_eq!(sentinel.get_common_keys_for_service("jira"), vec!["token", "email", "url"]);
    assert_eq!(sentinel.get_common_keys_for_service("notion"), vec!["token"]);
    assert_eq!(sentinel.get_common_keys_for_service("unknown"), vec!["token"]);
  }
} 