use anyhow::Result;
use clap::{Parser, Subcommand};

mod auth;
mod commands;
mod display;
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
  /// Mark the current thread as resolved
  Resolve,
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
  /// Acknowledge a comment with reaction
  Acknowledge {
    /// 👍 reaction flags
    #[arg(long)]
    thumbs_up: bool,
    #[arg(long)]
    ok: bool,
    #[arg(long)]
    yeah: bool,
    #[arg(long)]
    got_it: bool,

    /// 👎 reaction flags  
    #[arg(long)]
    thumbs_down: bool,
    #[arg(long)]
    f_you: bool,

    /// 😄 reaction flags
    #[arg(long)]
    laugh: bool,
    #[arg(long)]
    smile: bool,

    /// 🎉 reaction flags
    #[arg(long)]
    hooray: bool,
    #[arg(long)]
    tada: bool,
    #[arg(long)]
    yay: bool,
    #[arg(long)]
    huzzah: bool,
    #[arg(long)]
    sarcastic_cheer: bool,

    /// 😕 reaction flags
    #[arg(long)]
    confused: bool,
    #[arg(long)]
    frown: bool,
    #[arg(long)]
    sad: bool,

    /// ❤️ reaction flags
    #[arg(long)]
    love: bool,
    #[arg(long)]
    heart: bool,
    #[arg(long)]
    favorite: bool,

    /// 🚀 reaction flags
    #[arg(long)]
    rocket: bool,
    #[arg(long)]
    zoom: bool,
    #[arg(long)]
    launch: bool,
    #[arg(long)]
    shipped: bool,
    #[arg(long)]
    sarcastic_ship_it: bool,

    /// 👀 reaction flags
    #[arg(long)]
    eyes: bool,
    #[arg(long)]
    looking: bool,
    #[arg(long)]
    surprise: bool,
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
      commands::start::handle(repository, mr_number, platform).await
    }
    Commands::Status => commands::status::handle().await,
    Commands::Peek => commands::peek::handle().await,
    Commands::Pop { unresolved } => commands::pop::handle(unresolved).await,
    Commands::Resolve => commands::resolve::handle().await,
    Commands::Comment { text, new } => commands::comment::handle(text, new).await,
    Commands::Commit { message, details, thread_id } => {
      commands::commit::handle(message, details, thread_id).await
    }
    Commands::Acknowledge {
      thumbs_up,
      ok,
      yeah,
      got_it,
      thumbs_down,
      f_you,
      laugh,
      smile,
      hooray,
      tada,
      yay,
      huzzah,
      sarcastic_cheer,
      confused,
      frown,
      sad,
      love,
      heart,
      favorite,
      rocket,
      zoom,
      launch,
      shipped,
      sarcastic_ship_it,
      eyes,
      looking,
      surprise,
    } => {
      let flags = commands::acknowledge::AcknowledgeFlags {
        thumbs_up,
        ok,
        yeah,
        got_it,
        thumbs_down,
        f_you,
        laugh,
        smile,
        hooray,
        tada,
        yay,
        huzzah,
        sarcastic_cheer,
        confused,
        frown,
        sad,
        love,
        heart,
        favorite,
        rocket,
        zoom,
        launch,
        shipped,
        sarcastic_ship_it,
        eyes,
        looking,
        surprise,
      };
      let config = commands::acknowledge::AcknowledgeConfig::from_flags(flags);
      commands::acknowledge::handle(config).await
    }
    Commands::Finish => commands::finish::handle().await,
    Commands::Refresh => commands::refresh::handle().await,
  }
}
