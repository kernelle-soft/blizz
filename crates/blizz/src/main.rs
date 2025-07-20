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
    /// Optional topic to restrict search to
    #[arg(short, long)]
    topic: Option<String>,
    /// Case-sensitive search
    #[arg(short, long)]
    case_sensitive: bool,
    /// Search only in overview sections
    #[arg(short, long)]
    overview_only: bool,
    /// Use semantic + exact search only (drops neural for speed)
    #[cfg(feature = "semantic")]
    #[arg(short, long)]
    semantic: bool,
    /// Use exact term matching only (fastest, drops neural and semantic)
    #[arg(short, long)]
    exact: bool,
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
  #[cfg(feature = "neural")]
  Index {
    /// Force recompute even for insights that already have embeddings
    #[arg(short, long)]
    force: bool,
    /// Only recompute missing embeddings (default behavior)
    #[arg(short, long)]
    missing_only: bool,
  },
}

fn main() -> Result<()> {
  let cli = Cli::parse();

  match cli.command {
    Commands::Add { topic, name, overview, details } => {
      add_insight(&topic, &name, &overview, &details)?;
    }
    Commands::Search {
      topic,
      case_sensitive,
      overview_only,
      terms,
      #[cfg(feature = "semantic")]
      semantic,
      exact,
    } => {
      // Tiered search approach
      if exact {
        // Tier 3: Exact matching only (fastest)
        search_insights_exact(&terms, topic.as_deref(), case_sensitive, overview_only)?;
      } else if semantic {
        // Tier 2: Semantic + Exact (drops neural for speed)
        #[cfg(feature = "semantic")]
        search_insights_combined_semantic(&terms, topic.as_deref(), case_sensitive, overview_only)?;
        #[cfg(not(feature = "semantic"))]
        search_insights_exact(&terms, topic.as_deref(), case_sensitive, overview_only)?;
      } else {
        // Tier 1: All methods combined (best results)
        search_insights_combined_all(&terms, topic.as_deref(), case_sensitive, overview_only)?;
      }
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
    Commands::Delete { topic, name, force } => {
      delete_insight(&topic, &name, force)?;
    }
    Commands::Topics => {
      list_topics()?;
    }
    #[cfg(feature = "neural")]
    Commands::Index { force, missing_only } => {
      index_insights(force, missing_only)?;
    }
  }

  Ok(())
}
