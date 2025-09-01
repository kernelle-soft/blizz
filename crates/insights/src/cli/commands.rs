use anyhow::{anyhow, Result};
use colored::*;

use crate::cli::client::get_client;
use crate::cli::server_manager::ensure_server_running;
use crate::insight::{self, Insight, InsightMetaData};

/// Add a new insight to the knowledge base (production version)
pub async fn add_insight(topic: &str, name: &str, overview: &str, details: &str) -> Result<()> {
  ensure_server_running().await?;
  let client = get_client();
  client.add_insight(topic, name, overview, details).await?;

  println!("{} Added insight {}/{}", "✓".green(), topic.cyan(), name.yellow());
  Ok(())
}

/// Get content of a specific insight
pub async fn get_insight(topic: &str, name: &str, overview_only: bool) -> Result<()> {
  ensure_server_running().await?;

  let client = get_client();
  let response = client.get_insight(topic, name, overview_only).await?;

  if overview_only {
    println!("{}", response.insight.overview);
  } else {
    println!("---\n{}\n---\n\n{}", response.insight.overview, response.insight.details);
  }

  Ok(())
}

pub async fn list_insights(filter: Option<&str>, verbose: bool) -> Result<()> {
  ensure_server_running().await?;

  let client = get_client();
  let response = client.list_insights(Vec::new()).await?; // TODO: Add topic filtering

  let insights = if let Some(topic_filter) = filter {
    response
      .insights
      .into_iter()
      .filter(|insight| insight.topic == topic_filter)
      .collect::<Vec<_>>()
  } else {
    response.insights
  };

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

pub async fn list_topics() -> Result<()> {
  ensure_server_running().await?;

  let client = get_client();
  let topics = client.list_topics().await?;

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
pub async fn update_insight(
  topic: &str,
  name: &str,
  new_overview: Option<&str>,
  new_details: Option<&str>,
) -> Result<()> {
  ensure_server_running().await?;
  let client = get_client();
  client.update_insight(topic, name, new_overview, new_details).await?;

  println!("{} Updated insight {}/{}", "✓".green(), topic.cyan(), name.yellow());
  Ok(())
}

/// Delete an insight
pub async fn delete_insight(topic: &str, name: &str, force: bool) -> Result<()> {
  ensure_server_running().await?;

  // Check if insight exists first
  let client = get_client();
  if let Err(e) = client.get_insight(topic, name, true).await {
    let error_msg = e.to_string();
    if error_msg.contains("insight_not_found") || error_msg.contains("not found") {
      return Err(anyhow!("Insight {}/{} not found", topic, name));
    } else {
      return Err(e);
    }
  }

  // Ask for confirmation unless --force is used
  if !force {
    use std::io::{self, Write};

    print!("Are you sure you want to delete insight {}/{}? [y/N]: ", topic.cyan(), name.yellow());
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    let response = input.trim().to_lowercase();
    if response != "y" && response != "yes" {
      println!("Delete cancelled.");
      return Ok(());
    }
  }

  // Delete the insight
  client.remove_insight(topic, name).await?;
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

  for topic in &topics {
    let (updated, processed) = index_topics(topic, force)?;
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

/// Query daemon logs for debugging and monitoring
pub async fn logs(limit: usize, level: &str) -> Result<()> {
  ensure_server_running().await?;

  let client = get_client();

  // TODO: Add support for limit and level parameters to the REST API
  let logs_response = client.get_logs().await?;

  if logs_response.data.logs.is_empty() {
    println!("No logs found.");
    return Ok(());
  }

  println!("{} Server logs (showing {} entries):", "📋".cyan(), logs_response.data.logs.len());
  println!();

  for log in logs_response.data.logs {
    let level_colored = match log.level.as_str() {
      "error" => log.level.red().bold(),
      "warn" => log.level.yellow().bold(),
      "info" => log.level.blue().bold(),
      "debug" => log.level.green(),
      _ => log.level.normal(),
    };

    println!("{} [{}] {}", log.timestamp.to_string().cyan(), level_colored, log.message);
  }

  Ok(())
}
