use crate::Secrets;
use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use std::env;
use std::io::Write;
use std::path::{Path, PathBuf};
use tokio::net::UnixStream;
use tokio::time::{sleep, Duration};

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
  /// Retrieve/read a secret entry
  Read {
    /// Secret name/key
    name: String,
    /// Group/namespace for the secret (defaults to 'general')
    #[arg(short, long)]
    group: Option<String>,
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
    #[arg(short, long)]
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
  /// Reset master password (re-encrypts all secrets)
  ResetPassword {
    /// Skip confirmation prompt
    #[arg(long)]
    force: bool,
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
    Commands::ResetPassword { force } => {
      handle_reset_password(&secrets, force).await?;
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

/// Helper function to get master password, first trying daemon, then fallback to direct prompt
async fn get_master_password(_secrets: &Secrets) -> Result<String> {
  // Check if credentials file exists
  let base_path = if let Ok(kernelle_dir) = std::env::var("KERNELLE_DIR") {
    PathBuf::from(kernelle_dir)
  } else {
    dirs::home_dir().unwrap_or_else(|| std::env::current_dir().unwrap()).join(".kernelle")
  };

  let mut credentials_path = base_path.clone();
  credentials_path.push("persistent");
  credentials_path.push("keeper");
  credentials_path.push("credentials.enc");

  // Determine if this is a new vault setup
  let is_new_vault = !credentials_path.exists();

  if is_new_vault {
    // New vault - create master password (daemon not relevant for setup)
    return setup_new_vault().await;
  }

  // Existing vault - try to get password from daemon first
  match get_password_from_daemon(&base_path).await {
    Ok(password) => {
      bentley::verbose("retrieved password from daemon");
      Ok(password)
    }
    Err(_) => {
      // Daemon not available - start it and try again
      bentley::verbose("daemon not available, starting...");
      start_daemon_if_needed(&base_path).await?;

      // Try daemon again after starting
      match get_password_from_daemon(&base_path).await {
        Ok(password) => {
          bentley::verbose("retrieved password from daemon after startup");
          Ok(password)
        }
        Err(_) => {
          // Last resort - prompt directly
          bentley::verbose("daemon unavailable, prompting directly");
          prompt_for_existing_vault_password().await
        }
      }
    }
  }
}

/// Setup new vault with password confirmation
async fn setup_new_vault() -> Result<String> {
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

/// Prompt for password for existing vault
async fn prompt_for_existing_vault_password() -> Result<String> {
  bentley::info("enter master password:");
  print!("> ");
  std::io::stdout().flush()?;
  let password = rpassword::read_password()?;
  if password.trim().is_empty() {
    return Err(anyhow!("master password cannot be empty"));
  }
  Ok(password.trim().to_string())
}

/// Try to get password from running daemon
async fn get_password_from_daemon(base_path: &Path) -> Result<String> {
  let socket_path = base_path.join("persistent").join("keeper").join("keeper.sock");

  if !socket_path.exists() {
    return Err(anyhow!("daemon socket not found"));
  }

  let mut stream = UnixStream::connect(&socket_path)
    .await
    .map_err(|e| anyhow!("failed to connect to daemon: {}", e))?;

  use tokio::io::{AsyncReadExt, AsyncWriteExt};

  // Send GET request to daemon (with newline for protocol compatibility)
  stream
    .write_all(b"GET\n")
    .await
    .map_err(|e| anyhow!("failed to send request to daemon: {}", e))?;

  // Read password response
  let mut password = String::new();
  stream
    .read_to_string(&mut password)
    .await
    .map_err(|e| anyhow!("failed to read response from daemon: {}", e))?;

  let password = password.trim();
  if password.is_empty() {
    return Err(anyhow!("daemon returned empty password"));
  }

  Ok(password.to_string())
}

/// Start daemon if not running and wait for it to be ready
async fn start_daemon_if_needed(base_path: &Path) -> Result<()> {
  let socket_path = base_path.join("persistent").join("keeper").join("keeper.sock");
  let pid_file = base_path.join("persistent").join("keeper").join("keeper.pid");
  let keeper_path = base_path.join("persistent").join("keeper");

  // If socket already exists, daemon might be running
  if socket_path.exists() {
    return Ok(());
  }

  bentley::info("starting daemon...");
  start_agent(&socket_path, &pid_file, &keeper_path).await?;

  Ok(())
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
  // Get the credentials file path
  let base_path = if let Ok(kernelle_dir) = std::env::var("KERNELLE_DIR") {
    PathBuf::from(kernelle_dir)
  } else {
    dirs::home_dir().unwrap_or_else(|| std::env::current_dir().unwrap()).join(".kernelle")
  };

  let mut credentials_path = base_path.clone();
  credentials_path.push("persistent");
  credentials_path.push("keeper");
  credentials_path.push("credentials.enc");

  // Check if credentials file exists
  if !credentials_path.exists() {
    bentley::error(&format!("Secret not found: {group}/{name}"));
    std::process::exit(1);
  }

  // Load the encrypted store from file
  use crate::PasswordBasedCredentialStore;
  let store = match PasswordBasedCredentialStore::load_from_file(&credentials_path)? {
    Some(store) => store,
    None => {
      bentley::warn(&format!("secret not found: {group}/{name}"));
      std::process::exit(1);
    }
  };

  // Get master password using daemon integration
  let master_password = get_master_password(secrets).await?;

  // Decrypt all credentials
  let all_credentials = match store.decrypt_credentials(&master_password) {
    Ok(creds) => creds,
    Err(_) => {
      bentley::error("invalid master password or corrupted data");
      std::process::exit(1);
    }
  };

  // Look for the specific secret
  match all_credentials.get(group).and_then(|group_secrets| group_secrets.get(name)) {
    Some(value) => {
      println!("{value}");
    }
    None => {
      bentley::warn(&format!("secret not found: {group}/{name}"));
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
  // Get the credentials file path
  let base_path = if let Ok(kernelle_dir) = std::env::var("KERNELLE_DIR") {
    PathBuf::from(kernelle_dir)
  } else {
    dirs::home_dir().unwrap_or_else(|| std::env::current_dir().unwrap()).join(".kernelle")
  };

  let mut credentials_path = base_path.clone();
  credentials_path.push("persistent");
  credentials_path.push("keeper");
  credentials_path.push("credentials.enc");

  // Check if credentials file exists
  if !credentials_path.exists() {
    bentley::error("No secrets stored yet");
    return Ok(());
  }

  // Get master password using daemon integration
  let master_password = get_master_password(secrets).await?;

  // Load the encrypted store from file
  use crate::PasswordBasedCredentialStore;
  let store = match PasswordBasedCredentialStore::load_from_file(&credentials_path)? {
    Some(store) => store,
    None => {
      bentley::error("No secrets found");
      return Ok(());
    }
  };

  // Decrypt all credentials
  let mut all_credentials = match store.decrypt_credentials(&master_password) {
    Ok(creds) => creds,
    Err(_) => {
      bentley::error("Invalid master password or corrupted data");
      return Ok(());
    }
  };

  if let Some(name) = name {
    // Delete specific secret
    let secret_exists =
      all_credentials.get(group).is_some_and(|group_secrets| group_secrets.contains_key(&name));

    if !secret_exists {
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

    // Remove the secret
    if let Some(group_secrets) = all_credentials.get_mut(group) {
      group_secrets.remove(&name);

      // Remove the group entirely if no credentials left
      if group_secrets.is_empty() {
        all_credentials.remove(group);
      }
    }

    // Save updated credentials back to file
    let updated_store = PasswordBasedCredentialStore::new(&all_credentials, &master_password)?;
    updated_store.save_to_file(&credentials_path)?;

    bentley::success(&format!("Deleted secret: {group}/{name}"));
  } else {
    // Delete all secrets for group
    if !all_credentials.contains_key(group) {
      bentley::info(&format!("No secrets found for group: {group}"));
      return Ok(());
    }

    if !force {
      bentley::warn(&format!("This will delete ALL secrets for group: {group}"));
      let confirm = rpassword::prompt_password("Type 'yes' to confirm: ")?;
      if confirm.trim().to_lowercase() != "yes" {
        bentley::info("Cancelled");
        return Ok(());
      }
    }

    // Count secrets before deletion
    let secret_count = all_credentials.get(group).map_or(0, |secrets| secrets.len());

    // Remove the entire group
    all_credentials.remove(group);

    // Save updated credentials back to file
    let updated_store = PasswordBasedCredentialStore::new(&all_credentials, &master_password)?;
    updated_store.save_to_file(&credentials_path)?;

    bentley::success(&format!("Deleted {secret_count} secrets for group: {group}"));
  }

  Ok(())
}

async fn handle_list(
  secrets: &Secrets,
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

  let mut credentials_path = base_path.clone();
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

  // Get master password using daemon integration
  let master_password = get_master_password(secrets).await?;

  // Decrypt all credentials
  let all_credentials = match store.decrypt_credentials(&master_password) {
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
      bentley::info(&format!("no secrets found for group: {filter}"));
    } else {
      bentley::info("no secrets found");
    }
    return Ok(());
  }

  // Display format depends on show_keys flag
  if show_keys {
    // Show detailed view with group/key pairs
    for (group, secrets_map) in credentials_to_show {
      bentley::info(&format!("\n{group}/"));
      for key in secrets_map.keys() {
        bentley::info(&format!("   {group}/{key}"));
      }
    }
  } else {
    // Show summary view with just groups and counts
    for (group, secrets_map) in credentials_to_show {
      let count = secrets_map.len();
      let plural = if count == 1 { "secret" } else { "secrets" };
      bentley::info(&format!("{group}: {count} {plural}"));
    }

    if !quiet {
      bentley::info("\nuse --keys to see individual secret names");
    }
  }

  Ok(())
}

async fn handle_clear(secrets: &Secrets, force: bool, quiet: bool) -> Result<()> {
  bentley::warn("this will DELETE ALL SECRETS from the vault");
  bentley::warn("this action cannot be undone!");

  // If not forced, ask for confirmation
  if !force {
    bentley::info("type 'yes' to confirm vault clearing:");
    print!("> ");
    std::io::stdout().flush()?;
    let mut confirm = String::new();
    std::io::stdin().read_line(&mut confirm)?;
    if confirm.trim().to_lowercase() != "yes" {
      bentley::info("cancelled - vault contents preserved");
      return Ok(());
    }
  }

  // Get master password using daemon integration for verification
  let master_password = get_master_password(secrets).await?;

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
      match store.decrypt_credentials(&master_password) {
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
    let empty_store = PasswordBasedCredentialStore::new(&empty_credentials, &master_password)?;
    empty_store.save_to_file(&credentials_path)?;
  } else {
    bentley::info("no action taken - nothing to clear");
  }

  if !quiet {
    bentley::success("vault cleared");
  }

  Ok(())
}

async fn handle_reset_password(secrets: &Secrets, force: bool) -> Result<()> {
  bentley::verbose("resetting master password...");

  // Get the current master password from the daemon
  let current_password = get_master_password(secrets).await?;

  // Load current credentials store
  let base_path = if let Ok(kernelle_dir) = std::env::var("KERNELLE_DIR") {
    PathBuf::from(kernelle_dir)
  } else {
    dirs::home_dir().unwrap_or_else(|| std::env::current_dir().unwrap()).join(".kernelle")
  };

  let mut credentials_path = base_path;
  credentials_path.push("persistent");
  credentials_path.push("keeper");
  credentials_path.push("credentials.enc");

  if !credentials_path.exists() {
    return Err(anyhow::anyhow!("No vault exists to reset password for"));
  }

  // Load existing credentials with current password
  use crate::PasswordBasedCredentialStore;
  let existing_store = match PasswordBasedCredentialStore::load_from_file(&credentials_path)? {
    Some(store) => store,
    None => {
      return Err(anyhow::anyhow!("No vault exists to reset password for"));
    }
  };

  // Decrypt all credentials with current password
  let credentials = match existing_store.decrypt_credentials(&current_password) {
    Ok(creds) => creds,
    Err(_) => {
      return Err(anyhow::anyhow!("Failed to decrypt vault with current password"));
    }
  };

  if !force {
    eprintln!("This will re-encrypt all secrets with a new master password.");
    eprintln!("You currently have {} secret(s) stored.", credentials.len());
    eprint!("Are you sure you want to continue? (y/N): ");
    std::io::stdout().flush()?;

    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    let input = input.trim().to_lowercase();

    if input != "y" && input != "yes" {
      bentley::info("password reset cancelled");
      return Ok(());
    }
  }

  // Prompt for new password
  eprintln!("Enter new master password:");
  let new_password = rpassword::read_password()?;

  if new_password.is_empty() {
    return Err(anyhow::anyhow!("Password cannot be empty"));
  }

  // Confirm new password
  eprintln!("Confirm new master password:");
  let confirm_password = rpassword::read_password()?;

  if new_password != confirm_password {
    return Err(anyhow::anyhow!("Passwords do not match"));
  }

  // Create new encrypted store with new password
  let new_store = PasswordBasedCredentialStore::new(&credentials, &new_password)?;
  new_store.save_to_file(&credentials_path)?;

  bentley::success("master password reset successfully");
  bentley::info("please restart the daemon for the new password to take effect");

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
      start_agent(&socket_path, &pid_file, &keeper_path).await?;
    }

    AgentAction::Status => {
      get_agent_status(&socket_path).await?;
    }

    AgentAction::Stop => {
      stop_agent(&socket_path, &pid_file).await?;
    }

    AgentAction::Restart => {
      restart_agent(&socket_path, &pid_file, &keeper_path).await?;
    }
  }

  Ok(())
}

async fn start_agent(
  socket_path: &std::path::Path,
  pid_file: &std::path::Path,
  keeper_path: &std::path::Path,
) -> Result<()> {
  use std::{fs, process::Command};

  // Check if already running
  if socket_path.exists() {
    bentley::warn("agent appears to already be running");
    bentley::info("use 'secrets agent status' to check or 'secrets agent restart' to restart");
    return Ok(());
  }

  bentley::info("starting agent...");

  // Spawn keeper binary as background process
  let output = Command::new("keeper").spawn();

  match output {
    Ok(mut child) => {
      fs::create_dir_all(keeper_path)?;
      fs::write(pid_file, child.id().to_string())?;

      // Wait for socket to be created (indicates successful startup)
      // We'll wait indefinitely since password entry can take time
      loop {
        // Check if process exited unexpectedly
        if let Ok(Some(status)) = child.try_wait() {
          let _ = fs::remove_file(pid_file);
          if status.success() {
            bentley::error("keeper process exited unexpectedly");
          } else {
            bentley::error("keeper process failed to start");
          }
          return Ok(());
        }

        // Check if socket exists
        if socket_path.exists() {
          bentley::success("agent started successfully");
          return Ok(());
        }

        // Short sleep to avoid busy waiting
        sleep(Duration::from_millis(100)).await;
      }
    }
    Err(e) => {
      bentley::error(&format!("failed to start agent: {e}"));
      bentley::info("make sure the 'keeper' binary is in your PATH");
    }
  }

  Ok(())
}

async fn get_agent_status(socket_path: &std::path::Path) -> Result<()> {
  if !socket_path.exists() {
    bentley::info("agent is not running");
    bentley::info("use 'secrets agent start' to start the daemon");
    return Ok(());
  }

  match UnixStream::connect(&socket_path).await {
    Ok(mut stream) => {
      use tokio::io::{AsyncReadExt, AsyncWriteExt};
      if (stream.write_all(b"GET\n").await).is_err() {
        bentley::warn("socket exists but failed to communicate");
        return Ok(());
      }

      let mut response = String::new();
      if stream.read_to_string(&mut response).await.is_ok() && !response.trim().is_empty() {
        bentley::success("keeper is running and responsive");
      } else {
        bentley::error("keeper is running but not responding correctly");
      }
    }
    Err(_) => {
      bentley::error("socket file exists but connection failed");
      bentley::error("agent may be starting up or in bad state");
    }
  }

  Ok(())
}

async fn stop_agent(socket_path: &std::path::Path, pid_file: &std::path::Path) -> Result<()> {
  use std::{fs, process::Command};

  if !socket_path.exists() {
    bentley::info("agent is not running");
    return Ok(());
  }

  bentley::info("stopping agent...");

  if !pid_file.exists() {
    bentley::warn("PID file not found, cleaning up socket");
    let _ = fs::remove_file(socket_path);
    return Ok(());
  }

  let pid_str = fs::read_to_string(pid_file).ok();

  if !pid_file.exists() || pid_str.is_none() {
    bentley::warn("PID file not found or unreadable, cleaning up socket");
    let _ = fs::remove_file(socket_path);
    return Ok(());
  }

  let pid: u32 = pid_str.unwrap().trim().parse().unwrap_or(0);
  if pid == 0 {
    bentley::warn("invalid PID, cleaning up socket");
    let _ = fs::remove_file(socket_path);
    return Ok(());
  }

  let output = Command::new("kill").arg(pid.to_string()).output();
  match output {
    Ok(result) if result.status.success() => {
      // Wait a moment for graceful shutdown
      sleep(Duration::from_millis(500)).await;

      // Clean up files
      let _ = fs::remove_file(socket_path);
      let _ = fs::remove_file(pid_file);

      bentley::success("agent stopped");
    }
    _ => {
      bentley::warn("failed to stop agent gracefully, cleaning up files");
      let _ = fs::remove_file(socket_path);
      let _ = fs::remove_file(pid_file);
    }
  }

  Ok(())
}

async fn restart_agent(
  socket_path: &std::path::Path,
  pid_file: &std::path::Path,
  keeper_path: &std::path::Path,
) -> Result<()> {
  if socket_path.exists() {
    stop_agent(socket_path, pid_file).await?;
    sleep(Duration::from_millis(1000)).await;
  }

  start_agent(socket_path, pid_file, keeper_path).await?;

  Ok(())
}
