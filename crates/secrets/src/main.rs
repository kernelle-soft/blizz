use anyhow::Result;
use clap::{Parser, Subcommand};
use secrets::{services, Secrets};
use std::env;

#[derive(Parser)]
#[command(name = "secrets")]
#[command(
  about = "Secure credential storage for Kernelle tools - the watchful guardian of secrets"
)]
#[command(version = concat!(env!("CARGO_PKG_VERSION"), ", courtesy of kernelle"))]
struct Cli {
  #[command(subcommand)]
  command: Commands,
  /// Suppress banners and flourishes (useful when called from other tools)
  #[arg(long, global = true)]
  quiet: bool,
}

#[derive(Subcommand)]
enum Commands {
  /// Store an arbitrary credential entry
  Store {
    /// Service/namespace for the credential
    service: String,
    /// Key name for the credential
    key: String,
    /// Value to store (will be prompted securely if not provided)
    #[arg(long)]
    value: Option<String>,
    /// Force overwrite existing credential
    #[arg(long)]
    force: bool,
  },
  /// Retrieve a credential entry
  Get {
    /// Service/namespace for the credential
    service: String,
    /// Key name for the credential
    key: String,
    /// Show the credential value (default: just confirm existence)
    #[arg(long)]
    show: bool,
  },
  /// Update an existing credential entry
  Update {
    /// Service/namespace for the credential
    service: String,
    /// Key name for the credential
    key: String,
    /// New value to store (will be prompted securely if not provided)
    #[arg(long)]
    value: Option<String>,
    /// Skip confirmation prompt
    #[arg(long)]
    force: bool,
  },
  /// Delete credential entries
  Delete {
    /// Service/namespace for the credential
    service: String,
    /// Key name for the credential (optional - deletes all service credentials if not specified)
    key: Option<String>,
    /// Skip confirmation prompt
    #[arg(long)]
    force: bool,
  },
  /// List all credential entries
  List {
    /// Show only entries for a specific service
    service: Option<String>,
    /// Show credential keys (default: just service names)
    #[arg(long)]
    keys: bool,
  },
  /// Clear all credentials from the vault
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
}

#[tokio::main]
async fn main() -> Result<()> {
  let cli = Cli::parse();

  // Auto-detect quiet mode if called as subprocess or if SENTINEL_QUIET is set
  let quiet_mode = cli.quiet || env::var("SENTINEL_QUIET").is_ok() || is_subprocess();

  if !quiet_mode {
    bentley::spotlight("Sentinel - The Watchful Guardian of Secrets");
  }

  let secrets = Secrets::new();

  match cli.command {
    Commands::Store { service, key, value, force } => {
      handle_store(&secrets, &service, &key, value, force).await?;
    }
    Commands::Get { service, key, show } => {
      handle_get(&secrets, &service, &key, show).await?;
    }
    Commands::Update { service, key, value, force } => {
      handle_update(&secrets, &service, &key, value, force).await?;
    }
    Commands::Delete { service, key, force } => {
      handle_delete(&secrets, &service, key, force).await?;
    }
    Commands::List { service, keys } => {
      handle_list(&secrets, service, keys, quiet_mode).await?;
    }
    Commands::Clear { force } => {
      handle_clear(&secrets, force, quiet_mode).await?;
    }
    Commands::Verify { service } => {
      handle_verify(&secrets, &service).await?;
    }
  }

  Ok(())
}

/// Detect if we're running as a subprocess
fn is_subprocess() -> bool {
  // Check if parent process is not a shell-like process
  // Simple heuristic: if SENTINEL_QUIET env var is set by parent process
  env::var("PPID").is_ok() && env::var("SHLVL").map_or(true, |level| level != "1")
}

async fn handle_store(
  secrets: &Secrets,
  service: &str,
  key: &str,
  value: Option<String>,
  force: bool,
) -> Result<()> {
  // Check if credential already exists
  if !force && secrets.get_credential_raw(service, key).is_ok() {
    bentley::warn(&format!("Credential {service}/{key} already exists"));
    bentley::info("Use --force to overwrite existing credential");
    return Ok(());
  }

  let credential_value = if let Some(val) = value {
    val
  } else {
    let prompt = format!("Enter value for {service}/{key}: ");
    rpassword::prompt_password(prompt)?
  };

  if credential_value.trim().is_empty() {
    bentley::error("Cannot store empty credential value");
    return Ok(());
  }

  secrets.store_credential_raw(service, key, credential_value.trim())?;
  bentley::success(&format!("Stored credential: {service}/{key}"));
  Ok(())
}

async fn handle_get(secrets: &Secrets, service: &str, key: &str, show: bool) -> Result<()> {
  match secrets.get_credential_raw(service, key) {
    Ok(value) => {
      if show {
        bentley::info(&format!("Credential {service}/{key}:"));
        println!("{value}");
      } else {
        bentley::success(&format!("✅ Credential {service}/{key} exists"));
      }
    }
    Err(_) => {
      bentley::error(&format!("❌ Credential not found: {service}/{key}"));
      std::process::exit(1);
    }
  }
  Ok(())
}

