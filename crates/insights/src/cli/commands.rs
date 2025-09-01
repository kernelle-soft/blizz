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
      println!("No insights found for topic: {}", topic.yellow());
    } else {
      println!("No insights found.");
    }
    return Ok(());
  }

  // Group by topic for better display
  use std::collections::BTreeMap;
  let mut by_topic: BTreeMap<String, Vec<_>> = BTreeMap::new();
  for insight in insights {
    by_topic.entry(insight.topic.clone()).or_insert_with(Vec::new).push(insight);
  }

  for (topic, insights) in by_topic {
    println!("{} {}", "📂".cyan(), topic.blue().bold());

    for insight in insights {
      if verbose {
        println!("  {} {} - {}", "📄".yellow(), insight.name.bold(), insight.overview.dimmed());
      } else {
        println!("  {} {}", "📄".yellow(), insight.name.bold());
      }
    }
    println!();
  }

  Ok(())
}

pub async fn list_topics() -> Result<()> {
  ensure_server_running().await?;

  let client = get_client();
  let response = client.list_topics().await?;

  if response.is_empty() {
    println!("No topics found.");
    return Ok(());
  }

  println!("{} Available topics:", "📂".cyan());
  for topic in response {
    println!("  {}", topic.blue());
  }

  Ok(())
}

pub async fn update_insight(
  topic: &str,
  name: &str,
  overview: Option<&str>,
  details: Option<&str>,
) -> Result<()> {
  if overview.is_none() && details.is_none() {
    return Err(anyhow!("At least one of --overview or --details must be specified"));
  }

  ensure_server_running().await?;

  let client = get_client();
  client.update_insight(topic, name, overview, details).await?;

  println!("{} Updated insight {}/{}", "✓".green(), topic.cyan(), name.yellow());
  Ok(())
}

pub async fn delete_insight(topic: &str, name: &str, force: bool) -> Result<()> {
  ensure_server_running().await?;
  let client = get_client();

  // First check if the insight exists
  match client.get_insight(topic, name, true).await {
    Ok(_) => {
      // Insight exists, proceed with deletion
      if !force {
        // Ask for confirmation
        print!("Are you sure you want to delete insight {}/{}? (y/N): ", topic.cyan(), name.yellow());
        std::io::Write::flush(&mut std::io::stdout())?;

        let mut input = String::new();
        std::io::Read::read_to_string(&mut std::io::stdin(), &mut input)?;

        let response = input.trim().to_lowercase();
        if response != "y" && response != "yes" {
          println!("Delete operation cancelled.");
          return Ok(());
        }
      }

      // Proceed with deletion
      client.remove_insight(topic, name).await?;
      println!("{} Deleted insight {}/{}", "✓".green(), topic.cyan(), name.yellow());
      Ok(())
    }
    Err(_) => {
      // Insight doesn't exist
      return Err(anyhow!("Insight {}/{} not found", topic, name));
    }
  }
}

pub fn index_insights(force: bool) -> Result<()> {
  println!("{} Starting insight indexing...", "🔄".cyan());

  // Get all insights
  let insights = insight::get_insights(None)?;

  if insights.is_empty() {
    println!("No insights found to index.");
    return Ok(());
  }

  let total_insights = insights.len();
  let mut processed_count = 0;
  let mut updated_count = 0;

  for mut insight in insights {
    processed_count += 1;

    // Check if already has embeddings and force flag
    let needs_embedding = insight.embedding.is_none() || force;

    if needs_embedding {
      println!(
        "Processing {}/{}: {}/{}",
        processed_count.to_string().cyan(),
        total_insights.to_string().cyan(),
        insight.topic.blue(),
        insight.name.yellow()
      );

      // Create embedding for this insight
      // TODO: Implement embedding functionality via REST API
      println!("Embedding functionality not yet implemented in REST API mode");
      continue;

    } else {
      // Skip insights that already have embeddings (unless force is true)
      continue;
    }
  }

  let total_processed = processed_count;
  let total_updated = updated_count;

  println!();
  println!(
    "{} Indexed {} of {} insights",
    "✓".green(),
    total_updated.to_string().yellow(),
    total_processed.to_string().cyan()
  );

  Ok(())
}

/// Query daemon logs for debugging and monitoring
pub async fn logs(_limit: usize, _level: &str) -> Result<()> {
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

/// Search through all insights for matching content
pub async fn search_insights(
  terms: &[String],
  topic: Option<String>,
  case_sensitive: bool,
  overview_only: bool,
  exact: bool,
) -> Result<()> {
  ensure_server_running().await?;

  let client = get_client();
  let response = client
    .search_insights(terms.to_vec(), topic, case_sensitive, overview_only, exact)
    .await?;

  // Display results using the same logic as the original search
  display_search_results(&response.results, terms, overview_only);

  Ok(())
}

/// Display search results (moved from search.rs)
fn display_search_results(
  results: &[crate::server::types::SearchResultData], 
  terms: &[String], 
  overview_only: bool
) {
  if results.is_empty() {
    println!("No matches found for: {}", terms.join(" ").yellow());
  } else {
    for result in results {
      display_single_search_result(result, overview_only);
    }
  }
}

/// Display a single search result
fn display_single_search_result(result: &crate::server::types::SearchResultData, overview_only: bool) {
  let header = format!(
    "=== {}/{} ===",
    result.topic.blue().bold(),
    result.name.yellow().bold()
  );

  println!("{}", header);
        
  // Wrap and display the content with proper formatting
  let wrap_width = if header.len() < 80 { 80 } else { header.len() };

  let content = if overview_only {
    result.overview.to_string()
  } else {
    format!("{}\n\n{}", result.overview, result.details)
  };

  let wrapped_lines = wrap_text(&content, wrap_width);
  for line in wrapped_lines {
    println!("{}", line);
  }
  println!();
}

/// Wrap text to fit within a specified width
fn wrap_text(text: &str, width: usize) -> Vec<String> {
  let mut lines = Vec::new();
  
  for paragraph in text.split('\n') {
    if paragraph.trim().is_empty() {
      lines.push(String::new());
      continue;
    }
    
    let words: Vec<&str> = paragraph.split_whitespace().collect();
    let mut current_line = String::new();
    
    for word in words {
      if current_line.is_empty() {
        current_line = word.to_string();
      } else if current_line.len() + 1 + word.len() <= width {
        current_line.push(' ');
        current_line.push_str(word);
      } else {
        lines.push(current_line);
        current_line = word.to_string();
      }
    }
    
    if !current_line.is_empty() {
      lines.push(current_line);
    }
  }
  
  lines
}