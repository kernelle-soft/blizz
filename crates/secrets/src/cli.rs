use crate::{services, Secrets};
use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use std::env;
use std::io::Write;
use std::path::PathBuf;

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

  let secrets = Secrets::new();

  match command {
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
  _secrets: &Secrets,
  group_filter: Option<String>,
  show_keys: bool,
  quiet: bool,
) -> Result<()> {
  // Get the credentials file path (same logic as PasswordBasedCryptoManager::new)
  let base_path = if let Ok(kernelle_dir) = std::env::var("KERNELLE_DIR") {
    PathBuf::from(kernelle_dir)
  } else {
    dirs::home_dir().unwrap_or_else(|| std::env::current_dir().unwrap()).join(".kernelle")
  };

  let mut credentials_path = base_path;
  credentials_path.push("persistent");
  credentials_path.push("keeper");
  credentials_path.push("credentials.enc");

  // Check if credentials file exists
  if !credentials_path.exists() {
    bentley::log("no secrets stored yet");
    return Ok(());
  }

  // Load the encrypted store from file
  use crate::PasswordBasedCredentialStore;
  let store = match PasswordBasedCredentialStore::load_from_file(&credentials_path)? {
    Some(store) => store,
    None => {
      bentley::log("no secrets found");
      return Ok(());
    }
  };

  // Prompt for master password
  bentley::log("enter master password:");
  print!("> ");
  std::io::stdout().flush()?;
  let master_password = rpassword::read_password()?;

  if master_password.trim().is_empty() {
    return Err(anyhow!("master password cannot be empty"));
  }

  // Decrypt all credentials
  let all_credentials = match store.decrypt_credentials(master_password.trim()) {
    Ok(creds) => creds,
    Err(_) => {
      bentley::error("invalid master password or corrupted data");
      return Ok(());
    }
  };

  // Display the contents
  if all_credentials.is_empty() {
    bentley::log("vault is empty");
    return Ok(());
  }

  // Filter by group if specified
  let filter_group = group_filter.clone();
  let credentials_to_show = if let Some(filter) = group_filter {
    all_credentials.into_iter().filter(|(group, _)| group == &filter).collect()
  } else {
    all_credentials
  };

  if credentials_to_show.is_empty() {
    if let Some(filter) = filter_group {
      bentley::log(&format!("no secrets found for group: {}", filter));
    } else {
      bentley::log("no secrets found");
    }
    return Ok(());
  }

  // Display format depends on show_keys flag
  if show_keys {
    // Show detailed view with group/key pairs
    for (group, secrets_map) in credentials_to_show {
      bentley::log(&format!("\n📁 {}/", group));
      for key in secrets_map.keys() {
        println!("   🔑 {}/{}", group, key);
      }
    }
  } else {
    // Show summary view with just groups and counts
    for (group, secrets_map) in credentials_to_show {
      let count = secrets_map.len();
      let plural = if count == 1 { "secret" } else { "secrets" };
      bentley::log(&format!("📁 {}: {} {}", group, count, plural));
    }

    if !quiet {
      bentley::log("\nuse --keys to see individual secret names");
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
