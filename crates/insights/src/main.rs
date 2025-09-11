use anyhow::Result;
use clap::{Args, Parser, Subcommand};
use insights::cli::commands;

#[derive(Parser)]
#[command(name = "insights")]
#[command(
  about = "Insights - Knowledge Management System\nHigh-speed insight storage and retrieval for development workflows"
)]
#[command(version = concat!(env!("CARGO_PKG_VERSION"), ", courtesy of Blizz and Kernelle Software"))]
struct Cli {
  #[command(subcommand)]
  command: Command,
}

/// Common insight identifier arguments
#[derive(Args)]
struct InsightId {
  /// Topic category of the insight
  topic: String,
  /// Name of the insight
  name: String,
}

// violet ignore chunk
#[derive(Subcommand)]
enum Command {
  /// Add a new insight to the knowledge base
  Add {
    #[command(flatten)]
    id: InsightId,
    /// Brief overview/summary of the insight
    overview: String,
    /// Detailed content of the insight
    details: String,
  },
  /// Search through all insights for matching content
  Search {
    #[command(flatten)]
    options: insights::server::services::search::SearchCommandOptions,
    /// Search terms (space-separated)
    #[arg(required = true)]
    terms: Vec<String>,
  },
  /// Get content of a specific insight
  Get {
    #[command(flatten)]
    id: InsightId,
    /// Show only the overview section
    #[arg(short, long)]
    overview: bool,
  },
  /// List insights in a topic or all topics
  List {
    /// Optional topic to filter by
    topic: Option<String>,
    /// Show overview content for each insight
    #[arg(short, long)]
    verbose: bool,
  },
  /// Update an existing insight
  Update {
    #[command(flatten)]
    id: InsightId,
    /// New overview content
    #[arg(short, long)]
    overview: Option<String>,
    /// New details content
    #[arg(short, long)]
    details: Option<String>,
  },
  /// Delete an insight
  Delete {
    #[command(flatten)]
    id: InsightId,
    /// Skip confirmation prompt
    #[arg(short, long)]
    force: bool,
  },
  /// List all available topics
  Topics,
  /// Show count of topics and insights
  Count,
  /// Recompute embeddings for all insights
  Index {
    /// Force recompute even for insights that already have embeddings
    #[arg(short, long)]
    force: bool,
  },
  /// Query daemon logs for debugging and monitoring
  Logs {
    /// Maximum number of log entries to return
    #[arg(short, long, default_value = "50")]
    limit: usize,
    /// Filter by log level (info, warn, error, all)
    #[arg(long, default_value = "all")]
    level: String,
  },
}

async fn handle(command: Command) -> Result<()> {
  match command {
    Command::Add { id, overview, details } => {
      commands::add_insight(&id.topic, &id.name, &overview, &details).await
    }
    Command::Search { options, terms } => {
      commands::search_insights(
        &terms,
        options.topic.clone(),
        options.case_sensitive,
        options.overview_only,
        options.exact,
        options.semantic,
      )
      .await
    }
    Command::Get { id, overview } => commands::get_insight(&id.topic, &id.name, overview).await,
    Command::List { topic, verbose } => commands::list_insights(topic.as_deref(), verbose).await,
    Command::Update { id, overview, details } => {
      commands::update_insight(&id.topic, &id.name, overview.as_deref(), details.as_deref()).await
    }
    Command::Delete { id, force } => commands::delete_insight(&id.topic, &id.name, force).await,
    Command::Topics => commands::list_topics().await,
    Command::Count => commands::count_insights().await,
    Command::Index { force } => commands::index_insights(force).await,
    Command::Logs { limit, level } => commands::logs(limit, &level).await,
  }
}

#[tokio::main]
async fn main() -> Result<()> {
  let cli = Cli::parse();

  handle(cli.command).await?;
  Ok(())
}
