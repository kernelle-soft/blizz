use crate::Secrets;
use anyhow::Result;
use clap::{Parser, Subcommand};
use std::env;
use crate::commands;
use crate::keeper_client;

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
      commands::store(&secrets, &group, &name, value, force).await?;
    }
    Commands::Read { name, group } => {
      let group = group.unwrap_or_else(|| "general".to_string());
      commands::read(&secrets, &group, &name).await?;
    }
    Commands::Delete { name, group, force } => {
      let group = group.unwrap_or_else(|| "general".to_string());
      commands::delete(&secrets, &group, Some(name), force).await?;
    }
    Commands::List { group, keys } => {
      commands::list(&secrets, group, keys, quiet_mode).await?;
    }
    Commands::Clear { force } => {
      commands::clear(&secrets, force, quiet_mode).await?;
    }
    Commands::Agent { action } => {
      handle_agent(action).await?;
    }
    Commands::ResetPassword { force } => {
      commands::reset_password(&secrets, force).await?;
    }
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
      keeper_client::start(&socket_path, &pid_file, &keeper_path).await?;
    }

    AgentAction::Status => {
      keeper_client::status(&socket_path).await?;
    }

    AgentAction::Stop => {
      keeper_client::stop(&socket_path, &pid_file).await?;
    }

    AgentAction::Restart => {
      keeper_client::restart(&socket_path, &pid_file, &keeper_path).await?;
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