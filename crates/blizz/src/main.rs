use anyhow::Result;
use clap::{Args, Parser, Subcommand};

mod commands;
mod insight;

use commands::*;

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

/// Search configuration options
#[derive(Args)]
struct SearchOptions {
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
    options: SearchOptions,
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
    /// Only recompute missing embeddings (default behavior)
    #[arg(short, long)]
    missing_only: bool,
  },
}

fn execute_search(
  terms: &[String],
  topic: Option<&str>,
  case_sensitive: bool,
  overview_only: bool,
  exact: bool,
  #[cfg(feature = "semantic")] semantic: bool,
) -> Result<()> {
  if exact {
    return search_insights_exact(terms, topic, case_sensitive, overview_only);
  }

  #[cfg(feature = "semantic")]
  if semantic {
    search_insights_combined_semantic(terms, topic, case_sensitive, overview_only)
  } else {
    search_all(terms, topic, case_sensitive, overview_only)
  }

  #[cfg(not(feature = "semantic"))]
  search_all(terms, topic, case_sensitive, overview_only)
}

/// Handle insight-based commands that need topic/name extraction
fn handle_insight_command(id: &InsightId, command_type: InsightCommandType) -> Result<()> {
  match command_type {
    InsightCommandType::Add { overview, details } => {
      add_insight(&id.topic, &id.name, &overview, &details)
    }
    InsightCommandType::Get { overview } => {
      get_insight(&id.topic, &id.name, overview)
    }
    InsightCommandType::Update { overview, details } => {
      update_insight(&id.topic, &id.name, overview.as_deref(), details.as_deref())
    }
    InsightCommandType::Delete { force } => {
      delete_insight(&id.topic, &id.name, force)
    }
  }
}

/// Handle search command with all its complex options
fn handle_search_command(options: SearchOptions, terms: Vec<String>) -> Result<()> {
  execute_search(
    &terms, 
    options.topic.as_deref(), 
    options.case_sensitive, 
    options.overview_only, 
    options.exact,
    #[cfg(feature = "semantic")] options.semantic
  )
}

/// Enum for insight-based command types
enum InsightCommandType {
  Add { overview: String, details: String },
  Get { overview: bool },
  Update { overview: Option<String>, details: Option<String> },
  Delete { force: bool },
}

fn main() -> Result<()> {
  let cli = Cli::parse();

  match cli.command {
    Commands::Add { id, overview, details } => {
      handle_insight_command(&id, InsightCommandType::Add { overview, details })?;
    }
    Commands::Search { options, terms } => {
      handle_search_command(options, terms)?;
    }
    Commands::Get { id, overview } => {
      handle_insight_command(&id, InsightCommandType::Get { overview })?;
    }
    Commands::List { topic, verbose } => {
      list_insights(topic.as_deref(), verbose)?;
    }
    Commands::Update { id, overview, details } => {
      handle_insight_command(&id, InsightCommandType::Update { overview, details })?;
    }
    Commands::Delete { id, force } => {
      handle_insight_command(&id, InsightCommandType::Delete { force })?;
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
