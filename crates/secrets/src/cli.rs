use crate::{services, Secrets};
use anyhow::Result;
use clap::{Parser, Subcommand};
use std::env;

#[derive(Parser)]
#[command(name = "secrets")]
#[command(
  about = "Secrets management for Kernelle, the AI toolshed.",
  long_about = "Secure secret storage for Kernelle tools. Secrets are organized into groups for better management."
)]
#[command(version = concat!(env!("CARGO_PKG_VERSION"), ", courtesy of kernelle"))]
pub struct Cli {
  #[command(subcommand)]
  pub command: Commands,
  /// Suppress banners and flourishes (useful when called from other tools)
  #[arg(long, global = true)]
  pub quiet: bool,
}

#[derive(Subcommand)]
pub enum Commands {
  /// Setup credentials for a predefined service
  Setup {
    /// Service to setup (github, gitlab, jira, notion)
    service: String,
    /// Force reconfiguration of existing credentials
    #[arg(long)]
    force: bool,
  },
  /// Store a secret entry
  Store {
    /// Secret name/key
    name: String,
    /// Value to store (will be prompted securely if not provided)
    value: Option<String>,
    /// Group/namespace for the secret (defaults to 'general')
    #[arg(short, long)]
    group: Option<String>,
    /// Force overwrite existing secret
    #[arg(long)]
    force: bool,
  },
  /// Retrieve/read a secret entry
  Read {
    /// Secret name/key
    name: String,
    /// Group/namespace for the secret (defaults to 'general')
    #[arg(short, long)]
    group: Option<String>,
    /// Show the secret value (default: just confirm existence)
    #[arg(long)]
    show: bool,
  },
  /// Update an existing secret entry
  Update {
    /// Secret name/key
    name: String,
    /// Group/namespace for the secret (defaults to 'general')
    #[arg(short, long)]
    group: Option<String>,
    /// New value to store (will be prompted securely if not provided)
    #[arg(long)]
    value: Option<String>,
    /// Skip confirmation prompt
    #[arg(long)]
    force: bool,
  },
  /// Delete secret entries
  Delete {
    /// Secret name/key
    name: String,
    /// Group/namespace for the secret (defaults to 'general')
    #[arg(short, long)]
    group: Option<String>,
    /// Skip confirmation prompt
    #[arg(long)]
    force: bool,
  },
  /// List all secret entries
  #[command(visible_alias = "ls")]
  List {
    /// Show only entries for a specific group
    #[arg(short, long)]
    group: Option<String>,
    /// Show secret keys (default: just group names)
    #[arg(long)]
    keys: bool,
  },
  /// Clear all secrets from the vault
  Clear {
    /// Skip confirmation prompt
    #[arg(long)]
    force: bool,
  },
  /// Verify credentials for a predefined service
  Verify {
    /// Service to verify (github, gitlab, jira, notion)
    service: String,
  },

  // Legacy command aliases for backward compatibility
  /// @deprecated Use 'read' instead
  #[command(hide = true)]
  Get {
    /// Service/namespace for the credential
    service: String,
    /// Key name for the credential
    key: String,
    /// Show the credential value (default: just confirm existence)
    #[arg(long)]
    show: bool,
  },
}

/// Handle a secrets command
pub async fn handle_command(command: Commands) -> Result<()> {
  // Auto-detect quiet mode if called as subprocess or if SECRETS_QUIET is set
  let quiet_mode = env::var("SECRETS_QUIET").is_ok() || is_subprocess();

  if !quiet_mode {
    bentley::spotlight("Secrets");
  }

  let secrets = Secrets::new();

  match command {
    Commands::Setup { service, force } => {
      handle_setup(&secrets, &service, force, quiet_mode).await?;
    }
    Commands::Store { name, value, group, force } => {
      let group = group.unwrap_or_else(|| "general".to_string());
      handle_store(&secrets, &group, &name, value, force).await?;
    }
    Commands::Read { name, group, show } => {
      let group = group.unwrap_or_else(|| "general".to_string());
      handle_read(&secrets, &group, &name, show).await?;
    }
    Commands::Update { name, group, value, force } => {
      let group = group.unwrap_or_else(|| "general".to_string());
      handle_update(&secrets, &group, &name, value, force).await?;
    }
    Commands::Delete { name, group, force } => {
      let group = group.unwrap_or_else(|| "general".to_string());
      handle_delete(&secrets, &group, Some(name), force).await?;
    }
    Commands::List { group, keys } => {
      handle_list(&secrets, group, keys, quiet_mode).await?;
    }
    Commands::Clear { force } => {
      handle_clear(&secrets, force, quiet_mode).await?;
    }
    Commands::Verify { service } => {
      handle_verify(&secrets, &service).await?;
    }
    // Legacy command for backward compatibility
    Commands::Get { service, key, show } => {
      handle_read(&secrets, &service, &key, show).await?;
    }
  }

  Ok(())
}

