use anyhow::Result;
use colored::*;

use crate::embedding_client;
use crate::insight::*;
use crate::search;


/// Add a new insight to the knowledge base
pub fn add_insight(topic: &str, name: &str, overview: &str, details: &str) -> Result<()> {
  let mut insight =
    Insight::new(topic.to_string(), name.to_string(), overview.to_string(), details.to_string());

  // Compute embedding before saving
  let embedding = embedding_client::embed_insight(&mut insight);
  insight.set_embedding(embedding);
  insight.save()?;

  println!("{} Added insight {}/{}", "✓".green(), topic.cyan(), name.yellow());
  Ok(())
}

/// Exact term matching search (delegates to search module)
pub fn search_insights_exact(
  terms: &[String],
  topic_filter: Option<&str>,
  case_sensitive: bool,
  overview_only: bool,
) -> Result<()> {
  search::search_insights_exact(terms, topic_filter, case_sensitive, overview_only)
}

/// Semantic similarity search (delegates to search module)
#[cfg(feature = "semantic")]
#[allow(dead_code)]
pub fn search_insights_semantic(
  terms: &[String],
  topic_filter: Option<&str>,
  case_sensitive: bool,
  overview_only: bool,
) -> Result<()> {
  search::search_insights_semantic(terms, topic_filter, case_sensitive, overview_only)
}

/// Neural search (delegates to search module)
#[cfg(feature = "neural")]
#[allow(dead_code)]
pub fn search_neural(
  terms: &[String],
  topic_filter: Option<&str>,
  case_sensitive: bool,
  overview_only: bool,
) -> Result<()> {
  search::search_neural(terms, topic_filter, case_sensitive, overview_only)
}

/// Combined semantic search (delegates to search module)
#[cfg(feature = "semantic")]
pub fn search_insights_combined_semantic(
  terms: &[String],
  topic_filter: Option<&str>,
  case_sensitive: bool,
  overview_only: bool,
) -> Result<()> {
  search::search_insights_combined_semantic(terms, topic_filter, case_sensitive, overview_only)
}

/// Search using all available methods (delegates to search module)
pub fn search_all(
  terms: &[String],
  topic_filter: Option<&str>,
  case_sensitive: bool,
  overview_only: bool,
) -> Result<()> {
  search::search_all(terms, topic_filter, case_sensitive, overview_only)
}

/// Get content of a specific insight
pub fn get_insight(topic: &str, name: &str, overview_only: bool) -> Result<()> {
  let insight = Insight::load(topic, name)?;

  if overview_only {
    println!("{}", insight.overview);
  } else {
    println!("---\n{}\n---\n\n{}", insight.overview, insight.details);
  }

  Ok(())
}

pub fn list_insights(filter: Option<&str>, verbose: bool) -> Result<()> {
  let insights = get_insights(filter)?;

  if insights.is_empty() {
    if let Some(topic) = filter {
      println!("No insights found in topic: {}", topic.yellow());
    } else {
      println!("No insights found");
    }
    return Ok(());
  }

  for (topic, name) in insights {
    if verbose {
      if let Ok(insight) = Insight::load(&topic, &name) {
        println!(
          "{}/{}: {}",
          topic.cyan(),
          name.yellow(),
          insight.overview.trim().replace('\n', " ")
        );
      }
    } else {
      println!("{}/{}", topic.cyan(), name.yellow());
    }
  }

  Ok(())
}

/// Check if insight content has actually changed
fn has_content_changed(new_overview: Option<&str>, new_details: Option<&str>) -> bool {
  new_overview.is_some() || new_details.is_some()
}

/// Recompute embedding and save insight using existing methods
fn recompute_and_save_updated_insight(insight: &mut Insight) -> Result<()> {
  let embedding = embedding_client::embed_insight(insight);
  insight.set_embedding(embedding);

  // For updates, we need to delete first then save since save() checks for existing files
  let file_path = insight.file_path()?;
  if file_path.exists() {
    std::fs::remove_file(&file_path)?;
  }
  insight.save()?;

  Ok(())
}

/// Print success message for insight update
fn print_update_success(topic: &str, name: &str) {
  println!("{} Updated insight {}/{}", "✓".green(), topic.cyan(), name.yellow());
}

/// Update an existing insight
pub fn update_insight(
  topic: &str,
  name: &str,
  new_overview: Option<&str>,
  new_details: Option<&str>,
) -> Result<()> {
  let mut insight = Insight::load(topic, name)?;
  let content_changed = has_content_changed(new_overview, new_details);

  insight.update(new_overview, new_details)?;

  if content_changed {
    recompute_and_save_updated_insight(&mut insight)?;
  }

  print_update_success(topic, name);
  Ok(())
}

/// Delete an insight
pub fn delete_insight(topic: &str, name: &str, force: bool) -> Result<()> {
  let insight = Insight::load(topic, name)?;

  if !force {
    println!("Are you sure you want to delete {}/{}? [y/N]", topic.cyan(), name.yellow());

    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;

    if !input.trim().to_lowercase().starts_with('y') {
      println!("Deletion cancelled");
      return Ok(());
    }
  }

  insight.delete()?;

  println!("{} Deleted insight {}/{}", "✓".green(), topic.cyan(), name.yellow());
  Ok(())
}

