use anyhow::Result;
use colored::*;

use crate::embedding_client::{self, EmbeddingClient};
use crate::insight::{self, Insight, InsightMetaData};

/// Add a new insight to the knowledge base (testable version with dependency injection)
pub fn add_insight_with_client(
  topic: &str, 
  name: &str, 
  overview: &str, 
  details: &str,
  client: &EmbeddingClient
) -> Result<()> {
  let mut insight = Insight::new(topic.to_string(), name.to_string(), overview.to_string(), details.to_string());

  // Compute embedding before saving
  let embedding = embedding_client::embed_insight(client, &mut insight);
  insight::set_embedding(&mut insight, embedding);
  insight::save(&insight)?;

  println!("{} Added insight {}/{}", "✓".green(), topic.cyan(), name.yellow());
  Ok(())
}

/// Add a new insight to the knowledge base (production version)
pub fn add_insight(topic: &str, name: &str, overview: &str, details: &str) -> Result<()> {
  let client = embedding_client::new();
  add_insight_with_client(topic, name, overview, details, &client)
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
    println!("{}", formatted_name);
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

/// Update an existing insight's overview and/or details (testable version with dependency injection)
pub fn update_insight_with_client(
  topic: &str,
  name: &str,
  new_overview: Option<&str>,
  new_details: Option<&str>,
  client: &EmbeddingClient
) -> Result<()> {
  let mut insight = insight::load(topic, name)?;

  // Update the insight content using the existing update function
  insight::update(&mut insight, new_overview, new_details)?;

  // Recompute and set embedding after content change
  let embedding = embedding_client::embed_insight(client, &mut insight);
  insight::set_embedding(&mut insight, embedding);

  // Save the updated insight with new embedding
  let file_path = insight::file_path(&insight)?;
  let frontmatter = InsightMetaData {
    overview: insight.overview.clone(),
    embedding_version: insight.embedding_version.clone(),
    embedding: insight.embedding.clone(),
    embedding_text: insight.embedding_text.clone(),
    embedding_computed: insight.embedding_computed,
  };

  let yaml_content = serde_yaml::to_string(&frontmatter)?;
  let content = format!("---\n{}---\n\n# Details\n{}", yaml_content, insight.details);
  std::fs::write(&file_path, content)?;

  println!("{} Updated insight {}/{}", "✓".green(), topic.cyan(), name.yellow());

  Ok(())
}

/// Update an existing insight's overview and/or details (production version)
pub fn update_insight(topic: &str, name: &str, new_overview: Option<&str>, new_details: Option<&str>) -> Result<()> {
  let client = embedding_client::new();
  update_insight_with_client(topic, name, new_overview, new_details, &client)
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

fn process_insight_indexing(insight: &mut Insight, force: bool, missing_only: bool, client: &EmbeddingClient) -> Result<bool> {
  let should_update = if force {
    true
  } else if missing_only {
    !insight::has_embedding(insight)
  } else {
    !insight::has_embedding(insight)
  };

  if should_update {
    let embedding = embedding_client::embed_insight(client, insight);
    insight::set_embedding(insight, embedding);
    
    // Save the updated insight with new embedding (for existing insights)
    let file_path = insight::file_path(insight)?;
    let frontmatter = InsightMetaData {
      overview: insight.overview.clone(),
      embedding_version: insight.embedding_version.clone(),
      embedding: insight.embedding.clone(),
      embedding_text: insight.embedding_text.clone(),
      embedding_computed: insight.embedding_computed,
    };

    let yaml_content = serde_yaml::to_string(&frontmatter)?;
    let content = format!("---\n{}---\n\n# Details\n{}", yaml_content, insight.details);
    std::fs::write(&file_path, content)?;
    
    println!(
      "  {} Updated embeddings for {}/{}",
      "✓".green(),
      insight.topic.cyan(),
      insight.name.yellow()
    );
    Ok(true)
  } else {
    Ok(false)
  }
}

fn process_single_insight_index_with_client(
  topic: &str,
  name: &str,
  force: bool,
  missing_only: bool,
  client: &EmbeddingClient
) -> Result<bool> {
  match insight::load(topic, name) {
    Ok(mut insight) => {
      process_insight_indexing(&mut insight, force, missing_only, client)
    }
    Err(_) => {
      eprintln!(
        "  {} Warning: Could not load insight {}/{}",
        "⚠".yellow(),
        topic,
        name
      );
      Ok(false)
    }
  }
}

fn process_single_insight_index(topic: &str, name: &str, force: bool, missing_only: bool) -> Result<bool> {
  let client = embedding_client::new();
  process_single_insight_index_with_client(topic, name, force, missing_only, &client)
}

fn process_topic_indexing_with_client(
  topic: &str,
  force: bool,
  missing_only: bool,
  client: &EmbeddingClient
) -> Result<(usize, usize)> {
  let insights = insight::get_insights(Some(topic))?;
  let total = insights.len();
  let mut updated = 0;

  for mut insight in insights {
    if process_insight_indexing(&mut insight, force, missing_only, client)? {
      updated += 1;
    }
  }

  Ok((updated, total))
}

fn process_topic_indexing(topic: &str, force: bool, missing_only: bool) -> Result<(usize, usize)> {
  let client = embedding_client::new();
  process_topic_indexing_with_client(topic, force, missing_only, &client)
}

/// Recompute embeddings for insights (testable version with dependency injection)
pub fn index_insights_with_client(force: bool, missing_only: bool, client: &EmbeddingClient) -> Result<()> {
  let topics = insight::get_topics()?;
  
  if topics.is_empty() {
    println!("No topics found to index.");
    return Ok(());
  }

  let mut total_updated = 0;
  let mut total_processed = 0;

  for topic in topics {
    let (updated, processed) = process_topic_indexing_with_client(&topic, force, missing_only, client)?;
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

/// Recompute embeddings for insights (production version)
pub fn index_insights(force: bool, missing_only: bool) -> Result<()> {
  let client = embedding_client::new();
  index_insights_with_client(force, missing_only, &client)
}

// Legacy function names for backwards compatibility
pub fn add_insight_with_service<T: embedding_client::EmbeddingService>(
  topic: &str, 
  name: &str, 
  overview: &str, 
  details: &str,
  _service: &T
) -> Result<()> {
  let client = embedding_client::with_mock();
  add_insight_with_client(topic, name, overview, details, &client)
}

pub fn update_insight_with_service<T: embedding_client::EmbeddingService>(
  topic: &str,
  name: &str,
  new_overview: Option<&str>,
  new_details: Option<&str>,
  _service: &T
) -> Result<()> {
  let client = embedding_client::with_mock();
  update_insight_with_client(topic, name, new_overview, new_details, &client)
}

pub fn index_insights_with_service<T: embedding_client::EmbeddingService>(force: bool, missing_only: bool, _service: &T) -> Result<()> {
  let client = embedding_client::with_mock();
  index_insights_with_client(force, missing_only, &client)
}