/// Detect if we're running as a subprocess
fn is_subprocess() -> bool {
  // Check if parent process is not a shell-like process
  // Simple heuristic: if SECRETS_QUIET env var is set by parent process
  env::var("PPID").is_ok() && env::var("SHLVL").map_or(true, |level| level != "1")
}

async fn handle_setup(
  secrets: &Secrets,
  service_name: &str,
  force: bool,
  quiet: bool,
) -> Result<()> {
  let service_config = match service_name.to_lowercase().as_str() {
    "github" => services::github(),
    "gitlab" => services::gitlab(),
    "jira" => services::jira(),
    "notion" => services::notion(),
    _ => {
      bentley::error(&format!(
        "Unsupported service: {service_name}. Supported services: github, gitlab, jira, notion"
      ));
      bentley::info("Use 'secrets store' for arbitrary credential storage");
      return Ok(());
    }
  };

  // Check if credentials already exist
  let missing = secrets.verify_service_credentials(&service_config)?;

  if missing.is_empty() && !force {
    bentley::success(&format!(
      "All credentials for {} are already configured!",
      service_config.name
    ));
    bentley::info("Use --force to reconfigure existing credentials");
    return Ok(());
  }

  if !quiet {
    bentley::announce(&format!("Setting up credentials for {}", service_config.name));
  }
  bentley::info(&service_config.description);

  for cred_spec in &service_config.required_credentials {
    if force || missing.contains(&cred_spec.key) {
      bentley::info(&format!("\n📝 Setting up: {}", cred_spec.description));

      if let Some(example) = &cred_spec.example {
        bentley::info(&format!("   Example format: {example}"));
      }

      let prompt = format!("Enter {}: ", cred_spec.key);
      let value = rpassword::prompt_password(prompt)?;

      if value.trim().is_empty() {
        bentley::warn(&format!("Skipping empty {} - you can set it up later", cred_spec.key));
        continue;
      }

      secrets.store_secret_raw(&service_config.name, &cred_spec.key, value.trim())?;
    }
  }

  if !quiet {
    bentley::flourish(&format!("Credentials setup complete for {}!", service_config.name));
  } else {
    bentley::success(&format!("✅ {} credentials configured successfully!", service_config.name));
  }
  Ok(())
}

async fn handle_store(
  secrets: &Secrets,
  group: &str,
  name: &str,
  value: Option<String>,
  force: bool,
) -> Result<()> {
  // Check if secret already exists
  if !force && secrets.get_secret_raw_no_setup(group, name).is_ok() {
    bentley::warn(&format!("Secret {group}/{name} already exists"));
    bentley::info("Use --force to overwrite existing secret");
    return Ok(());
  }

  let secret_value = if let Some(val) = value {
    val
  } else {
    let prompt = format!("Enter value for {group}/{name}: ");
    rpassword::prompt_password(prompt)?
  };

  if secret_value.trim().is_empty() {
    bentley::error("Cannot store empty secret value");
    return Ok(());
  }

  secrets.store_secret_raw(group, name, secret_value.trim())?;
  bentley::success(&format!("Stored secret: {group}/{name}"));
  Ok(())
}

async fn handle_read(secrets: &Secrets, group: &str, name: &str, show: bool) -> Result<()> {
  match secrets.get_secret_raw_no_setup(group, name) {
    Ok(value) => {
      if show {
        bentley::info(&format!("Secret {group}/{name}:"));
        println!("{value}");
      } else {
        bentley::success(&format!("✅ Secret {group}/{name} exists"));
      }
    }
    Err(_) => {
      bentley::error(&format!("❌ Secret not found: {group}/{name}"));
      std::process::exit(1);
    }
  }
  Ok(())
}

