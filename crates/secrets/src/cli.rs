use crate::Secrets;
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
pub enum AgentAction {
  /// Start daemon, prompt for password once
  Start,
  /// Check daemon status and key validity  
  Status,
  /// Stop daemon, clear key from memory
  Stop,
  /// Restart daemon
  Restart,
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
    #[arg(short, long)]
    force: bool,
  },
  /// Retrieve/read a secret entry
  Read {
    /// Secret name/key
    name: String,
    /// Group/namespace for the secret (defaults to 'general')
    #[arg(short, long)]
    group: Option<String>,
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
  /// Daemon management commands
  Agent {
    #[command(subcommand)]
    action: AgentAction,
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
    Commands::Read { name, group } => {
      let group = group.unwrap_or_else(|| "general".to_string());
      handle_read(&secrets, &group, &name).await?;
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
    Commands::Agent { action } => {
      handle_agent(action).await?;
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

/// Helper function to get master password once, handling both existing and new vault scenarios
async fn get_master_password(_secrets: &Secrets) -> Result<String> {
  // Check if credentials file exists
  let base_path = if let Ok(kernelle_dir) = std::env::var("KERNELLE_DIR") {
    PathBuf::from(kernelle_dir)
  } else {
    dirs::home_dir().unwrap_or_else(|| std::env::current_dir().unwrap()).join(".kernelle")
  };

  let mut credentials_path = base_path;
  credentials_path.push("persistent");
  credentials_path.push("keeper");
  credentials_path.push("credentials.enc");

  if credentials_path.exists() {
    // Existing vault - just get password
    bentley::info("enter master password:");
    print!("> ");
    std::io::stdout().flush()?;
    let password = rpassword::read_password()?;
    if password.trim().is_empty() {
      return Err(anyhow!("master password cannot be empty"));
    }
    Ok(password.trim().to_string())
  } else {
    // New vault - create master password
    bentley::info("setting up vault - create master password:");
    print!("> ");
    std::io::stdout().flush()?;
    let password1 = rpassword::read_password()?;
    if password1.trim().is_empty() {
      return Err(anyhow!("master password cannot be empty"));
    }

    bentley::info("confirm master password:");
    print!("> ");
    std::io::stdout().flush()?;
    let password2 = rpassword::read_password()?;

    if password1 != password2 {
      return Err(anyhow!("passwords do not match"));
    }

    Ok(password1.trim().to_string())
  }
}

async fn handle_store(
  _secrets: &Secrets,
  group: &str,
  name: &str,
  value: Option<String>,
  force: bool,
) -> Result<()> {
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

  // Get master password once
  let master_password = get_master_password(_secrets).await?;

  // Load existing credentials or create new ones
  let base_path = if let Ok(kernelle_dir) = std::env::var("KERNELLE_DIR") {
    PathBuf::from(kernelle_dir)
  } else {
    dirs::home_dir().unwrap_or_else(|| std::env::current_dir().unwrap()).join(".kernelle")
  };

  let mut credentials_path = base_path;
  credentials_path.push("persistent");
  credentials_path.push("keeper");
  credentials_path.push("credentials.enc");

  // Load existing credentials or start with empty
  let mut all_credentials = if credentials_path.exists() {
    use crate::PasswordBasedCredentialStore;
    if let Some(store) = PasswordBasedCredentialStore::load_from_file(&credentials_path)? {
      match store.decrypt_credentials(&master_password) {
        Ok(creds) => creds,
        Err(_) => {
          bentley::error("invalid master password");
          return Ok(());
        }
      }
    } else {
      std::collections::HashMap::new()
    }
  } else {
    std::collections::HashMap::new()
  };

  // Check if secret already exists (now that we have the credentials loaded)
  if !force {
    if let Some(group_secrets) = all_credentials.get(group) {
      if group_secrets.contains_key(name) {
        bentley::warn(&format!("Secret {group}/{name} already exists"));
        bentley::info("Use --force to overwrite existing secret");
        return Ok(());
      }
    }
  }

  // Add/update the secret
  all_credentials
    .entry(group.to_string())
    .or_default()
    .insert(name.to_string(), secret_value.trim().to_string());

  // Save back to file
  use crate::PasswordBasedCredentialStore;
  let store = PasswordBasedCredentialStore::new(&all_credentials, &master_password)?;
  store.save_to_file(&credentials_path)?;

  bentley::success(&format!("Stored secret: {group}/{name}"));
  Ok(())
}

async fn handle_read(secrets: &Secrets, group: &str, name: &str) -> Result<()> {
  match secrets.get_secret_raw_no_setup(group, name) {
    Ok(value) => {
      println!("{value}");
    }
    Err(_) => {
      bentley::error(&format!("❌ Secret not found: {group}/{name}"));
      std::process::exit(1);
    }
  }
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
    bentley::info("no secrets stored yet");
    return Ok(());
  }

  // Load the encrypted store from file
  use crate::PasswordBasedCredentialStore;
  let store = match PasswordBasedCredentialStore::load_from_file(&credentials_path)? {
    Some(store) => store,
    None => {
      bentley::info("no secrets found");
      return Ok(());
    }
  };

  // Prompt for master password
  bentley::info("enter master password:");
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
    bentley::info("vault is empty");
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
      bentley::info(&format!("no secrets found for group: {}", filter));
    } else {
      bentley::info("no secrets found");
    }
    return Ok(());
  }

  // Display format depends on show_keys flag
  if show_keys {
    // Show detailed view with group/key pairs
    for (group, secrets_map) in credentials_to_show {
      bentley::info(&format!("\n📁 {}/", group));
      for key in secrets_map.keys() {
        bentley::info(&format!("   🔑 {}/{}", group, key));
      }
    }
  } else {
    // Show summary view with just groups and counts
    for (group, secrets_map) in credentials_to_show {
      let count = secrets_map.len();
      let plural = if count == 1 { "secret" } else { "secrets" };
      bentley::info(&format!("📁 {}: {} {}", group, count, plural));
    }

    if !quiet {
      bentley::info("\nuse --keys to see individual secret names");
    }
  }

  Ok(())
}

async fn handle_clear(_secrets: &Secrets, _force: bool, quiet: bool) -> Result<()> {
  bentley::warn("this will DELETE ALL SECRETS from the vault");
  bentley::warn("this action cannot be undone!");
  bentley::info("enter master password to confirm:");
  print!("> ");
  std::io::stdout().flush()?;
  let master_password = rpassword::read_password()?;

  if master_password.trim().is_empty() {
    bentley::info("cancelled - vault contents preserved");
    return Ok(());
  }

  // Try to verify the password by attempting to decrypt existing secrets
  let base_path = if let Ok(kernelle_dir) = std::env::var("KERNELLE_DIR") {
    PathBuf::from(kernelle_dir)
  } else {
    dirs::home_dir().unwrap_or_else(|| std::env::current_dir().unwrap()).join(".kernelle")
  };

  let mut credentials_path = base_path;
  credentials_path.push("persistent");
  credentials_path.push("keeper");
  credentials_path.push("credentials.enc");

  if credentials_path.exists() {
    use crate::PasswordBasedCredentialStore;
    if let Some(store) = PasswordBasedCredentialStore::load_from_file(&credentials_path)? {
      match store.decrypt_credentials(master_password.trim()) {
        Ok(_) => {
          // Password verified successfully
        }
        Err(_) => {
          bentley::error("invalid master password - vault contents preserved");
          return Ok(());
        }
      }
    }
  }

  bentley::verbose("clearing vault...");

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

  if credentials_path.exists() {
    // Create empty credentials structure
    let empty_credentials = std::collections::HashMap::new();

    // Create a new encrypted store with empty credentials
    use crate::PasswordBasedCredentialStore;
    let empty_store =
      PasswordBasedCredentialStore::new(&empty_credentials, master_password.trim())?;
    empty_store.save_to_file(&credentials_path)?;
  } else {
    bentley::info("no action taken - nothing to clear");
  }

  if !quiet {
    bentley::success("vault cleared");
  }
  
  Ok(())
}

async fn handle_agent(action: AgentAction) -> Result<()> {
  use dirs;

  let base = if let Ok(dir) = std::env::var("KERNELLE_HOME") {
    std::path::PathBuf::from(dir)
  } else {
    dirs::home_dir()
      .ok_or_else(|| anyhow::anyhow!("Failed to determine home directory"))?
      .join(".kernelle")
  };

  let keeper_path = base.join("persistent").join("keeper");
  let socket_path = keeper_path.join("keeper.sock");
  let pid_file = keeper_path.join("keeper.pid");

  match action {
    AgentAction::Start => {
      start_daemon(&socket_path, &pid_file, &keeper_path).await?;
    }

    AgentAction::Status => {
      if socket_path.exists() {
        // Try to connect and test the socket
        use std::io::{Read, Write};
        use std::os::unix::net::UnixStream;

        match UnixStream::connect(&socket_path) {
          Ok(mut stream) => {
            if let Err(_) = stream.write_all(b"GET\n") {
              bentley::warn("⚠️ Socket exists but failed to communicate");
              return Ok(());
            }

            let mut response = String::new();
            if stream.read_to_string(&mut response).is_ok() && !response.trim().is_empty() {
              bentley::success("✅ Keeper daemon is running and responsive");
              bentley::info("🔑 Master key is cached and available");
            } else {
              bentley::warn("⚠️ Keeper daemon is running but not responding correctly");
            }
          }
          Err(_) => {
            bentley::warn("⚠️ Socket file exists but connection failed");
            bentley::info("Daemon may be starting up or in bad state");
          }
        }
      } else {
        bentley::info("❌ Keeper daemon is not running");
        bentley::info("Use 'secrets agent start' to start the daemon");
      }
    }

    AgentAction::Stop => {
      stop_daemon(&socket_path, &pid_file).await?;
    }

    AgentAction::Restart => {
      bentley::info("Restarting keeper daemon...");

      // Stop first
      if socket_path.exists() {
        stop_daemon(&socket_path, &pid_file).await?;
        // Give it a moment to fully stop
        tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
      }

      // Then start
      start_daemon(&socket_path, &pid_file, &keeper_path).await?;
    }
  }

  Ok(())
}

async fn start_daemon(
  socket_path: &std::path::Path,
  pid_file: &std::path::Path,
  keeper_path: &std::path::Path,
) -> Result<()> {
  use std::{fs, process::Command};

  // Check if already running
  if socket_path.exists() {
    bentley::warn("Keeper daemon appears to already be running");
    bentley::info("Use 'secrets agent status' to check or 'secrets agent restart' to restart");
    return Ok(());
  }

  bentley::info("Starting keeper daemon...");

  // Spawn keeper binary as background process
  let output = Command::new("keeper").spawn();

  match output {
    Ok(child) => {
      // Store PID for later reference
      fs::create_dir_all(&keeper_path)?;
      fs::write(&pid_file, child.id().to_string())?;

      // Give it a moment to start
      tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

      if socket_path.exists() {
        bentley::success("✅ Keeper daemon started successfully");
      } else {
        bentley::error("❌ Keeper daemon failed to start (no socket created)");
        let _ = fs::remove_file(&pid_file);
      }
    }
    Err(e) => {
      bentley::error(&format!("❌ Failed to start keeper daemon: {}", e));
      bentley::info("Make sure the 'keeper' binary is in your PATH");
    }
  }

  Ok(())
}

async fn stop_daemon(socket_path: &std::path::Path, pid_file: &std::path::Path) -> Result<()> {
  use std::{fs, process::Command};

  if !socket_path.exists() {
    bentley::info("Keeper daemon is not running");
    return Ok(());
  }

  bentley::info("Stopping keeper daemon...");

  // Try to read PID and kill process
  if let Ok(pid_str) = fs::read_to_string(&pid_file) {
    if let Ok(pid) = pid_str.trim().parse::<u32>() {
      // Send SIGTERM
      let output = Command::new("kill").arg(pid.to_string()).output();

      match output {
        Ok(result) if result.status.success() => {
          // Wait a moment for graceful shutdown
          tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

          // Clean up files
          let _ = fs::remove_file(&socket_path);
          let _ = fs::remove_file(&pid_file);

          bentley::success("✅ Keeper daemon stopped");
        }
        _ => {
          bentley::warn("⚠️ Failed to stop daemon gracefully, cleaning up files");
          let _ = fs::remove_file(&socket_path);
          let _ = fs::remove_file(&pid_file);
        }
      }
    }
  } else {
    // No PID file, just clean up socket
    let _ = fs::remove_file(&socket_path);
    bentley::success("✅ Cleaned up daemon files");
  }

  Ok(())
}
