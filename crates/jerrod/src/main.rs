use anyhow::Result;
use clap::{Parser, Subcommand};

mod commands;
mod display;
mod github;
mod platform;
mod session;

#[derive(Parser)]
#[command(name = "jerrod")]
#[command(
  about = "GitLab and GitHub merge request review tool - the reliable guardian of code quality"
)]
struct Cli {
  /// GitHub personal access token (or use GITHUB_TOKEN env var)
  #[arg(long, env = "GITHUB_TOKEN")]
  github_token: Option<String>,

  /// GitLab personal access token (or use GITLAB_TOKEN env var)
  #[arg(long, env = "GITLAB_TOKEN")]
  gitlab_token: Option<String>,

  #[command(subcommand)]
  command: Commands,
}

#[derive(Subcommand)]
enum Commands {
  /// Start a new merge request review session
  Start {
    /// Repository URL or "owner/repo" format
    repository: String,
    /// Merge request/pull request number
    mr_number: u64,
    /// Platform (github or gitlab) - auto-detected if not specified
    #[arg(short, long)]
    platform: Option<String>,
  },
  /// Show current review session status
  Status,
  /// View the next thread in the review queue
  Peek,
  /// Remove a thread from the review queue
  Pop {
    /// Mark thread as unresolved for later follow-up
    #[arg(long)]
    unresolved: bool,
  },
  /// Add a comment to a thread or MR
  Comment {
    /// Comment text
    text: String,
    /// Create a new MR-level comment instead of replying to current thread
    #[arg(long)]
    new: bool,
  },
  /// Commit changes with automatic MR/thread linking
  Commit {
    /// Commit message
    message: String,
    /// Optional detailed description
    #[arg(short, long)]
    details: Option<String>,
    /// Thread ID being addressed (optional)
    #[arg(short, long)]
    thread_id: Option<String>,
  },
  /// Mark the current thread as resolved
  Resolve {
    /// Optional comment to add with linkback
    #[arg(short, long)]
    comment: Option<String>,
  },
  /// Acknowledge a non-actionable comment
  Acknowledge,
  /// Defer a comment to separate task
  Defer {
    /// Optional comment about the separate task
    #[arg(short, long)]
    comment: Option<String>,
  },
  /// Mark a comment as having a follow-up question
  Question {
    /// Required comment with question/clarification
    comment: String,
  },
  /// Finish the review session
  Finish,
  /// Refresh session data (clean and re-download)
  Refresh,
}

#[tokio::main]
async fn main() -> Result<()> {
  let cli = Cli::parse();

  match cli.command {
    Commands::Start { repository, mr_number, platform } => {
      commands::start::handle(repository, mr_number, platform, cli.github_token, cli.gitlab_token)
        .await
    }
    Commands::Status => commands::status::handle().await,
    Commands::Peek => commands::peek::handle().await,
    Commands::Pop { unresolved } => commands::pop::handle(unresolved).await,
    Commands::Comment { text, new } => commands::comment::handle(text, new).await,
    Commands::Commit { message, details, thread_id } => commands::commit::handle(message, details, thread_id).await,
    Commands::Resolve { comment } => commands::resolve::handle(comment).await,
    Commands::Acknowledge => commands::acknowledge::handle().await,
    Commands::Defer { comment } => commands::defer::handle(comment).await,
    Commands::Question { comment } => commands::question::handle(comment).await,
    Commands::Finish => commands::finish::handle().await,
    Commands::Refresh => commands::refresh::handle().await,
  }
}