async fn handle_update(
  secrets: &Secrets,
  group: &str,
  name: &str,
  value: Option<String>,
  force: bool,
) -> Result<()> {
  // Check if secret exists
  if secrets.get_secret_raw_no_setup(group, name).is_err() {
    bentley::warn(&format!("Secret not found: {group}/{name}"));
    return Ok(());
  }

  let new_value = if let Some(val) = value {
    val
  } else {
    bentley::info(&format!("Enter new value for {group}/{name}:"));
    rpassword::prompt_password("New value: ")?
  };

  if !force {
    bentley::info(&format!("Update secret {group}/{name}?"));
    let input = rpassword::prompt_password("Type 'yes' to confirm: ")?;
    if input.trim().to_lowercase() != "yes" {
      bentley::info("Update cancelled");
      return Ok(());
    }
  }

  secrets.store_secret_raw(group, name, &new_value)?;
  bentley::success(&format!("Updated secret: {group}/{name}"));
  Ok(())
}

async fn handle_delete(
  secrets: &Secrets,
  group: &str,
  name: Option<String>,
  force: bool,
) -> Result<()> {
  if let Some(name) = name {
    // Delete specific secret
    if secrets.get_secret_raw_no_setup(group, &name).is_err() {
      bentley::error(&format!("Secret not found: {group}/{name}"));
      return Ok(());
    }

    if !force {
      bentley::warn(&format!("This will delete the secret: {group}/{name}"));
      let confirm = rpassword::prompt_password("Type 'yes' to confirm: ")?;
      if confirm.trim().to_lowercase() != "yes" {
        bentley::info("Cancelled");
        return Ok(());
      }
    }

    secrets.delete_secret(group, &name)?;
    bentley::success(&format!("Deleted secret: {group}/{name}"));
  } else {
    // Delete all secrets for group
    if !force {
      bentley::warn(&format!("This will delete ALL secrets for group: {group}"));
      let confirm = rpassword::prompt_password("Type 'yes' to confirm: ")?;
      if confirm.trim().to_lowercase() != "yes" {
        bentley::info("Cancelled");
        return Ok(());
      }
    }

    // For arbitrary groups, we can't enumerate keys easily with the current API
    // So we'll try to delete common keys and let the user know
    bentley::info(&format!("Attempting to delete all secrets for group: {group}"));

    // Try common secret keys
    let common_keys = ["token", "api_key", "password", "secret", "key", "pat", "access_token"];
    let mut deleted_count = 0;

    for key in &common_keys {
      if secrets.get_secret_raw_no_setup(group, key).is_ok()
        && secrets.delete_secret(group, key).is_ok()
      {
        deleted_count += 1;
        bentley::info(&format!("Deleted: {group}/{key}"));
      }
    }

    if deleted_count > 0 {
      bentley::success(&format!("Deleted {deleted_count} secrets for group: {group}"));
    } else {
      bentley::info(&format!("No secrets found for group: {group}"));
    }
  }

  Ok(())
}

