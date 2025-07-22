use anyhow::Result;
use clap::{Args, Parser, Subcommand};

mod commands;
mod embedding_client;
mod embedding_model;
mod insight;
mod search;
mod semantic;
mod similarity;

#[derive(Parser)]
#[command(name = "blizz")]
#[command(
  about = "Blizz - Knowledge Management System\nHigh-speed insight storage and retrieval for development workflows"
)]
#[command(version)]
struct Cli {
  #[command(subcommand)]
  command: Commands,
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
enum Commands {
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
    options: search::SearchCommandOptions,
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
  /// Recompute embeddings for all insights
  #[cfg(feature = "neural")]
  Index {
    /// Force recompute even for insights that already have embeddings
    #[arg(short, long)]
    force: bool,
  },
}

fn main() -> Result<()> {
  let cli = Cli::parse();

  match cli.command {
    Commands::Add { id, overview, details } => {
      commands::add_insight(&id.topic, &id.name, &overview, &details)?;
    }
    Commands::Search { options, terms } => {
      let search_options = search::SearchOptions::from(&options);
      search::search(&terms, &search_options)?;
    }
    Commands::Get { id, overview } => {
      commands::get_insight(&id.topic, &id.name, overview)?;
    }
    Commands::List { topic, verbose } => {
      commands::list_insights(topic.as_deref(), verbose)?;
    }
    Commands::Update { id, overview, details } => {
      commands::update_insight(&id.topic, &id.name, overview.as_deref(), details.as_deref())?;
    }
    Commands::Delete { id, force } => {
      commands::delete_insight(&id.topic, &id.name, force)?;
    }
    Commands::Topics => {
      commands::list_topics()?;
    }
    #[cfg(feature = "neural")]
    Commands::Index { force } => {
      commands::index_insights(force)?;
    }
  }

  Ok(())
}
