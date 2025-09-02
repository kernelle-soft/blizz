use anyhow::{anyhow, Result};
use colored::*;

use crate::cli::client::get_client;
use crate::cli::display::display_search_result;
use crate::cli::server_manager::ensure_server_running;
// CLI is now a pure thin client - no business logic imports needed

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
        std::io::stdin().read_line(&mut input)?;

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

pub async fn index_insights(_force: bool) -> Result<()> {
  ensure_server_running().await?;
  let client = get_client();

  println!("{} Starting insight re-indexing...", "🔄".cyan());
  println!("   This will run in the background and may take some time");

  match client.reindex_insights().await {
    Ok(()) => {
      println!("{} Re-indexing started successfully!", "✓".green());
      println!("   Check server logs for progress updates");
      Ok(())
    }
    Err(e) => {
      println!("{} Failed to start re-indexing: {}", "✗".red(), e);
      Err(e)
    }
  }
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

  for log in logs_response.data.logs {
    let level_colored = match log.level.as_str() {
      "error" => log.level.red().bold(),
      "warn" => log.level.yellow().bold(),
      "info" => log.level.blue().bold(),
      "debug" => log.level.green(),
      "success" => log.level.bright_green().bold(),
      _ => log.level.normal(),
    };

    // Main log line with timestamp, level, and message
    println!("{} [{}] {}", log.timestamp.to_string().cyan(), level_colored, log.message);
    
    // Pretty-print context if available
    if let Some(context) = &log.context {
      let mut context_parts = Vec::new();
      
      if let Some(request_id) = &context.request_id {
        context_parts.push(format!("request_id: {}", request_id.bright_blue()));
      }
      
      if let Some(method) = &context.method {
        context_parts.push(format!("method: {}", method.magenta().bold()));
      }
      
      if let Some(path) = &context.path {
        context_parts.push(format!("path: {}", path.cyan()));
      }
      
      if let Some(user_agent) = &context.user_agent {
        context_parts.push(format!("user_agent: {}", user_agent.white().dimmed()));
      }
      
      if let Some(status_code) = context.status_code {
        let status_color = match status_code {
          200..=299 => status_code.to_string().green(),
          300..=399 => status_code.to_string().yellow(),
          400..=499 => status_code.to_string().red(),
          500..=599 => status_code.to_string().bright_red().bold(),
          _ => status_code.to_string().white(),
        };
        context_parts.push(format!("status: {}", status_color));
      }
      
      if let Some(duration) = context.duration_ms {
        let duration_color = if duration < 1.0 {
          format!("{:.2}ms", duration).bright_green()
        } else if duration < 10.0 {
          format!("{:.2}ms", duration).green()
        } else if duration < 100.0 {
          format!("{:.2}ms", duration).yellow()
        } else {
          format!("{:.2}ms", duration).red()
        };
        context_parts.push(format!("duration: {}", duration_color));
      }
      
      if !context_parts.is_empty() {
        for part in context_parts {
          println!("  {} {}", "└─".white().dimmed(), part);
        }
        println!();
      }
    }
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
      display_search_result(&result.topic, &result.name, &result.overview, &result.details, terms, overview_only);
    }
  }
}

// Display functions moved to cli/display.rs