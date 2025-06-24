use anyhow::Result;
use clap::{Parser, Subcommand};

mod commands;
mod insight;

use commands::*;

#[derive(Parser)]
#[command(name = "blizz")]
#[command(
  about = "⚡ Blizz - Knowledge Management System\nHigh-speed insight storage and retrieval for development workflows"
)]
#[command(version)]
struct Cli {
  #[command(subcommand)]
  command: Commands,
}

#[derive(Subcommand)]
enum Commands {
  /// Add a new insight to the knowledge base
  Add {
    /// Topic category for the insight
    topic: String,
    /// Name/identifier for the insight
    name: String,
    /// Brief overview/summary of the insight
    overview: String,
    /// Detailed content of the insight
    details: String,
  },
  /// Search through all insights for matching content
  Search {
    /// Search query text
    query: String,
    /// Optional topic to restrict search to
    #[arg(short, long)]
    topic: Option<String>,
    /// Case-sensitive search
    #[arg(long)]
    case_sensitive: bool,
    /// Search only in overview sections
    #[arg(long)]
    overview_only: bool,
  },
  /// Get content of a specific insight
  Get {
    /// Topic category of the insight
    topic: String,
    /// Name of the insight
    name: String,
    /// Show only the overview section
    #[arg(long)]
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
    #[arg(long)]
    overview: Option<String>,
    /// New details content
    #[arg(long)]
    details: Option<String>,
  },
  /// Create a link from one insight to another topic
  Link {
    /// Source topic
    src_topic: String,
    /// Source insight name
    src_name: String,
    /// Target topic
    target_topic: String,
    /// Target name (defaults to source name)
    target_name: Option<String>,
  },
  /// Delete an insight
  Delete {
    /// Topic category of the insight
    topic: String,
    /// Name of the insight
    name: String,
    /// Skip confirmation prompt
    #[arg(long)]
    force: bool,
  },
  /// List all available topics
  Topics,
}

fn main() -> Result<()> {
  let cli = Cli::parse();

  match cli.command {
    Commands::Add { topic, name, overview, details } => {
      add_insight(&topic, &name, &overview, &details)?;
    }
    Commands::Search { query, topic, case_sensitive, overview_only } => {
      search_insights(&query, topic.as_deref(), case_sensitive, overview_only)?;
    }
    Commands::Get { topic, name, overview } => {
      get_insight(&topic, &name, overview)?;
    }
    Commands::List { topic, verbose } => {
      list_insights(topic.as_deref(), verbose)?;
    }
    Commands::Update { topic, name, overview, details } => {
      update_insight(&topic, &name, overview.as_deref(), details.as_deref())?;
    }
    Commands::Link { src_topic, src_name, target_topic, target_name } => {
      link_insight(&src_topic, &src_name, &target_topic, target_name.as_deref())?;
    }
    Commands::Delete { topic, name, force } => {
      delete_insight(&topic, &name, force)?;
    }
    Commands::Topics => {
      list_topics()?;
    }
  }

  Ok(())
}