async fn handle_list(
  secrets: &Secrets,
  group_filter: Option<String>,
  show_keys: bool,
  quiet: bool,
) -> Result<()> {
  if !quiet {
    bentley::announce("Secret Vault Contents");
  }

  if let Some(group) = group_filter {
    // List secrets for specific group
    bentley::info(&format!("Group: {group}"));

    if show_keys {
      // Try common secret keys to see what exists
      let common_keys = ["token", "api_key", "password", "secret", "key", "pat", "access_token"];
      let mut found_keys = Vec::new();

      for key in &common_keys {
        if secrets.get_secret_raw_no_setup(&group, key).is_ok() {
          found_keys.push(key);
        }
      }

      if found_keys.is_empty() {
        bentley::info("  No secrets found");
      } else {
        for key in found_keys {
          bentley::success(&format!("  ✅ {key}"));
        }
      }
    } else {
      bentley::info("  Use --keys to show secret keys");
    }
  } else {
    // List all groups (predefined + any we can discover)
    let predefined_services = ["github", "gitlab", "jira", "notion"];
    let mut found_groups = Vec::new();

    for service_name in &predefined_services {
      let service_config = match *service_name {
        "github" => services::github(),
        "gitlab" => services::gitlab(),
        "jira" => services::jira(),
        "notion" => services::notion(),
        _ => continue,
      };

      let missing = secrets.verify_service_credentials(&service_config)?;
      let configured = service_config.required_credentials.len() - missing.len();

      if configured > 0 {
        found_groups.push(service_name);
        let total = service_config.required_credentials.len();
        bentley::success(&format!("📋 {}: {}/{} secrets", service_config.name, configured, total));

        if show_keys {
          for cred_spec in &service_config.required_credentials {
            if !missing.contains(&cred_spec.key) {
              bentley::info(&format!("    ✅ {}", cred_spec.key));
            } else {
              bentley::warn(&format!("    ❌ {}", cred_spec.key));
            }
          }
        }
      }
    }

    if found_groups.is_empty() {
      bentley::info("No predefined services configured");
      bentley::info("Use 'secrets setup <service>' for GitHub, GitLab, Jira, or Notion");
      bentley::info("Use 'secrets store <name> [-g <group>]' for arbitrary secrets");
    }
  }

  Ok(())
}

async fn handle_clear(secrets: &Secrets, force: bool, quiet: bool) -> Result<()> {
  if !force {
    bentley::warn("⚠️  This will DELETE ALL SECRETS from the vault!");
    bentley::warn("This action cannot be undone!");
    let confirm = rpassword::prompt_password("Type 'DELETE ALL' to confirm: ")?;
    if confirm.trim() != "DELETE ALL" {
      bentley::info("Cancelled - vault contents preserved");
      return Ok(());
    }
  }

  bentley::info("Clearing secret vault...");

  // Clear predefined services
  let services = ["github", "gitlab", "jira", "notion"];
  let mut cleared_count = 0;

  for service_name in &services {
    let service_config = match *service_name {
      "github" => services::github(),
      "gitlab" => services::gitlab(),
      "jira" => services::jira(),
      "notion" => services::notion(),
      _ => continue,
    };

    for cred_spec in &service_config.required_credentials {
      if secrets.get_secret_raw_no_setup(&service_config.name, &cred_spec.key).is_ok()
        && secrets.delete_secret(&service_config.name, &cred_spec.key).is_ok()
      {
        cleared_count += 1;
      }
    }
  }

  // Note: We can't easily enumerate all arbitrary secrets with the current API
  // So we inform the user about this limitation
  if cleared_count > 0 {
    bentley::success(&format!("Cleared {cleared_count} predefined service secrets"));
  }

  bentley::info(
    "Note: Any arbitrary secrets (not from predefined services) must be deleted individually",
  );
  bentley::info("Use 'secrets delete <name> [-g <group>]' to remove specific secrets");

  if !quiet {
    bentley::flourish("Vault clearing complete!");
  } else {
    bentley::success("Vault cleared successfully");
  }
  Ok(())
}

async fn handle_verify(secrets: &Secrets, service_name: &str) -> Result<()> {
  let service_config = match service_name.to_lowercase().as_str() {
    "github" => services::github(),
    "gitlab" => services::gitlab(),
    "jira" => services::jira(),
    "notion" => services::notion(),
    _ => {
      bentley::error(&format!("Unsupported service: {service_name}"));
      bentley::info("Use 'secrets get <service> <key>' to check arbitrary credentials");
      return Ok(());
    }
  };

  bentley::info(&format!("Verifying credentials for {}...", service_config.name));

  let missing = secrets.verify_service_credentials(&service_config)?;

  if missing.is_empty() {
    bentley::success(&format!(
      "✅ All required credentials configured for {}",
      service_config.name
    ));
  } else {
    bentley::warn(&format!(
      "❌ Missing credentials for {}: {}",
      service_config.name,
      missing.join(", ")
    ));
    bentley::info(&format!("Run 'secrets setup {service_name}' to configure missing credentials"));
  }

  Ok(())
}