async fn handle_update(
  secrets: &Secrets,
  service: &str,
  key: &str,
  value: Option<String>,
  force: bool,
) -> Result<()> {
  // Check if credential exists
  if secrets.get_credential_raw(service, key).is_err() {
    bentley::warn(&format!("Credential not found: {service}/{key}"));
    return Ok(());
  }

  let new_value = if let Some(val) = value {
    val
  } else {
    bentley::info(&format!("Enter new value for {service}/{key}:"));
    rpassword::prompt_password("New value: ")?
  };

  if !force {
    bentley::info(&format!("Update credential {service}/{key}?"));
    let input = rpassword::prompt_password("Type 'yes' to confirm: ")?;
    if input.trim().to_lowercase() != "yes" {
      bentley::info("Update cancelled");
      return Ok(());
    }
  }

  secrets.store_credential_raw(service, key, &new_value)?;
  bentley::success(&format!("Updated credential: {service}/{key}"));
  Ok(())
}

async fn handle_delete(
  secrets: &Secrets,
  service: &str,
  key: Option<String>,
  force: bool,
) -> Result<()> {
  if let Some(key) = key {
    // Delete specific credential
    if secrets.get_credential_raw(service, &key).is_err() {
      bentley::error(&format!("Credential not found: {service}/{key}"));
      return Ok(());
    }

    if !force {
      bentley::warn(&format!("This will delete the credential: {service}/{key}"));
      let confirm = rpassword::prompt_password("Type 'yes' to confirm: ")?;
      if confirm.trim().to_lowercase() != "yes" {
        bentley::info("Cancelled");
        return Ok(());
      }
    }

    secrets.delete_credential(service, &key)?;
    bentley::success(&format!("Deleted credential: {service}/{key}"));
  } else {
    // Delete all credentials for service
    if !force {
      bentley::warn(&format!("This will delete ALL credentials for service: {service}"));
      let confirm = rpassword::prompt_password("Type 'yes' to confirm: ")?;
      if confirm.trim().to_lowercase() != "yes" {
        bentley::info("Cancelled");
        return Ok(());
      }
    }

    // For arbitrary services, we can't enumerate keys easily with the current keyring API
    // So we'll try to delete common keys and let the user know
    bentley::info(&format!("Attempting to delete all credentials for service: {service}"));

    // Try common credential keys
    let common_keys = ["token", "api_key", "password", "secret", "key", "pat", "access_token"];
    let mut deleted_count = 0;

    for key in &common_keys {
      if secrets.get_credential_raw(service, key).is_ok()
        && secrets.delete_credential(service, key).is_ok()
      {
        deleted_count += 1;
        bentley::info(&format!("Deleted: {service}/{key}"));
      }
    }

    if deleted_count > 0 {
      bentley::success(&format!("Deleted {deleted_count} credentials for service: {service}"));
    } else {
      bentley::info(&format!("No credentials found for service: {service}"));
    }
  }

  Ok(())
}

async fn handle_list(
  secrets: &Secrets,
  service_filter: Option<String>,
  show_keys: bool,
  quiet: bool,
) -> Result<()> {
  if !quiet {
    bentley::announce("Credential Vault Contents");
  }

  if let Some(service) = service_filter {
    // List credentials for specific service
    bentley::info(&format!("Service: {service}"));

    if show_keys {
      // Try common credential keys to see what exists
      let common_keys = ["token", "api_key", "password", "secret", "key", "pat", "access_token"];
      let mut found_keys = Vec::new();

      for key in &common_keys {
        if secrets.get_credential_raw(&service, key).is_ok() {
          found_keys.push(key);
        }
      }

      if found_keys.is_empty() {
        bentley::info("  No credentials found");
      } else {
        for key in found_keys {
          bentley::success(&format!("  ✅ {key}"));
        }
      }
    } else {
      bentley::info("  Use --keys to show credential keys");
    }
  } else {
    // List all services (predefined + any we can discover)
    let predefined_services = ["github", "gitlab", "jira", "notion"];
    let mut found_services = Vec::new();

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
        found_services.push(service_name);
        let total = service_config.required_credentials.len();
        bentley::success(&format!(
          "📋 {}: {}/{} credentials",
          service_config.name, configured, total
        ));

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

    if found_services.is_empty() {
      bentley::info("No predefined services configured");
      bentley::info("Use 'secrets store <service> <key>' to configure individual credentials");
    }
  }

  Ok(())
}

async fn handle_clear(secrets: &Secrets, force: bool, quiet: bool) -> Result<()> {
  if !force {
    bentley::warn("⚠️  This will DELETE ALL CREDENTIALS from the vault!");
    bentley::warn("This action cannot be undone!");
    let confirm = rpassword::prompt_password("Type 'DELETE ALL' to confirm: ")?;
    if confirm.trim() != "DELETE ALL" {
      bentley::info("Cancelled - vault contents preserved");
      return Ok(());
    }
  }

  bentley::info("Clearing credential vault...");

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
      if secrets.get_credential_raw(&service_config.name, &cred_spec.key).is_ok()
        && secrets.delete_credential(&service_config.name, &cred_spec.key).is_ok()
      {
        cleared_count += 1;
      }
    }
  }

  // Note: We can't easily enumerate all arbitrary credentials with the keyring API
  // So we inform the user about this limitation
  if cleared_count > 0 {
    bentley::success(&format!("Cleared {cleared_count} predefined service credentials"));
  }

  bentley::info(
    "Note: Any arbitrary credentials (not from predefined services) must be deleted individually",
  );
  bentley::info("Use 'secrets delete <service> <key>' to remove specific credentials");

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
    bentley::info(&format!("Use 'secrets store {service_name} <key>' to configure missing credentials"));
  }

  Ok(())
}
