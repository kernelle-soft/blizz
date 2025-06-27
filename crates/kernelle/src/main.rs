use clap::{Parser, Subcommand};
use anyhow::Result;

mod commands;

#[derive(Parser)]
#[command(name = "kernelle")]
#[command(about = "Kernelle toolshed package manager and workflow orchestrator")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Add kernelle rules and workflows to a directory
    Add {
        /// Target directory (defaults to current directory)
        #[arg(default_value = ".")]
        dir: String,
    },
    /// Remove kernelle rules and workflows from a directory
    Remove {
        /// Target directory (defaults to current directory)
        #[arg(default_value = ".")]
        dir: String,
    },
    /// Manage daemon processes for MCPs
    Daemon {
        #[command(subcommand)]
        action: DaemonActions,
    },
    /// Store a credential or secret
    Store {
        /// The key to store the value under
        key: String,
        /// The value to store (will prompt if not provided)
        value: Option<String>,
    },
    /// Retrieve a stored credential or secret
    Retrieve {
        /// The key to retrieve
        key: String,
    },
}

#[derive(Subcommand)]
enum DaemonActions {
    /// Start daemon processes
    Up,
    /// Stop daemon processes
    Down,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Add { dir } => commands::add::execute(&dir).await,
        Commands::Remove { dir } => commands::remove::execute(&dir).await,
        Commands::Daemon { action } => match action {
            DaemonActions::Up => commands::daemon::up().await,
            DaemonActions::Down => commands::daemon::down().await,
        },
        Commands::Store { key, value } => commands::store::execute(&key, value.as_deref()).await,
        Commands::Retrieve { key } => commands::retrieve::execute(&key).await,
    }
} 