/// List all available topics
pub fn list_topics() -> Result<()> {
  let topics = get_topics()?;

  if topics.is_empty() {
    println!("No topics found");
    return Ok(());
  }

  for topic in topics {
    println!("{}", topic.cyan());
  }

  Ok(())
}

/// Check if insight should be skipped during indexing
#[cfg(feature = "neural")]
fn should_skip_insight(insight: &crate::insight::Insight, force: bool, missing_only: bool) -> bool {
  !force && missing_only && insight.has_embedding()
}

/// Save insight with embedding metadata to file
#[cfg(feature = "neural")]
fn save_insight_with_embedding(insight: &crate::insight::Insight) -> Result<()> {
  let file_path = insight.file_path()?;

  let frontmatter = crate::insight::FrontMatter {
    overview: insight.overview.clone(),
    embedding_version: insight.embedding_version.clone(),
    embedding: insight.embedding.clone(),
    embedding_text: insight.embedding_text.clone(),
    embedding_computed: insight.embedding_computed,
  };

  let yaml_content = serde_yaml::to_string(&frontmatter)?;
  let content = format!("---\n{}---\n\n# Details\n{}", yaml_content, insight.details);
  std::fs::write(&file_path, content)?;

  Ok(())
}

/// Process a single insight for indexing
#[cfg(feature = "neural")]
fn process_single_insight_index(
  topic: &str,
  insight_name: &str,
  force: bool,
  missing_only: bool,
) -> Result<(bool, bool)> {
  // (processed, skipped)
  use crate::insight::Insight;

  let mut insight = Insight::load(topic, insight_name)?;

  // Check if we should skip this insight
  if should_skip_insight(&insight, force, missing_only) {
    println!("    · {insight_name} (already has embedding)");
    return Ok((false, true));
  }

  print!("  · {insight_name}... ");
  std::io::Write::flush(&mut std::io::stdout())?;

  // Compute embedding
  let embedding = embedding_client::embed_insight(&mut insight);
  insight.set_embedding(embedding);
  save_insight_with_embedding(&insight)?;

  println!("done");
  Ok((true, false))
}

/// Print indexing summary
#[cfg(feature = "neural")]
fn print_index_summary(processed: usize, skipped: usize, errors: usize, topic_count: usize) {
  println!(
    "\nIndexed {} insights across {} topics {}",
    processed.to_string().yellow(),
    topic_count.to_string().yellow(),
    "⚡".blue()
  );

  if skipped > 0 {
    println!("  {} Skipped: {}", "⏭".blue(), skipped.to_string().yellow());
  }
  if errors > 0 {
    println!("  {} Errors: {}", "❌".red(), errors.to_string().yellow());
  }
}

/// Statistics tracking for indexing operations
#[cfg(feature = "neural")]
#[derive(Default)]
struct IndexingStats {
  processed: usize,
  skipped: usize,
  errors: usize,
}

#[cfg(feature = "neural")]
impl IndexingStats {
  fn add_result(&mut self, result: Result<(bool, bool)>) {
    match result {
      Ok((was_processed, was_skipped)) => {
        if was_processed {
          self.processed += 1;
        }
        if was_skipped {
          self.skipped += 1;
        }
      }
      Err(_) => {
        self.errors += 1;
      }
    }
  }
}

/// Validate that topics exist for indexing
#[cfg(feature = "neural")]
fn validate_topics_for_indexing() -> Result<Vec<String>> {
  use crate::insight::get_topics;

  let topics = get_topics()?;
  if topics.is_empty() {
    println!("No insights found to index.");
  }
  Ok(topics)
}

/// Process indexing for all insights in a single topic
#[cfg(feature = "neural")]
fn process_topic_indexing(
  topic: &str,
  force: bool,
  missing_only: bool,
  stats: &mut IndexingStats,
) -> Result<()> {
  use crate::insight::get_insights;

  let insights = get_insights(Some(topic))?;
  println!("{} {}:", "◈".blue(), topic.cyan());

  for (_, insight_name) in insights {
    let result = process_single_insight_index(topic, &insight_name, force, missing_only);
    if let Err(ref e) = result {
      eprintln!("    · Failed to process {insight_name}: {e}");
    }
    stats.add_result(result);
  }

  Ok(())
}

/// Recompute embeddings for all insights
#[cfg(feature = "neural")]
pub fn index_insights(force: bool, missing_only: bool) -> Result<()> {
  let topics = validate_topics_for_indexing()?;

  if topics.is_empty() {
    return Ok(());
  }

  let mut stats = IndexingStats::default();

  for topic in &topics {
    process_topic_indexing(topic, force, missing_only, &mut stats)?;
  }

  print_index_summary(stats.processed, stats.skipped, stats.errors, topics.len());
  Ok(())
}
