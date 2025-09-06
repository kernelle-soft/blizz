use anyhow::Result;
use insights::cli::commands;
use insights::server::services::search::SearchCommandOptions;

/// Insights CLI command structure - mirrors the main insights CLI
#[derive(clap::Subcommand)]
pub enum InsightsCommands {
  /// Add a new insight to the knowledge base
  Add {
    /// Topic category of the insight
    topic: String,
    /// Name of the insight
    name: String,
    /// Brief overview/summary of the insight
    overview: String,
    /// Detailed content of the insight
    details: String,
  },
  /// Search through all insights for matching content
  Search {
    #[command(flatten)]
    options: SearchCommandOptions,
    /// Search terms (space-separated)
    #[arg(required = true)]
    terms: Vec<String>,
  },
  /// Get content of a specific insight
  Get {
    /// Topic category of the insight
    topic: String,
    /// Name of the insight
    name: String,
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
    /// Topic category of the insight
    topic: String,
    /// Name of the insight
    name: String,
    /// New overview content
    #[arg(short, long)]
    overview: Option<String>,
    /// New details content
    #[arg(short, long)]
    details: Option<String>,
  },
  /// Delete an insight
  Delete {
    /// Topic category of the insight
    topic: String,
    /// Name of the insight
    name: String,
    /// Skip confirmation prompt
    #[arg(short, long)]
    force: bool,
  },
  /// List all available topics
  Topics,
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

pub async fn handle_insights_command(command: InsightsCommands) -> Result<()> {
  match command {
    InsightsCommands::Add { topic, name, overview, details } => {
      commands::add_insight(&topic, &name, &overview, &details).await
    }
    InsightsCommands::Search { options, terms } => {
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
    InsightsCommands::Get { topic, name, overview } => {
      commands::get_insight(&topic, &name, overview).await
    }
    InsightsCommands::List { topic, verbose } => {
      commands::list_insights(topic.as_deref(), verbose).await
    }
    InsightsCommands::Update { topic, name, overview, details } => {
      commands::update_insight(&topic, &name, overview.as_deref(), details.as_deref()).await
    }
    InsightsCommands::Delete { topic, name, force } => {
      commands::delete_insight(&topic, &name, force).await
    }
    InsightsCommands::Topics => commands::list_topics().await,
    InsightsCommands::Index { force } => commands::index_insights(force).await,
    InsightsCommands::Logs { limit, level } => commands::logs(limit, &level).await,
  }
}
