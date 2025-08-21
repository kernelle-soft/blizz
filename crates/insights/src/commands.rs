use anyhow::Result;
use colored::*;

use crate::insight::InsightMetaData;
use crate::insight::{self, Insight};

/// Add a new insight to the knowledge base (testable version with dependency injection)
pub fn add_insight_with_client(
  topic: &str,
  name: &str,
  overview: &str,
  details: &str,
) -> Result<()> {
  let insight =
    Insight::new(topic.to_string(), name.to_string(), overview.to_string(), details.to_string());

  insight::save(&insight)?;

  println!("{} Added insight {}/{}", "✓".green(), topic.cyan(), name.yellow());
  Ok(())
}

/// Add a new insight to the knowledge base (production version)
pub fn add_insight(topic: &str, name: &str, overview: &str, details: &str) -> Result<()> {
  add_insight_with_client(topic, name, overview, details)
}

/// Get content of a specific insight
pub fn get_insight(topic: &str, name: &str, overview_only: bool) -> Result<()> {
  let insight = insight::load(topic, name)?;

  if overview_only {
    println!("{}", insight.overview);
  } else {
    println!("---\n{}\n---\n\n{}", insight.overview, insight.details);
  }

  Ok(())
}

pub fn list_insights(filter: Option<&str>, verbose: bool) -> Result<()> {
  let insights = insight::get_insights(filter)?;

  if insights.is_empty() {
    if let Some(topic) = filter {
      println!("No insights found in topic: {}", topic.yellow());
    } else {
      println!("No insights found.");
    }
    return Ok(());
  }

  for insight in insights {
    let formatted_name = if verbose {
      format!("{}/{} - {}", insight.topic.cyan(), insight.name.yellow(), insight.overview)
    } else {
      format!("{}/{}", insight.topic.cyan(), insight.name.yellow())
    };
    println!("{formatted_name}");
  }

  Ok(())
}

pub fn list_topics() -> Result<()> {
  let topics = insight::get_topics()?;

  if topics.is_empty() {
    println!("No topics found.");
    return Ok(());
  }

  for topic in topics {
    println!("{}", topic.cyan());
  }

  Ok(())
}

/// Update an existing insight's overview and/or details
pub fn update_insight_with_client(
  topic: &str,
  name: &str,
  new_overview: Option<&str>,
  new_details: Option<&str>,
) -> Result<()> {
  let mut insight = insight::load(topic, name)?;

  insight::update(&mut insight, new_overview, new_details)?;

  let file_path = insight::file_path(&insight)?;
  let metadata = InsightMetaData {
    topic: insight.topic.clone(),
    name: insight.name.clone(),
    overview: insight.overview.clone(),
    embedding_version: insight.embedding_version.clone(),
    embedding: insight.embedding.clone(),
    embedding_text: insight.embedding_text.clone(),
    embedding_computed: insight.embedding_computed,
  };

  let yaml_content = serde_yaml::to_string(&metadata)?;
  let content = format!("---\n{}---\n\n{}", yaml_content, insight.details);
  std::fs::write(&file_path, content)?;

  println!("{} Updated insight {}/{}", "✓".green(), topic.cyan(), name.yellow());

  Ok(())
}

/// Update an existing insight's overview and/or details
pub fn update_insight(
  topic: &str,
  name: &str,
  new_overview: Option<&str>,
  new_details: Option<&str>,
) -> Result<()> {
  update_insight_with_client(topic, name, new_overview, new_details)
}

/// Delete an insight
pub fn delete_insight(topic: &str, name: &str, force: bool) -> Result<()> {
  if !force {
    return Err(anyhow::anyhow!("Delete operation requires --force flag"));
  }

  let insight = insight::load(topic, name)?;
  insight::delete(&insight)?;

  println!("{} Deleted insight {}/{}", "✓".green(), topic.cyan(), name.yellow());

  Ok(())
}

fn index_insight(insight: &mut Insight, _force: bool) -> Result<bool> {
  let file_path = insight::file_path(insight)?;
  let metadata = InsightMetaData {
    topic: insight.topic.clone(),
    name: insight.name.clone(),
    overview: insight.overview.clone(),
    embedding_version: insight.embedding_version.clone(),
    embedding: insight.embedding.clone(),
    embedding_text: insight.embedding_text.clone(),
    embedding_computed: insight.embedding_computed,
  };

  let yaml = serde_yaml::to_string(&metadata)?;
  let content = format!("---\n{}---\n\n# Details\n{}", yaml, insight.details);
  std::fs::write(&file_path, content)?;

  println!(
    "  {} Updated embeddings for {}/{}",
    "✓".green(),
    insight.topic.cyan(),
    insight.name.yellow()
  );

  Ok(true)
}

fn index_topics(topic: &str, force: bool) -> Result<(usize, usize)> {
  let insights = insight::get_insights(Some(topic))?;
  let total = insights.len();
  let mut updated = 0;

  for mut insight in insights {
    if index_insight(&mut insight, force)? {
      updated += 1;
    }
  }

  Ok((updated, total))
}

/// Recompute embeddings for insights (testable version with dependency injection)
pub fn index_insights(force: bool) -> Result<()> {
  let topics = insight::get_topics()?;

  if topics.is_empty() {
    println!("No topics found to index.");
    return Ok(());
  }

  let mut total_updated = 0;
  let mut total_processed = 0;

  for topic in topics {
    let (updated, processed) = index_topics(&topic, force)?;
    total_updated += updated;
    total_processed += processed;
  }

  println!(
    "{} Indexed {} of {} insights",
    "✓".green(),
    total_updated.to_string().yellow(),
    total_processed.to_string().cyan()
  );

  Ok(())
}
