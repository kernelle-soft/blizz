use anyhow::Result;
use clap::{Parser, Subcommand};
use std::process;

mod commands;

#[derive(Parser)]
#[command(name = "kernelle")]
#[command(about = "It takes a village.

Kernelle is a tool for managing projects from a personal perspective, and enabling them to work together with AI agents.
")]
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
  /// Run a task from the tasks file
  Do {
    /// The task name to run
    name: String,
    /// Arguments to pass to the task
    #[arg(trailing_var_arg = true)]
    args: Vec<String>,
    /// Run task silently (no output streaming)
    #[arg(long)]
    silent: bool,
    /// Path to tasks file
    #[arg(long, short = 'f')]
    file: Option<String>,
    /// Force enable colored output (overrides CI detection)
    #[arg(long)]
    color: bool,
    /// Force disable colored output
    #[arg(long)]
    no_color: bool,
  },
  /// List available tasks
  Tasks {
    /// Path to tasks file
    #[arg(long, short = 'f')]
    file: Option<String>,
    /// Show task commands as well as names
    #[arg(long)]
    verbose: bool,
  },
  /// Show version information
  Version {
    /// List all available releases
    #[arg(long)]
    list: bool,
  },
  /// Update kernelle to the latest or specified version
  Update {
    /// Specific version to update to (defaults to latest)
    #[arg(long, short)]
    version: Option<String>,
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
    Commands::Do { name, args, silent, file, color, no_color } => {
      execute_task(&name, &args, silent, file, color, no_color).await
    }
    Commands::Tasks { file, verbose } => list_tasks(file, verbose).await,
    Commands::Version { list } => commands::version::execute(list).await,
    Commands::Update { version } => {
      if let Err(err) = commands::update::execute(version.as_deref()).await {
        // Print nice message for VersionNotFound
        if let Some(commands::update::UpdateError::VersionNotFound { version }) =
          err.downcast_ref::<commands::update::UpdateError>()
        {
          eprintln!("Error: Version '{version}' not found");
          process::exit(1);
        }

        // For all other errors (typed or not), print the error and exit non-zero
        eprintln!("{err}");
        process::exit(1);
      }
      Ok(())
    }
  }
}

async fn execute_task(
  name: &str,
  args: &[String],
  silent: bool,
  file: Option<String>,
  color: bool,
  no_color: bool,
) -> Result<()> {
  let options = commands::r#do::TaskRunnerOptions {
    silent,
    tasks_file_path: file,
    force_color: color,
    no_color,
  };

  let result = commands::r#do::run_task(name, args, options).await?;

  if !result.success {
    if let Some(exit_code) = result.exit_code {
      process::exit(exit_code);
    } else {
      process::exit(1);
    }
  }

  Ok(())
}

async fn list_tasks(file: Option<String>, verbose: bool) -> Result<()> {
  if verbose {
    let tasks_file = commands::r#do::get_tasks_file(file).await?;
    println!("Available tasks:");

    // Find the longest task name for alignment
    let max_name_length = tasks_file.keys().map(|name| name.len()).max().unwrap_or(0);

    // Sort tasks for consistent output
    let mut sorted_tasks: Vec<_> = tasks_file.iter().collect();
    sorted_tasks.sort_by_key(|(name, _)| *name);

    for (name, command) in sorted_tasks {
      let dots_count = max_name_length - name.len() + 4; // +4 for some padding
      let dots = "·".repeat(dots_count);
      let command_display = command.to_command_string();
      println!("• {name} {dots} {command_display}");
    }
  } else {
    let mut tasks = commands::r#do::list_tasks(file).await?;
    tasks.sort(); // Sort for consistent output
    println!("Available tasks:");
    for task in tasks {
      println!("• {task}");
    }
  }

  Ok(())
}
